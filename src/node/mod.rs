//! The `Node` class: isosceles triangle geometry, directional vectors and
//! bidirectional links to adjacent nodes. Pure geometric helpers live in
//! `geometry`, child-link wiring and traversal in `topology`.
//! See `docs/classes-definitions/node.md`.

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

/// Shared, mutable link to a node. Used for the bidirectional `children` links.
pub type NodeRef = Rc<RefCell<Node>>;

/// Corner labeling convention for a node's triangle: `Normal` keeps the
/// default B/C assignment, `Mirrored` swaps it (and therefore the I/K
/// direction vectors). Construction-time only — afterwards the labeling is
/// implicit in the stored `points` triplet and propagates through `split()`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Labeling {
    Normal,
    Mirrored,
}

impl Labeling {
    /// The opposite labeling (used to mirror paired nodes).
    pub fn opposite(self) -> Self {
        match self {
            Labeling::Normal => Labeling::Mirrored,
            Labeling::Mirrored => Labeling::Normal,
        }
    }
}

/// A geometric node: isosceles triangle geometry, directional vectors and
/// bidirectional links to adjacent nodes. See `docs/classes-definitions/node.md`.
pub struct Node {
    // --- Identity ---
    /// Unique identifier across all nodes.
    pub name: String,
    /// Split depth; 0 for the root node, no upper bound.
    pub level: u32,

    // --- Geometry ---
    /// Center of the node.
    pub center: Vec2,
    /// Vector from the node center toward the origin.
    pub direction_to_origin: Vec2,
    /// Directional vectors `[i, j, k]`.
    pub directions: [Vec2; 3],
    /// Triangle corner points `[A, B, C]` — A is the apex, BC the base.
    pub points: [Vec2; 3],
    /// Orientation of the isosceles triangle: normalized vector pointing from
    /// base BC toward apex A, perpendicular to BC.
    pub direction_of_node: Vec2,
    /// Length of the base edge BC.
    pub base_length: f32,
    /// Height of the isosceles triangle (distance from base BC to apex A).
    pub height: f32,

    // --- Topology ---
    /// Bidirectional links `[nodeI, nodeJ, nodeK]`; `children[0]` is the link in
    /// direction I, `children[1]` in direction J, `children[2]` in direction K.
    pub children: [Option<NodeRef>; 3],
}

impl Node {
    /// Creates a node and initializes its geometry.
    ///
    /// - `direction_of_node` — direction pointing toward apex A (perpendicular
    ///   to base BC); stored normalized.
    /// - `center` — center point of the node.
    /// - `origin` — position of the origin, used to orient the node.
    /// - `base_length` — length of the base edge BC of the node's triangle.
    /// - `height` — height of the isosceles triangle from base BC to apex A.
    /// - `name` — unique name identifying the node.
    /// - `labeling` — corner labeling convention; `Mirrored` swaps the B/C
    ///   corner assignment and therefore the I/K direction vectors.
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

    /// Builds a node from an explicit points triplet: the center is the
    /// centroid, `direction_to_origin` and the directions are derived from the
    /// points.
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

    /// Splits the node into four new nodes (three corner nodes plus one center
    /// node), interconnects them and returns the center node.
    /// See `docs/classes-definitions/node.md` for the full specification.
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

    /// Destroys the node by severing all bidirectional `children` links: for
    /// each linked neighbor, the reciprocal back-link is cleared first, then
    /// the link itself. The node is freed automatically once its last `Rc`
    /// reference is dropped — there is no explicit self-destruction in Rust.
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
