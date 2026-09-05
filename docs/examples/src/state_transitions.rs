//! State transitions.
//!
//! Demonstrates a node-splitting state transition.

use glam::Vec2;
use planet_crafter_engine::node::{Labeling, Node};

/// Runs the state-transition demonstration.
///
/// A fresh node is split once and the resulting local topology is checked.
pub fn run() {
    let node = Node::new(
        Vec2::Y,
        Vec2::ZERO,
        Vec2::ZERO,
        100.0,
        100.0 * 3.0_f32.sqrt() / 2.0,
        "root",
        Labeling::Normal,
    );
    let center = node.borrow().split();
    assert_eq!(center.borrow().level, 1);
    assert_eq!(center.borrow().children.iter().flatten().count(), 3);
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn state_transition_demo_runs() {
        run();
    }
}
