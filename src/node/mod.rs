//! The `Node` module: isosceles triangle geometry, directional vectors, and
//! bidirectional links to adjacent nodes.
//!
//! A [`Node`] represents one isosceles triangle in a hierarchical mesh. It stores
//! its corner points, center, directional vectors, and up to three neighbors in
//! the `children` array. Nodes are reference-counted and mutable via
//! [`NodeRef`] so that bidirectional links can be shared.
//!
//! Pure geometric helpers live in `geometry`; child-link wiring and graph
//! traversal live in `topology`. The full contract is specified in
//! `docs/rust/book/specs/node.md`.
//!
//! # Example
//!
//! ```
//! use glam::Vec2;
//! use crate::node::{Labeling, Node};
//!
//! let node = Node::new(
//!     Vec2::Y,
//!     Vec2::ZERO,
//!     Vec2::ZERO,
//!     2.0,
//!     1.0,
//!     "root",
//!     Labeling::Normal,
//! );
//! let center = node.borrow().center;
//! assert_eq!(center, Vec2::ZERO);
//! ```

mod geometry;
pub(crate) mod topology;

#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::rc::Rc;

use glam::Vec2;

use geometry::{child_node, compute_directions, midpoint, triangle_points};
use topology::{link, reciprocal_index};

pub use topology::collect_nodes;

/// Shared, mutable reference to a [`Node`].
///
/// `NodeRef` is an `Rc<RefCell<Node>>`: multiple parts of the mesh can hold the
/// same node, and the mutable borrow is deferred to runtime. This is the
/// ownership model used for the bidirectional `children` links.
pub type NodeRef = Rc<RefCell<Node>>;

/// Corner labeling convention for a node's triangle.
///
/// - `Normal` keeps the default B/C corner assignment.
/// - `Mirrored` swaps the B/C corner assignment, which also swaps the I and K
///   direction vectors.
///
/// The labeling is a construction-time choice only. After construction the
/// labeling is implicit in the stored [`points`](Node::points) triplet and
/// propagates automatically through [`Node::split`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Labeling {
    Normal,
    Mirrored,
}

impl Labeling {
    /// Returns the opposite labeling.
    ///
    /// Used to mirror paired nodes so that both nodes of a pair label the same
    /// pentagon vertex with the same letter.
    pub fn opposite(self) -> Self {
        match self {
            Labeling::Normal => Labeling::Mirrored,
            Labeling::Mirrored => Labeling::Normal,
        }
    }
}

/// A geometric node: an isosceles triangle with directional vectors and
/// bidirectional links to adjacent nodes.
///
/// See `docs/rust/book/specs/node.md` for the full specification of the
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
    pub center: Vec2,
    /// Vector from the node center toward the origin used to construct the
    /// node. This is `origin - center` and is recomputed for each child during
    /// [`Node::split`].
    pub direction_to_origin: Vec2,
    /// Directional vectors `[i, j, k]`. Each vector is perpendicular to one
    /// edge of the triangle and points from the center toward that edge:
    /// - `i` is perpendicular to edge AB,
    /// - `j` is perpendicular to edge BC,
    /// - `k` is perpendicular to edge CA.
    pub directions: [Vec2; 3],
    /// Triangle corner points `[A, B, C]`. `A` is the apex and `BC` is the
    /// base. The base is perpendicular to `direction_of_node`.
    pub points: [Vec2; 3],
    /// Normalized orientation vector of the isosceles triangle, pointing from
    /// the base `BC` toward the apex `A`.
    pub direction_of_node: Vec2,
    /// Length of the base edge `BC`.
    pub base_length: f32,
    /// Perpendicular distance from the base `BC` to the apex `A`.
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
    /// Creates a new node and initializes its geometry.
    ///
    /// # Parameters
    ///
    /// - `direction_of_node` — direction pointing from base `BC` toward apex
    ///   `A`. Stored normalized; the base is constructed perpendicular to it.
    /// - `center` — centroid of the node's triangle.
    /// - `origin` — position used to compute `direction_to_origin` for the node
    ///   and its descendants.
    /// - `base_length` — length of the base edge `BC`.
    /// - `height` — perpendicular distance from base `BC` to apex `A`.
    /// - `name` — unique identifier for the node.
    /// - `labeling` — corner labeling convention. `Mirrored` swaps the B/C
    ///   assignment and therefore the I/K direction vectors.
    ///
    /// # Example
    ///
    /// ```
    /// use glam::Vec2;
    /// use crate::node::{Labeling, Node};
    ///
    /// let node = Node::new(Vec2::Y, Vec2::ZERO, Vec2::ZERO, 2.0, 1.0, "root", Labeling::Normal);
    /// assert_eq!(node.borrow().level, 0);
    /// ```
    pub fn new(
        direction_of_node: Vec2,
        center: Vec2,
        origin: Vec2,
        base_length: f32,
        height: f32,
        name: impl Into<String>,
        labeling: Labeling,
    ) -> NodeRef {
        let direction = direction_of_node.normalize();
        let points = triangle_points(direction, center, base_length, height, labeling);
        Rc::new(RefCell::new(Self::from_points(
            direction,
            points,
            origin,
            base_length,
            height,
            0,
            name.into(),
        )))
    }

    /// Builds a node from an explicit points triplet.
    ///
    /// The center is the centroid of the triplet, `direction_to_origin` and the
    /// `[i, j, k]` directions are derived from the points. This constructor is
    /// used internally by [`Node::new`] and [`Node::split`].
    fn from_points(
        direction_of_node: Vec2,
        points: [Vec2; 3],
        origin: Vec2,
        base_length: f32,
        height: f32,
        level: u32,
        name: String,
    ) -> Self {
        let center = (points[0] + points[1] + points[2]) / 3.0;
        Node {
            name,
            level,
            center,
            direction_to_origin: origin - center,
            directions: compute_directions(&points, center),
            points,
            direction_of_node,
            base_length,
            height,
            children: [None, None, None],
        }
    }

    /// Splits the node into four new nodes and returns the center node.
    ///
    /// The four new nodes are:
    /// - `NodeI`, `NodeJ`, `NodeK` — corner nodes that keep the parent's
    ///   `direction_of_node`.
    /// - `NodeCenter` — the inverted middle node with
    ///   `direction_of_node = -parent.direction_of_node`.
    ///
    /// The center node is internally connected to each corner node through
    /// reciprocal `children` links. The caller is responsible for wiring the
    /// corner nodes to neighboring split centers across the subdivided edges.
    ///
    /// Each new node receives half the parent's [`base_length`](Node::base_length)
    /// and [`height`](Node::height), and its [`level`](Node::level) is set to
    /// `parent.level + 1`.
    ///
    /// See `docs/rust/book/specs/node.md` for the full geometric construction
    /// and topology rules.
    ///
    /// # Example
    ///
    /// ```
    /// use glam::Vec2;
    /// use crate::node::{Labeling, Node};
    ///
    /// let node = Node::new(Vec2::Y, Vec2::ZERO, Vec2::ZERO, 2.0, 1.0, "root", Labeling::Normal);
    /// let center = node.borrow().split();
    /// assert_eq!(center.borrow().level, 1);
    /// assert!(center.borrow().children[0].is_some());
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
        // the centroids). Corner nodes keep the parent's `direction_of_node`
        // (their apex is a parent corner, so the geometry matches); the center
        // node is inverted relative to the parent triangle, so its direction
        // is flipped. Each new node gets half the parent's base length and
        // height. Names derive from the parent name to stay unique.
        let level = old_level + 1;
        let base_length = self.base_length / 2.0;
        let height = self.height / 2.0;
        let node_i = child_node(
            [p_a, p_ab, p_ca],
            self.direction_of_node,
            origin,
            base_length,
            height,
            level,
            format!("{}.I", self.name),
        );
        let node_j = child_node(
            [p_ab, p_b, p_bc],
            self.direction_of_node,
            origin,
            base_length,
            height,
            level,
            format!("{}.J", self.name),
        );
        let node_k = child_node(
            [p_ca, p_bc, p_c],
            self.direction_of_node,
            origin,
            base_length,
            height,
            level,
            format!("{}.K", self.name),
        );
        let node_center = child_node(
            [p_bc, p_ab, p_ca],
            -self.direction_of_node,
            origin,
            base_length,
            height,
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
    /// use glam::Vec2;
    /// use crate::node::{Labeling, Node};
    ///
    /// let node = Node::new(Vec2::Y, Vec2::ZERO, Vec2::ZERO, 2.0, 1.0, "root", Labeling::Normal);
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
