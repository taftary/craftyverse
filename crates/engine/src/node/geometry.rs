//! Pure geometric helpers for node construction: triangle points, edge
//! midpoints and the `[i, j, k]` direction triplet.

use std::cell::RefCell;
use std::rc::Rc;

use glam::Vec3;

use super::{Node, NodeRef};

/// Midpoint between two points.
pub(crate) fn midpoint(a: Vec3, b: Vec3) -> Vec3 {
    (a + b) / 2.0
}

/// Unit perpendicular of an edge, pointing from `center` toward the edge.
pub(crate) fn perpendicular_toward(a: Vec3, b: Vec3, center: Vec3) -> Vec3 {
    let edge = b - a;
    let normal = (b - a).cross(center - a).normalize();
    let candidate = edge.cross(normal);
    let toward_edge = midpoint(a, b) - center;
    let direction = if candidate.dot(toward_edge) >= 0.0 {
        candidate
    } else {
        -candidate
    };
    direction.normalize()
}

/// Computes the `[i, j, k]` direction triplet from the node's own points
/// triplet: I ⊥ AB, J ⊥ BC, K ⊥ CA, each pointing from the center toward its
/// edge.
pub(crate) fn compute_directions(points: &[Vec3; 3], center: Vec3) -> [Vec3; 3] {
    let [a, b, c] = *points;
    [
        perpendicular_toward(a, b, center),
        perpendicular_toward(b, c, center),
        perpendicular_toward(c, a, center),
    ]
}

/// Creates a node from an explicit points triplet, wrapped in a `NodeRef`.
pub(crate) fn child_node(points: [Vec3; 3], origin: Vec3, level: u32, name: String) -> NodeRef {
    Rc::new(RefCell::new(Node::from_points(points, origin, level, name)))
}
