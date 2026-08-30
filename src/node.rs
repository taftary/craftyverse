use std::cell::RefCell;
use std::rc::Rc;

use glam::Vec2;

/// Shared, mutable link to a node. Used for the bidirectional `children` links.
pub type NodeRef = Rc<RefCell<Node>>;

/// Direction set type of a node, see `docs/classes-definitions/node.md`.
///
/// The direction set only describes the **orientation (winding) of the UV
/// triplet**: `Reverted` is the mirror image of `Normal` (B and C swapped).
/// Direction vectors are always computed the same way from the node's own
/// UVs: I ⊥ AB, J ⊥ BC, K ⊥ CA, each pointing from the center toward its edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectionSet {
    /// A on top, B bottom-right, C bottom-left (I up-right, J down, K up-left).
    Normal,
    /// Mirrored: B bottom-left, C bottom-right (I up-left, J down, K up-right).
    Reverted,
}

impl DirectionSet {
    fn flipped(self) -> Self {
        match self {
            DirectionSet::Normal => DirectionSet::Reverted,
            DirectionSet::Reverted => DirectionSet::Normal,
        }
    }

    /// Short label used in debug output (`N` or `R`).
    pub fn label(self) -> char {
        match self {
            DirectionSet::Normal => 'N',
            DirectionSet::Reverted => 'R',
        }
    }
}

/// A geometric node: triangle geometry, directional vectors and bidirectional
/// links to adjacent nodes. See `docs/classes-definitions/node.md`.
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
    /// Barycentric UV coordinates `[A, B, C]` — the node's triangle corners.
    pub uvs: [Vec2; 3],
    /// Direction set type of the node.
    pub direction_of_node: DirectionSet,

    // --- Topology ---
    /// Bidirectional links `[nodeI, nodeJ, nodeK]`; `children[0]` is the link in
    /// direction I, `children[1]` in direction J, `children[2]` in direction K.
    pub children: [Option<NodeRef>; 3],
}

impl Node {
    /// Creates a node and initializes its geometry.
    ///
    /// - `direction_of_node` — direction set (`Normal` or `Reverted`).
    /// - `center` — center point of the node.
    /// - `origin` — position of the origin, used to orient the node.
    /// - `base_length` — length of the base edge BC of the node's triangle.
    /// - `name` — unique name identifying the node.
    pub fn new(
        direction_of_node: DirectionSet,
        center: Vec2,
        origin: Vec2,
        base_length: f32,
        name: impl Into<String>,
    ) -> NodeRef {
        // Equilateral triangle with its centroid at `center`, base BC
        // horizontal. Normal: B bottom-right, C bottom-left. Reverted is the
        // mirror image (B and C swapped).
        let height = base_length * 3.0_f32.sqrt() / 2.0;
        let (b_x, c_x) = match direction_of_node {
            DirectionSet::Normal => (base_length / 2.0, -base_length / 2.0),
            DirectionSet::Reverted => (-base_length / 2.0, base_length / 2.0),
        };
        let uvs = [
            center + Vec2::new(0.0, 2.0 * height / 3.0),
            center + Vec2::new(b_x, -height / 3.0),
            center + Vec2::new(c_x, -height / 3.0),
        ];
        Rc::new(RefCell::new(Self::from_uvs(
            direction_of_node,
            uvs,
            origin,
            0,
            name.into(),
        )))
    }

    /// Builds a node from an explicit UV triplet: the center is the centroid,
    /// `direction_to_origin` and the directions are derived from the UVs.
    fn from_uvs(
        direction_of_node: DirectionSet,
        uvs: [Vec2; 3],
        origin: Vec2,
        level: u32,
        name: String,
    ) -> Self {
        let center = (uvs[0] + uvs[1] + uvs[2]) / 3.0;
        Node {
            name,
            level,
            center,
            direction_to_origin: origin - center,
            directions: compute_directions(&uvs, center),
            uvs,
            direction_of_node,
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
        let [uv_a, uv_b, uv_c] = self.uvs;

        // 1. UV midpoints.
        let uv_ab = (uv_a + uv_b) / 2.0;
        let uv_bc = (uv_b + uv_c) / 2.0;
        let uv_ca = (uv_c + uv_a) / 2.0;

        // 2./3. New nodes from their subdivided UV triplets (centers are the
        // centroids). In each corner node the corner vertex keeps its letter
        // and the midpoint toward a neighbor takes that neighbor's letter, so
        // corner nodes keep the parent's orientation (and its direction set).
        // The center node gets [uvBC, uvAB, uvCA], the mirrored orientation:
        // its direction set is flipped. Names derive from the parent name to
        // stay unique.
        let level = old_level + 1;
        let node_i = Rc::new(RefCell::new(Self::from_uvs(
            self.direction_of_node,
            [uv_a, uv_ab, uv_ca],
            origin,
            level,
            format!("{}.I", self.name),
        )));
        let node_j = Rc::new(RefCell::new(Self::from_uvs(
            self.direction_of_node,
            [uv_ab, uv_b, uv_bc],
            origin,
            level,
            format!("{}.J", self.name),
        )));
        let node_k = Rc::new(RefCell::new(Self::from_uvs(
            self.direction_of_node,
            [uv_ca, uv_bc, uv_c],
            origin,
            level,
            format!("{}.K", self.name),
        )));
        let node_center = Rc::new(RefCell::new(Self::from_uvs(
            self.direction_of_node.flipped(),
            [uv_bc, uv_ab, uv_ca],
            origin,
            level,
            format!("{}.C", self.name),
        )));

        // 4. Internal interconnection (bidirectional).
        node_center.borrow_mut().children[0] = Some(Rc::clone(&node_i));
        node_i.borrow_mut().children[2] = Some(Rc::clone(&node_center));

        node_center.borrow_mut().children[1] = Some(Rc::clone(&node_j));
        node_j.borrow_mut().children[1] = Some(Rc::clone(&node_center));

        node_center.borrow_mut().children[2] = Some(Rc::clone(&node_k));
        node_k.borrow_mut().children[0] = Some(Rc::clone(&node_center));

        node_center
    }
}

/// Unit perpendicular of an edge, pointing from `center` toward the edge.
fn perpendicular_toward(a: Vec2, b: Vec2, center: Vec2) -> Vec2 {
    let edge = b - a;
    let candidate = Vec2::new(-edge.y, edge.x);
    let toward_edge = (a + b) / 2.0 - center;
    let direction = if candidate.dot(toward_edge) >= 0.0 {
        candidate
    } else {
        -candidate
    };
    direction.normalize()
}

/// Computes the `[i, j, k]` direction triplet from the node's own UV triplet:
/// I ⊥ AB, J ⊥ BC, K ⊥ CA, each pointing from the center toward its edge.
/// The rule is uniform — the node's direction set is already encoded in the
/// orientation (winding) of its UV triplet.
fn compute_directions(uvs: &[Vec2; 3], center: Vec2) -> [Vec2; 3] {
    let [a, b, c] = *uvs;
    [
        perpendicular_toward(a, b, center),
        perpendicular_toward(b, c, center),
        perpendicular_toward(c, a, center),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;
    const SQRT_3_2: f32 = 0.866_025_4; // √3 / 2

    fn approx_eq(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < EPSILON
    }

    fn test_node() -> NodeRef {
        Node::new(
            DirectionSet::Normal,
            Vec2::ZERO,
            Vec2::new(0.0, 1000.0),
            300.0,
            "root",
        )
    }

    #[test]
    fn new_initializes_identity_and_geometry() {
        let node = test_node();
        let node = node.borrow();

        assert_eq!(node.name, "root");
        assert_eq!(node.level, 0);
        assert!(approx_eq(node.center, Vec2::ZERO));
        assert!(approx_eq(node.direction_to_origin, Vec2::new(0.0, 1000.0)));
        assert!(node.children.iter().all(|slot| slot.is_none()));
    }

    #[test]
    fn new_builds_equilateral_triangle_around_center() {
        let node = test_node();
        let node = node.borrow();
        let [a, b, c] = node.uvs;

        // Centroid is the center, base BC has the requested length.
        assert!(approx_eq((a + b + c) / 3.0, node.center));
        assert!((b - c).length() - 300.0 < EPSILON);
        let side = (a - b).length();
        assert!(((a - b).length() - (b - c).length()).abs() < EPSILON);
        assert!(((c - a).length() - side).abs() < EPSILON);

        // Normal orientation: A on top, B bottom-right, C bottom-left.
        assert!(a.y > 0.0 && approx_eq(Vec2::new(a.x, 0.0), Vec2::ZERO));
        assert!(b.x > 0.0 && b.y < 0.0);
        assert!(c.x < 0.0 && c.y < 0.0);
    }

    #[test]
    fn normal_directions_match_expected_orientation() {
        let node = test_node();
        let [i, j, k] = node.borrow().directions;

        // I up-right (⊥ AB), J straight down (⊥ BC), K up-left (⊥ CA).
        assert!(approx_eq(i, Vec2::new(SQRT_3_2, 0.5)));
        assert!(approx_eq(j, Vec2::new(0.0, -1.0)));
        assert!(approx_eq(k, Vec2::new(-SQRT_3_2, 0.5)));
    }

    #[test]
    fn directions_are_perpendicular_and_point_toward_edges() {
        let node = test_node();
        let node = node.borrow();
        let [a, b, c] = node.uvs;
        let [i, j, k] = node.directions;

        // NormalDirection: I ⊥ AB, J ⊥ BC, K ⊥ CA.
        for (direction, edge_start, edge_end) in [(i, a, b), (j, b, c), (k, c, a)] {
            let edge = edge_end - edge_start;
            assert!(direction.dot(edge).abs() < EPSILON);
            let edge_mid = (edge_start + edge_end) / 2.0;
            assert!(direction.dot(edge_mid - node.center) > 0.0);
            assert!((direction.length() - 1.0).abs() < EPSILON);
        }
    }

    #[test]
    fn reverted_direction_set_mirrors_the_triangle() {
        let node = Node::new(
            DirectionSet::Reverted,
            Vec2::ZERO,
            Vec2::new(0.0, 1000.0),
            300.0,
            "rev",
        );
        let node = node.borrow();
        let [a, b, c] = node.uvs;
        let [i, j, k] = node.directions;

        // Reverted orientation: B bottom-left, C bottom-right (mirror of Normal).
        assert!(b.x < 0.0 && b.y < 0.0);
        assert!(c.x > 0.0 && c.y < 0.0);

        // Directions are mirrored too: I up-left, J down, K up-right.
        assert!(approx_eq(i, Vec2::new(-SQRT_3_2, 0.5)));
        assert!(approx_eq(j, Vec2::new(0.0, -1.0)));
        assert!(approx_eq(k, Vec2::new(SQRT_3_2, 0.5)));

        // The uniform rule still holds: I ⊥ AB, J ⊥ BC, K ⊥ CA, toward edges.
        for (direction, edge_start, edge_end) in [(i, a, b), (j, b, c), (k, c, a)] {
            assert!(direction.dot(edge_end - edge_start).abs() < EPSILON);
            let edge_mid = (edge_start + edge_end) / 2.0;
            assert!(direction.dot(edge_mid - node.center) > 0.0);
        }
    }

    #[test]
    fn split_returns_center_node_with_incremented_level() {
        let node = test_node();
        let center = node.borrow().split();

        assert_eq!(center.borrow().level, 1);
        assert_eq!(center.borrow().name, "root.C");
        assert_eq!(center.borrow().direction_of_node, DirectionSet::Reverted);
        // The parent node keeps its own level.
        assert_eq!(node.borrow().level, 0);
    }

    #[test]
    fn split_connects_center_and_corner_nodes_bidirectionally() {
        let node = test_node();
        let center = node.borrow().split();
        let center_ref = center.borrow();

        let node_i = center_ref.children[0].as_ref().expect("node I");
        let node_j = center_ref.children[1].as_ref().expect("node J");
        let node_k = center_ref.children[2].as_ref().expect("node K");

        assert!(Rc::ptr_eq(
            node_i.borrow().children[2].as_ref().unwrap(),
            &center
        ));
        assert!(Rc::ptr_eq(
            node_j.borrow().children[1].as_ref().unwrap(),
            &center
        ));
        assert!(Rc::ptr_eq(
            node_k.borrow().children[0].as_ref().unwrap(),
            &center
        ));

        // Corner nodes inherit the direction set and derive unique names.
        for (corner, suffix) in [(node_i, "root.I"), (node_j, "root.J"), (node_k, "root.K")] {
            let corner = corner.borrow();
            assert_eq!(corner.direction_of_node, DirectionSet::Normal);
            assert_eq!(corner.name, suffix);
            assert_eq!(corner.level, 1);
        }

        // No cross-connections between corner nodes.
        assert!(node_i.borrow().children[0].is_none());
        assert!(node_i.borrow().children[1].is_none());
    }

    #[test]
    fn split_subdivides_uvs() {
        let node = test_node();
        let [uv_a, uv_b, uv_c] = node.borrow().uvs;
        let center = node.borrow().split();

        let uv_ab = (uv_a + uv_b) / 2.0;
        let uv_bc = (uv_b + uv_c) / 2.0;
        let uv_ca = (uv_c + uv_a) / 2.0;

        let center_ref = center.borrow();
        // Center node: mirrored orientation [uvBC, uvAB, uvCA].
        for (actual, expected) in center_ref.uvs.iter().zip([uv_bc, uv_ab, uv_ca]) {
            assert!(approx_eq(*actual, expected));
        }
        // Corner nodes: the corner vertex keeps its letter, the midpoint
        // toward a neighbor takes that neighbor's letter.
        let node_i = center_ref.children[0].as_ref().unwrap().borrow();
        for (actual, expected) in node_i.uvs.iter().zip([uv_a, uv_ab, uv_ca]) {
            assert!(approx_eq(*actual, expected));
        }
        let node_j = center_ref.children[1].as_ref().unwrap().borrow();
        for (actual, expected) in node_j.uvs.iter().zip([uv_ab, uv_b, uv_bc]) {
            assert!(approx_eq(*actual, expected));
        }
        let node_k = center_ref.children[2].as_ref().unwrap().borrow();
        for (actual, expected) in node_k.uvs.iter().zip([uv_ca, uv_bc, uv_c]) {
            assert!(approx_eq(*actual, expected));
        }
    }

    #[test]
    fn split_corner_nodes_keep_parent_orientation() {
        let node = test_node();
        let center = node.borrow().split();
        let center_ref = center.borrow();

        // All corner nodes have the same global orientation as the parent:
        // I up-right, J straight down, K up-left.
        for slot in &center_ref.children {
            let [i, j, k] = slot.as_ref().unwrap().borrow().directions;
            assert!(approx_eq(i, Vec2::new(SQRT_3_2, 0.5)));
            assert!(approx_eq(j, Vec2::new(0.0, -1.0)));
            assert!(approx_eq(k, Vec2::new(-SQRT_3_2, 0.5)));
        }
    }

    #[test]
    fn split_center_node_has_mirrored_orientation() {
        let node = test_node();
        let center = node.borrow().split();
        let [i, j, k] = center.borrow().directions;

        // Center node: I down-right, J straight up, K down-left.
        assert!(approx_eq(i, Vec2::new(SQRT_3_2, -0.5)));
        assert!(approx_eq(j, Vec2::new(0.0, 1.0)));
        assert!(approx_eq(k, Vec2::new(-SQRT_3_2, -0.5)));
    }

    #[test]
    fn split_recomputes_directions_from_own_uvs() {
        let node = test_node();
        let center = node.borrow().split();
        let center_ref = center.borrow();
        let [a, b, c] = center_ref.uvs;
        let [i, j, k] = center_ref.directions;

        // The uniform rule is computed from the node's own UV triplet:
        // I ⊥ AB, J ⊥ BC, K ⊥ CA, all pointing toward their edge.
        for (direction, edge_start, edge_end) in [(i, a, b), (j, b, c), (k, c, a)] {
            assert!(direction.dot(edge_end - edge_start).abs() < EPSILON);
            let edge_mid = (edge_start + edge_end) / 2.0;
            assert!(direction.dot(edge_mid - center_ref.center) > 0.0);
        }
    }

    #[test]
    fn split_can_be_called_multiple_times() {
        let node = test_node();
        let first = node.borrow().split();
        let second = node.borrow().split();

        // Each call produces four new nodes; the returned centers are distinct.
        assert!(!Rc::ptr_eq(&first, &second));
        assert_eq!(first.borrow().level, 1);
        assert_eq!(second.borrow().level, 1);
    }
}
