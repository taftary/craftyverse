//! Ground-flattening vertex morph: the CPU reference implementation of the
//! displacement the textured vertex shader (`shaders::TEX_VERT`) evaluates
//! per vertex (feature 5, Decision 3 of `plan/RELATED.md`).
//!
//! Test-only: the whole module is gated behind the `test-internals` feature
//! (the runtime evaluates the GLSL mirror in `shaders`, never this
//! function), keeping the default build free of dead code. The GLSL mirrors
//! [`morph_vertex`] formula-for-formula in the anchor-relative frame; the
//! authoritative math consumed by gameplay and picking (same formula in the
//! world frame) lives in `crate::runtime::flatten`.

use glam::Vec3;

/// The morphed vertex position, mirroring `TEX_VERT` formula-for-formula:
/// shift into the anchor-relative frame (`pos - anchor`), project onto the
/// tangent plane at the anchor (the plane through the anchor with normal
/// `anchor_up`, the outward radial at the anchor), and blend by `flatten`
/// (0 = spherical, 1 = flat). The result is anchor-relative; the shader
/// multiplies it by the anchor-relative view-projection matrix.
///
/// At `flatten == 0` the result is exactly `pos - anchor`; at
/// `flatten == 1` it lies in the tangent plane. Skirt vertices morph with
/// the same formula, so chunks and their crack-masking skirts stay
/// consistent at every blend value.
pub fn morph_vertex(pos: Vec3, anchor: Vec3, anchor_up: Vec3, flatten: f32) -> Vec3 {
    let local = pos - anchor;
    let flattened = local - anchor_up * local.dot(anchor_up);
    local.lerp(flattened, flatten)
}
