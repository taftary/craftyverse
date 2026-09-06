use super::camera::{MAX_PITCH, MAX_ZOOM, MIN_ZOOM};
use super::colors::{DIRECTION_COLORS, LEVEL_COLORS, hex_rgb, level_color};
use super::geometry::{DOT_SEGMENTS, plane_basis, push_arrowhead, push_disc};
use super::options::ATTRIBUTES;
use super::*;
use crate::node::{Node, split_node};
use glam::{Mat4, Vec2, Vec3};

fn point(x: f32, y: f32) -> Vec3 {
    Vec3::new(x, y, 0.0)
}

/// Equilateral test node (base 300, apex up, height = base * √3 / 2).
fn test_node() -> NodeRef {
    Node::new(
        "root",
        [
            point(0.0, 100.0 * 3.0_f32.sqrt()),
            point(150.0, -50.0 * 3.0_f32.sqrt()),
            point(-150.0, -50.0 * 3.0_f32.sqrt()),
        ],
        Vec3::new(0.0, 1000.0, 0.0),
    )
}

/// Projects `point` with `mvp` into `viewport` pixels (same mapping as
/// `project_labels`).
fn to_pixel(mvp: &Mat4, point: Vec3, viewport: Vec2) -> Vec2 {
    let clip = *mvp * point.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    Vec2::new(
        (ndc.x + 1.0) * 0.5 * viewport.x,
        (ndc.y + 1.0) * 0.5 * viewport.y,
    )
}

#[test]
fn single_node_emits_all_element_kinds() {
    let node = test_node();
    let mesh = build_scene(&[node], &DisplayOptions::default());

    // 3 outline + 4 arrow shafts + at least one dashed origin segment.
    assert!(mesh.lines.len() >= (3 + 4 + 1) * 2);
    assert_eq!(mesh.lines.len() % 2, 0);
    // 5 arrowheads × 2 fins + 16 dot segments + 3 open-port discs = 74.
    assert_eq!(mesh.triangles.len(), 74 * 3);
    // 1 name label + 3 corner labels, anchored in world space.
    assert_eq!(mesh.labels.len(), 4);
    assert_eq!(mesh.labels[0].text, "root L0");
    // Checkbox labels are pixel-space text runs.
    assert_eq!(mesh.texts.len(), ATTRIBUTES.len());
    // One checkbox row per attribute, all ticked by default.
    assert_eq!(mesh.checkboxes.len(), ATTRIBUTES.len());
    assert_eq!(mesh.ui_lines.len(), ATTRIBUTES.len() * 4 * 2);
    assert_eq!(mesh.ui_triangles.len(), ATTRIBUTES.len() * 2 * 3);
}

#[test]
fn origin_arrow_is_skipped_when_node_is_at_origin() {
    let node = Node::new(
        "at_origin",
        [
            point(0.0, 4.0 / 3.0),
            point(1.5, -2.0 / 3.0),
            point(-1.5, -2.0 / 3.0),
        ],
        Vec3::ZERO,
    );
    let mesh = build_scene(&[node], &DisplayOptions::default());

    // Only outline + arrow shafts remain.
    assert_eq!(mesh.lines.len(), (3 + 4) * 2);
    // 4 arrowheads × 2 fins + dot segments + 3 open-port discs.
    assert_eq!(
        mesh.triangles.len(),
        (4 * 2 + DOT_SEGMENTS + 3 * DOT_SEGMENTS) * 3
    );
}

#[test]
fn split_scene_links_children_and_labels_all_nodes() {
    let node = test_node();
    let center = split_node(&node.borrow());
    let mut nodes = vec![center.clone()];
    for corner in center.borrow().children.iter().flatten() {
        nodes.push(corner.clone());
    }
    let mesh = build_scene(&nodes, &DisplayOptions::default());

    // 4 nodes × (3 outline + 4 shafts) + dashed origin segments +
    // 6 child links (3 center→corner, 3 corner→center).
    assert!(mesh.lines.len() >= (4 * 7 + 6) * 2);
    // 4 nodes × (5 arrowheads × 2 fins + 16 dot segments) + 6 open-port
    // discs (two per corner node; the center node is fully linked).
    assert_eq!(
        mesh.triangles.len(),
        (4 * (5 * 2 + DOT_SEGMENTS) + 6 * DOT_SEGMENTS) * 3
    );
    // 4 nodes × (1 name label + 3 corner labels).
    assert_eq!(mesh.labels.len(), 16);
    assert_eq!(mesh.labels[0].text, "root.C L1");
    assert_eq!(mesh.texts.len(), ATTRIBUTES.len());
}

#[test]
fn child_links_are_dashed_and_colored_by_direction() {
    let node = test_node();
    let center = split_node(&node.borrow());
    // Child links only: every emitted line is part of a dashed link.
    let options = DisplayOptions {
        child_links: true,
        child_links_ijk: [true; 3],
        ..DisplayOptions::none()
    };
    let mesh = build_scene(&[center], &options);

    // Dashed: the three links are split into more than one segment each.
    assert!(mesh.lines.len() > 3 * 2);
    assert_eq!(mesh.lines.len() % 2, 0);
    // Every segment carries one of the I/J/K direction colors, and all
    // three are used.
    assert!(
        mesh.lines
            .iter()
            .all(|vertex| DIRECTION_COLORS.contains(&vertex.color))
    );
    for color in DIRECTION_COLORS {
        assert!(mesh.lines.iter().any(|vertex| vertex.color == color));
    }
}

#[test]
fn open_ports_emit_bold_disc_per_null_port() {
    let node = test_node();
    // Open ports only: every emitted triangle is part of a port marker.
    let options = DisplayOptions {
        open_ports: true,
        open_ports_ijk: [true; 3],
        ..DisplayOptions::none()
    };
    let mesh = build_scene(std::slice::from_ref(&node), &options);

    // One disc per open port (all three are null), no lines.
    assert_eq!(mesh.triangles.len(), 3 * DOT_SEGMENTS * 3);
    assert!(mesh.lines.is_empty());
    // Every disc carries the direction color of its port; all three used.
    for color in DIRECTION_COLORS {
        assert!(mesh.triangles.iter().any(|vertex| vertex.color == color));
    }

    // The split center node is fully linked: no markers.
    let center = split_node(&node.borrow());
    let mesh = build_scene(std::slice::from_ref(&center), &options);
    assert!(mesh.triangles.is_empty());

    // Each corner node has two open ports: two discs.
    let node_i = center.borrow().children[0].clone().unwrap();
    let mesh = build_scene(&[node_i], &options);
    assert_eq!(mesh.triangles.len(), 2 * DOT_SEGMENTS * 3);
}

#[test]
fn per_port_switches_gate_each_group() {
    let node = test_node();
    let mut options = DisplayOptions {
        open_ports: true,
        open_ports_ijk: [true; 3],
        ..DisplayOptions::none()
    };

    // One port off: two discs remain, none in the disabled port's color.
    options.toggle(Attribute::OpenPort(1));
    let mesh = build_scene(std::slice::from_ref(&node), &options);
    assert_eq!(mesh.triangles.len(), 2 * DOT_SEGMENTS * 3);
    assert!(
        mesh.triangles
            .iter()
            .all(|vertex| vertex.color != DIRECTION_COLORS[1])
    );

    // The master gates the whole group without touching the per-port
    // switches: no markers while off, the same selection returns when on.
    options.toggle(Attribute::OpenPorts);
    let mesh = build_scene(std::slice::from_ref(&node), &options);
    assert!(mesh.triangles.is_empty());
    options.toggle(Attribute::OpenPorts);
    let mesh = build_scene(std::slice::from_ref(&node), &options);
    assert_eq!(mesh.triangles.len(), 2 * DOT_SEGMENTS * 3);

    // Directions: only port I enabled — one shaft and one two-fin arrowhead.
    options.open_ports = false;
    options.directions = true;
    options.directions_ijk = [true, false, false];
    let mesh = build_scene(std::slice::from_ref(&node), &options);
    assert_eq!(mesh.lines.len(), 2);
    assert_eq!(mesh.triangles.len(), 6);
    assert!(
        mesh.lines
            .iter()
            .chain(&mesh.triangles)
            .all(|vertex| vertex.color == DIRECTION_COLORS[0])
    );

    // Child links: the split center has three links; keep only I and K.
    let center = split_node(&node.borrow());
    options.directions = false;
    options.child_links = true;
    options.child_links_ijk = [true, false, true];
    let mesh = build_scene(&[center], &options);
    assert!(!mesh.lines.is_empty());
    assert!(
        mesh.lines
            .iter()
            .all(|vertex| vertex.color != DIRECTION_COLORS[1])
    );
    for color in [DIRECTION_COLORS[0], DIRECTION_COLORS[2]] {
        assert!(mesh.lines.iter().any(|vertex| vertex.color == color));
    }
}

#[test]
fn disabled_attributes_emit_no_geometry() {
    let node = test_node();
    let options = DisplayOptions::none();
    let mesh = build_scene(&[node], &options);

    // Only the checkbox panel remains, so options can be turned back on.
    assert!(mesh.lines.is_empty() && mesh.triangles.is_empty());
    assert!(mesh.labels.is_empty());
    assert_eq!(mesh.texts.len(), ATTRIBUTES.len());
    assert_eq!(mesh.checkboxes.len(), ATTRIBUTES.len());
    // Unticked boxes: outlines but no fills.
    assert_eq!(mesh.ui_lines.len(), ATTRIBUTES.len() * 4 * 2);
    assert!(mesh.ui_triangles.is_empty());
}

#[test]
fn toggle_gates_each_attribute() {
    let node = test_node();
    let mut options = DisplayOptions::default();
    options.toggle(Attribute::Directions);
    options.toggle(Attribute::DirectionOfNode);
    assert!(!options.value(Attribute::Directions));
    assert!(!options.value(Attribute::DirectionOfNode));
    let mesh = build_scene(&[node], &options);

    // No direction arrowheads left; only the origin arrowhead (two fins) +
    // dot + the three open-port discs.
    assert_eq!(
        mesh.triangles.len(),
        (2 + DOT_SEGMENTS + 3 * DOT_SEGMENTS) * 3
    );
    // Two of the checkboxes are unticked.
    assert_eq!(mesh.ui_triangles.len(), (ATTRIBUTES.len() - 2) * 2 * 3);
}

#[test]
fn checkbox_contains_hit_tests_rectangle() {
    let node = test_node();
    let mesh = build_scene(&[node], &DisplayOptions::default());

    let checkbox = mesh.checkboxes[0];
    let center = (checkbox.min + checkbox.max) / 2.0;
    assert!(checkbox.contains(center));
    assert!(!checkbox.contains(checkbox.min - Vec2::ONE));
    assert!(!checkbox.contains(checkbox.max + Vec2::ONE));
}

#[test]
fn level_color_cycles_through_palette() {
    assert_eq!(level_color(0), LEVEL_COLORS[0]);
    assert_eq!(level_color(7), LEVEL_COLORS[7]);
    assert_eq!(level_color(8), LEVEL_COLORS[0]);
}

#[test]
fn camera_fits_all_geometry_in_clip_space_at_any_angle() {
    let node = test_node();
    let mesh = build_scene(&[node], &DisplayOptions::default());

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
fn empty_scene_produces_unit_fit_sphere() {
    let mesh = build_scene(&[], &DisplayOptions::default());
    // No node geometry, but the checkbox panel is always generated.
    assert!(mesh.lines.is_empty() && mesh.triangles.is_empty());
    assert!(mesh.labels.is_empty());
    assert_eq!(mesh.texts.len(), ATTRIBUTES.len());
    assert_eq!(mesh.checkboxes.len(), ATTRIBUTES.len());
    assert_eq!(mesh.fit_center, Vec3::ZERO);
    assert_eq!(mesh.fit_radius, 1.0);
}

#[test]
fn hex_rgb_parses_channels() {
    assert_eq!(hex_rgb("#000000"), [0.0; 3]);
    assert_eq!(hex_rgb("#ffffff"), [1.0; 3]);
    assert_eq!(hex_rgb("#ff0000"), [1.0, 0.0, 0.0]);
}

#[test]
fn link_violations_are_highlighted() {
    use super::colors::VIOLATION_COLOR;
    use crate::node::build_icosphere;

    let mut options = DisplayOptions::none();
    options.link_violations = true;

    // Healthy meshes emit no highlight: a split node and the icosphere
    // (whose welded links all carry a recorded back-port) are fully intact.
    let center = split_node(&test_node().borrow());
    let scene = build_scene(&[center], &options);
    assert!(scene.lines.is_empty() && scene.triangles.is_empty());
    let icosphere = build_icosphere("t", 1.0, 1, Vec3::ZERO);
    let scene = build_scene(&icosphere.faces, &options);
    assert!(scene.lines.is_empty() && scene.triangles.is_empty());

    // A manually broken link (child set without the link wiring, so no
    // back-port record) is highlighted on the offending edge.
    let broken = test_node();
    broken.borrow_mut().children[0] = Some(test_node());
    let scene = build_scene(&[broken], &options);
    assert_eq!(scene.lines.len(), 2);
    assert_eq!(scene.triangles.len(), DOT_SEGMENTS * 3);
    assert!(scene.lines.iter().all(|v| v.color == VIOLATION_COLOR));
}

#[test]
fn plane_basis_returns_orthonormal_vectors() {
    for direction in [Vec3::X, Vec3::Y, Vec3::new(1.0, 2.0, 3.0)] {
        let n = direction.normalize();
        let (u, v) = plane_basis(direction);
        assert!((u.length() - 1.0).abs() < 1e-6);
        assert!((v.length() - 1.0).abs() < 1e-6);
        assert!(u.dot(n).abs() < 1e-6);
        assert!(v.dot(n).abs() < 1e-6);
        assert!(u.dot(v).abs() < 1e-6);
    }
    // Degenerate directions fall back to the XY plane instead of panicking.
    let (u, v) = plane_basis(Vec3::ZERO);
    assert!(u.dot(Vec3::Z).abs() < 1e-6);
    assert!(v.dot(Vec3::Z).abs() < 1e-6);
    assert!((u.length() - 1.0).abs() < 1e-6);
    assert!((v.length() - 1.0).abs() < 1e-6);
}

#[test]
fn disc_lies_in_plane_perpendicular_to_normal() {
    let center = Vec3::new(1.0, 2.0, 3.0);
    let mut buf = Vec::new();
    push_disc(&mut buf, center, Vec3::X, 0.5, DOT_SEGMENTS, [1.0; 3]);
    assert_eq!(buf.len(), DOT_SEGMENTS * 3);
    for vertex in &buf {
        assert!(
            (vertex.pos.x - center.x).abs() < 1e-6,
            "disc vertex off the x plane: {:?}",
            vertex.pos
        );
        assert!((vertex.pos - center).length() <= 0.5 + 1e-6);
    }
}

#[test]
fn arrowhead_has_two_perpendicular_fins() {
    let tip = Vec3::new(1.0, 0.0, 0.0);
    let mut buf = Vec::new();
    push_arrowhead(&mut buf, tip, Vec3::X, 0.25, [1.0; 3]);
    // Two triangles (fins), all within the head size of the tip.
    assert_eq!(buf.len(), 2 * 3);
    for vertex in &buf {
        assert!((vertex.pos - tip).length() <= 0.25 * 1.5);
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

#[test]
fn project_labels_anchors_centers_and_drops_behind_camera() {
    let node = test_node();
    let mesh = build_scene(&[node], &DisplayOptions::default());
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
            offset: LabelOffset::Outward(center, 8.0),
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
