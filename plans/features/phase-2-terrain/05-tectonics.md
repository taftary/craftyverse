# 05 — Tectonics

- **Phase:** 2 — Terrain
- **Status:** Planned
- **Depends on:** 04 — Data model
- **Plan reference:** notion.md Sections 6, 15 (parameters)

## Goal

Assign every cell to a tectonic plate, classify plate boundaries, and lay
down base elevation: continental plates high, oceanic plates low, mountains
at convergent boundaries, rifts at divergent ones.

## Design summary

1. **Plate seeding** — pick `P` cells as seeds (range 8-20, default 10),
   blue-noise sampling to avoid clustering; assign every cell a `plate_id`
   by flood fill / nearest-seed (Voronoi on the sphere).
2. **Plate attributes** — type (continental/oceanic), 2D motion vector.
   Density/thickness/age only if Tier 3 ever needs them.
3. **Boundary classification** — for each neighbor pair with different
   `plate_id`:

   ```text
   v_rel = v_A - v_B
   v_rel . n >  threshold  ->  convergent  (mountains, subduction)
   v_rel . n < -threshold  ->  divergent   (rifts, ridges)
   otherwise               ->  transform
   ```

   (`n` = boundary normal between the two cells.)
4. **Base elevation** — continental plates get a higher baseline, oceanic a
   lower one; mountains near convergent boundaries, rifts near divergent
   ones.
5. **Optional uplift diffusion** — single weighted-neighbor-average pass,
   O(n), 1-3 passes, `lambda` 0.1-0.5 (default 0.3):

   ```text
   U(t+1) = U(t) + lambda * sum_neighbors (U_n - U(t))
   ```

## Implementation steps

1. Blue-noise seed selection on the cell graph (seeded, deterministic).
2. Voronoi assignment: multi-source flood fill from seeds -> `plate_id`.
3. Plate table: type + motion vector per plate, derived from the world seed.
4. Boundary pass: classify every cross-plate neighbor pair (convergent /
   divergent / transform); store a boundary field or per-cell flags.
5. Base elevation into `elevation` (`u16` normalized range), including
   boundary mountain/rift terms.
6. Optional uplift diffusion pass (1-2 iterations, default `lambda` 0.3).
7. Viewer: plate-id coloring mode; document any new key in `README.md`.

## Debug visualization

**Scenario:** "tectonics" (new digit key). Cells filled by `plate_id`
color, with today's overlay vocabulary mapped onto plates:

- Seed markers: one disc per plate seed (same marker style as today's
  open-port markers).
- Boundary edges colored by type — convergent / divergent / transform in
  three distinct colors — each type toggleable from the checkbox panel.
- Plate motion arrows: one arrow per plate at its centroid (today's
  direction-arrow style); clicking a boundary shows the relative-velocity
  arrows `v_A - v_B` and the boundary normal `n` for that pair.
- Uplift field as a grayscale underlay toggle beneath the plate colors.
- Cell inspector on click: `plate_id`, plate type, motion vector, boundary
  classification of each edge.

## Acceptance criteria

- Debug screen shipped per the standard in `plans/features/README.md`;
  new keys/toggles documented in `README.md`.

- Every cell has a `plate_id`; plate count == configured `P` (unit test).
- Regenerating with the same seed yields identical `plate_id` and
  `elevation` arrays (determinism test).
- Boundary classification agrees with the sign of `v_rel . n` on synthetic
  two-plate fixtures (unit test).
- All passes are O(n) over cells (no per-cell global searches).
- `mesh-validator` exits 0; standard definition of done passes; spec
  written under `docs/book/specs/`.
