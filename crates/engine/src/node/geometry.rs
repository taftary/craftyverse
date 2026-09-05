//! Pure geometric helpers for node construction: triangle corner points, edge
//! midpoints and the `[i, j, k]` direction triplet.

use std::cell::RefCell;
use std::rc::Rc;

use glam::Vec2;

use super::{Node, NodeRef};

/// Midpoint between two points.
pub(crate) fn midpoint(a: Vec2, b: Vec2) -> Vec2 {
    (a + b) / 2.0
}

/// Unit perpendicular of an edge, pointing from `center` toward the edge.
pub(crate) fn perpendicular_toward(a: Vec2, b: Vec2, center: Vec2) -> Vec2 {
    let edge = b - a;
    let candidate = Vec2::new(-edge.y, edge.x);
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
pub(crate) fn compute_directions(points: &[Vec2; 3], center: Vec2) -> [Vec2; 3] {
    let [a, b, c] = *points;
    [
        perpendicular_toward(a, b, center),
        perpendicular_toward(b, c, center),
        perpendicular_toward(c, a, center),
    ]
}

/// Creates a node from an explicit points triplet, wrapped in a `NodeRef`.
pub(crate) fn child_node(points: [Vec2; 3], origin: Vec2, level: u32, name: String) -> NodeRef {
    Rc::new(RefCell::new(Node::from_points(points, origin, level, name)))
}
