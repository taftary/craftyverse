# 03 — Dual mesh -> hex/pent cells

- **Phase:** 1 — Geometry
- **Status:** Next
- **Depends on:** 01 — Spherical projection (02 recommended for quality)
- **Plan reference:** notion.md Sections 4.2 (item 3), 4.3 (scale table)

## Goal

Convert the triangle mesh into the dual cell mesh that all simulation layers
use: triangle centroids become cell vertices; each triangle vertex becomes a
cell. All cells are hexagons except the 12 pentagons at the original
icosahedron vertices — the same vertices the current North/South pentagon
bases encode.

## Design summary

- One cell per unique mesh vertex. Vertices shared by multiple triangles
  must be deduplicated (this is where coordinate dedup actually matters —
  notion.md Section 4.1).
- A cell's polygon is the ordered ring of centroids of the triangles around
  its vertex.
- Cell count follows the scale table: `10 * 4^n + 2` cells at split level
  `n` (level 5 -> 10,242 cells; LOD0 target).
- Exactly 12 pentagons at every level (the original icosahedron corners).
- Each cell stores its neighbor list (cells sharing an edge) — this is the
  adjacency every later pass (tectonics, flow, smoothing) iterates over.

## Implementation steps

1. Assign a stable cell index to every unique mesh vertex (dedup across the
   reciprocal-link graph, not by float comparison alone).
2. For each cell, collect the surrounding triangles in angular order around
   the vertex to build the polygon ring (centroids as corners).
3. Build per-cell neighbor lists from shared triangle edges.
4. Provide an iterator/collection over cells keyed by index (input to the
   SoA data model, feature 04).
5. Viewer: add a hex-cell rendering mode (cell outlines + pentagon
   highlight); document any new key in `README.md`.
6. Extend validation: assert cell counts per level against the scale table.

## Debug visualization

**Scenario:** "dual cells" (new digit key) — the cell counterpart of
today's node viewer:

- Cell polygons outlined; the 12 pentagons in a distinct highlight color;
  centroid dots; cell index labels (today's label style).
- Dashed neighbor links between cell centers, colored per neighbor slot
  (same vocabulary as today's child links), with checkbox toggles.
- Click a cell to select it: highlight its polygon and its neighbors, and
  show index, side count, and split level in an on-screen inspector.
- Live counters: total cells, pentagon count (must be 12), expected count
  `10 * 4^n + 2` from the scale table — mismatches visible at a glance.

## Acceptance criteria

- Debug screen shipped per the standard in `plans/features/README.md`;
  new keys/toggles documented in `README.md`.

- Cell count matches `10 * 4^n + 2` at levels 0-3 (unit test against the
  notion.md Section 4.3 table).
- Exactly 12 pentagons; all other cells are hexagons (unit test).
- Every cell's neighbor count equals its side count (5 or 6); neighbor
  relation is symmetric (unit test).
- `mesh-validator` exits 0; viewer shows the dual cells on the sphere.
- Standard definition of done passes; spec written under
  `docs/book/specs/`.
