// Tests of the runtime window's headless logic: the fly camera (player
// proxy), the fly-speed model, the clip planes and the overlay formatting.

use glam::{Vec2, Vec3};
use planet_crafter_engine::node::{build_icosphere, destroy_mesh};
use planet_crafter_engine::runtime::{PlanetConfig, PlanetRuntimeManager, PlanetaryLayer};
use planet_crafter_engine::testing::{
    FlyCamera, LodReadout, PoolStats, VisibilityReadout, chunk_bounds, clip_planes,
    flattening_lines, fly_speed, lod_lines, overlay_lines, pool_lines, visibility_lines,
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
    assert!(joined.contains("queued operations:  3"), "{joined}");
    assert!(joined.contains("budget used:        2/2"), "{joined}");
    assert!(joined.contains("splits/merges:      120/45"), "{joined}");
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
        camera_detached: true,
        tested: 80,
        visible: 23,
        frustum_culled: 40,
        horizon_culled: 17,
        draw_calls: 21,
    });
    let joined = lines.join("\n");

    assert!(joined.contains("camera:             detached"), "{joined}");
    assert!(joined.contains("chunks visible:     23/80"), "{joined}");
    assert!(joined.contains("frustum culled:     40"), "{joined}");
    assert!(joined.contains("horizon culled:     17"), "{joined}");
    assert!(joined.contains("draw calls:         21"), "{joined}");

    let attached = visibility_lines(&VisibilityReadout {
        camera_detached: false,
        tested: 1,
        visible: 1,
        frustum_culled: 0,
        horizon_culled: 0,
        draw_calls: 1,
    });
    assert!(attached.join("\n").contains("camera:             attached"));
}
