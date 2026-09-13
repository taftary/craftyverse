// Tests of the atmosphere shell geometry and the CPU reference of the
// atmosphere appearance (`render::atmosphere`), mirrored by `ATMO_FRAG`.

use glam::Vec3;
use planet_crafter_engine::runtime::{PlanetConfig, PlanetaryLayer};
use planet_crafter_engine::testing::{
    DOME_HAZE, HORIZON_COLOR, RIM_COLOR, RIM_MAX_ALPHA, SCATTER_COLOR, SKY_COLOR, appearance,
    atmosphere_lines, atmosphere_state_name, dome_weight, morph_vertex, rim_factor, rim_weight,
    scatter_weight, shell_vertices,
};

/// Asserts two colors match per channel within `tol`.
fn assert_color_close(actual: [f32; 3], expected: [f32; 3], tol: f32, context: &str) {
    for (a, e) in actual.into_iter().zip(expected) {
        assert!((a - e).abs() < tol, "{context}: {actual:?} vs {expected:?}");
    }
}

const TEST_CONFIG: PlanetConfig = PlanetConfig {
    planet_radius: 1000.0,
    planet_origin: Vec3::ZERO,
    atmosphere_multiplier: 1.25,
    orbit_multiplier: 2.0,
    sky_altitude: 100.0,
};

/// A RuntimeState-like input for `atmosphere_lines`: only the layer matters.
fn state_at(distance: f32) -> planet_crafter_engine::runtime::RuntimeState {
    planet_crafter_engine::runtime::PlanetRuntimeManager::new(TEST_CONFIG)
        .unwrap()
        .update(Vec3::new(0.0, distance, 0.0))
}

#[test]
fn shell_vertices_all_lie_on_the_shell_sphere() {
    let origin = Vec3::new(10.0, -20.0, 30.0);
    let radius = 1250.0;
    for vertex in shell_vertices(origin, radius, 64, 32) {
        assert!(
            ((vertex.pos - origin).length() - radius).abs() < 0.5,
            "vertex off the shell sphere: {:?}",
            vertex.pos
        );
        assert!(
            (vertex.dir.length() - 1.0).abs() < 1e-4,
            "radial not normalized"
        );
        // The radial is the outward sphere normal at the vertex.
        let expected = (vertex.pos - origin).normalize();
        assert!(vertex.dir.distance(expected) < 1e-3);
    }
}

#[test]
fn shell_grid_topology_and_degenerate_inputs() {
    assert_eq!(shell_vertices(Vec3::ZERO, 1.0, 64, 32).len(), 64 * 32 * 6);
    assert_eq!(shell_vertices(Vec3::ZERO, 1.0, 8, 4).len(), 8 * 4 * 6);
    assert!(shell_vertices(Vec3::ZERO, 1.0, 0, 4).is_empty());
    assert!(shell_vertices(Vec3::ZERO, 1.0, 8, 0).is_empty());
}

#[test]
fn shell_radius_follows_the_configured_multiplier() {
    let shell = shell_vertices(
        TEST_CONFIG.planet_origin,
        TEST_CONFIG.atmosphere_radius(),
        32,
        16,
    );
    assert_eq!(TEST_CONFIG.atmosphere_radius(), 1250.0);
    for vertex in &shell {
        assert!((vertex.pos.length() - 1250.0).abs() < 0.5);
    }
    // A larger multiplier moves every shell vertex outward by the same ratio.
    let bigger = PlanetConfig {
        atmosphere_multiplier: 1.5,
        ..TEST_CONFIG
    };
    for vertex in shell_vertices(bigger.planet_origin, bigger.atmosphere_radius(), 32, 16) {
        assert!((vertex.pos.length() - 1500.0).abs() < 0.5);
    }
}

#[test]
fn shell_stays_curved_under_full_ground_flattening() {
    // Curvature invariant: the shell is never passed through the terrain
    // morph (its generation takes no flatten factor, and the vertex shader
    // only subtracts the anchor), so every vertex stays on the sphere even
    // when the ground is fully flat. Contrast with the terrain morph: at
    // flatten == 1 the morph would pull shell vertices off the sphere, which
    // is exactly what the shell avoids.
    let origin = Vec3::ZERO;
    let radius = TEST_CONFIG.atmosphere_radius();
    let anchor = Vec3::Y * TEST_CONFIG.planet_radius;
    let anchor_up = Vec3::Y;
    let mut morph_would_move = false;
    for vertex in shell_vertices(origin, radius, 32, 16) {
        let morphed = morph_vertex(vertex.pos, anchor, anchor_up, 1.0);
        if ((morphed + anchor - origin).length() - radius).abs() > 1.0 {
            morph_would_move = true;
        }
        assert!((vertex.pos.length() - radius).abs() < 0.5);
    }
    assert!(
        morph_would_move,
        "the test setup should show the morph is incompatible with the shell"
    );
}

#[test]
fn rim_factor_ramps_across_the_orbit_layer() {
    let orbit = TEST_CONFIG.orbit_radius();
    let shell = TEST_CONFIG.atmosphere_radius();
    assert_eq!(rim_factor(orbit + 1.0, orbit, shell), 0.0);
    assert_eq!(rim_factor(orbit, orbit, shell), 0.0);
    assert_eq!(rim_factor((orbit + shell) / 2.0, orbit, shell), 0.5);
    assert_eq!(rim_factor(shell, orbit, shell), 1.0);
    assert_eq!(rim_factor(shell - 1.0, orbit, shell), 1.0);
}

#[test]
fn atmosphere_state_names_match_the_layers() {
    assert_eq!(atmosphere_state_name(PlanetaryLayer::Space), "none");
    assert_eq!(atmosphere_state_name(PlanetaryLayer::Orbit), "rim");
    assert_eq!(
        atmosphere_state_name(PlanetaryLayer::Atmosphere),
        "scattering"
    );
    assert_eq!(atmosphere_state_name(PlanetaryLayer::Sky), "fog");
    assert_eq!(atmosphere_state_name(PlanetaryLayer::Terrain), "sky dome");
}

#[test]
fn overlay_reports_atmosphere_state_and_multiplier() {
    let lines = atmosphere_lines(&state_at(TEST_CONFIG.orbit_radius() + 1.0), 1.25);
    assert_eq!(lines[0], "atmosphere state:   none");
    assert_eq!(lines[1], "shell multiplier:   1.25");
    let lines = atmosphere_lines(&state_at(TEST_CONFIG.planet_radius), 1.25);
    assert_eq!(lines[0], "atmosphere state:   sky dome");
}

#[test]
fn space_is_invisible() {
    // Beyond the shell edge the factor is 0; in space the rim ramp is 0 too.
    for cos_view in [-1.0, -0.3, 0.0, 0.5, 1.0] {
        let sample = appearance(cos_view, 0.0, 0.0, false);
        assert_eq!(sample.alpha, 0.0, "space must be invisible");
    }
}

#[test]
fn orbit_shows_a_thin_limb_rim() {
    // Limb view (radial perpendicular to the view direction): full rim.
    let limb = appearance(0.0, 0.0, 1.0, false);
    assert!((limb.alpha - RIM_MAX_ALPHA).abs() < 1e-5);
    assert_color_close(limb.color, RIM_COLOR, 1e-5, "limb rim color");
    // Face-on view: the rim vanishes (thin rim only at the limb).
    let face_on = appearance(1.0, 0.0, 1.0, false);
    assert!(face_on.alpha < 1e-4);
    // The rim fades in with the orbit-layer ramp.
    let half = appearance(0.0, 0.0, 0.5, false);
    assert!((half.alpha - limb.alpha / 2.0).abs() < 1e-5);
    assert_eq!(rim_weight(0.0, 1.0, 1.0), RIM_MAX_ALPHA);
}

#[test]
fn atmosphere_layer_shows_curved_scattering() {
    // Mid-atmosphere descent (inside the shell, factor 0.3): the scattering
    // layer is fully risen and the dome has not started.
    let f = 0.3;
    assert_eq!(scatter_weight(f), 1.0);
    assert_eq!(dome_weight(f), 0.0);
    let limb = appearance(0.0, f, 1.0, true);
    // Scattering dominates the color (the rim has mostly faded by f = 0.3).
    assert_color_close(limb.color, SCATTER_COLOR, 0.1, "scattering color");
    assert!(limb.alpha > 0.3);
    // Scattering is visible everywhere, strongest at the limb.
    let face_on = appearance(1.0, f, 1.0, true);
    assert!(face_on.alpha > 0.1);
    assert!(limb.alpha > face_on.alpha);
}

#[test]
fn sky_layer_shows_fog_and_horizon_haze() {
    let f = 0.8;
    let w_dome = dome_weight(f);
    assert!(w_dome > 0.3 && w_dome < 1.0);
    // Looking at the horizon (cos_view = 0): strong haze toward the horizon
    // color; looking up (cos_view = 1): no haze, sky color.
    let horizon = appearance(0.0, f, 1.0, true);
    let zenith = appearance(1.0, f, 1.0, true);
    let horizon_blend = DOME_HAZE * (0.4 + 0.6 * w_dome);
    // The scattering remnant shifts the color slightly; allow for it.
    let expected: [f32; 3] =
        std::array::from_fn(|c| SKY_COLOR[c] + (HORIZON_COLOR[c] - SKY_COLOR[c]) * horizon_blend);
    assert_color_close(horizon.color, expected, 0.15, "horizon haze");
    for (z, sky) in zenith.color.into_iter().zip(SKY_COLOR) {
        assert!(z <= sky + 0.15, "zenith should stay near the sky color");
    }
    assert!(horizon.alpha > 0.3);
}

#[test]
fn terrain_shows_a_full_sky_dome() {
    let f = 1.0;
    assert_eq!(dome_weight(f), 1.0);
    assert_eq!(scatter_weight(f), 0.0);
    // Fully opaque everywhere.
    for cos_view in [-1.0, 0.0, 0.5, 1.0] {
        assert_eq!(appearance(cos_view, f, 1.0, true).alpha, 1.0);
    }
    // Zenith is the sky color, the horizon the haze color.
    let zenith = appearance(1.0, f, 1.0, true);
    assert_color_close(zenith.color, SKY_COLOR, 1e-5, "zenith sky color");
    let horizon = appearance(0.0, f, 1.0, true);
    let expected: [f32; 3] =
        std::array::from_fn(|c| SKY_COLOR[c] + (HORIZON_COLOR[c] - SKY_COLOR[c]) * DOME_HAZE);
    assert_color_close(horizon.color, expected, 1e-4, "horizon haze color");
}

/// Dense sweep over the whole factor range: the alpha must be continuous
/// everywhere and the color continuous wherever the fragment is visible
/// (alpha above a small threshold) - no jumps at any layer boundary (the
/// factor parameterizes all transitions). At alpha ~ 0 the color is the
/// invisible fallback of the weightless mixture, which carries no visible
/// cut by construction.
#[test]
fn appearance_is_continuous_across_the_whole_factor_range() {
    const STEPS: usize = 8192;
    const VISIBLE: f32 = 0.02;
    for &cos_view in &[-1.0, -0.5, 0.0, 0.3, 1.0] {
        for &inside in &[false, true] {
            for &rim in &[0.0, 0.37, 1.0] {
                let mut previous = appearance(cos_view, 0.0, rim, inside);
                for step in 1..=STEPS {
                    let f = step as f32 / STEPS as f32;
                    let current = appearance(cos_view, f, rim, inside);
                    let d_alpha = (current.alpha - previous.alpha).abs();
                    assert!(
                        d_alpha < 0.01,
                        "alpha jump at f={f} (cos_view={cos_view}, inside={inside}, rim={rim})"
                    );
                    if previous.alpha > VISIBLE && current.alpha > VISIBLE {
                        let d_color: f32 = (0..3)
                            .map(|c| (current.color[c] - previous.color[c]).abs())
                            .fold(0.0, f32::max);
                        assert!(
                            d_color < 0.01,
                            "color jump at f={f} (cos_view={cos_view}, inside={inside}, \
                             rim={rim}): {d_color}"
                        );
                    }
                    previous = current;
                }
            }
        }
    }
}

/// The rim ramp is continuous across the space/orbit boundary and the whole
/// orbit layer (where the manager's factor is clamped to 0). Alpha is
/// continuous everywhere; the color follows wherever the fragment is
/// visible (see the factor sweep).
#[test]
fn appearance_is_continuous_across_the_orbit_layer() {
    const STEPS: usize = 4096;
    const VISIBLE: f32 = 0.02;
    for &cos_view in &[-0.7, 0.0, 0.9] {
        let mut previous = appearance(cos_view, 0.0, 0.0, false);
        for step in 1..=STEPS {
            let rim = step as f32 / STEPS as f32;
            let current = appearance(cos_view, 0.0, rim, false);
            let d_alpha = (current.alpha - previous.alpha).abs();
            assert!(d_alpha < 0.01, "alpha jump at rim_factor={rim}");
            if previous.alpha > VISIBLE && current.alpha > VISIBLE {
                let d_color: f32 = (0..3)
                    .map(|c| (current.color[c] - previous.color[c]).abs())
                    .fold(0.0, f32::max);
                assert!(d_color < 0.01, "color jump at rim_factor={rim}");
            }
            previous = current;
        }
    }
}

/// Crossing the shell (inside flag flips at factor 0) changes nothing: the
/// inside dome term is 0 at the shell edge, so the inside and outside views
/// agree exactly at the crossing.
#[test]
fn shell_crossing_is_seamless() {
    for &cos_view in &[-1.0, -0.2, 0.0, 0.6, 1.0] {
        let outside = appearance(cos_view, 0.0, 1.0, false);
        let inside = appearance(cos_view, 1e-6, 1.0, true);
        assert!((outside.alpha - inside.alpha).abs() < 1e-3);
        assert_color_close(outside.color, inside.color, 1e-3, "shell crossing");
    }
}
