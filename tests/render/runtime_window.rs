// Tests of the runtime window's headless logic: the fly camera (player
// proxy), the fly-speed model, the clip planes and the overlay formatting.

use glam::{Vec2, Vec3};
use planet_crafter_engine::node::{build_icosphere, destroy_mesh};
use planet_crafter_engine::runtime::{PlanetConfig, PlanetRuntimeManager, PlanetaryLayer};
use planet_crafter_engine::testing::{
    CameraMode, FlyCamera, LodReadout, PoolStats, VisibilityReadout, active_distance_for,
    chunk_bounds, clip_planes, culling_camera, draw_camera, flattening_lines, fly_speed,
    hybrid_lines, lod_lines, morphed_chunk_bounds, overlay_lines, player_marker_vertices,
    pool_lines, visibility_lines,
};

fn config() -> PlanetConfig {
    PlanetConfig {
        planet_radius: 300.0,
        planet_origin: Vec3::new(0.0, 1000.0, 0.0),
        atmosphere_multiplier: 1.25,
        orbit_multiplier: 2.0,
        sky_altitude: 30.0,
    }
}

#[test]
fn fly_speed_scales_with_altitude_and_stays_positive() {
    assert_eq!(fly_speed(0.0), 1.0);
    assert_eq!(fly_speed(0.5), 1.0);
    // Negative altitude (below the surface) flies as fast as above it.
    assert_eq!(fly_speed(-0.5), 1.0);
    assert_eq!(fly_speed(100.0), 100.0);
    assert_eq!(fly_speed(-100.0), 100.0);
    assert!(fly_speed(1e9).is_finite());
}

#[test]
fn clip_planes_cover_the_whole_planet() {
    let config = config();
    let (near, far) = clip_planes(config.planet_radius + 1.0, config.planet_radius);
    assert!(near > 0.0 && near < far);
    // From just above the surface, the far plane reaches beyond the planet.
    assert!(far >= 3.0 * config.planet_radius);
    // From deep space the near plane stays small relative to the far plane.
    let (near, far) = clip_planes(100.0 * config.planet_radius, config.planet_radius);
    assert!(near < far * 0.1);
}

#[test]
fn spawn_starts_in_space_facing_the_planet() {
    let config = config();
    let camera = FlyCamera::spawn(&config);

    let manager = PlanetRuntimeManager::new(config).unwrap();
    let state = manager.update(camera.position());
    // Spawn is beyond the orbit threshold: the sweep starts in the space layer.
    assert_eq!(state.layer, PlanetaryLayer::Space);
    assert!(state.distance_to_center > config.orbit_radius());

    // The camera faces the planet: forward aligns with the direction to the center.
    let alignment = camera.forward().dot(state.direction_to_center);
    assert!(alignment > 0.999, "alignment: {alignment}");
}

#[test]
fn look_clamps_pitch_short_of_the_poles() {
    let config = config();
    let mut camera = FlyCamera::spawn(&config);
    camera.look(0.0, 100.0);
    let up = camera.forward().y;
    assert!(up < 1.0 && up > 0.99, "forward.y: {up}");
    camera.look(1.0, -200.0);
    assert!(camera.forward().y > -1.0);
}

#[test]
fn move_local_moves_along_the_camera_frame() {
    let config = config();
    let mut camera = FlyCamera::spawn(&config);
    let before = camera.position();

    // Forward flight decreases the distance to the planet center.
    camera.move_local(Vec3::new(0.0, 0.0, 10.0));
    let manager = PlanetRuntimeManager::new(config).unwrap();
    assert!(
        manager.update(camera.position()).distance_to_center
            < manager.update(before).distance_to_center
    );

    // Vertical flight is along world Y regardless of the look direction.
    let before = camera.position();
    camera.move_local(Vec3::new(0.0, 5.0, 0.0));
    assert!((camera.position() - before - Vec3::new(0.0, 5.0, 0.0)).length() < 1e-4);
}

#[test]
fn speed_factor_is_clamped() {
    let config = config();
    let mut camera = FlyCamera::spawn(&config);
    assert_eq!(camera.speed_factor(), 1.0);
    for _ in 0..100 {
        camera.adjust_speed_factor(2.0);
    }
    assert_eq!(camera.speed_factor(), 32.0);
    for _ in 0..200 {
        camera.adjust_speed_factor(0.5);
    }
    assert_eq!(camera.speed_factor(), 1.0 / 32.0);
}

#[test]
fn view_projection_frames_a_point_ahead() {
    let config = config();
    let camera = FlyCamera::spawn(&config);
    let viewport = Vec2::new(800.0, 600.0);
    let (near, far) = clip_planes(825.0, 300.0);
    let mvp = camera.view_projection(viewport, near, far);

    // The planet center is dead ahead: it projects to the viewport center.
    let clip = mvp * config.planet_origin.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    assert!(ndc.x.abs() < 1e-3 && ndc.y.abs() < 1e-3, "ndc: {ndc:?}");
    assert!((0.0..=1.0).contains(&ndc.z), "ndc.z: {}", ndc.z);
}

#[test]
fn view_projection_relative_shifts_the_frame_to_the_anchor() {
    let config = config();
    let camera = FlyCamera::spawn(&config);
    let viewport = Vec2::new(800.0, 600.0);
    let (near, far) = clip_planes(825.0, 300.0);
    let anchor = Vec3::new(100.0, 1300.0, -50.0);
    let shifted = camera.view_projection_relative(viewport, near, far, anchor);

    // The render path: the vertex shader subtracts the anchor, then the
    // anchor-relative matrix transforms. The result must match the plain
    // world-space transform of the unshifted point.
    let point = Vec3::new(120.0, 1000.0, -80.0);
    let clip_shifted = shifted * (point - anchor).extend(1.0);
    let clip_plain = camera.view_projection(viewport, near, far) * point.extend(1.0);
    let ndc_shifted = clip_shifted.truncate() / clip_shifted.w;
    let ndc_plain = clip_plain.truncate() / clip_plain.w;
    assert!(
        (ndc_shifted - ndc_plain).length() < 1e-4,
        "{ndc_shifted:?} vs {ndc_plain:?}"
    );
}

#[test]
fn flattening_lines_report_the_blend_state() {
    let config = config();
    let manager = PlanetRuntimeManager::new(config).unwrap();
    // Mid-sky player: factor 0.5, half-blended gravity.
    let player = Vec3::new(0.0, 1000.0 + 315.0, 0.0);
    let state = manager.update(player);
    let lines = flattening_lines(&state, config.planet_radius);
    let joined = lines.join("\n");

    assert!(joined.contains("gravity:            50% flat"), "{joined}");
    assert!(
        joined.contains("gravity direction:  (0.000, -1.000, 0.000)"),
        "{joined}"
    );
    // The sampled world flatten recovers the authoritative factor through
    // the surface-height query.
    assert!(joined.contains("world flatten:      0.5"), "{joined}");
    assert!(joined.contains("f32 error bound:"), "{joined}");

    // Above the sky top: fully spherical, fully radial gravity.
    let state = manager.update(Vec3::new(0.0, 1000.0 + 400.0, 0.0));
    let joined = flattening_lines(&state, config.planet_radius).join("\n");
    assert!(joined.contains("gravity:            0% flat"), "{joined}");
    assert!(joined.contains("world flatten:      0.000"), "{joined}");

    // At the surface: fully flat, and the player sits on the anchor so the
    // f32 bound is at its minimum.
    let state = manager.update(Vec3::new(0.0, 1000.0, 0.0));
    let joined = flattening_lines(&state, config.planet_radius).join("\n");
    assert!(joined.contains("gravity:            100% flat"), "{joined}");
    assert!(joined.contains("world flatten:      1.000"), "{joined}");
}

#[test]
fn overlay_lines_report_the_runtime_state() {
    let config = config();
    let manager = PlanetRuntimeManager::new(config).unwrap();
    let player = Vec3::new(0.0, 1000.0 + 350.0, 0.0);
    let state = manager.update(player);
    let lines = overlay_lines(&state, 12.5);
    let joined = lines.join("\n");

    assert!(joined.contains("distance to center: 350.0"), "{joined}");
    assert!(joined.contains("altitude:           50.0"), "{joined}");
    assert!(
        joined.contains("layer:              atmosphere"),
        "{joined}"
    );
    assert!(joined.contains("atmosphere factor:  0.333"), "{joined}");
    assert!(joined.contains("flatten factor:     0.000"), "{joined}");
    assert!(
        joined.contains("anchor:             (0.0, 1300.0, 0.0)"),
        "{joined}"
    );
    assert!(
        joined.contains("player position:    (0.0, 1350.0, 0.0)"),
        "{joined}"
    );
    assert!(joined.contains("fly speed:          12.5"), "{joined}");
}

#[test]
fn lod_lines_report_the_scheduler_state() {
    let lines = lod_lines(&LodReadout {
        active_chunks: 42,
        level_histogram: vec![(0, 18), (1, 12), (2, 12)],
        min_level: 1,
        queued_operations: 3,
        operations_budget: 2,
        operations_used: 2,
        total_splits: 120,
        total_merges: 45,
    });
    let joined = lines.join("\n");

    assert!(joined.contains("loaded chunks:      42"), "{joined}");
    assert!(
        joined.contains("chunk levels:       L0:18 L1:12 L2:12"),
        "{joined}"
    );
    assert!(joined.contains("level floor:        1"), "{joined}");
    assert!(joined.contains("queued operations:  3"), "{joined}");
    assert!(joined.contains("budget used:        2/2"), "{joined}");
    assert!(joined.contains("splits/merges:      120/45"), "{joined}");
}

#[test]
fn active_distance_shrinks_from_orbit_to_surface() {
    assert!((active_distance_for(0.0, 300.0) - 225.0).abs() < 1e-3);
    assert!((active_distance_for(1.0, 300.0) - 15.0).abs() < 1e-3);
    let mid = active_distance_for(0.5, 300.0);
    assert!(mid > 15.0 && mid < 225.0, "{mid}");
    // Out-of-range factors clamp instead of extrapolating.
    assert!((active_distance_for(-2.0, 300.0) - 225.0).abs() < 1e-3);
    assert!((active_distance_for(2.0, 300.0) - 15.0).abs() < 1e-3);
}

#[test]
fn hybrid_lines_report_camera_and_shell_counts() {
    let lines = hybrid_lines(5200.0, 80, 12);
    let joined = lines.join("\n");
    assert!(joined.contains("camera distance:    5200.0"), "{joined}");
    assert!(joined.contains("shell/near chunks:  80/12"), "{joined}");
}

#[test]
fn pool_lines_report_the_pool_state() {
    let stats = PoolStats {
        capacity: 1024,
        used: 42,
        pending_jobs: 2,
        queued_assignments: 0,
        workers: 3,
        completed_jobs: 512,
    };
    let lines = pool_lines(&stats, 378);
    let joined = lines.join("\n");

    assert!(joined.contains("pool capacity:      1024"), "{joined}");
    assert!(joined.contains("slots used/free:    42/982"), "{joined}");
    assert!(joined.contains("queued assignments: 0"), "{joined}");
    assert!(joined.contains("vertex writes:      378"), "{joined}");
    assert!(joined.contains("pending jobs:       2"), "{joined}");
    assert!(
        joined.contains("workers:            3 threads, 512 jobs done"),
        "{joined}"
    );
}

#[test]
fn chunk_bounds_cover_the_corners_plus_the_skirt_margin() {
    let mesh = build_icosphere("planet", 300.0, 1, Vec3::ZERO);
    for face in &mesh.faces {
        let node = face.borrow();
        let [a, b, c] = node.vertices;
        let shortest_edge = (b - a).length().min((c - b).length()).min((a - c).length());
        let expected_margin = 0.02 * shortest_edge;
        let sphere = chunk_bounds(face).sphere;
        assert_eq!(sphere.center, node.center);
        for vertex in node.vertices {
            assert!(
                sphere.center.distance(vertex) + expected_margin <= sphere.radius + 1e-4,
                "corner outside the bounds of {}",
                node.name
            );
        }
    }
    destroy_mesh(&mesh.faces[0]);
}

#[test]
fn visibility_lines_report_the_culling_state() {
    let lines = visibility_lines(&VisibilityReadout {
        camera: CameraMode::Navigation,
        tested: 80,
        visible: 23,
        frustum_culled: 40,
        horizon_culled: 17,
        draw_calls: 21,
    });
    let joined = lines.join("\n");

    assert!(
        joined.contains("camera:             navigation"),
        "{joined}"
    );
    assert!(joined.contains("chunks visible:     23/80"), "{joined}");
    assert!(joined.contains("frustum culled:     40"), "{joined}");
    assert!(joined.contains("horizon culled:     17"), "{joined}");
    assert!(joined.contains("draw calls:         21"), "{joined}");

    let player = visibility_lines(&VisibilityReadout {
        camera: CameraMode::Player,
        tested: 1,
        visible: 1,
        frustum_culled: 0,
        horizon_culled: 0,
        draw_calls: 1,
    });
    assert!(player.join("\n").contains("camera:             player"));
}

#[test]
fn culling_always_follows_the_player_camera() {
    let config = config();
    let player_camera = FlyCamera::spawn(&config);
    let mut nav_camera = FlyCamera::spawn(&config);
    nav_camera.move_local(Vec3::new(500.0, 0.0, 0.0));

    for mode in [CameraMode::Player, CameraMode::Navigation] {
        // Culling always consumes the player camera, in every mode.
        let cull = culling_camera(&player_camera, &nav_camera, mode);
        assert_eq!(cull, &player_camera);
    }
    // Rendering switches viewpoint only in navigation mode.
    assert_eq!(
        draw_camera(&player_camera, &nav_camera, CameraMode::Player),
        &player_camera
    );
    assert_eq!(
        draw_camera(&player_camera, &nav_camera, CameraMode::Navigation),
        &nav_camera
    );
}

#[test]
fn morphed_chunk_bounds_cover_the_morphed_corners() {
    // A player standing on the surface: full flatten. The morphed bounds
    // must cover the morphed (rendered) corners of every chunk.
    let config = config();
    let manager = PlanetRuntimeManager::new(config).unwrap();
    let ground = config.planet_origin + Vec3::Y * config.planet_radius;
    let state = manager.update(ground + Vec3::Y * 5.0);
    assert!(state.flatten_factor > 0.0);

    let mesh = build_icosphere("planet", config.planet_radius, 1, config.planet_origin);
    for face in &mesh.faces {
        let sphere = morphed_chunk_bounds(face, &state).sphere;
        for vertex in face.borrow().vertices {
            let morphed = planet_crafter_engine::runtime::morph_point(vertex, &state);
            assert!(
                sphere.center.distance(morphed) <= sphere.radius + 1e-3,
                "morphed corner outside the bounds of {}",
                face.borrow().name
            );
        }
    }
    destroy_mesh(&mesh.faces[0]);
}

#[test]
fn player_marker_is_a_centered_three_bar_cross() {
    let player = Vec3::new(10.0, 305.0, -20.0);
    let up = Vec3::Y;
    let half_size = 4.0;
    let vertices = player_marker_vertices(player, half_size, up);
    // Three bars, one quad each, two triangles per quad.
    assert_eq!(vertices.len(), 18);
    for vertex in &vertices {
        let pos = Vec3::from_array(vertex.pos);
        // Every vertex sits within one arm's extent of the player, and the
        // cross is centered: all positions are player +/- bar +/- width.
        assert!(
            (pos - player).length() <= 2.0 * half_size,
            "marker vertex too far from the player: {pos:?}"
        );
    }
    // The marker spans the full arm length along some direction.
    let max_distance = vertices
        .iter()
        .map(|v| (Vec3::from_array(v.pos) - player).length())
        .fold(0.0_f32, f32::max);
    assert!(max_distance >= half_size);
    // The marker morphs with the terrain: its center stays the morphed
    // player position at full flatten (the morph is affine).
    let config = config();
    let manager = PlanetRuntimeManager::new(config).unwrap();
    let ground = config.planet_origin + Vec3::Y * config.planet_radius;
    let state = manager.update(ground + Vec3::Y);
    let up = planet_crafter_engine::runtime::anchor_up(&state);
    let vertices = player_marker_vertices(ground, half_size, up);
    let center = vertices
        .iter()
        .map(|v| planet_crafter_engine::runtime::morph_point(Vec3::from_array(v.pos), &state))
        .sum::<Vec3>()
        / vertices.len() as f32;
    let morphed_player = planet_crafter_engine::runtime::morph_point(ground, &state);
    assert!(
        center.distance(morphed_player) < 0.1,
        "morphed marker drifted from the player: {center:?} vs {morphed_player:?}"
    );
}

// --- Player-terrain collision -------------------------------------------

use planet_crafter_engine::runtime::{anchor_up, surface_height};
use planet_crafter_engine::testing::{
    MIN_EYE_HEIGHT, clamp_above_surface, terrain_collision_applies,
};

/// The signed clearance of `position` above the rendered surface, along
/// the local vertical (the collision invariant).
fn clearance(state: &planet_crafter_engine::runtime::RuntimeState, position: Vec3) -> f32 {
    (position - state.anchor).dot(anchor_up(state))
        - surface_height(state, position).expect("position above the planet")
}

/// A manager/state pair for a player at `altitude` above the +Y pole.
fn state_at(altitude: f32) -> planet_crafter_engine::runtime::RuntimeState {
    let manager = PlanetRuntimeManager::new(config()).unwrap();
    manager.update(config().planet_origin + Vec3::Y * (config().planet_radius + altitude))
}

#[test]
fn collision_clamp_holds_at_flatten_zero() {
    // Atmosphere/orbit layers: the surface is the unmorphed sphere.
    let state = state_at(100.0);
    assert_eq!(state.flatten_factor, 0.0);
    let up = anchor_up(&state);
    let ground = config().planet_origin + up * config().planet_radius;

    // Below the surface: pushed out to the surface + minimum height.
    let below = ground - up * 10.0;
    let clamped = clamp_above_surface(below, &state, MIN_EYE_HEIGHT);
    assert!((clearance(&state, clamped) - MIN_EYE_HEIGHT).abs() < 1e-3);

    // Already above: unchanged.
    let above = ground + up * 10.0;
    assert_eq!(clamp_above_surface(above, &state, MIN_EYE_HEIGHT), above);

    // Deep inside the planet body: pushed out in one step.
    let inside = config().planet_origin + up * (config().planet_radius * 0.5);
    let clamped = clamp_above_surface(inside, &state, MIN_EYE_HEIGHT);
    assert!((clearance(&state, clamped) - MIN_EYE_HEIGHT).abs() < 1e-3);
}

#[test]
fn collision_clamp_holds_at_partial_and_full_flatten() {
    for altitude in [15.0, 0.0] {
        let state = state_at(altitude);
        assert!(state.flatten_factor > 0.0, "altitude {altitude}");
        let up = anchor_up(&state);
        let ground = config().planet_origin + up * config().planet_radius;

        // Below the rendered (morphed) surface: pushed out along the
        // local vertical to exactly the minimum clearance.
        let below = ground - up * 5.0;
        let clamped = clamp_above_surface(below, &state, MIN_EYE_HEIGHT);
        let correction = clamped - below;
        assert!(
            correction.cross(up).length() < 1e-4,
            "correction not purely vertical at altitude {altitude}: {correction:?}"
        );
        assert!(
            (clearance(&state, clamped) - MIN_EYE_HEIGHT).abs() < 1e-3,
            "altitude {altitude}: clearance {}",
            clearance(&state, clamped)
        );

        // Exactly at the anchor: pushed up to the minimum height.
        let clamped = clamp_above_surface(state.anchor, &state, MIN_EYE_HEIGHT);
        assert!(clearance(&state, clamped) >= MIN_EYE_HEIGHT - 1e-3);
    }
}

#[test]
fn collision_slides_instead_of_sticking() {
    // A move with a tangential and a downward component that ends below
    // the surface: the clamp restores the clearance WITHOUT touching the
    // tangential displacement (the correction is purely vertical).
    let state = state_at(0.0);
    let up = anchor_up(&state);
    let tangent = Vec3::X;
    let start = state.anchor + up * (MIN_EYE_HEIGHT + 2.0);
    let moved = start + tangent * 30.0 - up * 10.0;
    let clamped = clamp_above_surface(moved, &state, MIN_EYE_HEIGHT);

    let correction = clamped - moved;
    assert!(correction.dot(up) > 0.0, "must push out, not pull");
    assert!(
        correction.cross(up).length() < 1e-4,
        "tangential motion changed: {correction:?}"
    );
    // The full tangential travel survived the clamp.
    let tangential_travel = (clamped - start).dot(tangent);
    assert!((tangential_travel - 30.0).abs() < 1e-3);
    assert!(clearance(&state, clamped) >= MIN_EYE_HEIGHT - 1e-3);
}

#[test]
fn collision_holds_across_the_descent() {
    // Descending from above the sky top to the ground (flatten 0 -> 1):
    // at every step a player pushed below the rendered surface is clamped
    // back out, and the clearance still holds under the RECOMPUTED state
    // (the anchor tracks the clamped position).
    let manager = PlanetRuntimeManager::new(config()).unwrap();
    let up = Vec3::Y;
    for step in 0..20 {
        let t = step as f32 / 19.0;
        let altitude = 60.0 - 59.8 * t;
        let ground = config().planet_origin + up * (config().planet_radius + altitude);
        let state = manager.update(ground);
        let below = ground - up * 3.0;
        let clamped = clamp_above_surface(below, &state, MIN_EYE_HEIGHT);
        let new_state = manager.update(clamped);
        let new_clearance = surface_height(&new_state, clamped)
            .map(|surface| (clamped - new_state.anchor).dot(anchor_up(&new_state)) - surface);
        assert!(
            new_clearance.is_none_or(|c| c >= MIN_EYE_HEIGHT - 0.05),
            "altitude {altitude}: clearance {new_clearance:?} after re-anchor"
        );
    }
}

#[test]
fn navigation_camera_is_exempt_from_collision() {
    assert!(terrain_collision_applies(CameraMode::Player));
    assert!(!terrain_collision_applies(CameraMode::Navigation));
}
