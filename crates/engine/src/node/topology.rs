//! Topological helpers for the `NodeRef` child-link graph: the reciprocal
//! port convention, bidirectional link wiring, graph traversal, and the
//! corner-weld lookups shared by sphere construction and mesh-level splits.

use std::collections::{HashSet, VecDeque};
use std::rc::Rc;

use glam::Vec3;

use super::NodeRef;

/// Reciprocal port of `port` under the split-link convention (`0 <-> 2`,
/// `1 <-> 1`): a link wired by a split through port `x` on one side uses port
/// `2 - x` on the other.
///
/// The convention holds on every edge of the welded icosphere (see
/// `icosphere`), but links still record their back-ports explicitly instead
/// of assuming this mapping, keeping the wiring and `destroy` exact for any
/// link, including hand-wired ones.
pub fn reciprocal_index(port: usize) -> usize {
    2 - port
}

/// Wires a bidirectional link between two nodes: sets `a.children[port_a]`
/// to `b` and `b.children[port_b]` to `a`, and records each side's
/// back-port so `destroy` can sever the link exactly.
///
/// Links created by splits and icosphere welds follow the reciprocal port
/// pattern `port_b == reciprocal_index(port_a)`; the back-port is recorded
/// rather than assumed so `link`/`destroy` stay exact for arbitrary port
/// pairs.
pub fn link(a: &NodeRef, port_a: usize, b: &NodeRef, port_b: usize) {
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
        .position(|corner| corner.borrow().vertices.contains(&endpoint))
        .expect("no corner child at the shared endpoint")
}

/// Port of `corner` on the edge from `endpoint` to the welded `midpoint`:
/// the port whose perpendicular edge has exactly those two endpoints.
pub(crate) fn port_on_edge(corner: &NodeRef, endpoint: Vec3, midpoint: Vec3) -> usize {
    let node = corner.borrow();
    let [a, b, c] = node.vertices;
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
