## LOD Specification (`crates/engine/src/lod/`)

### Overview

The `lod` module is the headless chunk LOD scheduler (feature 2 of the
procedural planet runtime, `plan/features/02-lod-scheduler.md`). It is
node-graph only - no GPU, window, or render dependency - and decides which
chunks split, merge, load, and unload, within a hard per-frame operation
budget. It covers all zoom behavior; there is no separate zoom system.

Status: **Current baseline** for the headless scheduler and the local
refinement operations, for the render-side consumption of the
`FrameReport` by the mesh pool (feature 3, see the [mesh pool
specification](mesh-pool.md)), and for frustum and horizon culling
(feature 4, see the [visibility specification](visibility.md)). The
shader-side terrain morph that widens the use
of the skirt metadata (feature 5) is **Planned**; the pool generates the
border skirts in the vertex data today. The hybrid camera-aware update
with the global coarse shell (feature 7,
`plan/features/07-camera-aware-lod.md`) and the altitude-driven active
zone with deep near-field LOD (feature 8,
`plan/features/08-terrain-ground-scale.md`) are **Current baseline**.

The scheduler consumes player position for loading in every mode; the
hybrid update additionally consumes the draw camera position for
refinement. Player orientation never participates in any decision.

### Structure

- **Configuration** (`config.rs`)
  - `LodConfig { base_split_distance, hysteresis_ratio, max_level, min_level, operations_per_frame, active_distance, min_active_meshes }` -
    one planet's LOD thresholds, level bounds, operation budget, and active
    zone, in world units.
  - `LodConfig::split_threshold(level) = base_split_distance / 2^level` -
    the geometric progression (x2 per level), matching the subdivision
    hierarchy where each triangle refines into four children.
  - `LodConfig::merge_threshold(level) = split_threshold(level) * hysteresis_ratio` -
    the hysteresis band (ratio 1.3 by default).
  - `LodConfig::validate() -> Result<(), LodConfigError>` - checks the
    field invariants; `LodConfigError` is a typed error implementing
    `std::error::Error`, one variant per violated invariant.
  - `LodConfig::default()` - base split distance 1000, hysteresis ratio
    1.3, max level 8, level floor 0, 2 operations per frame, active distance
    2000, minimum 20 active meshes.
- **Scheduler** (`scheduler.rs`)
  - `LodScheduler::new(config, roots) -> Result<Self, LodConfigError>` -
    creates a scheduler over the mesh reachable from `roots` (typically the
    faces of an `IcosphereMesh`). The roots are kept on live nodes: when a
    root node is split or its group merged, the root entry follows the
    replacement.
  - `LodScheduler::update(player_position) -> FrameReport` - the per-frame
    update; panics on a non-finite position (internal invariant).
    Single-reference form of `update_with_camera`.
  - `LodScheduler::update_with_camera(player_position, camera_position) -> FrameReport` -
    the hybrid per-frame update (feature 7): loading stays rooted at the
    player while refinement tests the nearer of the two positions;
    panics on a non-finite position.
  - `LodScheduler::set_active_distance(distance) -> Result<(), LodConfigError>` -
    adjusts the active-zone radius live (the runtime window drives it from
    the flatten factor, feature 8); rejects a non-positive or non-finite
    radius.
  - `LodScheduler::active_chunks() -> &[NodeRef]` - the current active
    chunk set.
  - `LodScheduler::set_min_level(min_level) -> Result<(), LodConfigError>` -
    adjusts the level floor live (the runtime window's Up/Down arrow keys);
    the next update queues the mandatory floor splits or re-enables the
    blocked merges. Rejects a floor above `max_level`.
  - `LodScheduler::queued_operations() -> usize` - queued split/merge
    operations waiting for budget.
  - `FrameReport { splits, merges, loaded, unloaded }` - what one update
    changed: the new split centers, the recovered merge parents, and the
    active-zone membership changes. This is the hook for the render side
    (feature 3).
  - `border_states(node) -> [BorderState; 3]` - per-port border
    classification for the skirt metadata: `Open` (no neighbor, or the open
    half-edge of a T-junction), `Welded` (same-level neighbor, watertight),
    `Coarser` (this chunk is the finer side of a T-junction and needs a
    skirt), `Finer` (this chunk is the coarser side).

### update() Specification

One frame, in order:

1. **Drain the queue within the budget.** At most `operations_per_frame`
   operations execute; each executed split or merge - forced neighbor
   splits included - consumes exactly one operation. Popped operations are
   revalidated at execution time and dropped when stale: a split is dropped
   when its node is no longer live, reached `max_level`, or - above the
   `min_level` floor - is no longer below its split threshold; a merge is
   dropped when its base name no longer resolves to a live center, when the
   resolved group is not complete and atomic (`split_group_members` - names
   repeat across generations after a merge and re-split, so the name alone
   is never enough), when the group sits at or below the floor or is no
   longer beyond its merge threshold, or when a finer group blocks it
   (re-evaluated on later frames).
2. **Queue newly threshold-crossing chunks.** A chunk at level `L` closer
   than `split_threshold(L)` is queued for a split, as is any chunk below
   the `min_level` floor regardless of distance; a complete, atomic
   split group above the floor whose parent center is farther than
   `merge_threshold(L)` is queued for a merge. Queued work is never dropped
   on arrival: excess work waits in the queue and drains over the following
   frames. Merge candidates are queued deepest-level first so blocking
   finer groups merge before the coarser groups they touch.
3. **Recompute the active zone.** The active set is the union of the
   player sphere (chunks closer than `active_distance` to the player) and
   the global coarse shell (every live chunk at or below `min_level`),
   ordered by hybrid distance; the rest are unloaded. When fewer than
   `min_active_meshes` chunks are inside the union, the nearest outside
   chunks (hybrid order) load to reach the minimum.

### Rules

- **Metric.** Loading decisions use the distance from the player position
  to the chunk center (`Node::center`); refinement tests the nearer of
  the player and the camera against the split threshold (either pulls
  detail in) and against the higher merge threshold (both must be far to
  coarsen); merges use the parent triangle's
  centroid, recovered from the group corners.
- **Hysteresis.** Splits and merges use strict comparisons against separate
  thresholds (`distance < split_threshold`, `distance > merge_threshold`).
  Between the two thresholds the state is stable: a player hovering at or
  near a threshold causes no split/merge oscillation.
- **Level floor.** `min_level` keeps the whole mesh refined to at least
  that level, anywhere on the planet and regardless of player distance:
  chunks below the floor split without a threshold check, and groups at
  the floor never merge. The runtime window uses a level-1 floor so the
  planet always reads as a sphere, never as the raw icosahedron.
- **Restricted subdivision.** The level difference across any shared edge
  is at most 1. A split whose neighbors are too coarse is redirected to the
  coarsest blocker: the scheduler splits that neighbor first (a forced
  neighbor split, counted against the same budget) and re-queues the
  original request. A merge whose group touches a finer group is dropped
  and re-evaluated on later frames.
- **Topology changes.** Splits go through `split_node_local` and merges
  through `unsplit_node` (see the [subdivision specification](subdivision.md)),
  so the node-graph invariants hold after every frame: reciprocal links,
  watertight shared edges between equal-level neighbors, exact merge
  recovery, and explicit cleanup of retired generations (retired nodes are
  left unlinked or destroyed; no `Rc` cycle leaks).
- **Active zone.** The zone is a full sphere around the player position;
  chunks load in all directions, including behind the camera. The global
  coarse shell (every live chunk at or below `min_level`) is always
  active on top, so deep space shows a closed planet. Loading and
  unloading only change active-set membership - the node graph itself stays
  resident so neighbor links remain intact. The runtime window shrinks the
  radius with altitude (0.75 radii in orbit to 0.05 at the surface,
  feature 8) and refines the near field to level 7 within a budget of 8
  operations per frame (feature 7).
- **Crack masking.** With the level difference bounded, the residual
  T-junction seams along chunk borders are metadata (`border_states`) for
  the skirt shader of the runtime window; no runtime cross-chunk vertex
  stitching is attempted.

### Files

- `crates/engine/src/lod/config.rs` - `LodConfig` and `LodConfigError`.
- `crates/engine/src/lod/scheduler.rs` - `LodScheduler`, `FrameReport`,
  `BorderState`, and `border_states`.
