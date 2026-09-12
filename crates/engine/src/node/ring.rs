//! The ring field: a mesh-global scalar field giving each corner vertex its
//! distance to the nearest seed vertex, in band-width units (a value of
//! `1.0` is one ring band of the procedural `rings` effect).
//!
//! Unlike UVs — which are deliberately per-face and may carry seams — the
//! ring field is continuous across the whole mesh by construction: it is
//! computed per distinct corner position (bit-identical positions, the weld
//! convention of the topology module) and written back to every corner that
//! shares the position, so adjacent triangles always agree at shared
//! vertices and the interpolated field continues across edges.
//!
//! Builders seed the field once with [`assign_geodesic_ring_field`] (true
//! arc distance on a sphere — evenly spaced rings) or
//! [`assign_planar_ring_field`] (euclidean distance on flat assemblies).
//! [`split_node`](super::split_node) then interpolates it with flat linear
//! midpoints, exactly like UVs: on a sphere that is a chord-space
//! approximation of the arc field, whose error shrinks with every
//! subdivision level (and is visually irrelevant beyond level 0);
//! [`unsplit_nodes`](super::unsplit_nodes) recovers the parent's corner
//! values exactly, again like UVs.

use std::collections::HashMap;

use glam::Vec3;

use super::NodeRef;

/// Ring field of an unseeded node: all corners in band zero (a single,
/// uniform band).
pub const DEFAULT_RING: [f32; 3] = [0.0; 3];

/// Ring bands per base-edge arc of the icosphere seeding: the distance
/// between two adjacent base vertices spans `RING_BANDS` ring bands.
pub const RING_BANDS: u32 = 8;

/// Assigns each corner of `nodes` its geodesic distance (arc length on the
/// sphere of `center`) to the nearest position of `seeds`, divided by
/// `band_width`.
///
/// `center` is the sphere center; the radius is taken per corner position
/// (`(p - center).length()`), so the helper tolerates small radial jitter.
/// Corner positions are clustered by bit-identical value: every corner
/// sharing a position receives the same value, keeping the field continuous
/// across the mesh. Panics on a zero or negative `band_width`; positions at
/// the antipode of a seed are clamped into the valid `acos` domain.
pub fn assign_geodesic_ring_field(
    nodes: &[NodeRef],
    seeds: &[Vec3],
    center: Vec3,
    band_width: f32,
) {
    assert!(band_width > 0.0, "ring band width must be positive");
    let seed_dirs: Vec<Vec3> = seeds
        .iter()
        .map(|seed| (*seed - center).normalize())
        .collect();
    assign_ring_field(nodes, |p| {
        let dir = (p - center).normalize();
        let radius = (p - center).length();
        let min_angle = seeds
            .iter()
            .zip(&seed_dirs)
            .map(|(seed, seed_dir)| {
                // Snap the seed itself to exactly zero: the dot of a
                // normalized vector with itself rounds a hair below 1.
                if p == *seed {
                    0.0
                } else {
                    dir.dot(*seed_dir).clamp(-1.0, 1.0).acos()
                }
            })
            .fold(f32::INFINITY, f32::min);
        min_angle * radius / band_width
    });
}

/// Assigns each corner of `nodes` its euclidean distance to the nearest
/// position of `seeds`, divided by `band_width`.
///
/// Corner positions are clustered by bit-identical value, keeping the field
/// continuous across the mesh (see the module documentation). Panics on a
/// zero or negative `band_width`.
pub fn assign_planar_ring_field(nodes: &[NodeRef], seeds: &[Vec3], band_width: f32) {
    assert!(band_width > 0.0, "ring band width must be positive");
    assign_ring_field(nodes, |p| {
        seeds
            .iter()
            .map(|seed| (p - *seed).length())
            .fold(f32::INFINITY, f32::min)
            / band_width
    });
}

/// Evaluates `distance` once per distinct corner position of `nodes` and
/// writes the value back to every corner sharing the position.
fn assign_ring_field(nodes: &[NodeRef], distance: impl Fn(Vec3) -> f32) {
    let mut values: HashMap<[u32; 3], f32> = HashMap::new();
    for node in nodes {
        let mut node = node.borrow_mut();
        let vertices = node.vertices;
        for (corner, vertex) in vertices.into_iter().enumerate() {
            let key = vertex.to_array().map(f32::to_bits);
            let value = *values.entry(key).or_insert_with(|| distance(vertex));
            node.seed_distance[corner] = value;
        }
    }
}
