//! The LOD scheduler: distance-based split and merge decisions over the
//! node graph, within a bounded per-frame operation budget.

use std::collections::{HashSet, VecDeque};
use std::rc::Rc;

use glam::Vec3;

use super::config::LodConfig;
use crate::node::{NodeRef, collect_nodes, split_group_members, split_node_local, unsplit_node};

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
/// never participates); [`update_with_camera`](Self::update_with_camera)
/// refines against both the player and the camera while loading stays
/// rooted at the player. Both drain at most
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
///
/// The level floor ([`min_level`](LodConfig::min_level)) keeps every chunk
/// refined to at least that level, anywhere on the planet and regardless of
/// player distance: chunks below the floor are queued for splits without a
/// threshold check, and groups at the floor never merge.
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

    /// Sets the level floor ([`min_level`](LodConfig::min_level)) live.
    ///
    /// The next [`update`](Self::update) picks the new floor up: raising it
    /// queues the mandatory floor splits, lowering it re-enables merges
    /// that the old floor blocked. Queued operations that no longer fit the
    /// floor are dropped at execution time.
    ///
    /// # Errors
    ///
    /// Returns [`LodConfigError::InvalidMinLevel`](crate::lod::LodConfigError::InvalidMinLevel)
    /// when `min_level` exceeds [`max_level`](LodConfig::max_level); the
    /// floor is left unchanged.
    pub fn set_min_level(&mut self, min_level: u32) -> Result<(), crate::lod::LodConfigError> {
        if min_level > self.config.max_level {
            return Err(crate::lod::LodConfigError::InvalidMinLevel);
        }
        self.config.min_level = min_level;
        Ok(())
    }

    /// Sets the active-zone radius ([`active_distance`](LodConfig::active_distance))
    /// live.
    ///
    /// The next [`update`](Self::update) picks the new radius up: growing
    /// it loads newly covered chunks, shrinking it unloads the chunks left
    /// outside (the global coarse shell at or below `min_level` stays
    /// loaded regardless). Used by the runtime window to shrink the zone
    /// with altitude (feature 8).
    ///
    /// # Errors
    ///
    /// Returns [`LodConfigError::InvalidActiveDistance`](crate::lod::LodConfigError::InvalidActiveDistance)
    /// when `distance` is not positive and finite; the radius is left
    /// unchanged.
    pub fn set_active_distance(&mut self, distance: f32) -> Result<(), crate::lod::LodConfigError> {
        if !distance.is_finite() || distance <= 0.0 {
            return Err(crate::lod::LodConfigError::InvalidActiveDistance);
        }
        self.config.active_distance = distance;
        Ok(())
    }

    /// The current active chunk set: the loaded chunks inside the active
    /// zone and the global coarse shell, plus the nearest outside chunks
    /// up to [`min_active_meshes`](LodConfig::min_active_meshes).
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
    /// Single-reference form of [`update_with_camera`](Self::update_with_camera):
    /// the player position drives both loading and refinement.
    ///
    /// # Panics
    ///
    /// Panics on a non-finite player position (internal invariant).
    pub fn update(&mut self, player_position: Vec3) -> FrameReport {
        self.update_with_camera(player_position, player_position)
    }

    /// The per-frame update with separate loading and refinement
    /// references (feature 7, Decision 7 of `plan/RELATED.md`).
    ///
    /// Loading stays rooted at the player (the active-zone sphere), while
    /// refinement tests both viewpoints: splits use the nearer distance
    /// against the split threshold (either viewpoint pulls detail in) and
    /// merges use the nearer distance against the merge threshold (both
    /// viewpoints must be far to coarsen).
    /// The active set is the union of the player zone and the global
    /// coarse shell (every live chunk at or below `min_level`), ordered
    /// by hybrid distance for pool contention.
    ///
    /// # Panics
    ///
    /// Panics on a non-finite player or camera position (internal
    /// invariant).
    pub fn update_with_camera(
        &mut self,
        player_position: Vec3,
        camera_position: Vec3,
    ) -> FrameReport {
        assert!(
            player_position.is_finite(),
            "player position must be finite"
        );
        assert!(
            camera_position.is_finite(),
            "camera position must be finite"
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
                    let node_ref = node.borrow();
                    let level = node_ref.level;
                    let stale = !live_ptrs.contains(&(Rc::as_ptr(&node) as usize))
                        || level >= self.config.max_level
                        // Below the level floor the split is mandatory;
                        // above it, re-validate the threshold against both
                        // viewpoints: either may have moved away since the
                        // request was queued.
                        || (level >= self.config.min_level
                            && split_distance(player_position, camera_position, node_ref.center)
                                >= self.config.split_threshold(level));
                    drop(node_ref);
                    if stale {
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
                    // Names repeat across generations (a group can merge and
                    // re-split under the same base name): validate the
                    // resolved group's atomicity, level, and threshold
                    // against the live node, not the queued name.
                    let Some(members) = split_group_members(center) else {
                        continue;
                    };
                    let group_level = members[3].borrow().level;
                    if group_level <= self.config.min_level
                        || split_distance(
                            player_position,
                            camera_position,
                            group_parent_center(&members),
                        ) <= self.config.merge_threshold(group_level - 1)
                    {
                        continue;
                    }
                    if !merge_eligible(&members) {
                        // A finer group blocks the merge; the evaluation
                        // pass re-queues it while the distance holds.
                        continue;
                    }
                    let Some(parent) = unsplit_node(center) else {
                        continue;
                    };
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
            let split_d = split_distance(player_position, camera_position, node_ref.center);
            if level < self.config.max_level
                && (level < self.config.min_level || split_d < self.config.split_threshold(level))
            {
                let key = format!("S:{}", node_ref.name);
                if !self.queued.contains(&key) {
                    drop(node_ref);
                    self.enqueue(Operation::Split(Rc::clone(node)));
                    continue;
                }
            }
            if level > self.config.min_level
                && let Some(base) = group_base(&node_ref.name)
            {
                // Only complete, atomic groups are merge candidates: a
                // center whose corner was split further has its port
                // retargeted to a grandchild and is skipped here.
                if let Some(members) = split_group_members(node) {
                    let parent_center = group_parent_center(&members);
                    if split_distance(player_position, camera_position, parent_center)
                        > self.config.merge_threshold(level - 1)
                    {
                        merge_candidates.push((level, base));
                    }
                }
            }
        }
        merge_candidates.sort_by_key(|(level, _)| std::cmp::Reverse(*level));
        for (_, base) in merge_candidates {
            if !self.queued.contains(&format!("M:{base}")) {
                self.enqueue(Operation::Merge(base));
            }
        }

        // 3. Recompute the active zone: the union of the player sphere
        //    and the global coarse shell (every live chunk at or below
        //    the level floor), ordered by hybrid distance so the pool
        //    assigns the most visible chunks first under contention.
        let previous: HashSet<usize> = self.active.iter().map(|n| Rc::as_ptr(n) as usize).collect();
        let mut by_distance: Vec<(f32, f32, NodeRef)> = live
            .iter()
            .map(|node| {
                let node_ref = node.borrow();
                let d_player = player_position.distance(node_ref.center);
                let hybrid = split_distance(player_position, camera_position, node_ref.center);
                drop(node_ref);
                (hybrid, d_player, Rc::clone(node))
            })
            .collect();
        by_distance.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut active: Vec<NodeRef> = by_distance
            .iter()
            .filter(|(_, d_player, node)| {
                *d_player < self.config.active_distance
                    || node.borrow().level <= self.config.min_level
            })
            .map(|(_, _, node)| Rc::clone(node))
            .collect();
        // Maintain the minimum number of active meshes with the nearest
        // outside chunks (hybrid order).
        for (_, _, node) in &by_distance {
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

/// Hybrid distance (feature 7): the nearer of the player and the camera
/// to `point`. Either viewpoint pulls detail in, and both must be far to
/// coarsen: splits test it against the split threshold, merges against
/// the (higher) merge threshold, so the hysteresis band stays stable.
fn split_distance(player_position: Vec3, camera_position: Vec3, point: Vec3) -> f32 {
    player_position
        .distance(point)
        .min(camera_position.distance(point))
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
fn group_parent_center(members: &[NodeRef; 4]) -> Vec3 {
    (members[0].borrow().vertices[0]
        + members[1].borrow().vertices[1]
        + members[2].borrow().vertices[2])
        / 3.0
}

/// Whether a split group may merge under restricted subdivision: every node
/// linked to the group from outside must be at most at the group level, so
/// the level difference across the shared edges stays at most 1 after the
/// merge.
fn merge_eligible(members: &[NodeRef; 4]) -> bool {
    let group_level = members[3].borrow().level;
    members.iter().all(|member| {
        let member_ref = member.borrow();
        member_ref.children.iter().flatten().all(|neighbor| {
            members.iter().any(|m| Rc::ptr_eq(m, neighbor))
                || neighbor.borrow().level <= group_level
        })
    })
}
