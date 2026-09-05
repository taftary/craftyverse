//! Renderer-neutral scene data.
//!
//! Demonstrates that the `scene` module produces GPU-independent vertex data
//! from a collection of nodes, without opening a window.
//!
//! See `docs/book/specs/scene.md` and `docs/book/specs/render.md`.

use glam::Vec2;
use planet_crafter_engine::node::Node;
use planet_crafter_engine::scene::build_scene;

/// Runs the renderer-neutral-scene demonstration.
///
/// A single node is turned into lines, triangles, text labels, and UI
/// checkboxes without touching any Vulkan object.
pub fn run() {
    let node = Node::new(
        "root",
        [
            Vec2::new(0.0, 2.0 / 3.0),
            Vec2::new(0.5, -1.0 / 3.0),
            Vec2::new(-0.5, -1.0 / 3.0),
        ],
        Vec2::ZERO,
    );

    let scene = build_scene(&[node], Vec2::new(800.0, 600.0), &Default::default());
    assert!(!scene.lines.is_empty());
    assert!(!scene.triangles.is_empty());
    assert!(!scene.texts.is_empty());
    assert!(!scene.checkboxes.is_empty());
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn renderer_neutral_scene_demo_runs() {
        run();
    }
}
