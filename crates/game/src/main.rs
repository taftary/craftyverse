use glam::Vec3;
use planet_crafter_engine::node::{Node, split_node, unfold_uvs};
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

    // 4. Flat 2 x 2 quad patch (8 triangles) unfolded into one continuous
    // UV space. Grid coordinates are exact f32 values, so shared corners
    // weld bit-exactly; each cell is split along the p00-p11 diagonal.
    let mut patch_nodes = Vec::new();
    for j in 0..2 {
        for i in 0..2 {
            let (x, y) = (150.0 * i as f32, 150.0 * j as f32);
            let p00 = Vec3::new(x, y, 0.0);
            let p10 = Vec3::new(x + 150.0, y, 0.0);
            let p01 = Vec3::new(x, y + 150.0, 0.0);
            let p11 = Vec3::new(x + 150.0, y + 150.0, 0.0);
            patch_nodes.push(Node::new(
                format!("patch.{i}.{j}.a"),
                [p00, p11, p10],
                origin,
            ));
            patch_nodes.push(Node::new(
                format!("patch.{i}.{j}.b"),
                [p00, p01, p11],
                origin,
            ));
        }
    }
    unfold_uvs(&patch_nodes);
    let patch = Scenario::Static(patch_nodes);

    // Opens the Vulkan viewer window; number keys select the scene.
    planet_crafter_engine::render::run(vec![no_split, split, icosphere, patch]);
}
