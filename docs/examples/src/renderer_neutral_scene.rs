//! Renderer-neutral scene data.
//!
//! Demonstrates that the `scene` module produces GPU-independent vertex data
//! from a collection of nodes, without opening a window.
//!
//! See `docs/book/specs/scene.md` and `docs/book/specs/render.md`.

use glam::Vec3;
use planet_crafter_engine::node::Node;
use planet_crafter_engine::scene::{ViewMode, build_scene};

/// Runs the renderer-neutral-scene demonstration.
///
/// A single node is turned into lines, triangles, world-anchored labels, and
/// UI panel rows without touching any Vulkan object.
pub fn run() {
    let node = Node::new(
        "root",
        [
            Vec3::new(0.0, 2.0 / 3.0, 0.0),
            Vec3::new(0.5, -1.0 / 3.0, 0.0),
            Vec3::new(-0.5, -1.0 / 3.0, 0.0),
        ],
        Vec3::ZERO,
    );

    let scene = build_scene(&[node], &Default::default(), ViewMode::Mesh);
    assert!(!scene.lines.is_empty());
    assert!(!scene.triangles.is_empty());
    assert!(!scene.labels.is_empty());
    assert!(!scene.texts.is_empty());
    assert!(!scene.panel_rows.is_empty());
    assert!(scene.fit_radius > 0.0);
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn renderer_neutral_scene_demo_runs() {
        run();
    }
}
