//! The `Plan` class: central manager of the spatial hierarchy. Encapsulates a
//! primary root node and orchestrates pentagonal base generation and the
//! dual-pentagon interlocked mesh. See `docs/classes-definitions/plan.md`.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::f32::consts::PI;
use std::rc::Rc;

use glam::Vec2;

use crate::node::{Labeling, Node, NodeRef};

/// Central manager of the node hierarchy. Holds a reference to the primary
/// base node that anchors the generated mesh.
#[derive(Default)]
pub struct Plan {
    /// Reference to the primary base node (North base root after `generate`).
    pub root_node: Option<NodeRef>,
}

impl Plan {
    pub fn new() -> Self {
        Self::default()
    }

    /// Constructs a pentagonal base structure composed of 5 paired Node
    /// structures (10 nodes total): 5 inward-pointing `base_node`s whose
    /// apexes converge at `pentagon_center`, and 5 outward-pointing
    /// `reverted_node`s. Each pair shares a base edge flush with a pentagon
    /// side. Returns the first generated base node.
    ///
    /// - `name` — name prefix of the base structure.
    /// - `side_length` — length of each pentagon edge.
    /// - `pentagon_direction` — initial orientation vector.
    /// - `pentagon_center` — center coordinates of the pentagon.
    /// - `labeling` — corner labeling convention for the `base_node`s;
    ///   `reverted_node`s receive the opposite labeling.
    pub fn generate_base(
        &mut self,
        name: &str,
        side_length: f32,
        pentagon_direction: Vec2,
        pentagon_center: Vec2,
        labeling: Labeling,
    ) -> NodeRef {
        let dir = pentagon_direction.normalize();
        // Apothem, node height (h = r) and centroid offset (c = h/3).
        let r = side_length / (2.0 * (PI / 5.0).tan());
        let height = r;
        let c = height / 3.0;

        let mut first_node: Option<NodeRef> = None;
        let mut next_node: Option<NodeRef> = None;

        for i in 0..5 {
            // Outward normal of pentagon side i.
            let theta = dir.y.atan2(dir.x) + i as f32 * (2.0 * PI / 5.0);
            let outward_normal = Vec2::new(theta.cos(), theta.sin());

            // Side midpoint (shared base edge boundary) and paired centroids,
            // each offset by c perpendicular to the side.
            let side_midpoint = pentagon_center + outward_normal * r;
            let direction_to_center = (pentagon_center - side_midpoint).normalize();
            let base_centroid = side_midpoint + direction_to_center * c;
            let reverted_centroid = side_midpoint + outward_normal * c;

            // The pair is built with opposite labelings: the reverted node is
            // mirrored across the shared base edge (B/C, hence I/K, swapped)
            // so both nodes label the same pentagon vertex with the same
            // letter — the pair's I arrows point toward the same vertex, and
            // so do the K arrows.
            let base_node = Node::new(
                direction_to_center,
                base_centroid,
                pentagon_center,
                side_length,
                height,
                format!("{name}base_node_{i}"),
                labeling,
            );
            let reverted_node = Node::new(
                outward_normal,
                reverted_centroid,
                pentagon_center,
                side_length,
                height,
                format!("{name}reverted_node_{i}"),
                labeling.opposite(),
            );

            // Opposing pair link (child index 1 / direction J).
            base_node.borrow_mut().children[1] = Some(Rc::clone(&reverted_node));
            reverted_node.borrow_mut().children[1] = Some(Rc::clone(&base_node));

            // Sequential perimeter loop (child indices 0 & 2).
            if i == 0 {
                first_node = Some(Rc::clone(&base_node));
            }
            if let Some(previous) = &next_node {
                base_node.borrow_mut().children[2] = Some(Rc::clone(previous));
                previous.borrow_mut().children[0] = Some(Rc::clone(&base_node));
            }
            if i == 4 {
                // Close the circular perimeter loop with the first node.
                let first = first_node.as_ref().unwrap();
                first.borrow_mut().children[2] = Some(Rc::clone(&base_node));
                base_node.borrow_mut().children[0] = Some(Rc::clone(first));
            }
            next_node = Some(base_node);
        }

        let first_node = first_node.expect("the loop always runs 5 iterations");
        self.root_node = Some(Rc::clone(&first_node));
        first_node
    }

    /// Generates North and South pentagonal base structures and connects North
    /// inner nodes to South inner nodes via their remaining open ports
    /// (reciprocal I <-> K links). The South base is offset along the Y-axis
    /// and keeps the same orientation as the North base, but is generated with
    /// mirrored labeling so every South node's I/K direction vectors are
    /// swapped relative to the North convention. Returns the primary root
    /// node (North base root).
    ///
    /// - `side_length` — length of each pentagon edge.
    pub fn generate(&mut self, side_length: f32) -> NodeRef {
        let north_center = Vec2::new(side_length * 3.0, side_length * 3.0);
        let north_dir = Vec2::Y;
        let r = side_length / (2.0 * (PI / 5.0).tan());
        let south_center = north_center + Vec2::new(0.0, 4.0 * r);

        let north_root = self.generate_base("north_", side_length, north_dir, north_center, Labeling::Normal);
        let south_root = self.generate_base("south_", side_length, north_dir, south_center, Labeling::Mirrored);

        let north_reverted = Self::get_reverted_nodes(&north_root);
        let south_reverted = Self::get_reverted_nodes(&south_root);

        // Wire the interlocking directional ports (reciprocal I <-> K links).
        for i in 0..5 {
            let north_node = &north_reverted[i];

            // North port I (index 0) to South port K (index 2), reflection
            // offset southIdx_I = (2 - i) mod 5 (the +5 keeps the unsigned
            // arithmetic non-negative).
            let south_idx_i = (2 + 5 - i) % 5;
            let target_south_node_k = &south_reverted[south_idx_i];
            north_node.borrow_mut().children[0] = Some(Rc::clone(target_south_node_k));
            target_south_node_k.borrow_mut().children[2] = Some(Rc::clone(north_node));

            // North port K (index 2) to South port I (index 0), reflection
            // offset southIdx_K = (3 - i) mod 5.
            let south_idx_k = (3 + 5 - i) % 5;
            let target_south_node_i = &south_reverted[south_idx_k];
            north_node.borrow_mut().children[2] = Some(Rc::clone(target_south_node_i));
            target_south_node_i.borrow_mut().children[0] = Some(Rc::clone(north_node));
        }

        self.root_node = Some(Rc::clone(&north_root));
        north_root
    }

    /// Subdivides the whole mesh one level: splits every node of the current
    /// level once, reconnects the resulting split-centers across the
    /// subdivided edges, and destroys the old nodes. See
    /// `docs/classes-definitions/plan.md` section 7.
    ///
    /// Two passes over the old level, which every node of survives until the
    /// end: first split every node and index the centers by parent, then wire
    /// the fresh corner nodes across every old edge exactly once — `0 <-> 2`
    /// edges from their port-0 side (`wire_chain_edge`), `1 <-> 1` edges from
    /// one canonical side (`wire_pair_edge`). `root_node` is re-anchored on
    /// the old root's center before the old nodes are destroyed.
    pub fn split(&mut self) {
        let root = self
            .root_node
            .clone()
            .expect("split() requires a generated mesh (root_node is None)");
        let old_nodes = collect_nodes(&root);

        // Split every node once, indexing the new center by parent. Old nodes
        // stay alive (and their addresses stable) until the destroy pass.
        let mut centers: HashMap<*const RefCell<Node>, NodeRef> = HashMap::with_capacity(old_nodes.len());
        for node in &old_nodes {
            centers.insert(Rc::as_ptr(node), node.borrow().split());
        }

        // Wire the new corner nodes across every old edge. Each edge appears
        // twice in the enumeration (once per endpoint); reciprocity (0 <-> 2,
        // 1 <-> 1) guarantees the canonicalization wires it exactly once.
        for node in &old_nodes {
            for index in 0..3 {
                let Some(neighbor) = child(node, index) else { continue };
                match index {
                    // 0 <-> 2 edges are wired from their port-0 side.
                    0 => wire_chain_edge(&centers[&Rc::as_ptr(node)], &centers[&Rc::as_ptr(&neighbor)]),
                    // 1 <-> 1 edges are wired from one canonical side.
                    1 if Rc::as_ptr(node) < Rc::as_ptr(&neighbor) => {
                        wire_pair_edge(&centers[&Rc::as_ptr(node)], &centers[&Rc::as_ptr(&neighbor)])
                    }
                    _ => {}
                }
            }
        }

        // Re-anchor on the old root's center and release the old level.
        self.root_node = Some(Rc::clone(&centers[&Rc::as_ptr(&root)]));
        for node in &old_nodes {
            node.borrow_mut().destroy();
        }
    }

    /// Extracts the 5 inner `reverted_node`s in circular sequence: walks the
    /// outer perimeter loop via `children[2]` from `root_base_node` and
    /// collects each paired inner node via `children[1]`.
    fn get_reverted_nodes(root_base_node: &NodeRef) -> [NodeRef; 5] {
        let mut reverted_nodes = Vec::with_capacity(5);
        let mut current_node = Rc::clone(root_base_node);
        for _ in 0..5 {
            let (reverted, next) = {
                let current = current_node.borrow();
                (
                    Rc::clone(current.children[1].as_ref().expect("paired reverted node")),
                    Rc::clone(current.children[2].as_ref().expect("next base node")),
                )
            };
            reverted_nodes.push(reverted);
            current_node = next;
        }
        reverted_nodes
            .try_into()
            .unwrap_or_else(|_| panic!("the loop always collects 5 nodes"))
    }
}

/// Clone of the link at `index` on `node` (`None` when the port is open).
fn child(node: &NodeRef, index: usize) -> Option<NodeRef> {
    node.borrow().children[index].clone()
}

/// Sets the link at `index` on `node` to `new_link`.
fn set_child(node: &NodeRef, index: usize, new_link: NodeRef) {
    node.borrow_mut().children[index] = Some(new_link);
}

/// Tolerance for the edge-midpoint coincidence test, relative to the corner
/// node's base length. The two halves of a subdivided edge sit a quarter of
/// the parent edge apart; belt edges sit the whole interlock gap apart, so
/// any value well below 0.25 classifies correctly at every level.
const COINCIDENCE_TOLERANCE: f32 = 0.01;

/// Midpoint of the edge faced by port `index` (I ⊥ AB, J ⊥ BC, K ⊥ CA).
fn edge_midpoint(node: &NodeRef, index: usize) -> Vec2 {
    let points = node.borrow().points;
    let (a, b) = match index {
        0 => (points[0], points[1]),
        1 => (points[1], points[2]),
        _ => (points[2], points[0]),
    };
    (a + b) / 2.0
}

/// Wires the two reciprocal corner-to-corner links across the edge two split
/// parents shared via `p_center`'s parent port I (`children[0]`) and
/// `q_center`'s parent port K (`children[2]`). Each half of the shared edge
/// carries one corner port: corner nodes I (near A) and J (near B) on the `p`
/// side, I (near A) and K (near C) on the `q` side.
///
/// When the corner nodes I of both sides sit at the same endpoint of the
/// shared edge (their facing edge midpoints coincide), the corner letters
/// match straight (`p.A = q.A`, `p.B = q.C`) and the halves wire `I<->I`,
/// `J<->K`. Otherwise — belt edges bridging the interlock gap between the two
/// pentagons, and internal center–corner edges from level 1 on — the letters
/// pair crosswise (`p.A = q.C`, `p.B = q.A`) and the halves wire `I<->K`,
/// `J<->I`, keeping every mesh vertex on its own side of the link.
fn wire_chain_edge(p_center: &NodeRef, q_center: &NodeRef) {
    let p_i = child(p_center, 1).expect("corner node I");
    let p_j = child(p_center, 0).expect("corner node J");
    let q_i = child(q_center, 1).expect("corner node I");
    let q_k = child(q_center, 2).expect("corner node K");

    let coincident = edge_midpoint(&p_i, 0).distance(edge_midpoint(&q_i, 2))
        < p_i.borrow().base_length * COINCIDENCE_TOLERANCE;
    let (q_near_a, q_near_b) = if coincident { (&q_i, &q_k) } else { (&q_k, &q_i) };
    set_child(&p_i, 0, Rc::clone(q_near_a));
    set_child(q_near_a, 2, Rc::clone(&p_i));
    set_child(&p_j, 0, Rc::clone(q_near_b));
    set_child(q_near_b, 2, Rc::clone(&p_j));
}

/// Wires the two reciprocal corner-to-corner links across the pair edge two
/// split parents shared via their J ports (`children[1]`). The shared base
/// edge has matching corner letters (`p.B = q.B`, `p.C = q.C`), so the corner
/// nodes J (near B) and K (near C) wire straight across.
fn wire_pair_edge(p_center: &NodeRef, q_center: &NodeRef) {
    let p_j = child(p_center, 0).expect("corner node J");
    let p_k = child(p_center, 2).expect("corner node K");
    let q_j = child(q_center, 0).expect("corner node J");
    let q_k = child(q_center, 2).expect("corner node K");
    set_child(&p_j, 1, Rc::clone(&q_j));
    set_child(&q_j, 1, Rc::clone(&p_j));
    set_child(&p_k, 1, Rc::clone(&q_k));
    set_child(&q_k, 1, Rc::clone(&p_k));
}

/// Collects every node reachable from `root` by following child links
/// (breadth-first, deduplicated by unique node name).
pub fn collect_nodes(root: &NodeRef) -> Vec<NodeRef> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::from([Rc::clone(root)]);
    let mut nodes = Vec::new();
    while let Some(node) = queue.pop_front() {
        if !visited.insert(node.borrow().name.clone()) {
            continue;
        }
        for child in node.borrow().children.iter().flatten() {
            queue.push_back(Rc::clone(child));
        }
        nodes.push(node);
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-3;
    const SIDE_LENGTH: f32 = 300.0;

    fn approx_eq(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < EPSILON
    }

    fn apothem() -> f32 {
        SIDE_LENGTH / (2.0 * (PI / 5.0).tan())
    }

    /// Walks the perimeter loop via `children[2]` from `root`, returning the 5
    /// base nodes in walk order.
    fn perimeter_loop(root: &NodeRef) -> Vec<NodeRef> {
        let mut loop_nodes = vec![Rc::clone(root)];
        for _ in 0..4 {
            let next = loop_nodes
                .last()
                .unwrap()
                .borrow()
                .children[2]
                .clone()
                .expect("next base node");
            loop_nodes.push(next);
        }
        loop_nodes
    }

    #[test]
    fn generate_base_creates_ten_level_zero_nodes() {
        let mut plan = Plan::new();
        let root = plan.generate_base("base_", SIDE_LENGTH, Vec2::Y, Vec2::ZERO, Labeling::Normal);

        let nodes = collect_nodes(&root);
        assert_eq!(nodes.len(), 10);
        for node in &nodes {
            assert_eq!(node.borrow().level, 0);
        }
        for i in 0..5 {
            assert!(nodes
                .iter()
                .any(|node| node.borrow().name == format!("base_base_node_{i}")));
            assert!(nodes
                .iter()
                .any(|node| node.borrow().name == format!("base_reverted_node_{i}")));
        }
        // rootNode state points at the first base node.
        assert!(Rc::ptr_eq(plan.root_node.as_ref().unwrap(), &root));
        assert_eq!(root.borrow().name, "base_base_node_0");
    }

    #[test]
    fn generate_base_base_apexes_converge_at_pentagon_center() {
        let center = Vec2::new(100.0, 200.0);
        let mut plan = Plan::new();
        let root = plan.generate_base("base_", SIDE_LENGTH, Vec2::new(1.0, 1.0), center, Labeling::Normal);

        // h = r, so every inward base_node apex lands on the pentagon center
        // and its base midpoint sits exactly one apothem away from it.
        for base_node in perimeter_loop(&root) {
            let base_node = base_node.borrow();
            assert!(approx_eq(base_node.points[0], center));
            let base_mid = (base_node.points[1] + base_node.points[2]) / 2.0;
            assert!(((base_mid - center).length() - apothem()).abs() < EPSILON);
        }
    }

    #[test]
    fn generate_base_pairs_share_base_edge_and_link_reciprocally() {
        let mut plan = Plan::new();
        let root = plan.generate_base("base_", SIDE_LENGTH, Vec2::Y, Vec2::ZERO, Labeling::Normal);

        for base_node in perimeter_loop(&root) {
            let reverted = base_node.borrow().children[1]
                .clone()
                .expect("paired reverted node");
            let base_points = base_node.borrow().points;
            let reverted_points = reverted.borrow().points;
            // The base edge coincides with the pair's base edge, with matching
            // corner letters: the reverted node is mirrored (B/C swapped), so
            // both nodes label the same pentagon vertex with the same letter.
            assert!(approx_eq(base_points[1], reverted_points[1]));
            assert!(approx_eq(base_points[2], reverted_points[2]));
            // Reciprocal pair invariant across children[1].
            assert!(Rc::ptr_eq(
                reverted.borrow().children[1].as_ref().unwrap(),
                &base_node
            ));
        }
    }

    #[test]
    fn generate_base_reverted_nodes_have_mirrored_i_and_k() {
        let mut plan = Plan::new();
        let root = plan.generate_base("base_", SIDE_LENGTH, Vec2::Y, Vec2::ZERO, Labeling::Normal);

        for base_node in perimeter_loop(&root) {
            let reverted = base_node.borrow().children[1].clone().unwrap();
            let base = base_node.borrow();
            let reverted = reverted.borrow();

            // The uniform direction rule still holds on the reverted node's
            // own (swapped) points: I ⊥ AB, J ⊥ BC, K ⊥ CA.
            let [a, b, c] = reverted.points;
            for (direction, edge_start, edge_end) in [
                (reverted.directions[0], a, b),
                (reverted.directions[1], b, c),
                (reverted.directions[2], c, a),
            ] {
                assert!(direction.dot(edge_end - edge_start).abs() < EPSILON);
                let edge_mid = (edge_start + edge_end) / 2.0;
                assert!(direction.dot(edge_mid - reverted.center) > 0.0);
            }

            // The pair's I arrows point toward the same pentagon vertex (B),
            // and so do the K arrows (toward C).
            assert!(base.directions[0].dot(base.points[1] - base.center) > 0.0);
            assert!(reverted.directions[0].dot(reverted.points[1] - reverted.center) > 0.0);
            assert!(base.directions[2].dot(base.points[2] - base.center) > 0.0);
            assert!(reverted.directions[2].dot(reverted.points[2] - reverted.center) > 0.0);
        }
    }

    #[test]
    fn generate_base_perimeter_loop_closes_after_five_hops() {
        let mut plan = Plan::new();
        let root = plan.generate_base("base_", SIDE_LENGTH, Vec2::Y, Vec2::ZERO, Labeling::Normal);

        // children[2] visits 5 distinct base nodes, then returns to the start.
        let loop_nodes = perimeter_loop(&root);
        for (i, node) in loop_nodes.iter().enumerate() {
            assert_eq!(node.borrow().name, format!("base_base_node_{}", (5 - i) % 5));
        }
        let back = loop_nodes[4].borrow().children[2].clone().unwrap();
        assert!(Rc::ptr_eq(&back, &root));

        // Reciprocity: children[0] of each node is the previous in walk order.
        for i in 0..5 {
            let previous = &loop_nodes[(i + 4) % 5];
            let child0 = loop_nodes[i].borrow().children[0].clone().unwrap();
            assert!(Rc::ptr_eq(&child0, previous));
        }
    }

    #[test]
    fn get_reverted_nodes_returns_inner_ring_in_circular_sequence() {
        let mut plan = Plan::new();
        let root = plan.generate_base("north_", SIDE_LENGTH, Vec2::Y, Vec2::ZERO, Labeling::Normal);

        // The children[2] walk goes base_0 -> base_4 -> base_3 -> ..., so the
        // paired reverted nodes come out in the same rotated order.
        let reverted = Plan::get_reverted_nodes(&root);
        for (node, suffix) in reverted.iter().zip([0, 4, 3, 2, 1]) {
            assert_eq!(
                node.borrow().name,
                format!("north_reverted_node_{suffix}")
            );
        }
    }

    #[test]
    fn generate_builds_twenty_node_dual_mesh() {
        let mut plan = Plan::new();
        let root = plan.generate(SIDE_LENGTH);

        let nodes = collect_nodes(&root);
        assert_eq!(nodes.len(), 20);
        assert_eq!(root.borrow().name, "north_base_node_0");
        assert!(Rc::ptr_eq(plan.root_node.as_ref().unwrap(), &root));
    }

    #[test]
    fn generate_saturates_every_child_port_reciprocally() {
        let mut plan = Plan::new();
        let root = plan.generate(SIDE_LENGTH);

        // Full mesh saturation: all 20 nodes have all 3 ports connected, with
        // port reciprocity 0 <-> 2 and 1 <-> 1 across every link.
        for node in collect_nodes(&root) {
            let node_ref = node.borrow();
            for (index, child) in node_ref.children.iter().enumerate() {
                let child = child.as_ref().unwrap_or_else(|| {
                    panic!("{} port {index} is unconnected", node_ref.name)
                });
                let reciprocal_index = match index {
                    0 => 2,
                    1 => 1,
                    2 => 0,
                    _ => unreachable!(),
                };
                let reciprocal = child.borrow().children[reciprocal_index]
                    .clone()
                    .expect("reciprocal port");
                assert!(
                    Rc::ptr_eq(&reciprocal, &node),
                    "{} port {index} is not reciprocated",
                    node_ref.name
                );
            }
        }
    }

    #[test]
    fn generate_wires_interlock_ports_per_connection_table() {
        let mut plan = Plan::new();
        let root = plan.generate(SIDE_LENGTH);
        let nodes = collect_nodes(&root);

        // Resolved connection table from the spec, indexed by getRevertedNodes
        // walk order (reverted_node_{0,4,3,2,1}): north node i connects port I
        // (children[0]) to south node (2 - i) mod 5 and port K (children[2])
        // to south node (3 - i) mod 5.
        let order = [0, 4, 3, 2, 1];
        let table = [(2, 3), (1, 2), (0, 1), (4, 0), (3, 4)];
        for (i, &(port_i, port_k)) in table.iter().enumerate() {
            let north = nodes
                .iter()
                .find(|node| node.borrow().name == format!("north_reverted_node_{}", order[i]))
                .unwrap();
            let child_i = north.borrow().children[0].clone().unwrap();
            let child_k = north.borrow().children[2].clone().unwrap();
            assert_eq!(
                child_i.borrow().name,
                format!("south_reverted_node_{}", order[port_i])
            );
            assert_eq!(
                child_k.borrow().name,
                format!("south_reverted_node_{}", order[port_k])
            );
        }
    }

    #[test]
    fn generate_offsets_south_base_without_rotation() {
        let mut plan = Plan::new();
        let root = plan.generate(SIDE_LENGTH);

        let north_center = Vec2::new(SIDE_LENGTH * 3.0, SIDE_LENGTH * 3.0);
        let south_center = north_center + Vec2::new(0.0, 4.0 * apothem());
        let nodes = collect_nodes(&root);
        for node in nodes
            .iter()
            .filter(|node| node.borrow().name.starts_with("south_base_node_"))
        {
            assert!(approx_eq(node.borrow().points[0], south_center));
        }
        // Same orientation for both bases: the south base is the north base
        // translated by the (0, 4r) center offset, but generated with mirrored
        // labeling — B/C (hence I/K) are swapped on every south node.
        let offset = Vec2::new(0.0, 4.0 * apothem());
        for i in 0..5 {
            let north = nodes
                .iter()
                .find(|node| node.borrow().name == format!("north_reverted_node_{i}"))
                .unwrap();
            let south = nodes
                .iter()
                .find(|node| node.borrow().name == format!("south_reverted_node_{i}"))
                .unwrap();
            let north_points = north.borrow().points;
            let south_points = south.borrow().points;
            assert!(approx_eq(north_points[0] + offset, south_points[0]));
            assert!(approx_eq(north_points[1] + offset, south_points[2]));
            assert!(approx_eq(north_points[2] + offset, south_points[1]));
            let north_directions = north.borrow().directions;
            let south_directions = south.borrow().directions;
            assert!(approx_eq(north_directions[0], south_directions[2]));
            assert!(approx_eq(north_directions[2], south_directions[0]));
        }
    }

    #[test]
    fn split_subdivides_all_twenty_nodes_into_level_one_nodes() {
        let mut plan = Plan::new();
        plan.generate(SIDE_LENGTH);
        plan.split();

        // 20 old nodes x 4 new nodes each; the old nodes are destroyed.
        let nodes = collect_nodes(plan.root_node.as_ref().unwrap());
        assert_eq!(nodes.len(), 80);
        for node in &nodes {
            assert_eq!(node.borrow().level, 1);
        }

        // The traversal visited every node exactly once: each original name
        // produced its three corner nodes (.I/.J/.K) and its center (.C).
        for prefix in [
            "north_base_node_",
            "north_reverted_node_",
            "south_base_node_",
            "south_reverted_node_",
        ] {
            for i in 0..5 {
                for suffix in [".I", ".J", ".K", ".C"] {
                    let name = format!("{prefix}{i}{suffix}");
                    assert!(
                        nodes.iter().any(|node| node.borrow().name == name),
                        "missing {name}"
                    );
                }
            }
        }

        // root_node is re-anchored on the first split center.
        assert_eq!(
            plan.root_node.as_ref().unwrap().borrow().name,
            "north_base_node_0.C"
        );
    }

    /// After a full run, every port of every node of the new level is
    /// connected exactly once, with port reciprocity (0 <-> 2, 1 <-> 1).
    #[test]
    fn split_wires_every_port_reciprocally() {
        let mut plan = Plan::new();
        plan.generate(SIDE_LENGTH);
        plan.split();
        let (open, one_way) = wiring_gaps(plan.root_node.as_ref().unwrap());
        assert!(
            open.is_empty() && one_way.is_empty(),
            "open ports ({}): {:?}; one-way links ({}): {:?}",
            open.len(),
            open,
            one_way.len(),
            one_way,
        );
    }

    /// Anchor checks for the three edge kinds: perimeter edges wire straight
    /// (matching corner letters), pair edges wire straight across the J
    /// ports, and belt edges wire crosswise across the interlock gap.
    #[test]
    fn split_wires_corner_pairs_across_each_edge_kind() {
        let mut plan = Plan::new();
        plan.generate(SIDE_LENGTH);
        plan.split();
        let nodes = collect_nodes(plan.root_node.as_ref().unwrap());
        let by_name = |name: &str| {
            nodes
                .iter()
                .find(|node| node.borrow().name == name)
                .cloned()
                .unwrap_or_else(|| panic!("missing {name}"))
        };
        let assert_linked = |a: &NodeRef, port_a: usize, b: &NodeRef, port_b: usize| {
            let forward = child(a, port_a).unwrap_or_else(|| panic!("{}[{port_a}] is open", a.borrow().name));
            assert!(
                Rc::ptr_eq(&forward, b),
                "{}[{port_a}] does not link to {}",
                a.borrow().name,
                b.borrow().name
            );
            let back = child(b, port_b).unwrap_or_else(|| panic!("{}[{port_b}] is open", b.borrow().name));
            assert!(
                Rc::ptr_eq(&back, a),
                "{}[{port_b}] does not link back to {}",
                b.borrow().name,
                a.borrow().name
            );
        };

        // Perimeter edge north_base_node_0 -> north_base_node_1 (coincident,
        // straight pairing I<->I, J<->K).
        assert_linked(&by_name("north_base_node_0.I"), 0, &by_name("north_base_node_1.I"), 2);
        assert_linked(&by_name("north_base_node_0.J"), 0, &by_name("north_base_node_1.K"), 2);

        // Pair edge north_base_node_0 <-> north_reverted_node_0 (straight
        // pairing J<->J, K<->K across the shared base edge).
        assert_linked(&by_name("north_base_node_0.J"), 1, &by_name("north_reverted_node_0.J"), 1);
        assert_linked(&by_name("north_base_node_0.K"), 1, &by_name("north_reverted_node_0.K"), 1);

        // Belt edge north_reverted_node_4 -> south_reverted_node_4 (crosswise
        // pairing I<->K, J<->I across the interlock gap).
        assert_linked(&by_name("north_reverted_node_4.I"), 0, &by_name("south_reverted_node_4.K"), 2);
        assert_linked(&by_name("north_reverted_node_4.J"), 0, &by_name("south_reverted_node_4.I"), 2);

        // Ring closes: north cap (straight) and belt (crosswise).
        assert_linked(&by_name("north_base_node_4.I"), 0, &by_name("north_base_node_0.I"), 2);
        assert_linked(&by_name("north_base_node_4.J"), 0, &by_name("north_base_node_0.K"), 2);
        assert_linked(&by_name("south_reverted_node_3.I"), 0, &by_name("north_reverted_node_4.K"), 2);
        assert_linked(&by_name("south_reverted_node_3.J"), 0, &by_name("north_reverted_node_4.I"), 2);
    }

    /// Repeated subdivision keeps the mesh fully wired and connected: at each
    /// level every node is reachable from the root and every port is
    /// reciprocally connected.
    #[test]
    fn repeated_splits_keep_mesh_fully_wired_and_connected() {
        let mut plan = Plan::new();
        plan.generate(SIDE_LENGTH);
        for level in 1..=3 {
            plan.split();
            let nodes = collect_nodes(plan.root_node.as_ref().unwrap());
            assert_eq!(nodes.len(), 20 * 4_usize.pow(level));
            for node in &nodes {
                assert_eq!(node.borrow().level, level);
            }
            let (open, one_way) = wiring_gaps(plan.root_node.as_ref().unwrap());
            assert!(
                open.is_empty() && one_way.is_empty(),
                "level {level}: open ports ({}): {:?}; one-way links ({}): {:?}",
                open.len(),
                open,
                one_way.len(),
                one_way,
            );
        }
    }

    /// Counts open ports and one-way (non-reciprocal) links across the whole
    /// mesh reachable from `root`, reported as `"<name>[<port>]"` strings.
    fn wiring_gaps(root: &NodeRef) -> (Vec<String>, Vec<String>) {
        let mut open = Vec::new();
        let mut one_way = Vec::new();
        for node in collect_nodes(root) {
            for index in 0..3 {
                let slot = node.borrow().children[index].clone();
                let Some(target) = slot else {
                    open.push(format!("{}[{index}]", node.borrow().name));
                    continue;
                };
                let back_index = match index {
                    0 => 2,
                    1 => 1,
                    _ => 0,
                };
                let reciprocal = target.borrow().children[back_index].clone();
                if !reciprocal.is_some_and(|r| Rc::ptr_eq(&r, &node)) {
                    one_way.push(format!("{}[{index}]", node.borrow().name));
                }
            }
        }
        (open, one_way)
    }
}
