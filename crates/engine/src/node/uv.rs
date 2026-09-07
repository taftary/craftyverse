//! UV coordinates for nodes: the canonical default triangle assigned by
//! [`Node::new`](super::Node::new) and the icosahedral net layout that seeds
//! the base faces of [`build_icosphere`](super::build_icosphere).
//!
//! # The icosahedral net
//!
//! The 20 base faces of the icosahedron are unwrapped into the classic flat
//! net (after Paul Bourke's icosahedral maps,
//! <https://paulbourke.net/panorama/icosahedral/>): a horizontal zigzag strip
//! of the 10 equatorial faces, the 5 faces around icosahedron vertex 0 fanned
//! across the top, and the 5 faces around vertex 9 fanned across the bottom —
//! a 5.5 x 3 triangle grid (aspect ratio ~2.117), normalized into `[0, 1]^2`
//! with a small margin.
//!
//! UVs are stored per node corner and duplicated across neighbors, exactly
//! like [`Node::vertices`](super::Node::vertices). The net is continuous
//! across every edge it preserves (strip adjacencies and fan attachments):
//! two faces sharing such an edge hold identical UVs for the shared vertices.
//! Every cut edge of the net is a **seam**: the two sides hold different UVs
//! for the same 3D vertex. Subdivision interpolates UVs linearly (flat
//! midpoints, never sphere-projected), so the property holds at every level.
//!
//! The layout is constructed by mirroring: an anchor face is placed, and each
//! other face is the reflection of an already-placed 3D neighbor across the
//! shared edge, with endpoints matched by icosahedron vertex index. This
//! makes the construction independent of the base-face winding (5 of the 20
//! base faces are deliberately wound inward; see the `icosphere` module).

use glam::Vec2;

use super::icosphere::ICOSAHEDRON_FACES;

/// Margin left around the net inside `[0, 1]^2`, as a fraction of the UV
/// space. Keeps bilinear filtering footprints inside the sampled region.
const NET_MARGIN: f32 = 0.01;

/// Default UV triangle of a node created outside an icosphere: a canonical
/// equilateral triangle (base 0.9, centered in `[0, 1]^2`), `A` at the apex,
/// so a lone triangle shows an undistorted texture.
pub const DEFAULT_UV: [Vec2; 3] = [
    Vec2::new(0.5, 0.889_711_44),
    Vec2::new(0.05, 0.110_288_56),
    Vec2::new(0.95, 0.110_288_56),
];

/// Strip order of the 10 equatorial faces: consecutive faces share an edge,
/// even positions point up (bottom-fan attachments) and odd positions point
/// down (top-fan attachments). Derived from the base-face adjacency of
/// `ICOSAHEDRON_FACES`.
const STRIP: [usize; 10] = [6, 5, 8, 9, 14, 16, 18, 17, 11, 7];

/// Top fan (the faces around icosahedron vertex 0), in strip order: entry
/// `i` attaches to `STRIP[2 * i + 1]`.
const TOP: [usize; 5] = [0, 2, 3, 4, 1];

/// Bottom fan (the faces around icosahedron vertex 9), in strip order: entry
/// `i` attaches to `STRIP[2 * i]`.
const BOTTOM: [usize; 5] = [10, 13, 15, 19, 12];

/// Computes the icosahedral net layout: `result[face_index]` holds the
/// `[uA, uB, uC]` corners of the matching `ICOSAHEDRON_FACES` entry, in
/// `[0, 1]^2`.
///
/// Pure and deterministic; computed on each icosphere build (20 triangles —
/// negligible). `pub` inside the private `uv` module; it only escapes under
/// the `test-internals` feature for the spec tests.
pub fn icosphere_net_uv() -> [[Vec2; 3]; 20] {
    // Equilateral triangle height for base = 1.
    let h = 0.5 * 3.0_f32.sqrt();
    let mut net = [[Vec2::ZERO; 3]; 20];
    let mut placed = [false; 20];

    // Anchor: the first strip face, up-pointing — A at the apex, B at the
    // base-left, C at the base-right.
    net[STRIP[0]] = [Vec2::new(0.5, h), Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0)];
    placed[STRIP[0]] = true;

    // Unfold the strip: each face is the mirror image of its predecessor
    // across the shared edge.
    for pair in STRIP.windows(2) {
        let [prev, face] = [pair[0], pair[1]];
        net[face] = mirror_across_shared_edge(net[prev], prev, face);
        placed[face] = true;
    }

    // Unfold the fans across the attachment edges of their strip faces.
    for (index, &fan) in TOP.iter().enumerate() {
        let anchor = STRIP[2 * index + 1];
        net[fan] = mirror_across_shared_edge(net[anchor], anchor, fan);
        placed[fan] = true;
    }
    for (index, &fan) in BOTTOM.iter().enumerate() {
        let anchor = STRIP[2 * index];
        net[fan] = mirror_across_shared_edge(net[anchor], anchor, fan);
        placed[fan] = true;
    }
    debug_assert!(placed.into_iter().all(|flag| flag));

    normalize(net)
}

/// Places `face` as the reflection of the already-placed `anchor` face across
/// their shared 3D edge. The shared edge endpoints are matched by icosahedron
/// vertex index, so the correspondence is exact for any winding.
fn mirror_across_shared_edge(anchor: [Vec2; 3], anchor_face: usize, face: usize) -> [Vec2; 3] {
    let anchor_corners = ICOSAHEDRON_FACES[anchor_face];
    let corners = ICOSAHEDRON_FACES[face];

    let mut uv = [Vec2::ZERO; 3];
    let mut edge = [Vec2::ZERO; 2];
    let mut edge_len = 0;
    let mut anchor_apex = None;
    let mut face_apex = None;
    for (corner, &vertex) in anchor_corners.iter().enumerate() {
        match corners.iter().position(|&v| v == vertex) {
            Some(shared) => {
                uv[shared] = anchor[corner];
                edge[edge_len] = anchor[corner];
                edge_len += 1;
            }
            None => anchor_apex = Some(anchor[corner]),
        }
    }
    for (corner, &vertex) in corners.iter().enumerate() {
        if !anchor_corners.contains(&vertex) {
            face_apex = Some(corner);
        }
    }
    debug_assert_eq!(edge_len, 2, "net faces must share exactly one edge");

    uv[face_apex.expect("net faces must share an edge")] = reflect(
        anchor_apex.expect("net faces must share an edge"),
        edge[0],
        edge[1],
    );
    uv
}

/// Reflects `point` across the line through `line_a` and `line_b`.
fn reflect(point: Vec2, line_a: Vec2, line_b: Vec2) -> Vec2 {
    let direction = (line_b - line_a).normalize();
    let projection = line_a + direction * (point - line_a).dot(direction);
    2.0 * projection - point
}

/// Normalizes the net into `[0, 1]^2`: uniform scale preserving the aspect
/// ratio, centered, with a `NET_MARGIN` border.
fn normalize(mut net: [[Vec2; 3]; 20]) -> [[Vec2; 3]; 20] {
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for face in &net {
        for &point in face {
            min = min.min(point);
            max = max.max(point);
        }
    }
    let extent = max - min;
    let scale = (1.0 - 2.0 * NET_MARGIN) / extent.max_element();
    let offset =
        Vec2::splat(NET_MARGIN) + (Vec2::splat(1.0 - 2.0 * NET_MARGIN) - extent * scale) * 0.5;
    for face in &mut net {
        for point in face {
            *point = (*point - min) * scale + offset;
        }
    }
    net
}
