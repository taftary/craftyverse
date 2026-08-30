mod node;
mod render;
mod scene;
mod text;

use glam::Vec2;
use node::{DirectionSet, Node};

fn main() {
    let origin = Vec2::new(0.0, 1000.0);

    // 1. One node, without split.
    let no_split = vec![Node::new(
        DirectionSet::Normal,
        Vec2::ZERO,
        origin,
        300.0,
        "root",
    )];

    // 2. One node with split — display only the split nodes.
    let parent = Node::new(DirectionSet::Normal, Vec2::ZERO, origin, 300.0, "parent");
    let center = parent.borrow().split();
    let mut split = vec![center.clone()];
    for corner in center.borrow().children.iter().flatten() {
        split.push(corner.clone());
    }

    // Opens the Vulkan viewer window; key 1 selects scene 1, key 2 scene 2.
    render::run(vec![no_split, split]);
}
