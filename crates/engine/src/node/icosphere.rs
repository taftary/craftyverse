//! Icosphere construction: a closed, watertight, geodesic sphere built on
//! top of [`Node`](super::Node) and the existing split machinery.
//!
//! [`build_icosphere`] seeds a regular icosahedron (12 vertices, 20 faces)
//! at the target radius, then refines every leaf triangle one full
//! generation at a time. New edge midpoints are projected back onto the
//! sphere surface, and triangles sharing an edge are welded corner-to-corner
//! so the mesh stays connected at every level.
//!
//! # Link correctness and the reciprocal port pattern
//!
//! Every link welded here is fully correct: each carries an explicitly
//! recorded back-port (see the `back_ports` field of
//! [`Node`](super::Node)), so traversal, pruning, and cleanup are exact for
//! every edge of the mesh.
//!
//! The historical `0 <-> 2`, `1 <-> 1` port *pattern* is a different story:
//! it cannot hold on every edge of a closed icosahedron-based mesh —
//! satisfying it on all 30 base edges is a constraint system over the
//! dodecahedron dual with no solution. The base face labeling used here
//! maximizes conformance: only 6 of the 30 base edges (and their
//! subdivision descendants) have a back-port different from `2 - index`.
//! This is a topological curiosity, not a defect — which is exactly why the
//! back-port is stored rather than assumed.

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
    /// All leaf-level (deepest) triangles, fully linked to their neighbors.
    pub faces: Vec<NodeRef>,
    /// Number of leaf faces: `20 * 4^subdivisions`.
    pub face_count: usize,
    /// Number of distinct vertices: `10 * 4^subdivisions + 2` (Euler's
    /// formula for a closed triangulated sphere).
    pub vertex_count: usize,
}

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

/// The 20 base faces as vertex index triplets `[A, B, C]` with consistent
/// outward winding. The per-face rotations are chosen so that the reciprocal
/// port rule (`local_edge_a + local_edge_b == 2` on every shared edge) holds
/// on 24 of the 30 base edges; the remaining 6 cannot be satisfied by any
/// labeling (see the module documentation).
const ICOSAHEDRON_FACES: [[usize; 3]; 20] = [
    [0, 2, 1],
    [0, 1, 7],
    [0, 6, 2],
    [0, 8, 6],
    [0, 7, 8],
    [5, 1, 2],
    [5, 3, 1],
    [1, 3, 7],
    [5, 2, 4],
    [6, 4, 2],
    [5, 9, 3],
    [11, 7, 3],
    [11, 3, 9],
    [5, 4, 9],
    [6, 10, 4],
    [10, 9, 4],
    [6, 8, 10],
    [11, 8, 7],
    [11, 10, 8],
    [11, 9, 10],
];

/// Builds a closed, watertight geodesic sphere of `radius` around `origin`.
///
/// Seeds a regular icosahedron and applies `subdivisions` generations of
/// refinement to every face. Every newly created midpoint is projected back
/// onto the sphere surface, and every shared edge is welded so adjacent
/// triangles stay linked at every level. `subdivisions = 0` returns the raw
/// 20-face icosahedron, fully linked.
///
/// `radius` must be greater than `0.0`. Base faces are named
/// `"{name_prefix}.{face_index}"` and extended with the usual `.I` / `.J` /
/// `.K` / `.C` suffixes per generation.
///
/// Cleanup follows the existing pattern: `collect_nodes` on any face
/// reaches the whole sphere, then `destroy()` each collected node.
pub fn build_icosphere(
    name_prefix: impl Into<String>,
    radius: f32,
    subdivisions: u32,
    origin: Vec3,
) -> IcosphereMesh {
    assert!(radius > 0.0, "icosphere radius must be positive");

    let name_prefix = name_prefix.into();
    let face_count = 20 * 4_usize.pow(subdivisions);
    let vertex_count = 10 * 4_usize.pow(subdivisions) + 2;

    // Step 1 — base icosahedron, seeded directly at the target radius.
    let vertices: Vec<Vec3> = ICOSAHEDRON_VERTICES
        .iter()
        .map(|v| origin + v.normalize() * radius)
        .collect();
    let mut leaves: Vec<NodeRef> = ICOSAHEDRON_FACES
        .iter()
        .enumerate()
        .map(|(face_index, &[a, b, c])| {
            super::Node::new(
                format!("{name_prefix}.{face_index}"),
                [vertices[a], vertices[b], vertices[c]],
                origin,
            )
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
            let (points, neighbors) = {
                let node = leaf.borrow();
                (node.points, node.children.clone())
            };
            let parent_key = Rc::as_ptr(leaf) as usize;

            // Resolve the three edge midpoints through the cache so each
            // shared edge is projected exactly once.
            let flat = [
                midpoint(points[0], points[1]),
                midpoint(points[1], points[2]),
                midpoint(points[2], points[0]),
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
                let endpoints = [points[edge], points[(edge + 1) % 3]];
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
        face_count,
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

#[cfg(test)]
mod tests;
