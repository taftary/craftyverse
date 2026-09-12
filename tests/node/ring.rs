//! Tests for the ring field: builder seeding (geodesic and planar),
//! cross-leaf continuity, band-spacing uniformity, split interpolation and
//! unsplit recovery.

use std::collections::HashMap;

use glam::Vec3;

use planet_crafter_engine::node::{
    DEFAULT_RING, Node, NodeRef, RING_BANDS, assign_planar_ring_field, build_icosphere,
    destroy_mesh, split_node, split_nodes, unsplit_nodes,
};

/// The doc-example triangle (equilateral, in the z = 0 plane).
fn points() -> [Vec3; 3] {
    [
        Vec3::new(0.0, 2.0 / 3.0, 0.0),
        Vec3::new(0.5, -1.0 / 3.0, 0.0),
        Vec3::new(-0.5, -1.0 / 3.0, 0.0),
    ]
}

/// Signed barycentric coordinates of `p` in the triangle `[a, b, c]`.
fn barycentric(a: Vec3, b: Vec3, c: Vec3, p: Vec3) -> Vec3 {
    let n = (b - a).cross(c - a);
    let area2 = n.length_squared();
    Vec3::new(
        (b - p).cross(c - p).dot(n) / area2,
        (c - p).cross(a - p).dot(n) / area2,
        (a - p).cross(b - p).dot(n) / area2,
    )
}

/// Interpolated ring field at sphere point `p` (the face containing it and
/// the barycentric interpolation of its corner values).
fn field_at(faces: &[NodeRef], p: Vec3) -> Option<f32> {
    // On edges several faces contain the point; the most interior one
    // interpolates instead of extrapolating. The plane-distance filter
    // rejects far-side impostors: an extended triangle plane can intersect
    // the opposite side of the sphere with all-positive weights.
    let radius = p.length();
    let mut best: Option<(f32, f32)> = None; // (minimum bary component, field value)
    for face in faces {
        let node = face.borrow();
        let [a, b, c] = node.vertices;
        let normal = (b - a).cross(c - a).try_normalize().unwrap_or(Vec3::Z);
        if (p - a).dot(normal).abs() > 0.1 * radius {
            continue;
        }
        let bary = barycentric(a, b, c, p);
        let interiority = bary.min_element();
        if interiority < -0.05 {
            continue;
        }
        let ring = node.seed_distance;
        let value = bary.x * ring[0] + bary.y * ring[1] + bary.z * ring[2];
        if best
            .as_ref()
            .is_none_or(|(best_interiority, _)| interiority > *best_interiority)
        {
            best = Some((interiority, value));
        }
    }
    best.map(|(_, value)| value)
}

/// Distinct corner positions of the mesh, with one field value per
/// position: `(position, value, sharing corner count)`.
fn position_values(faces: &[NodeRef]) -> Vec<(Vec3, f32, usize)> {
    let mut clusters: HashMap<[u32; 3], (f32, usize)> = HashMap::new();
    for face in faces {
        let node = face.borrow();
        for (corner, vertex) in node.vertices.iter().enumerate() {
            let key = vertex.to_array().map(f32::to_bits);
            let entry = clusters
                .entry(key)
                .or_insert((node.seed_distance[corner], 0));
            assert_eq!(
                entry.0, node.seed_distance[corner],
                "ring field discontinuity at {vertex:?}"
            );
            entry.1 += 1;
        }
    }
    clusters
        .into_iter()
        .map(|(bits, (value, count))| (Vec3::from_array(bits.map(f32::from_bits)), value, count))
        .collect()
}

#[test]
fn new_node_defaults_to_zero_ring() {
    let node = Node::new("root", points(), Vec3::ZERO);
    assert_eq!(node.borrow().seed_distance, DEFAULT_RING);
}

#[test]
fn icosphere_seeds_zero_at_base_vertices() {
    for level in [0, 1, 2] {
        let mesh = build_icosphere("planet", 300.0, level, Vec3::ZERO);
        let values = position_values(&mesh.faces);
        // The 12 base vertices are the valence-5 positions; each is a seed.
        let seeds: Vec<_> = values.iter().filter(|(_, _, count)| *count == 5).collect();
        assert_eq!(seeds.len(), 12, "level {level}: expected 12 base vertices");
        for (position, value, _) in seeds {
            assert_eq!(
                *value, 0.0,
                "level {level}: seed vertex {position:?} not in band zero"
            );
        }
        destroy_mesh(&mesh.faces[0]);
    }
}

#[test]
fn icosphere_ring_bands_are_uniformly_spaced() {
    let radius = 300.0;
    let mesh = build_icosphere("planet", radius, 1, Vec3::ZERO);
    let values = position_values(&mesh.faces);

    // Walk the meridian from the top base vertex toward an adjacent one:
    // the field must increase monotonically (up to the Voronoi boundary at
    // half the base-edge arc) and band transitions must land at constant
    // arc intervals — the distribution the user sees.
    let seeds: Vec<Vec3> = values
        .iter()
        .filter(|(_, _, count)| *count == 5)
        .map(|(p, _, _)| *p)
        .collect();
    let top = seeds
        .iter()
        .max_by(|a, b| a.y.partial_cmp(&b.y).unwrap())
        .copied()
        .unwrap();
    let top_dir = top.normalize();
    let neighbor = seeds
        .iter()
        .filter(|s| **s != top)
        .min_by(|a, b| {
            (**a - top)
                .length()
                .partial_cmp(&(**b - top).length())
                .unwrap()
        })
        .copied()
        .unwrap();
    let neighbor_dir = neighbor.normalize();
    let tangent = (neighbor_dir - top_dir * neighbor_dir.dot(top_dir)).normalize();
    let edge_arc = top_dir.dot(neighbor_dir).clamp(-1.0, 1.0).acos();
    let band_angle = edge_arc / RING_BANDS as f32;

    let mut previous_value = 0.0_f32;
    let mut transitions: Vec<f32> = Vec::new();
    let mut previous_band = 0i64;
    for step in 0..=48 {
        // Sweep to just below half the edge arc (the Voronoi boundary,
        // where the nearest seed changes and the field turns back down).
        let theta = step as f32 * 0.01 * edge_arc;
        let dir = top_dir * theta.cos() + tangent * theta.sin();
        let value = field_at(&mesh.faces, dir * radius).expect("point on the mesh");
        assert!(
            value >= previous_value - 0.05,
            "field decreased at theta={theta}: {previous_value} -> {value}"
        );
        previous_value = value;
        let band = value.floor() as i64;
        if band != previous_band {
            transitions.push(theta);
            previous_band = band;
        }
    }
    // Every band gap is the constant band angle, within the chord-space
    // interpolation tolerance of the coarse mesh.
    assert!(transitions.len() >= 3, "expected several band transitions");
    for gap in transitions.windows(2).map(|w| w[1] - w[0]) {
        assert!(
            (0.7..=1.3).contains(&(gap / band_angle)),
            "band gap {gap} vs expected {band_angle}"
        );
    }
    destroy_mesh(&mesh.faces[0]);
}

#[test]
fn icosphere_base_edge_midpoint_is_half_way_in_bands() {
    let mesh = build_icosphere("planet", 300.0, 1, Vec3::ZERO);
    let values = position_values(&mesh.faces);
    // At level 1 the valence-6 positions are exactly the 30 base-edge
    // midpoints: equidistant to the two endpoint seeds — half the edge
    // arc, i.e. RING_BANDS / 2 bands.
    let midpoint_value = (RING_BANDS as f32) / 2.0;
    let valence_six: Vec<_> = values.iter().filter(|(_, _, count)| *count == 6).collect();
    assert_eq!(valence_six.len(), 30);
    for (position, value, _) in &valence_six {
        assert!(
            (*value - midpoint_value).abs() < 1e-3,
            "base-edge midpoint {position:?} at {value} bands, expected {midpoint_value}"
        );
    }
    destroy_mesh(&mesh.faces[0]);
}

#[test]
fn planar_seeding_is_continuous_and_exact() {
    // One quad, two triangles sharing the diagonal, seeds at its corners.
    let p00 = Vec3::new(0.0, 0.0, 0.0);
    let p10 = Vec3::new(100.0, 0.0, 0.0);
    let p01 = Vec3::new(0.0, 100.0, 0.0);
    let p11 = Vec3::new(100.0, 100.0, 0.0);
    let nodes = vec![
        Node::new("quad.a", [p00, p11, p10], Vec3::ZERO),
        Node::new("quad.b", [p00, p01, p11], Vec3::ZERO),
    ];
    let seeds = [p00, p10, p01, p11];
    assign_planar_ring_field(&nodes, &seeds, 50.0);

    // Corners at seeds are zero; p11's diagonal appears in both triangles
    // with the same value (continuity); distances are exact.
    let a = nodes[0].borrow();
    let b = nodes[1].borrow();
    assert_eq!(a.seed_distance[0], 0.0); // p00
    assert_eq!(a.seed_distance[2], 0.0); // p10
    assert_eq!(b.seed_distance[1], 0.0); // p01
    assert_eq!(a.seed_distance[1], 0.0); // p11 is a seed
    assert_eq!(a.seed_distance[1], b.seed_distance[2]);
    drop(a);
    drop(b);
    for node in &nodes {
        node.borrow_mut().destroy();
    }
}

#[test]
fn split_interpolates_ring_with_flat_midpoints() {
    let node = Node::new("root", points(), Vec3::ZERO);
    node.borrow_mut().seed_distance = [0.0, 2.0, 4.0];
    let center = split_node(&node.borrow());
    // The center child gets the midpoint triple [mBC, mAB, mCA].
    assert_eq!(center.borrow().seed_distance, [3.0, 1.0, 2.0]);
    // Center ports link to J, I, K in that order (see split_node): corner
    // children get [A, mAB, mCA], [mAB, B, mBC], [mCA, mBC, C].
    let expected: [[f32; 3]; 3] = [[1.0, 2.0, 3.0], [0.0, 1.0, 2.0], [2.0, 3.0, 4.0]];
    for (child, want) in center.borrow().children.iter().zip(expected) {
        assert_eq!(child.as_ref().unwrap().borrow().seed_distance, want);
    }
    destroy_mesh(&center);
}

#[test]
fn unsplit_recovers_parent_ring() {
    let node = Node::new("root", points(), Vec3::ZERO);
    node.borrow_mut().seed_distance = [1.0, 3.0, 5.0];
    let leaves = split_nodes(&node);
    let parents = unsplit_nodes(&leaves[0]);
    assert_eq!(parents.len(), 1);
    assert_eq!(parents[0].borrow().seed_distance, [1.0, 3.0, 5.0]);
}
