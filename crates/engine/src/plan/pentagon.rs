//! Pentagonal base generation: the apothem geometry, the paired base/reverted
//! node ring, the perimeter-ring walk and the North/South interlock wiring.

use std::f32::consts::PI;
use std::rc::Rc;

use glam::Vec2;

use crate::node::topology::link;
use crate::node::{Labeling, Node, NodeRef};

/// Number of sides of the pentagonal base — hence of base/reverted node
/// pairs and of ports wired by the interlock.
pub(crate) const PENTAGON_SIDES: usize = 5;

/// Apothem of a regular pentagon with the given side length; also the node
/// height (h = r) of the base structure's triangles.
pub(crate) fn apothem(side_length: f32) -> f32 {
    side_length / (2.0 * (PI / PENTAGON_SIDES as f32).tan())
}

/// Constructs a pentagonal base structure.
///
/// The structure is composed of 5 paired nodes (10 nodes total):
/// - 5 inward-pointing `base_node`s whose apexes converge at `pentagon_center`.
/// - 5 outward-pointing `reverted_node`s.
///
/// Each pair shares a base edge flush with a pentagon side. The returned node
/// is the first generated base node; traversing `children[2]` visits all 5 base
/// nodes in a closed circular loop.
///
/// # Parameters
///
/// - `name` — name prefix of the base structure.
/// - `side_length` — length of each pentagon edge.
/// - `pentagon_direction` — initial orientation vector; used to rotate the
///   pentagon.
/// - `pentagon_center` — center coordinates of the pentagon.
/// - `labeling` — corner labeling convention for the `base_node`s. Each
///   `reverted_node` receives the opposite labeling so both nodes of a pair
///   label the same pentagon vertex with the same letter.
///
/// # Example
///
/// ```
/// use glam::Vec2;
/// use planet_crafter_engine::node::Labeling;
/// use planet_crafter_engine::plan::generate_base;
///
/// let root = generate_base("test_", 1.0, Vec2::Y, Vec2::ZERO, Labeling::Normal);
/// assert_eq!(root.borrow().name, "test_base_node_0");
/// ```
pub fn generate_base(
    name: &str,
    side_length: f32,
    pentagon_direction: Vec2,
    pentagon_center: Vec2,
    labeling: Labeling,
) -> NodeRef {
    let dir = pentagon_direction.normalize();
    // Apothem, node height (h = r) and centroid offset (c = h/3).
    let r = apothem(side_length);
    let height = r;
    let c = height / 3.0;

    let mut first_node: Option<NodeRef> = None;
    let mut previous_node: Option<NodeRef> = None;

    for i in 0..PENTAGON_SIDES {
        // Outward normal of pentagon side i.
        let theta = dir.y.atan2(dir.x) + i as f32 * (2.0 * PI / PENTAGON_SIDES as f32);
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
        link(&base_node, 1, &reverted_node, 1);

        // Sequential perimeter loop (child indices 0 & 2).
        if i == 0 {
            first_node = Some(Rc::clone(&base_node));
        }
        if let Some(previous) = &previous_node {
            link(&base_node, 2, previous, 0);
        }
        if i == PENTAGON_SIDES - 1 {
            // Close the circular perimeter loop with the first node.
            let first = first_node.as_ref().unwrap();
            link(first, 2, &base_node, 0);
        }
        previous_node = Some(base_node);
    }

    first_node.expect("the loop always runs 5 iterations")
}

/// Walks the perimeter loop via `children[2]` from `root`, returning the 5
/// base nodes in walk order (starting with `root`).
pub(crate) fn walk_perimeter(root: &NodeRef) -> [NodeRef; PENTAGON_SIDES] {
    let mut loop_nodes = Vec::with_capacity(PENTAGON_SIDES);
    loop_nodes.push(Rc::clone(root));
    for _ in 1..PENTAGON_SIDES {
        let next = loop_nodes.last().unwrap().borrow().children[2]
            .clone()
            .expect("next base node");
        loop_nodes.push(next);
    }
    loop_nodes
        .try_into()
        .unwrap_or_else(|_| panic!("the loop always collects {PENTAGON_SIDES} nodes"))
}

/// Extracts the 5 inner `reverted_node`s in circular sequence: the paired
/// inner node (`children[1]`) of each perimeter-ring base node, in walk
/// order.
pub(crate) fn reverted_nodes(root_base_node: &NodeRef) -> [NodeRef; PENTAGON_SIDES] {
    walk_perimeter(root_base_node).map(|base_node| {
        Rc::clone(
            base_node.borrow().children[1]
                .as_ref()
                .expect("paired reverted node"),
        )
    })
}

/// Wires the interlocking directional ports between the North and South
/// inner rings (reciprocal I <-> K links).
pub(crate) fn wire_interlock(
    north_reverted: &[NodeRef; PENTAGON_SIDES],
    south_reverted: &[NodeRef; PENTAGON_SIDES],
) {
    for (i, north_node) in north_reverted.iter().enumerate() {
        // North port I (index 0) to South port K (index 2), reflection
        // offset southIdx_I = (2 - i) mod 5 (the +5 keeps the unsigned
        // arithmetic non-negative).
        let south_idx_i = (2 + PENTAGON_SIDES - i) % PENTAGON_SIDES;
        link(north_node, 0, &south_reverted[south_idx_i], 2);

        // North port K (index 2) to South port I (index 0), reflection
        // offset southIdx_K = (3 - i) mod 5.
        let south_idx_k = (3 + PENTAGON_SIDES - i) % PENTAGON_SIDES;
        link(north_node, 2, &south_reverted[south_idx_k], 0);
    }
}
