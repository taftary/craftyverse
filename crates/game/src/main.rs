use glam::Vec2;
use planet_crafter_engine::node::Node;
use planet_crafter_engine::render::Scenario;

fn main() {
    let origin = Vec2::new(0.0, 1000.0);
    // Equilateral triangles pointing up: height = base * √3 / 2.
    let height = 300.0 * 3.0_f32.sqrt() / 2.0;
    let points = [
        Vec2::new(0.0, 2.0 * height / 3.0),
        Vec2::new(150.0, -height / 3.0),
        Vec2::new(-150.0, -height / 3.0),
    ];

    // 1. One node, without split.
    let no_split = Scenario::Static(vec![Node::new("root", points, origin)]);

    // 2. One node with split — display only the split nodes.
    let parent = Node::new("parent", points, origin);
    let center = parent.borrow().split();
    let mut split_nodes = vec![center.clone()];
    for corner in center.borrow().children.iter().flatten() {
        split_nodes.push(corner.clone());
    }
    let split = Scenario::Static(split_nodes);

    // Opens the Vulkan viewer window; number keys select the scene.
    planet_crafter_engine::render::run(vec![no_split, split]);
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
                Vec2::new(0.0, 2.0 * height / 3.0),
                Vec2::new(150.0, -height / 3.0),
                Vec2::new(-150.0, -height / 3.0),
            ],
            Vec2::new(0.0, 1000.0),
        );
        let no_split = Scenario::Static(vec![node.clone()]);
        assert_eq!(no_split.nodes().len(), 1);

        // Static scenario with a split node: center + 3 corners.
        let center = node.borrow().split();
        let mut split_nodes = vec![center.clone()];
        for corner in center.borrow().children.iter().flatten() {
            split_nodes.push(corner.clone());
        }
        let split = Scenario::Static(split_nodes);
        assert_eq!(split.nodes().len(), 4);
    }
}
