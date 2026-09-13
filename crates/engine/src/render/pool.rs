//! The mesh pool: fixed-capacity render slots for terrain chunks plus the
//! asynchronous vertex update pipeline (feature 3 of the planet runtime,
//! `plan/features/03-mesh-pool.md`).
//!
//! The pool lives inside the `render` module because `VertexBuffer` and
//! [`TexVertexGpu`] are crate-internal (see the visibility note in
//! `plan/RELATED.md`); the pool core itself is GPU-free - no vulkano type
//! appears here - so slot bookkeeping and the worker pipeline are fully
//! testable headless. The GPU binding (one pre-allocated
//! `VertexBuffer<TexVertexGpu>` per slot, uploaded from the dirty slots)
//! lives in the runtime window renderer.
//!
//! Model (Decision 2 and 4 of `plan/RELATED.md`):
//!
//! - The slot count is fixed at creation and never grows; freed slots are
//!   recycled. No GPU mesh object is created during any LOD transition:
//!   splits, merges and displacement only rewrite vertex data inside the
//!   assigned slot.
//! - The node graph is `Rc<RefCell<Node>>` and therefore `!Send`. The main
//!   thread extracts plain geometry inputs ([`ChunkGeometry`]) from the
//!   nodes; worker threads compute the vertex data
//!   ([`compute_chunk_vertices`]) from that plain data with double-buffered
//!   staging (the slot keeps drawing its live buffer while the worker fills
//!   a staging buffer; the two are swapped on completion, and the retired
//!   buffer is recycled as staging); results come back to the main thread
//!   for the GPU upload. Workers never see a node reference - the boundary
//!   is enforced by the types.
//! - Retired node generations are cleaned up by the scheduler's topology
//!   operations; the pool itself only recycles slots and buffers and never
//!   holds a node graph cycle (slot `NodeRef`s are released on unassign).

use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use glam::Vec3;

use crate::lod::{BorderState, FrameReport, border_states};
use crate::node::NodeRef;

use super::vertices::TexVertexGpu;

/// Vertex count of a chunk's main triangle.
pub const MAIN_VERTICES: usize = 3;
/// Vertex count of one border skirt (a quad emitted as two triangles).
pub const SKIRT_VERTICES: usize = 6;
/// Maximum vertex count one chunk slot can produce: the main triangle plus
/// one skirt per border. Slot GPU allocations are sized to this bound.
pub const MAX_CHUNK_VERTICES: usize = MAIN_VERTICES + 3 * SKIRT_VERTICES;

/// Skirt depth relative to the chunk's shortest edge: short enough to stay
/// hidden below the surface, long enough to mask a T-junction seam. Also
/// the margin the visibility pass adds to chunk bounding volumes so the
/// skirts stay inside the culled volume.
pub(crate) const SKIRT_DEPTH_FACTOR: f32 = 0.02;

/// Barycentric coordinates of the triangle corners, in A/B/C order (same
/// convention as the scene builders).
const CORNER_BARY: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// Configuration of a [`MeshPool`], fixed at creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolConfig {
    /// Number of render slots. Never grows at runtime; when every slot is
    /// assigned, further assignments queue until a slot is released.
    pub slots: usize,
    /// Number of vertex worker threads. Clamped to at least 1.
    pub workers: usize,
}

/// Plain, `Send` geometry inputs of one chunk, extracted from a node on the
/// main thread. This is the thread boundary of the async pipeline (Decision
/// 4): workers receive only this plain data, never a node reference.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChunkGeometry {
    /// Triangle corners `[A, B, C]` in world space.
    vertices: [[f32; 3]; 3],
    /// Per-corner texture coordinates, in A/B/C order.
    uv: [[f32; 2]; 3],
    /// Per-corner ring-field values, in A/B/C order.
    ring: [f32; 3],
    /// Topology parity sign (+1/-1).
    parity: f32,
    /// Unit direction from the chunk center toward the planet origin (the
    /// downward direction for the border skirts).
    down: [f32; 3],
    /// Per-port border classification (skirt metadata).
    borders: [BorderState; 3],
    /// Skirt depth in world units.
    skirt_depth: f32,
}

impl ChunkGeometry {
    /// Extracts the plain geometry inputs of `node`. Main thread only: this
    /// is the single place where the pipeline touches the node graph.
    pub fn extract(node: &NodeRef) -> Self {
        let node_ref = node.borrow();
        let [a, b, c] = node_ref.vertices;
        let shortest_edge = (b - a).length().min((c - b).length()).min((a - c).length());
        ChunkGeometry {
            vertices: node_ref.vertices.map(|p| p.to_array()),
            uv: node_ref.uv.map(|uv| uv.to_array()),
            ring: node_ref.seed_distance,
            parity: node_ref.parity.sign() as f32,
            down: node_ref
                .direction_to_origin
                .try_normalize()
                .unwrap_or(Vec3::ZERO)
                .to_array(),
            borders: border_states(node),
            skirt_depth: SKIRT_DEPTH_FACTOR * shortest_edge,
        }
    }
}

/// Computes the vertex data of one chunk from its plain geometry inputs:
/// the main triangle (in A/B/C order, matching
/// `SceneBuilder::add_textured_triangle`) plus, for every border that is not
/// welded to a same-level neighbor, a short skirt quad displaced toward the
/// planet center (the T-junction crack mask of Decision 5). Pure function of
/// the inputs: it is both the worker body and the synchronous reference, so
/// the two paths are byte-identical by construction.
pub fn compute_chunk_vertices(geometry: &ChunkGeometry) -> Vec<TexVertexGpu> {
    let down = Vec3::from_array(geometry.down);
    let vertex = |corner: usize| TexVertexGpu {
        pos: geometry.vertices[corner],
        uv: geometry.uv[corner],
        bary: CORNER_BARY[corner],
        parity: geometry.parity,
        radial: geometry.down,
        ring: geometry.ring[corner],
    };
    let mut out = Vec::with_capacity(MAX_CHUNK_VERTICES);
    for corner in 0..3 {
        out.push(vertex(corner));
    }
    for (port, border) in geometry.borders.iter().enumerate() {
        // A welded border shares its edge vertices exactly with the
        // neighbor; every other border gets a skirt (Open boundaries and
        // both sides of a T-junction).
        if *border == BorderState::Welded {
            continue;
        }
        // Port I/J/K faces edge AB/BC/CA.
        let (u_corner, v_corner) = (port, (port + 1) % 3);
        let u = Vec3::from_array(geometry.vertices[u_corner]);
        let v = Vec3::from_array(geometry.vertices[v_corner]);
        let top_u = vertex(u_corner);
        let top_v = vertex(v_corner);
        let mut bottom_u = top_u;
        let mut bottom_v = top_v;
        // The skirt bottom hangs toward the planet center; its other
        // attributes stay the edge corners', so the skirt shades like its
        // border edge.
        bottom_u.pos = (u + down * geometry.skirt_depth).to_array();
        bottom_v.pos = (v + down * geometry.skirt_depth).to_array();
        out.extend_from_slice(&[top_u, top_v, bottom_v, top_u, bottom_v, bottom_u]);
    }
    out
}

/// One unit of worker input: the staging buffer is filled by the worker and
/// handed back in the matching [`VertexResult`].
struct VertexJob {
    slot: usize,
    /// Assignment generation of the slot at dispatch time; a result whose
    /// generation no longer matches the slot is stale and dropped.
    generation: u64,
    /// Per-slot write sequence; only the latest dispatched write applies.
    write: u64,
    geometry: ChunkGeometry,
    staging: Vec<TexVertexGpu>,
}

/// A completed vertex computation, handed back to the main thread.
struct VertexResult {
    slot: usize,
    generation: u64,
    write: u64,
    vertices: Vec<TexVertexGpu>,
}

/// A bounded pool of worker threads computing chunk vertex data from plain
/// geometry inputs. Main-thread API: jobs are dispatched with a staging
/// buffer (recycled from completed results), results are drained by
/// [`MeshPool::poll`]. Dropping the pool closes the job channel and joins
/// the workers.
struct VertexWorkers {
    job_tx: Option<Sender<VertexJob>>,
    result_rx: Receiver<VertexResult>,
    /// Recycled staging buffers (double-buffered staging: a slot's live
    /// buffer keeps drawing while a staging buffer is filled off-thread).
    staging_rx: Receiver<Vec<TexVertexGpu>>,
    staging_tx: Sender<Vec<TexVertexGpu>>,
    handles: Vec<JoinHandle<()>>,
    /// Jobs dispatched but not yet drained.
    pending: usize,
    /// Total jobs completed (worker activity readout).
    completed: u64,
}

impl VertexWorkers {
    fn new(count: usize) -> Self {
        let count = count.max(1);
        let (job_tx, job_rx) = mpsc::channel::<VertexJob>();
        let (result_tx, result_rx) = mpsc::channel::<VertexResult>();
        let (staging_tx, staging_rx) = mpsc::channel::<Vec<TexVertexGpu>>();
        // One shared job queue: the receiver is behind a mutex so every
        // worker pops the next job (std mpsc receivers are not cloneable).
        let job_rx = Arc::new(Mutex::new(job_rx));
        let handles = (0..count)
            .map(|_| {
                let job_rx = Arc::clone(&job_rx);
                let result_tx = result_tx.clone();
                std::thread::spawn(move || {
                    loop {
                        let job = {
                            let Ok(guard) = job_rx.lock() else { return };
                            guard.recv()
                        };
                        let Ok(mut job) = job else { return };
                        job.staging.clear();
                        job.staging
                            .extend_from_slice(&compute_chunk_vertices(&job.geometry));
                        if result_tx
                            .send(VertexResult {
                                slot: job.slot,
                                generation: job.generation,
                                write: job.write,
                                vertices: job.staging,
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                })
            })
            .collect();
        VertexWorkers {
            job_tx: Some(job_tx),
            result_rx,
            staging_rx,
            staging_tx,
            handles,
            pending: 0,
            completed: 0,
        }
    }

    /// A recycled staging buffer, or a fresh one when none is available.
    fn take_staging(&self) -> Vec<TexVertexGpu> {
        self.staging_rx
            .try_recv()
            .unwrap_or_else(|_| Vec::with_capacity(MAX_CHUNK_VERTICES))
    }

    /// Hands a staging buffer back for reuse. Empty (never-allocated)
    /// buffers are dropped instead of polluting the recycling channel.
    fn recycle_staging(&self, buffer: Vec<TexVertexGpu>) {
        if buffer.capacity() > 0 {
            let _ = self.staging_tx.send(buffer);
        }
    }
}

impl Drop for VertexWorkers {
    fn drop(&mut self) {
        // Closing the job channel ends every worker loop after its current
        // job; the results are discarded with the channels.
        self.job_tx.take();
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}

/// One render slot: a stable assignment target with a live CPU-side vertex
/// buffer. The matching GPU allocation lives in the runtime window renderer,
/// indexed by the slot number, and is uploaded from the dirty slots.
struct PoolSlot {
    /// The assigned chunk (stable node name and graph handle for
    /// re-extraction), or `None` while the slot is free.
    chunk: Option<(String, NodeRef)>,
    /// Assignment generation, bumped on every assign/unassign; invalidates
    /// in-flight results when the slot is recycled.
    generation: u64,
    /// Latest dispatched write sequence; only its result applies.
    write: u64,
    /// Live CPU-side vertex data (the source of the GPU uploads).
    vertices: Vec<TexVertexGpu>,
    /// Whether `vertices` changed since the last GPU upload.
    dirty: bool,
    /// Whether a worker job for the latest write is in flight.
    in_flight: bool,
    /// Whether a rewrite was requested while a job was in flight; the pool
    /// re-dispatches with fresh geometry when the in-flight result lands.
    rewrite_pending: bool,
}

impl PoolSlot {
    fn empty() -> Self {
        PoolSlot {
            chunk: None,
            generation: 0,
            write: 0,
            vertices: Vec::new(),
            dirty: false,
            in_flight: false,
            rewrite_pending: false,
        }
    }
}

/// What one [`MeshPool::poll`] applied.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PollOutcome {
    /// Worker results applied to their slots this poll.
    pub applied_jobs: usize,
    /// Vertices written into slot buffers this poll (the per-frame vertex
    /// write readout).
    pub vertex_writes: usize,
}

/// A snapshot of the pool state, for the debug overlay and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolStats {
    /// Fixed slot count.
    pub capacity: usize,
    /// Slots currently assigned to a chunk.
    pub used: usize,
    /// Worker jobs dispatched but not yet applied.
    pub pending_jobs: usize,
    /// Chunks waiting for a free slot (pool exhaustion backlog).
    pub queued_assignments: usize,
    /// Number of worker threads.
    pub workers: usize,
    /// Total worker jobs completed since creation (worker activity).
    pub completed_jobs: u64,
}

/// The fixed-capacity mesh pool. Main thread only: [`apply_report`](Self::apply_report)
/// consumes the LOD scheduler's [`FrameReport`] and drives slot
/// assign/unassign plus vertex rewrites; [`poll`](Self::poll) applies
/// completed worker results; the renderer uploads the dirty slots.
pub struct MeshPool {
    slots: Vec<PoolSlot>,
    free: Vec<usize>,
    /// Chunk name to slot index, for assigned chunks only.
    by_name: HashMap<String, usize>,
    /// Chunks that could not be assigned yet (pool exhaustion), FIFO.
    queued: VecDeque<(String, NodeRef)>,
    workers: VertexWorkers,
}

impl MeshPool {
    /// Creates a pool with a fixed slot count and a bounded worker pool.
    /// No GPU object is created here; the renderer pre-allocates one
    /// `VertexBuffer<TexVertexGpu>` per slot, sized to
    /// [`MAX_CHUNK_VERTICES`].
    pub fn new(config: PoolConfig) -> Self {
        let slots = config.slots.max(1);
        MeshPool {
            slots: (0..slots).map(|_| PoolSlot::empty()).collect(),
            free: (0..slots).rev().collect(),
            by_name: HashMap::new(),
            queued: VecDeque::new(),
            workers: VertexWorkers::new(config.workers),
        }
    }

    /// Applies one frame of the LOD scheduler's report:
    ///
    /// 1. **Splits** - the split chunk's slot is re-keyed to the center
    ///    child (rewritten in place) and the three corner children are
    ///    assigned slots; every group member and every assigned external
    ///    neighbor (whose border states changed) gets a vertex rewrite.
    /// 2. **Merges** - the four group slots collapse into one, re-keyed to
    ///    the recovered parent; the parent and its assigned neighbors are
    ///    rewritten.
    /// 3. **Unloaded** - chunks leaving the active zone release their slot.
    /// 4. **Loaded** - chunks entering the active zone are assigned a slot
    ///    and rewritten.
    ///
    /// All geometry extraction happens here, on the main thread; the worker
    /// threads only receive the plain [`ChunkGeometry`].
    pub fn apply_report(&mut self, report: &FrameReport) {
        for center in &report.splits {
            let members = split_group(center);
            let base = group_base_name(center);
            if let Some(slot) = self.by_name.get(&base).copied() {
                // The split chunk's slot is rewritten in place: re-key it to
                // the center child instead of freeing it.
                self.rekey(slot, base, &members[3]);
            }
            for member in &members {
                self.ensure_assigned(member);
            }
            self.rewrite_group_and_neighbors(&members);
        }
        for parent in &report.merges {
            let name = parent.borrow().name.clone();
            // Collapse the group slots into one, re-keyed to the parent.
            let mut kept: Option<usize> = None;
            for suffix in [".C", ".I", ".J", ".K"] {
                let child_name = format!("{name}{suffix}");
                if let Some(slot) = self.by_name.get(&child_name).copied() {
                    if kept.is_none() {
                        self.rekey(slot, child_name, parent);
                        kept = Some(slot);
                    } else {
                        self.release(child_name.as_str());
                    }
                }
            }
            if kept.is_some() {
                self.rewrite_chunk(parent);
                self.rewrite_neighbors(parent);
            }
        }
        for node in &report.unloaded {
            let name = node.borrow().name.clone();
            self.release(&name);
        }
        for node in &report.loaded {
            self.ensure_assigned(node);
            self.rewrite_chunk(node);
        }
    }

    /// Drains the completed worker results: each fresh result swaps its
    /// staging buffer into the slot (the live buffer is recycled as
    /// staging); stale results (recycled slot or superseded write) are
    /// dropped and their buffer recycled. Slots with a pending rewrite
    /// re-dispatch immediately with freshly extracted geometry.
    pub fn poll(&mut self) -> PollOutcome {
        let mut outcome = PollOutcome::default();
        while let Ok(result) = self.workers.result_rx.try_recv() {
            self.workers.pending -= 1;
            self.workers.completed += 1;
            let mut redispatch = None;
            {
                let slot = &mut self.slots[result.slot];
                let fresh = slot.chunk.is_some()
                    && slot.generation == result.generation
                    && slot.write == result.write;
                if fresh {
                    outcome.applied_jobs += 1;
                    outcome.vertex_writes += result.vertices.len();
                    let retired = std::mem::replace(&mut slot.vertices, result.vertices);
                    slot.dirty = true;
                    self.workers.recycle_staging(retired);
                } else {
                    // Stale: the slot was recycled (generation bumped) or a
                    // newer write superseded this one. Drop the data.
                    self.workers.recycle_staging(result.vertices);
                }
                slot.in_flight = false;
                if slot.rewrite_pending
                    && let Some((_, node)) = &slot.chunk
                {
                    slot.rewrite_pending = false;
                    redispatch = Some(Rc::clone(node));
                }
            }
            // A rewrite requested while this job was in flight re-dispatches
            // now, with freshly extracted geometry.
            if let Some(node) = redispatch {
                self.dispatch(result.slot, &node);
            }
        }
        outcome
    }

    /// The dirty slots as `(slot index, live vertex data)`, clearing the
    /// dirty flags. The renderer uploads each to the slot's GPU buffer.
    pub fn take_dirty(&mut self) -> Vec<(usize, &[TexVertexGpu])> {
        let mut dirty = Vec::new();
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if slot.dirty && slot.chunk.is_some() {
                slot.dirty = false;
                dirty.push((index, slot.vertices.as_slice()));
            }
        }
        dirty
    }

    /// A snapshot of the pool state, for the debug overlay and tests.
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            capacity: self.slots.len(),
            used: self.by_name.len(),
            pending_jobs: self.workers.pending,
            queued_assignments: self.queued.len(),
            workers: self.workers.handles.len(),
            completed_jobs: self.workers.completed,
        }
    }

    /// The slot index of an assigned chunk, for tests.
    pub fn slot_of(&self, chunk: &str) -> Option<usize> {
        self.by_name.get(chunk).copied()
    }

    /// The live CPU-side vertex data of a slot, for tests and the renderer.
    pub fn slot_vertices(&self, slot: usize) -> &[TexVertexGpu] {
        &self.slots[slot].vertices
    }

    /// Assigns `node` to a free slot when it has none; queues the assignment
    /// when the pool is exhausted (it is retried as slots are released).
    fn ensure_assigned(&mut self, node: &NodeRef) {
        let name = node.borrow().name.clone();
        if self.by_name.contains_key(&name) || self.queued.iter().any(|(queued, _)| *queued == name)
        {
            return;
        }
        match self.free.pop() {
            Some(slot) => self.assign(slot, name, node),
            None => self.queued.push_back((name, Rc::clone(node))),
        }
    }

    /// Moves a free slot to `node` under `name`, bumping the generation so
    /// in-flight results for the previous assignment become stale.
    fn assign(&mut self, slot: usize, name: String, node: &NodeRef) {
        let entry = &mut self.slots[slot];
        entry.generation += 1;
        entry.rewrite_pending = false;
        let retired = std::mem::take(&mut entry.vertices);
        self.workers.recycle_staging(retired);
        entry.chunk = Some((name.clone(), Rc::clone(node)));
        entry.dirty = false;
        self.by_name.insert(name, slot);
    }

    /// Re-keys `slot` from `old_name` to `node` (split center or merge
    /// parent): the slot and its GPU allocation stay, only the vertex data
    /// is rewritten. Bumps the generation, invalidating in-flight results.
    fn rekey(&mut self, slot: usize, old_name: String, node: &NodeRef) {
        self.by_name.remove(&old_name);
        let name = node.borrow().name.clone();
        let entry = &mut self.slots[slot];
        entry.generation += 1;
        entry.rewrite_pending = false;
        entry.chunk = Some((name.clone(), Rc::clone(node)));
        self.by_name.insert(name, slot);
    }

    /// Releases the slot of `name` back to the free list (bumping its
    /// generation), then retries queued assignments FIFO.
    fn release(&mut self, name: &str) {
        if let Some(slot) = self.by_name.remove(name) {
            let entry = &mut self.slots[slot];
            entry.generation += 1;
            entry.rewrite_pending = false;
            let retired = std::mem::take(&mut entry.vertices);
            self.workers.recycle_staging(retired);
            entry.chunk = None;
            entry.dirty = false;
            self.free.push(slot);
        }
        while let Some(slot) = self.free.pop() {
            let Some((name, node)) = self.queued.pop_front() else {
                self.free.push(slot);
                break;
            };
            if self.by_name.contains_key(&name) {
                self.free.push(slot);
                continue;
            }
            self.assign(slot, name.clone(), &node);
            let node = Rc::clone(&node);
            self.dispatch(self.by_name[&name], &node);
        }
    }

    /// Dispatches a vertex rewrite for the slot of `node` when assigned.
    fn rewrite_chunk(&mut self, node: &NodeRef) {
        let name = node.borrow().name.clone();
        let Some(slot) = self.by_name.get(&name).copied() else {
            return;
        };
        self.dispatch(slot, node);
    }

    /// Dispatches a vertex rewrite for every group member and every assigned
    /// neighbor linked outside the group (their border states changed).
    fn rewrite_group_and_neighbors(&mut self, members: &[NodeRef; 4]) {
        for member in members {
            self.rewrite_chunk(member);
        }
        let mut neighbors: Vec<NodeRef> = Vec::new();
        for member in members {
            for neighbor in member.borrow().children.iter().flatten() {
                if members.iter().any(|m| Rc::ptr_eq(m, neighbor))
                    || neighbors.iter().any(|n| Rc::ptr_eq(n, neighbor))
                {
                    continue;
                }
                neighbors.push(Rc::clone(neighbor));
            }
        }
        for neighbor in neighbors {
            self.rewrite_chunk(&neighbor);
        }
    }

    /// Dispatches a vertex rewrite for every assigned neighbor of `node`.
    fn rewrite_neighbors(&mut self, node: &NodeRef) {
        let neighbors: Vec<NodeRef> = node
            .borrow()
            .children
            .iter()
            .flatten()
            .map(Rc::clone)
            .collect();
        for neighbor in neighbors {
            self.rewrite_chunk(&neighbor);
        }
    }

    /// Extracts the geometry of `node` on the main thread and dispatches a
    /// worker job for `slot`; a job already in flight defers the rewrite to
    /// its completion (with fresh geometry re-extracted then).
    fn dispatch(&mut self, slot: usize, node: &NodeRef) {
        if self.slots[slot].in_flight {
            self.slots[slot].rewrite_pending = true;
            return;
        }
        let geometry = ChunkGeometry::extract(node);
        let staging = self.workers.take_staging();
        let entry = &mut self.slots[slot];
        entry.in_flight = true;
        entry.write += 1;
        let job = VertexJob {
            slot,
            generation: entry.generation,
            write: entry.write,
            geometry,
            staging,
        };
        self.workers.pending += 1;
        if let Some(tx) = &self.workers.job_tx {
            // The send only fails while the pool is being dropped.
            let _ = tx.send(job);
        }
    }
}

impl Drop for MeshPool {
    fn drop(&mut self) {
        // Release the graph handles explicitly: the slots only ever hold
        // chunk nodes by `Rc` and never create a cycle, but dropping them
        // here keeps the graph cleanup ordering obvious (the graph itself
        // is destroyed by its owner, the runtime window).
        for slot in &mut self.slots {
            slot.chunk = None;
        }
        self.queued.clear();
    }
}

/// The four members of the split group of `center`: the corner children
/// `.I`, `.J`, `.K` (linked from the center's ports 1, 0, 2 - the same
/// wiring the scheduler relies on) and the center child itself, last.
fn split_group(center: &NodeRef) -> [NodeRef; 4] {
    let center_ref = center.borrow();
    let node_j = Rc::clone(center_ref.children[0].as_ref().expect("group corner J"));
    let node_i = Rc::clone(center_ref.children[1].as_ref().expect("group corner I"));
    let node_k = Rc::clone(center_ref.children[2].as_ref().expect("group corner K"));
    drop(center_ref);
    [node_i, node_j, node_k, Rc::clone(center)]
}

/// Base name of the split group `center` belongs to (its `.C` name minus
/// the suffix).
fn group_base_name(center: &NodeRef) -> String {
    let name = &center.borrow().name;
    name.strip_suffix(".C")
        .expect("split center name ends in .C")
        .to_string()
}
