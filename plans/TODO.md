# TODO — Procedural Hex-Sphere Planet Generator

Organized to-do list for the consolidated plan
([`plans/notion.md`](notion.md)). Each item links to its feature file under
[`plans/features/`](features/README.md). Check items off as they land.

Status language follows notion.md: **Implemented / Next / Planned /
Deferred**.

Every feature ships with an interactive debug screen in the Vulkan viewer,
at least as rich as today's node viewer — the standard is defined in
[`plans/features/README.md`](features/README.md#debug-visualization-standard)
and the per-feature overlays in each feature file.

## Phase 0 — Foundation (Implemented)

- [x] `Node` system (hierarchical triangle node, `split`, `destroy`)
- [x] `IcosahedronPlan` (dual-pentagon interlocked mesh, whole-mesh split)
- [x] Vulkan debug viewer (keys `1`-`4`, `S`, `R`) — per-node overlays:
  outline, I/J/K arrows, `direction_of_node`, origin arrow, child links,
  open-port markers, center dot, labels, checkbox toggles
- [x] `mesh-validator` (headless wiring check)
- [x] Unit tests beside implementation; specs under `docs/book/specs/`

## Phase 1 — Geometry (Next)

- [ ] **01 — Spherical projection** → [feature](features/phase-1-geometry/01-spherical-projection.md)
  - [ ] Decide: extend `Node` to `Vec3` vs. separate 3D embedding (record in spec)
  - [ ] Icosahedron face table (20 faces) matching the North/South layout
  - [ ] Barycentric positions for subdivided nodes; project `v = R * normalize(v3)`
  - [ ] Tests: `|pos| == R`; topology unchanged; validator exit 0
  - [ ] Debug screen: sphere scenario with today's full node overlays,
    projection-error coloring, flat-net/sphere toggle, deviation readout
- [ ] **02 — Vertex relaxation** → [feature](features/phase-1-geometry/02-relaxation.md)
  - [ ] Vertex adjacency from the node graph
  - [ ] Spherical Lloyd iteration + reprojection
  - [ ] Sphere-constrained Laplacian smoothing
  - [ ] Hybrid schedule, 2 passes default (range 2-3)
  - [ ] Tests: uniformity improves; stays on sphere; deterministic
  - [ ] Debug screen: pass stepping + before/after toggle, displacement
    arrows, live uniformity readout
- [ ] **03 — Dual mesh -> hex/pent cells** → [feature](features/phase-1-geometry/03-dual-mesh-cells.md)
  - [ ] Stable cell index per unique mesh vertex (dedup)
  - [ ] Cell polygons from ordered surrounding centroids
  - [ ] Per-cell neighbor lists (symmetric)
  - [ ] Tests: counts match `10 * 4^n + 2`; exactly 12 pentagons
  - [ ] Debug screen: cell outlines + pentagon highlight, neighbor links,
    click-to-inspect cell, live cell/pentagon counters

## Phase 2 — Terrain (Planned)

- [ ] **04 — Data model (SoA)** → [feature](features/phase-2-terrain/04-data-model.md)
  - [ ] `CellFields` struct: `elevation/temperature/moisture/plate_id/biome/flow_dir/flow_accum`
  - [ ] Cell -> index bridge + neighbor index arrays
  - [ ] World-seed type + sub-seed derivation
  - [ ] Tests: sizing, index validity, determinism
  - [ ] Debug screen: per-cell field inspector (raw SoA values),
    neighbor-index overlay, per-field min/max/mean stats
- [ ] **05 — Tectonics** → [feature](features/phase-2-terrain/05-tectonics.md)
  - [ ] Blue-noise plate seeds (P = 8-20, default 10)
  - [ ] Voronoi `plate_id` assignment (flood fill / nearest-seed)
  - [ ] Plate table: type + motion vector (seeded)
  - [ ] Boundary classification (convergent/divergent/transform)
  - [ ] Base elevation + boundary mountains/rifts
  - [ ] Optional uplift diffusion (1-2 passes, `lambda` 0.3)
  - [ ] Debug screen: plate colors, seed markers, boundary-type colors,
    plate motion arrows, uplift underlay, cell inspector
- [ ] **06 — Elevation and sea level** → [feature](features/phase-2-terrain/06-elevation.md)
  - [ ] Seeded 3D noise + 3-scale FBM (4-5 octaves, persistence 0.5)
  - [ ] Combine `e = e_tect + w1*n_large + w2*n_mid + w3*n_small`
  - [ ] Mountain refinement near convergent boundaries
  - [ ] Normalize + sea-level threshold + ocean mask
  - [ ] 1-2 smoothing passes
  - [ ] Debug screen: height ramp + ocean mask, contours, live sea-level
    control, histogram, per-layer FBM underlays

## Phase 3 — Surface (Planned)

- [ ] **07 — Climate** → [feature](features/phase-3-surface/07-climate.md)
  - [ ] Temperature: `T = T_equator - |lat|*delta_T - elevation*lapse_rate`
  - [ ] Wind bands by latitude
  - [ ] Precipitation: ITCZ band + orographic term -> `moisture`
  - [ ] Debug screen: temperature/moisture color modes, wind arrows colored
    by band, latitude rings, orographic + formula inspector
- [ ] **08 — Biomes** → [feature](features/phase-3-surface/08-biomes.md)
  - [ ] Biome enum + exhaustive T x R x elevation lookup
  - [ ] O(n) fill of `biome`; ocean cells as water
  - [ ] Debug screen: biome colors + legend, lookup inspector
    (T/moisture/elevation -> biome), underlays, coverage counters
- [ ] **09 — Hydrology** → [feature](features/phase-3-surface/09-hydrology.md)
  - [ ] `flow_dir` steepest descent (with sentinel)
  - [ ] Downhill topological order + O(n) `flow_accum`
  - [ ] Rivers: `A > A_threshold` (tunable)
  - [ ] Lakes: local minima + targeted priority-flood
  - [ ] One-time river carve into elevation
  - [ ] Debug screen: flow arrows, accumulation-scaled rivers, live
    threshold control, lake markers, carve before/after toggle

## Phase 4 — LOD and streaming (Planned)

- [ ] **10 — LOD and streaming** → [feature](features/phase-4-lod/10-lod-streaming.md)
  - [ ] LOD0: whole planet at split level 5 (~10k cells), full pipeline once
  - [ ] Selective/local split of a chosen subtree
  - [ ] Seam stitching to coarser surroundings; extend `mesh-validator`
  - [ ] Camera-driven regional refinement + regional fields
  - [ ] Async streaming; free far patches; Vulkan buffer strategy
  - [ ] Ground LOD (heightmap tiles, GPU tessellation) — only after regional works
  - [ ] Debug screen: level-colored mixed mesh, seam highlight, LOD stats,
    threshold ring, manual refine/coarsen step key

## Phase 5 — Tier 3 (Optional, re-approve before starting)

- [ ] **11 — Tier 3 extras** → [feature](features/phase-5-tier3/11-tier3-optional.md)
  - [ ] Epoch snapshots
  - [ ] Seasonal climate
  - [ ] Cheap Airy isostasy
  - [ ] Plate age / fragmentation
  - [ ] Glacial effects
  - [ ] Debug screen per scheduled item (epoch slider, season slider,
    isostasy toggle — standard in `features/README.md`)

## Per-feature definition of done

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --workspace --all-targets` and `cargo test --doc --workspace` pass
- [ ] `cargo run -p planet-crafter-tools --bin mesh-validator` exits 0 (if wiring changed)
- [ ] Debug screen shipped per the `plans/features/README.md` standard
  (dedicated scenario, overlay vocabulary, per-cell inspection); new
  keys/toggles documented in `README.md`
- [ ] Spec under `docs/book/specs/` written/updated; `docs/scripts/check-book.sh` passes
- [ ] `AGENTS.md` updated (if commands/layout/conventions changed)

## Deferred (reference only — do not schedule)

- 2D toroidal grid alternative
- Recompute-from-`t` temporal model
- Iterative stream-power erosion
- FFT / flexural isostasy
- Rust core / C++ renderer split
- Full fluid sim / per-frame erosion / dense AoS cell structs
