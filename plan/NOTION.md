# 🌍 **Procedural Planet Runtime Specification**

### **Goal**
Build a procedural planet runtime system (desktop Vulkan baseline, designed within mobile performance budgets) where the player can move freely around a planet, transition through all planetary layers (space → orbit → atmosphere → sky → terrain), and only the necessary terrain chunks are active.

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
- Publishes the authoritative flattening blend factor and floating‑origin anchor each frame (see Ground Flattening Requirement).
- Communicates layer to other systems.

### **2. Chunk LOD Runtime System**

- Uses existing planet generation.
- Only manages:

- Chunk activation/deactivation
- Chunk subdivision/merge
- LOD decisions based solely on **player position**: distance from the player to the chunk center. Player orientation never participates.
- Split and merge use separate thresholds with hysteresis: merge threshold = split threshold × 1.3 per level.
- LOD distance thresholds follow a geometric progression (×2 per level), matching the subdivision hierarchy.
- Restricted subdivision: the level difference across any shared edge is at most 1; chunk‑border skirts mask the residual seams.
- Must limit LOD operations per frame (configurable budget, initially 2 split/merge operations per frame).
- Frustum culling and horizon culling are **not** LOD operations; they belong to the camera-driven visibility pass (see Chunk Visibility Requirement).

## **Chunk Visibility Requirement**

- Loading and rendering are separate concerns:
- **Loading** follows the player: the active zone is a full sphere around the player position, independent of orientation.
- **Rendering** follows the camera: chunks are culled by the camera frustum and by the planet horizon.
- The camera may detach from the player; LOD and loading still follow the player, culling still follows the camera. A detached camera sees whatever is loaded around the player, including gaps.
- When the player or camera moves, visible chunks must be recalculated.
- Goal: **reduce rendered mesh count as much as possible**.

## **Dynamic Mesh Loading Requirement**

- Planet meshes are already connected.
- As the player moves:

- Chunks must **load** when entering the player’s active zone.
- Chunks must **unload** when leaving the player’s active zone.
- The active zone is a sphere around the player position (see Chunk Visibility Requirement).
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
- No parent/child layered rendering during LOD transitions; a transition only rewrites vertex data in pooled slots.
- Mesh pooling must be used for all runtime operations.

## **Ground Flattening Requirement**

- Purpose: simpler ground gameplay - movement, building, and physics use flat local math near the ground.
- As the player approaches ground level:

- The spherical terrain must **smoothly transition** into a **flat terrain** through a **shader-side morph**: a uniform blend factor displaces vertices toward the local tangent plane in the vertex shader.
- The morph applies to the **whole visible region** with one global blend factor; the visible flattening of terrain during descent is an intended effect.
- The transition must be seamless and distance‑based, blended across the sky layer.
- Gameplay transitions with it: gravity, movement, and building blend from spherical (radial gravity) to flat (fixed down) using the same normalized factor as the shader.
- The flat frame is a **continuous floating origin**, re‑anchored to the player’s ground projection every frame (this also bounds float32 precision error far from the planet center).
- One authoritative blend factor and anchor is published per frame so rendering, physics, and picking never disagree.
- Pooled vertex data stays spherical; no CPU vertex rewrites are used for flattening.

## **Atmosphere Requirement**

- Atmosphere must remain **curved**, following the planet’s curvature.
- Atmosphere appearance must change based on **player distance**:

- Space → no atmosphere
- Orbit → thin rim
- Atmosphere → curved scattering layer
- Sky → fog and horizon
- Terrain → full sky
- Uses normalized distance factor for blending.
- At full ground flattening, the curved shell reads as a sky dome above flat ground; the shell itself stays curved.

### **4. Rendering Flow Controller**

- Switches rendering behavior based on **player layer**:

- Space: minimal planet
- Orbit: spherical low‑detail planet
- Atmosphere: curved scattering
- Sky: fog, horizon, sky color
- Terrain: flat ground
- Controls shader detail levels and visual transitions.
- Owns the camera‑driven visibility pass: frustum culling and horizon culling.
- Applies the smooth transformation from **sphere** to **flat terrain** using the authoritative blend factor and floating origin published by the Planet Runtime Manager.

## **LOD Zoom Transitions**

- "Zoom" means the player approaching or leaving; it is handled entirely by the distance‑based Chunk LOD Runtime System. There is no separate zoom system.
- (Superseded: the earlier Fibonacci‑based inline zoom requirement. Distance thresholds follow a geometric ×2 progression instead, matching the subdivision hierarchy.)
- When zooming **in**:

- Chunks split by rewriting vertex data in pooled mesh slots.
- No new mesh objects are created for any chunk or LOD level.
- When zooming **out**:

- The process is reversed: coarser vertex data replaces finer data in the same pooled slots.
- All zoom operations reuse the same mesh objects.

## **Performance Requirements**

- Desktop Vulkan is the validated baseline; the mobile rules below are design constraints, not validated targets (no mobile build or CI exists yet).
- Minimal draw calls.
- Only nearby chunks active (active zone sphere around the player).
- Only camera‑visible chunks rendered (frustum + horizon culling).
- Mesh pooling.
- Fully asynchronous vertex updates: worker threads with double‑buffered staging; workers consume extracted geometry data, not node graph references (the graph is single‑threaded).
- Single terrain shader.
- Single curved‑atmosphere shader.
- No per‑frame CPU terrain generation.
- Hard, configurable limit on LOD operations per frame (initially 2;
  the runtime window tunes to 8 with the camera-aware scheduler).

## **Enhancement Addendum (Features 7-8)**

- LOD refinement is hybrid: loading follows the player sphere, splits
  react to the nearer of the player and the draw camera, merges require
  the nearer distance to clear the higher merge threshold (both
  viewpoints far to coarsen; same geometric thresholds and hysteresis).
- A global coarse shell (every chunk at or below the level floor) stays
  loaded from any distance, so deep space always shows a closed planet.
- Terrain-level ground is exactly flat, subdivided to level 7 only near
  the player (active zone shrinks from 0.75 radii in orbit to 0.05 radii
  at the surface), with micro shading detail and horizon haze selling
  the scale. No height displacement.
