## Mesh Pool Specification (`crates/engine/src/render/pool.rs`)

### Overview

The mesh pool is a submodule of `render` (feature 3 of the procedural
planet runtime, `plan/features/03-mesh-pool.md`): a fixed-capacity pool of
render slots for terrain chunks plus the fully asynchronous vertex update
pipeline. It lives inside the `render` module because `VertexBuffer` and
`TexVertexGpu` are crate-internal; the pool core itself is GPU-free (no
vulkano type appears in `pool.rs`), so slot bookkeeping and the worker
pipeline are fully testable headless. The GPU binding - one pre-allocated
`VertexBuffer<TexVertexGpu>` per slot, uploaded from the dirty slots -
lives in the runtime window renderer (see the [render
specification](render.md)).

Status: **Current baseline** for slot pooling, in-place vertex rewrites,
skirt generation in the vertex data, the async worker pipeline, and
frustum/horizon culling with draw-call reduction (feature 4, see the
[visibility specification](visibility.md)). The
shader-side terrain morph that displaces the skirts further (feature 5)
is **Planned**.

### Structure

- **Plain geometry inputs** - `ChunkGeometry`: the `Send` extraction of one
  chunk node (corner positions, UVs, ring values, parity sign, the
  normalized `direction_to_origin` as the skirt down direction, the three
  `BorderState`s, and the skirt depth). `ChunkGeometry::extract(&NodeRef)`
  is the single place where the pipeline touches the node graph, and it
  runs on the main thread only. This is the thread boundary of Decision 4
  (`plan/RELATED.md`): the graph is `Rc<RefCell<Node>>` and `!Send`, so
  workers receive only this plain data, enforced by the types.
- **Vertex computation** - `compute_chunk_vertices(&ChunkGeometry) ->
  Vec<TexVertexGpu>`: a pure function emitting the chunk's main triangle
  (in A/B/C order, same attribute convention as
  `SceneBuilder::add_textured_triangle`) plus, for every border that is not
  `Welded`, a short skirt quad (two triangles) displaced toward the planet
  center by `skirt_depth` (2% of the chunk's shortest edge). It is both the
  worker body and the synchronous reference, so the two paths are
  byte-identical by construction.
- **Worker pool** - `VertexWorkers`: a bounded pool of `std::thread`
  workers (no new dependencies) fed through one shared job queue with
  double-buffered staging: a slot's live buffer keeps drawing while a
  worker fills a recycled staging buffer; the two are swapped when the
  result lands on the main thread, and the retired buffer is recycled as
  staging. Dropping the pool closes the job channel and joins the workers.
- **Pool** - `MeshPool`:
  - `MeshPool::new(PoolConfig { slots, workers })` - fixed slot count,
    never grows at runtime; freed slots are recycled. The renderer
    pre-allocates one GPU vertex buffer per slot, sized to
    `MAX_CHUNK_VERTICES` (21: the main triangle plus one 6-vertex skirt
    per border), so no GPU mesh object is created during any LOD
    transition.
  - `apply_report(&FrameReport)` - drives assignment from the LOD
    scheduler (see "Report mapping" below).
  - `poll() -> PollOutcome` - drains completed worker results into the
    slots (fresh results swap in and mark the slot dirty; stale results of
    recycled slots or superseded writes are dropped), and re-dispatches
    rewrites that were deferred while a job was in flight.
  - `take_dirty() -> Vec<(usize, &[TexVertexGpu])>` - the slots to upload,
    clearing the dirty flags (consumed by the runtime window renderer).
  - `stats() -> PoolStats` - capacity, used slots, pending jobs, queued
    assignments (exhaustion backlog), worker count, completed jobs; the
    debug overlay readouts.

### Report mapping

`apply_report` consumes one frame of the scheduler's
[`FrameReport`](lod.md), in order:

1. **Splits** - the split chunk's slot is re-keyed to the center child
   (the slot and its GPU allocation stay; only the vertex data is
   rewritten) and the three corner children are assigned slots. Every
   group member and every assigned external neighbor - whose border states
   changed - gets a vertex rewrite.
2. **Merges** - the four group slots collapse into one, re-keyed to the
   recovered parent; the other three are released. The parent and its
   assigned neighbors are rewritten.
3. **Unloaded** - chunks leaving the active zone release their slot.
4. **Loaded** - chunks entering the active zone are assigned a free slot
   and rewritten; with no free slot, the assignment queues FIFO until a
   slot is released (the pool never grows).

Every slot mutation (assign, release, re-key) bumps the slot's assignment
generation, so a worker result that crosses a recycle is detected as stale
and dropped. Each dispatch bumps a per-slot write sequence; a rewrite
requested while a job is in flight is deferred and re-dispatched with
freshly extracted geometry when the result lands.

### Rules

- The slot count is fixed at creation; assignments recycle freed slots and
  queue when the pool is exhausted. It never grows at runtime.
- No GPU mesh object is created or resized during any LOD transition:
  splits, merges and displacement only rewrite vertex data inside the
  assigned slot. Split/unsplit only update vertex positions (Decision 2);
  no parent/child layered rendering occurs.
- Workers never access the node graph: the main thread extracts plain
  `ChunkGeometry`, workers compute `TexVertexGpu` data from it, and results
  come back to the main thread for the GPU upload (Decision 4). Worker
  output is byte-identical to the synchronous reference computation for
  the same inputs.
- Worker-produced vertex data is bounded by `MAX_CHUNK_VERTICES`, the size
  of every slot's stable GPU allocation.
- Retired node generations are cleaned up by the scheduler's topology
  operations (`unsplit_node` destroys the merged group); the pool holds
  chunk nodes by `Rc` only while assigned, never creates an `Rc` cycle,
  and releases every handle on unassign. The planet graph itself is
  destroyed by its owner (the runtime window, via `destroy_mesh`).
- The single terrain shader is unchanged: skirt vertices are displaced in
  the vertex data, not by the shader (feature 5 owns the terrain shader
  morph).
- The pool is main-thread-only API: `apply_report`, `poll` and
  `take_dirty` all run on the window's event-loop thread.

### Files

- `crates/engine/src/render/pool.rs` - `ChunkGeometry`,
  `compute_chunk_vertices`, `VertexWorkers`, `MeshPool`, `PoolConfig`,
  `PoolStats`, `PollOutcome`, `MAX_CHUNK_VERTICES`.
- `crates/engine/src/render/runtime_window.rs` - the GPU binding: one
  `VertexBuffer::with_capacity(MAX_CHUNK_VERTICES)` per slot, uploaded
  from `take_dirty` behind a device wait.
- `tests/render/pool.rs` - headless tests: exhaustion, recycling,
  byte-identical worker output, ordering, stale results, the `Send` type
  boundary, and a scripted player-path integration through the scheduler.
