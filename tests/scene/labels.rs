use glam::{Mat4, Vec2, Vec3};
use planet_crafter_engine::scene::{
    DisplayOptions, LabelOffset, OrbitCamera, ViewMode, WorldLabel, build_scene, project_labels,
};

use planet_crafter_tests::fixtures::test_node;

/// Projects `point` with `mvp` into `viewport` pixels (same mapping as
/// `project_labels`).
fn to_pixel(mvp: &Mat4, point: Vec3, viewport: Vec2) -> Vec2 {
    let clip = *mvp * point.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    Vec2::new(
        (ndc.x + 1.0) * 0.5 * viewport.x,
        (1.0 - ndc.y) * 0.5 * viewport.y,
    )
}

#[test]
fn labels_only_scene_fits_label_anchors_on_screen() {
    // Labels on, every geometry attribute off: no world vertices are
    // emitted, but the label anchors must still drive the camera fit.
    let node = test_node();
    let options = DisplayOptions {
        labels: true,
        ..DisplayOptions::none()
    };
    let mesh = build_scene(std::slice::from_ref(&node), &options, ViewMode::Mesh);
    assert!(mesh.lines.is_empty() && mesh.triangles.is_empty());
    assert_eq!(mesh.labels.len(), 4);
    // The fit covers the node's extent, not the collapsed MIN_FIT_RADIUS.
    assert!(mesh.fit_radius > 100.0, "fit radius {}", mesh.fit_radius);

    // Every label projects inside the viewport with the default camera.
    let viewport = Vec2::new(800.0, 600.0);
    let mvp = OrbitCamera::default().view_projection(mesh.fit_center, mesh.fit_radius, viewport);
    let runs = project_labels(&mesh.labels, &mvp, viewport);
    assert_eq!(runs.len(), mesh.labels.len());
    for run in &runs {
        assert!(
            run.anchor.cmpge(Vec2::ZERO).all() && run.anchor.cmple(viewport).all(),
            "label anchor {:?} outside the viewport",
            run.anchor
        );
    }
}

#[test]
fn project_labels_anchors_centers_and_drops_behind_camera() {
    let node = test_node();
    let mesh = build_scene(&[node], &DisplayOptions::default(), ViewMode::Mesh);
    let viewport = Vec2::new(800.0, 600.0);
    let mvp = OrbitCamera::default().view_projection(mesh.fit_center, mesh.fit_radius, viewport);

    let labels = [
        WorldLabel {
            text: "center".into(),
            world_pos: mesh.fit_center,
            offset: LabelOffset::Fixed(Vec2::ZERO),
            size_px: 12.0,
            color: [0.0; 3],
            centered: false,
        },
        WorldLabel {
            text: "behind".into(),
            // Far behind the default camera (which sits on +Z of the center).
            world_pos: mesh.fit_center + Vec3::Z * 1e9,
            offset: LabelOffset::Fixed(Vec2::ZERO),
            size_px: 12.0,
            color: [0.0; 3],
            centered: false,
        },
    ];
    let runs = project_labels(&labels, &mvp, viewport);

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "center");
    let expected = to_pixel(&mvp, mesh.fit_center, viewport);
    assert!((runs[0].anchor - expected).length() < 1e-3);
    // The content center projects to (nearly) the viewport center.
    assert!((runs[0].anchor - viewport / 2.0).length() < 1.0);
}

#[test]
fn project_labels_pushes_corner_labels_outward() {
    let center = Vec3::ZERO;
    let corner = Vec3::X * 100.0;
    let viewport = Vec2::new(800.0, 600.0);
    let mvp = OrbitCamera::default().view_projection(center, 100.0, viewport);

    let labels = [
        WorldLabel {
            text: "raw".into(),
            world_pos: corner,
            offset: LabelOffset::Fixed(Vec2::ZERO),
            size_px: 10.0,
            color: [0.0; 3],
            centered: true,
        },
        WorldLabel {
            text: "corner".into(),
            world_pos: corner,
            offset: LabelOffset::Outward {
                from: center,
                distance_px: 8.0,
            },
            size_px: 10.0,
            color: [0.0; 3],
            centered: true,
        },
    ];
    let runs = project_labels(&labels, &mvp, viewport);

    // The outward label sits 8 px from the raw anchor, pushed further away
    // from the projected center (which lands right of it: corner is at +X).
    let offset = runs[1].anchor - runs[0].anchor;
    assert!((offset.length() - 8.0).abs() < 1e-3);
    let center_px = to_pixel(&mvp, center, viewport);
    assert!(runs[0].anchor.x > center_px.x);
    assert!(offset.x > 0.0);
    assert!(offset.y.abs() < 1e-3);
}
