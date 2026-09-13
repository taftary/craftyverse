//! The LOD scheduler: distance-based split and merge decisions over the
//! node graph, within a bounded per-frame operation budget.

use std::collections::{HashSet, VecDeque};
use std::rc::Rc;

use glam::Vec3;

use super::config::LodConfig;
use crate::node::{NodeRef, collect_nodes, split_node_local, unsplit_node};

/// A queued topology operation. Excess work is queued, never dropped; the
/// queue drains over the following frames.
enum Operation {
    /// Split this chunk node. The [`NodeRef`] is validated against the live
    /// node set at execution time; a stale reference drops the operation.
    Split(NodeRef),
    /// Merge the split group with this base name (its members are named
    /// `"{base}.I"`, `".J"`, `".K"`, `".C"`). Resolved by name at execution
    /// time; a missing or ineligible group drops the operation.
    Merge(String),
}

impl Operation {
    /// Stable dedup key: one queued operation per chunk or group.
    fn key(&self) -> String {
        match self {
            Operation::Split(node) => format!("S:{}", node.borrow().name),
            Operation::Merge(base) => format!("M:{base}"),
        }
    }
}

/// What one [`LodScheduler::update`] call changed.
///
/// The report is the hook for the render side (feature 3): new centers and
/// recovered parents are the chunks whose vertex data changed, and
/// `loaded` / `unloaded` track the active-zone membership changes.
#[derive(Default)]
pub struct FrameReport {
    /// Center nodes of the chunks split this frame, in execution order
    /// (forced neighbor splits included).
    pub splits: Vec<NodeRef>,
    /// Parent nodes recovered by merges this frame, in execution order.
    pub merges: Vec<NodeRef>,
    /// Chunks that entered the active zone this frame.
    pub loaded: Vec<NodeRef>,
    /// Chunks that left the active zone this frame.
    pub unloaded: Vec<NodeRef>,
}

impl std::fmt::Debug for FrameReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names = |nodes: &[NodeRef]| {
            nodes
                .iter()
                .map(|node| node.borrow().name.clone())
                .collect::<Vec<_>>()
        };
        f.debug_struct("FrameReport")
            .field("splits", &names(&self.splits))
            .field("merges", &names(&self.merges))
            .field("loaded", &names(&self.loaded))
            .field("unloaded", &names(&self.unloaded))
            .finish()
    }
}

/// The state of one chunk border port, for skirt (crack-masking) metadata.
///
/// With restricted subdivision the level difference across a shared edge is
/// at most 1, so a border is either watertight or a T-junction that the
/// terrain shader masks with a short skirt; no runtime cross-chunk
/// stitching is attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderState {
    /// No neighbor across this edge (mesh boundary or the open half-edge of
    /// a T-junction on the finer side).
    Open,
    /// Linked to a neighbor at the same level: the edge is watertight.
    Welded,
    /// Linked to a neighbor one level coarser: this chunk is the finer side
    /// of a T-junction and needs a skirt along this border.
    Coarser,
    /// Linked to a neighbor one level finer: this chunk is the coarser side
    /// of a T-junction.
    Finer,
}

/// Classifies the three border ports of `node` for the skirt metadata the
/// runtime window consumes (features 3/5 own the shader).
pub fn border_states(node: &NodeRef) -> [BorderState; 3] {
    let node_ref = node.borrow();
    let level = node_ref.level;
    std::array::from_fn(|port| match &node_ref.children[port] {
        None => BorderState::Open,
        Some(neighbor) => {
            let neighbor_level = neighbor.borrow().level;
            match neighbor_level.cmp(&level) {
                std::cmp::Ordering::Equal => BorderState::Welded,
                std::cmp::Ordering::Less => BorderState::Coarser,
                std::cmp::Ordering::Greater => BorderState::Finer,
            }
        }
    })
}

/// Distance-driven LOD scheduler over a planet's node graph.
///
/// Each [`update`](Self::update) reads the player position (orientation
/// never participates), drains at most
/// [`operations_per_frame`](LodConfig::operations_per_frame) queued
/// split/merge operations, queues newly threshold-crossing chunks, and
/// recomputes the active zone. Topology changes go through
/// [`split_node_local`] and [`unsplit_node`], so the node-graph invariants
/// (watertight shared edges, reciprocal links, exact merge recovery, no
/// leaked `Rc` cycles) hold after every frame.
///
/// Restricted subdivision is enforced at execution time: a split whose
/// neighbors are too coarse is redirected to the coarsest neighbor first
/// (a forced neighbor split, counted against the same budget), and a merge
/// whose group touches a finer group is dropped and re-evaluated on later
/// frames. The level difference across any shared edge therefore never
/// exceeds 1.
pub struct LodScheduler {
    config: LodConfig,
    roots: Vec<NodeRef>,
    active: Vec<NodeRef>,
    queue: VecDeque<Operation>,
    queued: HashSet<String>,
}

impl LodScheduler {
    /// Creates a scheduler over the mesh reachable from `roots` (typically
    /// the faces of an [`IcosphereMesh`](crate::node::IcosphereMesh)).
    ///
    /// # Errors
    ///
    /// Returns the violated [`LodConfig`](LodConfig) invariant as a
    /// [`LodConfigError`](crate::lod::LodConfigError).
    pub fn new(config: LodConfig, roots: Vec<NodeRef>) -> Result<Self, crate::lod::LodConfigError> {
        config.validate()?;
        Ok(LodScheduler {
            config,
            roots,
            active: Vec::new(),
            queue: VecDeque::new(),
            queued: HashSet::new(),
        })
    }

    /// The scheduler's configuration.
    pub fn config(&self) -> &LodConfig {
        &self.config
    }

    /// The current active chunk set: the loaded chunks inside the active
    /// zone, plus the nearest outside chunks up to
    /// [`min_active_meshes`](LodConfig::min_active_meshes).
    pub fn active_chunks(&self) -> &[NodeRef] {
        &self.active
    }

    /// Number of queued split/merge operations waiting for budget.
    pub fn queued_operations(&self) -> usize {
        self.queue.len()
    }

    /// The per-frame update: drain the operation queue within the budget,
    /// queue chunks that crossed a threshold, and recompute the active
    /// zone.
    ///
    /// # Panics
    ///
    /// Panics on a non-finite player position (internal invariant).
    pub fn update(&mut self, player_position: Vec3) -> FrameReport {
        assert!(
            player_position.is_finite(),
            "player position must be finite"
        );
        let live = self.live_nodes();
        let live_ptrs: HashSet<usize> = live.iter().map(|n| Rc::as_ptr(n) as usize).collect();
        let mut report = FrameReport::default();

        // 1. Drain the queue within the per-frame budget. Each executed
        //    split or merge - forced neighbor splits included - consumes
        //    exactly one operation.
        let mut operations = 0;
        while operations < self.config.operations_per_frame {
            let Some(operation) = self.queue.pop_front() else {
                break;
            };
            match operation {
                Operation::Split(node) => {
                    let key = Operation::Split(Rc::clone(&node)).key();
                    self.queued.remove(&key);
                    if !live_ptrs.contains(&(Rc::as_ptr(&node) as usize))
                        || node.borrow().level >= self.config.max_level
                    {
                        continue;
                    }
                    // Restricted subdivision: redirect to the coarsest
                    // blocker so every executed split is legal.
                    let target = split_target(&node);
                    let center = split_node_local(&target);
                    // Keep the traversal roots on live nodes.
                    for root in &mut self.roots {
                        if Rc::ptr_eq(root, &target) {
                            *root = Rc::clone(&center);
                        }
                    }
                    operations += 1;
                    report.splits.push(center);
                    if !Rc::ptr_eq(&target, &node) {
                        // Forced neighbor split: the original request goes
                        // back into the queue.
                        self.enqueue(Operation::Split(node));
                    }
                }
                Operation::Merge(base) => {
                    self.queued.remove(&format!("M:{base}"));
                    let Some(center) = find_group_center(&live, &base) else {
                        continue;
                    };
                    if !merge_eligible(center) {
                        // A finer group blocks the merge; the evaluation
                        // pass re-queues it while the distance holds.
                        continue;
                    }
                    let parent = unsplit_node(center);
                    // Keep the traversal roots on live nodes.
                    for root in &mut self.roots {
                        if Rc::ptr_eq(root, center) {
                            *root = Rc::clone(&parent);
                        }
                    }
                    operations += 1;
                    report.merges.push(parent);
                }
            }
        }

        // 2. Queue newly threshold-crossing chunks. Merge candidates are
        //    queued deepest-level first so blocking finer groups merge
        //    before the coarser groups they touch. The live set is
        //    re-collected after the drain: executed operations retired
        //    nodes of the previous one.
        let live = self.live_nodes();
        let mut merge_candidates: Vec<(u32, String)> = Vec::new();
        for node in &live {
            let node_ref = node.borrow();
            let level = node_ref.level;
            let distance = player_position.distance(node_ref.center);
            if level < self.config.max_level && distance < self.config.split_threshold(level) {
                let key = format!("S:{}", node_ref.name);
                if !self.queued.contains(&key) {
                    drop(node_ref);
                    self.enqueue(Operation::Split(Rc::clone(node)));
                    continue;
                }
            }
            if level >= 1
                && let Some(base) = group_base(&node_ref.name)
            {
                let parent_center = group_parent_center(node);
                if player_position.distance(parent_center) > self.config.merge_threshold(level - 1)
                {
                    merge_candidates.push((level, base));
                }
            }
        }
        merge_candidates.sort_by_key(|(level, _)| std::cmp::Reverse(*level));
        for (_, base) in merge_candidates {
            if !self.queued.contains(&format!("M:{base}")) {
                self.enqueue(Operation::Merge(base));
            }
        }

        // 3. Recompute the active zone.
        let previous: HashSet<usize> = self.active.iter().map(|n| Rc::as_ptr(n) as usize).collect();
        let mut by_distance: Vec<(f32, NodeRef)> = live
            .iter()
            .map(|node| {
                (
                    player_position.distance(node.borrow().center),
                    Rc::clone(node),
                )
            })
            .collect();
        by_distance.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut active: Vec<NodeRef> = by_distance
            .iter()
            .filter(|(distance, _)| *distance < self.config.active_distance)
            .map(|(_, node)| Rc::clone(node))
            .collect();
        // Maintain the minimum number of active meshes with the nearest
        // outside chunks.
        for (_, node) in &by_distance {
            if active.len() >= self.config.min_active_meshes {
                break;
            }
            if !active.iter().any(|n| Rc::ptr_eq(n, node)) {
                active.push(Rc::clone(node));
            }
        }
        let current: HashSet<usize> = active.iter().map(|n| Rc::as_ptr(n) as usize).collect();
        report.loaded = active
            .iter()
            .filter(|n| !previous.contains(&(Rc::as_ptr(n) as usize)))
            .cloned()
            .collect();
        report.unloaded = self
            .active
            .iter()
            .filter(|n| !current.contains(&(Rc::as_ptr(n) as usize)))
            .cloned()
            .collect();
        self.active = active;

        report
    }

    /// Every node of the connected components reachable from the roots.
    fn live_nodes(&self) -> Vec<NodeRef> {
        let mut seen = HashSet::new();
        let mut live = Vec::new();
        for root in &self.roots {
            for node in collect_nodes(root) {
                if seen.insert(Rc::as_ptr(&node) as usize) {
                    live.push(node);
                }
            }
        }
        live
    }

    /// Appends an operation to the queue, recording its dedup key.
    fn enqueue(&mut self, operation: Operation) {
        self.queued.insert(operation.key());
        self.queue.push_back(operation);
    }
}

/// Resolves the node to actually split for a `Split(node)` request: `node`
/// itself when every linked neighbor is at least at its level, otherwise
/// the coarsest blocker reached by repeatedly descending to a lower-level
/// neighbor. The descent terminates because levels strictly decrease, and
/// the returned node is always legal to split under restricted subdivision.
fn split_target(node: &NodeRef) -> NodeRef {
    let mut current = Rc::clone(node);
    loop {
        let blocker = {
            let node_ref = current.borrow();
            let level = node_ref.level;
            node_ref
                .children
                .iter()
                .flatten()
                .find(|neighbor| neighbor.borrow().level < level)
                .cloned()
        };
        match blocker {
            Some(neighbor) => current = neighbor,
            None => return current,
        }
    }
}

/// Base name of the split group `name` (ending in `.C`) belongs to, or
/// `None` for non-center nodes.
fn group_base(name: &str) -> Option<String> {
    let (base, suffix) = name.rsplit_once('.')?;
    (suffix == "C").then(|| base.to_string())
}

/// Finds the live center node of the split group named `base`.
fn find_group_center<'a>(live: &'a [NodeRef], base: &str) -> Option<&'a NodeRef> {
    let center_name = format!("{base}.C");
    live.iter().find(|node| node.borrow().name == center_name)
}

/// Centroid of a split group's parent triangle: the corners hold the
/// parent's `A` (`I.vertices[0]`), `B` (`J.vertices[1]`), and `C`
/// (`K.vertices[2]`) vertices exactly.
fn group_parent_center(center: &NodeRef) -> Vec3 {
    let center_ref = center.borrow();
    let node_i = center_ref.children[1].as_ref().expect("group corner I");
    let node_j = center_ref.children[0].as_ref().expect("group corner J");
    let node_k = center_ref.children[2].as_ref().expect("group corner K");
    (node_i.borrow().vertices[0] + node_j.borrow().vertices[1] + node_k.borrow().vertices[2]) / 3.0
}

/// Whether a split group may merge under restricted subdivision: every node
/// linked to the group from outside must be at most at the group level, so
/// the level difference across the shared edges stays at most 1 after the
/// merge.
fn merge_eligible(center: &NodeRef) -> bool {
    let group_level = center.borrow().level;
    let group = [
        Rc::clone(center),
        Rc::clone(
            center.borrow().children[0]
                .as_ref()
                .expect("group corner J"),
        ),
        Rc::clone(
            center.borrow().children[1]
                .as_ref()
                .expect("group corner I"),
        ),
        Rc::clone(
            center.borrow().children[2]
                .as_ref()
                .expect("group corner K"),
        ),
    ];
    group.iter().all(|member| {
        let member_ref = member.borrow();
        member_ref.children.iter().flatten().all(|neighbor| {
            group.iter().any(|m| Rc::ptr_eq(m, neighbor)) || neighbor.borrow().level <= group_level
        })
    })
}
