use glam::Vec2;
use planet_crafter_engine::node::split_node;
use planet_crafter_engine::scene::{Attribute, DisplayOptions, Port, ViewMode, build_scene};
use planet_crafter_engine::testing::{ATTRIBUTES, DIRECTION_COLORS, DOT_SEGMENTS};

use planet_crafter_tests::fixtures::test_node;

#[test]
fn per_port_switches_gate_each_group() {
    let node = test_node();
    let mut options = DisplayOptions {
        open_ports: true,
        open_ports_ijk: [true; 3],
        ..DisplayOptions::none()
    };

    // One port off: two discs remain, none in the disabled port's color.
    options.toggle(Attribute::OpenPort(Port::J));
    let mesh = build_scene(std::slice::from_ref(&node), &options, ViewMode::Mesh);
    assert_eq!(mesh.triangles.len(), 2 * DOT_SEGMENTS * 3);
    assert!(
        mesh.triangles
            .iter()
            .all(|vertex| vertex.color != DIRECTION_COLORS[1])
    );

    // The master gates the whole group without touching the per-port
    // switches: no markers while off, the same selection returns when on.
    options.toggle(Attribute::OpenPorts);
    let mesh = build_scene(std::slice::from_ref(&node), &options, ViewMode::Mesh);
    assert!(mesh.triangles.is_empty());
    options.toggle(Attribute::OpenPorts);
    let mesh = build_scene(std::slice::from_ref(&node), &options, ViewMode::Mesh);
    assert_eq!(mesh.triangles.len(), 2 * DOT_SEGMENTS * 3);

    // Directions: only port I enabled — one shaft and one two-fin arrowhead.
    options.open_ports = false;
    options.directions = true;
    options.directions_ijk = [true, false, false];
    let mesh = build_scene(std::slice::from_ref(&node), &options, ViewMode::Mesh);
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
    let mesh = build_scene(&[center], &options, ViewMode::Mesh);
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
    let mesh = build_scene(&[node], &options, ViewMode::Mesh);

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
    let mesh = build_scene(&[node], &options, ViewMode::Mesh);

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
    let mesh = build_scene(&[node], &DisplayOptions::default(), ViewMode::Mesh);

    let checkbox = mesh.checkboxes[0];
    let center = (checkbox.min + checkbox.max) / 2.0;
    assert!(checkbox.contains(center));
    assert!(!checkbox.contains(checkbox.min - Vec2::ONE));
    assert!(!checkbox.contains(checkbox.max + Vec2::ONE));
}
