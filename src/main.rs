mod node;
mod plan;
mod render;
mod scene;
mod text;

use glam::Vec2;
use node::{Labeling, Node};
use plan::Plan;

fn main() {
    let origin = Vec2::new(0.0, 1000.0);
    // Equilateral triangles pointing up: height = base * √3 / 2.
    let height = 300.0 * 3.0_f32.sqrt() / 2.0;

    // 1. One node, without split.
    let no_split = vec![Node::new(Vec2::Y, Vec2::ZERO, origin, 300.0, height, "root", Labeling::Normal)];

    // 2. One node with split — display only the split nodes.
    let parent = Node::new(Vec2::Y, Vec2::ZERO, origin, 300.0, height, "parent", Labeling::Normal);
    let center = parent.borrow().split();
    let mut split = vec![center.clone()];
    for corner in center.borrow().children.iter().flatten() {
        split.push(corner.clone());
    }

    // 3. One pentagonal base: 5 inward base nodes + 5 outward reverted nodes.
    let mut base_plan = Plan::new();
    let base_root = base_plan.generate_base("base_", 300.0, Vec2::Y, Vec2::ZERO, Labeling::Normal);
    let base = plan::collect_nodes(&base_root);

    // 4. Full dual-pentagon interlocked mesh (North + South, 20 nodes).
    let mut plan = Plan::new();
    let root = plan.generate(300.0);
    let dual_mesh = plan::collect_nodes(&root);

    // Opens the Vulkan viewer window; keys 1..4 select the scene.
    render::run(vec![no_split, split, base, dual_mesh]);
}
