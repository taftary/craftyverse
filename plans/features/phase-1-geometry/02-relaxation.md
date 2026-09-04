# 02 — Vertex relaxation

- **Phase:** 1 — Geometry
- **Status:** Next
- **Depends on:** 01 — Spherical projection
- **Plan reference:** notion.md Sections 4.2 (item 2), 15 (relaxation passes
  2-3 hybrid, default 2)

## Goal

Even out the cell size/shape distribution on the sphere with 2-3 hybrid
relaxation passes: a few Lloyd iterations plus sphere-constrained Laplacian
smoothing. Not naive Lloyd for 100+ iterations — it converges too slowly.

## Design summary

- **Lloyd iteration (spherical):** move each vertex toward the centroid of
  its Voronoi (dual) cell, then reproject onto the sphere
  (`normalize * R`).
- **Sphere-constrained Laplacian smoothing:** move each vertex toward the
  average of its neighbors, reproject to the sphere.
- Hybrid: alternate/combine both for 2 passes total (default; range 2-3).
- Needs vertex adjacency — the same neighbor structure feature 03 builds
  for dual cells; build adjacency first, or land 03's adjacency pass here
  and reuse it.

## Implementation steps

1. Build vertex adjacency from the node graph (neighbor lists per mesh
   vertex, deduped across shared edges).
2. Implement spherical Lloyd iteration (centroid of the surrounding cell,
   reproject).
3. Implement sphere-constrained Laplacian smoothing.
4. Compose the hybrid schedule (default 2 passes; parameter in the range
   2-3).
5. Add a uniformity metric (e.g. min/max edge length ratio or cell area
   variance) as a test oracle.
6. Viewer: optional before/after toggle for visual check (debug flag, not a
   new required control).

## Debug visualization

**Scenario:** "relaxation" (new digit key). Same per-node overlay baseline
as today, plus:

- Pass stepping: a key applies one relaxation pass at a time; before/after
  toggle to compare against the pre-relaxation mesh.
- Displacement arrows from each vertex's pre-pass position to its post-pass
  position (today's arrow vocabulary, reused).
- Live uniformity readout on screen: min/max edge-length ratio and
  cell-area variance, updated per pass.
- Checkbox toggles for the new overlays.

## Acceptance criteria

- Debug screen shipped per the standard in `plans/features/README.md`;
  new keys/toggles documented in `README.md`.

- Uniformity metric improves monotonically over the configured passes (unit
  test).
- All vertices remain at radius `R` after every pass (unit test).
- Deterministic: same mesh in -> same mesh out.
- Topology unchanged: `mesh-validator` exits 0.
- Standard definition of done passes (fmt, clippy, tests, spec updated).
