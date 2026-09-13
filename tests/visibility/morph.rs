// Tests of the morph-aware culling (the disappearing-mesh fix): while the
// ground-flattening morph is active (sky/terrain layers), the visibility
// pass must test the RENDERED (morphed) geometry - chunk bounds built from
// the morphed corners and the horizon occluder shrunk to the sphere
// inscribed in the morphed ellipsoid.

use glam::{Mat4, Vec3};
use planet_crafter_engine::node::{NodeRef, build_icosphere, destroy_mesh};
use planet_crafter_engine::runtime::{
    PlanetConfig, PlanetRuntimeManager, RuntimeState, anchor_up, morph_point,
};
use planet_crafter_engine::visibility::{ChunkBounds, Frustum, PlanetHorizon, cull_chunks};

const RADIUS: f32 = 300.0;

fn config() -> PlanetConfig {
    PlanetConfig {
        planet_radius: RADIUS,
        planet_origin: Vec3::ZERO,
        atmosphere_multiplier: 1.25,
        orbit_multiplier: 2.0,
        sky_altitude: 30.0,
    }
}

fn state_at(altitude: f32) -> (PlanetRuntimeManager, RuntimeState) {
    let manager = PlanetRuntimeManager::new(config()).unwrap();
    let state = manager.update(Vec3::new(0.0, RADIUS + altitude, 0.0));
    (manager, state)
}

/// The runtime window's projection: 45-degree vertical field of view,
/// square viewport, the engine's y-up NDC-depth-0..1 convention.
fn view_projection(position: Vec3, target: Vec3) -> Mat4 {
    let view = glam::camera::rh::view::look_at_mat4(position, target, Vec3::Y);
    let proj = glam::camera::rh::proj::directx::perspective(
        std::f32::consts::FRAC_PI_4,
        1.0,
        0.05,
        100.0 * RADIUS,
    );
    proj * view
}

/// Whether a world-space point projects inside the camera's clip volume
/// (the exact frustum test, per point).
fn inside_frustum(mvp: Mat4, point: Vec3) -> bool {
    let clip = mvp * point.extend(1.0);
    clip.w > 0.0
        && clip.x.abs() <= clip.w
        && clip.y.abs() <= clip.w
        && (0.0..=clip.w).contains(&clip.z)
}

fn morphed_bounds(faces: &[NodeRef], state: &RuntimeState) -> Vec<ChunkBounds> {
    faces
        .iter()
        .map(|face| {
            let node = face.borrow();
            ChunkBounds::from_triangle(
                morph_point(node.center, state),
                node.vertices.map(|v| morph_point(v, state)),
                // A generous skirt stand-in: the assertion targets the
                // corners, and the morph only contracts the margin.
                10.0,
            )
        })
        .collect()
}

fn unmorphed_bounds(faces: &[NodeRef]) -> Vec<ChunkBounds> {
    faces
        .iter()
        .map(|face| ChunkBounds::from_node(face, 10.0))
        .collect()
}

#[test]
fn morphed_horizon_at_zero_flatten_is_the_unmorphed_body() {
    let base = PlanetHorizon::new(Vec3::ZERO, RADIUS);
    let morphed = PlanetHorizon::morphed(Vec3::ZERO, RADIUS, Vec3::Y, 0.0);
    assert_eq!(base, morphed);
}

#[test]
fn full_flatten_hides_nothing_behind_the_limb() {
    // At flatten == 1 the terrain is a flat plane: the occlusion body is
    // degenerate and even the antipode chunk stays visible.
    let horizon = PlanetHorizon::morphed(Vec3::ZERO, RADIUS, Vec3::Y, 1.0);
    let camera = Vec3::new(0.0, RADIUS + 5.0, 0.0);
    let antipode =
        planet_crafter_engine::visibility::BoundingSphere::new(Vec3::new(0.0, -RADIUS, 0.0), 10.0);
    assert!(!horizon.occludes(camera, &antipode));
}

#[test]
fn unmorphed_bounds_cull_rendered_chunks_morphed_bounds_keep_them() {
    // Regression evidence for the disappearing-mesh bug: in the sky layer
    // the vertex shader morphs the terrain toward the anchor's tangent
    // plane, but the unmorphed spherical bounds sag below it - the
    // frustum test drops chunks whose RENDERED (morphed) corners are
    // plainly inside the view. The morph-aware pass keeps them all.
    let mesh = build_icosphere("planet", RADIUS, 2, Vec3::ZERO);
    let (_, state) = state_at(5.0);
    assert!(
        state.flatten_factor > 0.5,
        "factor: {}",
        state.flatten_factor
    );
    let camera = Vec3::new(0.0, RADIUS + 5.0, 0.0);
    // Looking toward the horizon, slightly downward.
    let mvp = view_projection(camera, Vec3::new(0.0, RADIUS, -100.0));
    let frustum = Frustum::from_view_projection(mvp);

    // Chunks whose rendered corners are inside the frustum.
    let rendered_visible: Vec<usize> = mesh
        .faces
        .iter()
        .enumerate()
        .filter(|(_, face)| {
            face.borrow()
                .vertices
                .iter()
                .any(|v| inside_frustum(mvp, morph_point(*v, &state)))
        })
        .map(|(index, _)| index)
        .collect();
    assert!(
        !rendered_visible.is_empty(),
        "expected some rendered-visible chunks"
    );

    // The old pass (unmorphed bounds, unmorphed horizon) drops some of
    // them: the bug.
    let old = cull_chunks(
        &unmorphed_bounds(&mesh.faces),
        camera,
        &frustum,
        &PlanetHorizon::new(Vec3::ZERO, RADIUS),
    );
    let dropped: Vec<usize> = rendered_visible
        .iter()
        .copied()
        .filter(|index| !old.visible.contains(index))
        .collect();
    assert!(
        !dropped.is_empty(),
        "the unmorphed pass should drop rendered chunks here (bug evidence)"
    );

    // The fixed pass (morphed bounds, inscribed-ellipsoid horizon) keeps
    // every one of them.
    let fixed = cull_chunks(
        &morphed_bounds(&mesh.faces, &state),
        camera,
        &frustum,
        &PlanetHorizon::morphed(Vec3::ZERO, RADIUS, anchor_up(&state), state.flatten_factor),
    );
    for index in &rendered_visible {
        assert!(
            fixed.visible.contains(index),
            "morph-aware pass culled rendered chunk {}",
            mesh.faces[*index].borrow().name
        );
    }
    destroy_mesh(&mesh.faces[0]);
}

#[test]
fn descent_into_terrain_never_culls_the_chunk_under_the_player() {
    // No-popping sweep with the real morph math: descending from above
    // the sky top to the surface (flatten 0 -> 1), the chunk under the
    // player (at the anchor) and every chunk with a rendered corner
    // inside the frustum near the anchor stays visible at every step.
    let mesh = build_icosphere("planet", RADIUS, 2, Vec3::ZERO);
    for step in 0..24 {
        let t = step as f32 / 23.0;
        let altitude = 60.0 - 59.5 * t;
        let (_, state) = state_at(altitude);
        let camera = Vec3::new(0.0, RADIUS + altitude, 0.0);
        let mvp = view_projection(camera, Vec3::new(0.0, RADIUS, -100.0));
        let frustum = Frustum::from_view_projection(mvp);
        let bounds = if state.flatten_factor > 0.0 {
            morphed_bounds(&mesh.faces, &state)
        } else {
            unmorphed_bounds(&mesh.faces)
        };
        let horizon =
            PlanetHorizon::morphed(Vec3::ZERO, RADIUS, anchor_up(&state), state.flatten_factor);
        let report = cull_chunks(&bounds, camera, &frustum, &horizon);

        // The chunk nearest the anchor is always visible.
        let (nearest, _) = mesh
            .faces
            .iter()
            .enumerate()
            .map(|(i, face)| (i, face.borrow().center.distance(state.anchor)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        assert!(
            report.visible.contains(&nearest),
            "anchor chunk culled at altitude {altitude} (flatten {})",
            state.flatten_factor
        );

        // Every chunk with a rendered (morphed) corner inside the
        // frustum and close to the anchor (clearly unoccluded) survives.
        for (index, face) in mesh.faces.iter().enumerate() {
            let node = face.borrow();
            let lateral = node.center.distance(state.anchor);
            if lateral > 0.5 * RADIUS {
                continue;
            }
            let in_view = node
                .vertices
                .iter()
                .any(|v| inside_frustum(mvp, morph_point(*v, &state)));
            assert!(
                !in_view || report.visible.contains(&index),
                "rendered chunk {} popped at altitude {altitude} (flatten {})",
                node.name,
                state.flatten_factor
            );
        }
    }
    destroy_mesh(&mesh.faces[0]);
}
