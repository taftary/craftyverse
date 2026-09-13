# Feature 2: LOD Scheduler

Phase 2 of the planet runtime. Decides which chunks split, merge, load,
and unload, within a hard per-frame operation budget. Covers all zoom
behavior; there is no separate zoom system.

## Goal

Keep the node graph at the right detail level for the player's position:
refine chunks that get close, coarsen chunks that get far, and keep only
the active-zone chunks loaded.

## Specification sources

- `plan/NOTION.md`: Chunk LOD Runtime System, Dynamic Mesh Loading
  Requirement, LOD Zoom Transitions, Performance Requirements.
- `plan/RELATED.md`: Decision 1 (metric, hysteresis, active zone),
  Decision 2 (geometric thresholds, vertex-only transitions), Decision 4
  (operation budget), Decision 5 (local refinement, restricted
  subdivision, border skirts); "Subdivision and Merge Logic" and
  "Topology Utilities" for the reusable APIs.

## Scope

- LOD metric: distance from player position to chunk center
  (`Node::center`). Player orientation never participates.
- A new local refinement operation: split one chunk and retarget the
  neighboring links that pointed at the parent. The existing whole-graph
  `split_nodes` is a generation tool, not a runtime operation.
- Local merge built on the existing `unsplit_nodes` complete-group rules.
- Restricted subdivision: the level difference across any shared edge is
  at most 1; a violating split first splits the coarser neighbor, and
  forced neighbor splits count against the operation budget.
- Crack masking: short skirts (a downward flange at chunk borders)
  displaced by the terrain shader; no runtime cross-chunk stitching.
- Thresholds: geometric progression (x2 per level); hysteresis band:
  merge threshold = split threshold x 1.3 per level.
- Active zone: full sphere around the player position; chunks load when
  entering, unload when leaving, continuously and distance-based.
- A bounded per-frame operation queue: configurable budget, initially 2
  split/merge operations per frame; excess work is queued, not dropped.
- Minimum number of active meshes maintained at all times.

## Constraints

- Runtime topology changes go through the local refinement operation and
  local merge; render resources stay pooled (feature 3 owns the pool).
- Existing node tests are invariants: shared edges stay watertight,
  reciprocal links stay correct, merges recover original vertices and
  attributes, cleanup breaks all `Rc` cycles (`destroy_mesh` or
  equivalent for retired generations).

## Acceptance criteria

- A player hovering at a threshold causes no split/merge oscillation.
- The per-frame budget is never exceeded (forced neighbor splits
  included); queued work drains over following frames.
- After any sequence of transitions, the graph stays watertight, neighbor
  links stay retargeted, and the node test suite still passes.
- No shared edge ever has a level difference greater than 1; skirts hide
  the residual seams in the runtime window.
- Headless unit tests drive a scripted player path and assert the
  resulting level assignments.

## Out of scope

- GPU slot assignment and vertex uploads (feature 3).
- Frustum and horizon culling (feature 4).
