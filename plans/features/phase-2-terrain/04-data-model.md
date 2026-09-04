# 04 — Simulation data model (structure-of-arrays)

- **Phase:** 2 — Terrain
- **Status:** Planned
- **Depends on:** 03 — Dual mesh cells
- **Plan reference:** notion.md Sections 5, 3 (design principles)

## Goal

Introduce the simulation storage every later layer reads and writes:
structure-of-arrays keyed by cell index, one compact array per field, about
10 bytes per cell for the core maps.

## Design summary

```rust
elevation:   Vec<u16>,   // 0..=65535, normalized to sea level
temperature: Vec<u8>,
moisture:    Vec<u8>,
plate_id:    Vec<u16>,
biome:       Vec<u8>,
flow_dir:    Vec<u8>,    // neighbor index of the downhill flow
flow_accum:  Vec<u16>,
```

- The topology graph (`Node`/`NodeRef`, cells from feature 03) remains the
  skeleton; simulation passes operate only on these arrays.
- Neighbor lists are available directly from the graph or prebuilt into an
  index array per level (decide at implementation; record in spec).
- Store only fields consumed downstream; debug layers stay behind flags.
- Single world seed: plate seeds, noise offsets, and plate motion vectors
  all derive from it (deterministic generation, notion.md Section 3).

## Implementation steps

1. Define the field struct (e.g. `CellFields`) with the SoA arrays above;
   constructor takes a cell count.
2. Bridge cells -> indices: expose per-cell neighbor index lists from
   feature 03 (or prebuild a flat neighbor index array).
3. Define the world-seed type and the derivation scheme for sub-seeds
   (tectonics, noise, etc.).
4. Unit tests: array sizing, neighbor index validity (in-range, symmetric),
   seed derivation determinism.

## Debug visualization

**Scenario:** "field inspector" (new digit key). Cell outlines as the
geometry underlay, plus a numeric inspector beyond today's baseline:

- Hover/click a cell to list every SoA field with its raw value
  (`elevation`, `temperature`, `moisture`, `plate_id`, `biome`, `flow_dir`,
  `flow_accum`) in a side panel.
- Neighbor-index overlay: dashed links from the selected cell to its
  indexed neighbors, verifying the graph<->array bridge visually.
- Stats strip per field: min/max/mean over the whole array, updated live.
- World seed displayed; editing the seed regenerates and re-inspects.

## Acceptance criteria

- Debug screen shipped per the standard in `plans/features/README.md`;
  new keys/toggles documented in `README.md`.

- All arrays sized to the cell count of the level under construction
  (10,242 at level 5).
- Neighbor index access is O(1) per cell; no graph traversal inside
  simulation passes.
- Same seed -> byte-identical fields after any pass (property test with a
  trivial fill pass).
- Standard definition of done passes; spec written under
  `docs/book/specs/`.
