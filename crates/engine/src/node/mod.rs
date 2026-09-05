//! The `Node` module: triangle geometry, directional vectors, and
//! bidirectional links to adjacent nodes.
//!
//! A [`Node`] represents one non-degenerate triangle in a hierarchical mesh. It stores
//! its corner points, center, directional vectors, and up to three neighbors in
//! the `children` array. Nodes are reference-counted and mutable via
//! [`NodeRef`] so that bidirectional links can be shared.
//!
//! Pure geometric helpers live in `geometry`; child-link wiring and graph
//! traversal live in `topology`. The full contract is specified in
//! `docs/book/specs/node.md`.
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
pub(crate) mod topology;

#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::rc::Rc;

use glam::Vec3;

use geometry::{child_node, compute_directions, midpoint};
use topology::{link, reciprocal_index};

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
    /// generation produced by [`Node::split`]. There is no upper bound.
    pub level: u32,

    // --- Geometry ---
    /// Centroid of the node's triangle, computed from the corner points.
    pub center: Vec3,
    /// Vector from the node center toward the origin used to construct the
    /// node. This is `origin - center` and is recomputed for each child during
    /// [`Node::split`].
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
    /// used internally by [`Node::new`] and [`Node::split`].
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

    /// Splits the node into four new nodes and returns the center node.
    ///
    /// The four new nodes are `NodeI`, `NodeJ`, `NodeK`, and `NodeCenter`.
    ///
    /// The center node is internally connected to each corner node through
    /// reciprocal `children` links. The caller is responsible for wiring the
    /// corner nodes to neighboring split centers across the subdivided edges.
    ///
    /// Each new node derives its dimensions and orientation from its own point
    /// triplet, and its [`level`](Node::level) is set to `parent.level + 1`.
    ///
    /// See `docs/book/specs/node.md` for the full geometric construction
    /// and topology rules.
    ///
    /// # Example
    ///
    /// ```
    /// use glam::Vec3;
    /// use planet_crafter_engine::node::Node;
    ///
    /// let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
    /// let center = node.borrow().split();
    /// assert_eq!(center.borrow().level, 1);
    /// assert!(center.borrow().children[0].is_some());
    /// ```
    ///
    /// # Subdivision invariants
    ///
    /// ```
    /// use std::rc::Rc;
    /// use glam::Vec3;
    /// use planet_crafter_engine::node::Node;
    ///
    /// let node = Node::new("root", [Vec3::new(0.0, 4.0 / 3.0, 0.0), Vec3::new(1.0, -2.0 / 3.0, 0.0), Vec3::new(-1.0, -2.0 / 3.0, 0.0)], Vec3::ZERO);
    /// let center = node.borrow().split();
    /// let center_ref = center.borrow();
    ///
    /// assert_eq!(center_ref.level, 1);
    /// assert_eq!(center_ref.base_length, 1.0);
    /// assert_eq!(center_ref.height, 1.0);
    /// assert!(center_ref.children.iter().all(|c| c.is_some()));
    ///
    /// // Reciprocity: each corner links back to the center on the expected port.
    /// assert!(Rc::ptr_eq(
    ///     center_ref.children[0].as_ref().unwrap().borrow().children[2].as_ref().unwrap(),
    ///     &center,
    /// ));
    /// assert!(Rc::ptr_eq(
    ///     center_ref.children[1].as_ref().unwrap().borrow().children[1].as_ref().unwrap(),
    ///     &center,
    /// ));
    /// assert!(Rc::ptr_eq(
    ///     center_ref.children[2].as_ref().unwrap().borrow().children[0].as_ref().unwrap(),
    ///     &center,
    /// ));
    /// ```
    pub fn split(&self) -> NodeRef {
        let old_level = self.level;
        // The node stores no origin; recover it from center + direction_to_origin.
        let origin = self.center + self.direction_to_origin;
        let [p_a, p_b, p_c] = self.points;

        // 1. Point midpoints.
        let p_ab = midpoint(p_a, p_b);
        let p_bc = midpoint(p_b, p_c);
        let p_ca = midpoint(p_c, p_a);

        // 2./3. New nodes from their subdivided points triplets (centers are
        // the centroids). Each child derives its own altitude and dimensions
        // from its point triplet. Names derive from the parent name to stay
        // unique.
        let level = old_level + 1;
        let node_i = child_node([p_a, p_ab, p_ca], origin, level, format!("{}.I", self.name));
        let node_j = child_node([p_ab, p_b, p_bc], origin, level, format!("{}.J", self.name));
        let node_k = child_node([p_ca, p_bc, p_c], origin, level, format!("{}.K", self.name));
        let node_center = child_node(
            [p_bc, p_ab, p_ca],
            origin,
            level,
            format!("{}.C", self.name),
        );

        // 4. Internal interconnection (bidirectional). Each center port is
        // linked to the corner node across its edge: center I (⊥ pBC–pAB)
        // faces node J, center J (⊥ pAB–pCA) faces node I, center K
        // (⊥ pCA–pBC) faces node K; the reciprocal corner port faces the
        // center the same way.
        link(&node_center, 0, &node_j, 2);
        link(&node_center, 1, &node_i, 1);
        link(&node_center, 2, &node_k, 0);

        node_center
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
    /// use planet_crafter_engine::node::Node;
    ///
    /// let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
    /// let center = node.borrow().split();
    /// center.borrow_mut().destroy();
    /// assert!(center.borrow().children.iter().all(|c| c.is_none()));
    /// ```
    pub fn destroy(&mut self) {
        // (link index on self, reciprocal back-link index on the neighbor),
        // mirroring the interconnections established by `split()`.
        for index in 0..3 {
            if let Some(child) = self.children[index].take() {
                child.borrow_mut().children[reciprocal_index(index)] = None;
            }
        }
    }
}
