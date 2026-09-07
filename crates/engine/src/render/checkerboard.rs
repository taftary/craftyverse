//! Analytic checkerboard debug texture for the textured view modes.
//!
//! The texture is generated on the CPU, one image per mip level, and uploaded
//! to the GPU once at renderer startup. Every texel of every level is
//! evaluated directly from the global checkerboard parity function — never by
//! averaging or downsampling the previous level. This matters because the
//! icosahedral UV net the texture is mapped with has seams: texels near a cut
//! edge have neighbors across the seam whose texel values belong to a
//! different part of the net. Analytic per-level evaluation guarantees no
//! texel ever mixes information across a cut edge, so bilinear filtering and
//! mip sampling are seam-safe without gutter engineering. (Gutters — dilated
//! texel borders around UV islands — remain the strategy for future authored
//! art textures, whose texels cannot be re-evaluated analytically.)

/// Width of the checkerboard texture in texels (aspect-matched to the UV
/// net: the net is about 2.117 times wider than tall).
pub const CHECKER_WIDTH: u32 = 2048;
/// Height of the checkerboard texture in texels (see [`CHECKER_WIDTH`]).
pub const CHECKER_HEIGHT: u32 = 968;
/// Checkerboard repetitions along U: 4 checks per base-triangle edge, sized
/// to keep the checks near-square given the texture aspect.
pub const CHECKS_U: u32 = 22;
/// Checkerboard repetitions along V (see [`CHECKS_U`]).
pub const CHECKS_V: u32 = 10;

/// Number of mip levels of a `width` x `height` texture:
/// `floor(log2(max(width, height))) + 1` (a 1x1 texture has one level).
pub fn mip_level_count(width: u32, height: u32) -> u32 {
    width.max(height).max(1).ilog2() + 1
}

/// Generates the checkerboard as one tightly packed RGBA8 image per mip
/// level (`levels[i]` is `max(1, width >> i)` x `max(1, height >> i)` texels,
/// `w * h * 4` bytes, row-major, alpha always `0xFF`).
///
/// Every texel of every level is evaluated analytically from the global
/// parity function: with `u = (x + 0.5) / level_w` and
/// `v = (y + 0.5) / level_h` the texel is black when
/// `(floor(u * checks_u) + floor(v * checks_v)) % 2 == 0`, white otherwise.
/// See the module doc for why mip levels are not downsampled from level 0.
pub fn checkerboard_mips(width: u32, height: u32, checks_u: u32, checks_v: u32) -> Vec<Vec<u8>> {
    (0..mip_level_count(width, height))
        .map(|level| {
            let level_w = (width >> level).max(1);
            let level_h = (height >> level).max(1);
            let mut pixels = Vec::with_capacity((level_w * level_h * 4) as usize);
            for y in 0..level_h {
                for x in 0..level_w {
                    let u = (x as f64 + 0.5) / f64::from(level_w);
                    let v = (y as f64 + 0.5) / f64::from(level_h);
                    let check = (u * f64::from(checks_u)).floor() as u64
                        + (v * f64::from(checks_v)).floor() as u64;
                    let value = if check % 2 == 0 { 0x00 } else { 0xFF };
                    pixels.extend_from_slice(&[value, value, value, 0xFF]);
                }
            }
            pixels
        })
        .collect()
}
