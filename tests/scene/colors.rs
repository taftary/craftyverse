use glam::Vec3;
use planet_crafter_engine::node::split_node;
use planet_crafter_engine::scene::{DisplayOptions, ViewMode, build_scene};
use planet_crafter_engine::testing::{
    DIRECTION_COLORS, DOT_SEGMENTS, LEVEL_COLORS, hex_rgb, level_color,
};

use planet_crafter_tests::fixtures::test_node;

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
    let mesh = build_scene(&[center], &options, ViewMode::Mesh);

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
fn level_color_cycles_through_palette() {
    assert_eq!(level_color(0), LEVEL_COLORS[0]);
    assert_eq!(level_color(7), LEVEL_COLORS[7]);
    assert_eq!(level_color(8), LEVEL_COLORS[0]);
}

#[test]
fn hex_rgb_parses_channels() {
    assert_eq!(hex_rgb("#000000"), [0.0; 3]);
    assert_eq!(hex_rgb("#ffffff"), [1.0; 3]);
    assert_eq!(hex_rgb("#ff0000"), [1.0, 0.0, 0.0]);
}

#[test]
fn link_violations_are_highlighted() {
    use planet_crafter_engine::node::build_icosphere;
    use planet_crafter_engine::testing::VIOLATION_COLOR;

    let mut options = DisplayOptions::none();
    options.link_violations = true;

    // Healthy meshes emit no highlight: a split node and the icosphere
    // (whose welded links all carry a recorded back-port) are fully intact.
    let center = split_node(&test_node().borrow());
    let scene = build_scene(&[center], &options, ViewMode::Mesh);
    assert!(scene.lines.is_empty() && scene.triangles.is_empty());
    let icosphere = build_icosphere("t", 1.0, 1, Vec3::ZERO);
    let scene = build_scene(&icosphere.faces, &options, ViewMode::Mesh);
    assert!(scene.lines.is_empty() && scene.triangles.is_empty());

    // A manually broken link (child set without the link wiring, so no
    // back-port record) is highlighted on the offending edge.
    let broken = test_node();
    broken.borrow_mut().children[0] = Some(test_node());
    let scene = build_scene(&[broken], &options, ViewMode::Mesh);
    assert_eq!(scene.lines.len(), 2);
    assert_eq!(scene.triangles.len(), DOT_SEGMENTS * 3);
    assert!(scene.lines.iter().all(|v| v.color == VIOLATION_COLOR));
}
