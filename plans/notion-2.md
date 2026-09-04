# 🌍 Procedural Tectonic World Generator — Organized Specification

---

## 1. Overview & Design Philosophy

**Goal:** A deterministic, seamless, physically-inspired world generator optimized for **mobile performance**.

**Core Principles:**
- **O(n)** algorithms wherever possible
- Compact data types (`u8`, `u16`, `f32`)
- Minimal stored state — world is **recomputed from time `t`**
- Fully deterministic from a single seed
- Modular, replaceable subsystems

### 1.1 Two Geometric Representations

The system supports **two interchangeable base geometries**, sharing the same simulation layers:

| Feature | 2D Toroidal Grid | 3D Hex-Sphere |
|---|---|---|
| Topology | Flat grid, wraps at edges | Icosahedron → subdivision → hex-pent sphere |
| Seamlessness | Toroidal wrap via modulo | Natural sphere, no wrapping needed |
| Cell shape | Square pixels | Mostly hexagons (12 pentagons) |
| Target use | Heightmap rendering, 2D games | Full 3D planet rendering |
| Mobile cost | Lower | Higher (3D mesh, LOD) |

> **Recommendation for mobile:** Start with the 2D toroidal model for gameplay, upgrade to hex-sphere only if 3D planet rendering is essential.

---

## 2. System Architecture

### 2.1 Layered Pipeline (Bottom → Top)

```
┌─────────────────────────────────────────────────┐
│  LOD System (hex-sphere only)                  │  ← Rendering
├─────────────────────────────────────────────────┤
│  Temporal Simulation (recompute-from-t)        │  ← Time navigation
├─────────────────────────────────────────────────┤
│  Climate → Hydrology → Erosion → Isostasy      │  ← Surface processes
├─────────────────────────────────────────────────┤
│  Elevation Construction (FBM + tectonics)      │  ← Heightmap
├─────────────────────────────────────────────────┤
│  Tectonic Simulation (plates, boundaries)      │  ← Geodynamics
├─────────────────────────────────────────────────┤
│  Plate Generation (Voronoi, blue noise)        │  ← Initialization
├─────────────────────────────────────────────────┤
│  Geometry (toroidal grid / hex-sphere)         │  ← Base topology
└─────────────────────────────────────────────────┘
```

### 2.2 Subsystem Dependencies

```
Geometry ──→ Plate Gen ──→ Tectonics ──→ Elevation ──→ Climate ──→ Hydrology ──→ Erosion
      │            │             │             │             │            │
      └────────────┴─────────────┴─────────────┴─────────────┴────────────┘
                                    │
                                    ▼
                          Temporal Simulation
                                    │
                                    ▼
                              Final Output
```

---

## 3. Geometry & Topology

### 3.1 2D Toroidal Grid

**Dimensions:** `W × H` (e.g., 512×512, 1024×1024)

**Toroidal Distance:**
```
d_x(x, c_x) = min(|x - c_x|, W - |x - c_x|)
d_y(y, c_y) = min(|y - c_y|, H - |y - c_y|)
d(x, y) = √(d_x² + d_y²)
```

**Wrap function:**
```
wrap(x, y) → ((x + W) mod W, (y + H) mod H)
```

### 3.2 3D Hex-Sphere (Mobile-Optimized)

**Pipeline:**
1. **Icosahedron:** 12 vertices, 20 triangular faces
2. **Subdivision:** Replace each triangle with 4 smaller triangles (repeat N times)
3. **Spherical projection:** Normalize each vertex to radius `R`
4. **Relaxation:** Lloyd iterations (5–10) or Laplacian smoothing → more uniform cells
5. **Dual mesh:** Triangle centroids → hex-pent polygon cells

**Cell counts by subdivision level:**

| Subdivisions | Triangle Count | Hex Cells (approx) |
|---|---|---|
| 1 | 80 | 42 |
| 2 | 320 | 162 |
| 3 | 1,280 | 642 |
| 4 | 5,120 | 2,562 |
| 5 | 20,480 | 10,242 |
| 6 | 81,920 | 40,962 |

> **Mobile target:** Subdivision 5 (~10K cells) for LOD0; higher detail streamed via LOD.

### 3.3 Mesh Integrity (Hex-Sphere)

| Issue | Solution |
|---|---|
| Subdivision mismatch | Consistent edge registry across levels |
| Floating-point duplicates | Vertex snapping (tolerance `1e-6`) after projection |
| LOD seams | Stitching pass at LOD boundaries |
| Dangling edges | Automated validation; wireframe debug mode |

---

## 4. Tectonic Simulation

### 4.1 Plate Generation

```
Input:  seed, plate_count P (8–20)
Output: plate_id map, plate attributes
```

**Steps:**
1. **Blue-noise sampling** → `P` plate centers (avoids clustering)
2. **Toroidal Voronoi partition** → assign each cell to nearest center
3. **Plate attributes** per plate `p`:

| Attribute | Type | Description |
|---|---|---|
| Type | `enum` | Continental / Oceanic |
| Density `ρ_p` | `f32` | Affects subduction |
| Thickness `T_p` | `f32` | Affects base elevation |
| Velocity `v⃗_p` | `vec2` | Plate motion |
| Acceleration `a⃗_p` | `vec2` | Optional (Tier 3) |
| Age | `f32` | Tier 3 |

### 4.2 Boundary Classification

For each adjacent cell pair with different `plate_id`:

1. Compute **relative velocity**: `v⃗_rel = v⃗_A - v⃗_B`
2. Dot product with boundary normal `n̂`:
   - `v⃗_rel · n̂ > threshold` → **Convergent** (→ mountains, subduction)
   - `v⃗_rel · n̂ < -threshold` → **Divergent** (→ rifts, mid-ocean ridges)
   - `|v⃗_rel · n̂| ≤ threshold` → **Transform**

### 4.3 Uplift Field

**Initial uplift** from convergent boundaries:
```
U₀(x,y) = k_conv · max(0, v⃗_rel · n̂) · (convergent ? 1 : 0)
```

**Uplift diffusion** (smooths sharp boundaries):
```
U_{t+1}(x,y) = U_t + λ · Σ_{(i,j)∈N(x,y)} (U_i - U_t)
```
where `λ` is the diffusion coefficient and `N(x,y)` is the 8-neighborhood.

### 4.4 Plate Motion Over Time

```
P_p(t) = (P₀ + v⃗_p · t + ½·a⃗_p · t²) mod (W, H)
```

---

## 5. Elevation Construction

### 5.1 Base Elevation from Tectonics

```
h_tect(x,y) = h₀ + k_M · M_m(x,y) + k_U · U(x,y)
```
where:
- `M_m` = mountain mask (convergent boundaries)
- `U` = uplift field
- `k_M`, `k_U` = weighting coefficients

### 5.2 FBM Noise Blending

Three region masks classify terrain:

| Mask | Region | Noise Field |
|---|---|---|
| `M_m` | Mountains | `N_m` (high-frequency, rugged) |
| `M_p` | Plains | `N_p` (mid-frequency, smooth) |
| `M_o` | Oceans | `N_o` (low-frequency, broad) |

**Blended noise:**
```
N(x,y) = M_m·N_m + M_p·N_p + M_o·N_o
```

**FBM formula:**
```
FBM(x,y) = Σ_{i=0}^{octaves-1} amplitude_i · noise(frequency_i · (x,y))
```
- `octaves`: 3–7 (mobile: 4–5)
- `frequency_i = 2^i · base_freq`
- `amplitude_i = persistence^i · base_amp`
- Typical `persistence`: 0.5

### 5.3 Final Base Heightmap

```
h_base(x,y) = h₀ + k_M·M_m + k_U·U + k_N·N
```

### 5.4 Isostatic Adjustment (Tier 3)

**Airy (local) isostasy:**
```
h_iso(x,y) = k_iso · E_cumulative(x,y)
```

**Flexural isostasy** (Gaussian kernel convolution):
```
h_iso(x,y) = (E_cumulative * K_flex)(x,y)
K_flex(r) = exp(-r² / 2σ_flex²)
```
Implementation: Gaussian blur or FFT convolution.

---

## 6. Climate, Hydrology & Erosion

### 6.1 Climate Model

**Latitude rainfall band (ITCZ):**
```
R_lat = R₀ · e^(-α·(lat - lat_ITCZ)²)
```

**Orographic precipitation** (rain shadow effect):
```
R_oro = k_oro · max(0, ∇h · ŵ)
```
where `ŵ` is the prevailing wind direction.

**Total rainfall:**
```
R(x,y) = R_lat(x,y) + R_oro(x,y)
```

### 6.2 Hydrology — Flow Routing

**D∞ flow algorithm** (for 2D grid): Each cell routes flow to its steepest downslope neighbor, distributing across multiple directions for smoothness.

**Flow accumulation** (topological order processing):
```
A(x,y) = 1 + Σ_{(i,j) ∈ upstream(x,y)} w_ij · A(i,j)
```

**River mask:**
```
R_river(x,y) = A(x,y) > A_threshold ? 1 : 0
```

### 6.3 Erosion

**Stream power erosion:**
```
E = k_E · A^m · S^n
```
- `A` = flow accumulation
- `S` = slope
- `m ≈ 0.5`, `n ≈ 1.0` (typical values)

**Sediment transport capacity:**
```
C = k_C · A^m_C · S^n_C
```

**Thermal erosion** (slope threshold):
```
h_{t+1} = h_t - k_therm · max(0, S - S_crit)
```

**Erosion-deposition rule:**
- If `E > C` → erode (height decreases)
- If `E < C` → deposit (height increases)
- Sediment surplus is transported downstream

### 6.4 Lakes, Basins & Deltas

| Feature | Algorithm |
|---|---|
| Basin detection | Find cells with no downslope neighbor |
| Lake filling | Priority-flood algorithm |
| Delta formation | Sediment deposition at ocean inlets |

---

## 7. Temporal Simulation (Recompute-From-t)

### 7.1 Model

```
t ∈ (-∞, +∞)   (infinite forward and backward)

World state:
W(t) = {P(t), U(t), h(t), R(t), A(t), S(t), C(t)}
```

**Key insight:** Backward time does **not** invert erosion. It recomputes the world from `t = 0` using deterministic equations with `t` as a parameter.

### 7.2 Time-Dependent Quantities

**Plate positions:**
```
P_p(t) = (P₀ + v⃗_p·t + ½·a⃗_p·t²) mod (W,H)
```

**Uplift (integrated tectonic force):**
```
U(t) = ∫₀ᵗ F_tectonic(τ) dτ
```

**Erosion (integrated erosion rate):**
```
E(t) = ∫₀ᵗ F_erosion(τ) dτ
```

**Net elevation:**
```
h(t) = h₀ + U(t) - E(t)
```

### 7.3 API

```
generate(t)           → W(t)
generate(t + Δt)      → forward step
generate(t - Δt)      → backward step (recompute)
generate(t_target)    → jump to arbitrary time
```

### 7.4 Debug Layers (Optional)

- Plate migration paths
- Boundary evolution over time
- Uplift timeline
- Erosion timeline
- River network evolution
- Mountain height evolution

---

## 8. Multi-Scale LOD (Hex-Sphere Only)

### 8.1 LOD Levels

| Level | Viewing Distance | Cell Count | Purpose |
|---|---|---|---|
| LOD0 | 10,000+ km | ~10K | Space view (whole planet) |
| LOD1 | 1,000–10,000 km | ~70K | Orbital view |
| LOD2 | 100–1,000 km | ~190K | Atmospheric entry |
| LOD3 | 10–100 km | ~370K | Regional view |
| LOD4+ | <10 km | Heightmap tiles | Ground exploration |

### 8.2 Subdivision Strategy

```
LOD0:   1 hex cell
LOD1:   7 hex cells   (hex + 6 neighbors)
LOD2:   19 hex cells  (LOD1 + 12 ring)
LOD3:   37 hex cells  (LOD2 + 18 ring)
```

### 8.3 Streaming System

- **Camera-driven:** Only generate LODs near the camera
- **Asynchronous:** Rust/C++ core generates on background threads
- **Triple buffering:** Avoid GPU synchronization stalls
- **Patch lifecycle:** Generate → Upload → Render → Free when distant

### 8.4 LOD4+ Ground Tiles

```
Regional hex → Heightmap tile (128×128 or 256×256)
                ↓
        GPU tessellation / mesh shaders
                ↓
        Micro-noise (rocks, cliffs)
        Material masks (grass, sand, rock)
        Object placement (trees, props)
```

---

## 9. Mobile Performance Optimization

### 9.1 Data Types (Compact)

| Field | Type | Bits |
|---|---|---|
| Elevation | `u16` (0–65535) | 16 |
| Temperature | `u8` (0–255) | 8 |
| Moisture / Precipitation | `u8` | 8 |
| Plate ID | `u16` | 16 |
| Biome | `u8` | 8 |
| Flow direction | `u8` (8 directions) | 8 |
| Flow accumulation | `u16` | 16 |

> **Total per cell:** ~10 bytes (vs. ~48 bytes with naive `f32`/`i32` everywhere)

### 9.2 Structure of Arrays (SoA)

```rust
struct PlanetData {
    elevation: Vec<u16>,      // contiguous
    temperature: Vec<u8>,     // contiguous
    moisture: Vec<u8>,        // contiguous
    plate_id: Vec<u16>,       // contiguous
    biome: Vec<u8>,           // contiguous
    flow_dir: Vec<u8>,        // contiguous
    flow_accum: Vec<u16>,     // contiguous
}
```

**Benefits:** Cache-friendly, SIMD-friendly, minimal memory overhead.

### 9.3 Algorithm Complexity Budget

| Stage | Complexity | Notes |
|---|---|---|
| Voronoi partition | O(P · N) or O(N log P) | P = plates (small) |
| Boundary detection | O(N) | Single pass over cells |
| Uplift diffusion | O(N · iterations) | 1–3 iterations |
| FBM noise | O(N · octaves) | 3–7 octaves |
| Flow accumulation | O(N log N) | Topological sort |
| Erosion | O(N · iterations) | 1–5 iterations |
| Lake filling | O(N log N) | Priority queue |

### 9.4 GPU / Vulkan Considerations

- **Use Vulkan** for maximum performance and control
- Compute shaders for: noise generation, erosion, flow accumulation
- Persistent mapped buffers to avoid CPU-GPU sync stalls
- Triple buffering for LOD patches
- Mesh shaders for LOD4+ terrain tessellation

### 9.5 What to Avoid on Mobile

| Avoid | Reason | Alternative |
|---|---|---|
| Full fluid simulation | O(N²) or worse | Flow accumulation (O(N log N)) |
| Complex multi-pass erosion | Each pass = full N iterations | 1–2 lightweight passes |
| `f64` everywhere | 2× memory, slower | `f32` / `u16` / `u8` |
| Dense grid storage | RAM-heavy | SoA with compact types |
| Naive Lloyd relaxation (100+ iterations) | Slow convergence | Hybrid: few Lloyd + Laplacian |

---

## 10. Data Outputs

### 10.1 Final Maps

| Map | Type | Resolution |
|---|---|---|
| Heightmap | `f32` / `u16` | Per cell |
| Plate map | `u16` | Per cell |
| Boundary map | `u8` categorical | Per cell (0=none, 1=conv, 2=div, 3=transform) |
| River mask | `u8` binary | Per cell |
| Basin map | `u16` | Per cell |
| Erosion map | `f32` | Per cell |
| Rainfall map | `u8` | Per cell |
| Biome map | `u8` | Per cell |

### 10.2 Heightmap Normalization

```
color(x,y) = (h(x,y) - h_min) / (h_max - h_min)
```

---

## 11. Recommended Parameters

| Parameter | Range | Mobile Default |
|---|---|---|
| Plate count | 8–20 | 10 |
| Plate velocity | 0.1–1.0 units/step | 0.3 |
| FBM octaves | 3–7 | 4 |
| FBM persistence | 0.3–0.7 | 0.5 |
| Stream power `k_E` | 0.01–0.05 | 0.02 |
| Sediment `k_C` | Low | Low |
| Thermal erosion threshold `S_crit` | 0.05–0.2 | 0.1 |
| Uplift diffusion `λ` | 0.1–0.5 | 0.3 |
| Erosion iterations | 1–50+ | 2–3 |
| Grid resolution (2D) | 256²–2048² | 512² |
| Hex-sphere subdivisions | 3–6 | 5 |

---

## 12. Complexity Tiers

### Tier 1 — Simple (Prototype / Fast Mobile)

- ✅ Basic plate generation (random centers, Voronoi)
- ✅ Simple boundary classification (convergent/divergent only)
- ✅ Single FBM noise for elevation
- ✅ Basic erosion (1 pass)
- ❌ No climate, no hydrology, no time

### Tier 2 — Intermediate (Full Features)

- ✅ Blue-noise plate sampling
- ✅ Subduction direction
- ✅ Uplift diffusion (1–2 iterations)
- ✅ 3-region FBM blending
- ✅ Climate model (ITCZ + orographic)
- ✅ D∞ flow routing + flow accumulation
- ✅ Stream power + thermal erosion (2–3 iterations)
- ✅ Lake filling
- ✅ Basic temporal recompute

### Tier 3 — Advanced (Simulation Quality)

- ✅ Plate age & fragmentation
- ✅ Dynamic boundaries (plate interactions change over time)
- ✅ Seasonal climate variation
- ✅ Glacial processes
- ✅ Isostatic adjustment (flexural)
- ✅ Full temporal evolution
- ✅ LOD system (hex-sphere)
- ✅ GPU compute shaders

---

## 13. Implementation Roadmap (Mobile-First)

### Phase 1: Core (2D Toroidal, Tier 1–2)

```
1. Toroidal grid + hash functions
2. Plate generation (Voronoi)
3. Boundary classification
4. FBM noise
5. Base elevation
6. Simple erosion (1 pass)
7. Normalize + output heightmap
```

### Phase 2: Climate & Hydrology (Tier 2)

```
8. Climate model (ITCZ, orographic)
9. D∞ flow routing
10. Flow accumulation
11. River mask
12. Stream power erosion + thermal erosion
13. Lake filling + basin detection
```

### Phase 3: Temporal (Tier 2)

```
14. Time-dependent plate motion
15. Time-dependent uplift
16. Time-dependent erosion
17. Recompute-from-t API
```

### Phase 4: Hex-Sphere Upgrade (Optional, Tier 3)

```
18. Icosahedron → subdivision → dual mesh
19. Port all algorithms to hex grid
20. LOD system
21. GPU compute shaders
```

### Phase 5: Advanced (Tier 3)

```
22. Isostasy
23. Glacial processes
24. Seasonal climate
25. Full temporal evolution
```

---

## 14. Key Decisions to Make

| Decision | Options | Recommendation |
|---|---|---|
| Geometry | 2D toroidal vs 3D hex-sphere | **2D toroidal** for mobile MVP |
| Language | C/C++/Rust | **Rust** (safety + performance) or **C++** (ecosystem) |
| Rendering | Vulkan vs OpenGL ES | **Vulkan** if targeting high-end; **GLES 3.1** for compatibility |
| Time model | Recompute-from-t vs stateful | **Recompute-from-t** (deterministic, less memory) |
| Erosion detail | 1 pass vs iterative | **1–3 passes** for mobile |
| LOD | Built-in from start vs added later | **Later** (only needed for hex-sphere) |

---

## 15. Quick Reference — All Formulas

### Plate Generation
```
d_toroidal = √(min(|x-cx|, W-|x-cx|)² + min(|y-cy|, H-|y-cy|)²)
```

### Uplift Diffusion
```
U_{t+1} = U_t + λ · Σ_neighbors (U_n - U_t)
```

### FBM Noise
```
FBM(x,y) = Σ octaves · amplitude_i · noise(2^i · frequency · (x,y))
```

### Base Elevation
```
h_base = h₀ + k_M·M_m + k_U·U + k_N·N
```

### Rainfall
```
R = R₀·e^(-α·(lat-lat_ITCZ)²) + k_oro·max(0, ∇h·ŵ)
```

### Flow Accumulation
```
A(x,y) = 1 + Σ_upstream w·A
```

### Stream Power Erosion
```
E = k_E · A^m · S^n
```

### Sediment Capacity
```
C = k_C · A^m_C · S^n_C
```

### Thermal Erosion
```
h_{t+1} = h_t - k_therm · max(0, S - S_crit)
```

### Isostasy (Airy)
```
h_iso = k_iso · E_cumulative
```

### Isostasy (Flexural)
```
h_iso = (E_cumulative * K_flex)   where K_flex(r) = e^(-r²/2σ²)
```

### Plate Motion
```
P_p(t) = (P₀ + v⃗_p·t + ½·a⃗_p·t²) mod (W,H)
```

### Net Elevation Over Time
```
h(t) = h₀ + U(t) - E(t) = h₀ + ∫₀ᵗ F_tectonic(τ)dτ - ∫₀ᵗ F_erosion(τ)dτ
```

---

## 16. Summary — Recommended Architecture for Mobile

**Start with:** 2D toroidal grid, 512×512, Tier 2 features, Rust or C++ core, deterministic recompute-from-t.

**Data:** SoA with compact types (`u8`, `u16`, `f32`). Target ≤ 10 bytes per cell for core maps.

**Algorithms:** All O(n) or O(n log n). 1–3 erosion iterations. 4 FBM octaves. 10 plates.

**If hex-sphere is needed:** Add LOD system later. Subdivision 5 for LOD0. Stream LOD1–3 asynchronously. Use Vulkan compute shaders for generation offload.

**Avoid:** Full fluid simulation, heavy multi-pass erosion, `f64` math, dense per-cell structs.

---

*This organized specification consolidates all notes, removes duplicates, and provides a clear implementation path focused on mobile performance.*
