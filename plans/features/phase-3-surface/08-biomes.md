# 08 — Biomes

- **Phase:** 3 — Surface
- **Status:** Planned
- **Depends on:** 07 — Climate (and 06 for elevation)
- **Plan reference:** notion.md Section 9

## Goal

Assign a biome to every cell by pure lookup over temperature x precipitation
x elevation — no simulation.

## Design summary

Biome set (notion.md Section 9): Tundra, Boreal forest, Temperate forest,
Grassland, Desert, Savanna, Rainforest. Ocean cells (below sea level,
feature 06) are handled as water, not as a land biome.

## Implementation steps

1. Define the biome enum (one `u8` discriminant per biome) and the lookup
   table over (temperature, moisture, elevation) bands.
2. Single O(n) pass filling `biome` from `temperature`, `moisture`, and the
   ocean mask.
3. Viewer: biome coloring mode with a legend; document any new key in
   `README.md`.

## Debug visualization

**Scenario:** "biomes" (new digit key). Cells filled by biome color with an
on-screen legend (color + name per biome). Additions beyond the baseline:

- Hover a cell: inspector shows the (temperature, moisture, elevation)
  triple and the biome it resolved to — the lookup made visible.
- Toggleable underlays: temperature or moisture beneath the biome fill,
  to see why a biome landed where it did.
- Ocean/water cells rendered distinctly from every land biome.
- Biome coverage counters (cells per biome) on screen.

## Acceptance criteria

- Debug screen shipped per the standard in `plans/features/README.md`;
  new keys/toggles documented in `README.md`.

- Every land cell gets exactly one biome; ocean cells are marked as water
  (unit test).
- Lookup covers the full `u8` x `u8` input range without gaps (table
  exhaustiveness test).
- Same seed -> identical `biome` array.
- Standard definition of done passes; spec written under
  `docs/book/specs/`.
