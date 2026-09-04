//! Mesh subdivision: `Plan::split` plus the edge-wiring helpers that
//! reconnect the fresh corner nodes across every subdivided edge.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use glam::Vec2;

use crate::node::topology::link;
use crate::node::{Node, NodeRef, collect_nodes};

use super::Plan;

impl Plan {
    /// Subdivides the whole mesh one level: splits every node of the current
    /// level once, reconnects the resulting split-centers across the
    /// subdivided edges, and destroys the old nodes. See
    /// `docs/book/specs/plan.md` section 7.
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
        let mut centers: HashMap<*const RefCell<Node>, NodeRef> =
            HashMap::with_capacity(old_nodes.len());
        for node in &old_nodes {
            centers.insert(Rc::as_ptr(node), node.borrow().split());
        }

        // Wire the new corner nodes across every old edge. Each edge appears
        // twice in the enumeration (once per endpoint); reciprocity (0 <-> 2,
        // 1 <-> 1) guarantees the canonicalization wires it exactly once.
        for node in &old_nodes {
            for index in 0..3 {
                let Some(neighbor) = child(node, index) else {
                    continue;
                };
                match index {
                    // 0 <-> 2 edges are wired from their port-0 side.
                    0 => wire_chain_edge(
                        &centers[&Rc::as_ptr(node)],
                        &centers[&Rc::as_ptr(&neighbor)],
                    ),
                    // 1 <-> 1 edges are wired from one canonical side.
                    1 if Rc::as_ptr(node) < Rc::as_ptr(&neighbor) => wire_pair_edge(
                        &centers[&Rc::as_ptr(node)],
                        &centers[&Rc::as_ptr(&neighbor)],
                    ),
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
}

/// Clone of the link at `index` on `node` (`None` when the port is open).
pub(crate) fn child(node: &NodeRef, index: usize) -> Option<NodeRef> {
    node.borrow().children[index].clone()
}

/// Tolerance for the edge-midpoint coincidence test, relative to the corner
/// node's base length. The two halves of a subdivided edge sit a quarter of
/// the parent edge apart; belt edges sit the whole interlock gap apart, so
/// any value well below 0.25 classifies correctly at every level.
const COINCIDENCE_TOLERANCE: f32 = 0.01;

/// Midpoint of the edge faced by port `index` (I ⊥ AB, J ⊥ BC, K ⊥ CA).
fn edge_midpoint(node: &NodeRef, index: usize) -> Vec2 {
    debug_assert!(index < 3);
    let points = node.borrow().points;
    let (a, b) = match index {
        0 => (points[0], points[1]),
        1 => (points[1], points[2]),
        2 => (points[2], points[0]),
        _ => unreachable!("port index is always < 3"),
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
    let (q_near_a, q_near_b) = if coincident {
        (&q_i, &q_k)
    } else {
        (&q_k, &q_i)
    };
    link(&p_i, 0, q_near_a, 2);
    link(&p_j, 0, q_near_b, 2);
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
    link(&p_j, 1, &q_j, 1);
    link(&p_k, 1, &q_k, 1);
}
