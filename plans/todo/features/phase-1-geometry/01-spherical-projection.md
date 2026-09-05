# 01 — Spherical projection

- **Phase:** 1 — Geometry
- **Status:** Next
- **Depends on:** nothing (current code: `Node`)
- **Plan reference:** notion.md Sections 4.2 (item 1), 4.3

## Goal

Give every node a 3D position on a sphere of radius `R`. The current mesh
is a flat 2D net (`Vec2`) with icosahedron topology; after this feature each
node carries a position with `v = R * normalize(v3)`.

## Design summary

- Each of the 20 root nodes corresponds to one face of the icosahedron.
  Subdivided
  nodes inherit positions inside their root face (barycentric on the face),
  then every position is projected: `normalize(v3) * R`.
- **Open design choice (decide at implementation time, then update the
  specs):** extend `Node` to `Vec3` positions, or keep the 2D net and add a
  separate 3D embedding alongside it.
- Topology does not change: reciprocal `children[3]` links are unaffected;
  this feature only adds/moves coordinates.

## Implementation steps

1. Record the design choice (extend `Node` to `Vec3` vs. separate 3D
   embedding) in the feature PR and in the module spec.
2. Define the 20 icosahedron face vertex positions in 3D (constant table).
3. Map each root node to its face; compute subdivided-node positions
   barycentrically from the parent split geometry.
4. Project all positions onto the sphere: `pos = R * normalize(pos)`.
5. Update `scene`/`render` to draw the spherical mesh.
6. Update the node spec (`docs/book/specs/node.md`) if `Node` fields changed.

## Debug visualization

**Scenario:** "spherical projection" (new digit key). The full per-node
overlay set from today's viewer — triangle outline, I/J/K direction arrows,
`direction_of_node`, origin arrow, child links, open-port markers, center
dot, labels — keeps working, now drawn on the 3D sphere. Beyond today's
baseline:

- Projection-error coloring: outline/center dot colored by `|pos| - R`
  deviation (should be ~0; makes projection bugs immediately visible).
- Flat-net <-> sphere toggle to compare before/after projection.
- On-screen readout: node count, split level, radius `R`, max deviation.
- Checkbox toggles for the new overlays, like today's `DisplayOptions`.

## Acceptance criteria

- Debug screen shipped per the standard in `plans/features/README.md`;
  new keys/toggles documented in `README.md`.

- Every node position satisfies `|pos| == R` within float tolerance (unit
  test).
- Link topology remains reciprocal and connected at split levels 0-3.
- Viewer renders a sphere wireframe; documented controls still accurate.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features
  -- -D warnings`, `cargo test --workspace --all-targets` pass.
- Specs under `docs/book/specs/` describe the new positions; the open design
  choice is resolved and recorded.
