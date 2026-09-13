use glam::Vec3;

use planet_crafter_engine::runtime::{PlanetRuntimeManager, PlanetaryLayer};
use planet_crafter_tests::fixtures::test_planet_config;

const EPSILON: f32 = 1e-3;

fn manager() -> PlanetRuntimeManager {
    PlanetRuntimeManager::new(test_planet_config()).unwrap()
}

/// Runtime state for a player at `distance` from the center along +Y.
fn state_at(distance: f32) -> planet_crafter_engine::runtime::RuntimeState {
    manager().update(Vec3::new(0.0, distance, 0.0))
}

// --- Classification at and around every threshold ---

#[test]
fn classification_in_layer_interiors() {
    assert_eq!(state_at(10_000.0).layer, PlanetaryLayer::Space);
    assert_eq!(state_at(2000.0).layer, PlanetaryLayer::Orbit);
    assert_eq!(state_at(1200.0).layer, PlanetaryLayer::Atmosphere);
    assert_eq!(state_at(1050.0).layer, PlanetaryLayer::Sky);
    assert_eq!(state_at(900.0).layer, PlanetaryLayer::Terrain);
}

#[test]
fn classification_at_exact_thresholds() {
    // At a threshold the outer layer wins, except at the surface.
    assert_eq!(state_at(2500.0).layer, PlanetaryLayer::Space);
    assert_eq!(state_at(1250.0).layer, PlanetaryLayer::Orbit);
    assert_eq!(state_at(1100.0).layer, PlanetaryLayer::Atmosphere);
    assert_eq!(state_at(1000.0).layer, PlanetaryLayer::Terrain);
}

#[test]
fn classification_around_every_threshold() {
    let cases = [
        (2500.0, PlanetaryLayer::Space, PlanetaryLayer::Orbit),
        (1250.0, PlanetaryLayer::Orbit, PlanetaryLayer::Atmosphere),
        (1100.0, PlanetaryLayer::Atmosphere, PlanetaryLayer::Sky),
        (1000.0, PlanetaryLayer::Sky, PlanetaryLayer::Terrain),
    ];
    for (threshold, outer, inner) in cases {
        assert_eq!(state_at(threshold + 0.5).layer, outer, "above {threshold}");
        assert_eq!(state_at(threshold - 0.5).layer, inner, "below {threshold}");
    }
}

// --- Distance, direction, and altitude ---

#[test]
fn distance_direction_and_altitude() {
    let state = manager().update(Vec3::new(300.0, 400.0, 0.0));
    assert!((state.distance_to_center - 500.0).abs() < EPSILON);
    assert!((state.altitude - -500.0).abs() < EPSILON);
    let expected = (Vec3::ZERO - Vec3::new(300.0, 400.0, 0.0)).normalize();
    assert!((state.direction_to_center - expected).length() < EPSILON);
}

#[test]
fn off_center_origin_is_respected() {
    let config = planet_crafter_engine::runtime::PlanetConfig {
        planet_origin: Vec3::new(5000.0, 0.0, 0.0),
        ..test_planet_config()
    };
    let manager = PlanetRuntimeManager::new(config).unwrap();
    let state = manager.update(Vec3::new(5000.0, 1050.0, 0.0));
    assert_eq!(state.layer, PlanetaryLayer::Sky);
    assert!((state.distance_to_center - 1050.0).abs() < EPSILON);
    assert!((state.direction_to_center - Vec3::NEG_Y).length() < EPSILON);
}

#[test]
fn player_at_planet_center_uses_fallback_radial() {
    let state = manager().update(Vec3::ZERO);
    assert_eq!(state.distance_to_center, 0.0);
    assert_eq!(state.direction_to_center, Vec3::ZERO);
    assert_eq!(state.layer, PlanetaryLayer::Terrain);
    assert_eq!(state.anchor, Vec3::new(0.0, 1000.0, 0.0));
    assert_eq!(state.flatten_factor, 1.0);
    assert_eq!(state.atmosphere_factor, 1.0);
}

// --- Factor values at key points ---

#[test]
fn atmosphere_factor_key_points() {
    assert_eq!(state_at(2500.0).atmosphere_factor, 0.0);
    assert_eq!(state_at(1250.0).atmosphere_factor, 0.0);
    assert!((state_at(1125.0).atmosphere_factor - 0.5).abs() < EPSILON);
    assert_eq!(state_at(1000.0).atmosphere_factor, 1.0);
    assert_eq!(state_at(500.0).atmosphere_factor, 1.0);
}

#[test]
fn flatten_factor_key_points() {
    assert_eq!(state_at(1250.0).flatten_factor, 0.0);
    assert_eq!(state_at(1100.0).flatten_factor, 0.0);
    assert!((state_at(1050.0).flatten_factor - 0.5).abs() < EPSILON);
    assert_eq!(state_at(1000.0).flatten_factor, 1.0);
    assert_eq!(state_at(500.0).flatten_factor, 1.0);
}

// --- Dense sweeps: continuity and monotonicity across every boundary ---

#[test]
fn factors_are_continuous_and_monotone_across_all_boundaries() {
    let steps = 200_000;
    let min = 900.0_f32;
    let max = 2600.0_f32;
    let mut previous = state_at(max);
    for i in 1..=steps {
        let distance = max - (max - min) * (i as f32) / (steps as f32);
        let state = state_at(distance);

        for (before, after) in [
            (previous.atmosphere_factor, state.atmosphere_factor),
            (previous.flatten_factor, state.flatten_factor),
        ] {
            assert!(
                (after - before).abs() < 1e-3,
                "factor jump at distance {distance}: {before} -> {after}"
            );
            assert!(
                after >= before,
                "factor decreased with altitude at distance {distance}: {before} -> {after}"
            );
        }
        previous = state;
    }
}

#[test]
fn factors_stay_normalized_across_the_full_sweep() {
    for i in 0..=10_000 {
        let distance = 0.0 + 10_000.0 * (i as f32) / 10_000.0;
        let state = state_at(distance);
        assert!((0.0..=1.0).contains(&state.atmosphere_factor));
        assert!((0.0..=1.0).contains(&state.flatten_factor));
    }
}

#[test]
fn layer_transitions_exactly_once_during_descent() {
    let mut layer = PlanetaryLayer::Space;
    let mut transitions = Vec::new();
    for i in 0..=100_000 {
        let distance = 2600.0 - (2600.0 - 900.0) * (i as f32) / 100_000.0;
        let current = state_at(distance).layer;
        if current != layer {
            transitions.push(current);
            layer = current;
        }
    }
    assert_eq!(
        transitions,
        [
            PlanetaryLayer::Orbit,
            PlanetaryLayer::Atmosphere,
            PlanetaryLayer::Sky,
            PlanetaryLayer::Terrain,
        ]
    );
}

// --- Anchor exactness and local-frame consistency ---

#[test]
fn anchor_lies_on_the_surface_along_the_player_radial() {
    // Sweeps several directions and altitudes, including non-axis ones.
    let players = [
        Vec3::new(0.0, 1050.0, 0.0),
        Vec3::new(3000.0, 0.0, 0.0),
        Vec3::new(700.0, 700.0, 700.0),
        Vec3::new(-1234.5, 678.9, -42.0),
        Vec3::new(0.0, 900.0, 0.0),
    ];
    for player in players {
        let state = manager().update(player);
        let radial = (player - Vec3::ZERO).normalize();
        // On the surface sphere.
        assert!(
            (state.anchor.length() - 1000.0).abs() < 0.01,
            "anchor not on surface for {player:?}: {:?}",
            state.anchor
        );
        // Along the player radial from the center.
        let cross = state.anchor.normalize().cross(radial);
        assert!(
            cross.length() < 1e-4,
            "anchor not radial for {player:?}: {:?}",
            state.anchor
        );
        assert!(state.anchor.dot(radial) > 0.0);
    }
}

#[test]
fn anchor_tracks_the_player_every_update() {
    let manager = manager();
    let first = manager.update(Vec3::new(0.0, 1050.0, 0.0));
    let second = manager.update(Vec3::new(1050.0, 0.0, 0.0));
    assert!((first.anchor - Vec3::new(0.0, 1000.0, 0.0)).length() < 0.01);
    assert!((second.anchor - Vec3::new(1000.0, 0.0, 0.0)).length() < 0.01);
}

#[test]
fn local_frame_is_consistent_with_world_space() {
    let players = [
        Vec3::new(0.0, 1050.0, 0.0),
        Vec3::new(-1234.5, 678.9, -42.0),
        Vec3::ZERO,
    ];
    for player in players {
        let state = manager().update(player);
        // Local values reconstruct the world-space ones through the anchor.
        assert!((state.local_player + state.anchor - player).length() < 1e-3);
        assert!((state.local_planet_center + state.anchor - Vec3::ZERO).length() < 1e-3);
        // The planet center is always one radius away from the anchor.
        assert!((state.local_planet_center.length() - 1000.0).abs() < 0.01);
        // Local and world distances to the center agree.
        let local_distance = (state.local_player - state.local_planet_center).length();
        assert!((local_distance - state.distance_to_center).abs() < 1e-2);
    }
}

// --- Orientation never participates ---

#[test]
fn state_depends_only_on_position() {
    // The manager has no orientation input at all; calling update twice with
    // the same position always yields the identical state.
    let manager = manager();
    let a = manager.update(Vec3::new(100.0, 1050.0, 50.0));
    let b = manager.update(Vec3::new(100.0, 1050.0, 50.0));
    assert_eq!(a, b);
}
