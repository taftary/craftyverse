# Features — Implementation Breakdown

This folder splits the consolidated plan ([`plans/notion.md`](../notion.md))
into individually implementable features, grouped by roadmap phase
(notion.md Section 14).

Each feature file is self-contained: goal, design summary, implementation
steps, dependencies, and acceptance criteria. A feature is done when its
acceptance criteria pass, including the definition of done in `AGENTS.md`
(fmt, clippy, tests, spec accuracy).

Track progress in [`plans/TODO.md`](../TODO.md).

## Phase 1 — Geometry (Next)

| # | Feature | Status | File |
|---|---------|--------|------|
| 01 | Spherical projection | Next | [phase-1-geometry/01-spherical-projection.md](phase-1-geometry/01-spherical-projection.md) |
| 02 | Vertex relaxation | Next | [phase-1-geometry/02-relaxation.md](phase-1-geometry/02-relaxation.md) |
| 03 | Dual mesh -> hex/pent cells | Next | [phase-1-geometry/03-dual-mesh-cells.md](phase-1-geometry/03-dual-mesh-cells.md) |

## Phase 2 — Terrain (Planned)

| # | Feature | Status | File |
|---|---------|--------|------|
| 04 | Simulation data model (SoA) | Planned | [phase-2-terrain/04-data-model.md](phase-2-terrain/04-data-model.md) |
| 05 | Tectonics | Planned | [phase-2-terrain/05-tectonics.md](phase-2-terrain/05-tectonics.md) |
| 06 | Elevation and sea level | Planned | [phase-2-terrain/06-elevation.md](phase-2-terrain/06-elevation.md) |

## Phase 3 — Surface (Planned)

| # | Feature | Status | File |
|---|---------|--------|------|
| 07 | Climate | Planned | [phase-3-surface/07-climate.md](phase-3-surface/07-climate.md) |
| 08 | Biomes | Planned | [phase-3-surface/08-biomes.md](phase-3-surface/08-biomes.md) |
| 09 | Hydrology | Planned | [phase-3-surface/09-hydrology.md](phase-3-surface/09-hydrology.md) |

## Phase 4 — LOD and streaming (Planned)

| # | Feature | Status | File |
|---|---------|--------|------|
| 10 | LOD and streaming | Planned | [phase-4-lod/10-lod-streaming.md](phase-4-lod/10-lod-streaming.md) |

## Phase 5 — Tier 3 (Optional)

| # | Feature | Status | File |
|---|---------|--------|------|
| 11 | Tier 3 extras | Optional | [phase-5-tier3/11-tier3-optional.md](phase-5-tier3/11-tier3-optional.md) |

## Debug visualization standard

Every feature ships with an interactive debug screen in the Vulkan viewer,
at least as rich as today's node viewer — and richer where the feature's
data justifies it. Today's baseline (`crates/engine/src/scene/geometry.rs`,
`crates/engine/src/render/viewer.rs`), per node:

- Triangle outline colored by level; filled center dot.
- I/J/K direction arrows, `direction_of_node` arrow, dashed origin arrow.
- Dashed child links colored per port; disc markers on open ports.
- Name/level labels plus A/B/C corner labels.
- Checkbox panel toggles per overlay group (`DisplayOptions`).
- Scenarios are selected with number keys.

Each feature's debug screen must:

1. Add one dedicated scenario (new digit key) for the feature's data.
2. Keep the existing overlay vocabulary working where it applies
   (outlines, arrows, dashed links, disc markers, labels, checkbox
   toggles).
3. Add the feature-specific overlays listed in the feature file, plus
   hover/click inspection of the underlying per-cell values.
4. Document every new key/toggle in `README.md` and keep the spec accurate.

## Dependency order

```text
01 projection -> 02 relaxation -> 03 dual mesh
03 dual mesh  -> 04 data model -> 05 tectonics -> 06 elevation
06 elevation  -> 07 climate    -> 08 biomes
06 elevation  -> 09 hydrology
03 + 06       -> 10 LOD/streaming
Phase 1-4     -> 11 Tier 3 (optional)
```

Rules for every feature:

- Deterministic from a single seed (notion.md Section 3).
- O(n) passes over cells; no per-cell unbounded neighbor searches.
- Compact per-cell fields (`u8`/`u16`), structure-of-arrays storage.
- Ships its debug screen per the standard above.
- Each landed feature gets its spec under `docs/book/specs/` and keeps
   `AGENTS.md` and the viewer controls in `README.md`
  accurate.
