mod node;
mod plan;
mod render;
mod scene;
mod text;

use glam::Vec2;
use node::{Labeling, Node};
use plan::Plan;
use render::Scenario;

/// Pentagonal base scenario: 5 inward base nodes + 5 outward reverted nodes.
fn base_plan() -> Plan {
    let mut plan = Plan::new();
    plan.generate_base("base_", 300.0, Vec2::Y, Vec2::ZERO, Labeling::Normal);
    plan
}

/// Full dual-pentagon interlocked mesh scenario (North + South, 20 nodes).
fn dual_mesh_plan() -> Plan {
    let mut plan = Plan::new();
    plan.generate(300.0);
    plan
}

fn main() {
    let origin = Vec2::new(0.0, 1000.0);
    // Equilateral triangles pointing up: height = base * √3 / 2.
    let height = 300.0 * 3.0_f32.sqrt() / 2.0;

    // 1. One node, without split.
    let no_split = Scenario::Static(vec![Node::new(Vec2::Y, Vec2::ZERO, origin, 300.0, height, "root", Labeling::Normal)]);

    // 2. One node with split — display only the split nodes.
    let parent = Node::new(Vec2::Y, Vec2::ZERO, origin, 300.0, height, "parent", Labeling::Normal);
    let center = parent.borrow().split();
    let mut split_nodes = vec![center.clone()];
    for corner in center.borrow().children.iter().flatten() {
        split_nodes.push(corner.clone());
    }
    let split = Scenario::Static(split_nodes);

    // 3. One pentagonal base — live plan: S subdivides it, R regenerates it.
    let base = Scenario::Plan { plan: base_plan(), rebuild: base_plan };

    // 4. Full dual-pentagon interlocked mesh — live plan: S subdivides it,
    //    R regenerates it.
    let dual_mesh = Scenario::Plan { plan: dual_mesh_plan(), rebuild: dual_mesh_plan };

    // Opens the Vulkan viewer window; keys 1..4 select the scene, S splits
    // the current plan one level, R regenerates it.
    render::run(vec![no_split, split, base, dual_mesh]);
}
