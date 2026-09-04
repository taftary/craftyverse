use std::rc::Rc;

use glam::Vec2;

use super::Plan;
use super::pentagon::{apothem, generate_base, reverted_nodes, walk_perimeter};
use super::subdivide::child;
use crate::node::{Labeling, NodeRef, collect_nodes};

const EPSILON: f32 = 1e-3;
const SIDE_LENGTH: f32 = 300.0;

fn approx_eq(a: Vec2, b: Vec2) -> bool {
    (a - b).length() < EPSILON
}

/// Walks the perimeter loop via `children[2]` from `root`, returning the 5
/// base nodes in walk order.
fn perimeter_loop(root: &NodeRef) -> Vec<NodeRef> {
    walk_perimeter(root).into()
}

#[test]
fn generate_base_creates_ten_level_zero_nodes() {
    let mut plan = Plan::default();
    let root = generate_base("base_", SIDE_LENGTH, Vec2::Y, Vec2::ZERO, Labeling::Normal);
    plan.root_node = Some(Rc::clone(&root));

    let nodes = collect_nodes(&root);
    assert_eq!(nodes.len(), 10);
    for node in &nodes {
        assert_eq!(node.borrow().level, 0);
    }
    for i in 0..5 {
        assert!(
            nodes
                .iter()
                .any(|node| node.borrow().name == format!("base_base_node_{i}"))
        );
        assert!(
            nodes
                .iter()
                .any(|node| node.borrow().name == format!("base_reverted_node_{i}"))
        );
    }
    // rootNode state points at the first base node.
    assert!(Rc::ptr_eq(plan.root_node.as_ref().unwrap(), &root));
    assert_eq!(root.borrow().name, "base_base_node_0");
}

#[test]
fn generate_base_base_apexes_converge_at_pentagon_center() {
    let center = Vec2::new(100.0, 200.0);
    let root = generate_base(
        "base_",
        SIDE_LENGTH,
        Vec2::new(1.0, 1.0),
        center,
        Labeling::Normal,
    );

    // h = r, so every inward base_node apex lands on the pentagon center
    // and its base midpoint sits exactly one apothem away from it.
    for base_node in perimeter_loop(&root) {
        let base_node = base_node.borrow();
        assert!(approx_eq(base_node.points[0], center));
        let base_mid = (base_node.points[1] + base_node.points[2]) / 2.0;
        assert!(((base_mid - center).length() - apothem(SIDE_LENGTH)).abs() < EPSILON);
    }
}

#[test]
fn generate_base_pairs_share_base_edge_and_link_reciprocally() {
    let root = generate_base("base_", SIDE_LENGTH, Vec2::Y, Vec2::ZERO, Labeling::Normal);

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
    let root = generate_base("base_", SIDE_LENGTH, Vec2::Y, Vec2::ZERO, Labeling::Normal);

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
    let root = generate_base("base_", SIDE_LENGTH, Vec2::Y, Vec2::ZERO, Labeling::Normal);

    // children[2] visits 5 distinct base nodes, then returns to the start.
    let loop_nodes = perimeter_loop(&root);
    for (i, node) in loop_nodes.iter().enumerate() {
        assert_eq!(
            node.borrow().name,
            format!("base_base_node_{}", (5 - i) % 5)
        );
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
    let root = generate_base("north_", SIDE_LENGTH, Vec2::Y, Vec2::ZERO, Labeling::Normal);

    // The children[2] walk goes base_0 -> base_4 -> base_3 -> ..., so the
    // paired reverted nodes come out in the same rotated order.
    let reverted = reverted_nodes(&root);
    for (node, suffix) in reverted.iter().zip([0, 4, 3, 2, 1]) {
        assert_eq!(node.borrow().name, format!("north_reverted_node_{suffix}"));
    }
}

#[test]
fn generate_builds_twenty_node_dual_mesh() {
    let mut plan = Plan::default();
    let root = plan.generate(SIDE_LENGTH);

    let nodes = collect_nodes(&root);
    assert_eq!(nodes.len(), 20);
    assert_eq!(root.borrow().name, "north_base_node_0");
    assert!(Rc::ptr_eq(plan.root_node.as_ref().unwrap(), &root));
}

#[test]
fn generate_saturates_every_child_port_reciprocally() {
    let mut plan = Plan::default();
    let root = plan.generate(SIDE_LENGTH);

    // Full mesh saturation: all 20 nodes have all 3 ports connected, with
    // port reciprocity 0 <-> 2 and 1 <-> 1 across every link.
    for node in collect_nodes(&root) {
        let node_ref = node.borrow();
        for (index, child) in node_ref.children.iter().enumerate() {
            let child = child
                .as_ref()
                .unwrap_or_else(|| panic!("{} port {index} is unconnected", node_ref.name));
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
    let mut plan = Plan::default();
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
    let mut plan = Plan::default();
    let root = plan.generate(SIDE_LENGTH);

    let north_center = Vec2::new(SIDE_LENGTH * 3.0, SIDE_LENGTH * 3.0);
    let south_center = north_center + Vec2::new(0.0, 4.0 * apothem(SIDE_LENGTH));
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
    let offset = Vec2::new(0.0, 4.0 * apothem(SIDE_LENGTH));
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
    let mut plan = Plan::default();
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
    let mut plan = Plan::default();
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
    let mut plan = Plan::default();
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
        let forward =
            child(a, port_a).unwrap_or_else(|| panic!("{}[{port_a}] is open", a.borrow().name));
        assert!(
            Rc::ptr_eq(&forward, b),
            "{}[{port_a}] does not link to {}",
            a.borrow().name,
            b.borrow().name
        );
        let back =
            child(b, port_b).unwrap_or_else(|| panic!("{}[{port_b}] is open", b.borrow().name));
        assert!(
            Rc::ptr_eq(&back, a),
            "{}[{port_b}] does not link back to {}",
            b.borrow().name,
            a.borrow().name
        );
    };

    // Perimeter edge north_base_node_0 -> north_base_node_1 (coincident,
    // straight pairing I<->I, J<->K).
    assert_linked(
        &by_name("north_base_node_0.I"),
        0,
        &by_name("north_base_node_1.I"),
        2,
    );
    assert_linked(
        &by_name("north_base_node_0.J"),
        0,
        &by_name("north_base_node_1.K"),
        2,
    );

    // Pair edge north_base_node_0 <-> north_reverted_node_0 (straight
    // pairing J<->J, K<->K across the shared base edge).
    assert_linked(
        &by_name("north_base_node_0.J"),
        1,
        &by_name("north_reverted_node_0.J"),
        1,
    );
    assert_linked(
        &by_name("north_base_node_0.K"),
        1,
        &by_name("north_reverted_node_0.K"),
        1,
    );

    // Belt edge north_reverted_node_4 -> south_reverted_node_4 (crosswise
    // pairing I<->K, J<->I across the interlock gap).
    assert_linked(
        &by_name("north_reverted_node_4.I"),
        0,
        &by_name("south_reverted_node_4.K"),
        2,
    );
    assert_linked(
        &by_name("north_reverted_node_4.J"),
        0,
        &by_name("south_reverted_node_4.I"),
        2,
    );

    // Ring closes: north cap (straight) and belt (crosswise).
    assert_linked(
        &by_name("north_base_node_4.I"),
        0,
        &by_name("north_base_node_0.I"),
        2,
    );
    assert_linked(
        &by_name("north_base_node_4.J"),
        0,
        &by_name("north_base_node_0.K"),
        2,
    );
    assert_linked(
        &by_name("south_reverted_node_3.I"),
        0,
        &by_name("north_reverted_node_4.K"),
        2,
    );
    assert_linked(
        &by_name("south_reverted_node_3.J"),
        0,
        &by_name("north_reverted_node_4.I"),
        2,
    );
}

/// Repeated subdivision keeps the mesh fully wired and connected: at each
/// level every node is reachable from the root and every port is
/// reciprocally connected.
#[test]
fn repeated_splits_keep_mesh_fully_wired_and_connected() {
    let mut plan = Plan::default();
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
