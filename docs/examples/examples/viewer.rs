//! GPU/platform-specific viewer example.
//!
//! This example opens a Vulkan window and is gated behind the `gpu` feature. It
//! is intentionally excluded from headless CI runs.
//!
//! See `docs/book/practices/testing.md`.

use glam::Vec2;
use planet_crafter_engine::node::Node;
use planet_crafter_engine::render::{Scenario, run};

fn main() {
    let node = Node::new(
        "root",
        [
            Vec2::new(0.0, 2.0 / 3.0),
            Vec2::new(0.5, -1.0 / 3.0),
            Vec2::new(-0.5, -1.0 / 3.0),
        ],
        Vec2::ZERO,
    );
    run(vec![Scenario::Static(vec![node])]);
}
