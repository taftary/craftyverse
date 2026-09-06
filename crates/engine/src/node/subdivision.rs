//! Triangle subdivision.
//!
//! This module owns the operations that split one [`Node`] into four
//! level-plus-one nodes and wire the new center node to its corners
//! ([`split_node`]), split a whole connected mesh one generation deeper
//! ([`split_nodes`]), and merge a split generation back into its parents
//! ([`unsplit_nodes`]).

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use glam::Vec3;

use super::Node;
use super::NodeRef;
use crate::node::geometry::{child_node, midpoint, triangle_points};
use crate::node::topology::{
    collect_nodes, corner_near, corner_nodes, link, port_on_edge, reciprocal_index,
};

/// Splits `node` into four new nodes and returns the center node.
///
/// The four new nodes are `NodeI`, `NodeJ`, `NodeK`, and `NodeCenter`.
///
/// The center node is internally connected to each corner node through
/// reciprocal `children` links. The caller is responsible for wiring the
/// corner nodes to neighboring split centers across the subdivided edges.
///
/// Each new node derives its orientation from its own vertex triplet, and its
/// [`level`](Node::level) is set to `node.level + 1`.
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
    let [p_a, p_b, p_c] = node.vertices;

    // 1. Vertex midpoints.
    let p_ab = midpoint(p_a, p_b);
    let p_bc = midpoint(p_b, p_c);
    let p_ca = midpoint(p_c, p_a);

    split_node_with_midpoints(node, [p_ab, p_bc, p_ca])
}

/// Splits `node` into four new nodes using caller-supplied edge midpoints.
///
/// Behaves exactly like [`split_node`], except the three edge midpoints
/// `[p_ab, p_bc, p_ca]` are provided by the caller instead of being computed
/// as flat linear interpolations. Sphere construction uses this to inject
/// midpoints projected onto the sphere surface.
pub(crate) fn split_node_with_midpoints(node: &Node, midpoints: [Vec3; 3]) -> NodeRef {
    let old_level = node.level;
    // The node stores no origin; recover it from center + direction_to_origin.
    let origin = node.center + node.direction_to_origin;

    // 2./3. New nodes from their subdivided vertex triplets (centers are the
    // centroids). Each child derives its own orientation from its vertex
    // triplet. Names derive from `node.name` to stay unique.
    let level = old_level + 1;
    let [vertices_i, vertices_j, vertices_k, vertices_center] =
        triangle_points(&node.vertices, midpoints);
    let node_i = child_node(vertices_i, origin, level, format!("{}.I", node.name));
    let node_j = child_node(vertices_j, origin, level, format!("{}.J", node.name));
    let node_k = child_node(vertices_k, origin, level, format!("{}.K", node.name));
    let node_center = child_node(vertices_center, origin, level, format!("{}.C", node.name));

    // 4. Internal interconnection (bidirectional). Each center port is
    // linked to the corner node across its edge: center I (⊥ pBC–pAB)
    // faces node J, center J (⊥ pAB–pCA) faces node I, center K
    // (⊥ pCA–pBC) faces node K; the reciprocal corner port faces the
    // center the same way.
    link(&node_center, 0, &node_j, reciprocal_index(0));
    link(&node_center, 1, &node_i, reciprocal_index(1));
    link(&node_center, 2, &node_k, reciprocal_index(2));

    node_center
}

/// Splits every node reachable from `first` into four new nodes, welds the
/// new corner nodes across the old edges, destroys the old nodes, and
/// returns the new leaf set.
///
/// This is the mesh-level counterpart of [`split_node`]: where `split_node`
/// refines one triangle and leaves the cross-edge wiring to the caller,
/// `split_nodes` refines the whole connected component at once and restores
/// the neighbor links on the new generation. For each old link
/// `N.port p <-> M.port q`, the two half-edges of the shared edge are welded
/// corner-to-corner; the ports are resolved geometrically with exact vertex
/// comparison (corner vertices and flat edge midpoints are bit-identical on
/// both sides of a shared edge).
///
/// Open ports stay open. Nodes not reachable from `first` are untouched.
/// The old nodes are destroyed (their links severed) before returning; any
/// reference kept to them points at an unlinked node.
///
/// Returns `[I, J, K, C]` per old node, in old-node order.
///
/// # Panics
///
/// Panics when the two sides of a shared edge do not hold bit-identical
/// vertices: the weld lookups compare corner vertices and edge midpoints
/// with exact `==`. Meshes produced by `split_nodes` and
/// [`build_icosphere`](crate::node::build_icosphere) satisfy this by
/// construction.
///
/// # Example
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::node::{Node, split_nodes};
///
/// let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
/// let leaves = split_nodes(&node);
/// assert_eq!(leaves.len(), 4);
/// assert!(leaves.iter().all(|leaf| leaf.borrow().level == 1));
/// // The old node is destroyed: all its links are gone.
/// assert!(node.borrow().children.iter().all(|c| c.is_none()));
/// ```
///
/// A closed mesh stays closed:
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::node::{build_icosphere, split_nodes};
///
/// let mesh = build_icosphere("planet", 300.0, 0, Vec3::ZERO);
/// let leaves = split_nodes(&mesh.faces[0]);
/// assert_eq!(leaves.len(), 80);
/// assert!(leaves.iter().all(|leaf| leaf.borrow().children.iter().all(|c| c.is_some())));
/// # for leaf in &leaves { leaf.borrow_mut().destroy(); }
/// ```
pub fn split_nodes(first: &NodeRef) -> Vec<NodeRef> {
    let old = collect_nodes(first);

    // Split every old node; remember its corner children for the weld pass.
    let mut corners_of: Vec<[NodeRef; 3]> = Vec::with_capacity(old.len());
    let mut leaves = Vec::with_capacity(4 * old.len());
    for node in &old {
        let center = split_node(&node.borrow());
        let corners = corner_nodes(&center);
        leaves.extend(corners.iter().cloned());
        leaves.push(center);
        corners_of.push(corners);
    }

    // Weld the new corners across every old link, once per old edge.
    let index_of: HashMap<usize, usize> = old
        .iter()
        .enumerate()
        .map(|(index, node)| (Rc::as_ptr(node) as usize, index))
        .collect();
    for (index, node) in old.iter().enumerate() {
        let old_node = node.borrow();
        for port in 0..3 {
            let Some(neighbor) = old_node.children[port].as_ref() else {
                continue;
            };
            let neighbor_index = index_of[&(Rc::as_ptr(neighbor) as usize)];
            if neighbor_index <= index {
                continue;
            }
            let endpoints = [old_node.vertices[port], old_node.vertices[(port + 1) % 3]];
            let edge_midpoint = midpoint(endpoints[0], endpoints[1]);
            for endpoint in endpoints {
                let near = &corners_of[index][corner_near(&corners_of[index], endpoint)];
                let other =
                    &corners_of[neighbor_index][corner_near(&corners_of[neighbor_index], endpoint)];
                link(
                    near,
                    port_on_edge(near, endpoint, edge_midpoint),
                    other,
                    port_on_edge(other, endpoint, edge_midpoint),
                );
            }
        }
    }

    // Break the old generation's reciprocal cycles so it can deallocate.
    for node in &old {
        node.borrow_mut().destroy();
    }
    leaves
}

/// Merges every complete split group reachable from `first` back into its
/// parent node — the reverse of [`split_nodes`].
///
/// A *split group* is a set of four nodes named `"{base}.I"`, `"{base}.J"`,
/// `"{base}.K"`, `"{base}.C"` at the same level `>= 1`, as produced by
/// [`split_node`]. Each complete group is replaced by its parent: the
/// parent's vertices are recovered from the corners (`I` holds `A`,
/// `J` holds `B`, `K` holds `C` — the exact original vertices, even when a
/// sphere build projected the midpoints), its name is the group base name,
/// and its level is the group level minus one. The parents are then
/// re-linked across the old edges: a corner's external port number equals
/// its parent edge's port number, so every link between corners of
/// different groups maps verbatim to a parent link with the recorded
/// back-port.
///
/// Nodes that are not part of a complete split group (a base mesh, or a
/// mesh that was never split) are kept unchanged, so calling this on an
/// unsplittable mesh returns the same nodes. A base name whose suffix
/// appears twice (a name collision, not a split group) keeps its whole
/// group unchanged. The children of merged groups are destroyed;
/// references kept to them point at unlinked nodes. Links from kept nodes
/// into a merged group are re-targeted to the surviving parent: the kept
/// node keeps its port, and the parent inherits the merged corner's
/// external port — the parent edge's port number.
///
/// Returns the new parents in group discovery order, followed by the
/// unchanged nodes.
///
/// # Example
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::node::{Node, split_nodes, unsplit_nodes};
///
/// let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
/// let leaves = split_nodes(&node);
/// let parents = unsplit_nodes(&leaves[0]);
/// assert_eq!(parents.len(), 1);
/// let parent = parents[0].borrow();
/// assert_eq!(parent.name, "root");
/// assert_eq!(parent.level, 0);
/// ```
pub fn unsplit_nodes(first: &NodeRef) -> Vec<NodeRef> {
    let current = collect_nodes(first);

    // Group by base name (the name without its split suffix).
    let mut groups: HashMap<String, [Option<NodeRef>; 4]> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut kept: Vec<NodeRef> = Vec::new();
    let mut collided: HashSet<String> = HashSet::new();
    for node in &current {
        let suffix = split_suffix(&node.borrow().name);
        match suffix {
            Some((base, slot)) => {
                let group = groups.entry(base.clone()).or_insert_with(|| {
                    order.push(base.clone());
                    [None, None, None, None]
                });
                // A duplicate suffix is a name collision, not a split
                // group: the whole group is kept unchanged.
                if group[slot].is_none() {
                    group[slot] = Some(Rc::clone(node));
                } else {
                    collided.insert(base);
                    kept.push(Rc::clone(node));
                }
            }
            None => kept.push(Rc::clone(node)),
        }
    }

    // Rebuild a parent for each complete, single-level group.
    let mut parent_of: HashMap<usize, NodeRef> = HashMap::new();
    let mut merged: Vec<NodeRef> = Vec::new();
    let mut parents: Vec<NodeRef> = Vec::new();
    for base in order {
        let group = groups.remove(&base).expect("group recorded in order");
        if collided.contains(&base) {
            kept.extend(group.into_iter().flatten());
            continue;
        }
        let level = group
            .iter()
            .flatten()
            .next()
            .map(|node| node.borrow().level);
        let complete = group.iter().all(Option::is_some)
            && group
                .iter()
                .flatten()
                .all(|node| node.borrow().level == level.unwrap_or(0));
        if !complete || level.unwrap_or(0) == 0 {
            kept.extend(group.into_iter().flatten());
            continue;
        }
        let [node_i, node_j, node_k, node_c] = group.map(Option::unwrap);
        debug_assert!(Rc::ptr_eq(
            node_c.borrow().children[1]
                .as_ref()
                .expect("split group center"),
            &node_i
        ));
        debug_assert!(Rc::ptr_eq(
            node_c.borrow().children[0]
                .as_ref()
                .expect("split group center"),
            &node_j
        ));
        debug_assert!(Rc::ptr_eq(
            node_c.borrow().children[2]
                .as_ref()
                .expect("split group center"),
            &node_k
        ));
        let (origin, level) = {
            let node = node_i.borrow();
            (node.center + node.direction_to_origin, node.level - 1)
        };
        let vertices = [
            node_i.borrow().vertices[0],
            node_j.borrow().vertices[1],
            node_k.borrow().vertices[2],
        ];
        let parent = child_node(vertices, origin, level, base);
        for child in [&node_i, &node_j, &node_k, &node_c] {
            parent_of.insert(Rc::as_ptr(child) as usize, Rc::clone(&parent));
            merged.push(Rc::clone(child));
        }
        parents.push(parent);
    }

    // Re-link the parents across every cross-group corner link. A corner's
    // external port number equals its parent edge's port number, and each
    // parent edge is reached twice (once per half-edge) with the same
    // ports, so an already-linked parent port is simply skipped. Links
    // toward kept nodes are collected and re-targeted to the surviving
    // parent after the merged children are destroyed.
    let mut kept_links: Vec<(NodeRef, usize, NodeRef, usize)> = Vec::new();
    for child in &merged {
        let node = child.borrow();
        let parent = Rc::clone(&parent_of[&(Rc::as_ptr(child) as usize)]);
        for port in 0..3 {
            let (Some(neighbor), Some(back)) = (&node.children[port], node.back_ports[port]) else {
                continue;
            };
            let Some(other) = parent_of.get(&(Rc::as_ptr(neighbor) as usize)) else {
                // The neighbor was not merged: re-target its port to the
                // surviving parent (see below).
                kept_links.push((Rc::clone(&parent), port, Rc::clone(neighbor), back));
                continue;
            };
            if Rc::ptr_eq(&parent, other) || parent.borrow().children[port].is_some() {
                continue;
            }
            link(&parent, port, other, back);
        }
    }

    // Break the merged groups' reciprocal cycles so they can deallocate.
    for child in &merged {
        child.borrow_mut().destroy();
    }

    // Re-link the kept neighbors to the surviving parents on the ports the
    // merged corners used. This must happen after the `destroy` pass above:
    // that pass severs the old links, and re-linking earlier would let it
    // clear the new ones.
    for (parent, port, neighbor, back) in kept_links {
        if parent.borrow().children[port].is_none() {
            link(&parent, port, &neighbor, back);
        }
    }
    parents.extend(kept);
    parents
}

/// Splits `name` into its base and group slot (`I` = 0, `J` = 1, `K` = 2,
/// `C` = 3) when it ends with a [`split_node`] suffix (`.I`/`.J`/`.K`/`.C`).
fn split_suffix(name: &str) -> Option<(String, usize)> {
    let (base, suffix) = name.rsplit_once('.')?;
    let slot = match suffix {
        "I" => 0,
        "J" => 1,
        "K" => 2,
        "C" => 3,
        _ => return None,
    };
    Some((base.to_string(), slot))
}
