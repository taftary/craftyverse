//! Tests for node UV coordinates: the default triangle, split/unsplit
//! inheritance, and the icosahedral net layout (range, shape, continuity,
//! seams).

use std::collections::HashSet;
use std::rc::Rc;

use glam::{Vec2, Vec3};

use planet_crafter_engine::node::{
    DEFAULT_UV, Node, NodeRef, build_icosphere, destroy_mesh, icosphere_net_uv, split_node,
    unfold_uvs, unsplit_nodes,
};

const EPSILON: f32 = 1e-4;

/// Squared distance between two UV points.
fn uv_dist(a: Vec2, b: Vec2) -> f32 {
    (a - b).length()
}

/// Signed doubled area of a UV triangle (sign = winding).
fn doubled_area(t: &[Vec2; 3]) -> f32 {
    (t[1] - t[0]).perp_dot(t[2] - t[0])
}

/// Whether point `p` lies strictly inside triangle `t`.
fn inside(p: Vec2, t: &[Vec2; 3]) -> bool {
    let sign = doubled_area(t).signum();
    let d0 = (t[1] - t[0]).perp_dot(p - t[0]) * sign;
    let d1 = (t[2] - t[1]).perp_dot(p - t[1]) * sign;
    let d2 = (t[0] - t[2]).perp_dot(p - t[2]) * sign;
    d0 > EPSILON && d1 > EPSILON && d2 > EPSILON
}

/// Whether two segments intersect (proper crossing, not just touching).
fn segments_cross(a0: Vec2, a1: Vec2, b0: Vec2, b1: Vec2) -> bool {
    let d1 = a1 - a0;
    let d2 = b1 - b0;
    let s1 = d1.perp_dot(b0 - a0);
    let s2 = d1.perp_dot(b1 - a0);
    let s3 = d2.perp_dot(a0 - b0);
    let s4 = d2.perp_dot(a1 - b0);
    (s1 * s2 < -EPSILON * EPSILON) && (s3 * s4 < -EPSILON * EPSILON)
}

/// Whether two UV triangles overlap with positive area.
fn triangles_overlap(a: &[Vec2; 3], b: &[Vec2; 3]) -> bool {
    for i in 0..3 {
        let edge_a = (a[i], a[(i + 1) % 3]);
        for j in 0..3 {
            if segments_cross(edge_a.0, edge_a.1, b[j], b[(j + 1) % 3]) {
                return true;
            }
        }
    }
    a.iter().any(|&p| inside(p, b)) || b.iter().any(|&p| inside(p, a))
}

/// The 3D vertex positions of a node, for exact matching across faces.
fn position_set(node: &NodeRef) -> [[u64; 3]; 3] {
    node.borrow().vertices.map(|p| {
        [
            (p.x * 1e5).round() as i64 as u64,
            (p.y * 1e5).round() as i64 as u64,
            (p.z * 1e5).round() as i64 as u64,
        ]
    })
}

/// The distinct 3D vertices shared by two nodes (matched bit-exactly).
fn shared_positions(a: &NodeRef, b: &NodeRef) -> Vec<usize> {
    let pa = position_set(a);
    let pb = position_set(b);
    let mut shared = Vec::new();
    for (index, key) in pa.iter().enumerate() {
        if pb.contains(key) {
            shared.push(index);
        }
    }
    shared
}

#[test]
fn new_nodes_get_the_default_uv_triangle() {
    let node = Node::new(
        "root",
        [
            Vec3::new(0.0, 2.0 / 3.0, 0.0),
            Vec3::new(0.5, -1.0 / 3.0, 0.0),
            Vec3::new(-0.5, -1.0 / 3.0, 0.0),
        ],
        Vec3::ZERO,
    );
    assert_eq!(node.borrow().uv, DEFAULT_UV);

    // The default is an equilateral triangle inside [0, 1]^2.
    let [a, b, c] = DEFAULT_UV;
    for p in [a, b, c] {
        assert!((0.0..=1.0).contains(&p.x) && (0.0..=1.0).contains(&p.y));
    }
    let ab = uv_dist(a, b);
    assert!((uv_dist(b, c) - ab).abs() < EPSILON);
    assert!((uv_dist(c, a) - ab).abs() < EPSILON);
}

#[test]
fn split_interpolates_uvs_linearly() {
    let node = Node::new(
        "root",
        [
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(-1.0, -1.0, 0.0),
        ],
        Vec3::ZERO,
    );
    let uv = [
        Vec2::new(0.2, 0.9),
        Vec2::new(0.8, 0.4),
        Vec2::new(0.1, 0.1),
    ];
    node.borrow_mut().uv = uv;

    let center = split_node(&node.borrow());
    {
        let center_ref = center.borrow();
        // Center links: port 0 -> J, port 1 -> I, port 2 -> K.
        let node_j = center_ref.children[0].as_ref().unwrap();
        let node_i = center_ref.children[1].as_ref().unwrap();
        let node_k = center_ref.children[2].as_ref().unwrap();

        let mid_ab = (uv[0] + uv[1]) / 2.0;
        let mid_bc = (uv[1] + uv[2]) / 2.0;
        let mid_ca = (uv[2] + uv[0]) / 2.0;

        // Corner children keep the parent corner UVs bit-exactly.
        assert_eq!(node_i.borrow().uv, [uv[0], mid_ab, mid_ca]);
        assert_eq!(node_j.borrow().uv, [mid_ab, uv[1], mid_bc]);
        assert_eq!(node_k.borrow().uv, [mid_ca, mid_bc, uv[2]]);
        // The center child is built from the edge midpoints alone.
        assert_eq!(center_ref.uv, [mid_bc, mid_ab, mid_ca]);
    }

    destroy_mesh(&center);
}

#[test]
fn sphere_projection_does_not_apply_to_uvs() {
    // On an icosphere the 3D edge midpoints are projected onto the sphere;
    // the UV midpoints must stay flat linear interpolations.
    let mesh = build_icosphere("test", 1.0, 1, Vec3::ZERO);
    let net = icosphere_net_uv();

    {
        let center = mesh
            .faces
            .iter()
            .find(|face| face.borrow().name == "test.3.C")
            .expect("center child of base face 3");
        let center = center.borrow();
        let parent_uv = net[3];
        assert_eq!(
            center.uv,
            [
                (parent_uv[1] + parent_uv[2]) / 2.0,
                (parent_uv[0] + parent_uv[1]) / 2.0,
                (parent_uv[2] + parent_uv[0]) / 2.0,
            ],
            "UV midpoints must be flat lerps, not sphere-projected"
        );
    }

    destroy_mesh(&mesh.faces[0]);
}

#[test]
fn unsplit_recovers_parent_uvs_exactly() {
    let node = Node::new(
        "root",
        [
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            Vec3::new(-1.0, -1.0, 0.0),
        ],
        Vec3::ZERO,
    );
    let uv = [
        Vec2::new(0.7, 0.6),
        Vec2::new(0.3, 0.8),
        Vec2::new(0.4, 0.2),
    ];
    node.borrow_mut().uv = uv;

    let center = split_node(&node.borrow());
    let parents = unsplit_nodes(&center);
    assert_eq!(parents.len(), 1);
    assert_eq!(parents[0].borrow().uv, uv);
}

#[test]
fn net_layout_is_valid() {
    let net = icosphere_net_uv();

    // Every UV inside [0, 1]^2; all triangles equilateral with equal area.
    let reference_area = doubled_area(&net[0]).abs();
    let reference_edge = uv_dist(net[0][0], net[0][1]);
    for (index, triangle) in net.iter().enumerate() {
        for &p in triangle {
            assert!(
                (0.0..=1.0).contains(&p.x) && (0.0..=1.0).contains(&p.y),
                "face {index} UV {p:?} out of range"
            );
        }
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            assert!(
                (uv_dist(triangle[a], triangle[b]) - reference_edge).abs() < EPSILON,
                "face {index} is not equilateral"
            );
        }
        assert!(
            (doubled_area(triangle).abs() - reference_area).abs() < EPSILON,
            "face {index} has a different area"
        );
    }

    // No two triangles overlap with positive area.
    for (i, a) in net.iter().enumerate() {
        for (j, b) in net.iter().enumerate().skip(i + 1) {
            assert!(!triangles_overlap(a, b), "faces {i} and {j} overlap");
        }
    }

    // Deterministic.
    assert_eq!(net, icosphere_net_uv());
}

#[test]
fn net_continuity_and_seams_match_the_unwrap() {
    // Base faces: two faces are 3D-adjacent when they share exactly two
    // vertices (bit-identical positions by construction).
    let mesh = build_icosphere("test", 1.0, 0, Vec3::ZERO);

    let mut preserved = 0;
    let mut point_cuts = 0;
    let mut full_cuts = 0;
    for (i, face_a) in mesh.faces.iter().enumerate() {
        for face_b in mesh.faces.iter().skip(i + 1) {
            let shared = shared_positions(face_a, face_b);
            if shared.len() != 2 {
                continue;
            }
            let uv_a = face_a.borrow().uv;
            let uv_b = face_b.borrow().uv;
            let pos_b = position_set(face_b);
            // Compare the UVs of each shared 3D vertex on both sides.
            let distances: Vec<f32> = shared
                .iter()
                .map(|&corner| {
                    let key = position_set(face_a)[corner];
                    let other = pos_b.iter().position(|k| *k == key).unwrap();
                    uv_dist(uv_a[corner], uv_b[other])
                })
                .collect();
            if distances.iter().all(|&d| d < EPSILON) {
                preserved += 1;
            } else {
                // A cut edge: the two sides of the edge have different UVs.
                // Ring-adjacent fan triangles still touch at one point in
                // the net (their shared ring vertex has coincident copies),
                // so a cut may be point-continuous without being
                // edge-continuous.
                if distances.iter().any(|&d| d < EPSILON) {
                    point_cuts += 1;
                } else {
                    full_cuts += 1;
                }
            }
        }
    }

    // A connected net of 20 faces preserves exactly 19 of the 30 icosahedron
    // edges; the other 11 are seams. Of the seams, 8 are point-coincident
    // (the ring-adjacent fan pairs, four per pole) and 3 are fully disjoint
    // (the fan wrap-arounds and the strip wrap-around).
    assert_eq!(preserved, 19, "net must be connected through 19 edges");
    assert_eq!(point_cuts, 8, "fan neighbors touch at one point only");
    assert_eq!(full_cuts, 3, "wrap-around edges are full cuts");

    destroy_mesh(&mesh.faces[0]);
}

#[test]
fn subdivision_keeps_net_continuity() {
    // At subdivision 1: links inside one base face are always continuous;
    // links across a preserved base edge stay continuous; links across a cut
    // base edge stay seams.
    let mesh = build_icosphere("test", 1.0, 1, Vec3::ZERO);

    let mut continuous = 0;
    let mut seams = 0;
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    for face in &mesh.faces {
        for child in face.borrow().children.iter().flatten() {
            let key = {
                let (a, b) = (Rc::as_ptr(face) as usize, Rc::as_ptr(child) as usize);
                if a < b { (a, b) } else { (b, a) }
            };
            if !seen.insert(key) {
                continue;
            }
            let shared = shared_positions(face, child);
            assert_eq!(shared.len(), 2, "linked faces must share an edge");
            let uv_a = face.borrow().uv;
            let uv_b = child.borrow().uv;
            let pos_b = position_set(child);
            let pos_a = position_set(face);
            let continuous_edge = shared.iter().all(|&corner| {
                let key = pos_a[corner];
                let other = pos_b.iter().position(|k| *k == key).unwrap();
                uv_dist(uv_a[corner], uv_b[other]) < EPSILON
            });
            if continuous_edge {
                continuous += 1;
            } else {
                seams += 1;
            }
        }
    }

    // 80 leaves -> 120 unique links. Continuous: 60 intra-face + 2 x 19
    // across preserved base edges. Seams: 2 x 11 across cut base edges.
    assert_eq!(continuous, 60 + 38);
    assert_eq!(seams, 22);

    destroy_mesh(&mesh.faces[0]);
}

/// Two right triangles forming a 300 x 200 rectangle, sharing the diagonal
/// edge (300,0,0)-(0,200,0). The shared endpoints are exact f32 literals,
/// so the geometric weld matches bit-exactly.
fn right_triangle_pair() -> [NodeRef; 2] {
    let a = Node::new(
        "a",
        [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(300.0, 0.0, 0.0),
            Vec3::new(0.0, 200.0, 0.0),
        ],
        Vec3::ZERO,
    );
    let b = Node::new(
        "b",
        [
            Vec3::new(300.0, 0.0, 0.0),
            Vec3::new(300.0, 200.0, 0.0),
            Vec3::new(0.0, 200.0, 0.0),
        ],
        Vec3::ZERO,
    );
    [a, b]
}

#[test]
fn unfold_makes_shared_edges_continuous() {
    let [a, b] = right_triangle_pair();
    unfold_uvs(&[a.clone(), b.clone()]);

    // Shared edge: a.B == b.A == (300, 0, 0), a.C == b.C == (0, 200, 0).
    let (uv_a, uv_b) = (a.borrow().uv, b.borrow().uv);
    assert_eq!(uv_a[1], uv_b[0]);
    assert_eq!(uv_a[2], uv_b[2]);
}

#[test]
fn unfold_uses_one_uniform_scale_per_component() {
    // Zero-stretch unfold + one normalization scale: every edge of every
    // triangle of the component has the same uv_length / 3d_length ratio.
    let [a, b] = right_triangle_pair();
    unfold_uvs(&[a.clone(), b.clone()]);

    let mut scales = Vec::new();
    for node in [&a, &b] {
        let node = node.borrow();
        let [pa, pb, pc] = node.vertices;
        let [ua, ub, uc] = node.uv;
        for ((p, q), (u, v)) in [
            ((pa, pb), (ua, ub)),
            ((pb, pc), (ub, uc)),
            ((pc, pa), (uc, ua)),
        ] {
            scales.push((u - v).length() / (q - p).length());
        }
    }
    let reference = scales[0];
    for scale in scales {
        assert!(
            ((scale - reference) / reference).abs() < EPSILON,
            "edge scale {scale} differs from the uniform scale {reference}"
        );
    }
}

#[test]
fn lone_triangle_keeps_the_default_uv() {
    let node = Node::new(
        "lone",
        [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::new(0.0, 100.0, 0.0),
        ],
        Vec3::ZERO,
    );
    // Dirty the UVs: the unfold of a lone triangle resets the default.
    node.borrow_mut().uv = [Vec2::ZERO; 3];

    unfold_uvs(std::slice::from_ref(&node));
    assert_eq!(node.borrow().uv, DEFAULT_UV);
}

#[test]
fn unfold_handles_mixed_connected_and_isolated_nodes() {
    let [a, b] = right_triangle_pair();
    // Far away from the pair: no shared corner, hence no adjacency.
    let lone = Node::new(
        "lone",
        [
            Vec3::new(0.0, 500.0, 0.0),
            Vec3::new(300.0, 500.0, 0.0),
            Vec3::new(0.0, 700.0, 0.0),
        ],
        Vec3::ZERO,
    );
    unfold_uvs(&[a.clone(), b.clone(), lone.clone()]);

    let (uv_a, uv_b) = (a.borrow().uv, b.borrow().uv);
    assert_eq!(uv_a[1], uv_b[0]);
    assert_eq!(uv_a[2], uv_b[2]);
    assert_eq!(lone.borrow().uv, DEFAULT_UV);
}

#[test]
fn tetrahedron_unfold_keeps_a_spanning_tree_of_edges() {
    // A closed tetrahedron: 4 faces, all 6 edges shared bit-exactly.
    let p0 = Vec3::new(0.0, 0.0, 0.0);
    let p1 = Vec3::new(300.0, 0.0, 0.0);
    let p2 = Vec3::new(150.0, 200.0, 0.0);
    let p3 = Vec3::new(150.0, 100.0, 150.0);
    let nodes: Vec<NodeRef> = [
        Node::new("t.0", [p0, p1, p2], Vec3::ZERO),
        Node::new("t.1", [p0, p1, p3], Vec3::ZERO),
        Node::new("t.2", [p1, p2, p3], Vec3::ZERO),
        Node::new("t.3", [p2, p0, p3], Vec3::ZERO),
    ]
    .to_vec();
    unfold_uvs(&nodes);

    // Of the 6 adjacencies, exactly 3 (a spanning tree of the 4 faces) are
    // UV-continuous; the other 3 are seams.
    let mut continuous = 0;
    let mut seams = 0;
    for (i, face_a) in nodes.iter().enumerate() {
        for face_b in nodes.iter().skip(i + 1) {
            let shared = shared_positions(face_a, face_b);
            if shared.len() != 2 {
                continue;
            }
            let uv_a = face_a.borrow().uv;
            let uv_b = face_b.borrow().uv;
            let pos_a = position_set(face_a);
            let pos_b = position_set(face_b);
            let edge_continuous = shared.iter().all(|&corner| {
                let key = pos_a[corner];
                let other = pos_b.iter().position(|k| *k == key).unwrap();
                uv_dist(uv_a[corner], uv_b[other]) < EPSILON
            });
            if edge_continuous {
                continuous += 1;
            } else {
                seams += 1;
            }
        }
    }
    assert_eq!(continuous, 3, "a spanning tree of 4 faces crosses 3 edges");
    assert_eq!(seams, 3, "the uncrossed adjacencies are seams");
}

#[test]
fn unfold_normalizes_into_unit_range() {
    let [a, b] = right_triangle_pair();
    unfold_uvs(&[a.clone(), b.clone()]);

    for node in [&a, &b] {
        for (corner, p) in node.borrow().uv.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(&p.x) && (0.0..=1.0).contains(&p.y),
                "corner {corner} UV {p:?} out of range"
            );
        }
    }
}

#[test]
fn unfold_is_deterministic() {
    let first = right_triangle_pair();
    let second = right_triangle_pair();
    unfold_uvs(&first);
    unfold_uvs(&second);

    for (a, b) in first.iter().zip(second.iter()) {
        assert_eq!(a.borrow().uv, b.borrow().uv);
    }
}
