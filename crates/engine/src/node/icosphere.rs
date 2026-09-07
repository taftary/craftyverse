//! Icosphere construction: a closed, watertight, geodesic sphere built on
//! top of [`Node`](super::Node) and the existing split machinery.
//!
//! [`build_icosphere`] seeds a regular icosahedron (12 vertices, 20 faces)
//! at the target radius, then refines every leaf triangle one full
//! generation at a time. New edge midpoints are projected back onto the
//! sphere surface, and triangles sharing an edge are welded corner-to-corner
//! so the mesh stays connected at every level.
//!
//! Each base face is also assigned its triangle of the icosahedral UV net
//! (see the `uv` module). Refinement inherits UVs by flat linear
//! interpolation — the sphere projection of the edge midpoints does not
//! apply to texture space — so UVs stay continuous inside each base face and
//! across the net's preserved edges, with seams exactly on the cut edges.
//!
//! # Link correctness and the reciprocal port pattern
//!
//! Every link welded here follows the `0 <-> 2`, `1 <-> 1` reciprocal port
//! pattern: a link through port `x` on one side uses port `2 - x` on the
//! other, on every edge of every subdivision level. Each link still carries
//! an explicitly recorded back-port (see the `back_ports` field of
//! [`Node`](super::Node)), so traversal, pruning, and cleanup stay exact and
//! never assume the pattern.
//!
//! Full conformance has a price: with all faces wound outward, satisfying
//! the pattern on all 30 base edges is a constraint system over the
//! dodecahedron dual with no solution. The base face labeling used here
//! solves it by winding 5 of the 20 faces inward (reversed vertex order,
//! `abc -> acb`). Winding is therefore not a mesh invariant — corner
//! children inherit their parent's winding and the center child reverses
//! it — and nothing may rely on it; the renderer uses no backface culling,
//! and every derived direction is winding-independent by construction.

use std::collections::HashMap;
use std::rc::Rc;

use glam::Vec3;

use super::NodeRef;
use super::geometry::midpoint;
use super::subdivision::split_node_with_midpoints;
use super::topology::{corner_near, corner_nodes, link, port_on_edge};

/// The result of [`build_icosphere`]: the fully linked leaf graph plus the
/// closed-form mesh statistics.
pub struct IcosphereMesh {
    /// All leaf-level (deepest) triangles, fully linked to their neighbors:
    /// `20 * 4^subdivisions` faces.
    pub faces: Vec<NodeRef>,
    /// Number of distinct vertices: `10 * 4^subdivisions + 2` (Euler's
    /// formula for a closed triangulated sphere).
    pub vertex_count: usize,
}

/// Upper bound on the `subdivisions` argument of [`build_icosphere`].
///
/// Each generation multiplies the face count by four, so 8 generations
/// already produce `20 * 4^8 = 1.3M` faces; the cap keeps the closed-form
/// counts far from `usize` overflow and the build cost sane. The viewer's
/// own policy cap (`render::MAX_ICOSPHERE_SUBDIVISIONS`) is lower and
/// independent.
pub const MAX_SUBDIVISIONS: u32 = 8;

/// Golden ratio, the base of the canonical icosahedron coordinates.
const PHI: f32 = 1.618_034;

/// The 12 canonical icosahedron vertices.
const ICOSAHEDRON_VERTICES: [Vec3; 12] = [
    Vec3::new(0.0, 1.0, PHI),
    Vec3::new(1.0, PHI, 0.0),
    Vec3::new(PHI, 0.0, 1.0),
    Vec3::new(0.0, 1.0, -PHI),
    Vec3::new(1.0, -PHI, 0.0),
    Vec3::new(PHI, 0.0, -1.0),
    Vec3::new(0.0, -1.0, PHI),
    Vec3::new(-1.0, PHI, 0.0),
    Vec3::new(-PHI, 0.0, 1.0),
    Vec3::new(0.0, -1.0, -PHI),
    Vec3::new(-1.0, -PHI, 0.0),
    Vec3::new(-PHI, 0.0, -1.0),
];

/// The 20 base faces as vertex index triplets `[A, B, C]`. 15 faces are
/// wound outward; faces 6, 8, 11, 14 and 18 are deliberately reversed
/// (`abc -> acb`, inward) so the reciprocal port rule
/// (`local_edge_a + local_edge_b == 2` on every shared edge) holds on all 30
/// base edges — with all-outward winding the constraint system has no
/// solution (see the module documentation). The vertex sets are the canonical
/// icosahedron faces; only the local A/B/C assignment differs.
///
/// Also the indexing base of the icosahedral net layout (see the `uv`
/// module): net table entry `i` belongs to face `i`.
pub(crate) const ICOSAHEDRON_FACES: [[usize; 3]; 20] = [
    [0, 2, 1],
    [0, 1, 7],
    [0, 6, 2],
    [0, 8, 6],
    [0, 7, 8],
    [5, 1, 2],
    [1, 3, 5],
    [3, 7, 1],
    [2, 5, 4],
    [4, 2, 6],
    [9, 3, 5],
    [7, 11, 3],
    [9, 11, 3],
    [9, 5, 4],
    [6, 4, 10],
    [9, 4, 10],
    [10, 6, 8],
    [11, 8, 7],
    [8, 10, 11],
    [9, 10, 11],
];

/// Builds a closed, watertight geodesic sphere of `radius` around `origin`.
///
/// Seeds a regular icosahedron and applies `subdivisions` generations of
/// refinement to every face. Every newly created midpoint is projected back
/// onto the sphere surface, and every shared edge is welded so adjacent
/// triangles stay linked at every level. `subdivisions = 0` returns the raw
/// 20-face icosahedron, fully linked.
///
/// Base faces are named `"{name_prefix}.{face_index}"` and extended with
/// the usual `.I` / `.J` / `.K` / `.C` suffixes per generation.
///
/// Cleanup: call [`destroy_mesh`](super::destroy_mesh) on any face before
/// dropping the mesh, or the reciprocal-link cycles leak every node.
///
/// # Panics
///
/// Panics when `radius` is not greater than `0.0`, or when `subdivisions`
/// exceeds [`MAX_SUBDIVISIONS`].
pub fn build_icosphere(
    name_prefix: impl Into<String>,
    radius: f32,
    subdivisions: u32,
    origin: Vec3,
) -> IcosphereMesh {
    assert!(radius > 0.0, "icosphere radius must be positive");
    assert!(
        subdivisions <= MAX_SUBDIVISIONS,
        "icosphere subdivisions must be at most {MAX_SUBDIVISIONS}"
    );

    let name_prefix = name_prefix.into();
    // Safe after the cap above: `4^MAX_SUBDIVISIONS` fits easily in `usize`.
    let vertex_count = 10 * 4_usize.pow(subdivisions) + 2;

    // Step 1 — base icosahedron, seeded directly at the target radius. Each
    // base face also receives its icosahedral net UV triangle (see the `uv`
    // module); refinement inherits the UVs by linear interpolation.
    let net = super::uv::icosphere_net_uv();
    let vertices: Vec<Vec3> = ICOSAHEDRON_VERTICES
        .iter()
        .map(|v| origin + v.normalize() * radius)
        .collect();
    let mut leaves: Vec<NodeRef> = ICOSAHEDRON_FACES
        .iter()
        .enumerate()
        .map(|(face_index, &[a, b, c])| {
            let node = super::Node::new(
                format!("{name_prefix}.{face_index}"),
                [vertices[a], vertices[b], vertices[c]],
                origin,
            );
            node.borrow_mut().uv = net[face_index];
            node
        })
        .collect();

    // Step 2 — base edge adjacency: link the 20 base nodes corner-to-corner
    // so the level-0 mesh is closed from the start.
    let mut edge_adjacency: HashMap<(usize, usize), Vec<(usize, usize)>> = HashMap::new();
    for (face_index, [a, b, c]) in ICOSAHEDRON_FACES.iter().enumerate() {
        for (local_edge, (u, v)) in [(*a, *b), (*b, *c), (*c, *a)].iter().enumerate() {
            let key = if u < v { (*u, *v) } else { (*v, *u) };
            edge_adjacency
                .entry(key)
                .or_default()
                .push((face_index, local_edge));
        }
    }
    for endpoints in edge_adjacency.values() {
        let [(face_a, edge_a), (face_b, edge_b)] = endpoints.as_slice() else {
            panic!("icosahedron base mesh is not closed");
        };
        link(&leaves[*face_a], *edge_a, &leaves[*face_b], *edge_b);
    }

    // Step 3 — generation-synchronized refinement.
    let mut next_leaves: Vec<NodeRef> = Vec::new();
    for _ in 0..subdivisions {
        next_leaves.clear();
        next_leaves.reserve(4 * leaves.len());

        // Edge-weld cache, keyed by the unordered pair of parent nodes
        // sharing an edge. The first side to reach the edge computes and
        // stores the projected midpoint; the first side to split records its
        // waiting corner children; the second side claims them, which wires
        // the corner-to-corner links and drains the entry.
        let mut edge_cache: HashMap<(usize, usize), WeldEntry> = HashMap::new();
        edge_cache.reserve(3 * leaves.len() / 2);

        for leaf in &leaves {
            let (vertices, neighbors) = {
                let node = leaf.borrow();
                (node.vertices, node.children.clone())
            };
            let parent_key = Rc::as_ptr(leaf) as usize;

            // Resolve the three edge midpoints through the cache so each
            // shared edge is projected exactly once.
            let flat = [
                midpoint(vertices[0], vertices[1]),
                midpoint(vertices[1], vertices[2]),
                midpoint(vertices[2], vertices[0]),
            ];
            let mut projected = [Vec3::ZERO; 3];
            for (edge, flat_midpoint) in flat.iter().enumerate() {
                let neighbor = neighbors[edge]
                    .as_ref()
                    .expect("icosphere leaf is missing a neighbor link");
                let key = cache_key(parent_key, Rc::as_ptr(neighbor) as usize);
                projected[edge] = match edge_cache.get(&key) {
                    Some(entry) => entry.projected_midpoint,
                    None => {
                        let projected_midpoint =
                            origin + (*flat_midpoint - origin).normalize() * radius;
                        edge_cache.insert(
                            key,
                            WeldEntry {
                                projected_midpoint,
                                waiting: None,
                            },
                        );
                        projected_midpoint
                    }
                };
            }

            let center = split_node_with_midpoints(&leaf.borrow(), projected);
            let corners = corner_nodes(&center);

            // Register or claim the cross-face corner links for each edge.
            for (edge, neighbor) in neighbors.iter().enumerate() {
                let neighbor = neighbor
                    .as_ref()
                    .expect("icosphere leaf is missing a neighbor link");
                let key = cache_key(parent_key, Rc::as_ptr(neighbor) as usize);
                // The two endpoints of this parent edge, in triplet order.
                let endpoints = [vertices[edge], vertices[(edge + 1) % 3]];
                let entry = edge_cache.get_mut(&key).expect("edge cache entry missing");
                let projected_midpoint = entry.projected_midpoint;
                match entry.waiting.take() {
                    None => {
                        entry.waiting = Some(endpoints.map(|endpoint| {
                            (
                                Rc::clone(&corners[corner_near(&corners, endpoint)]),
                                endpoint,
                            )
                        }));
                    }
                    Some(waiting_there) => {
                        // Wire both halves of the shared edge: the waiting
                        // corner near each shared endpoint links to this
                        // side's corner near the same endpoint, with ports
                        // resolved geometrically on the shared edge
                        // (endpoint to welded midpoint).
                        for (child, endpoint) in waiting_there {
                            let other = Rc::clone(&corners[corner_near(&corners, endpoint)]);
                            link(
                                &child,
                                port_on_edge(&child, endpoint, projected_midpoint),
                                &other,
                                port_on_edge(&other, endpoint, projected_midpoint),
                            );
                        }
                        edge_cache.remove(&key);
                    }
                }
            }

            next_leaves.extend(corners);
            next_leaves.push(center);
        }

        debug_assert!(
            edge_cache.is_empty(),
            "edge weld cache not drained: mesh is not closed"
        );

        // Step 4 — prune the outgoing generation before starting the next.
        for leaf in &leaves {
            leaf.borrow_mut().destroy();
        }
        std::mem::swap(&mut leaves, &mut next_leaves);
    }

    IcosphereMesh {
        faces: leaves,
        vertex_count,
    }
}

/// One edge-weld cache entry: the projected midpoint (set on first touch)
/// plus the corner children of the first split side waiting to be linked.
struct WeldEntry {
    projected_midpoint: Vec3,
    /// `(corner node, shared endpoint)` for the two half-edges of the first
    /// side to split.
    waiting: Option<[(NodeRef, Vec3); 2]>,
}

/// Unordered cache key for the edge shared by two parent nodes.
fn cache_key(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}
