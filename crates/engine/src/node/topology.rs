//! Topological helpers for the `NodeRef` child-link graph: the reciprocal
//! port convention, bidirectional link wiring and graph traversal.

use std::collections::{HashSet, VecDeque};
use std::rc::Rc;

use super::NodeRef;

/// Reciprocal port index of the `children` link convention: a link on port
/// `index` is backed by port `reciprocal_index(index)` on the neighbor —
/// 0 ↔ 2 (I ↔ K) and 1 ↔ 1 (J ↔ J), mirroring the interconnections
/// established by `split_node`.
pub(crate) fn reciprocal_index(index: usize) -> usize {
    2 - index
}

/// Wires a bidirectional link between two nodes: sets `a.children[port_a]`
/// to `b` and `b.children[port_b]` to `a`.
pub(crate) fn link(a: &NodeRef, port_a: usize, b: &NodeRef, port_b: usize) {
    a.borrow_mut().children[port_a] = Some(Rc::clone(b));
    b.borrow_mut().children[port_b] = Some(Rc::clone(a));
}

/// Collects every node reachable from `root` by following child links.
///
/// Traversal is breadth-first and deduplicated by pointer identity, so each
/// `NodeRef` appears exactly once even when the mesh contains cycles. The
/// returned vector is ordered by discovery distance from `root`.
///
/// # Example
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::node::{collect_nodes, split_node, Node};
///
/// let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
/// let center = split_node(&node.borrow());
/// let all = collect_nodes(&center);
/// assert_eq!(all.len(), 4);
/// ```
pub fn collect_nodes(root: &NodeRef) -> Vec<NodeRef> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::from([Rc::clone(root)]);
    let mut nodes = Vec::new();
    while let Some(node) = queue.pop_front() {
        if !visited.insert(Rc::as_ptr(&node)) {
            continue;
        }
        for child in node.borrow().children.iter().flatten() {
            queue.push_back(Rc::clone(child));
        }
        nodes.push(node);
    }
    nodes
}
