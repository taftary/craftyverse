//! Shared test fixtures: 2D point construction and the standard equilateral
//! test node used by the `node` and `scene` test suites.

use glam::Vec3;

use planet_crafter_engine::node::{Node, NodeRef};
use planet_crafter_engine::runtime::PlanetConfig;

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

/// Standard test planet: radius 1000 at the origin, atmosphere shell at
/// 1250, orbit edge at 2500, sky layer up to altitude 100.
pub fn test_planet_config() -> PlanetConfig {
    PlanetConfig {
        planet_radius: 1000.0,
        planet_origin: Vec3::ZERO,
        atmosphere_multiplier: 1.25,
        orbit_multiplier: 2.0,
        sky_altitude: 100.0,
    }
}
