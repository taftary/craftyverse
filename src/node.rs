use std::cell::RefCell;
use std::rc::Rc;

use glam::Vec2;

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
        // Isosceles triangle whose centroid is `center`: base BC is
        // perpendicular to `direction_of_node` and apex A is aligned with it
        // at distance `height` from BC. The centroid sits at 1/3 of the height
        // from the base (2/3 from the apex). With `Labeling::Normal`, B is
        // placed right of the direction axis and C left (when the direction
        // points up); `Labeling::Mirrored` swaps the B/C assignment and
        // therefore the I/K directions.
        let direction = direction_of_node.normalize();
        let mut perpendicular = Vec2::new(-direction.y, direction.x);
        if labeling == Labeling::Mirrored {
            perpendicular = -perpendicular;
        }
        let apex = center + direction * (2.0 * height / 3.0);
        let base_mid = center - direction * (height / 3.0);
        let points = [
            apex,
            base_mid - perpendicular * (base_length / 2.0),
            base_mid + perpendicular * (base_length / 2.0),
        ];
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
        let p_ab = (p_a + p_b) / 2.0;
        let p_bc = (p_b + p_c) / 2.0;
        let p_ca = (p_c + p_a) / 2.0;

        // 2./3. New nodes from their subdivided points triplets (centers are
        // the centroids). Corner nodes keep the parent's `direction_of_node`
        // (their apex is a parent corner, so the geometry matches); the center
        // node is inverted relative to the parent triangle, so its direction
        // is flipped. Each new node gets half the parent's base length and
        // height. Names derive from the parent name to stay unique.
        let level = old_level + 1;
        let base_length = self.base_length / 2.0;
        let height = self.height / 2.0;
        let node_i = Rc::new(RefCell::new(Self::from_points(
            self.direction_of_node,
            [p_a, p_ab, p_ca],
            origin,
            base_length,
            height,
            level,
            format!("{}.I", self.name),
        )));
        let node_j = Rc::new(RefCell::new(Self::from_points(
            self.direction_of_node,
            [p_ab, p_b, p_bc],
            origin,
            base_length,
            height,
            level,
            format!("{}.J", self.name),
        )));
        let node_k = Rc::new(RefCell::new(Self::from_points(
            self.direction_of_node,
            [p_ca, p_bc, p_c],
            origin,
            base_length,
            height,
            level,
            format!("{}.K", self.name),
        )));
        let node_center = Rc::new(RefCell::new(Self::from_points(
            -self.direction_of_node,
            [p_bc, p_ab, p_ca],
            origin,
            base_length,
            height,
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

    /// Destroys the node by severing all bidirectional `children` links: for
    /// each linked neighbor, the reciprocal back-link is cleared first, then
    /// the link itself. The node is freed automatically once its last `Rc`
    /// reference is dropped — there is no explicit self-destruction in Rust.
    pub fn destroy(&mut self) {
        // (link index on self, reciprocal back-link index on the neighbor),
        // mirroring the interconnections established by `split()`.
        for (index, back_index) in [(0, 2), (1, 1), (2, 0)] {
            if let Some(child) = self.children[index].take() {
                child.borrow_mut().children[back_index] = None;
            }
        }
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

/// Computes the `[i, j, k]` direction triplet from the node's own points
/// triplet: I ⊥ AB, J ⊥ BC, K ⊥ CA, each pointing from the center toward its
/// edge.
fn compute_directions(points: &[Vec2; 3], center: Vec2) -> [Vec2; 3] {
    let [a, b, c] = *points;
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

    /// Equilateral test node (base 300, apex up, height = base * √3 / 2).
    fn test_node() -> NodeRef {
        Node::new(
            Vec2::Y,
            Vec2::ZERO,
            Vec2::new(0.0, 1000.0),
            300.0,
            300.0 * 3.0_f32.sqrt() / 2.0,
            "root",
            Labeling::Normal,
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
        assert!(approx_eq(node.direction_of_node, Vec2::Y));
        assert_eq!(node.base_length, 300.0);
        assert_eq!(node.height, 300.0 * 3.0_f32.sqrt() / 2.0);
        assert!(node.children.iter().all(|slot| slot.is_none()));
    }

    #[test]
    fn new_normalizes_direction_of_node() {
        let node = Node::new(
            Vec2::new(0.0, 42.0),
            Vec2::ZERO,
            Vec2::ZERO,
            300.0,
            200.0,
            "scaled",
            Labeling::Normal,
        );
        assert!(approx_eq(node.borrow().direction_of_node, Vec2::Y));
    }

    #[test]
    fn new_builds_equilateral_triangle_around_center() {
        let node = test_node();
        let node = node.borrow();
        let [a, b, c] = node.points;

        // Centroid is the center, base BC has the requested length.
        assert!(approx_eq((a + b + c) / 3.0, node.center));
        assert!(((b - c).length() - 300.0).abs() < EPSILON);
        let side = (a - b).length();
        assert!(((a - b).length() - (b - c).length()).abs() < EPSILON);
        assert!(((c - a).length() - side).abs() < EPSILON);

        // Apex up: A on top, B bottom-right, C bottom-left.
        assert!(a.y > 0.0 && approx_eq(Vec2::new(a.x, 0.0), Vec2::ZERO));
        assert!(b.x > 0.0 && b.y < 0.0);
        assert!(c.x < 0.0 && c.y < 0.0);
    }

    #[test]
    fn new_builds_isosceles_triangle_from_direction_and_dimensions() {
        let node = Node::new(Vec2::X, Vec2::ZERO, Vec2::ZERO, 300.0, 200.0, "iso", Labeling::Normal);
        let node = node.borrow();
        let [a, b, c] = node.points;

        // Direction stored normalized, centroid is the center.
        assert!(approx_eq(node.direction_of_node, Vec2::X));
        assert!(approx_eq((a + b + c) / 3.0, Vec2::ZERO));

        // Apex A is 2/3 of the height along the direction, the base midpoint
        // 1/3 against it; BC is perpendicular to the direction, B and C half
        // the base length away on the perpendicular axis.
        assert!(approx_eq(a, Vec2::new(2.0 * 200.0 / 3.0, 0.0)));
        assert!(approx_eq(b, Vec2::new(-200.0 / 3.0, -150.0)));
        assert!(approx_eq(c, Vec2::new(-200.0 / 3.0, 150.0)));

        // Isosceles: AB == AC, apex at distance `height` from base BC.
        assert!(((a - b).length() - (a - c).length()).abs() < EPSILON);
        assert!(((a - (b + c) / 2.0).length() - 200.0).abs() < EPSILON);
    }

    #[test]
    fn new_mirrored_labeling_swaps_b_c_and_i_k() {
        let normal = Node::new(Vec2::X, Vec2::ZERO, Vec2::ZERO, 300.0, 200.0, "n", Labeling::Normal);
        let mirrored = Node::new(Vec2::X, Vec2::ZERO, Vec2::ZERO, 300.0, 200.0, "m", Labeling::Mirrored);
        let normal = normal.borrow();
        let mirrored = mirrored.borrow();

        // Same triangle, swapped B/C labels — hence swapped I/K directions,
        // with J (perpendicular to the unchanged base edge) preserved.
        assert!(approx_eq(normal.points[0], mirrored.points[0]));
        assert!(approx_eq(normal.points[1], mirrored.points[2]));
        assert!(approx_eq(normal.points[2], mirrored.points[1]));
        assert!(approx_eq(normal.directions[0], mirrored.directions[2]));
        assert!(approx_eq(normal.directions[2], mirrored.directions[0]));
        assert!(approx_eq(normal.directions[1], mirrored.directions[1]));
    }

    #[test]
    fn equilateral_directions_match_expected_orientation() {
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
        let [a, b, c] = node.points;
        let [i, j, k] = node.directions;

        // Uniform rule: I ⊥ AB, J ⊥ BC, K ⊥ CA.
        for (direction, edge_start, edge_end) in [(i, a, b), (j, b, c), (k, c, a)] {
            let edge = edge_end - edge_start;
            assert!(direction.dot(edge).abs() < EPSILON);
            let edge_mid = (edge_start + edge_end) / 2.0;
            assert!(direction.dot(edge_mid - node.center) > 0.0);
            assert!((direction.length() - 1.0).abs() < EPSILON);
        }
    }

    #[test]
    fn split_returns_center_node_with_incremented_level() {
        let node = test_node();
        let center = node.borrow().split();

        assert_eq!(center.borrow().level, 1);
        assert_eq!(center.borrow().name, "root.C");
        // The center node is inverted relative to the parent.
        assert!(approx_eq(center.borrow().direction_of_node, -Vec2::Y));
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

        // Corner nodes keep the parent's direction_of_node and derive unique
        // names.
        for (corner, suffix) in [(node_i, "root.I"), (node_j, "root.J"), (node_k, "root.K")] {
            let corner = corner.borrow();
            assert!(approx_eq(corner.direction_of_node, Vec2::Y));
            assert_eq!(corner.name, suffix);
            assert_eq!(corner.level, 1);
        }

        // No cross-connections between corner nodes.
        assert!(node_i.borrow().children[0].is_none());
        assert!(node_i.borrow().children[1].is_none());
    }

    #[test]
    fn split_subdivides_points() {
        let node = test_node();
        let [p_a, p_b, p_c] = node.borrow().points;
        let center = node.borrow().split();

        let p_ab = (p_a + p_b) / 2.0;
        let p_bc = (p_b + p_c) / 2.0;
        let p_ca = (p_c + p_a) / 2.0;

        let center_ref = center.borrow();
        // Center node: inverted orientation [pBC, pAB, pCA].
        for (actual, expected) in center_ref.points.iter().zip([p_bc, p_ab, p_ca]) {
            assert!(approx_eq(*actual, expected));
        }
        // Corner nodes: the corner point keeps its letter, the midpoint toward
        // a neighbor takes that neighbor's letter.
        let node_i = center_ref.children[0].as_ref().unwrap().borrow();
        for (actual, expected) in node_i.points.iter().zip([p_a, p_ab, p_ca]) {
            assert!(approx_eq(*actual, expected));
        }
        let node_j = center_ref.children[1].as_ref().unwrap().borrow();
        for (actual, expected) in node_j.points.iter().zip([p_ab, p_b, p_bc]) {
            assert!(approx_eq(*actual, expected));
        }
        let node_k = center_ref.children[2].as_ref().unwrap().borrow();
        for (actual, expected) in node_k.points.iter().zip([p_ca, p_bc, p_c]) {
            assert!(approx_eq(*actual, expected));
        }
    }

    #[test]
    fn split_halves_base_length_and_height() {
        let node = test_node();
        let (base_length, height) = {
            let node = node.borrow();
            (node.base_length, node.height)
        };
        let center = node.borrow().split();
        let center_ref = center.borrow();

        // All four new nodes get half the parent's base length and height, and
        // the stored dimensions match the actual subdivided points geometry.
        for slot in &center_ref.children {
            let corner = slot.as_ref().unwrap().borrow();
            assert_eq!(corner.base_length, base_length / 2.0);
            assert_eq!(corner.height, height / 2.0);
            let [a, b, c] = corner.points;
            assert!(((b - c).length() - corner.base_length).abs() < EPSILON);
            assert!(((a - (b + c) / 2.0).length() - corner.height).abs() < EPSILON);
        }
        assert_eq!(center_ref.base_length, base_length / 2.0);
        assert_eq!(center_ref.height, height / 2.0);
        let [a, b, c] = center_ref.points;
        assert!(((b - c).length() - center_ref.base_length).abs() < EPSILON);
        assert!(((a - (b + c) / 2.0).length() - center_ref.height).abs() < EPSILON);
    }

    #[test]
    fn split_corner_nodes_keep_parent_orientation() {
        let node = test_node();
        let center = node.borrow().split();
        let center_ref = center.borrow();

        // All corner nodes keep the parent's direction_of_node and have the
        // same global orientation as the parent: I up-right, J straight down,
        // K up-left.
        for slot in &center_ref.children {
            let corner = slot.as_ref().unwrap().borrow();
            assert!(approx_eq(corner.direction_of_node, Vec2::Y));
            let [i, j, k] = corner.directions;
            assert!(approx_eq(i, Vec2::new(SQRT_3_2, 0.5)));
            assert!(approx_eq(j, Vec2::new(0.0, -1.0)));
            assert!(approx_eq(k, Vec2::new(-SQRT_3_2, 0.5)));
        }
    }

    #[test]
    fn split_center_node_has_mirrored_orientation() {
        let node = test_node();
        let center = node.borrow().split();
        let center_ref = center.borrow();

        assert!(approx_eq(center_ref.direction_of_node, -Vec2::Y));
        // Center node: I down-right, J straight up, K down-left.
        let [i, j, k] = center_ref.directions;
        assert!(approx_eq(i, Vec2::new(SQRT_3_2, -0.5)));
        assert!(approx_eq(j, Vec2::new(0.0, 1.0)));
        assert!(approx_eq(k, Vec2::new(-SQRT_3_2, -0.5)));
    }

    #[test]
    fn split_recomputes_directions_from_own_points() {
        let node = test_node();
        let center = node.borrow().split();
        let center_ref = center.borrow();
        let [a, b, c] = center_ref.points;
        let [i, j, k] = center_ref.directions;

        // The uniform rule is computed from the node's own points triplet:
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

    #[test]
    fn destroy_severs_all_bidirectional_links() {
        let node = test_node();
        let center = node.borrow().split();
        let (node_i, node_j, node_k) = {
            let center_ref = center.borrow();
            (
                Rc::clone(center_ref.children[0].as_ref().unwrap()),
                Rc::clone(center_ref.children[1].as_ref().unwrap()),
                Rc::clone(center_ref.children[2].as_ref().unwrap()),
            )
        };

        center.borrow_mut().destroy();

        // All of the center node's links are cleared.
        assert!(center.borrow().children.iter().all(|slot| slot.is_none()));
        // Each corner node's reciprocal back-link is cleared too.
        assert!(node_i.borrow().children[2].is_none());
        assert!(node_j.borrow().children[1].is_none());
        assert!(node_k.borrow().children[0].is_none());
        // With the links gone, only the test's own references keep the corner
        // nodes alive.
        assert_eq!(Rc::strong_count(&node_i), 1);
        assert_eq!(Rc::strong_count(&node_j), 1);
        assert_eq!(Rc::strong_count(&node_k), 1);
    }

    #[test]
    fn destroy_without_links_is_noop() {
        let node = test_node();
        node.borrow_mut().destroy();
        assert!(node.borrow().children.iter().all(|slot| slot.is_none()));
    }
}
