use glam::Vec3;
use planet_crafter_engine::node::{Node, split_node};
use planet_crafter_engine::scene::{DisplayOptions, build_scene};
use planet_crafter_engine::testing::{
    DIRECTION_COLORS, DOT_SEGMENTS, plane_basis, push_arrowhead, push_disc,
};

use planet_crafter_tests::fixtures::{point, test_node};

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
