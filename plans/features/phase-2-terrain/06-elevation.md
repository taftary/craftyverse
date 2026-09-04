# 06 — Elevation and sea level

- **Phase:** 2 — Terrain
- **Status:** Planned
- **Depends on:** 05 — Tectonics
- **Plan reference:** notion.md Sections 7, 15 (parameters)

## Goal

Refine the tectonic base elevation with multi-scale FBM noise and fix the
sea level, producing the final land/ocean height field for LOD0.

## Design summary

- **FBM noise, three scales** — large: continents; mid: mountain-range
  shaping; small: hills/detail. 4-5 octaves, persistence 0.5:

  ```text
  FBM(p) = sum_i  persistence^i * amplitude * noise(2^i * frequency * p)
  ```

- **Combine with the tectonic base:**

  ```text
  e = e_tect + w1*n_large + w2*n_mid + w3*n_small
  ```

- **Mountains** — base from convergent boundaries, refined by mid-scale
  noise; optional ridge lines along boundaries.
- **Sea level** — normalize elevation, mark cells below threshold as ocean.
- **Smoothing** — 1-2 neighbor-averaging passes only. No Gaussian/FFT
  isostasy (deferred, notion.md Appendix).

## Implementation steps

1. Seeded 3D noise source sampled at cell positions on the sphere (noise
   offsets derived from the world seed). No new dependency without a stated
   need — check existing crates first.
2. Three FBM layers with parameters from notion.md Section 15 (octaves 4-5,
   persistence 0.5).
3. Combine with `e_tect` using weights `w1..w3`; keep the result in the
   normalized `u16` range.
4. Mountain refinement: scale the mid layer near convergent boundaries.
5. Normalize, apply sea-level threshold, produce the ocean mask.
6. 1 neighbor-averaging smoothing pass (range 1-2).
7. Viewer: elevation ramp + ocean coloring mode; document any new key in
   `README.md`.

## Debug visualization

**Scenario:** "elevation" (new digit key). Height-ramp coloring with the
ocean mask, beyond today's baseline:

- Contour-line overlay (iso-lines at fixed elevation steps), toggleable.
- Live sea-level control (keys/slider): raising/lowering the threshold
  updates the ocean mask in place.
- Hover a cell: normalized elevation plus the raw `u16` value in the
  inspector.
- Elevation histogram panel, to sanity-check normalization and the
  land/ocean ratio.
- Smoothing before/after toggle; FBM layers (large/mid/small) viewable
  individually as grayscale underlays.

## Acceptance criteria

- Debug screen shipped per the standard in `plans/features/README.md`;
  new keys/toggles documented in `README.md`.

- Same seed -> identical final `elevation` array (determinism test).
- Ocean fraction is stable and tunable via the sea-level threshold (unit
  test on a fixed seed).
- Elevation stays in the normalized `u16` range after all passes.
- Smoothing is the only neighborhood pass and runs O(n).
- Standard definition of done passes; spec written under
  `docs/book/specs/`; `AGENTS.md`/README controls updated if keys change.
