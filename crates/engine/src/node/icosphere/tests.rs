//! Tests for icosphere construction: closed-form counts, the radius
//! invariant, watertightness, link reciprocity, vertex valences, and
//! cleanup.

use std::collections::HashMap;
use std::rc::Rc;

use glam::Vec3;

use super::{IcosphereMesh, build_icosphere};
use crate::node::topology::reciprocal_index;
use crate::node::{collect_nodes, destroy_mesh};

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
fn all_faces(mesh: &IcosphereMesh) -> Vec<crate::node::NodeRef> {
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
            for point in face.borrow().points {
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
            for point in face.borrow().points {
                assert!(
                    ((point - origin).length() - 2.5).abs() < EPSILON,
                    "vertex {point:?} is not at radius 2.5"
                );
            }
        }
    }
}

#[test]
fn base_faces_have_outward_winding() {
    let mesh = build_icosphere("test", 1.0, 0, Vec3::ZERO);
    for face in all_faces(&mesh) {
        let node = face.borrow();
        let [a, b, c] = node.points;
        let normal = (b - a).cross(c - a);
        assert!(
            normal.dot(node.center) > 0.0,
            "face {} has inward winding",
            node.name
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
fn reciprocal_port_pattern_holds_where_topology_allows() {
    // The `0 <-> 2`, `1 <-> 1` port pattern is not an invariant — it cannot
    // hold on every edge of a closed icosahedron-based mesh (see the module
    // documentation) — but it must still hold on the 24 satisfiable base
    // edges and all internal `split_node` edges. Links are correct
    // regardless, because every link carries a recorded back-port.
    let mesh = build_icosphere("test", 1.0, 1, Vec3::ZERO);
    let all = all_faces(&mesh);
    let mut conforming = 0;
    let mut total = 0;
    for face in &all {
        let node = face.borrow();
        for (port, child) in node.children.iter().enumerate() {
            let child = child.as_ref().unwrap();
            total += 1;
            let back = &child.borrow().children[reciprocal_index(port)];
            if back.as_ref().is_some_and(|back| Rc::ptr_eq(back, face)) {
                conforming += 1;
            }
        }
    }
    assert_eq!(total, 3 * all.len());
    // The 6 non-conforming base edges each split into 2 half-edges with 2
    // directed links apiece, so at most 6 x 2 x 2 = 24 of the 240 directed
    // links may miss the conventional pattern.
    assert!(conforming >= total - 24, "{conforming}/{total} conforming");
}

#[test]
fn destroy_severs_links_via_recorded_back_ports() {
    // Find a face incident to a link whose back-port is not the
    // conventional `reciprocal_index(port)`.
    let mesh = build_icosphere("test", 1.0, 0, Vec3::ZERO);
    let all = all_faces(&mesh);
    let mut found = None;
    for face in &all {
        let node = face.borrow();
        for (port, child) in node.children.iter().enumerate() {
            let child = child.as_ref().unwrap();
            let conforming = child.borrow().children[reciprocal_index(port)]
                .as_ref()
                .is_some_and(|back| Rc::ptr_eq(back, face));
            if !conforming {
                found = Some(Rc::clone(face));
                break;
            }
        }
        if found.is_some() {
            break;
        }
    }
    let face = found.expect("level-0 icosphere has non-conforming links");

    let neighbors: Vec<_> = face
        .borrow()
        .children
        .iter()
        .flatten()
        .map(Rc::clone)
        .collect();
    face.borrow_mut().destroy();

    // All three of the face's links are gone on both sides — including the
    // non-conforming one, whose back-link does not sit at
    // `reciprocal_index(port)`.
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
        for point in face.borrow().points {
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
