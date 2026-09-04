# Procedural Hex-Sphere Planet Generator — Consolidated Design

**Status:** living plan. This document merges and supersedes the four original
notion drafts (`notion1.md`, `notion-2.md`, `notion-3.md`, `notion4.md`) and
re-anchors the plan on the code that already exists: the `Node` class and the
`IcosahedronPlan`.

Status language used below:

- **Implemented** — exists in the repo today.
- **Next** — the immediate continuation of the current work.
- **Planned** — designed, not started.
- **Deferred** — kept for reference, not on the roadmap (see Appendix).

How this file relates to the specs: this is the *plan*. Once a layer is
implemented, its contract moves into a module spec under
`docs/book/specs/`, which remains the source of truth for implemented
behavior. This document never describes unimplemented layers as existing.

---

## 1. Purpose

Build a deterministic, seamless, hex-dominant planet (icosahedral base, 12
pentagons) with tectonics, elevation, climate, biomes, and hydrology, on top
of the existing hierarchical triangle-node system. Performance discipline is
mobile-grade (O(n) passes, compact per-cell data), even though the current
baseline is desktop Vulkan.

Two decisions are already made:

- **Hex-sphere first.** The icosahedral path is the committed geometry — the
  node system already builds the icosahedron topology. The 2D toroidal grid
  alternative from the early drafts is deferred (Appendix).
- **Pure Rust + Vulkan.** The engine is `planet-crafter-engine` (vulkano,
  winit, naga, fontdue, glam). The "Rust core / C++ renderer" split from the
  drafts does not apply.

---

## 2. Current foundation (Implemented)

### 2.1 The `Node` class

Source: [`crates/engine/src/node/mod.rs`](../crates/engine/src/node/mod.rs)
Spec: [`docs/book/specs/node.md`](../docs/book/specs/node.md)

A `Node` is one isosceles triangle in a hierarchical mesh.

- **Geometry:** `center` (centroid), corner `points` `[A, B, C]` (`A` = apex,
  `BC` = base, perpendicular to the orientation), `direction_of_node`
  (base -> apex, normalized), `direction_to_origin`, and the directional
  vectors `[i, j, k]` — each perpendicular to one edge and pointing outward
  from the center.
- **Topology:** `children[3]` — bidirectional links to adjacent nodes, one per
  direction (I/J/K). Links are reciprocal (`0 <-> 2`, `1 <-> 1`).
- **Identity:** unique `name`; `level` (split depth, 0 for roots).
- **Methods:** `new(...)` builds the triangle; `split()` subdivides into four
  nodes (corner nodes I/J/K keep the parent direction, the inverted center
  node flips it), halves base length and height, wires the internal
  center <-> corner links, and returns the center node; `destroy()` severs
  all reciprocal links.

Ownership: `NodeRef = Rc<RefCell<Node>>` — shared, runtime-borrowed links.

### 2.2 The `IcosahedronPlan`

Source: [`crates/engine/src/icosahedron_plan/mod.rs`](../crates/engine/src/icosahedron_plan/mod.rs)
Spec: [`docs/book/specs/icosahedron-plan.md`](../docs/book/specs/icosahedron-plan.md)

Central manager of the node hierarchy.

- `generate(side_length)` builds the **dual-pentagon interlocked mesh**: a
  North pentagonal base (5 base + 5 reverted nodes, `Labeling::Normal`) and a
  South one (mirrored labeling), wired together through reciprocal I <-> K
  links with reflection offsets. Total: 20 nodes = the icosahedron's 20
  triangular faces, laid out as a flat 2D net.
- `split()` subdivides the whole mesh one level: every node splits once, the
  fresh corner nodes are re-wired across every subdivided edge
  (`wire_chain_edge` / `wire_pair_edge`), the root is re-anchored, and the old
  nodes are destroyed. Each level multiplies the triangle count by 4.
- `root_node` anchors the mesh (North base root).

### 2.3 Tooling

- **Vulkan debug viewer** (`cargo run --bin planet-crafter`): keys **1–4**
  select scenes (single node, split node, pentagonal base, dual-mesh
  interlocked plan), **S** subdivides one level, **R** regenerates. Per node
  it displays the triangle outline, I/J/K arrows, `direction_of_node`, the
  origin arrow, child links, and labels.
- **`mesh-validator`** (`cargo run -p planet-crafter-tools --bin
  mesh-validator`): headless mesh-wiring check, exit 0/1.
- **Unit tests** beside the implementation: `node/tests.rs`,
  `icosahedron_plan/tests.rs`.

### 2.4 Mapping: draft pipeline -> implementation status

| Draft pipeline step                  | Status      | Where |
|--------------------------------------|-------------|-------|
| Icosahedron base (20 faces)          | Implemented | `IcosahedronPlan::generate` |
| Triangle subdivision (x4 per level)  | Implemented | `IcosahedronPlan::split`, `Node::split` |
| Mesh integrity / wiring validation   | Implemented | link-based topology, `mesh-validator`, viewer |
| Spherical projection (net -> sphere) | Next        | geometry layer (Section 4) |
| Vertex relaxation                    | Next        | geometry layer (Section 4) |
| Dual mesh -> hex/pent cells          | Next        | geometry layer (Section 4) |
| Tectonics                            | Planned     | Section 6 |
| Elevation                            | Planned     | Section 7 |
| Climate                              | Planned     | Section 8 |
| Biomes                               | Planned     | Section 9 |
| Hydrology                            | Planned     | Section 10 |
| LOD streaming                        | Planned     | Section 11 |
| Temporal evolution                   | Deferred    | Section 12, Appendix |

---

## 3. Design principles and constraints

Carried over from the drafts, unchanged:

- **O(n) passes** over cells wherever possible; no per-cell unbounded
  neighbor searches.
- **Deterministic** from a single seed: plate seeds, noise offsets, and plate
  motion vectors all derive from it.
- **Compact data:** `u8`/`u16` per-cell fields, structure-of-arrays storage
  (Section 5).
- **No heavy simulation:** no full fluid sim, no continuous-integral erosion,
  no FFT convolution (flexural isostasy). Cheap one-pass approximations
  instead.
- **Modular layers:** each generation layer builds on the previous one and is
  independently replaceable/tunable.
- Desktop Vulkan is the current baseline; the mobile-grade discipline above
  is what keeps the door open for the **Planned** Android/iOS targets.

---

## 4. Geometry pipeline

### 4.1 Implemented

- **Icosahedron base** — the dual-pentagon interlocked mesh (Section 2.2).
- **Subdivision** — `Node::split` per node, `IcosahedronPlan::split` for the
  whole mesh, one level at a time.
- **Mesh integrity** — mostly dissolved by construction: connectivity lives
  in reciprocal links, not in coordinates, so the classic failure modes from
  the drafts (subdivision mismatch, float-duplicate vertices, vertex
  snapping, edge registries) do not apply to topology. Coordinates still
  matter for rendering; dedup is only needed when flattening the graph into a
  render mesh. Validation is covered by the `mesh-validator`, the viewer's
  wireframe display, and the unit tests.

### 4.2 Next

1. **Spherical projection.** The current mesh is a flat 2D net (`Vec2`) with
   icosahedron topology. The next step gives every node a 3D position on the
   sphere: `v = R * normalize(v3)`. Open design choice (decide at
   implementation time, then update the specs): extend `Node` to `Vec3`
   positions, or keep the 2D net and add a separate 3D embedding.
2. **Relaxation.** 2–3 hybrid passes: a few Lloyd iterations plus
   sphere-constrained Laplacian smoothing. Not naive Lloyd for 100+
   iterations — it converges too slowly.
3. **Dual mesh -> hex-sphere cells.** Triangle centroids become cell
   vertices; each triangle vertex becomes a cell. All cells are hexagons
   except the 12 pentagons at the original icosahedron vertices — the same
   vertices the current North/South pentagon bases encode.

### 4.3 Scale table

Cell counts per split level (triangles = `20 * 4^n`, dual cells =
`10 * 4^n + 2`):

| Split level | Triangles | Hex/pent cells |
|-------------|-----------|----------------|
| 0           | 20        | 12             |
| 1           | 80        | 42             |
| 2           | 320       | 162            |
| 3           | 1,280     | 642            |
| 4           | 5,120     | 2,562          |
| 5           | 20,480    | 10,242         |
| 6           | 81,920    | 40,962         |

LOD0 target: **level 5 (~10k cells)** — a complete low-detail planet for
space-view rendering at O(n) generation cost. `level` in the code is exactly
the split count in this table.

---

## 5. Data model

Two complementary parts:

- **Topology graph (Implemented):** the existing `Node`/`NodeRef` structure.
  It owns geometry and neighbor traversal — the skeleton of the planet.
- **Simulation fields (Planned):** structure-of-arrays storage keyed by cell
  index, one compact array per field. The graph provides neighbor lists
  (directly, or prebuilt into an index array per level); simulation passes
  operate only on the arrays.

```rust
elevation:   Vec<u16>,   // 0..=65535, normalized to sea level
temperature: Vec<u8>,
moisture:    Vec<u8>,
plate_id:    Vec<u16>,
biome:       Vec<u8>,
flow_dir:    Vec<u8>,    // neighbor index of the downhill flow
flow_accum:  Vec<u16>,
```

Roughly **10 bytes per cell** for the core maps. Cache-friendly,
SIMD-friendly, and cheap to upload to the GPU. Store only fields that are
consumed downstream; debug layers stay behind flags.

---

## 6. Tectonics (Planned)

1. **Plate seeding** — pick `P` cells as seeds (range 8–20, default 10),
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
5. **Optional uplift diffusion** — the one cheap heavier-concept worth
   keeping: a single weighted-neighbor-average pass, O(n), to soften sharp
   boundary steps:

   ```text
   U(t+1) = U(t) + lambda * sum_neighbors (U_n - U(t))    // 1-3 passes
   ```

---

## 7. Elevation (Planned)

- **FBM noise, three scales** — large: continents; mid: mountain-range
  shaping; small: hills/detail. 4–5 octaves, persistence 0.5:

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
- **Smoothing** — 1–2 neighbor-averaging passes only. No Gaussian/FFT
  isostasy (deferred, Appendix).

---

## 8. Climate (Planned)

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

---

## 9. Biomes (Planned)

Pure lookup over temperature x precipitation x elevation — no simulation:

Tundra, Boreal forest, Temperate forest, Grassland, Desert, Savanna,
Rainforest.

---

## 10. Hydrology (Planned)

- **Flow direction** — each land cell flows to its lowest neighbor
  (steepest descent on the cell graph), stored in `flow_dir`.
- **Flow accumulation** — one pass in topological (downhill) order, O(n):

  ```text
  A(cell) = 1 + sum( w_ij * A(upstream_j) )
  ```

- **Rivers** — cells with `A > A_threshold`.
- **Lakes** — cells with no lower neighbor (local minima); lightweight
  priority-flood fill only where basins are visually significant, not
  globally at LOD0.
- **River carve** — a one-time static pass to etch river channels into the
  elevation. No iterative stream-power erosion solver (deferred, Appendix).

---

## 11. LOD and streaming (Planned)

The node hierarchy *is* the LOD system: each split level doubles the
resolution per edge. This replaces the drafts' 7/19/37 hex-patch scheme.

- **LOD0 (planet scale)** — whole planet at split level ~5 (~10k cells):
  low-frequency elevation, plate IDs, coarse climate and biomes. Runs the
  full generation pipeline once. Static mesh, space-view rendering.
- **Regional LOD** — when the camera approaches, refine only the region
  under it. This needs a **selective/local split**: an extension of the
  current whole-mesh `IcosahedronPlan::split` that subdivides a chosen
  subtree and stitches the boundary to the coarser surroundings
  (seam-stitching pass — the one mesh-integrity item that stays real).
  Adds: mid-frequency elevation, local climate downscaling, rivers.
- **Ground LOD** — heightmap tiles (128x128 or 256x256) with GPU
  tessellation, micro-noise, material masks, object placement. Deferred
  until regional LOD works.
- **Streaming** — camera-driven, asynchronous generation, patches freed
  when far away; persistent mapped buffers / triple buffering on the Vulkan
  side when we get there.

---

## 12. Temporal evolution (Deferred)

The drafts' live recompute-from-`t` model (continuous integrals, infinite
forward/backward time travel) is not on the roadmap. If a sense of
geological time is ever wanted (Tier 3 only):

- Precompute a handful of discrete **epoch snapshots** (plate positions,
  elevation) at world-gen time; blend between them at runtime.
- Plate motion per epoch is trivial: `X_p(epoch) = X0 + v_p * epoch` (no
  acceleration term needed).
- Never integrate `F(τ)` live per frame.

---

## 13. Complexity tiers

| Tier | Content | Status |
|------|---------|--------|
| **Tier 1 — Simple** | Geometry, basic plates, simple boundaries, single FBM, 1 smoothing pass | Geometry implemented; rest is the immediate build target |
| **Tier 2 — Intermediate** | Blue-noise seeds, subduction direction, uplift diffusion (1–2 passes), 3-scale FBM, climate, rivers via flow accumulation, lakes, biomes | Default target |
| **Tier 3 — Advanced** | Plate age/fragmentation, dynamic boundaries, seasonal climate, glacial effects, epoch snapshots, cheap isostasy | Optional, after Tier 2 |

---

## 14. Roadmap (anchored on the code)

- **Phase 0 — done:** node system (`Node`), icosahedron plan
  (`IcosahedronPlan`), debug viewer, `mesh-validator`.
- **Phase 1 — geometry (Next):** spherical projection, relaxation, dual
  mesh -> hex/pent cells.
- **Phase 2 — terrain:** tectonics, base elevation, FBM refinement, sea
  level.
- **Phase 3 — surface:** climate, biomes, hydrology (rivers, lakes, carve).
- **Phase 4 — LOD:** selective split + seam stitching, camera-driven
  regional refinement, async streaming.
- **Phase 5 — optional Tier 3:** epoch snapshots, seasonal climate, cheap
  isostasy.

Each phase lands with its spec under `docs/book/specs/` and keeps the
definition of done in `AGENTS.md` (fmt, clippy, tests, mesh-validator,
spec accuracy).

---

## 15. Recommended parameters

| Parameter                  | Range        | Default |
|----------------------------|--------------|---------|
| Plate count                | 8–20         | 10      |
| Plate velocity             | 0.1–1.0      | 0.3     |
| FBM octaves                | 3–7          | 4–5     |
| FBM persistence            | 0.3–0.7      | 0.5     |
| Uplift diffusion `lambda`  | 0.1–0.5      | 0.3     |
| Uplift diffusion passes    | 1–3          | 1–2     |
| Elevation smoothing passes | 1–2          | 1       |
| Relaxation passes          | 2–3 (hybrid) | 2       |
| LOD0 split level           | 3–6          | 5       |
| River threshold `A_thr`    | tune per scale | —     |

---

## 16. Appendix — deferred and dropped ideas

Kept for reference only. Not on the roadmap; do not implement without
re-opening the decision.

- **2D toroidal grid** (from `notion4.md`/`notion-2.md`) — flat `W x H`
  grid with wrap-around distance
  (`dx = min(|x-cx|, W-|x-cx|)`, same for `dy`). Was the "mobile MVP"
  alternative; superseded by the hex-sphere, which the code already builds.
- **Recompute-from-`t` temporal model** (from `notion4.md`) — world state
  `W(t) = {P, U, h, R, A, S, C}` recomputed via continuous integrals,
  `h(t) = h0 + integral(F_tectonic) - integral(F_erosion)`, plate motion
  `P(t) = (P0 + v*t + 0.5*a*t^2) mod domain`. Too heavy and unnecessary;
  replaced by epoch snapshots (Section 12) if anything.
- **Iterative stream-power erosion** (from `notion4.md`/`notion-2.md`) —
  `E = k_E * A^m * S^n` vs sediment capacity `C = k_C * A^mc * S^nc`, plus
  thermal erosion `h(t+1) = h(t) - k_therm * max(0, S - S_crit)`, as a
  simulation loop. Replaced by the one-time river carve (Section 10).
  Suggested constants if ever revisited: `k_E` 0.01–0.05, `m` ~0.5, `n` ~1,
  `S_crit` 0.05–0.2.
- **FFT / flexural isostasy** (from `notion4.md`) — Gaussian-kernel
  convolution `K(r) = exp(-r^2 / (2*sigma^2))`. The single most expensive
  item in the old drafts for near-zero visible gain at hex-sphere
  resolutions. Airy isostasy (`h_iso = k_iso * E_cumulative`) is the only
  cheap variant, Tier 3 at most.
- **Rust core / C++ renderer split** (from `notion1.md`/`notion-3.md`) —
  the engine is pure Rust + Vulkan; nothing to split.
- **Full fluid simulation, per-frame erosion solving, `f64`/`f32`
  everywhere, dense AoS cell structs** — permanently rejected; see the
  design principles (Section 3).
