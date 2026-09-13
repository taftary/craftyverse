## Visibility Specification (`crates/engine/src/visibility/`)

### Overview

The `visibility` module is the camera-driven culling pass of the planet
runtime (feature 4 of `plan/features/04-visibility.md`). It is pure glam
geometry - no GPU, window, or render dependency - and runs two
conservative tests over the active chunk set: frustum culling against the
camera view volume and horizon culling against the planet body. It is a
read-only consumer of the active chunk set: it never influences LOD or
loading (Decision 1 of `plan/RELATED.md` - LOD and loading follow the
player, culling follows the player camera; the navigation spectator
camera never re-culls and sees whatever is loaded around the player,
gaps included).

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
  - `PlanetHorizon::morphed(origin, radius, anchor_up, flatten)` - the
    conservative occlusion body while the ground-flattening morph is
    active (feature 5): the sphere inscribed in the morphed ellipsoid
    (see "Horizon test math" below). At `flatten == 0` this is exactly
    `new`; at `flatten == 1` the radius is 0 and nothing is culled.
  - `PlanetHorizon::occludes(camera, &BoundingSphere) -> bool` - whether
    the sphere is certainly hidden behind the planet limb from the
    camera. See "Horizon test math" below.
- **Pass** (`pass.rs`)
  - `ChunkBounds` - the culling input of one active chunk (its bounding
    sphere). `ChunkBounds::from_node(&NodeRef, skirt_margin)` is the
    read-only extraction from the node graph (spherical geometry);
    `ChunkBounds::from_triangle(center, vertices, margin)` is the
    morph-aware variant, fed with the already-morphed corners while the
    flatten factor is nonzero. The pass itself consumes plain data only.
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

### Morph-aware culling (feature 5 interaction)

While the ground-flattening morph is active (sky and terrain layers,
`flatten_factor > 0`), the terrain vertex shader displaces every vertex
toward the tangent plane at the anchor by up to `flatten_factor x height
above the plane` - far beyond the chunk bounding radius near the
active-zone edge. Culling against the unmorphed spherical volumes drops
chunks that are visibly rendered (the disappearing-mesh bug). The pass
therefore tests the RENDERED geometry:

- **Morphed chunk bounds.** The morph is affine, so the morphed chunk is
  exactly the triangle through the morphed corners, the morphed center is
  the morph of the center, and the skirt displacement only contracts
  under it. `ChunkBounds::from_triangle` built from the morphed corners
  ([`morph_point`](runtime.md)) with the usual skirt margin covers the
  rendered chunk at every factor.
- **Morphed occlusion body.** With `up` the anchor radial, a sphere
  point `O + q_perp + t * up` (`|q| = radius`) maps to
  `(O + flatten * radius * up) + q_perp + t * (1 - flatten) * up`: the
  surface sphere morphs into an ellipsoid of center
  `O + flatten * radius * up` with lateral semi-axis `radius` and
  vertical semi-axis `radius * (1 - flatten)`. Occlusion is monotone in
  the occluder (a segment hitting an inner body crosses the rendered
  shell), so the sphere inscribed in that ellipsoid
  (`PlanetHorizon::morphed`) only ever culls certainly-hidden chunks. At
  full flatten its radius is 0: the flat plane hides nothing behind a
  limb, and horizon culling switches off exactly when the spherical
  horizon ceases to exist.

### Rules

- **Separation of concerns.** Loading follows the player (the LOD
  scheduler's active zone); culling follows the player camera in every
  camera mode. The pass consumes the camera position and view-projection
  only and never feeds back into the scheduler.
- **Conservative culling.** A chunk is culled only when certainly
  invisible; every marginal case stays visible. There is no popping at
  the horizon line, and while the ground-flattening morph is active the
  pass tests the morphed (rendered) geometry, never the unmorphed
  sphere.
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
  (`chunk_bounds`), so the crack-masking skirts stay inside the culled
  volume; while the flatten factor is nonzero they are built from the
  morphed corners instead (`morphed_chunk_bounds`), and the horizon
  occluder is `PlanetHorizon::morphed` - the pass always tests the
  rendered geometry.
- The frustum is rebuilt from the PLAYER camera's view-projection every
  frame.
- Only the pool slots of surviving chunks are drawn (one draw batch per
  visible live slot).
- **Camera model.** Two cameras exist. The player camera (default) is
  first-person, attached to the player: flying moves the player,
  LOD/loading follow the player, and culling follows this camera in
  every mode. The **F** key toggles the navigation camera, a free-fly
  spectator for navigating space: it changes only the rendered viewpoint
  - no re-culling (it sees whatever the player camera would show, gaps
  included, the Decision 1 detached-camera contract), no LOD/loading
  influence, no overlay changes beyond the mode readout. Entering
  navigation mode continues from the current view; switching back
  returns to the player camera view. **R** respawns the player camera
  and returns to player mode. A small world-space cross
  (`player_marker_vertices`) marks the player position in both modes,
  drawn through the textured pipeline with the same anchor-relative
  morph constants as the terrain.
- The overlay gained the visibility readouts (`visibility_lines`):
  active camera (`player` / `navigation`), chunks visible vs tested,
  frustum and horizon cull counts, and terrain draw calls.

### Files

- `crates/engine/src/visibility/frustum.rs` - `BoundingSphere`,
  `Frustum`.
- `crates/engine/src/visibility/horizon.rs` - `PlanetHorizon`.
- `crates/engine/src/visibility/pass.rs` - `ChunkBounds`, `cull_chunks`,
  `VisibilityReport`.
- `crates/engine/src/render/runtime_window.rs` - the per-frame pass over
  the active set, the visible-slot draw filtering, the player/navigation
  camera model, the player marker, and the `visibility_lines` overlay
  formatter.
- `tests/visibility/` - headless tests: frustum plane cases, horizon
  far-side/near-side/limb cases, a descending-camera no-popping sweep,
  count consistency, read-only behavior, the player-camera culling
  contract, and the morph-aware regression tests (descent into the
  sky/terrain layers with the real morph math).
