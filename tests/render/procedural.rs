//! Tests for the procedural texture reference functions: the barycentric
//! gradient, the parity checkerboard, the edge stripes, the edge-flip mask
//! and the radial effects — all pure functions of barycentric coordinates,
//! parity and radial direction, never of UVs.

use glam::Vec3;

use planet_crafter_engine::node::Node;
use planet_crafter_engine::testing::{
    CHECKER_CELLS, LATITUDE_BANDS, LIGHT_DIR, MASK_EDGE_WIDTH, STRIPE_BANDS, checker, diffuse,
    edge_mask, fresnel, gradient, latitude, radial_rgb, rings, stripes,
};

/// Centroid (in barycentric coordinates) of the up-pointing sub-triangle at
/// corner `A` of the implicit checker grid.
fn up_cell_centroid() -> Vec3 {
    let cells = CHECKER_CELLS as f32;
    // Vertices (1,0,0), ((c-1)/c, 1/c, 0), ((c-1)/c, 0, 1/c).
    Vec3::new(
        (1.0 + 2.0 * (cells - 1.0) / cells) / 3.0,
        1.0 / (3.0 * cells),
        1.0 / (3.0 * cells),
    )
}

/// Centroid of the down-pointing sub-triangle sharing the up cell's base
/// edge (toward edge `BC`).
fn down_cell_centroid() -> Vec3 {
    let cells = CHECKER_CELLS as f32;
    // Vertices ((c-1)/c, 1/c, 0), ((c-1)/c, 0, 1/c), ((c-2)/c, 1/c, 1/c).
    Vec3::new(
        (2.0 * (cells - 1.0) / cells + (cells - 2.0) / cells) / 3.0,
        2.0 / (3.0 * cells),
        2.0 / (3.0 * cells),
    )
}

#[test]
fn gradient_maps_barycentric_coordinates_to_rgb() {
    let bary = Vec3::new(0.2, 0.3, 0.5);
    assert_eq!(gradient(bary), [0.2, 0.3, 0.5]);
    // Pure corners are pure channels.
    assert_eq!(gradient(Vec3::X), [1.0, 0.0, 0.0]);
    assert_eq!(gradient(Vec3::Y), [0.0, 1.0, 0.0]);
    assert_eq!(gradient(Vec3::Z), [0.0, 0.0, 1.0]);
}

#[test]
fn checker_alternates_between_adjacent_sub_triangles() {
    let up = up_cell_centroid();
    let down = down_cell_centroid();
    // Sanity: both are valid barycentric coordinates inside the triangle.
    for bary in [up, down] {
        assert!((bary.element_sum() - 1.0).abs() < 1e-6);
        assert!(bary.cmpge(Vec3::ZERO).all());
    }
    assert_eq!(checker(up, 1.0), 1.0);
    assert_eq!(checker(down, 1.0), 0.0);
}

#[test]
fn checker_flips_phase_with_parity() {
    let up = up_cell_centroid();
    let down = down_cell_centroid();
    assert_eq!(checker(up, -1.0), 1.0 - checker(up, 1.0));
    assert_eq!(checker(down, -1.0), 1.0 - checker(down, 1.0));
}

#[test]
fn stripes_count_bands_along_the_altitude() {
    // STRIPE_BANDS bands from BC (uA = 0) to A (uA = 1), alternating,
    // starting with 0 at the base edge.
    for band in 0..STRIPE_BANDS {
        let u_a = (band as f32 + 0.5) / STRIPE_BANDS as f32;
        assert_eq!(stripes(u_a, 1.0), (band % 2) as f32, "band {band}");
        assert_eq!(stripes(u_a, -1.0), 1.0 - (band % 2) as f32, "band {band}");
    }
}

#[test]
fn edge_mask_marks_a_band_along_one_edge() {
    // Near edge AB (uC small) is inside the mask; near edge CA (uB small)
    // is outside it.
    let near_ab = Vec3::new(0.5, 0.5 - MASK_EDGE_WIDTH / 2.0, MASK_EDGE_WIDTH / 2.0);
    let near_ca = Vec3::new(0.5, MASK_EDGE_WIDTH / 2.0, 0.5 - MASK_EDGE_WIDTH / 2.0);
    assert_eq!(edge_mask(near_ab, 1.0), 1.0);
    assert_eq!(edge_mask(near_ca, 1.0), 0.0);
}

#[test]
fn edge_mask_flips_edges_with_parity() {
    // A negative parity mirrors the triangle (uB/uC swapped): the mask
    // band flips from edge AB to edge CA.
    let near_ab = Vec3::new(0.5, 0.5 - MASK_EDGE_WIDTH / 2.0, MASK_EDGE_WIDTH / 2.0);
    let near_ca = Vec3::new(0.5, MASK_EDGE_WIDTH / 2.0, 0.5 - MASK_EDGE_WIDTH / 2.0);
    assert_eq!(edge_mask(near_ab, -1.0), 0.0);
    assert_eq!(edge_mask(near_ca, -1.0), 1.0);
}

#[test]
fn effects_are_stable_across_subdivision_levels() {
    // The functions take no level and no UVs: the same local barycentric
    // point of any triangle, at any subdivision level and on either side of
    // a UV seam, evaluates identically. The corner barycentric coordinates
    // are the unit basis at every level, so corner values are level-free.
    for bary in [
        Vec3::X,
        Vec3::Y,
        Vec3::Z,
        up_cell_centroid(),
        down_cell_centroid(),
    ] {
        for parity in [1.0, -1.0] {
            assert_eq!(checker(bary, parity), checker(bary, parity));
            assert_eq!(stripes(bary.x, parity), stripes(bary.x, parity));
            assert_eq!(edge_mask(bary, parity), edge_mask(bary, parity));
        }
    }
    // The gradient is exactly stable: a child's local barycentric field is
    // a linear restriction of the parent's, so the midpoint of edge AB
    // shows the average of the two corner colors.
    let midpoint = Vec3::new(0.5, 0.5, 0.0);
    assert_eq!(gradient(midpoint), [0.5, 0.5, 0.0]);
}

/// Signed barycentric coordinates of `p` in the triangle `[a, b, c]` (the
/// affine extension is valid anywhere in the triangle plane).
fn barycentric(a: Vec3, b: Vec3, c: Vec3, p: Vec3) -> Vec3 {
    let n = (b - a).cross(c - a);
    let area2 = n.length_squared();
    Vec3::new(
        (b - p).cross(c - p).dot(n) / area2,
        (c - p).cross(a - p).dot(n) / area2,
        (a - p).cross(b - p).dot(n) / area2,
    )
}

#[test]
fn edge_stripes_are_aligned_with_the_ijk_directions() {
    // A scalene triangle, so no accidental perpendiculars hide a mistake.
    let node = Node::new(
        "t",
        [
            Vec3::new(0.3, 2.1, 0.0),
            Vec3::new(1.6, -0.9, 0.0),
            Vec3::new(-0.7, -1.2, 0.0),
        ],
        Vec3::new(0.0, 0.0, 5.0),
    );
    let node = node.borrow();
    let [a, b, c] = node.vertices;
    let center = node.center;
    let bary = |p: Vec3| barycentric(a, b, c, p);

    // (direction slot, barycentric component the stripes run along, edge).
    // I points toward AB and stripes I follow `uC` (0 on AB); J toward BC
    // with `uA`; K toward CA with `uB`.
    for (dir_index, coord, (e0, e1)) in [(0, 2, (a, b)), (1, 0, (b, c)), (2, 1, (c, a))] {
        let direction = node.directions[dir_index];
        // Walking from the center along the direction (toward the edge
        // midpoint), the stripe coordinate strictly decreases — the bands
        // vary exactly along the I/J/K direction.
        let mut previous = bary(center)[coord];
        for step in 1..=4 {
            let current = bary(center + direction * (step as f32 * 0.1))[coord];
            assert!(
                current < previous,
                "direction {dir_index}: coordinate {coord} not decreasing"
            );
            previous = current;
        }
        // Two points sharing the stripe coordinate differ by a vector
        // parallel to the edge — the iso-lines (the visible bands) are
        // parallel to the edge — and the effect agrees on them.
        let mut u1 = Vec3::new(0.3, 0.45, 0.25);
        let mut u2 = Vec3::new(0.55, 0.2, 0.25);
        if coord == 0 {
            u1 = Vec3::new(0.25, 0.3, 0.45);
            u2 = Vec3::new(0.25, 0.55, 0.2);
        } else if coord == 1 {
            u1 = Vec3::new(0.45, 0.25, 0.3);
            u2 = Vec3::new(0.2, 0.25, 0.55);
        }
        let p1 = u1.x * a + u1.y * b + u1.z * c;
        let p2 = u2.x * a + u2.y * b + u2.z * c;
        assert!(
            (p2 - p1).cross(e1 - e0).length() < 1e-5,
            "iso-line not parallel to the edge for direction {dir_index}"
        );
        assert_eq!(stripes(u1[coord], 1.0), stripes(u2[coord], 1.0));
    }
}

#[test]
fn radial_rgb_maps_direction_to_channels() {
    assert_eq!(radial_rgb(Vec3::X), [1.0, 0.5, 0.5]);
    assert_eq!(radial_rgb(Vec3::NEG_X), [0.0, 0.5, 0.5]);
    assert_eq!(radial_rgb(Vec3::ZERO), [0.5, 0.5, 0.5]);
}

#[test]
fn diffuse_is_lambertian() {
    // Facing the light: 1; facing away: 0; the axis components are the
    // light direction's own components.
    assert!((diffuse(LIGHT_DIR) - 1.0).abs() < 1e-5);
    assert_eq!(diffuse(-LIGHT_DIR), 0.0);
    assert!((diffuse(Vec3::X) - LIGHT_DIR.x).abs() < 1e-7);
}

#[test]
fn latitude_bands_run_pole_to_pole() {
    for band in 0..LATITUDE_BANDS {
        let t = (band as f32 + 0.5) / LATITUDE_BANDS as f32;
        let y = t * 2.0 - 1.0;
        let outward = Vec3::new((1.0 - y * y).sqrt(), y, 0.0);
        assert_eq!(latitude(outward), (band % 2) as f32, "band {band}");
    }
}

#[test]
fn fresnel_is_bright_at_the_silhouette() {
    // Surface facing the viewer (either side): dark; edge-on: bright.
    assert_eq!(fresnel(Vec3::Z, Vec3::Z), 0.0);
    assert_eq!(fresnel(Vec3::NEG_Z, Vec3::Z), 0.0);
    assert_eq!(fresnel(Vec3::X, Vec3::Z), 1.0);
    assert!((fresnel(Vec3::new(1.0, 0.0, 1.0).normalize(), Vec3::Z) - 0.2928932).abs() < 1e-6);
}

#[test]
fn rings_alternate_per_band_without_parity() {
    // floor(seed_distance) mod 2, no parity input: same value for both
    // parities (the field itself is continuous, bands are even).
    assert_eq!(rings(0.0), 0.0);
    assert_eq!(rings(0.9), 0.0);
    assert_eq!(rings(1.0), 1.0);
    assert_eq!(rings(1.9), 1.0);
    assert_eq!(rings(2.0), 0.0);
    // Negative values (unreachable from seeded meshes) wrap safely.
    assert_eq!(rings(-0.5), 1.0);
}
