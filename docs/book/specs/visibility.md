## Visibility Specification (`crates/engine/src/visibility/`)

### Overview

The `visibility` module is the camera-driven culling pass of the planet
runtime (feature 4 of `plan/features/04-visibility.md`). It is pure glam
geometry - no GPU, window, or render dependency - and runs two
conservative tests over the active chunk set: frustum culling against the
camera view volume and horizon culling against the planet body. It is a
read-only consumer of the active chunk set: it never influences LOD or
loading (Decision 1 of `plan/RELATED.md` - LOD and loading follow the
player, culling follows the camera, and a detached camera sees whatever
is loaded around the player, gaps included).

Status: **Current baseline** for frustum culling, horizon culling, and
the runtime-window integration (only visible chunk slots are drawn, and
the overlay reports the culling counts). The atmosphere shell (feature 6)
is **Current baseline** (see [`atmosphere`](atmosphere.md)); it is not
culled - the pass culls terrain chunks only.

### Structure

- **Frustum** (`frustum.rs`)
  - `BoundingSphere { center, radius }` - the conservative bounding
    volume of one chunk. `BoundingSphere::from_triangle(center, vertices,
    margin)` centers the sphere on the chunk center and covers every
    corner plus `margin` (the skirt depth, so the downward crack-masking
    flanges stay inside the volume).
  - `Frustum::from_view_projection(mvp)` - the six view-volume planes
    (left, right, bottom, top, near, far) extracted with the
    Gribb-Hartmann row-combination method. The matrix convention matches
    the engine's cameras: right-handed, y-up, NDC depth `0..=1` (the
    `directx` projection of the runtime window's fly camera), so the near
    plane is `z >= 0` in clip space. Each plane is normalized.
  - `Frustum::contains_sphere(&BoundingSphere) -> bool` - a sphere is
    culled only when its closest point is outside one plane (signed
    distance below `-radius`), so the test is conservative.
- **Horizon** (`horizon.rs`)
  - `PlanetHorizon::new(origin, radius)` - the planet occlusion body:
    the surface sphere the chunks live on.
  - `PlanetHorizon::occludes(camera, &BoundingSphere) -> bool` - whether
    the sphere is certainly hidden behind the planet limb from the
    camera. See "Horizon test math" below.
- **Pass** (`pass.rs`)
  - `ChunkBounds` - the culling input of one active chunk (its bounding
    sphere). `ChunkBounds::from_node(&NodeRef, skirt_margin)` is the
    single read-only extraction point from the node graph; the pass
    itself consumes plain data only.
  - `cull_chunks(&[ChunkBounds], camera, &Frustum, &PlanetHorizon) ->
    VisibilityReport` - the per-frame pass: frustum first, horizon
    second, in input order.
  - `VisibilityReport { tested, visible, frustum_culled, horizon_culled
    }` - the visible indices into the input slice plus the per-test cull
    counts for the debug overlay.

### Horizon test math

From a camera at distance `h > radius` from the planet center, the limb
is the tangent cone toward the planet with half-angle
`theta = asin(radius / h)`. A point is occluded iff its direction from
the camera lies inside the cone and its distance exceeds the near sphere
intersection along that ray (at most the tangent distance
`sqrt(h^2 - radius^2)`, reached at the cone boundary). A sphere is
culled when both hold for every point of it:

1. **Cone containment.** The sphere's angular radius about the camera,
   `alpha = asin(sphere_radius / d)` (`d` = center distance), added to
   its center angle `phi` off the cone axis, stays within `theta`:
   `cos(phi + alpha) >= cos(theta)`, evaluated with the cosine sum
   formula (all angles in `[0, pi]`, where cosine is monotone).
2. **Radial distance.** The sphere's nearest point lies beyond the
   tangent distance: `d - sphere_radius > sqrt(h^2 - radius^2)`.

The test errs toward visible in every marginal case, so nothing pops at
the horizon line while the camera moves:

- A camera at or below the surface radius (`h <= radius`) culls nothing
  (near-ground and inside-atmosphere flight keeps the full loaded set).
- A sphere that pokes outside the tangent cone, however slightly, stays
  visible - chunks straddling the limb are never culled.
- The radial test uses the tangent distance, the worst case over the
  whole cone, never the per-direction intersection.
- A sphere containing the camera is never culled.

### Rules

- **Separation of concerns.** Loading follows the player (the LOD
  scheduler's active zone); rendering follows the camera. The pass
  consumes the camera position and view-projection only and never feeds
  back into the scheduler.
- **Conservative culling.** A chunk is culled only when certainly
  invisible; every marginal case stays visible. There is no popping at
  the horizon line.
- **Goal: minimal draw calls.** Only chunks surviving both tests are
  drawn; the runtime window reports tested vs visible counts, the
  per-test cull counts, and the issued terrain draw calls.
- **Recalculation.** The pass reruns every frame (the camera moves on
  most frames); it is cheap plain math over the active set.
- **Read-only.** The node graph is touched only by
  `ChunkBounds::from_node`, which borrows the node and copies out the
  center and corners.

### Runtime window integration

The runtime window (`crates/engine/src/render/runtime_window.rs`) runs
the pass every frame over `LodScheduler::active_chunks()`:

- Chunk bounds are extracted with the border-skirt depth as the margin
  (`chunk_bounds`, reusing the mesh pool's skirt depth factor), so the
  crack-masking skirts stay inside the culled volume.
- The frustum is rebuilt from the fly camera's view-projection every
  frame; the horizon comes from the planet configuration.
- Only the pool slots of surviving chunks are drawn (one draw batch per
  visible live slot).
- The **F** key detaches the camera from the player proxy: the player
  freezes in place (LOD and loading keep following it) while the camera
  keeps flying; re-attaching snaps the player back to the camera. **R**
  respawns and re-attaches. Culling always follows the camera, so a
  detached camera sees whatever is loaded around the player, including
  gaps.
- The overlay gained the visibility readouts (`visibility_lines`):
  camera attached/detached state, chunks visible vs tested, frustum and
  horizon cull counts, and terrain draw calls.

### Files

- `crates/engine/src/visibility/frustum.rs` - `BoundingSphere`,
  `Frustum`.
- `crates/engine/src/visibility/horizon.rs` - `PlanetHorizon`.
- `crates/engine/src/visibility/pass.rs` - `ChunkBounds`, `cull_chunks`,
  `VisibilityReport`.
- `crates/engine/src/render/runtime_window.rs` - the per-frame pass over
  the active set, the visible-slot draw filtering, the detach toggle,
  and the `visibility_lines` overlay formatter.
- `tests/visibility/` - headless tests: frustum plane cases, horizon
  far-side/near-side/limb cases, a descending-camera no-popping sweep,
  count consistency, read-only behavior, and the detached-camera
  separation.
