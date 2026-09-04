Procedural Tectonic World Generator

1. Introduction

This document defines a physically‑inspired 2D world generator based on tectonic plate motion, crust deformation, uplift, erosion, climate, hydrology, and unbounded temporal evolution.The output is a seamless toroidal grayscale heightmap with deterministic generation from a seed.

The system supports infinite forward and backward time travel, where world states are recomputed from time (t), ensuring full determinism without needing to physically reverse erosion.

2. High‑Level Goals

Realistic tectonic terrain

Time‑based uplift, erosion, and sedimentation

Infinite forward/backward time navigation

Hydraulic + thermal erosion

Climate‑driven rainfall

Natural river networks

Toroidal world

Deterministic generation

Modular complexity tiers

3. System Architecture

Subsystems

Plate generation

Tectonic simulation

Elevation construction

Climate & hydrology

Temporal simulation

Final normalization

Each subsystem is modular and replaceable.

4. Generation Pipeline

Step 1 — Plate Generation

Blue‑noise sampling → plate centers

Toroidal Voronoi partition → plate ID map

Plate attributes:

Type (continental/oceanic)

Density (\rho_p)

Thickness (T_p)

Velocity (\vec{v}_p)

Optional acceleration (\vec{a}_p)

Age (Tier 3)

3.x Toroidal Voronoi Distance

Distance between cell ((x,y)) and plate center ((c_x, c_y)):

[ d_x = \min(|x - c_x|,; W - |x - c_x|) ]

[ d_y = \min(|y - c_y|,; H - |y - c_y|) ]

Toroidal distance:

[ d(x,y) = \sqrt{d_x^2 + d_y^2} ]

This ensures seamless wrap‑around and correct plate interactions across edges.

Step 2 — Tectonic Interaction

Boundary detection

Relative velocity field

Boundary classification: convergent, divergent, transform

Subduction direction

Uplift field

Uplift diffusion:

[ U_{t+1} = U_t + \lambda \sum_{(i,j) \in N(x,y)} (U_i - U_t) ]

Step 3 — Base Elevation Construction

The base heightmap for the sphere is generated using a procedural algorithm combining fractal noise, Voronoi-based tectonic plates, and physical simulation of plate interactions. The key components are:

Hash functions: Generate pseudo-random values for noise and plate positioning.

Fractional Brownian Motion (FBM): Create fractal noise for terrain detail.

Toroidal wrapping: Ensure seamless tiling of the heightmap on a spherical surface.

Voronoi partitioning: Define tectonic plates by partitioning the surface into cells.

Plate velocity and collision fields: Simulate relative plate motions and collisions to model uplift and deformation.

Crust thickness variation: Modulate elevation based on plate type and noise.

Elevation calculation: Combine crust thickness and collision effects to compute base elevation.

Erosion and river effects: Apply smoothing and river carving to refine terrain features.

The algorithm proceeds as follows:

Compute plate IDs using a toroidal Voronoi partition with seeded randomness.

Calculate plate velocities as normalized random vectors.

Determine collision forces from relative plate velocities to simulate tectonic uplift.

Generate crust thickness using base values modulated by FBM noise.

Combine crust thickness and collision uplift to form the base elevation.

Apply erosion smoothing by averaging neighboring elevations.

Simulate river valleys by identifying low-slope regions and carving channels.

This approach produces a seamless, evolving heightmap suitable for spherical worlds, fully deterministic from a seed and time parameter.

Step 4 — Climate, Erosion & Rivers

Climate

Latitude rainfall band:

[ R_{lat} = R_0 e^{-\alpha(lat - lat_{ITCZ})^2} ]

Orographic precipitation:

[ R_{oro} = k_{oro} \max(0, \nabla h \cdot \hat{w}) ]

Final rainfall:

[ R = R_{lat} + R_{oro} ]

Hydrology

D∞ flow routing

Flow accumulation:

[ A(x,y) = 1 + \sum_{(i,j) \in U(x,y)} w_{ij} A(i,j) ]

River mask:

[ R_{river}(x,y) = \begin{cases} 1 & A(x,y) > A_{thr} \ 0 & \text{otherwise} \end{cases} ]

Erosion

Stream power:

[ E = k_E A^m S^n ]

Sediment capacity:

[ C = k_C A^{m_C} S^{n_C} ]

Thermal erosion:

[ h_{t+1} = h_t - k_{therm}(S - S_{crit}) ]

Lakes, Basins, Deltas

Priority‑flood lake filling

Basin detection

Delta formation at ocean boundaries

---

Step 1 — Plate Generation

Blue‑noise sampling → plate centers

Toroidal Voronoi partition → plate ID map

Plate attributes:

Type (continental/oceanic)

Density (\rho_p)

Thickness (T_p)

Velocity (\vec{v}_p)

Optional acceleration (\vec{a}_p)

Age (Tier 3)

3.x Toroidal Voronoi Distance

Distance between cell ((x,y)) and plate center ((c_x, c_y)):

[ d_x = \min(|x - c_x|,; W - |x - c_x|) ]

[ d_y = \min(|y - c_y|,; H - |y - c_y|) ]

Toroidal distance:

[ d(x,y) = \sqrt{d_x^2 + d_y^2} ]

This ensures seamless wrap‑around and correct plate interactions across edges.

Step 2 — Tectonic Interaction

Boundary detection

Relative velocity field

Boundary classification: convergent, divergent, transform

Subduction direction

Uplift field

Uplift diffusion:

[ U_{t+1} = U_t + \lambda \sum_{(i,j) \in N(x,y)} (U_i - U_t) ]

Step 3 — Base Elevation Construction

Crust elevation from plate type/thickness

Region masks: mountains (M_m), plains (M_p), oceans (M_o)

FBM noise blending:

[ N = M_m N_m + M_p N_p + M_o N_o ]

Base elevation:

[ h_{base} = h_0 + k_M M_m + k_U U + k_N N ]

Step 4 — Climate, Erosion & Rivers

Climate

Latitude rainfall band:

[ R_{lat} = R_0 e^{-\alpha(lat - lat_{ITCZ})^2} ]

Orographic precipitation:

[ R_{oro} = k_{oro} \max(0, \nabla h \cdot \hat{w}) ]

Final rainfall:

[ R = R_{lat} + R_{oro} ]

Hydrology

D∞ flow routing

Flow accumulation:

[ A(x,y) = 1 + \sum_{(i,j) \in U(x,y)} w_{ij} A(i,j) ]

River mask:

[ R_{river}(x,y) = \begin{cases} 1 & A(x,y) > A_{thr} \ 0 & \text{otherwise} \end{cases} ]

Erosion

Stream power:

[ E = k_E A^m S^n ]

Sediment capacity:

[ C = k_C A^{m_C} S^{n_C} ]

Thermal erosion:

[ h_{t+1} = h_t - k_{therm}(S - S_{crit}) ]

Lakes, Basins, Deltas

Priority‑flood lake filling

Basin detection

Delta formation at ocean boundaries

4.x Isostatic Adjustment

Airy Isostasy (Local Compensation)

[ h_{iso}(x,y) = k_{iso} \cdot E_c(x,y) ]

Where (E_c(x,y)) is cumulative crustal erosion and (k_{iso}) a rebound coefficient.

Flexural Isostasy (Lithospheric Bending)

[ h_{iso}(x,y) = (E_c * K_{flex})(x,y) ]

Gaussian kernel:

[ K_{flex}(r) = \exp\left(-\frac{r^2}{2\sigma_{flex}^2}\right) ]

Implemented via Gaussian blur or FFT convolution.

5. Temporal Simulation (Recompute‑From‑t Model)

5.1 Infinite Time Model

[ t \in (-\infty, +\infty) ]

World state:

[ W(t) = {P(t), U(t), h(t), R(t), A(t), S(t), C(t)} ]

5.2 Forward and Backward Time

Forward: plates move, uplift accumulates, erosion increases.

Backward: world is recomputed at earlier (t), producing a younger landscape.

Backward time does not invert erosion; it recomputes the world from initial conditions using deterministic equations.

5.3 Plate Motion Over Time

[ X_p(t) = (X_0 + \vec{v}_p t + \tfrac{1}{2}\vec{a}_p t^2) \mod (W, H) ]

5.4 Time‑Dependent Uplift

[ U(t) = \int_0^t F_{tectonic}(\tau), d\tau ]

5.5 Time‑Dependent Erosion

[ E(t) = \int_0^t F_{erosion}(\tau), d\tau ]

Elevation:

[ h(t) = h_0 + U(t) - E(t) ]

5.6 Deterministic Time Navigation API

(W(t + \Delta t))

(W(t - \Delta t))

(W(t_{target}))

5.7 Temporal Debug Layers

Plate migration paths

Boundary evolution

Uplift timeline

Erosion timeline

River network evolution

Mountain height evolution

6. Toroidal Normalization

Toroidal wrap:

[ x' = (x + W) \mod W,\quad y' = (y + H) \mod H ]

Height normalization:

[ color(x,y) = \frac{h(x,y) - h_{min}}{h_{max} - h_{min}} ]

7. Data Formats

Heightmap: float

Plate map: int

Boundary map: categorical

River mask: binary

Basin map: int

Erosion map: float

Rainfall map: float

8. Suggested Parameters

Plates: 8–20

Velocity: 0.1–1.0

FBM octaves: 3–7

Stream power (k_E): 0.01–0.05

Sediment transport: low

Thermal erosion: small slope threshold

Iterations: Tier 1 (1–3), Tier 2 (5–20), Tier 3 (50+)

9. Complexity Tiers

Tier 1 — Simple

Basic plates, simple boundaries, single FBM, basic erosion.

Tier 2 — Intermediate

Subduction, uplift diffusion, climate rainfall, rivers.

Tier 3 — Advanced

Plate age, fragmentation, dynamic boundaries, seasonal climate, glacial processes, full temporal evolution.

10. Final Outputs

Final heightmap

Plate map

Boundary map

River mask

Basin map

Erosion map

Rainfall map

Temporal evolution layers



Use Vulkan if you need maximum performance, advanced graphics features, and are prepared for more complex development.
