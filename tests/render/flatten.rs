// Tests of the ground-flattening vertex morph (feature 5): the CPU
// reference (`render::flatten::morph_vertex`, the mirror of the `TEX_VERT`
// GLSL) and its agreement with the authoritative runtime math.

use glam::Vec3;
use planet_crafter_engine::runtime::{
    PlanetConfig, PlanetRuntimeManager, RuntimeState, anchor_up, morph_point,
};
use planet_crafter_engine::testing::{TEX_VERT, compile_spirv, morph_vertex};

fn state_at_factor(factor: f32) -> RuntimeState {
    let config = PlanetConfig {
        planet_radius: 1000.0,
        planet_origin: Vec3::ZERO,
        atmosphere_multiplier: 1.25,
        orbit_multiplier: 2.0,
        sky_altitude: 100.0,
    };
    let manager = PlanetRuntimeManager::new(config).unwrap();
    // The surface-touching player has factor 1; override it to exercise
    // every blend value against the same anchor.
    let mut state = manager.update(Vec3::new(0.0, 1000.0, 0.0));
    state.flatten_factor = factor;
    state
}

#[test]
fn morph_shader_compiles_to_spirv() {
    let words = compile_spirv(TEX_VERT, naga::ShaderStage::Vertex).expect("TEX_VERT compiles");
    assert_eq!(words[0], 0x0723_0203);
}

/// Drift guard: the GLSL morph in `TEX_VERT` must keep the formula the CPU
/// reference mirrors (floating-origin shift, tangent-plane projection,
/// blend by the authoritative factor).
#[test]
fn tex_vert_morph_matches_the_cpu_reference_structure() {
    for snippet in [
        "vec3 local = pos - pc.anchor;",
        "vec3 flattened = local - pc.anchor_up * dot(local, pc.anchor_up);",
        "vec3 morphed = mix(local, flattened, pc.flatten);",
        "gl_Position = pc.mvp * vec4(morphed, 1.0);",
        "vec3 anchor;",
        "vec3 anchor_up;",
        "float flatten;",
    ] {
        assert!(TEX_VERT.contains(snippet), "TEX_VERT drifted: {snippet}");
    }
}

#[test]
fn morph_at_factor_zero_is_exactly_the_input() {
    let anchor = Vec3::new(0.0, 1000.0, 0.0);
    let up = Vec3::Y;
    for pos in [
        Vec3::new(0.0, 1000.0, 0.0),
        Vec3::new(50.0, 998.0, -30.0),
        Vec3::new(-700.0, 700.0, 120.0),
    ] {
        assert_eq!(morph_vertex(pos, anchor, up, 0.0), pos - anchor);
    }
}

#[test]
fn morph_at_factor_one_lies_in_the_tangent_plane() {
    let anchor = Vec3::new(0.0, 1000.0, 0.0);
    let up = Vec3::Y;
    for pos in [
        Vec3::new(0.0, 1000.0, 0.0),
        Vec3::new(50.0, 998.0, -30.0),
        Vec3::new(-700.0, 700.0, 120.0),
    ] {
        let morphed = morph_vertex(pos, anchor, up, 1.0);
        assert!(morphed.dot(up).abs() < 1e-3, "height for {pos:?}");
    }
}

#[test]
fn morph_is_continuous_in_the_blend_factor() {
    let anchor = Vec3::new(0.0, 1000.0, 0.0);
    let up = Vec3::Y;
    let pos = Vec3::new(80.0, 996.8, 0.0);
    let mut previous = morph_vertex(pos, anchor, up, 0.0);
    for step in 1..=100 {
        let factor = step as f32 / 100.0;
        let morphed = morph_vertex(pos, anchor, up, factor);
        let jump = (morphed - previous).length();
        assert!(jump < 0.1, "discontinuity at factor {factor}: {jump}");
        previous = morphed;
    }
}

#[test]
fn morph_mirror_agrees_with_the_authoritative_runtime_morph() {
    // The shader mirror works in the anchor-relative frame, the runtime
    // `morph_point` in the world frame; they must agree up to the anchor
    // shift at every blend value (same formula, same factor, same anchor).
    for &factor in &[0.0, 0.25, 0.5, 0.75, 1.0] {
        let state = state_at_factor(factor);
        let up = anchor_up(&state);
        for pos in [
            Vec3::new(0.0, 1000.0, 0.0),
            Vec3::new(50.0, 998.0, -30.0),
            Vec3::new(-700.0, 700.0, 120.0),
        ] {
            let shader_side = morph_vertex(pos, state.anchor, up, factor) + state.anchor;
            let runtime_side = morph_point(pos, &state);
            assert!(
                (shader_side - runtime_side).length() < 1e-3,
                "factor {factor}, pos {pos:?}: {shader_side:?} vs {runtime_side:?}"
            );
        }
    }
}

#[test]
fn skirt_vertices_morph_with_the_same_formula() {
    // A skirt vertex is displaced toward the planet center from its chunk
    // corner; applying the same morph keeps it directly below the morphed
    // corner (same lateral coordinates), so the crack mask holds at every
    // blend value.
    let state = state_at_factor(0.6);
    let up = anchor_up(&state);
    let corner = Vec3::new(40.0, 999.2, 15.0);
    let skirt = corner - up * 5.0;
    for &factor in &[0.0, 0.3, 0.6, 1.0] {
        let morphed_corner = morph_vertex(corner, state.anchor, up, factor);
        let morphed_skirt = morph_vertex(skirt, state.anchor, up, factor);
        // Same lateral (in-plane) coordinates as the corner, pushed down
        // along the plane normal by the blended skirt depth.
        let lateral_corner = morphed_corner - up * morphed_corner.dot(up);
        let lateral_skirt = morphed_skirt - up * morphed_skirt.dot(up);
        assert!(
            (lateral_corner - lateral_skirt).length() < 1e-3,
            "factor {factor}: skirt slipped laterally"
        );
        let depth = (morphed_corner - morphed_skirt).dot(up);
        // The blended skirt depth is `5 * (1 - factor)`: positive below
        // full flatten, and exactly 0 at factor 1 where the whole surface
        // is planar (the planar projection is affine, so no T-junction
        // cracks remain to mask).
        assert!(
            (depth - 5.0 * (1.0 - factor)).abs() < 1e-3,
            "factor {factor}: skirt depth {depth}"
        );
    }
}
