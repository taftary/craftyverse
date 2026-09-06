//! Shared test fixtures: 2D point construction and the standard equilateral
//! test node used by the `node` and `scene` test suites.

use glam::Vec3;

use planet_crafter_engine::node::{Node, NodeRef};

/// 2D point on the z = 0 plane.
pub fn point(x: f32, y: f32) -> Vec3 {
    Vec3::new(x, y, 0.0)
}

/// Equilateral test node (base 300, apex up, height = base * sqrt(3) / 2).
pub fn test_node() -> NodeRef {
    Node::new(
        "root",
        [
            point(0.0, 100.0 * 3.0_f32.sqrt()),
            point(150.0, -50.0 * 3.0_f32.sqrt()),
            point(-150.0, -50.0 * 3.0_f32.sqrt()),
        ],
        point(0.0, 1000.0),
    )
}
