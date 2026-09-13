//! Triangle subdivision.
//!
//! This module owns the operations that split one [`Node`] into four
//! level-plus-one nodes and wire the new center node to its corners
//! ([`split_node`]), split a whole connected mesh one generation deeper
//! ([`split_nodes`]), and merge a split generation back into its parents
//! ([`unsplit_nodes`]). The local runtime operations [`split_node_local`]
//! and [`unsplit_node`] refine or coarsen exactly one chunk of a live mesh,
//! retargeting the neighboring links that pointed at the replaced nodes.

use std::collections::{HashMap, HashSet, VecDeque};
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
    // triplet. Names derive from `node.name` to stay unique. UVs are split
    // with the same barycentric pattern, but their midpoints are always flat
    // linear interpolations — the sphere projection of the 3D midpoints does
    // not apply to texture space. The ring field follows the UV pattern
    // (flat midpoints; a documented approximation of the true distance
    // field — see the `ring` module). Parity propagates topologically:
    // corner children inherit the parent parity, the center child flips it.
    let level = old_level + 1;
    let [vertices_i, vertices_j, vertices_k, vertices_center] =
        triangle_points(&node.vertices, midpoints);
    let uv_midpoints = [
        midpoint(node.uv[0], node.uv[1]),
        midpoint(node.uv[1], node.uv[2]),
        midpoint(node.uv[2], node.uv[0]),
    ];
    let [uv_i, uv_j, uv_k, uv_center] = triangle_points(&node.uv, uv_midpoints);
    let ring_midpoints = [
        midpoint(node.seed_distance[0], node.seed_distance[1]),
        midpoint(node.seed_distance[1], node.seed_distance[2]),
        midpoint(node.seed_distance[2], node.seed_distance[0]),
    ];
    let [ring_i, ring_j, ring_k, ring_center] =
        triangle_points(&node.seed_distance, ring_midpoints);
    let parity = node.parity;
    let node_i = child_node(
        vertices_i,
        uv_i,
        ring_i,
        origin,
        level,
        format!("{}.I", node.name),
        parity,
    );
    let node_j = child_node(
        vertices_j,
        uv_j,
        ring_j,
        origin,
        level,
        format!("{}.J", node.name),
        parity,
    );
    let node_k = child_node(
        vertices_k,
        uv_k,
        ring_k,
        origin,
        level,
        format!("{}.K", node.name),
        parity,
    );
    let node_center = child_node(
        vertices_center,
        uv_center,
        ring_center,
        origin,
        level,
        format!("{}.C", node.name),
        parity.flipped(),
    );

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

/// Splits exactly one node of a live mesh and welds the new corner nodes
/// across the shared edges — the runtime counterpart of [`split_nodes`].
///
/// Where `split_nodes` refines a whole connected component one generation,
/// `split_node_local` refines the single `node` in place: it severs the
/// node's links, splits it with [`split_node`], and retargets the links that
/// pointed at the node to the new corner nodes. The old node is left fully
/// unlinked; dropping the caller's last reference deallocates it.
///
/// For each old link `node.port p <-> neighbor.port q` the shared edge is
/// resolved geometrically with exact vertex comparison, as in
/// [`split_nodes`]:
///
/// - If the neighbor is a corner of an already split group (same generation
///   as the new corners), both half-edges are welded corner-to-corner and
///   the edge stays watertight.
/// - If the neighbor is one level coarser (not split), only the corner near
///   the edge's first endpoint (`node.vertices[p]`) links to the neighbor's
///   recorded port; the second half-edge port stays open. This T-junction is
///   the accepted level-difference-1 boundary of the restricted-subdivision
///   rule (see `docs/book/specs/lod.md`); it is welded later when the
///   neighbor itself splits.
///
/// Open ports stay open. Returns the new center node.
///
/// # Panics
///
/// Panics when a linked neighbor holds neither the shared edge's endpoints
/// nor its midpoint (a mesh not produced by [`split_node`],
/// [`split_nodes`], or [`build_icosphere`](crate::node::build_icosphere)).
///
/// # Example
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::node::{build_icosphere, destroy_mesh, split_node_local};
///
/// let mesh = build_icosphere("planet", 300.0, 0, Vec3::ZERO);
/// let center = split_node_local(&mesh.faces[0]);
/// assert_eq!(center.borrow().level, 1);
/// // The old face node is retired: fully unlinked.
/// assert!(mesh.faces[0].borrow().children.iter().all(|c| c.is_none()));
/// destroy_mesh(&center);
/// ```
pub fn split_node_local(node: &NodeRef) -> NodeRef {
    // Record the old links and sever them.
    let old_vertices = node.borrow().vertices;
    let mut old_links: [Option<(NodeRef, usize)>; 3] = [None, None, None];
    for (port, slot) in old_links.iter_mut().enumerate() {
        let node_ref = node.borrow();
        if let (Some(neighbor), Some(back)) = (&node_ref.children[port], node_ref.back_ports[port])
        {
            *slot = Some((Rc::clone(neighbor), back));
        }
    }
    node.borrow_mut().destroy();

    let center = split_node(&node.borrow());
    let corners = corner_nodes(&center);

    // Retarget every old link across the shared edge.
    for (port, old_link) in old_links.iter().enumerate() {
        let Some((neighbor, back)) = old_link else {
            continue;
        };
        let endpoints = [old_vertices[port], old_vertices[(port + 1) % 3]];
        let edge_midpoint = midpoint(endpoints[0], endpoints[1]);
        if neighbor.borrow().vertices.contains(&edge_midpoint) {
            // The neighbor is a corner at the new corners' generation: weld
            // each half-edge to the corner that holds it. The counterpart
            // is searched outward from the old neighbor instead of through
            // its group center: after further local splits the group may
            // not be atomic anymore (the center's links can point at
            // grandchildren), while the corner nodes themselves are still
            // the right weld targets. A counterpart that no longer exists
            // (a group torn down past the bounded search) leaves the port
            // open rather than panicking.
            let target_level = center.borrow().level;
            for endpoint in endpoints {
                let near = &corners[corner_near(&corners, endpoint)];
                let Some(other) = half_edge_counterpart(
                    neighbor,
                    endpoint,
                    edge_midpoint,
                    target_level,
                    &corners,
                ) else {
                    continue;
                };
                link(
                    near,
                    port_on_edge(near, endpoint, edge_midpoint),
                    &other,
                    port_on_edge(&other, endpoint, edge_midpoint),
                );
            }
        } else {
            // The neighbor is one level coarser: link only the corner near
            // the edge's first endpoint and leave the second half-edge port
            // open (the restricted-subdivision T-junction).
            let near = &corners[corner_near(&corners, endpoints[0])];
            link(
                near,
                port_on_edge(near, endpoints[0], edge_midpoint),
                neighbor,
                *back,
            );
        }
    }

    center
}

/// Merges one complete split group back into its parent node — the runtime
/// counterpart of [`unsplit_nodes`], scoped to a single group.
///
/// `center` is the group's center node (named `"{base}.C"`), as returned by
/// [`split_node`] or [`split_node_local`]. The parent is rebuilt exactly as
/// in [`unsplit_nodes`]: vertices, UVs, ring values, and parity recovered
/// from the corner nodes, name and level from the group. Every link from a
/// group member to a node outside the group is retargeted to the surviving
/// parent: the outside node keeps its port, and the parent inherits the
/// corner's external port, which equals the parent edge's port number. The
/// four group members are destroyed (their links severed) before the
/// retargeting links are wired.
///
/// Returns `Some(parent)` when the group is complete and atomic (see
/// [`split_group_members`]). Returns `None` — without touching the graph —
/// when `center` is not the center of a complete group: a wrong name, a
/// level-0 node, or a non-atomic group whose center or corner was split
/// further and had its links retargeted to grandchildren. The non-atomic
/// states are reachable through local operations (the [`lod`](crate::lod)
/// scheduler splits group centers and corners as ordinary chunks), so they
/// are reported, not panicked on.
///
/// The caller is responsible for merge eligibility under restricted
/// subdivision: every node linked to the group must be at most at the group
/// level, so the level difference across the shared edges stays at most 1
/// after the merge. The [`lod`](crate::lod) scheduler enforces this.
///
/// # Example
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::node::{build_icosphere, destroy_mesh, split_node_local, unsplit_node};
///
/// let mesh = build_icosphere("planet", 300.0, 0, Vec3::ZERO);
/// let center = split_node_local(&mesh.faces[0]);
/// let parent = unsplit_node(&center).unwrap();
/// assert_eq!(parent.borrow().name, "planet.0");
/// assert_eq!(parent.borrow().level, 0);
/// destroy_mesh(&parent);
/// ```
pub fn unsplit_node(center: &NodeRef) -> Option<NodeRef> {
    let [node_i, node_j, node_k, center] = split_group_members(center)?;
    let (base, _) = split_suffix(&center.borrow().name)?;

    let (origin, level, parity) = {
        let node = node_i.borrow();
        (
            node.center + node.direction_to_origin,
            node.level - 1,
            node.parity,
        )
    };
    let vertices = [
        node_i.borrow().vertices[0],
        node_j.borrow().vertices[1],
        node_k.borrow().vertices[2],
    ];
    let uv = [
        node_i.borrow().uv[0],
        node_j.borrow().uv[1],
        node_k.borrow().uv[2],
    ];
    let ring = [
        node_i.borrow().seed_distance[0],
        node_j.borrow().seed_distance[1],
        node_k.borrow().seed_distance[2],
    ];
    let parent = child_node(vertices, uv, ring, origin, level, base, parity);

    // Collect the external links, then destroy the group, then retarget —
    // the same ordering as `unsplit_nodes`, so the destroy pass cannot
    // sever the new links.
    let group: [&NodeRef; 4] = [&node_i, &node_j, &node_k, &center];
    let mut kept_links: Vec<(usize, NodeRef, usize)> = Vec::new();
    for child in group {
        let node = child.borrow();
        for port in 0..3 {
            let (Some(neighbor), Some(back)) = (&node.children[port], node.back_ports[port]) else {
                continue;
            };
            if group.iter().any(|member| Rc::ptr_eq(member, neighbor)) {
                continue;
            }
            kept_links.push((port, Rc::clone(neighbor), back));
        }
    }
    for child in group {
        child.borrow_mut().destroy();
    }
    for (port, neighbor, back) in kept_links {
        if parent.borrow().children[port].is_none() {
            link(&parent, port, &neighbor, back);
        }
    }
    Some(parent)
}

/// The four members `[I, J, K, C]` of the split group `center` belongs to,
/// when the group is complete and atomic.
///
/// `center` must be named `"{base}.C"`, be linked to exactly its three
/// corner nodes named `"{base}.I"`, `"{base}.J"`, `"{base}.K"` through the
/// split port layout, and all four members must share the same level
/// `>= 1`. After further local operations the group may no longer be
/// atomic - a corner that was split leaves the center's port retargeted to
/// a grandchild - in which case this returns `None` instead of panicking.
pub fn split_group_members(center: &NodeRef) -> Option<[NodeRef; 4]> {
    let node = center.borrow();
    let (base, slot) = split_suffix(&node.name)?;
    if slot != 3 || node.level == 0 {
        return None;
    }
    let level = node.level;
    let corner = |port: usize, suffix: &str| {
        let child = node.children[port].as_ref()?;
        let child_ref = child.borrow();
        (child_ref.name == format!("{base}.{suffix}") && child_ref.level == level)
            .then(|| Rc::clone(child))
    };
    // The split port layout: center port 0 -> J, port 1 -> I, port 2 -> K.
    let node_j = corner(0, "J")?;
    let node_i = corner(1, "I")?;
    let node_k = corner(2, "K")?;
    Some([node_i, node_j, node_k, Rc::clone(center)])
}

/// Finds the node across the half-edge from `endpoint` to `midpoint` at
/// `level`, searching outward from `start` (the recorded old neighbor) with
/// a bounded breadth-first search.
///
/// The counterpart holds both vertices and has an open port on that edge;
/// nodes in `exclude` (the new corners on the near side) never match. A
/// small hop bound is enough: the counterparts of a well-formed split are
/// at most a few links away, even when the neighbor's split group is no
/// longer atomic (its center split further). A group torn down past the
/// bound - only reachable by splitting nodes without the level-difference
/// discipline of the [`lod`](crate::lod) scheduler - yields `None`.
fn half_edge_counterpart(
    start: &NodeRef,
    endpoint: Vec3,
    midpoint: Vec3,
    level: u32,
    exclude: &[NodeRef; 3],
) -> Option<NodeRef> {
    const MAX_HOPS: usize = 8;
    let mut visited = HashSet::new();
    let mut queue = VecDeque::from([(Rc::clone(start), 0usize)]);
    while let Some((node, hops)) = queue.pop_front() {
        if !visited.insert(Rc::as_ptr(&node)) {
            continue;
        }
        {
            let node_ref = node.borrow();
            if node_ref.level == level
                && node_ref.vertices.contains(&endpoint)
                && node_ref.vertices.contains(&midpoint)
                && !exclude.iter().any(|n| Rc::ptr_eq(n, &node))
                && node_ref.children[port_on_edge(&node, endpoint, midpoint)].is_none()
            {
                return Some(Rc::clone(&node));
            }
        }
        if hops == MAX_HOPS {
            continue;
        }
        for child in node.borrow().children.iter().flatten() {
            queue.push_back((Rc::clone(child), hops + 1));
        }
    }
    None
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
/// sphere build projected the midpoints), its UVs are recovered from the
/// same corners, its name is the group base name, and its level is the
/// group level minus one. The parents are then
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
        let (origin, level, parity) = {
            let node = node_i.borrow();
            (
                node.center + node.direction_to_origin,
                node.level - 1,
                node.parity,
            )
        };
        let vertices = [
            node_i.borrow().vertices[0],
            node_j.borrow().vertices[1],
            node_k.borrow().vertices[2],
        ];
        // The parent's UVs are recovered from the same corners as the
        // vertices: `I` holds `uA`, `J` holds `uB`, `K` holds `uC` — the
        // exact original UVs. Parity comes from the same channel: corner
        // children inherit the parent parity, so any corner holds it. The
        // ring field recovers exactly like the UVs: corner children hold
        // the parent's corner values verbatim.
        let uv = [
            node_i.borrow().uv[0],
            node_j.borrow().uv[1],
            node_k.borrow().uv[2],
        ];
        let ring = [
            node_i.borrow().seed_distance[0],
            node_j.borrow().seed_distance[1],
            node_k.borrow().seed_distance[2],
        ];
        let parent = child_node(vertices, uv, ring, origin, level, base, parity);
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
