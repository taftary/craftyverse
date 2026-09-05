//! Triangle subdivision.
//!
//! This module owns the operation that splits one [`Node`] into four
//! level-plus-one nodes and wires the new center node to its corners.

use super::Node;
use super::NodeRef;
use crate::node::geometry::{child_node, midpoint};
use crate::node::topology::link;

/// Splits `node` into four new nodes and returns the center node.
///
/// The four new nodes are `NodeI`, `NodeJ`, `NodeK`, and `NodeCenter`.
///
/// The center node is internally connected to each corner node through
/// reciprocal `children` links. The caller is responsible for wiring the
/// corner nodes to neighboring split centers across the subdivided edges.
///
/// Each new node derives its dimensions and orientation from its own point
/// triplet, and its [`level`](Node::level) is set to `node.level + 1`.
///
/// See `docs/book/specs/node.md` for the full geometric construction
/// and topology rules.
///
/// # Example
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::node::{split_node, Node};
///
/// let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
/// let center = split_node(&node.borrow());
/// assert_eq!(center.borrow().level, 1);
/// assert!(center.borrow().children[0].is_some());
/// ```
///
/// # Subdivision invariants
///
/// ```
/// use std::rc::Rc;
/// use glam::Vec3;
/// use planet_crafter_engine::node::{split_node, Node};
///
/// let node = Node::new("root", [Vec3::new(0.0, 4.0 / 3.0, 0.0), Vec3::new(1.0, -2.0 / 3.0, 0.0), Vec3::new(-1.0, -2.0 / 3.0, 0.0)], Vec3::ZERO);
/// let center = split_node(&node.borrow());
/// let center_ref = center.borrow();
///
/// assert_eq!(center_ref.level, 1);
/// assert_eq!(center_ref.base_length, 1.0);
/// assert_eq!(center_ref.height, 1.0);
/// assert!(center_ref.children.iter().all(|c| c.is_some()));
///
/// // Reciprocity: each corner links back to the center on the expected port.
/// assert!(Rc::ptr_eq(
///     center_ref.children[0].as_ref().unwrap().borrow().children[2].as_ref().unwrap(),
///     &center,
/// ));
/// assert!(Rc::ptr_eq(
///     center_ref.children[1].as_ref().unwrap().borrow().children[1].as_ref().unwrap(),
///     &center,
/// ));
/// assert!(Rc::ptr_eq(
///     center_ref.children[2].as_ref().unwrap().borrow().children[0].as_ref().unwrap(),
///     &center,
/// ));
/// ```
pub fn split_node(node: &Node) -> NodeRef {
    let old_level = node.level;
    // The node stores no origin; recover it from center + direction_to_origin.
    let origin = node.center + node.direction_to_origin;
    let [p_a, p_b, p_c] = node.points;

    // 1. Point midpoints.
    let p_ab = midpoint(p_a, p_b);
    let p_bc = midpoint(p_b, p_c);
    let p_ca = midpoint(p_c, p_a);

    // 2./3. New nodes from their subdivided points triplets (centers are
    // the centroids). Each child derives its own altitude and dimensions
    // from its point triplet. Names derive from `node.name` to stay unique.
    let level = old_level + 1;
    let node_i = child_node([p_a, p_ab, p_ca], origin, level, format!("{}.I", node.name));
    let node_j = child_node([p_ab, p_b, p_bc], origin, level, format!("{}.J", node.name));
    let node_k = child_node([p_ca, p_bc, p_c], origin, level, format!("{}.K", node.name));
    let node_center = child_node(
        [p_bc, p_ab, p_ca],
        origin,
        level,
        format!("{}.C", node.name),
    );

    // 4. Internal interconnection (bidirectional). Each center port is
    // linked to the corner node across its edge: center I (⊥ pBC–pAB)
    // faces node J, center J (⊥ pAB–pCA) faces node I, center K
    // (⊥ pCA–pBC) faces node K; the reciprocal corner port faces the
    // center the same way.
    link(&node_center, 0, &node_j, 2);
    link(&node_center, 1, &node_i, 1);
    link(&node_center, 2, &node_k, 0);

    node_center
}
