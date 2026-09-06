use super::*;
use crate::test_utils::{point, test_node};

const EPSILON: f32 = 1e-4;
const SQRT_3_2: f32 = 0.866_025_4; // sqrt(3) / 2

fn approx_eq(a: Vec3, b: Vec3) -> bool {
    (a - b).length() < EPSILON
}

#[test]
fn new_initializes_identity_and_geometry() {
    let node = test_node();
    let node = node.borrow();

    assert_eq!(node.name, "root");
    assert_eq!(node.level, 0);
    assert!(approx_eq(node.center, Vec3::ZERO));
    assert!(approx_eq(node.direction_to_origin, point(0.0, 1000.0)));
    assert!(approx_eq(node.direction_of_node, Vec3::Y));
    assert_eq!(node.base_length, 300.0);
    assert_eq!(node.height, 300.0 * 3.0_f32.sqrt() / 2.0);
    assert!(node.children.iter().all(|slot| slot.is_none()));
}

#[test]
fn new_normalizes_direction_of_node() {
    let node = Node::new(
        "scaled",
        [point(0.0, 2.0), point(1.5, -1.0), point(-1.5, -1.0)],
        Vec3::ZERO,
    );
    assert!(approx_eq(node.borrow().direction_of_node, Vec3::Y));
}

#[test]
fn new_builds_equilateral_triangle_around_center() {
    let node = test_node();
    let node = node.borrow();
    let [a, b, c] = node.points;

    // Centroid is the center, base BC has the requested length.
    assert!(approx_eq((a + b + c) / 3.0, node.center));
    assert!(((b - c).length() - 300.0).abs() < EPSILON);
    let side = (a - b).length();
    assert!(((a - b).length() - (b - c).length()).abs() < EPSILON);
    assert!(((c - a).length() - side).abs() < EPSILON);

    // Apex up: A on top, B bottom-right, C bottom-left.
    assert!(a.y > 0.0 && approx_eq(point(a.x, 0.0), Vec3::ZERO));
    assert!(b.x > 0.0 && b.y < 0.0);
    assert!(c.x < 0.0 && c.y < 0.0);
}

#[test]
fn new_derives_altitude_and_dimensions_from_triangle_points() {
    let node = Node::new(
        "iso",
        [
            point(2.0 * 200.0 / 3.0, 0.0),
            point(-200.0 / 3.0, -150.0),
            point(-200.0 / 3.0, 150.0),
        ],
        Vec3::ZERO,
    );
    let node = node.borrow();
    let [a, b, c] = node.points;

    // Direction stored normalized, centroid is the center.
    assert!(approx_eq(node.direction_of_node, Vec3::X));
    assert!(approx_eq((a + b + c) / 3.0, Vec3::ZERO));

    // Apex A is 2/3 of the height along the direction, the base midpoint
    // 1/3 against it; BC is perpendicular to the direction, B and C half
    // the base length away on the perpendicular axis.
    assert!(approx_eq(a, point(2.0 * 200.0 / 3.0, 0.0)));
    assert!(approx_eq(b, point(-200.0 / 3.0, -150.0)));
    assert!(approx_eq(c, point(-200.0 / 3.0, 150.0)));

    // Height is the perpendicular distance from A to line BC.
    assert!(((a - (b + c) / 2.0).length() - 200.0).abs() < EPSILON);
}

#[test]
fn new_supports_scalene_triangles() {
    let node = Node::new(
        "scalene",
        [point(1.0, 3.0), point(0.0, 0.0), point(4.0, 0.0)],
        Vec3::ZERO,
    );
    let node = node.borrow();

    assert!(approx_eq(node.direction_of_node, Vec3::Y));
    assert_eq!(node.base_length, 4.0);
    assert_eq!(node.height, 3.0);
}

#[test]
fn point_order_controls_direction_labels() {
    let normal = Node::new(
        "normal",
        [point(0.0, 2.0), point(1.5, -1.0), point(-1.5, -1.0)],
        Vec3::ZERO,
    );
    let mirrored = Node::new(
        "mirrored",
        [point(0.0, 2.0), point(-1.5, -1.0), point(1.5, -1.0)],
        Vec3::ZERO,
    );
    let normal = normal.borrow();
    let mirrored = mirrored.borrow();

    assert!(approx_eq(normal.points[1], mirrored.points[2]));
    assert!(approx_eq(normal.points[2], mirrored.points[1]));
    assert!(approx_eq(normal.directions[0], mirrored.directions[2]));
    assert!(approx_eq(normal.directions[2], mirrored.directions[0]));
    assert!(approx_eq(normal.directions[1], mirrored.directions[1]));
}

#[test]
fn equilateral_directions_match_expected_orientation() {
    let node = test_node();
    let [i, j, k] = node.borrow().directions;

    // I up-right (perpendicular to AB), J straight down (perpendicular to BC),
    assert!(approx_eq(i, point(SQRT_3_2, 0.5)));
    assert!(approx_eq(j, point(0.0, -1.0)));
    assert!(approx_eq(k, point(-SQRT_3_2, 0.5)));
}

#[test]
fn directions_are_perpendicular_and_point_toward_edges() {
    let node = test_node();
    let node = node.borrow();
    let [a, b, c] = node.points;
    let [i, j, k] = node.directions;

    // Uniform rule: I perpendicular to AB, J perpendicular to BC, K
    for (direction, edge_start, edge_end) in [(i, a, b), (j, b, c), (k, c, a)] {
        let edge = edge_end - edge_start;
        assert!(direction.dot(edge).abs() < EPSILON);
        let edge_mid = (edge_start + edge_end) / 2.0;
        assert!(direction.dot(edge_mid - node.center) > 0.0);
        assert!((direction.length() - 1.0).abs() < EPSILON);
    }
}

#[test]
fn split_returns_center_node_with_incremented_level() {
    let node = test_node();
    let center = split_node(&node.borrow());

    assert_eq!(center.borrow().level, 1);
    assert_eq!(center.borrow().name, "root.C");
    // The symmetric fixture produces the opposite center altitude.
    assert!(approx_eq(center.borrow().direction_of_node, -Vec3::Y));
    // The parent node keeps its own level.
    assert_eq!(node.borrow().level, 0);
}

#[test]
fn split_connects_center_and_corner_nodes_bidirectionally() {
    let node = test_node();
    let center = split_node(&node.borrow());
    let center_ref = center.borrow();

    // Each center port faces the corner across its edge: I -> node J,
    // J -> node I, K -> node K.
    let node_j = center_ref.children[0].as_ref().expect("node J");
    let node_i = center_ref.children[1].as_ref().expect("node I");
    let node_k = center_ref.children[2].as_ref().expect("node K");

    assert!(Rc::ptr_eq(
        node_j.borrow().children[2].as_ref().unwrap(),
        &center
    ));
    assert!(Rc::ptr_eq(
        node_i.borrow().children[1].as_ref().unwrap(),
        &center
    ));
    assert!(Rc::ptr_eq(
        node_k.borrow().children[0].as_ref().unwrap(),
        &center
    ));

    // Corner nodes keep the parent's direction_of_node and derive unique
    // names.
    for (corner, suffix) in [(node_i, "root.I"), (node_j, "root.J"), (node_k, "root.K")] {
        let corner = corner.borrow();
        assert!(approx_eq(corner.direction_of_node, Vec3::Y));
        assert_eq!(corner.name, suffix);
        assert_eq!(corner.level, 1);
    }

    // No cross-connections between corner nodes: only the port facing the
    // center node is linked.
    for (corner, linked) in [(node_i, 1), (node_j, 2), (node_k, 0)] {
        for index in 0..3 {
            let corner = corner.borrow();
            match index == linked {
                true => assert!(corner.children[index].is_some()),
                false => assert!(corner.children[index].is_none()),
            }
        }
    }
}

#[test]
fn split_subdivides_points() {
    let node = test_node();
    let [p_a, p_b, p_c] = node.borrow().points;
    let center = split_node(&node.borrow());

    let p_ab = (p_a + p_b) / 2.0;
    let p_bc = (p_b + p_c) / 2.0;
    let p_ca = (p_c + p_a) / 2.0;

    let center_ref = center.borrow();
    // Center node: inverted orientation [pBC, pAB, pCA].
    for (actual, expected) in center_ref.points.iter().zip([p_bc, p_ab, p_ca]) {
        assert!(approx_eq(*actual, expected));
    }
    // Corner nodes: the corner point keeps its letter, the midpoint toward
    // a neighbor takes that neighbor's letter.
    let node_i = center_ref.children[1].as_ref().unwrap().borrow();
    for (actual, expected) in node_i.points.iter().zip([p_a, p_ab, p_ca]) {
        assert!(approx_eq(*actual, expected));
    }
    let node_j = center_ref.children[0].as_ref().unwrap().borrow();
    for (actual, expected) in node_j.points.iter().zip([p_ab, p_b, p_bc]) {
        assert!(approx_eq(*actual, expected));
    }
    let node_k = center_ref.children[2].as_ref().unwrap().borrow();
    for (actual, expected) in node_k.points.iter().zip([p_ca, p_bc, p_c]) {
        assert!(approx_eq(*actual, expected));
    }
}

#[test]
fn split_derives_dimensions_from_child_points() {
    let node = test_node();
    let center = split_node(&node.borrow());
    let center_ref = center.borrow();

    // Stored dimensions match each child's actual point geometry.
    for slot in &center_ref.children {
        let corner = slot.as_ref().unwrap().borrow();
        let [a, b, c] = corner.points;
        assert!(((b - c).length() - corner.base_length).abs() < EPSILON);
        let base_direction = (c - b).normalize();
        let base_projection = b + base_direction * (a - b).dot(base_direction);
        assert!(((a - base_projection).length() - corner.height).abs() < EPSILON);
    }
    let [a, b, c] = center_ref.points;
    assert!(((b - c).length() - center_ref.base_length).abs() < EPSILON);
    let base_direction = (c - b).normalize();
    let base_projection = b + base_direction * (a - b).dot(base_direction);
    assert!(((a - base_projection).length() - center_ref.height).abs() < EPSILON);
}

#[test]
fn split_corner_nodes_keep_parent_orientation() {
    let node = test_node();
    let center = split_node(&node.borrow());
    let center_ref = center.borrow();

    // All corner nodes keep the parent's direction_of_node and have the
    // I up-right (perpendicular to AB), J straight down (perpendicular to BC),
    // K up-left (perpendicular to CA).
    for slot in &center_ref.children {
        let corner = slot.as_ref().unwrap().borrow();
        assert!(approx_eq(corner.direction_of_node, Vec3::Y));
        let [i, j, k] = corner.directions;
        assert!(approx_eq(i, point(SQRT_3_2, 0.5)));
        assert!(approx_eq(j, point(0.0, -1.0)));
        assert!(approx_eq(k, point(-SQRT_3_2, 0.5)));
    }
}

#[test]
fn split_center_node_has_mirrored_orientation() {
    let node = test_node();
    let center = split_node(&node.borrow());
    let center_ref = center.borrow();

    assert!(approx_eq(center_ref.direction_of_node, -Vec3::Y));
    // Center node: I down-right, J straight up, K down-left.
    let [i, j, k] = center_ref.directions;
    assert!(approx_eq(i, point(SQRT_3_2, -0.5)));
    assert!(approx_eq(j, point(0.0, 1.0)));
    assert!(approx_eq(k, point(-SQRT_3_2, -0.5)));
}

#[test]
fn split_recomputes_directions_from_own_points() {
    let node = test_node();
    let center = split_node(&node.borrow());
    let center_ref = center.borrow();
    let [a, b, c] = center_ref.points;
    let [i, j, k] = center_ref.directions;

    // The uniform rule is computed from the node's own points triplet:
    // I perpendicular to AB, J perpendicular to BC, K perpendicular to CA,
    // all pointing toward their edge.
    for (direction, edge_start, edge_end) in [(i, a, b), (j, b, c), (k, c, a)] {
        assert!(direction.dot(edge_end - edge_start).abs() < EPSILON);
        let edge_mid = (edge_start + edge_end) / 2.0;
        assert!(direction.dot(edge_mid - center_ref.center) > 0.0);
    }
}

#[test]
fn split_can_be_called_multiple_times() {
    let node = test_node();
    let first = split_node(&node.borrow());
    let second = split_node(&node.borrow());

    // Each call produces four new nodes; the returned centers are distinct.
    assert!(!Rc::ptr_eq(&first, &second));
    assert_eq!(first.borrow().level, 1);
    assert_eq!(second.borrow().level, 1);
}

#[test]
fn destroy_severs_all_bidirectional_links() {
    let node = test_node();
    let center = split_node(&node.borrow());
    let (node_j, node_i, node_k) = {
        let center_ref = center.borrow();
        (
            Rc::clone(center_ref.children[0].as_ref().unwrap()),
            Rc::clone(center_ref.children[1].as_ref().unwrap()),
            Rc::clone(center_ref.children[2].as_ref().unwrap()),
        )
    };

    center.borrow_mut().destroy();

    // All of the center node's links are cleared.
    assert!(center.borrow().children.iter().all(|slot| slot.is_none()));
    // Each corner node's reciprocal back-link is cleared too.
    assert!(node_j.borrow().children[2].is_none());
    assert!(node_i.borrow().children[1].is_none());
    assert!(node_k.borrow().children[0].is_none());
    // With the links gone, only the test's own references keep the corner
    // nodes alive.
    assert_eq!(Rc::strong_count(&node_i), 1);
    assert_eq!(Rc::strong_count(&node_j), 1);
    assert_eq!(Rc::strong_count(&node_k), 1);
}

#[test]
fn destroy_without_links_is_noop() {
    let node = test_node();
    node.borrow_mut().destroy();
    assert!(node.borrow().children.iter().all(|slot| slot.is_none()));
}

#[test]
fn collect_nodes_deduplicates_reciprocal_cycles() {
    let node = test_node();
    let center = split_node(&node.borrow());
    let nodes = collect_nodes(&center);

    assert_eq!(nodes.len(), 4);
    assert_eq!(
        nodes
            .iter()
            .map(Rc::as_ptr)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );
}

#[test]
fn split_preserves_origin_for_descendants() {
    let origin = point(0.0, 1000.0);
    let node = Node::new(
        "root",
        [
            point(0.0, 100.0 * 3.0_f32.sqrt()),
            point(150.0, -50.0 * 3.0_f32.sqrt()),
            point(-150.0, -50.0 * 3.0_f32.sqrt()),
        ],
        origin,
    );
    let center = split_node(&node.borrow());
    let grandchild = split_node(&center.borrow().children[0].as_ref().unwrap().borrow());

    for descendant in collect_nodes(&grandchild) {
        let descendant = descendant.borrow();
        assert!(approx_eq(
            descendant.direction_to_origin,
            origin - descendant.center
        ));
    }
}

/// Two nodes sharing an edge: `root` (the test fixture) and a neighbor
/// mirrored across the base edge BC, linked `root.1 <-> neighbor.1`.
fn linked_pair() -> (NodeRef, NodeRef) {
    let root = test_node();
    let [a, b, c] = root.borrow().points;
    // Mirror of A across the horizontal base edge BC.
    let mirrored = Vec3::new(a.x, 2.0 * b.y - a.y, a.z);
    let neighbor = Node::new("neighbor", [mirrored, c, b], point(0.0, 1000.0));
    super::topology::link(&root, 1, &neighbor, 1);
    (root, neighbor)
}

/// Finds a leaf by name in `nodes`.
fn by_name(nodes: &[NodeRef], name: &str) -> NodeRef {
    Rc::clone(
        nodes
            .iter()
            .find(|node| node.borrow().name == name)
            .expect("node missing"),
    )
}

#[test]
fn split_nodes_splits_single_node_and_destroys_old() {
    let node = test_node();
    let leaves = split_nodes(&node);

    assert_eq!(leaves.len(), 4);
    for (leaf, suffix) in leaves.iter().zip(["root.I", "root.J", "root.K", "root.C"]) {
        let leaf = leaf.borrow();
        assert_eq!(leaf.name, suffix);
        assert_eq!(leaf.level, 1);
    }
    // The old node is destroyed: all its links are gone.
    assert!(node.borrow().children.iter().all(|slot| slot.is_none()));
}

#[test]
fn split_nodes_welds_corners_across_old_links() {
    let (root, neighbor) = linked_pair();
    let leaves = split_nodes(&root);

    assert_eq!(leaves.len(), 8);
    // The half-edges of the former shared edge are welded corner-to-corner.
    let root_j = by_name(&leaves, "root.J");
    let root_k = by_name(&leaves, "root.K");
    let neighbor_j = by_name(&leaves, "neighbor.J");
    let neighbor_k = by_name(&leaves, "neighbor.K");
    assert!(Rc::ptr_eq(
        root_j.borrow().children[1].as_ref().unwrap(),
        &neighbor_k
    ));
    assert!(Rc::ptr_eq(
        root_k.borrow().children[1].as_ref().unwrap(),
        &neighbor_j
    ));
    // Reciprocal back-ports are recorded on both welds.
    assert_eq!(root_j.borrow().back_ports[1], Some(1));
    assert_eq!(neighbor_j.borrow().back_ports[1], Some(1));
    // Ports that were open stay open.
    let root_i = by_name(&leaves, "root.I");
    assert!(root_i.borrow().children[0].is_none());
    assert!(root_i.borrow().children[2].is_none());
    // The old nodes are destroyed.
    assert!(root.borrow().children.iter().all(|slot| slot.is_none()));
    assert!(neighbor.borrow().children.iter().all(|slot| slot.is_none()));

    destroy_mesh(&leaves[0]);
}

#[test]
fn unsplit_nodes_rebuilds_parents_and_their_links() {
    let (root, neighbor) = linked_pair();
    let root_points = root.borrow().points;
    let neighbor_points = neighbor.borrow().points;
    let leaves = split_nodes(&root);

    let parents = unsplit_nodes(&leaves[0]);
    assert_eq!(parents.len(), 2);
    let new_root = by_name(&parents, "root");
    let new_neighbor = by_name(&parents, "neighbor");
    for (actual, expected) in new_root.borrow().points.iter().zip(root_points) {
        assert!(approx_eq(*actual, expected));
    }
    for (actual, expected) in new_neighbor.borrow().points.iter().zip(neighbor_points) {
        assert!(approx_eq(*actual, expected));
    }
    assert_eq!(new_root.borrow().level, 0);
    // The parent link across the shared edge is restored; the ports that
    // were open before the split are open again.
    assert!(Rc::ptr_eq(
        new_root.borrow().children[1].as_ref().unwrap(),
        &new_neighbor
    ));
    assert_eq!(new_root.borrow().back_ports[1], Some(1));
    assert!(new_root.borrow().children[0].is_none());
    assert!(new_root.borrow().children[2].is_none());
    // The merged children are destroyed.
    assert!(
        leaves
            .iter()
            .all(|leaf| leaf.borrow().children.iter().all(|slot| slot.is_none()))
    );
}

#[test]
fn unsplit_nodes_keeps_unsplittable_mesh() {
    let root = test_node();
    let kept = unsplit_nodes(&root);
    assert_eq!(kept.len(), 1);
    assert!(Rc::ptr_eq(&kept[0], &root));
}

#[test]
fn unsplit_nodes_relinks_kept_neighbors_to_the_parent() {
    // Mixed-level mesh: `root` is split, `neighbor` is not. The neighbor
    // links into the split group through one half-edge of the old shared
    // edge (corner J's port 1, like a `split_nodes` weld near B).
    let root = test_node();
    let [a, b, c] = root.borrow().points;
    let mirrored = Vec3::new(a.x, 2.0 * b.y - a.y, a.z);
    let neighbor = Node::new("neighbor", [mirrored, c, b], point(0.0, 1000.0));

    let center = split_node(&root.borrow());
    let root_j = Rc::clone(center.borrow().children[0].as_ref().unwrap());
    super::topology::link(&root_j, 1, &neighbor, 1);

    let parents = unsplit_nodes(&root_j);
    assert_eq!(parents.len(), 2);
    let new_root = by_name(&parents, "root");
    // The kept neighbor's port is re-targeted to the surviving parent, on
    // the same ports the merged corner used.
    assert!(Rc::ptr_eq(
        new_root.borrow().children[1].as_ref().unwrap(),
        &neighbor
    ));
    assert_eq!(new_root.borrow().back_ports[1], Some(1));
    assert!(Rc::ptr_eq(
        neighbor.borrow().children[1].as_ref().unwrap(),
        &new_root
    ));
    assert_eq!(neighbor.borrow().back_ports[1], Some(1));
    // The other parent ports stay open, and the mesh is fully reachable.
    assert!(new_root.borrow().children[0].is_none());
    assert!(new_root.borrow().children[2].is_none());
    assert_eq!(collect_nodes(&new_root).len(), 2);
    // The merged children are destroyed.
    assert!(center.borrow().children.iter().all(|slot| slot.is_none()));
    assert!(root_j.borrow().children.iter().all(|slot| slot.is_none()));

    destroy_mesh(&new_root);
}

#[test]
fn unsplit_nodes_keeps_the_whole_group_on_name_collisions() {
    let node = test_node();
    let leaves = split_nodes(&node);
    // A second node named "root.I": a name collision, not a split group.
    let duplicate = Node::new(
        "root.I",
        [point(0.0, 2.0), point(1.5, -1.0), point(-1.5, -1.0)],
        point(0.0, 1000.0),
    );
    let root_i = by_name(&leaves, "root.I");
    super::topology::link(&root_i, 0, &duplicate, 1);

    let parents = unsplit_nodes(&root_i);

    // Nothing merges: the four group members and the duplicate are kept.
    assert_eq!(parents.len(), 5);
    assert!(parents.iter().all(|node| node.borrow().name != "root"));
    assert_eq!(
        parents
            .iter()
            .filter(|node| node.borrow().name == "root.I")
            .count(),
        2
    );
    // The split group is untouched: levels and links are unchanged.
    let root_c = by_name(&parents, "root.C");
    assert_eq!(root_c.borrow().level, 1);
    assert!(root_c.borrow().children.iter().all(|slot| slot.is_some()));
    assert_eq!(collect_nodes(&root_i).len(), 5);

    destroy_mesh(&root_i);
}

#[test]
fn split_unsplit_round_trip_over_two_generations() {
    let root = test_node();
    let first = split_nodes(&root);
    let second = split_nodes(&first[0]);
    assert_eq!(second.len(), 16);

    let merged = unsplit_nodes(&second[0]);
    assert_eq!(merged.len(), 4);
    assert!(merged.iter().all(|node| node.borrow().level == 1));
    let merged = unsplit_nodes(&merged[0]);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].borrow().name, "root");
    assert_eq!(merged[0].borrow().level, 0);
}

#[test]
fn unsplit_nodes_merges_an_icosphere_generation() {
    let mesh = build_icosphere("planet", 300.0, 1, Vec3::ZERO);
    assert_eq!(mesh.faces.len(), 80);

    let parents = unsplit_nodes(&mesh.faces[0]);
    assert_eq!(parents.len(), 20);
    for parent in &parents {
        let parent = parent.borrow();
        assert_eq!(parent.level, 0);
        // The merged mesh is closed again: every port is linked.
        assert!(parent.children.iter().all(|slot| slot.is_some()));
    }

    destroy_mesh(&parents[0]);
}
