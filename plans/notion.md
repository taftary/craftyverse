# Procedural Hex-Sphere Planet Generator — Mobile Design Doc

**Foundation:** 3D icosahedron → hex-sphere planet, built for mobile performance (limited CPU/RAM/battery). The toroidal 2D full-physics document you had is kept only as an *ideas appendix* (Section 13) — some of its formulas are reused below in lightweight form, but its heavy machinery (continuous-time integrals, FFT flexural isostasy, full recompute-from-t) is dropped as unsuitable for mobile.

---

## 1. Goals & Constraints

- **Platform:** mobile (limited CPU, RAM, battery)
- **Topology:** hex-dominant sphere (12 pentagons, icosahedral base)
- **Design rules:**
  - O(n) passes over cells wherever possible
  - No full fluid sim, no continuous-integral erosion, no FFT convolution
  - Minimize stored data per cell (compact types: `int16`, `uint8`)
  - Deterministic and reproducible from a seed
  - Camera-driven, asynchronous LOD streaming

---

## 2. High-Level Architecture

Generation happens in dependent layers, each building on the last:

1. **Geometry** — icosahedron → subdivided sphere → hex-sphere
2. **Tectonics** — plates, boundaries, base elevation
3. **Elevation** — refine heightmap with noise + light smoothing
4. **Climate** — temperature, wind, precipitation
5. **Biomes** — classify cells into ecosystems
6. **Hydrology** — rivers, lakes, flow accumulation
7. **LOD streaming** — planet → regional → ground detail on demand

Each subsystem is modular and independently replaceable/tunable.

---

## 3. Geometry Pipeline

1. **Icosahedron generation** — 12 canonical vertices, 20 triangular faces, normalized to radius `R`.
2. **Subdivision** — for each triangle, compute midpoints, replace with 4 smaller triangles. Repeat *N* times.
3. **Spherical projection** — `v = R * normalize(v)` for every vertex.
4. **Relaxation** — move each vertex toward the centroid of its neighbors, reproject to sphere, repeat 5–10 iterations.
   - Naive Lloyd relaxation converges slowly. For mobile, prefer a **hybrid**: a small number of Lloyd passes (2–3) combined with Laplacian smoothing constrained to the sphere, or a CVT-style optimization. This cuts iteration count substantially without hurting mesh quality.
5. **Dual mesh → hex-sphere** — compute triangle centroids, connect centroids of adjacent triangles to build polygon cells (mostly hexagons, 12 pentagons at the icosahedron's original vertices).

### Mesh Integrity (must-do, not optional)

Common failure modes and fixes:

| Issue | Fix |
|---|---|
| Subdivision mismatch between adjacent faces | Shared **edge registry** (hash map) during subdivision so edges reuse vertices |
| Near-duplicate vertices from float precision | **Vertex snapping** — round projected coords to a fixed tolerance (e.g. `1e-6`) before dedup |
| Projection rounding breaking connectivity | Deduplicate **immediately after** sphere projection, not before |
| LOD seams at transition boundaries | Mandatory **seam-correction/stitching pass** after each LOD build |

Testing: wireframe mode to visually confirm connectivity; log vertex counts before/after dedup; automated checks for dangling edges.

---

## 4. Data Model (Structure of Arrays)

```cpp
// Per-cell (planet-wide, LOD0)
struct Cell {
    Vector3 position;
    int     neighbors[6];   // 5 for the 12 pentagon cells
    float   elevation;
    float   temperature;
    float   precipitation;
    int     biome;
    int     plate_id;
    bool    is_ocean;
    int     flow_to;
    int     flow_accum;
};

struct Plate {
    int     id;
    Vector2 motion;
    bool    is_continental;
};
```

For the Rust core, use compact SoA storage instead of AoS structs:

```rust
elevation:   Vec<u16>,
temperature: Vec<u8>,
moisture:    Vec<u8>,
plate_id:    Vec<u16>,
biome:       Vec<u8>,
flow_dir:    Vec<u8>,
flow_accum:  Vec<u16>,
```

---

## 5. Tectonic Simulation

1. **Plate seeding** — pick `P` random cells as seeds (suggested range: 8–20 plates), flood-fill to assign `plate_id`.
2. **Plate motion** — assign a random 2D motion vector per plate.
3. **Boundary classification** — compare motion vectors along shared edges → convergent / divergent / transform.
4. **Base elevation from tectonics:**
   - Continental plates → higher baseline
   - Oceanic plates → lower baseline
   - Mountains near convergent boundaries
   - Rifts near divergent boundaries

This intentionally skips the heavier concepts from the toroidal doc (plate density/thickness fields, uplift diffusion PDE, plate acceleration) — those add cost without a proportional mobile-visible benefit. If you later want more geological nuance, uplift diffusion is the cheapest one to add back (single weighted-neighbor-average pass, O(n)).

---

## 6. Elevation Generation

**Noise fields (FBM, multi-octave):**
- Large scale → continents
- Mid scale → mountain range shaping
- Small scale → hills/terrain detail

**Mountains** — generated primarily at convergent boundaries (tectonic base), then refined by mid-scale noise. Add:
- Ridge generation along plate boundaries
- Lightweight erosion *approximation* (not a full sim) to soften peaks/valleys
- Cheap noise modulation for variation

**Combine:**
```
e = e_tectonic + w1*noise_large + w2*noise_mid + w3*noise_small
```

**Sea level** — normalize elevation, mark `is_ocean` below threshold.

**Smoothing** — 1–2 lightweight passes only (neighbor averaging). Do not use the toroidal doc's Gaussian/FFT flexural isostasy on mobile — it's the single most expensive item in these notes and buys little visible improvement at hex-sphere resolutions.

---

## 7. Climate Model

**Temperature:**
```
T = T_equator - |lat| * deltaT - elevation * lapse_rate
```

**Wind bands:**
- Equatorial → east–west
- Subtropical → west–east
- Mid-latitude → mixed
- Polar → weak

**Precipitation:**
- Moisture originates over oceans, moves along wind direction
- Rains when forced upward by terrain (orographic effect) → rain shadows

Optional refinement borrowed from the toroidal notes (cheap, single pass, worth keeping):
```
R_lat = R0 * exp(-alpha * (lat - lat_ITCZ)^2)      // latitude rainfall band
R_oro = k_oro * max(0, dot(grad_h, wind_dir))       // orographic term
R     = R_lat + R_oro
```

---

## 8. Hydrology

- **Flow direction** — each land cell flows to its lowest neighbor.
- **Flow accumulation** — count upstream cells (O(n) with a sorted/topological pass):
  ```
  A(cell) = 1 + sum(w_ij * A(upstream_i))
  ```
- **Rivers** — cells with accumulation above a threshold `A_thr`.
- **Lakes** — cells with no lower neighbor (local minima); use a lightweight priority-flood fill only where basins are visually significant, not globally at LOD0.

Skip continuous stream-power/sediment-capacity erosion (`E = k_E·A^m·S^n`, `C = k_C·A^mc·S^nc`) as a *simulation loop* — too costly for mobile. If you want its visual effect, bake a one-time static "river carve" pass instead of an iterative erosion solver.

---

## 9. Biome Assignment

Based on temperature, precipitation, elevation:

Tundra · Boreal forest · Temperate forest · Grassland · Desert · Savanna · Rainforest

Simple lookup table (temperature × precipitation × elevation bands) — no simulation needed here.

---

## 10. Multi-Scale LOD Architecture

### LOD0 — Planet Scale
- Whole-planet hex-sphere, ~10k–100k cells
- Low-frequency elevation, plate ID, coarse moisture/temperature/biome
- Runs: tectonic sim, low-freq elevation noise, global climate (Hadley/Ferrel/Polar cells)
- Output: complete low-detail planet for space-view rendering

### LOD1–3 — Regional Scale
- Each LOD0 hex subdivides into a local hex patch:
  - LOD1: 7 cells · LOD2: 19 cells · LOD3: 37 cells
- Adds: medium-frequency elevation, local moisture/temperature, river flow, local biome refinement
- Algorithms: multi-octave noise refinement, *local* lightweight erosion, river generation via flow accumulation, climate downscaling
- Triggered when camera approaches; must stitch seamlessly to LOD0/adjacent patches

### LOD4+ — Ground Scale
- Regional hex patch → heightmap tile (128×128 or 256×256), GPU-tessellated
- Adds: high-frequency elevation, micro-detail noise (rocks/cliffs), material masks (grass/sand/rock), object spawn maps (trees/props)
- Triggered near ground; smooth transition from orbit to surface

### Streaming System
- Camera requests LOD each frame based on distance-to-surface
- Rust core generates data **asynchronously**
- C++/GPU side uploads via SSBO/TBO with **persistent mapped buffers** and **triple buffering** (avoids sync stalls)
- Old patches freed once far from camera

### Engine Split
- **Rust core:** hex-sphere build, tectonics, elevation, climate, LOD generation pipeline, SoA data storage
- **C++ rendering:** static LOD0 mesh, dynamic LOD1–3 patch meshes, heightmap-based LOD4+ terrain; vertex shader for hex-sphere deformation, tessellation shader for ground refinement, fragment shader for biome-based material blending

---

## 11. Optional: Temporal Evolution (Tier 3 only — use sparingly on mobile)

The toroidal doc's infinite forward/backward time-travel model (`W(t) = {P(t), U(t), h(t), R(t), A(t), S(t), C(t)}`, recomputed via continuous integrals) is not mobile-appropriate as a live system. If you want *some* sense of geological time:

- **Precompute, don't integrate live.** Bake a handful of discrete "epoch" snapshots (e.g. 5–10 keyframes of plate position/elevation) at world-gen time, and interpolate/blend between them at runtime instead of solving `∫F(τ)dτ` per frame.
- Plate positions over discrete epochs are cheap (`X_p(epoch) = X0 + v_p * epoch`, no acceleration term needed).
- Treat this as strictly optional and gate it behind your highest complexity tier.

---

## 12. Performance & Memory Rules

- Use compact types (`int16`, `uint8`) everywhere per-cell data is stored.
- No full fluid simulation, no per-frame erosion solving, no FFT convolution.
- Store only fields you actually consume downstream (drop unused debug layers on device builds).
- Every generation pass must be O(n) over cells — no cell should trigger unbounded neighbor searches.
- LOD1+ generation is async and cancellable if the camera moves away before it completes.

---

## 13. Complexity Tiers (mapped to mobile feasibility)

| Tier | Content | Mobile fit |
|---|---|---|
| **Tier 1 — Simple** | Basic plates, simple boundaries, single FBM, basic smoothing | ✅ Always on |
| **Tier 2 — Intermediate** | Subduction-style boundaries, climate rainfall, rivers via flow accumulation | ✅ Default target |
| **Tier 3 — Advanced** | Plate age/fragmentation, dynamic boundaries, seasonal climate, discrete-epoch time evolution | ⚠️ Optional, high-end devices / low LOD only |

Formula-heavy items from the original toroidal notes (continuous-time integrals, uplift diffusion PDE at full resolution, flexural isostasy via FFT) are **not recommended at any tier on mobile** — they're desktop-simulation-grade, not real-time-mobile-grade.

---

## 14. Full Generation Pipeline (Summary)

1. Build icosahedron
2. Subdivide
3. Project to sphere
4. Relax vertices (hybrid Lloyd + Laplacian/CVT)
5. Build hex-sphere (dedup + edge registry)
6. Generate tectonic plates
7. Compute base elevation (tectonics + noise layers)
8. Normalize elevation + sea level
9. Compute temperature
10. Compute wind + precipitation
11. Assign biomes
12. Compute rivers + lakes (flow direction/accumulation)
13. Stream LOD1–3 regional detail on camera approach
14. Stream LOD4+ ground detail near surface

---

## 15. Appendix — Rendering API Note

If graphics performance becomes the bottleneck (not generation), Vulkan is the option to reach for — it gives the most headroom for advanced GPU features but costs more development complexity than a higher-level API. Worth deferring until the generation pipeline above is solid and profiled.