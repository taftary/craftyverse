//! GPU/platform-specific viewer example.
//!
//! This example opens a Vulkan window and is gated behind the `gpu` feature. It
//! is intentionally excluded from headless CI runs.
//!
//! See `docs/rust/book/practices/testing.md`.

use glam::Vec2;
use planet_crafter_engine::node::{Labeling, Node};
use planet_crafter_engine::render::{Scenario, run};

fn main() {
    let node = Node::new(
        Vec2::Y,
        Vec2::ZERO,
        Vec2::ZERO,
        2.0,
        1.0,
        "root",
        Labeling::Normal,
    );
    run(vec![Scenario::Static(vec![node])]);
}
