# Feature 4: Visibility

Phase 4 of the planet runtime. Camera-driven culling over the active
chunks. Strictly separate from LOD: LOD follows the player, visibility
follows the camera.

## Goal

Render the minimum number of meshes: cull everything the camera cannot
see, including everything hidden by the planet itself.

## Specification sources

- `plan/NOTION.md`: Chunk Visibility Requirement, Performance
  Requirements.
- `plan/RELATED.md`: Decision 1 (culling consumes the camera only;
  detached camera sees player LOD); "Geometry and Planet-Side Culling
  Inputs" for the reusable node values.

## Scope

- Frustum culling against the camera view volume.
- Horizon culling against the planet: chunks fully behind the planet
  limb relative to the camera are discarded, using `Node::center`,
  `Node::vertices`, and `direction_to_origin`.
- Recalculation whenever the camera moves.
- Detached camera support: culling uses the camera even when it leaves
  the player; chunks beyond the player's active zone simply are not
  loaded, and the camera sees the gaps (accepted behavior).

## Constraints

- No influence on LOD or loading decisions; read-only consumer of the
  active chunk set.
- Must reduce rendered mesh count without popping artifacts at the
  horizon (conservative horizon test).

## Acceptance criteria

- Chunks behind the camera frustum are never drawn.
- Chunks fully occluded by the planet body are never drawn.
- No visible popping at the horizon line while the camera moves.
- Headless tests place cameras and assert the culled/visible sets.

## Out of scope

- Chunk loading and unloading (feature 2), slot management (feature 3),
  atmosphere shell rendering (feature 6).
