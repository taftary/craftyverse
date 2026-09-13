## Runtime Module Definition (`crates/engine/src/runtime/`)

### Overview

The `runtime` module is the headless core of the planet runtime manager
(phase 1 of the procedural planet runtime, `plan/features/01-planet-runtime-manager.md`).
It is pure math over `glam`: no GPU, window, winit, or node-graph dependency,
so it is fully unit-testable headless.

Each frame, `PlanetRuntimeManager::update` reads the player position and
publishes a `RuntimeState`: distance and direction to the planet center, the
current `PlanetaryLayer`, normalized atmosphere and flattening blend factors,
the floating-origin anchor, and an anchor-relative local frame. Player
orientation never participates - every output is a function of position only.

Status: **Current baseline**. The player proxy (a fly-mode camera) and the
runtime debug window of feature 1 are also **Current baseline**; they live
in the `render` module (see "Runtime Window" in [`render`](render.md)).
The LOD scheduler (feature 2) is **Current baseline** as a headless module;
see [`lod`](lod.md). Visibility (feature 4) is **Current baseline**; see
[`visibility`](visibility.md). The ground-flattening math (feature 5) is
**Current baseline** (`flatten.rs`, below; the shader-side morph that
consumes it is in [`render`](render.md)). The curved atmosphere shell
(feature 6) is **Current baseline**; see
[`atmosphere`](atmosphere.md).

### Structure

- **Configuration** (`config.rs`)
  - `PlanetConfig { planet_radius, planet_origin, atmosphere_multiplier, orbit_multiplier, sky_altitude }` -
    one planet's geometry and layer thresholds, in world units. Four
    concentric shells around `planet_origin` delimit the five layers:
    - `orbit_radius() = planet_radius * atmosphere_multiplier * orbit_multiplier` -
      the space/orbit boundary.
    - `atmosphere_radius() = planet_radius * atmosphere_multiplier` - the
      outer edge of the curved atmosphere shell.
    - `sky_top_radius() = planet_radius + sky_altitude` - the upper edge of
      the sky layer, where the flattening blend starts.
    - `planet_radius` - the surface sphere.
  - `PlanetConfig::validate() -> Result<(), PlanetConfigError>` - checks the
    field invariants: positive finite radius, finite origin, multipliers
    finite and greater than 1, sky altitude positive and keeping the sky top
    below the atmosphere shell.
  - `PlanetConfigError` - one variant per violated invariant; a typed error
    implementing `std::error::Error`.
- **Layers** (`layer.rs`)
  - `PlanetaryLayer { Space, Orbit, Atmosphere, Sky, Terrain }` - the five
    ordered layers, outermost first (`PlanetaryLayer::ALL` lists them in
    that order; `name()` returns a stable lowercase name for readouts).
- **Manager** (`manager.rs`)
  - `PlanetRuntimeManager` - holds a validated `PlanetConfig`;
    `new(config) -> Result<Self, PlanetConfigError>`, `config()` returns it.
  - `PlanetRuntimeManager::update(player_position: Vec3) -> RuntimeState` -
    the per-frame update; panics on a non-finite position (internal
    invariant).
  - `RuntimeState` - the published per-frame state:
    - `distance_to_center: f32` - player distance to the planet center, in
      world units.
    - `altitude: f32` - signed height above the surface sphere
      (`distance_to_center - planet_radius`); negative below the surface.
    - `direction_to_center: Vec3` - unit direction from the player toward
      the planet center; `Vec3::ZERO` when the player coincides exactly
      with the center.
    - `layer: PlanetaryLayer` - the current layer.
    - `atmosphere_factor: f32` - normalized distance factor for atmosphere
      blending: 0 at and beyond the atmosphere shell edge, ramping linearly
      to 1 at the surface.
    - `flatten_factor: f32` - the authoritative flattening blend factor:
      0 (spherical) at and above the sky top, ramping linearly by altitude
      across the sky layer to 1 (flat) at and below the surface.
    - `anchor: Vec3` - the floating-origin anchor: the player's ground
      projection, the point on the surface sphere along the player's radial,
      recomputed every update.
    - `local_player: Vec3` / `local_planet_center: Vec3` - player position
      and planet center relative to the anchor, so later features (LOD,
      culling, gameplay) work in local f32 coordinates.
- **Ground flattening** (`flatten.rs`) - the headless math of feature 5
  (Decision 3 of `plan/RELATED.md`), pure consumers of the authoritative
  `RuntimeState`:
  - `anchor_up(state) -> Vec3` - the outward radial at the anchor: the
    normal of the tangent plane the terrain morphs toward, and the up
    direction of the flat local frame. Always a unit vector.
  - `morph_point(point, state) -> Vec3` - the morphed position of a
    spherical world-space point: its projection onto the tangent plane at
    the anchor, linearly blended by `flatten_factor`. Exactly the input at
    factor 0; lies in the plane at factor 1. The terrain vertex shader
    evaluates the same formula in the anchor-relative frame.
  - `gravity_direction(state, position) -> Vec3` - the blended gravity
    direction at a world position, as a unit vector: the local radial
    (toward the planet center) blended toward the fixed flat-frame down by
    `flatten_factor`, renormalized. Pure radial at factor 0, fixed down at
    factor 1. At the player's own position the endpoints are colinear
    (player, anchor and center share one radial line); for objects away
    from the player radial the blend tilts smoothly from their local
    radial to the shared flat down.
  - `surface_height(state, position) -> Option<f32>` - the height of the
    rendered (morphed) surface above the tangent plane, measured along the
    local vertical through a world position. Exact for the surface the
    shader renders at the state's factor (the morph preserves lateral
    in-plane coordinates, so the height scales by `1 - flatten_factor`);
    `None` beyond the planet's silhouette. The headless query future
    picking will use.
  - `precision_error_bound(state) -> f32` - the f32 rounding-error bound
    the floating origin keeps contained: the unit roundoff (2^-24) scaled
    by the player's distance from the anchor.

### Methods

- `PlanetRuntimeManager::new(config)` validates the configuration, then
  stores it; invalid configurations are rejected with a `PlanetConfigError`.
- `update(player_position)` computes, from position only:
  - the distance and unit direction to the planet center;
  - the layer by comparing the distance against the four shells - at a
    threshold exactly, the outer layer wins, except at the surface radius
    where the layer is `Terrain`;
  - both blend factors as clamped linear ramps (`value / range` clamped to
    `0..=1`), continuous and monotone non-increasing with distance;
  - the anchor as `origin + radial * planet_radius`, where `radial` is the
    unit vector from the center toward the player; when the player coincides
    exactly with the planet center the radial is undefined, the direction to
    the center is reported as `Vec3::ZERO`, and +Y is used as the anchor
    radial;
  - the local frame as plain subtraction from the anchor.

The module is a folder module: `PlanetConfig` and `PlanetConfigError` in
`config.rs`, `PlanetaryLayer` in `layer.rs`, the manager and `RuntimeState`
in `manager.rs`, the ground-flattening math in `flatten.rs`, re-exported
from `mod.rs`. Tests live in `tests/runtime/`,
split one file per submodule.

### Rules

- **Position only** - player orientation never participates in any output.
- **Continuity** - `atmosphere_factor` and `flatten_factor` are continuous
  across every layer boundary (no jumps) and monotone non-increasing with
  distance; both stay in `0..=1`. The integration tests sweep altitude
  densely across every threshold to assert this.
- **Anchor exactness** - the anchor always lies on the surface sphere along
  the player's radial and is recomputed on every update, so the local frame
  re-anchors continuously (bounds float32 error far from the planet center).
- **Local consistency** - `local_player + anchor == player` and
  `local_planet_center + anchor == origin`; `local_planet_center` always has
  length `planet_radius`.
- **One authoritative state** - one `RuntimeState` per frame is the single
  source for flattening and anchoring, so rendering, physics, and picking
  never disagree (Decision 3 in `plan/RELATED.md`).
- **Morph exactness** - the morph is the linear blend between the sphere
  and the tangent plane at the anchor: identity at factor 0, exactly
  planar at factor 1, continuous in between; it preserves lateral
  in-plane coordinates, so the surface-height query matches the rendered
  surface at every blend value.
- **Gravity blend** - the published gravity direction blends the local
  radial with the fixed flat-frame down by the same `flatten_factor` the
  shader uses, and is always a unit vector.
- **Headless** - the module has no GPU, window, or node-graph dependency and
  never mutates external state.
