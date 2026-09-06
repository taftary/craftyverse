//! The `Node` module: triangle geometry, directional vectors, and
//! bidirectional links to adjacent nodes.
//!
//! A [`Node`] represents one non-degenerate triangle in a hierarchical mesh. It stores
//! its corner vertices, center, directional vectors, and up to three neighbors in
//! the `children` array. Nodes are reference-counted and mutable via
//! [`NodeRef`] so that bidirectional links can be shared.
//!
//! Pure geometric helpers live in `geometry`; child-link wiring and graph
//! traversal live in `topology`; triangle subdivision (single-node and
//! whole-mesh) lives in `subdivision`. The full contract is specified in
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
mod icosphere;
mod subdivision;
mod topology;

use std::cell::RefCell;
use std::rc::Rc;

use glam::Vec3;

use geometry::{child_node, compute_directions};

pub use icosphere::{IcosphereMesh, MAX_SUBDIVISIONS, build_icosphere};
pub use subdivision::{split_node, split_nodes, unsplit_nodes};
pub use topology::{collect_nodes, destroy_mesh};

#[cfg(feature = "test-internals")]
pub use topology::{link, reciprocal_index};

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
    // --- Geometry ---
    /// Triangle corner vertices `[A, B, C]`, where `A` is the apex and `BC`
    /// is the reference base edge.
    pub vertices: [Vec3; 3],
    /// Centroid of the node's triangle, computed from the corner vertices.
    pub center: Vec3,
    /// Vector from the node center toward the origin used to construct the
    /// node. This is `origin - center` and is recomputed for each child during
    /// [`split_node`].
    pub direction_to_origin: Vec3,
    /// Directional vectors `[i, j, k]`. Each vector points from the center
    /// toward the midpoint of one edge of the triangle:
    /// - `i` points toward the midpoint of edge AB,
    /// - `j` points toward the midpoint of edge BC,
    /// - `k` points toward the midpoint of edge CA.
    pub directions: [Vec3; 3],
    /// Normalized altitude direction from the base edge `BC` toward `A`.
    pub direction_of_node: Vec3,

    // --- Topology ---
    /// Bidirectional links to adjacent nodes, indexed `[node_i, node_j, node_k]`.
    ///
    /// - `children[0]` is the link in direction `i` (toward the midpoint of edge AB).
    /// - `children[1]` is the link in direction `j` (toward the midpoint of edge BC).
    /// - `children[2]` is the link in direction `k` (toward the midpoint of edge CA).
    ///
    /// Links are bidirectional with an explicitly recorded back-port: if
    /// `A.children[x] == B`, then `A.back_ports[x] == Some(y)` and
    /// `B.children[y] == A` (and `B.back_ports[y] == Some(x)`). Links
    /// created by [`split_node`] and by the icosphere welds also follow the
    /// reciprocal port pattern `y == 2 - x`; the back-port is stored rather
    /// than assumed so wiring and cleanup stay exact for any link.
    pub children: [Option<NodeRef>; 3],
    /// Port of the back-link on the neighbor: `back_ports[x]` is the slot of
    /// `children[x]`'s neighbor that points back to this node. Recorded by
    /// the link wiring; `None` exactly where `children[x]` is `None`.
    pub back_ports: [Option<usize>; 3],

    // --- Identity ---
    /// Unique identifier across all nodes. The caller is responsible for
    /// keeping names unique; duplicate names do not affect geometry but make
    /// debugging and logging ambiguous.
    pub name: String,
    /// Split depth. `0` for a root node; incremented by one for every
    /// generation produced by [`split_node`]. There is no upper bound.
    pub level: u32,
}

impl Node {
    /// Creates a level-zero node from an explicit non-degenerate triangle.
    ///
    /// # Parameters
    ///
    /// - `name` — unique identifier for the node. Names ending in `.I`,
    ///   `.J`, `.K`, or `.C` are reserved: [`split_node`] derives them for
    ///   its children and [`unsplit_nodes`] groups nodes by them.
    /// - `vertices` — triangle corners `[A, B, C]`, where `A` is the apex and
    ///   `BC` is the base.
    /// - `origin` — position used to compute `direction_to_origin` for the node
    ///   and its descendants.
    ///
    /// The caller must provide a non-degenerate triangle. The center, altitude,
    /// and edge directions are derived from `vertices`.
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
    /// let [a, b, c] = node.vertices;
    ///
    /// // Direction is normalized and the centroid matches the request.
    /// assert!((node.direction_of_node.length() - 1.0).abs() < 1e-4);
    /// assert!(((a + b + c) / 3.0 - node.center).length() < 1e-4);
    ///
    /// // I/J/K directions point through the edge midpoint and are normalized.
    /// for (dir, start, end) in [(node.directions[0], a, b), (node.directions[1], b, c), (node.directions[2], c, a)] {
    ///     let mid = (start + end) / 2.0;
    ///     let toward_mid = (mid - node.center).normalize();
    ///     assert!((dir - toward_mid).length() < 1e-4);
    ///     assert!(dir.dot(mid - node.center) > 0.0);
    ///     assert!((dir.length() - 1.0).abs() < 1e-4);
    /// }
    /// ```
    pub fn new(name: impl Into<String>, vertices: [Vec3; 3], origin: Vec3) -> NodeRef {
        child_node(vertices, origin, 0, name.into())
    }

    /// Builds a node from an explicit vertices triplet.
    ///
    /// The center is the centroid of the triplet, `direction_to_origin` and the
    /// `[i, j, k]` directions are derived from the vertices. This constructor is
    /// used internally by [`Node::new`] and [`split_node`].
    fn from_vertices(vertices: [Vec3; 3], origin: Vec3, level: u32, name: String) -> Self {
        let center = (vertices[0] + vertices[1] + vertices[2]) / 3.0;
        let base_direction = (vertices[2] - vertices[1]).normalize();
        let base_projection =
            vertices[1] + base_direction * (vertices[0] - vertices[1]).dot(base_direction);
        let height_vector = vertices[0] - base_projection;
        Node {
            vertices,
            center,
            direction_to_origin: origin - center,
            directions: compute_directions(&vertices, center),
            direction_of_node: height_vector.normalize(),
            children: [None, None, None],
            back_ports: [None, None, None],
            name,
            level,
        }
    }

    /// Severs all bidirectional `children` links.
    ///
    /// For each occupied port, the local port and back-port record are
    /// taken first, then the neighbor's recorded back-port slot is cleared.
    /// Because the back-port is stored explicitly by the link
    /// wiring, `destroy` is exact for any link, without assuming the
    /// `0 <-> 2`, `1 <-> 1` pattern. The node itself is freed automatically
    /// once its last `Rc` reference is dropped; there is no explicit
    /// self-destruction in Rust.
    ///
    /// # Panics
    ///
    /// Panics when an occupied port has no recorded back-port. Links wired
    /// through the topology helpers always record one, so a panic means the
    /// `children` array was modified by hand.
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
        for index in 0..3 {
            if let Some(child) = self.children[index].take() {
                let back = self.back_ports[index]
                    .take()
                    .expect("link without a recorded back-port");
                let mut child = child.borrow_mut();
                child.children[back] = None;
                child.back_ports[back] = None;
            }
        }
    }
}
