use glam::Vec2;
use planet_crafter_engine::node::split_node;
use planet_crafter_engine::scene::{
    Attribute, DisplayOptions, PanelItem, Port, TextureEffect, ViewMode, build_scene,
};
use planet_crafter_engine::testing::{ATTRIBUTES, DIRECTION_COLORS, DOT_SEGMENTS, EFFECTS};

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
    let row_count = ATTRIBUTES.len() + EFFECTS.len();
    assert_eq!(mesh.texts.len(), row_count);
    assert_eq!(mesh.panel_rows.len(), row_count);
    // Unticked boxes: outlines but no fills — except the radio row of the
    // active texture effect, which always has exactly one selection.
    assert_eq!(mesh.ui_lines.len(), row_count * 4 * 2);
    assert_eq!(mesh.ui_triangles.len(), 2 * 3);
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
    // Two of the checkboxes are unticked; the one active effect radio row
    // stays filled.
    assert_eq!(mesh.ui_triangles.len(), (ATTRIBUTES.len() - 2 + 1) * 2 * 3);
}

#[test]
fn effect_radio_rows_select_exactly_one_effect() {
    let node = test_node();
    for (selected, _) in EFFECTS {
        let options = DisplayOptions {
            effect: selected,
            ..DisplayOptions::default()
        };
        let mesh = build_scene(std::slice::from_ref(&node), &options, ViewMode::Mesh);

        // One panel row per attribute plus one per effect, in order.
        let effect_rows = &mesh.panel_rows[ATTRIBUTES.len()..];
        assert_eq!(effect_rows.len(), EFFECTS.len());
        for (row, &(effect, _)) in effect_rows.iter().zip(EFFECTS.iter()) {
            assert_eq!(row.item, PanelItem::Effect(effect));
        }
        // Fills: every attribute is on by default, plus exactly the
        // selected effect's radio row.
        assert_eq!(mesh.ui_triangles.len(), (ATTRIBUTES.len() + 1) * 2 * 3);
    }

    // The default effect is the checkerboard.
    let mesh = build_scene(
        std::slice::from_ref(&node),
        &DisplayOptions::default(),
        ViewMode::Mesh,
    );
    assert!(
        mesh.panel_rows
            .iter()
            .any(|row| row.item == PanelItem::Effect(TextureEffect::Checkerboard))
    );
}

#[test]
fn panel_row_contains_hit_tests_rectangle() {
    let node = test_node();
    let mesh = build_scene(&[node], &DisplayOptions::default(), ViewMode::Mesh);

    let row = mesh.panel_rows[0];
    let center = (row.min + row.max) / 2.0;
    assert!(row.contains(center));
    assert!(!row.contains(row.min - Vec2::ONE));
    assert!(!row.contains(row.max + Vec2::ONE));
}
