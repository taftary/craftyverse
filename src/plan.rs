//! The `Plan` class: central manager of the spatial hierarchy. Encapsulates a
//! primary root node and orchestrates pentagonal base generation and the
//! dual-pentagon interlocked mesh. See `docs/classes-definitions/plan.md`.

use std::collections::{HashSet, VecDeque};
use std::f32::consts::PI;
use std::rc::Rc;

use glam::Vec2;

use crate::node::{Node, NodeRef};

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
    pub fn generate_base(
        &mut self,
        name: &str,
        side_length: f32,
        pentagon_direction: Vec2,
        pentagon_center: Vec2,
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

            let base_node = Node::new(
                direction_to_center,
                base_centroid,
                pentagon_center,
                side_length,
                height,
                format!("{name}base_node_{i}"),
            );
            let reverted_node = Node::new(
                outward_normal,
                reverted_centroid,
                pentagon_center,
                side_length,
                height,
                format!("{name}reverted_node_{i}"),
            );
            // Mirror the reverted node across the shared base edge: swap B/C
            // (hence I/K) so both nodes of a pair label the same pentagon
            // vertex with the same letter — the pair's I arrows point toward
            // the same vertex, and so do the K arrows.
            {
                let mut reverted = reverted_node.borrow_mut();
                reverted.points.swap(1, 2);
                reverted.directions.swap(0, 2);
            }

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
    /// and keeps the same orientation as the North base. Returns the primary
    /// root node (North base root).
    ///
    /// - `side_length` — length of each pentagon edge.
    pub fn generate(&mut self, side_length: f32) -> NodeRef {
        let north_center = Vec2::new(side_length * 3.0, side_length * 3.0);
        let north_dir = Vec2::Y;
        let r = side_length / (2.0 * (PI / 5.0).tan());
        let south_center = north_center + Vec2::new(0.0, 4.0 * r);

        let north_root = self.generate_base("north_", side_length, north_dir, north_center);
        let south_root = self.generate_base("south_", side_length, north_dir, south_center);

        let north_reverted = Self::get_reverted_nodes(&north_root);
        let south_reverted = Self::get_reverted_nodes(&south_root);

        // Wire the interlocking directional ports (reciprocal I <-> K links).
        for i in 0..5 {
            let north_node = &north_reverted[i];

            // North port I (index 0) to South port K (index 2).
            let target_south_node_k = &south_reverted[(i + 2) % 5];
            north_node.borrow_mut().children[0] = Some(Rc::clone(target_south_node_k));
            target_south_node_k.borrow_mut().children[2] = Some(Rc::clone(north_node));

            // North port K (index 2) to South port I (index 0).
            let target_south_node_i = &south_reverted[(i + 3) % 5];
            north_node.borrow_mut().children[2] = Some(Rc::clone(target_south_node_i));
            target_south_node_i.borrow_mut().children[0] = Some(Rc::clone(north_node));
        }

        self.root_node = Some(Rc::clone(&north_root));
        north_root
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
        let root = plan.generate_base("base_", SIDE_LENGTH, Vec2::Y, Vec2::ZERO);

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
        let root = plan.generate_base("base_", SIDE_LENGTH, Vec2::new(1.0, 1.0), center);

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
        let root = plan.generate_base("base_", SIDE_LENGTH, Vec2::Y, Vec2::ZERO);

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
        let root = plan.generate_base("base_", SIDE_LENGTH, Vec2::Y, Vec2::ZERO);

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
        let root = plan.generate_base("base_", SIDE_LENGTH, Vec2::Y, Vec2::ZERO);

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
        let root = plan.generate_base("north_", SIDE_LENGTH, Vec2::Y, Vec2::ZERO);

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
        // translated by the (0, 4r) center offset.
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
            for (north_point, south_point) in north_points.iter().zip(south_points.iter()) {
                assert!(approx_eq(*north_point + offset, *south_point));
            }
        }
    }
}
