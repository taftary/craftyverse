//! Engine API consumed by game code.
//!
//! Demonstrates the crate-boundary rule that the game layer consumes engine
//! contracts without reaching into backend details.
//!
//! See `docs/book/architecture/crate-boundaries.md`.

use glam::Vec2;
use planet_crafter_engine::node::Node;

/// Runs the engine-API demonstration.
///
/// A game-level function builds a standalone node using only the public
/// surface of the engine crate.
pub fn run() {
    let node = Node::new(
        "demo",
        [
            Vec2::new(0.0, 2.0 / 3.0),
            Vec2::new(0.5, -1.0 / 3.0),
            Vec2::new(-0.5, -1.0 / 3.0),
        ],
        Vec2::ZERO,
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
