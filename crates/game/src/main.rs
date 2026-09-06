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
