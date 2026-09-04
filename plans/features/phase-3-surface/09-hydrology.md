# 09 — Hydrology

- **Phase:** 3 — Surface
- **Status:** Planned
- **Depends on:** 06 — Elevation and sea level (07 recommended for moisture
  context)
- **Plan reference:** notion.md Sections 10, 15 (river threshold)

## Goal

Compute river networks and lakes from the elevation field, and carve river
channels into the elevation once, statically.

## Design summary

- **Flow direction** — each land cell flows to its lowest neighbor
  (steepest descent on the cell graph), stored in `flow_dir`.
- **Flow accumulation** — one pass in topological (downhill) order, O(n):

  ```text
  A(cell) = 1 + sum( w_ij * A(upstream_j) )
  ```

- **Rivers** — cells with `A > A_threshold` (tune per scale).
- **Lakes** — cells with no lower neighbor (local minima); lightweight
  priority-flood fill only where basins are visually significant, not
  globally at LOD0.
- **River carve** — a one-time static pass to etch river channels into the
  elevation. No iterative stream-power erosion solver (deferred, notion.md
  Appendix).

## Implementation steps

1. `flow_dir` pass: steepest descent over the neighbor index arrays;
   encode the downhill neighbor index in `u8`, with a sentinel for
   ocean/no-outflow cells.
2. Downhill ordering of cells (topological sort over `flow_dir`), O(n).
3. Flow accumulation pass into `flow_accum` in that order.
4. River mask: `flow_accum > A_threshold`; make the threshold a tunable
   parameter.
5. Lake detection: local minima; priority-flood fill for significant
   basins only.
6. River carve: lower elevation along river cells (one static pass), then
   recompute `flow_dir`/`flow_accum` if the carve changes drainage.
7. Viewer: river/lake overlay on the elevation view; document any new key
   in `README.md`.

## Debug visualization

**Scenario:** "hydrology" (new digit key). Flow made visible with today's
arrow vocabulary:

- `flow_dir` arrow per land cell (small, steepest-descent direction).
- River cells highlighted; line thickness or color intensity scales with
  `flow_accum`; `A_threshold` adjustable live to watch rivers appear/grow.
- Lake / local-minimum markers (disc style from today's open-port
  markers); priority-flood basins outlined.
- River carve before/after toggle; carve depth visible against the
  elevation underlay.
- Cell inspector on hover: `flow_dir` target index, `flow_accum`,
  river/lake flags.

## Acceptance criteria

- Debug screen shipped per the standard in `plans/features/README.md`;
  new keys/toggles documented in `README.md`.

- `flow_dir` always points to a strictly lower neighbor or the sentinel
  (unit test).
- Accumulation pass completes in one O(n) sweep over the downhill order
  (unit test with a known synthetic basin).
- Rivers connect downhill to the ocean or a lake on synthetic fixtures
  (unit test).
- Same seed -> identical `flow_dir`/`flow_accum` arrays.
- Standard definition of done passes; spec written under
  `docs/book/specs/`.
