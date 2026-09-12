//! Procedural per-triangle texture: the CPU reference implementation of the
//! effects the textured view mode's fragment shader evaluates per pixel.
//!
//! Test-only: the whole module is gated behind the `test-internals` feature
//! (the runtime evaluates the GLSL mirror in `shaders`, never these
//! functions), keeping the default build free of dead code.
//!
//! Every effect is a pure function of the triangle-local barycentric
//! coordinates `(uA, uB, uC)` — `uA` is the altitude coordinate (1 at `A`,
//! 0 on `BC`), `uB` and `uC` the coordinates of `B` and `C` — the
//! node's topology parity (`+1` / `-1`, see [`crate::node::Parity`]) and,
//! for the radial effects, the node's outward radial direction (the negated,
//! normalized `direction_to_origin`; the sphere surface normal).
//! Nothing here reads UVs, so the procedural output is independent of UV
//! seams by construction. The GLSL fragment shader in `shaders` mirrors
//! these functions formula-for-formula with the same constants; keep the
//! two in sync (the render tests assert the constants match).
//!
//! The texture is computed per triangle: each triangle evaluates the
//! effects in its own barycentric space, so a subdivided mesh re-tiles the
//! pattern per leaf rather than inheriting the parent's pixels. (The
//! gradient is the exception in appearance only: child barycentric fields
//! are linear restrictions of the parent's, so it looks identical at every
//! level.)

use glam::Vec3;

/// Sub-cells per triangle edge of the parity checkerboard. The triangle is
/// implicitly divided into `CHECKER_CELLS^2` sub-triangles.
pub const CHECKER_CELLS: u32 = 8;
/// Number of bands of a stripe effect between the band's edge (`u = 0`) and
/// its opposite corner (`u = 1`).
pub const STRIPE_BANDS: u32 = 8;
/// Width of the edge mask band, in barycentric units of the coordinate that
/// is `0` on the masked edge.
pub const MASK_EDGE_WIDTH: f32 = 0.15;
/// Direction of the diffuse light, normalized `(1, 1, 1)`.
pub const LIGHT_DIR: Vec3 = Vec3::new(0.577_350_3, 0.577_350_3, 0.577_350_3);
/// Number of latitude bands of the latitude effect from pole to pole.
pub const LATITUDE_BANDS: u32 = 12;

/// XOR phase flip shared by the alternating effects: a negative parity
/// inverts the black/white assignment.
fn apply_parity(value: f32, parity: f32) -> f32 {
    if parity < 0.0 { 1.0 - value } else { value }
}

/// Barycentric gradient: `uA` → red, `uB` → green, `uC` → blue. Parity does
/// not apply (there is no black/white phase to flip).
pub fn gradient(bary: Vec3) -> [f32; 3] {
    [bary.x, bary.y, bary.z]
}

/// Parity checkerboard, `0.0` or `1.0`. With `uA + uB + uC = 1`, the sum of
/// `floor(bary * cells)` is `cells - 1` on up-pointing sub-triangles and
/// `cells - 2` on down-pointing ones, so the parity of that sum alternates
/// between every pair of edge-adjacent sub-triangles; the node parity flips
/// the phase.
pub fn checker(bary: Vec3, parity: f32) -> f32 {
    let cells = CHECKER_CELLS as f32;
    let cell = (bary.x * cells).floor() + (bary.y * cells).floor() + (bary.z * cells).floor();
    let value = (cell as i64).rem_euclid(2) as f32;
    apply_parity(value, parity)
}

/// Edge stripes, `0.0` or `1.0`: `STRIPE_BANDS` bands along one barycentric
/// coordinate `u` — `uA` for stripes parallel to `BC` (direction `J`), `uB`
/// for `CA` (`K`), `uC` for `AB` (`I`); the node parity flips the phase.
pub fn stripes(u: f32, parity: f32) -> f32 {
    let value = ((u * STRIPE_BANDS as f32).floor() as i64).rem_euclid(2) as f32;
    apply_parity(value, parity)
}

/// Edge-flip mask, `0.0` or `1.0`: a band of width [`MASK_EDGE_WIDTH`] along
/// edge `AB` (where `uC = 0`). A negative parity mirrors the triangle first
/// (`uB`/`uC` swapped), so the band flips to edge `CA` — the same formula
/// produces edge-flipped masks on `Acb` triangles.
pub fn edge_mask(bary: Vec3, parity: f32) -> f32 {
    let bary = if parity < 0.0 {
        Vec3::new(bary.x, bary.z, bary.y)
    } else {
        bary
    };
    if bary.z <= MASK_EDGE_WIDTH { 1.0 } else { 0.0 }
}

/// Radial normal visualization: the outward radial direction mapped from
/// `[-1, 1]` to `[0, 1]` per channel (x → red, y → green, z → blue). Parity
/// does not apply.
pub fn radial_rgb(outward: Vec3) -> [f32; 3] {
    (outward * 0.5 + 0.5).to_array()
}

/// Diffuse lighting, `0.0..=1.0`: Lambert term of the outward radial against
/// [`LIGHT_DIR`], grayscale.
pub fn diffuse(outward: Vec3) -> f32 {
    outward.dot(LIGHT_DIR).max(0.0)
}

/// Latitude bands, `0.0` or `1.0`: `LATITUDE_BANDS` bands from the south
/// pole to the north pole of the `Y` axis, alternating.
pub fn latitude(outward: Vec3) -> f32 {
    let t = outward.dot(Vec3::Y) * 0.5 + 0.5;
    ((t * LATITUDE_BANDS as f32).floor() as i64).rem_euclid(2) as f32
}

/// Fresnel rim, `0.0..=1.0`: bright at silhouette edges, where the outward
/// radial is perpendicular to the view direction.
pub fn fresnel(outward: Vec3, view_dir: Vec3) -> f32 {
    1.0 - outward.dot(view_dir).abs()
}
