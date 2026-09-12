//! UV coordinates for nodes: the canonical default triangle assigned by
//! [`Node::new`](super::Node::new), the icosahedral net layout that seeds
//! the base faces of [`build_icosphere`](super::build_icosphere), and the
//! generalized unfold of arbitrary triangle assemblies ([`unfold_uvs`]).
//!
//! # The icosahedral net
//!
//! The 20 base faces of the icosahedron are unwrapped into the classic flat
//! net (after Paul Bourke's icosahedral maps,
//! <https://paulbourke.net/panorama/icosahedral/>): a horizontal zigzag strip
//! of the 10 equatorial faces, the 5 faces around icosahedron vertex 0 fanned
//! across the top, and the 5 faces around vertex 9 fanned across the bottom —
//! a 5.5 x 3 triangle grid (aspect ratio ~2.117), normalized into `[0, 1]^2`
//! with a small margin.
//!
//! UVs are stored per node corner and duplicated across neighbors, exactly
//! like [`Node::vertices`](super::Node::vertices). The net is continuous
//! across every edge it preserves (strip adjacencies and fan attachments):
//! two faces sharing such an edge hold identical UVs for the shared vertices.
//! Every cut edge of the net is a **seam**: the two sides hold different UVs
//! for the same 3D vertex. Subdivision interpolates UVs linearly (flat
//! midpoints, never sphere-projected), so the property holds at every level.
//!
//! The layout is constructed by mirroring: an anchor face is placed, and each
//! other face is the reflection of an already-placed 3D neighbor across the
//! shared edge, with endpoints matched by icosahedron vertex index. This
//! makes the construction independent of the base-face winding (5 of the 20
//! base faces are deliberately wound inward; see the `icosphere` module).
//!
//! # Unfolding arbitrary assemblies
//!
//! For triangle assemblies outside the icosphere, [`unfold_uvs`] computes a
//! generalized net: a rigid breadth-first unfold of each connected component
//! (adjacency by bit-identical corner positions), normalized into `[0, 1]^2`.

use std::collections::{HashMap, HashSet, VecDeque};

use glam::{Vec2, Vec3};

use super::NodeRef;
use super::icosphere::ICOSAHEDRON_FACES;

/// Margin left around the net inside `[0, 1]^2`, as a fraction of the UV
/// space. Keeps bilinear filtering footprints inside the sampled region.
const NET_MARGIN: f32 = 0.01;

/// Default UV triangle of a node created outside an icosphere: a canonical
/// equilateral triangle (base 0.9, centered in `[0, 1]^2`), `A` at the apex,
/// so a lone triangle shows an undistorted texture.
pub const DEFAULT_UV: [Vec2; 3] = [
    Vec2::new(0.5, 0.889_711_44),
    Vec2::new(0.05, 0.110_288_56),
    Vec2::new(0.95, 0.110_288_56),
];

/// Strip order of the 10 equatorial faces: consecutive faces share an edge,
/// even positions point up (bottom-fan attachments) and odd positions point
/// down (top-fan attachments). Derived from the base-face adjacency of
/// `ICOSAHEDRON_FACES`.
const STRIP: [usize; 10] = [6, 5, 8, 9, 14, 16, 18, 17, 11, 7];

/// Top fan (the faces around icosahedron vertex 0), in strip order: entry
/// `i` attaches to `STRIP[2 * i + 1]`.
const TOP: [usize; 5] = [0, 2, 3, 4, 1];

/// Bottom fan (the faces around icosahedron vertex 9), in strip order: entry
/// `i` attaches to `STRIP[2 * i]`.
const BOTTOM: [usize; 5] = [10, 13, 15, 19, 12];

/// Computes the icosahedral net layout: `result[face_index]` holds the
/// `[uA, uB, uC]` corners of the matching `ICOSAHEDRON_FACES` entry, in
/// `[0, 1]^2`.
///
/// Pure and deterministic; computed on each icosphere build (20 triangles —
/// negligible). `pub` inside the private `uv` module; it only escapes under
/// the `test-internals` feature for the spec tests.
pub fn icosphere_net_uv() -> [[Vec2; 3]; 20] {
    // Equilateral triangle height for base = 1.
    let h = 0.5 * 3.0_f32.sqrt();
    let mut net = [[Vec2::ZERO; 3]; 20];
    let mut placed = [false; 20];

    // Anchor: the first strip face, up-pointing — A at the apex, B at the
    // base-left, C at the base-right.
    net[STRIP[0]] = [Vec2::new(0.5, h), Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0)];
    placed[STRIP[0]] = true;

    // Unfold the strip: each face is the mirror image of its predecessor
    // across the shared edge.
    for pair in STRIP.windows(2) {
        let [prev, face] = [pair[0], pair[1]];
        net[face] = mirror_across_shared_edge(net[prev], prev, face);
        placed[face] = true;
    }

    // Unfold the fans across the attachment edges of their strip faces.
    for (index, &fan) in TOP.iter().enumerate() {
        let anchor = STRIP[2 * index + 1];
        net[fan] = mirror_across_shared_edge(net[anchor], anchor, fan);
        placed[fan] = true;
    }
    for (index, &fan) in BOTTOM.iter().enumerate() {
        let anchor = STRIP[2 * index];
        net[fan] = mirror_across_shared_edge(net[anchor], anchor, fan);
        placed[fan] = true;
    }
    debug_assert!(placed.into_iter().all(|flag| flag));

    normalize(&mut net);
    net
}

/// Places `face` as the reflection of the already-placed `anchor` face across
/// their shared 3D edge. The shared edge endpoints are matched by icosahedron
/// vertex index, so the correspondence is exact for any winding.
fn mirror_across_shared_edge(anchor: [Vec2; 3], anchor_face: usize, face: usize) -> [Vec2; 3] {
    let anchor_corners = ICOSAHEDRON_FACES[anchor_face];
    let corners = ICOSAHEDRON_FACES[face];

    let mut uv = [Vec2::ZERO; 3];
    let mut edge = [Vec2::ZERO; 2];
    let mut edge_len = 0;
    let mut anchor_apex = None;
    let mut face_apex = None;
    for (corner, &vertex) in anchor_corners.iter().enumerate() {
        match corners.iter().position(|&v| v == vertex) {
            Some(shared) => {
                uv[shared] = anchor[corner];
                edge[edge_len] = anchor[corner];
                edge_len += 1;
            }
            None => anchor_apex = Some(anchor[corner]),
        }
    }
    for (corner, &vertex) in corners.iter().enumerate() {
        if !anchor_corners.contains(&vertex) {
            face_apex = Some(corner);
        }
    }
    debug_assert_eq!(edge_len, 2, "net faces must share exactly one edge");

    uv[face_apex.expect("net faces must share an edge")] = reflect(
        anchor_apex.expect("net faces must share an edge"),
        edge[0],
        edge[1],
    );
    uv
}

/// Reflects `point` across the line through `line_a` and `line_b`.
fn reflect(point: Vec2, line_a: Vec2, line_b: Vec2) -> Vec2 {
    let direction = (line_b - line_a).normalize();
    let projection = line_a + direction * (point - line_a).dot(direction);
    2.0 * projection - point
}

/// Normalizes `faces` into `[0, 1]^2`: uniform scale preserving the aspect
/// ratio, centered, with a `NET_MARGIN` border. Shared by the icosahedral
/// net and the components unfolded by [`unfold_uvs`].
fn normalize(faces: &mut [[Vec2; 3]]) {
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for face in faces.iter() {
        for &point in face {
            min = min.min(point);
            max = max.max(point);
        }
    }
    let extent = max - min;
    let scale = (1.0 - 2.0 * NET_MARGIN) / extent.max_element();
    let offset =
        Vec2::splat(NET_MARGIN) + (Vec2::splat(1.0 - 2.0 * NET_MARGIN) - extent * scale) * 0.5;
    for face in faces {
        for point in face {
            *point = (*point - min) * scale + offset;
        }
    }
}

/// Unfolds an arbitrary assembly of triangles into continuous texture
/// coordinates: a generalized net computed by a rigid breadth-first unfold
/// of each connected component.
///
/// # Contract
///
/// - Two nodes are adjacent when they share an edge: two corner positions
///   that are **bit-identical** `Vec3` values (the weld convention used
///   across the node system; see the `topology` module). Only exact
///   positions match - nearly-equal vertices do not weld. Matching is by
///   position, so any winding works.
/// - Across every shared edge the unfold crosses, the shared corners hold
///   bit-identical UVs on both nodes: the texture is continuous there.
///   Adjacencies the unfold does not cross become natural **seams**: the
///   two sides hold different UVs for the same 3D vertex (UVs are stored
///   per corner, duplicated across neighbors).
/// - Each triangle's UV shape is congruent to its 3D shape (zero stretch);
///   for equilateral triangles the placement reduces exactly to the
///   reflection used by the icosahedral net. After placement each component
///   is normalized into `[0, 1]^2` with a `NET_MARGIN` border, one uniform
///   scale per component.
/// - Lone triangles (components of one node) get exactly [`DEFAULT_UV`];
///   this also resets a previously unfolded lone triangle.
/// - Degenerate geometry never panics: a node whose placement needs a
///   zero-length or non-finite edge keeps [`DEFAULT_UV`] and the unfold
///   does not propagate through it. Non-manifold edges (registered by more
///   than two nodes) pair the first two nodes and ignore the rest.
///
/// Only the input order drives the traversal: components and their
/// breadth-first unfolds are processed in input order, so the result is
/// deterministic.
///
/// # Split and unsplit compatibility
///
/// [`split_node`](super::split_node) interpolates UVs linearly along the
/// edges, so continuity across a welded edge survives subdivision in both
/// directions: unfolding an already-split welded mesh works, and splitting
/// an unfolded mesh keeps the shared-edge UVs identical on both sides.
///
/// # Icosphere meshes
///
/// Base faces built by [`build_icosphere`](super::build_icosphere) already
/// carry the curated icosahedral net (see the module documentation); do not
/// pass them here.
///
/// # Example
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::node::{Node, unfold_uvs};
///
/// // Two right triangles sharing the edge (300,0,0)-(0,200,0).
/// let a = Node::new("a", [Vec3::new(0.0, 0.0, 0.0), Vec3::new(300.0, 0.0, 0.0), Vec3::new(0.0, 200.0, 0.0)], Vec3::ZERO);
/// let b = Node::new("b", [Vec3::new(300.0, 0.0, 0.0), Vec3::new(300.0, 200.0, 0.0), Vec3::new(0.0, 200.0, 0.0)], Vec3::ZERO);
/// unfold_uvs(&[a.clone(), b.clone()]);
///
/// // The shared corners hold bit-identical UVs on both nodes.
/// let (uv_a, uv_b) = (a.borrow().uv, b.borrow().uv);
/// assert_eq!(uv_a[1], uv_b[0]); // (300, 0, 0)
/// assert_eq!(uv_a[2], uv_b[2]); // (0, 200, 0)
/// ```
pub fn unfold_uvs(nodes: &[NodeRef]) {
    let adjacency = geometric_adjacency(nodes);
    let mut visited = vec![false; nodes.len()];
    for root in 0..nodes.len() {
        if visited[root] {
            continue;
        }
        // Collect the connected component in breadth-first order.
        let mut component = vec![root];
        let mut queue = VecDeque::from([root]);
        visited[root] = true;
        while let Some(index) = queue.pop_front() {
            for neighbor in adjacency[index].into_iter().flatten() {
                if !visited[neighbor] {
                    visited[neighbor] = true;
                    component.push(neighbor);
                    queue.push_back(neighbor);
                }
            }
        }
        if component.len() == 1 {
            nodes[root].borrow_mut().uv = DEFAULT_UV;
        } else {
            unfold_component(nodes, &adjacency, &component);
        }
    }
}

/// Bit pattern of a vertex position: positions match bit-exactly.
type VertexKey = [u32; 3];

/// Unordered pair of vertex keys identifying an edge: `(p, q)` and `(q, p)`
/// produce the same key.
type EdgeKey = [VertexKey; 2];

/// Bit pattern of `point`, coordinate by coordinate.
fn vertex_key(point: Vec3) -> VertexKey {
    [point.x.to_bits(), point.y.to_bits(), point.z.to_bits()]
}

/// Key of the edge between `p` and `q`, independent of the endpoint order.
fn edge_key(p: Vec3, q: Vec3) -> EdgeKey {
    let (p, q) = (vertex_key(p), vertex_key(q));
    if p <= q { [p, q] } else { [q, p] }
}

/// Geometric adjacency of a node set: `result[index][port]` is the index of
/// the node sharing the edge across `port`, matched by bit-identical
/// endpoints. Ports follow the `children` convention: 0 = AB, 1 = BC,
/// 2 = CA. An edge registered by more than two nodes pairs the first two
/// and ignores the rest.
fn geometric_adjacency(nodes: &[NodeRef]) -> Vec<[Option<usize>; 3]> {
    let mut registered: HashMap<EdgeKey, (usize, usize)> = HashMap::new();
    let mut paired: HashSet<EdgeKey> = HashSet::new();
    let mut adjacency = vec![[None; 3]; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        let vertices = node.borrow().vertices;
        for (port, (i, j)) in [(0, 1), (1, 2), (2, 0)].into_iter().enumerate() {
            let key = edge_key(vertices[i], vertices[j]);
            if paired.contains(&key) {
                continue;
            }
            match registered.remove(&key) {
                Some((other, other_port)) => {
                    adjacency[other][other_port] = Some(index);
                    adjacency[index][port] = Some(other);
                    paired.insert(key);
                }
                None => {
                    registered.insert(key, (index, port));
                }
            }
        }
    }
    adjacency
}

/// Rigid breadth-first unfold of one connected component (at least two
/// nodes): `component[0]` is the root, each further node is placed against
/// its already-placed BFS parent, and the whole component is normalized
/// into `[0, 1]^2` at the end. Nodes that cannot be placed (degenerate or
/// non-finite geometry) keep [`DEFAULT_UV`] and do not propagate.
fn unfold_component(nodes: &[NodeRef], adjacency: &[[Option<usize>; 3]], component: &[usize]) {
    for &index in component {
        nodes[index].borrow_mut().uv = DEFAULT_UV;
    }

    let mut placed = vec![false; nodes.len()];
    let mut queue = VecDeque::new();
    let root = component[0];
    placed[root] = true;
    if place_root(&nodes[root]) {
        queue.push_back(root);
    }
    while let Some(parent) = queue.pop_front() {
        for neighbor in adjacency[parent].into_iter().flatten() {
            if placed[neighbor] {
                continue;
            }
            placed[neighbor] = true;
            if place_neighbor(&nodes[parent], &nodes[neighbor]) {
                queue.push_back(neighbor);
            }
        }
    }

    let mut uvs: Vec<[Vec2; 3]> = component
        .iter()
        .map(|&index| nodes[index].borrow().uv)
        .collect();
    normalize(&mut uvs);
    for (&index, uv) in component.iter().zip(uvs) {
        nodes[index].borrow_mut().uv = uv;
    }
}

/// Rigid placement of a component root: `A` at the origin, `B` on the
/// positive x axis, `C` in the upper half-plane, preserving the 3D edge
/// lengths. Returns false (leaving the node at [`DEFAULT_UV`]) when the
/// triangle is degenerate or non-finite.
fn place_root(node: &NodeRef) -> bool {
    let [a, b, c] = node.borrow().vertices;
    let ab = (b - a).length();
    let cb = (b - c).length();
    let ca = (a - c).length();
    let x = (ca * ca - cb * cb + ab * ab) / (2.0 * ab);
    let h = (ca * ca - x * x).max(0.0).sqrt();
    let uv = [Vec2::ZERO, Vec2::new(ab, 0.0), Vec2::new(x, h)];
    if !ab.is_finite() || ab <= 1e-6 || uv.iter().any(|point| !point.is_finite()) {
        return false;
    }
    node.borrow_mut().uv = uv;
    true
}

/// Rigid placement of `node` against its already-placed `parent` across
/// their shared edge: the shared corners copy the parent's UVs (matched by
/// bit-exact 3D position) and the third corner goes to the side of the
/// shared edge opposite the parent's third corner, preserving the 3D edge
/// lengths. Returns false (leaving the node at [`DEFAULT_UV`]) when the
/// shared edge is degenerate or the geometry is non-finite.
fn place_neighbor(parent: &NodeRef, node: &NodeRef) -> bool {
    let parent = parent.borrow();
    let mut node = node.borrow_mut();

    let mut uv = DEFAULT_UV;
    let mut edge_uv = [Vec2::ZERO; 2];
    let mut edge_3d = [Vec3::ZERO; 2];
    let mut matched = [false; 3];
    let mut shared = 0;
    let mut apex = None;
    for (corner, &vertex) in node.vertices.iter().enumerate() {
        match parent.vertices.iter().position(|&p| p == vertex) {
            Some(index) => {
                matched[index] = true;
                if shared < 2 {
                    edge_uv[shared] = parent.uv[index];
                    edge_3d[shared] = vertex;
                }
                shared += 1;
                uv[corner] = parent.uv[index];
            }
            None => apex = Some(corner),
        }
    }
    if shared != 2 {
        return false;
    }
    let (Some(apex), Some(parent_apex)) = (
        apex,
        matched
            .iter()
            .position(|&flag| !flag)
            .map(|index| parent.uv[index]),
    ) else {
        return false;
    };

    // Third corner from the edge lengths: the same law-of-cosines placement
    // as the root, against the shared edge (u1, u2).
    let [u1, u2] = edge_uv;
    let [v1, v2] = edge_3d;
    let v3 = node.vertices[apex];
    let d = (u2 - u1).length();
    if !d.is_finite() || d <= 1e-6 {
        return false;
    }
    let l1 = (v3 - v1).length();
    let l2 = (v3 - v2).length();
    let a = (l1 * l1 - l2 * l2 + d * d) / (2.0 * d);
    let h = (l1 * l1 - a * a).max(0.0).sqrt();
    let e = (u2 - u1) / d;
    let perp = Vec2::new(-e.y, e.x);
    let base = u1 + a * e;
    // `base + h * perp` lies left of u1 -> u2, `base - h * perp` right; the
    // apex goes to the side opposite the parent's third corner. A parent
    // corner exactly on the line (or h == 0) takes the minus branch.
    let side = (u2 - u1).perp_dot(parent_apex - u1);
    let apex_uv = if side < 0.0 {
        base + h * perp
    } else {
        base - h * perp
    };
    if !apex_uv.is_finite() {
        return false;
    }
    uv[apex] = apex_uv;
    node.uv = uv;
    true
}
