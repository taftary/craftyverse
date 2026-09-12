//! Pure geometric helpers for node construction: triangle vertices, edge
//! midpoints, child triangle points, and the `[i, j, k]` direction triplet.

use std::cell::RefCell;
use std::ops::{Add, Div};
use std::rc::Rc;

use glam::{Vec2, Vec3};

use super::{Node, NodeRef, Parity};

/// Midpoint between two points. Generic over the vector type so vertex and
/// UV midpoints share one implementation.
pub(crate) fn midpoint<T>(a: T, b: T) -> T
where
    T: Copy + Add<Output = T> + Div<f32, Output = T>,
{
    (a + b) / 2.0
}

/// Vertex triplets of the four children produced by a split, given the
/// parent's `vertices` triplet `[A, B, C]` and its edge midpoints
/// `[pAB, pBC, pCA]`.
///
/// Returns `[I, J, K, Center]` triplets: `I` holds corner `A`, `J` holds `B`,
/// `K` holds `C`, and the center triplet is built from the midpoints alone.
/// Generic over the point type: the same barycentric split applies to the
/// 3D vertices and to their UV coordinates.
pub(crate) fn triangle_points<T: Copy>(points: &[T; 3], midpoints: [T; 3]) -> [[T; 3]; 4] {
    let [p_a, p_b, p_c] = *points;
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
#[allow(clippy::too_many_arguments)]
pub(crate) fn child_node(
    vertices: [Vec3; 3],
    uv: [Vec2; 3],
    ring: [f32; 3],
    origin: Vec3,
    level: u32,
    name: String,
    parity: Parity,
) -> NodeRef {
    Rc::new(RefCell::new(Node::from_vertices(
        vertices, uv, ring, origin, level, name, parity,
    )))
}
