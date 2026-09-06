use glam::Vec3;
use planet_crafter_engine::node::{Node, split_node};
use planet_crafter_engine::render::Scenario;

fn main() {
    let origin = Vec3::new(0.0, 1000.0, 0.0);
    // Equilateral triangles pointing up: height = base * sqrt(3) / 2.
    let height = 300.0 * 3.0_f32.sqrt() / 2.0;
    let points = [
        Vec3::new(0.0, 2.0 * height / 3.0, 0.0),
        Vec3::new(150.0, -height / 3.0, 0.0),
        Vec3::new(-150.0, -height / 3.0, 0.0),
    ];

    // 1. One node, without split.
    let no_split = Scenario::Static(vec![Node::new("root", points, origin)]);

    // 2. One node with split — display only the split nodes.
    let parent = Node::new("parent", points, origin);
    let center = split_node(&parent.borrow());
    let mut split_nodes = vec![center.clone()];
    for corner in center.borrow().children.iter().flatten() {
        split_nodes.push(corner.clone());
    }
    let split = Scenario::Static(split_nodes);

    // 3. Parametric icosphere — arrow keys adjust subdivisions and radius.
    let icosphere = Scenario::icosphere("planet", 300.0, 1, origin);

    // Opens the Vulkan viewer window; number keys select the scene.
    planet_crafter_engine::render::run(vec![no_split, split, icosphere]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenarios_have_expected_node_counts() {
        // Static scenario with one unsplit node.
        let height = 300.0 * 3.0_f32.sqrt() / 2.0;
        let node = Node::new(
            "root",
            [
                Vec3::new(0.0, 2.0 * height / 3.0, 0.0),
                Vec3::new(150.0, -height / 3.0, 0.0),
                Vec3::new(-150.0, -height / 3.0, 0.0),
            ],
            Vec3::new(0.0, 1000.0, 0.0),
        );
        let no_split = Scenario::Static(vec![node.clone()]);
        assert_eq!(no_split.nodes().len(), 1);

        // Static scenario with a split node: center + 3 corners.
        let center = split_node(&node.borrow());
        let mut split_nodes = vec![center.clone()];
        for corner in center.borrow().children.iter().flatten() {
            split_nodes.push(corner.clone());
        }
        let split = Scenario::Static(split_nodes);
        assert_eq!(split.nodes().len(), 4);
    }

    #[test]
    fn icosphere_scenario_builds_and_adjusts() {
        let mut scenario = Scenario::icosphere("planet", 300.0, 1, Vec3::ZERO);
        assert_eq!(scenario.nodes().len(), 80);
        assert_eq!(scenario.subdivisions(), Some(1));
        assert_eq!(scenario.radius(), Some(300.0));

        assert!(scenario.adjust_subdivisions(1));
        assert_eq!(scenario.nodes().len(), 320);
        assert!(scenario.scale_radius(2.0));
        assert_eq!(scenario.radius(), Some(600.0));

        // Hemisphere split moves north faces right and restores on toggle.
        assert_eq!(scenario.hemisphere_split(), Some(false));
        let before: Vec<f32> = scenario
            .nodes()
            .iter()
            .map(|n| n.borrow().center.x)
            .collect();
        assert!(scenario.toggle_hemisphere_split());
        let after: Vec<f32> = scenario
            .nodes()
            .iter()
            .map(|n| n.borrow().center.x)
            .collect();
        let gap = 1.2 * 600.0;
        for (face, (&x0, &x1)) in scenario.nodes().iter().zip(before.iter().zip(&after)) {
            let expected = if face.borrow().center.z >= 0.0 {
                x0 + gap
            } else {
                x0 - gap
            };
            assert!((x1 - expected).abs() < 1e-3, "face moved incorrectly");
        }
        assert!(scenario.toggle_hemisphere_split());
        let restored: Vec<f32> = scenario
            .nodes()
            .iter()
            .map(|n| n.borrow().center.x)
            .collect();
        for (&x0, &x2) in before.iter().zip(&restored) {
            assert!((x2 - x0).abs() < 1e-3, "face not restored");
        }

        // Static scenarios ignore adjustments.
        let mut static_scenario = Scenario::Static(vec![]);
        assert!(!static_scenario.adjust_subdivisions(1));
        assert!(!static_scenario.scale_radius(2.0));
        assert!(!static_scenario.toggle_hemisphere_split());
    }

    #[test]
    fn scenario_split_and_unsplit() {
        // Static scenario: split quadruples the mesh, unsplit merges it back.
        let height = 300.0 * 3.0_f32.sqrt() / 2.0;
        let node = Node::new(
            "root",
            [
                Vec3::new(0.0, 2.0 * height / 3.0, 0.0),
                Vec3::new(150.0, -height / 3.0, 0.0),
                Vec3::new(-150.0, -height / 3.0, 0.0),
            ],
            Vec3::new(0.0, 1000.0, 0.0),
        );
        let mut scenario = Scenario::Static(vec![node]);
        assert!(scenario.split());
        assert_eq!(scenario.nodes().len(), 4);
        assert!(scenario.split());
        assert_eq!(scenario.nodes().len(), 16);
        assert!(scenario.unsplit());
        assert_eq!(scenario.nodes().len(), 4);
        assert!(scenario.unsplit());
        assert_eq!(scenario.nodes().len(), 1);
        // Nothing left to merge.
        assert!(!scenario.unsplit());

        // Empty static scenarios ignore both.
        let mut empty = Scenario::Static(vec![]);
        assert!(!empty.split());
        assert!(!empty.unsplit());

        // Icosphere scenarios map split/unsplit to subdivision adjustments.
        let mut icosphere = Scenario::icosphere("planet", 300.0, 1, Vec3::ZERO);
        assert!(icosphere.split());
        assert_eq!(icosphere.subdivisions(), Some(2));
        assert_eq!(icosphere.nodes().len(), 320);
        assert!(icosphere.unsplit());
        assert_eq!(icosphere.subdivisions(), Some(1));
        assert_eq!(icosphere.nodes().len(), 80);
    }
}
