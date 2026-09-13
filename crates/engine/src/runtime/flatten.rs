//! Ground-flattening math (feature 5, Decision 3 of `plan/RELATED.md`):
//! the sphere-to-tangent-plane morph, the blended gravity direction, the
//! surface height query against the morphed surface, and the f32 precision
//! bound of the floating origin.
//!
//! Every function is a pure consumer of the authoritative
//! [`RuntimeState`] published by
//! [`PlanetRuntimeManager::update`](crate::runtime::PlanetRuntimeManager::update):
//! the same per-frame `flatten_factor` and `anchor` drive the terrain vertex
//! shader, gameplay, and picking, so they never disagree. The vertex shader
//! (`render::shaders::TEX_VERT`) evaluates the same morph formula in the
//! anchor-relative frame; `render::flatten` (test-internals) is its CPU
//! mirror.

use glam::Vec3;

use crate::runtime::manager::RuntimeState;

/// f32 unit roundoff (2^-24): the maximum relative rounding error of one
/// f32 operation. Used by [`precision_error_bound`].
const F32_UNIT_ROUNDOFF: f32 = 5.960_464_5e-8;

/// The outward radial direction at the anchor: the normal of the tangent
/// plane the terrain morphs toward, and the up direction of the flat local
/// frame.
///
/// Always a unit vector: `local_planet_center` has length `planet_radius`
/// (nonzero by configuration), even when the player coincides with the
/// planet center (the anchor then uses the documented +Y fallback radial).
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::runtime::{
///     PlanetConfig, PlanetRuntimeManager, anchor_up,
/// };
///
/// # let config = PlanetConfig {
/// #     planet_radius: 1000.0,
/// #     planet_origin: Vec3::ZERO,
/// #     atmosphere_multiplier: 1.25,
/// #     orbit_multiplier: 2.0,
/// #     sky_altitude: 100.0,
/// # };
/// let manager = PlanetRuntimeManager::new(config).unwrap();
/// let state = manager.update(Vec3::new(0.0, 1050.0, 0.0));
/// assert!((anchor_up(&state) - Vec3::Y).length() < 1e-6);
/// ```
pub fn anchor_up(state: &RuntimeState) -> Vec3 {
    -state.local_planet_center.normalize_or_zero()
}

/// The morphed position of a spherical world-space `point` under the
/// authoritative blend state: the point's projection onto the tangent plane
/// at the anchor (the plane through the anchor with normal
/// [`anchor_up`]), linearly blended by `flatten_factor`.
///
/// At factor 0 the result is exactly `point`; at factor 1 the result lies
/// in the tangent plane. This is the world-frame form of the morph the
/// terrain vertex shader evaluates in the anchor-relative frame; both use
/// the same `flatten_factor` and `anchor`.
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::runtime::{
///     PlanetConfig, PlanetRuntimeManager, morph_point,
/// };
///
/// # let config = PlanetConfig {
/// #     planet_radius: 1000.0,
/// #     planet_origin: Vec3::ZERO,
/// #     atmosphere_multiplier: 1.25,
/// #     orbit_multiplier: 2.0,
/// #     sky_altitude: 100.0,
/// # };
/// let manager = PlanetRuntimeManager::new(config).unwrap();
/// // Above the sky top the factor is 0: no morph.
/// let state = manager.update(Vec3::new(0.0, 1200.0, 0.0));
/// let point = Vec3::new(50.0, 1000.0, 0.0);
/// assert_eq!(morph_point(point, &state), point);
/// ```
pub fn morph_point(point: Vec3, state: &RuntimeState) -> Vec3 {
    let up = anchor_up(state);
    let flattened = point - up * (point - state.anchor).dot(up);
    point.lerp(flattened, state.flatten_factor)
}

/// The blended gravity direction at `position` (world units), as a unit
/// vector: the radial gravity direction at that point (toward the planet
/// center) blended toward the fixed flat-frame down direction (from the
/// anchor toward the planet center) by the authoritative `flatten_factor`.
///
/// Pure radial at factor 0, the fixed down of the flat frame at factor 1,
/// and renormalized in between so movement code always reads a unit
/// direction. At the player's own position the two endpoints are colinear
/// (player, anchor and planet center share one radial line), so the blend
/// is exactly the radial there; for objects away from the player radial the
/// blend tilts smoothly from their local radial to the shared flat down.
/// Gameplay systems (movement, building) are future work; this is the
/// published value they will consume.
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::runtime::{
///     PlanetConfig, PlanetRuntimeManager, gravity_direction,
/// };
///
/// # let config = PlanetConfig {
/// #     planet_radius: 1000.0,
/// #     planet_origin: Vec3::ZERO,
/// #     atmosphere_multiplier: 1.25,
/// #     orbit_multiplier: 2.0,
/// #     sky_altitude: 100.0,
/// # };
/// let manager = PlanetRuntimeManager::new(config).unwrap();
/// let player = Vec3::new(0.0, 1050.0, 0.0);
/// let state = manager.update(player);
/// let gravity = gravity_direction(&state, player + Vec3::new(40.0, 0.0, 0.0));
/// assert!((gravity.length() - 1.0).abs() < 1e-6);
/// ```
pub fn gravity_direction(state: &RuntimeState, position: Vec3) -> Vec3 {
    let flat_down = state.local_planet_center.normalize_or_zero();
    let origin = state.anchor + state.local_planet_center;
    let radial = (origin - position).normalize_or(flat_down);
    radial
        .lerp(flat_down, state.flatten_factor)
        .normalize_or(flat_down)
}

/// The height of the rendered (morphed) surface above the tangent plane at
/// the anchor, measured along the local vertical through `position` (a
/// world-space point), in world units. Negative: the spherical surface lies
/// below the tangent plane everywhere except at the anchor itself.
///
/// This is the exact height of the surface the terrain vertex shader
/// renders at the state's `flatten_factor`: the morph preserves the lateral
/// (in-plane) coordinates of every vertex, so the vertical line through
/// `position` meets the morphed surface at the morphed sphere point of the
/// same lateral coordinates, whose height above the plane scales by
/// `1 - flatten_factor`. Returns `None` when the vertical line misses the
/// surface sphere (`position` is beyond the planet's silhouette).
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::runtime::{
///     PlanetConfig, PlanetRuntimeManager, surface_height,
/// };
///
/// # let config = PlanetConfig {
/// #     planet_radius: 1000.0,
/// #     planet_origin: Vec3::ZERO,
/// #     atmosphere_multiplier: 1.25,
/// #     orbit_multiplier: 2.0,
/// #     sky_altitude: 100.0,
/// # };
/// let manager = PlanetRuntimeManager::new(config).unwrap();
/// // Fully flat at the surface: the tangent plane itself, height 0.
/// let state = manager.update(Vec3::new(0.0, 1000.0, 0.0));
/// let height = surface_height(&state, Vec3::new(50.0, 1000.0, 0.0)).unwrap();
/// assert!(height.abs() < 1e-3);
/// ```
pub fn surface_height(state: &RuntimeState, position: Vec3) -> Option<f32> {
    let up = anchor_up(state);
    let origin = state.anchor + state.local_planet_center;
    let radius = state.local_planet_center.length();
    // Intersect the vertical line `position - t * up` with the surface
    // sphere; the smaller root is the upper (near-side) intersection.
    let m = position - origin;
    let projection = m.dot(up);
    let discriminant = projection * projection - (m.length_squared() - radius * radius);
    if discriminant < 0.0 {
        return None;
    }
    let t = projection - discriminant.sqrt();
    let sphere_point = position - up * t;
    Some((sphere_point - state.anchor).dot(up) * (1.0 - state.flatten_factor))
}

/// The f32 precision-error bound the floating origin keeps contained, in
/// world units: the unit roundoff (2^-24) scaled by the player's distance
/// from the anchor - the maximum rounding error of one f32 operation on a
/// coordinate of that magnitude. Because world rendering subtracts the
/// anchor before transforming (and the anchor chases the player's ground
/// projection every frame), rendered coordinates near the player stay
/// small and this bound stays contained instead of growing with the
/// distance from the planet center.
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::runtime::{
///     PlanetConfig, PlanetRuntimeManager, precision_error_bound,
/// };
///
/// # let config = PlanetConfig {
/// #     planet_radius: 1000.0,
/// #     planet_origin: Vec3::ZERO,
/// #     atmosphere_multiplier: 1.25,
/// #     orbit_multiplier: 2.0,
/// #     sky_altitude: 100.0,
/// # };
/// let manager = PlanetRuntimeManager::new(config).unwrap();
/// // Near the ground the player sits ~on the anchor: sub-millimeter bound.
/// let state = manager.update(Vec3::new(0.0, 1001.0, 0.0));
/// assert!(precision_error_bound(&state) < 1e-4);
/// ```
pub fn precision_error_bound(state: &RuntimeState) -> f32 {
    state.local_player.length() * F32_UNIT_ROUNDOFF
}
