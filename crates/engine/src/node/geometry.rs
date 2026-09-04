//! Pure geometric helpers for node construction: triangle corner points, edge
//! midpoints and the `[i, j, k]` direction triplet.

use std::cell::RefCell;
use std::rc::Rc;

use glam::Vec2;

use super::{Labeling, Node, NodeRef};

/// Corner points `[A, B, C]` of the node's isosceles triangle: A is the apex,
/// BC the base. `direction_of_node` must be normalized.
///
/// Isosceles triangle whose centroid is `center`: base BC is
/// perpendicular to `direction_of_node` and apex A is aligned with it
/// at distance `height` from BC. The centroid sits at 1/3 of the height
/// from the base (2/3 from the apex). With `Labeling::Normal`, B is
/// placed right of the direction axis and C left (when the direction
/// points up); `Labeling::Mirrored` swaps the B/C assignment and
/// therefore the I/K directions.
pub(crate) fn triangle_points(
    direction_of_node: Vec2,
    center: Vec2,
    base_length: f32,
    height: f32,
    labeling: Labeling,
) -> [Vec2; 3] {
    let mut perpendicular = Vec2::new(-direction_of_node.y, direction_of_node.x);
    if labeling == Labeling::Mirrored {
        perpendicular = -perpendicular;
    }
    let apex = center + direction_of_node * (2.0 * height / 3.0);
    let base_mid = center - direction_of_node * (height / 3.0);
    [
        apex,
        base_mid - perpendicular * (base_length / 2.0),
        base_mid + perpendicular * (base_length / 2.0),
    ]
}

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
pub(crate) fn child_node(
    points: [Vec2; 3],
    direction_of_node: Vec2,
    origin: Vec2,
    base_length: f32,
    height: f32,
    level: u32,
    name: String,
) -> NodeRef {
    Rc::new(RefCell::new(Node::from_points(
        direction_of_node,
        points,
        origin,
        base_length,
        height,
        level,
        name,
    )))
}
