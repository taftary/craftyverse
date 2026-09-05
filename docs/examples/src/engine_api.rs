//! Engine API consumed by game code.
//!
//! Demonstrates the crate-boundary rule that the game layer consumes engine
//! contracts without reaching into backend details.
//!
//! See `docs/book/architecture/crate-boundaries.md`.

use glam::Vec2;
use planet_crafter_engine::node::{Labeling, Node};

/// Runs the engine-API demonstration.
///
/// A game-level function builds a standalone node using only the public
/// surface of the engine crate.
pub fn run() {
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
