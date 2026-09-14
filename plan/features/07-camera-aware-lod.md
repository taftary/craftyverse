# Feature 7: Camera-Aware LOD

Phase 7 of the planet runtime. Fixes the far-away incomplete planet and
relates refinement to what the camera sees, while keeping loading rooted
at the player and culling rooted at the player camera.

## Goal

From space and orbit the planet renders as a closed sphere at the level
floor; near the player the mesh refines where either the player or the
draw camera is close. No more 20-chunk partial planet from deep space.

## Specification sources

- `plan/NOTION.md`: Chunk LOD Runtime System, Dynamic Mesh Loading
  Requirement, LOD Zoom Transitions, Performance Requirements.
- `plan/RELATED.md`: Decision 1 (player LOD, camera culling), Decision 2
  (geometric thresholds, vertex-only transitions), Decision 4 (operation
  budget), Decision 5 (local refinement, restricted subdivision, border
  skirts), Decision 7 (hybrid metric, global shell).

## Scope

- Hybrid distance metric: splits test the nearer of the player and the
  draw camera against `split_threshold(L)` (either viewpoint pulls
  detail in); merges test the nearer distance against
  `merge_threshold(L)` (both viewpoints must be far to coarsen). Same
  1.3x hysteresis band, same strict comparisons, so hovering causes no
  oscillation.
- Global coarse shell: every live chunk at or below `min_level` is always
  active, regardless of `active_distance`. With the runtime window's
  level-1 floor this keeps the full 80-chunk shell loaded from any
  distance, closing the sphere.
- Near field: chunks within `active_distance` of the player stay active as
  before; the active set is the union of the shell and the near field,
  plus the nearest outside chunks up to `min_active_meshes`.
- Slot pressure ordering: the active set is ordered by hybrid distance so
  the mesh pool assigns the most visible chunks first when slots contend.
- Per-frame budget: 8 split/merge operations per frame in the runtime
  window (forced neighbor splits included); excess work queues and drains.
- Culling unchanged: `culling_camera` still returns the player camera in
  every mode (Decision 1); navigation-mode gaps stay intentional.
- Overlay: camera distance plus coarse-shell vs near-field chunk counts.

## Constraints

- `update(player)` keeps working and delegates to
  `update_with_camera(player, player)`; existing thresholds, restricted
  subdivision, and node invariants are unchanged.
- Render resources stay pooled; no new GPU mesh objects at runtime.
- Pool capacity stays 1024 slots; contention queues, never grows.

## Acceptance criteria

- From beyond every split threshold the active set contains the full
  coarse shell (closed sphere, no missing limb).
- A scripted space-to-ground path refines toward `max_level` near the
  player/camera and merges back on retreat, within budget every frame.
- Hysteresis-band hovering causes no split/merge oscillation for either
  reference point.
- Headless tests cover the shell, the hybrid split/merge, and the queue
  draining; the node test suite still passes.

## Out of scope

- Ground density targets and altitude-driven zone sizing (feature 8).
- Frustum/horizon culling changes (none).
- Atmosphere rendering changes (none).
