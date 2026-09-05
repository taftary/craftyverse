//! The `Node` module: triangle geometry, directional vectors, and
//! bidirectional links to adjacent nodes.
//!
//! A [`Node`] represents one non-degenerate triangle in a hierarchical mesh. It stores
//! its corner points, center, directional vectors, and up to three neighbors in
//! the `children` array. Nodes are reference-counted and mutable via
//! [`NodeRef`] so that bidirectional links can be shared.
//!
//! Pure geometric helpers live in `geometry`; child-link wiring and graph
//! traversal live in `topology`; triangle subdivision lives in `subdivision`.
//! The full contract is specified in `docs/book/specs/node.md`.
//!
//! # Example
//!
//! ```
//! use glam::Vec3;
//! use planet_crafter_engine::node::Node;
//!
//! let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
//! let center = node.borrow().center;
//! assert_eq!(center, Vec3::ZERO);
//! ```

mod geometry;
mod subdivision;
pub(crate) mod topology;

#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::rc::Rc;

use glam::Vec3;

use geometry::compute_directions;
use topology::reciprocal_index;

pub use subdivision::split_node;
pub use topology::collect_nodes;

/// Shared, mutable reference to a [`Node`].
///
/// `NodeRef` is an `Rc<RefCell<Node>>`: multiple parts of the mesh can hold the
/// same node, and the mutable borrow is deferred to runtime. This is the
/// ownership model used for the bidirectional `children` links.
pub type NodeRef = Rc<RefCell<Node>>;

/// A geometric node: a triangle with directional vectors and
/// bidirectional links to adjacent nodes.
///
/// See `docs/book/specs/node.md` for the full specification of the
/// geometry, topology, and identity rules.
pub struct Node {
    // --- Identity ---
    /// Unique identifier across all nodes. The caller is responsible for
    /// keeping names unique; duplicate names do not affect geometry but make
    /// debugging and logging ambiguous.
    pub name: String,
    /// Split depth. `0` for a root node; incremented by one for every
    /// generation produced by [`split_node`]. There is no upper bound.
    pub level: u32,

    // --- Geometry ---
    /// Centroid of the node's triangle, computed from the corner points.
    pub center: Vec3,
    /// Vector from the node center toward the origin used to construct the
    /// node. This is `origin - center` and is recomputed for each child during
    /// [`split_node`].
    pub direction_to_origin: Vec3,
    /// Directional vectors `[i, j, k]`. Each vector is perpendicular to one
    /// edge of the triangle and points from the center toward that edge:
    /// - `i` is perpendicular to edge AB,
    /// - `j` is perpendicular to edge BC,
    /// - `k` is perpendicular to edge CA.
    pub directions: [Vec3; 3],
    /// Triangle corner points `[A, B, C]`. `BC` is the reference base edge.
    pub points: [Vec3; 3],
    /// Normalized altitude direction from the base edge `BC` toward `A`.
    pub direction_of_node: Vec3,
    /// Length of the base edge `BC`.
    pub base_length: f32,
    /// Perpendicular distance from the line `BC` to `A`.
    pub height: f32,

    // --- Topology ---
    /// Bidirectional links to adjacent nodes, indexed `[node_i, node_j, node_k]`.
    ///
    /// - `children[0]` is the link in direction `i` (perpendicular to edge AB).
    /// - `children[1]` is the link in direction `j` (perpendicular to edge BC).
    /// - `children[2]` is the link in direction `k` (perpendicular to edge CA).
    ///
    /// Links are reciprocal: if `A.children[x] == B`, then
    /// `B.children[reciprocal_index(x)] == A`.
    pub children: [Option<NodeRef>; 3],
}

impl Node {
    /// Creates a level-zero node from an explicit non-degenerate triangle.
    ///
    /// # Parameters
    ///
    /// - `name` — unique identifier for the node.
    /// - `points` — triangle corners `[A, B, C]`, where `A` is the apex and
    ///   `BC` is the base.
    /// - `origin` — position used to compute `direction_to_origin` for the node
    ///   and its descendants.
    ///
    /// The caller must provide a non-degenerate triangle. The center, altitude,
    /// dimensions, and edge directions are derived from `points`.
    ///
    /// # Example
    ///
    /// ```
    /// use glam::Vec3;
    /// use planet_crafter_engine::node::Node;
    ///
    /// let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
    /// assert_eq!(node.borrow().level, 0);
    /// ```
    ///
    /// # Geometry invariants
    ///
    /// ```
    /// use glam::Vec3;
    /// use planet_crafter_engine::node::Node;
    ///
    /// let node = Node::new("root", [Vec3::new(0.0, 8.0 / 3.0, 0.0), Vec3::new(3.0, -4.0 / 3.0, 0.0), Vec3::new(-3.0, -4.0 / 3.0, 0.0)], Vec3::ZERO);
    /// let node = node.borrow();
    /// let [a, b, c] = node.points;
    ///
    /// // Direction is normalized; centroid and dimensions match the request.
    /// assert!((node.direction_of_node.length() - 1.0).abs() < 1e-4);
    /// assert!(((a + b + c) / 3.0 - node.center).length() < 1e-4);
    /// assert!(((b - c).length() - node.base_length).abs() < 1e-4);
    /// assert!(((a - (b + c) / 2.0).length() - node.height).abs() < 1e-4);
    ///
    /// // I/J/K directions are perpendicular to their edge and point outward.
    /// for (dir, start, end) in [(node.directions[0], a, b), (node.directions[1], b, c), (node.directions[2], c, a)] {
    ///     assert!(dir.dot(end - start).abs() < 1e-4);
    ///     let mid = (start + end) / 2.0;
    ///     assert!(dir.dot(mid - node.center) > 0.0);
    ///     assert!((dir.length() - 1.0).abs() < 1e-4);
    /// }
    /// ```
    pub fn new(name: impl Into<String>, points: [Vec3; 3], origin: Vec3) -> NodeRef {
        Rc::new(RefCell::new(Self::from_points(
            points,
            origin,
            0,
            name.into(),
        )))
    }

    /// Builds a node from an explicit points triplet.
    ///
    /// The center is the centroid of the triplet, `direction_to_origin` and the
    /// `[i, j, k]` directions are derived from the points. This constructor is
    /// used internally by [`Node::new`] and [`split_node`].
    fn from_points(points: [Vec3; 3], origin: Vec3, level: u32, name: String) -> Self {
        let center = (points[0] + points[1] + points[2]) / 3.0;
        let base_direction = (points[2] - points[1]).normalize();
        let base_projection =
            points[1] + base_direction * (points[0] - points[1]).dot(base_direction);
        let height_vector = points[0] - base_projection;
        Node {
            name,
            level,
            center,
            direction_to_origin: origin - center,
            directions: compute_directions(&points, center),
            points,
            direction_of_node: height_vector.normalize(),
            base_length: (points[1] - points[2]).length(),
            height: height_vector.length(),
            children: [None, None, None],
        }
    }

    /// Severs all bidirectional `children` links.
    ///
    /// For each linked neighbor, the reciprocal back-link is cleared first,
    /// then the link on this node. The node itself is freed automatically once
    /// its last `Rc` reference is dropped; there is no explicit
    /// self-destruction in Rust.
    ///
    /// # Example
    ///
    /// ```
    /// use glam::Vec3;
    /// use planet_crafter_engine::node::{split_node, Node};
    ///
    /// let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
    /// let center = split_node(&node.borrow());
    /// center.borrow_mut().destroy();
    /// assert!(center.borrow().children.iter().all(|c| c.is_none()));
    /// ```
    pub fn destroy(&mut self) {
        // (link index on self, reciprocal back-link index on the neighbor),
        // mirroring the interconnections established by `split_node`.
        for index in 0..3 {
            if let Some(child) = self.children[index].take() {
                child.borrow_mut().children[reciprocal_index(index)] = None;
            }
        }
    }
}
