use glam::Vec3;
use planet_crafter_engine::node::split_node;
use planet_crafter_engine::scene::{DisplayOptions, ViewMode, build_scene};
use planet_crafter_engine::testing::{ATTRIBUTES, DOT_SEGMENTS};

use planet_crafter_tests::fixtures::test_node;

#[test]
fn single_node_emits_all_element_kinds() {
    let node = test_node();
    let mesh = build_scene(&[node], &DisplayOptions::default(), ViewMode::Mesh);

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
fn split_scene_links_children_and_labels_all_nodes() {
    let node = test_node();
    let center = split_node(&node.borrow());
    let mut nodes = vec![center.clone()];
    for corner in center.borrow().children.iter().flatten() {
        nodes.push(corner.clone());
    }
    let mesh = build_scene(&nodes, &DisplayOptions::default(), ViewMode::Mesh);

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
fn empty_scene_produces_unit_fit_sphere() {
    let mesh = build_scene(&[], &DisplayOptions::default(), ViewMode::Mesh);
    // No node geometry, but the checkbox panel is always generated.
    assert!(mesh.lines.is_empty() && mesh.triangles.is_empty());
    assert!(mesh.labels.is_empty());
    assert_eq!(mesh.texts.len(), ATTRIBUTES.len());
    assert_eq!(mesh.checkboxes.len(), ATTRIBUTES.len());
    assert_eq!(mesh.fit_center, Vec3::ZERO);
    assert_eq!(mesh.fit_radius, 1.0);
}
