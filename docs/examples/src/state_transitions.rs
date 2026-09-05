//! State transitions.
//!
//! Demonstrates a node-splitting state transition.

use glam::Vec3;
use planet_crafter_engine::node::Node;

/// Runs the state-transition demonstration.
///
/// A fresh node is split once and the resulting local topology is checked.
pub fn run() {
    let height = 100.0 * 3.0_f32.sqrt() / 2.0;
    let node = Node::new(
        "root",
        [
            Vec3::new(0.0, 2.0 * height / 3.0, 0.0),
            Vec3::new(50.0, -height / 3.0, 0.0),
            Vec3::new(-50.0, -height / 3.0, 0.0),
        ],
        Vec3::ZERO,
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
