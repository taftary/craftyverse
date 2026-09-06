use glam::Vec3;

use planet_crafter_engine::node::{Node, collect_nodes, split_node};
use planet_crafter_tests::fixtures::{point, test_node};

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
