# 07 — Climate

- **Phase:** 3 — Surface
- **Status:** Planned
- **Depends on:** 06 — Elevation and sea level
- **Plan reference:** notion.md Section 8

## Goal

Fill `temperature` and `moisture` per cell from latitude, elevation, wind
bands, and a precipitation model with orographic rain shadows.

## Design summary

- **Temperature:**

  ```text
  T = T_equator - |lat| * delta_T - elevation * lapse_rate
  ```

- **Wind bands** — equatorial: east-west; subtropical: west-east;
  mid-latitude: mixed; polar: weak.
- **Precipitation** — moisture originates over oceans, travels along the
  wind, rains when forced upward by terrain (orographic effect -> rain
  shadows):

  ```text
  R_lat = R0 * exp(-alpha * (lat - lat_ITCZ)^2)   // latitude band (ITCZ)
  R_oro = k_oro * max(0, grad(h) . wind_dir)      // orographic term
  R     = R_lat + R_oro
  ```

## Implementation steps

1. Latitude per cell from its 3D position on the sphere.
2. Temperature pass into `temperature` (`u8`), from the formula above.
3. Wind-band classification per cell (band + local wind direction).
4. Precipitation pass: latitude band (ITCZ Gaussian) plus orographic term
   using elevation gradients along the wind; store in `moisture` (`u8`).
5. Keep every pass O(n) over cells (gradients from neighbor arrays only).
6. Viewer: temperature and moisture coloring modes; document any new key in
   `README.md`.

## Debug visualization

**Scenario:** "climate" (new digit key). Temperature and moisture color
modes (toggle between them), plus:

- Wind arrows per cell, colored by wind band — today's direction-arrow
  vocabulary at cell scale; latitude band guide rings as reference
  geometry.
- Orographic overlay for the hovered cell: local elevation gradient along
  the wind direction, with the resulting `R_oro` term shown.
- Cell inspector: latitude, `T`, lapse-rate term, `R_lat`, `R_oro`, `R`.
- Checkbox toggles per overlay (temperature, moisture, wind, bands).

## Acceptance criteria

- Debug screen shipped per the standard in `plans/features/README.md`;
  new keys/toggles documented in `README.md`.

- Temperature decreases with `|lat|` and with elevation on synthetic
  fixtures (unit test).
- Rain-shadow behavior: leeward cells behind a ridge are drier than
  windward cells on a synthetic fixture (unit test).
- Same seed -> identical `temperature`/`moisture` arrays.
- Standard definition of done passes; spec written under
  `docs/book/specs/`.
