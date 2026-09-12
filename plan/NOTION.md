# 🌍 **Procedural Planet Runtime Specification**

### **Goal**
Build a mobile‑optimized procedural planet runtime system where the player can move freely around a planet, transition through all planetary layers (space → orbit → atmosphere → sky → terrain), and only the necessary terrain chunks are active.

## **Planet**

- Planet generation is already completed.
- Planet radius is known.
- Atmosphere radius must be calculated from planet radius using a configurable multiplier.
- Atmosphere must remain **curved**, following the planet’s curvature.
- Planet must smoothly transform from a **sphere** (far) to a **flat terrain** (ground level).
- Ground level must behave as a **flat plane**, not curved, when the player is close enough.

## **Player**

- Player has a world‑space position.
- Player orientation does not affect LOD or atmosphere logic.
- System must use **player position** to determine:

- Distance to planet center
- Direction vector from player to planet center
- Runtime mode (space / orbit / atmosphere / sky / terrain)

## **Planetary Layers**
The runtime must support the following ordered layers:

1. **Space Layer**

- Player is far enough to be considered detached from orbit.
- Planet appears extremely small or simplified.
- No atmosphere effects.
2. **Orbit Layer**

- Player is close enough to be considered in orbital range.
- Planet is spherical.
- Low‑detail rendering.
3. **Atmosphere Layer**

- Player enters the curved atmosphere shell.
- Atmospheric scattering and color transitions begin.
4. **Sky Layer**

- Player is inside the lower atmosphere.
- Sky color, fog, and horizon effects dominate.
5. **Terrain Layer**

- Player reaches ground level.
- Terrain becomes **flat**, replacing spherical curvature.

## **Runtime Systems Needed**

### **1. Planet Runtime Manager**

- Reads **player position** each frame.
- Computes:

- Distance to planet center
- Atmosphere radius (planetRadius × multiplier)
- Determines current layer:

- Space
- Orbit
- Atmosphere
- Sky
- Terrain
- Communicates layer to other systems.

### **2. Chunk LOD Runtime System**

- Uses existing planet generation.
- Only manages:

- Chunk activation/deactivation
- Chunk subdivision/merge
- Frustum culling
- Horizon culling
- LOD decisions based solely on **player distance to chunk**.
- Must limit LOD operations per frame for mobile performance.

## **Chunk Visibility Requirement**

- **Only the side of the planet where the player is located should be rendered.**
- All other sides must not be rendered.
- When the player moves, visible chunks must be recalculated.
- Goal: **reduce mesh count as much as possible**.

## **Dynamic Mesh Loading Requirement**

- Planet meshes are already connected.
- As the player moves:

- Chunks must **load** when entering the player’s active zone.
- Chunks must **unload** when leaving the player’s active zone.
- Loading/unloading must be continuous and distance‑based.
- System must always maintain the minimum number of active meshes.

## **Mesh Reuse Requirement**

- The system must **always reuse the same mesh objects**.
- Splitting and unsplitting must **only update vertex positions**.
- No new mesh objects must be created for each chunk or LOD level.
- All LOD transitions must be handled through:

- Vertex displacement
- Vertex refinement
- Vertex simplification
- Mesh pooling must be used for all runtime operations.

## **Ground Flattening Requirement**

- As the player approaches ground level:

- The spherical terrain must **smoothly transition** into a **flat terrain**.
- The flat terrain must replace the curved geometry under the player.
- The transition must be seamless and distance‑based.

## **Atmosphere Requirement**

- Atmosphere must remain **curved**, following the planet’s curvature.
- Atmosphere appearance must change based on **player distance**:

- Space → no atmosphere
- Orbit → thin rim
- Atmosphere → curved scattering layer
- Sky → fog and horizon
- Terrain → full sky
- Uses normalized distance factor for blending.

### **4. Rendering Flow Controller**

- Switches rendering behavior based on **player layer**:

- Space: minimal planet
- Orbit: spherical low‑detail planet
- Atmosphere: curved scattering
- Sky: fog, horizon, sky color
- Terrain: flat ground
- Controls shader detail levels and visual transitions.
- Handles smooth transformation from **sphere** to **flat terrain** as player approaches ground.

## **Inline LOD Zoom Requirement (Fibonacci‑based)**

- System must use a **Fibonacci‑based inline LOD zoom method**.
- When zooming **in**:

- New vertex‑refined meshes must be created **under** the current meshes.
- Zoom transitions must follow Fibonacci‑based scaling.
- When zooming **out**:

- The process must be reversed.
- Higher‑level meshes replace lower‑level ones smoothly.
- All zoom operations must reuse the same mesh objects.

## **Mobile Performance Requirements**

- Minimal draw calls.
- Only nearby chunks active.
- Only player‑side chunks rendered.
- Mesh pooling.
- Async vertex updates.
- Single terrain shader.
- Single curved‑atmosphere shader.
- No per‑frame CPU terrain generation.
- Hard limit on LOD operations per frame.
