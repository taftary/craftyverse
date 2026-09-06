//! Pure geometric helpers for node construction: triangle vertices, edge
//! midpoints, child triangle points, and the `[i, j, k]` direction triplet.

use std::cell::RefCell;
use std::rc::Rc;

use glam::Vec3;

use super::{Node, NodeRef};

/// Midpoint between two points.
pub(crate) fn midpoint(a: Vec3, b: Vec3) -> Vec3 {
    (a + b) / 2.0
}

/// Vertex triplets of the four children produced by a split, given the
/// parent's `vertices` triplet `[A, B, C]` and its edge midpoints
/// `[pAB, pBC, pCA]`.
///
/// Returns `[I, J, K, Center]` triplets: `I` holds corner `A`, `J` holds `B`,
/// `K` holds `C`, and the center triplet is built from the midpoints alone.
pub(crate) fn triangle_points(vertices: &[Vec3; 3], midpoints: [Vec3; 3]) -> [[Vec3; 3]; 4] {
    let [p_a, p_b, p_c] = *vertices;
    let [p_ab, p_bc, p_ca] = midpoints;
    [
        [p_a, p_ab, p_ca],
        [p_ab, p_b, p_bc],
        [p_ca, p_bc, p_c],
        [p_bc, p_ab, p_ca],
    ]
}

/// Unit direction from `center` toward the midpoint of edge `ab`.
pub(crate) fn direction_toward_edge_midpoint(a: Vec3, b: Vec3, center: Vec3) -> Vec3 {
    (midpoint(a, b) - center).normalize()
}

/// Computes the `[i, j, k]` direction triplet from the node's own vertices
/// triplet: I toward the midpoint of AB, J toward the midpoint of BC, K toward
/// the midpoint of CA, each pointing from the center toward its edge.
pub(crate) fn compute_directions(vertices: &[Vec3; 3], center: Vec3) -> [Vec3; 3] {
    let [a, b, c] = *vertices;
    [
        direction_toward_edge_midpoint(a, b, center),
        direction_toward_edge_midpoint(b, c, center),
        direction_toward_edge_midpoint(c, a, center),
    ]
}

/// Creates a node from an explicit vertices triplet, wrapped in a `NodeRef`.
pub(crate) fn child_node(vertices: [Vec3; 3], origin: Vec3, level: u32, name: String) -> NodeRef {
    Rc::new(RefCell::new(Node::from_vertices(
        vertices, origin, level, name,
    )))
}
