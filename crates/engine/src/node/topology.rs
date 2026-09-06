//! Topological helpers for the `NodeRef` child-link graph: the reciprocal
//! port convention, bidirectional link wiring, graph traversal, and the
//! corner-weld lookups shared by sphere construction and mesh-level splits.

use std::collections::{HashSet, VecDeque};
use std::rc::Rc;

use glam::Vec3;

use super::NodeRef;

/// Reciprocal port index of the historical `children` link convention: a
/// link on port `index` backed by port `reciprocal_index(index)` on the
/// neighbor — 0 ↔ 2 (I ↔ K) and 1 ↔ 1 (J ↔ J), mirroring the
/// interconnections established by `split_node`. This is a common pattern,
/// not an invariant: links carry an explicit `back_ports` record instead.
/// Only tests measure where the pattern holds.
#[cfg(test)]
pub(crate) fn reciprocal_index(index: usize) -> usize {
    2 - index
}

/// Wires a bidirectional link between two nodes: sets `a.children[port_a]`
/// to `b` and `b.children[port_b]` to `a`, and records each side's
/// back-port so `destroy` can sever the link exactly.
pub(crate) fn link(a: &NodeRef, port_a: usize, b: &NodeRef, port_b: usize) {
    a.borrow_mut().children[port_a] = Some(Rc::clone(b));
    a.borrow_mut().back_ports[port_a] = Some(port_b);
    b.borrow_mut().children[port_b] = Some(Rc::clone(a));
    b.borrow_mut().back_ports[port_b] = Some(port_a);
}

/// Returns the corner children `[I, J, K]` of a freshly split center node,
/// recovered through the center's port layout (`center.1 -> I`,
/// `center.0 -> J`, `center.2 -> K`).
pub(crate) fn corner_nodes(center: &NodeRef) -> [NodeRef; 3] {
    let node = center.borrow();
    let get = |port: usize| {
        Rc::clone(
            node.children[port]
                .as_ref()
                .expect("freshly split center node is not fully linked"),
        )
    };
    [get(1), get(0), get(2)]
}

/// Index (into `[I, J, K]`) of the corner child containing the vertex
/// `endpoint`. Corner `I` holds the parent's `A`, `J` holds `B`, `K` holds
/// `C`; positions are bit-identical across welded neighbors, so exact
/// comparison is correct here.
pub(crate) fn corner_near(corners: &[NodeRef; 3], endpoint: Vec3) -> usize {
    corners
        .iter()
        .position(|corner| corner.borrow().points.contains(&endpoint))
        .expect("no corner child at the shared endpoint")
}

/// Port of `corner` on the edge from `endpoint` to the welded `midpoint`:
/// the port whose perpendicular edge has exactly those two endpoints.
pub(crate) fn port_on_edge(corner: &NodeRef, endpoint: Vec3, midpoint: Vec3) -> usize {
    let node = corner.borrow();
    let [a, b, c] = node.points;
    [(a, b), (b, c), (c, a)]
        .iter()
        .position(|(u, v)| (*u == endpoint && *v == midpoint) || (*u == midpoint && *v == endpoint))
        .expect("corner has no port on the shared edge")
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

/// Destroys every node reachable from `root`, breaking the reciprocal-link
/// cycles of the mesh so it can deallocate.
///
/// Meshes built by [`build_icosphere`](super::build_icosphere) or
/// [`split_nodes`](super::split_nodes) are `Rc` cycle graphs: dropping them
/// without severing the links leaks every node. This is the prescribed
/// cleanup: collect the whole component with [`collect_nodes`], then
/// [`destroy`](super::Node::destroy) each node.
///
/// References kept to the nodes point at unlinked nodes afterwards.
///
/// # Example
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::node::{Node, destroy_mesh, split_nodes};
///
/// let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
/// let leaves = split_nodes(&node);
/// destroy_mesh(&leaves[0]);
/// assert!(leaves.iter().all(|leaf| leaf.borrow().children.iter().all(|c| c.is_none())));
/// ```
pub fn destroy_mesh(root: &NodeRef) {
    for node in collect_nodes(root) {
        node.borrow_mut().destroy();
    }
}
