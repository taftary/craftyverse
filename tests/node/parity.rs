//! Tests for topology parity: the default seed, the split propagation
//! (corner children inherit, the center child flips), the unsplit recovery,
//! and the closed-form predictability across subdivision levels.

use glam::Vec3;

use planet_crafter_engine::node::{
    Node, Parity, build_icosphere, destroy_mesh, split_node, split_nodes, unsplit_nodes,
};

/// The doc-example triangle (equilateral, in the z = 0 plane).
fn points() -> [Vec3; 3] {
    [
        Vec3::new(0.0, 2.0 / 3.0, 0.0),
        Vec3::new(0.5, -1.0 / 3.0, 0.0),
        Vec3::new(-0.5, -1.0 / 3.0, 0.0),
    ]
}

#[test]
fn parity_sign_flip_and_from_sign() {
    assert_eq!(Parity::Abc.sign(), 1);
    assert_eq!(Parity::Acb.sign(), -1);
    assert_eq!(Parity::Abc.flipped(), Parity::Acb);
    assert_eq!(Parity::Acb.flipped(), Parity::Abc);
    assert_eq!(Parity::from_sign(1.0), Parity::Abc);
    assert_eq!(Parity::from_sign(-1.0), Parity::Acb);
    // A degenerate (zero) reference sign maps to `Abc`.
    assert_eq!(Parity::from_sign(0.0), Parity::Abc);
}

#[test]
fn new_node_defaults_to_abc() {
    let node = Node::new("root", points(), Vec3::ZERO);
    assert_eq!(node.borrow().parity, Parity::Abc);
}

#[test]
fn split_propagates_parity_for_both_windings() {
    for parent in [Parity::Abc, Parity::Acb] {
        let node = Node::new("root", points(), Vec3::ZERO);
        node.borrow_mut().parity = parent;
        let center = split_node(&node.borrow());
        // The center child flips; every corner child inherits.
        assert_eq!(center.borrow().parity, parent.flipped());
        for corner in center.borrow().children.iter().flatten() {
            assert_eq!(corner.borrow().parity, parent);
        }
        destroy_mesh(&center);
    }
}

#[test]
fn split_nodes_keeps_parity_predictable_at_mesh_level() {
    let node = Node::new("root", points(), Vec3::ZERO);
    let leaves = split_nodes(&node);
    assert_eq!(leaves.len(), 4);
    for leaf in &leaves {
        let leaf = leaf.borrow();
        let expected = if leaf.name.ends_with(".C") {
            Parity::Acb
        } else {
            Parity::Abc
        };
        assert_eq!(leaf.parity, expected, "node {}", leaf.name);
    }

    // One more generation: parity is the base parity times (-1) per `.C`
    // segment in the name path.
    let leaves = split_nodes(&leaves[0]);
    assert_eq!(leaves.len(), 16);
    for leaf in &leaves {
        let leaf = leaf.borrow();
        let flips = leaf.name.matches(".C").count();
        let expected = if flips % 2 == 0 {
            Parity::Abc
        } else {
            Parity::Acb
        };
        assert_eq!(leaf.parity, expected, "node {}", leaf.name);
    }
    destroy_mesh(&leaves[0]);
}

#[test]
fn unsplit_recovers_parent_parity() {
    for parent in [Parity::Abc, Parity::Acb] {
        let node = Node::new("root", points(), Vec3::ZERO);
        node.borrow_mut().parity = parent;
        let leaves = split_nodes(&node);
        let parents = unsplit_nodes(&leaves[0]);
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0].borrow().name, "root");
        assert_eq!(parents[0].borrow().parity, parent);
    }
}

#[test]
fn icosphere_parity_is_predictable_across_levels() {
    for level in [0, 1, 2] {
        let mesh = build_icosphere("planet", 1.0, level, Vec3::ZERO);
        for face in &mesh.faces {
            let face = face.borrow();
            // Names are "planet.{base}(.I|.J|.K|.C)*": the base face index
            // gives the base parity (the 5 reversed faces are `Acb`), then
            // every `.C` segment flips it once.
            let mut parts = face.name.split('.');
            let _prefix = parts.next();
            let base: usize = parts
                .next()
                .expect("base face index in the name")
                .parse()
                .expect("numeric base face index");
            let base_parity = if [6, 8, 11, 14, 18].contains(&base) {
                Parity::Acb
            } else {
                Parity::Abc
            };
            let flips = parts.filter(|&suffix| suffix == "C").count();
            let expected = if flips % 2 == 0 {
                base_parity
            } else {
                base_parity.flipped()
            };
            assert_eq!(face.parity, expected, "face {}", face.name);
        }
        destroy_mesh(&mesh.faces[0]);
    }
}
