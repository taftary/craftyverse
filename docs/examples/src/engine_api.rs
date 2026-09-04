//! Engine API consumed by game code.
//!
//! Demonstrates the crate-boundary rule that the game layer consumes engine
//! contracts without reaching into backend details.
//!
//! See `docs/book/architecture/crate-boundaries.md`.

use glam::Vec2;
use planet_crafter_engine::node::{Labeling, Node};
use planet_crafter_engine::plan::Plan;

/// Runs the engine-API demonstration.
///
/// A game-level function builds a plan and a standalone node using only the
/// public surface of the engine crate.
pub fn run() {
    let mut plan = Plan::default();
    let root = plan.generate(100.0);
    assert!(plan.root_node.is_some());
    assert_eq!(root.borrow().name, "north_base_node_0");

    let node = Node::new(
        Vec2::Y,
        Vec2::ZERO,
        Vec2::ZERO,
        2.0,
        1.0,
        "demo",
        Labeling::Normal,
    );
    assert_eq!(node.borrow().level, 0);
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn engine_api_demo_runs() {
        run();
    }
}
