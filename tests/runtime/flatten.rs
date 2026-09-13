// Tests of the runtime module's ground-flattening math (feature 5): the
// sphere-to-tangent-plane morph, the blended gravity direction, the surface
// height query against the morphed surface, and the f32 precision bound.

use glam::Vec3;
use planet_crafter_engine::runtime::{
    PlanetConfig, PlanetRuntimeManager, RuntimeState, anchor_up, gravity_direction, morph_point,
    precision_error_bound, surface_height,
};

fn config() -> PlanetConfig {
    PlanetConfig {
        planet_radius: 1000.0,
        planet_origin: Vec3::ZERO,
        atmosphere_multiplier: 1.25,
        orbit_multiplier: 2.0,
        sky_altitude: 100.0,
    }
}

fn manager() -> PlanetRuntimeManager {
    PlanetRuntimeManager::new(config()).unwrap()
}

/// A runtime state with an explicit flatten factor: takes the state of the
/// surface-touching player (factor 1) and overrides the factor, so the
/// endpoints and intermediate blends can be exercised directly.
fn state_at_factor(factor: f32) -> RuntimeState {
    let mut state = manager().update(Vec3::new(0.0, 1000.0, 0.0));
    state.flatten_factor = factor;
    state
}

/// The sphere point at lateral distance `u` from the anchor on the tangent
/// plane (the upper intersection of the local vertical with the sphere).
fn sphere_point(state: &RuntimeState, u: f32) -> Vec3 {
    let up = anchor_up(state);
    let origin = state.anchor + state.local_planet_center;
    let radius = state.local_planet_center.length();
    origin + up * (radius * radius - u * u).sqrt() + up.cross(Vec3::X).normalize() * u
}

#[test]
fn anchor_up_is_the_outward_radial_at_the_anchor() {
    let state = manager().update(Vec3::new(300.0, 1050.0, -200.0));
    let up = anchor_up(&state);
    assert!((up.length() - 1.0).abs() < 1e-6);
    let radial = state.anchor.normalize();
    assert!(
        (up - radial).length() < 1e-4,
        "up {up:?} vs radial {radial:?}"
    );
}

#[test]
fn morph_at_factor_zero_is_exactly_spherical() {
    let state = state_at_factor(0.0);
    for point in [
        Vec3::new(0.0, 1000.0, 0.0),
        Vec3::new(50.0, 998.0, -30.0),
        Vec3::new(-700.0, 700.0, 120.0),
    ] {
        assert_eq!(morph_point(point, &state), point);
    }
}

#[test]
fn morph_at_factor_one_lies_exactly_in_the_tangent_plane() {
    let state = state_at_factor(1.0);
    let up = anchor_up(&state);
    for point in [
        Vec3::new(0.0, 1000.0, 0.0),
        Vec3::new(50.0, 998.0, -30.0),
        Vec3::new(-700.0, 700.0, 120.0),
    ] {
        let morphed = morph_point(point, &state);
        let height = (morphed - state.anchor).dot(up);
        assert!(height.abs() < 1e-3, "height {height} for {point:?}");
    }
}

#[test]
fn morph_blends_linearly_and_is_continuous_across_the_sky_layer() {
    let up_manager = manager();
    let mut previous: Option<f32> = None;
    // Descend from just above the sky top (factor 0) to the surface
    // (factor 1): the factor ramps continuously and the morph stays
    // between the sphere and the plane.
    for step in 0..=200 {
        let altitude = 101.0 - step as f32 * 1.01;
        let state = up_manager.update(Vec3::new(0.0, 1000.0 + altitude, 0.0));
        let point = Vec3::new(80.0, 996.8, 0.0);
        let morphed = morph_point(point, &state);
        let up = anchor_up(&state);
        let spherical_height = (point - state.anchor).dot(up);
        let height = (morphed - state.anchor).dot(up);
        assert!(
            ((1.0 - state.flatten_factor) * spherical_height - height).abs() < 1e-3,
            "height {height} vs factor {} at altitude {altitude}",
            state.flatten_factor
        );
        if let Some(previous) = previous {
            assert!(
                (state.flatten_factor - previous).abs() <= 0.011,
                "factor jump at altitude {altitude}"
            );
        }
        previous = Some(state.flatten_factor);
    }
}

#[test]
fn gravity_is_pure_radial_at_factor_zero_and_fixed_down_at_factor_one() {
    // The blend is visible away from the player radial: radial gravity
    // follows the queried position, flat gravity stays fixed.
    let player = Vec3::new(300.0, 1050.0, -200.0);
    let mut state = manager().update(player);
    let position = player + Vec3::new(60.0, 0.0, 30.0);
    let origin = state.anchor + state.local_planet_center;

    state.flatten_factor = 0.0;
    let gravity = gravity_direction(&state, position);
    let radial = (origin - position).normalize();
    assert!(
        (gravity - radial).length() < 1e-6,
        "radial endpoint: {gravity:?} vs {radial:?}"
    );

    state.flatten_factor = 1.0;
    let gravity = gravity_direction(&state, position);
    let flat_down = state.local_planet_center.normalize();
    assert!(
        (gravity - flat_down).length() < 1e-6,
        "flat endpoint: {gravity:?} vs {flat_down:?}"
    );
    // The fixed down must not follow the queried position's radial.
    assert!(flat_down.dot(radial) < 1.0 - 1e-6);
}

#[test]
fn gravity_at_the_player_position_is_the_radial_at_every_blend() {
    // Player, anchor and planet center share one radial line, so both
    // endpoints coincide and the blend is exactly the radial direction.
    let player = Vec3::new(300.0, 1050.0, -200.0);
    let mut state = manager().update(player);
    for step in 0..=10 {
        state.flatten_factor = step as f32 / 10.0;
        let gravity = gravity_direction(&state, player);
        assert!(
            (gravity - state.direction_to_center).length() < 1e-5,
            "factor {}: {gravity:?}",
            state.flatten_factor
        );
    }
}

#[test]
fn gravity_blend_stays_unit_and_monotone_across_the_sky_layer() {
    let player = Vec3::new(300.0, 1050.0, -200.0);
    let mut state = manager().update(player);
    let position = player + Vec3::new(60.0, 0.0, 30.0);
    let origin = state.anchor + state.local_planet_center;
    let radial = (origin - position).normalize();
    let flat_down = state.local_planet_center.normalize();
    let mut previous_angle = f32::NAN;
    for step in 0..=100 {
        state.flatten_factor = step as f32 / 100.0;
        let gravity = gravity_direction(&state, position);
        assert!((gravity.length() - 1.0).abs() < 1e-5, "len at step {step}");
        // The blended direction tilts monotonically from radial to flat:
        // its angle to the flat down shrinks from the full angle to 0.
        let angle = gravity.dot(flat_down).clamp(-1.0, 1.0).acos();
        if step > 0 {
            assert!(
                angle <= previous_angle + 1e-5,
                "angle regression at step {step}: {angle} > {previous_angle}"
            );
        }
        previous_angle = angle;
        // The blend interpolates the angle between the two endpoints.
        let full_angle = radial.dot(flat_down).clamp(-1.0, 1.0).acos();
        assert!(angle <= full_angle + 1e-4);
    }
}

#[test]
fn surface_height_matches_the_morphed_surface_at_every_blend() {
    for &factor in &[0.0, 0.25, 0.5, 0.75, 1.0] {
        let state = state_at_factor(factor);
        for &u in &[0.0_f32, 40.0, 120.0, 250.0] {
            // The vertical line at lateral distance u meets the morphed
            // surface at the morphed sphere point of the same lateral
            // coordinates: the query must match the shader's morph.
            let point = sphere_point(&state, u);
            let expected = (morph_point(point, &state) - state.anchor).dot(anchor_up(&state));
            let queried = surface_height(&state, point).unwrap();
            assert!(
                (queried - expected).abs() < 1e-2,
                "factor {factor}, u {u}: query {queried} vs morph {expected}"
            );
        }
    }
}

#[test]
fn surface_height_endpoints_are_the_sphere_and_the_plane() {
    let u = 100.0_f32;
    let radius = 1000.0_f32;
    let spherical_depth = -(radius - (radius * radius - u * u).sqrt());

    let state = state_at_factor(0.0);
    let point = sphere_point(&state, u);
    let height = surface_height(&state, point).unwrap();
    assert!(
        (height - spherical_depth).abs() < 1e-2,
        "spherical endpoint: {height} vs {spherical_depth}"
    );

    let state = state_at_factor(1.0);
    let point = sphere_point(&state, u);
    let height = surface_height(&state, point).unwrap();
    assert!(height.abs() < 1e-3, "flat endpoint: {height}");
}

#[test]
fn surface_height_misses_beyond_the_planet_silhouette() {
    let state = state_at_factor(0.5);
    // A vertical line 1.5 radii from the anchor never meets the sphere.
    let far = state.anchor + anchor_up(&state).cross(Vec3::X).normalize() * 1500.0;
    assert_eq!(surface_height(&state, far), None);
}

#[test]
fn precision_error_bound_scales_with_the_distance_from_the_anchor() {
    // At the ground the player sits on the anchor: the bound is ~zero.
    let ground = manager().update(Vec3::new(0.0, 1001.0, 0.0));
    let bound = precision_error_bound(&ground);
    assert!((0.0..1e-4).contains(&bound), "ground bound {bound}");

    // In orbit the player is hundreds of units from the anchor: the bound
    // grows with the distance but stays far below world-unit scale.
    let orbit = manager().update(Vec3::new(0.0, 2100.0, 0.0));
    let orbit_bound = precision_error_bound(&orbit);
    assert!(orbit_bound > bound);
    let expected = orbit.local_player.length() * 5.960_464_5e-8;
    assert!((orbit_bound - expected).abs() < 1e-9);
}
