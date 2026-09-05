use super::colors::{DIRECTION_COLORS, LEVEL_COLORS, hex_rgb, level_color};
use super::geometry::DOT_SEGMENTS;
use super::options::ATTRIBUTES;
use super::*;
use crate::node::{Node, split_node};
use glam::Vec3;

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

#[test]
fn single_node_emits_all_element_kinds() {
    let node = test_node();
    let mesh = build_scene(&[node], Vec2::new(800.0, 800.0), &DisplayOptions::default());

    // 3 outline + 4 arrow shafts + at least one dashed origin segment.
    assert!(mesh.lines.len() >= (3 + 4 + 1) * 2);
    assert_eq!(mesh.lines.len() % 2, 0);
    // 5 arrowheads + 16 dot segments + 3 open-port discs = 69 triangles.
    assert_eq!(mesh.triangles.len(), 69 * 3);
    // 1 name label + 3 corner labels + checkbox labels.
    assert_eq!(mesh.texts.len(), 4 + ATTRIBUTES.len());
    assert_eq!(mesh.texts[0].text, "root L0");
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
    let mesh = build_scene(&[node], Vec2::new(800.0, 800.0), &DisplayOptions::default());

    // Only outline + arrow shafts remain.
    assert_eq!(mesh.lines.len(), (3 + 4) * 2);
    // 4 arrowheads + dot segments + 3 open-port discs.
    assert_eq!(
        mesh.triangles.len(),
        (4 + DOT_SEGMENTS + 3 * DOT_SEGMENTS) * 3
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
    let mesh = build_scene(&nodes, Vec2::new(800.0, 800.0), &DisplayOptions::default());

    // 4 nodes × (3 outline + 4 shafts) + dashed origin segments +
    // 6 child links (3 center→corner, 3 corner→center).
    assert!(mesh.lines.len() >= (4 * 7 + 6) * 2);
    // 4 nodes × (5 arrowheads + 16 dot segments) + 6 open-port discs (two
    // per corner node; the center node is fully linked).
    assert_eq!(mesh.triangles.len(), (4 * 21 + 6 * DOT_SEGMENTS) * 3);
    // 4 nodes × (1 name label + 3 corner labels) + checkbox labels.
    assert_eq!(mesh.texts.len(), 16 + ATTRIBUTES.len());
    assert_eq!(mesh.texts[0].text, "root.C L1");
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
    let mesh = build_scene(&[center], Vec2::new(800.0, 800.0), &options);

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
    let mesh = build_scene(
        std::slice::from_ref(&node),
        Vec2::new(800.0, 800.0),
        &options,
    );

    // One disc per open port (all three are null), no lines.
    assert_eq!(mesh.triangles.len(), 3 * DOT_SEGMENTS * 3);
    assert!(mesh.lines.is_empty());
    // Every disc carries the direction color of its port; all three used.
    for color in DIRECTION_COLORS {
        assert!(mesh.triangles.iter().any(|vertex| vertex.color == color));
    }

    // The split center node is fully linked: no markers.
    let center = split_node(&node.borrow());
    let mesh = build_scene(
        std::slice::from_ref(&center),
        Vec2::new(800.0, 800.0),
        &options,
    );
    assert!(mesh.triangles.is_empty());

    // Each corner node has two open ports: two discs.
    let node_i = center.borrow().children[0].clone().unwrap();
    let mesh = build_scene(&[node_i], Vec2::new(800.0, 800.0), &options);
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
    let mesh = build_scene(
        std::slice::from_ref(&node),
        Vec2::new(800.0, 800.0),
        &options,
    );
    assert_eq!(mesh.triangles.len(), 2 * DOT_SEGMENTS * 3);
    assert!(
        mesh.triangles
            .iter()
            .all(|vertex| vertex.color != DIRECTION_COLORS[1])
    );

    // The master gates the whole group without touching the per-port
    // switches: no markers while off, the same selection returns when on.
    options.toggle(Attribute::OpenPorts);
    let mesh = build_scene(
        std::slice::from_ref(&node),
        Vec2::new(800.0, 800.0),
        &options,
    );
    assert!(mesh.triangles.is_empty());
    options.toggle(Attribute::OpenPorts);
    let mesh = build_scene(
        std::slice::from_ref(&node),
        Vec2::new(800.0, 800.0),
        &options,
    );
    assert_eq!(mesh.triangles.len(), 2 * DOT_SEGMENTS * 3);

    // Directions: only port I enabled — one shaft and one arrowhead, red.
    options.open_ports = false;
    options.directions = true;
    options.directions_ijk = [true, false, false];
    let mesh = build_scene(
        std::slice::from_ref(&node),
        Vec2::new(800.0, 800.0),
        &options,
    );
    assert_eq!(mesh.lines.len(), 2);
    assert_eq!(mesh.triangles.len(), 3);
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
    let mesh = build_scene(&[center], Vec2::new(800.0, 800.0), &options);
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
    let mesh = build_scene(&[node], Vec2::new(800.0, 800.0), &options);

    // Only the checkbox panel remains, so options can be turned back on.
    assert!(mesh.lines.is_empty() && mesh.triangles.is_empty());
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
    let mesh = build_scene(&[node], Vec2::new(800.0, 800.0), &options);

    // No direction arrowheads left; only the origin arrowhead + dot + the
    // three open-port discs.
    assert_eq!(
        mesh.triangles.len(),
        (1 + DOT_SEGMENTS + 3 * DOT_SEGMENTS) * 3
    );
    // Two of the checkboxes are unticked.
    assert_eq!(mesh.ui_triangles.len(), (ATTRIBUTES.len() - 2) * 2 * 3);
}

#[test]
fn checkbox_contains_hit_tests_rectangle() {
    let node = test_node();
    let mesh = build_scene(&[node], Vec2::new(800.0, 800.0), &DisplayOptions::default());

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
fn clip_transform_maps_all_geometry_inside_clip_space() {
    let node = test_node();
    let mesh = build_scene(&[node], Vec2::new(800.0, 800.0), &DisplayOptions::default());

    for vertex in mesh.lines.iter().chain(&mesh.triangles) {
        let clip = vertex.pos * mesh.world_to_clip.scale + mesh.world_to_clip.offset;
        assert!(clip.x.abs() <= 1.0, "clip.x {} out of range", clip.x);
        assert!(clip.y.abs() <= 1.0, "clip.y {} out of range", clip.y);
    }
}

#[test]
fn empty_scene_produces_identity_like_transform() {
    let mesh = build_scene(&[], Vec2::new(800.0, 800.0), &DisplayOptions::default());
    // No node geometry, but the checkbox panel is always generated.
    assert!(mesh.lines.is_empty() && mesh.triangles.is_empty());
    assert_eq!(mesh.texts.len(), ATTRIBUTES.len());
    assert_eq!(mesh.checkboxes.len(), ATTRIBUTES.len());
    assert_eq!(
        mesh.world_to_clip.scale,
        Vec2::new(2.0 / 800.0, -2.0 / 800.0)
    );
}

#[test]
fn hex_rgb_parses_channels() {
    assert_eq!(hex_rgb("#000000"), [0.0; 3]);
    assert_eq!(hex_rgb("#ffffff"), [1.0; 3]);
    assert_eq!(hex_rgb("#ff0000"), [1.0, 0.0, 0.0]);
}
