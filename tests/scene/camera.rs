use glam::Vec2;
use planet_crafter_engine::scene::{DisplayOptions, OrbitCamera, ViewMode, build_scene};
use planet_crafter_engine::testing::{MAX_PITCH, MAX_ZOOM, MIN_ZOOM};

use planet_crafter_tests::fixtures::test_node;

#[test]
fn camera_fits_all_geometry_in_clip_space_at_any_angle() {
    let node = test_node();
    let mesh = build_scene(&[node], &DisplayOptions::default(), ViewMode::Mesh);

    // The bounding-sphere fit is angle-independent: the default head-on
    // camera and orbited cameras all keep every vertex inside clip space.
    for (yaw, pitch) in [(0.0, 0.0), (1.2, 0.6), (-2.4, -1.0)] {
        let mut camera = OrbitCamera::default();
        camera.orbit(yaw, pitch);
        let mvp = camera.view_projection(mesh.fit_center, mesh.fit_radius, Vec2::new(800.0, 800.0));
        for vertex in mesh.lines.iter().chain(&mesh.triangles) {
            let clip = mvp * vertex.pos.extend(1.0);
            let ndc = clip.truncate() / clip.w;
            assert!(ndc.x.abs() <= 1.0, "ndc.x {} out of range", ndc.x);
            assert!(ndc.y.abs() <= 1.0, "ndc.y {} out of range", ndc.y);
            assert!((0.0..=1.0).contains(&ndc.z), "ndc.z {} out of range", ndc.z);
        }
    }
}

#[test]
fn orbit_camera_orbits_and_clamps_pitch() {
    let mut camera = OrbitCamera::default();
    assert_eq!(camera.yaw(), 0.0);
    assert_eq!(camera.pitch(), 0.0);
    assert_eq!(camera.zoom(), 1.0);

    camera.orbit(0.5, 0.25);
    assert_eq!(camera.yaw(), 0.5);
    assert_eq!(camera.pitch(), 0.25);

    // The pitch clamp stops just short of the poles.
    camera.orbit(0.0, 100.0);
    assert_eq!(camera.pitch(), MAX_PITCH);
    camera.orbit(0.0, -200.0);
    assert_eq!(camera.pitch(), -MAX_PITCH);
}

#[test]
fn orbit_camera_zoom_clamps_and_reset_restores_default() {
    let mut camera = OrbitCamera::default();
    camera.zoom_by(1e9);
    assert_eq!(camera.zoom(), MAX_ZOOM);
    camera.zoom_by(1e-9);
    assert_eq!(camera.zoom(), MIN_ZOOM);

    camera.orbit(1.0, 0.5);
    camera.reset();
    assert_eq!(camera, OrbitCamera::default());
}
