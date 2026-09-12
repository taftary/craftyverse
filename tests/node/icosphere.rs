//! Tests for icosphere construction: closed-form counts, the radius
//! invariant, watertightness, link reciprocity, vertex valences, and
//! cleanup.

use std::collections::HashMap;
use std::rc::Rc;

use glam::Vec3;

use planet_crafter_engine::node::{
    IcosphereMesh, NodeRef, Parity, build_icosphere, collect_nodes, destroy_mesh,
};

const EPSILON: f32 = 1e-3;

/// Quantizes a position so that bit-identical welded vertices (and only
/// those) share a key. Welded vertices are computed once and reused, so
/// exact keys are correct; the rounding only guards against platform float
/// noise in the base vertices.
fn position_key(p: Vec3) -> [i64; 3] {
    const SCALE: f32 = 1e5;
    [
        (p.x * SCALE).round() as i64,
        (p.y * SCALE).round() as i64,
        (p.z * SCALE).round() as i64,
    ]
}

/// Collects all leaf nodes reachable from the first face.
fn all_faces(mesh: &IcosphereMesh) -> Vec<NodeRef> {
    collect_nodes(&mesh.faces[0])
}

#[test]
fn face_and_vertex_counts_follow_euler() {
    for subdivisions in 0..=3 {
        let mesh = build_icosphere("test", 1.0, subdivisions, Vec3::ZERO);
        let expected_faces = 20 * 4_usize.pow(subdivisions);
        assert_eq!(mesh.vertex_count, 10 * 4_usize.pow(subdivisions) + 2);
        assert_eq!(mesh.faces.len(), expected_faces);

        let all = all_faces(&mesh);
        assert_eq!(all.len(), expected_faces);

        let mut vertices = std::collections::HashSet::new();
        for face in &all {
            for point in face.borrow().vertices {
                vertices.insert(position_key(point));
            }
        }
        assert_eq!(vertices.len(), mesh.vertex_count);
    }
}

#[test]
fn every_vertex_lies_on_the_sphere() {
    for subdivisions in 0..=2 {
        let origin = Vec3::new(1.0, -2.0, 3.0);
        let mesh = build_icosphere("test", 2.5, subdivisions, origin);
        for face in all_faces(&mesh) {
            for point in face.borrow().vertices {
                assert!(
                    ((point - origin).length() - 2.5).abs() < EPSILON,
                    "vertex {point:?} is not at radius 2.5"
                );
            }
        }
    }
}

#[test]
fn base_face_winding_matches_the_port_pattern_labeling() {
    // Full port-pattern conformance on all 30 base edges requires 5 of the
    // 20 base faces to be wound inward (see the icosphere module docs).
    const INWARD: [usize; 5] = [6, 8, 11, 14, 18];
    let mesh = build_icosphere("test", 1.0, 0, Vec3::ZERO);
    for face in all_faces(&mesh) {
        let node = face.borrow();
        let index: usize = node.name.rsplit('.').next().unwrap().parse().unwrap();
        let [a, b, c] = node.vertices;
        let normal = (b - a).cross(c - a);
        let outward = normal.dot(node.center) > 0.0;
        assert_eq!(
            outward,
            !INWARD.contains(&index),
            "face {index} has unexpected winding"
        );
    }
}

#[test]
fn mesh_is_watertight() {
    for subdivisions in 0..=2 {
        let mesh = build_icosphere("test", 1.0, subdivisions, Vec3::ZERO);
        for face in all_faces(&mesh) {
            assert!(
                face.borrow().children.iter().all(|c| c.is_some()),
                "face {} has an open port",
                face.borrow().name
            );
        }
    }
}

#[test]
fn links_are_bidirectional() {
    let mesh = build_icosphere("test", 1.0, 2, Vec3::ZERO);
    for face in all_faces(&mesh) {
        let node = face.borrow();
        for (port, child) in node.children.iter().enumerate() {
            let child = child.as_ref().unwrap();
            let linked_back = child
                .borrow()
                .children
                .iter()
                .flatten()
                .any(|back| Rc::ptr_eq(back, &face));
            assert!(
                linked_back,
                "link {} -> {} (port {port}) is not reciprocated",
                node.name,
                child.borrow().name
            );
        }
    }
}

#[test]
fn reciprocal_port_pattern_holds_on_every_link() {
    // The abc/acb base-face labeling makes the `0 <-> 2`, `1 <-> 1` port
    // pattern satisfiable on all 30 base edges, and every split/weld keeps
    // it: at every level, a link through port `x` uses port `2 - x` on the
    // other side. Links still carry a recorded back-port; the pattern is
    // asserted here, not assumed by the code.
    for subdivisions in 0..=3 {
        let mesh = build_icosphere("test", 1.0, subdivisions, Vec3::ZERO);
        for face in all_faces(&mesh) {
            let node = face.borrow();
            for (port, child) in node.children.iter().enumerate() {
                let child = child.as_ref().unwrap();
                assert!(
                    child.borrow().children[2 - port]
                        .as_ref()
                        .is_some_and(|back| Rc::ptr_eq(back, &face)),
                    "link {} port {port} breaks the reciprocal port pattern",
                    node.name,
                );
            }
        }
    }
}

#[test]
fn destroy_severs_links_via_recorded_back_ports() {
    let mesh = build_icosphere("test", 1.0, 0, Vec3::ZERO);
    let all = all_faces(&mesh);
    let face = Rc::clone(&mesh.faces[0]);

    let neighbors: Vec<_> = face
        .borrow()
        .children
        .iter()
        .flatten()
        .map(Rc::clone)
        .collect();
    face.borrow_mut().destroy();

    // All three of the face's links are gone on both sides.
    assert!(face.borrow().children.iter().all(|c| c.is_none()));
    for neighbor in &neighbors {
        let node = neighbor.borrow();
        assert!(
            node.children
                .iter()
                .flatten()
                .all(|back| !Rc::ptr_eq(back, &face)),
            "neighbor still links back to the destroyed face"
        );
    }
    // Every other link in the mesh is untouched.
    let touched: std::collections::HashSet<_> = neighbors
        .iter()
        .map(Rc::as_ptr)
        .chain([Rc::as_ptr(&face)])
        .collect();
    for other in &all {
        if touched.contains(&Rc::as_ptr(other)) {
            continue;
        }
        assert!(
            other.borrow().children.iter().all(|c| c.is_some()),
            "unrelated link was cleared"
        );
    }
}

#[test]
fn vertex_valences_are_five_or_six() {
    let mesh = build_icosphere("test", 1.0, 2, Vec3::ZERO);
    let mut valences: HashMap<[i64; 3], usize> = HashMap::new();
    for face in all_faces(&mesh) {
        // Each triangle corner at a vertex corresponds to one incident
        // triangle, so the corner count at a vertex equals its valence.
        for point in face.borrow().vertices {
            *valences.entry(position_key(point)).or_insert(0) += 1;
        }
    }
    let mut fives = 0;
    let mut sixes = 0;
    for valence in valences.values() {
        match *valence {
            5 => fives += 1,
            6 => sixes += 1,
            other => panic!("unexpected valence {other}"),
        }
    }
    assert_eq!(fives, 12);
    assert_eq!(sixes + fives, mesh.vertex_count);
}

#[test]
fn all_faces_are_at_the_subdivision_level() {
    let mesh = build_icosphere("test", 1.0, 3, Vec3::ZERO);
    for face in all_faces(&mesh) {
        assert_eq!(face.borrow().level, 3);
    }
}

#[test]
fn subdivisions_zero_returns_the_linked_base_mesh() {
    let mesh = build_icosphere("test", 1.0, 0, Vec3::ZERO);
    assert_eq!(mesh.faces.len(), 20);
    assert!(mesh.faces.iter().all(|face| face.borrow().level == 0));
}

#[test]
fn cleanup_clears_every_link() {
    let mesh = build_icosphere("test", 1.0, 2, Vec3::ZERO);
    let all = all_faces(&mesh);
    destroy_mesh(&mesh.faces[0]);
    for face in &all {
        assert!(face.borrow().children.iter().all(|c| c.is_none()));
    }
}

#[test]
fn node_names_carry_the_prefix_and_suffixes() {
    let mesh = build_icosphere("planet", 1.0, 1, Vec3::ZERO);
    for face in all_faces(&mesh) {
        let name = &face.borrow().name;
        let suffix = name.rsplit('.').next().unwrap();
        assert!(
            name.starts_with("planet.") && ["I", "J", "K", "C"].contains(&suffix),
            "unexpected node name {name}"
        );
    }
}

#[test]
fn every_link_has_a_consistent_back_port() {
    let mesh = build_icosphere("test", 1.0, 2, Vec3::ZERO);
    for face in all_faces(&mesh) {
        let node = face.borrow();
        for (port, child) in node.children.iter().enumerate() {
            let child = child.as_ref().unwrap();
            let back = node.back_ports[port].expect("missing back-port record");
            let child = child.borrow();
            assert!(
                child.children[back]
                    .as_ref()
                    .is_some_and(|c| Rc::ptr_eq(c, &face)),
                "back-port does not point back"
            );
            assert_eq!(child.back_ports[back], Some(port));
        }
    }
}

#[test]
fn base_faces_seed_parity_from_their_winding() {
    let mesh = build_icosphere("planet", 1.0, 0, Vec3::ZERO);
    assert_eq!(mesh.faces.len(), 20);
    let mut acb_count = 0;
    for face in &mesh.faces {
        let face = face.borrow();
        let [a, b, c] = face.vertices;
        // The geometric winding: face normal vs. the radial direction (the
        // origin is `Vec3::ZERO`). Outward-wound faces are `Abc`.
        let winding = (b - a).cross(c - a).dot(face.center);
        let expected = if winding < 0.0 {
            Parity::Acb
        } else {
            Parity::Abc
        };
        assert_eq!(face.parity, expected, "face {}", face.name);
        if face.parity == Parity::Acb {
            acb_count += 1;
        }
    }
    // The 5 deliberately inward-wound faces (see the icosphere spec).
    assert_eq!(acb_count, 5);
    destroy_mesh(&mesh.faces[0]);
}
