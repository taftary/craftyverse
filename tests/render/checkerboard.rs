use planet_crafter_engine::testing::{
    CHECKER_HEIGHT, CHECKER_WIDTH, CHECKS_U, CHECKS_V, checkerboard_mips, mip_level_count,
};

/// Expected value of one texel channel, computed from the analytic parity
/// function (the contract of `checkerboard_mips`).
fn expected_value(x: u32, y: u32, level_w: u32, level_h: u32, checks_u: u32, checks_v: u32) -> u8 {
    let u = (x as f64 + 0.5) / f64::from(level_w);
    let v = (y as f64 + 0.5) / f64::from(level_h);
    let check = (u * f64::from(checks_u)).floor() as u64 + (v * f64::from(checks_v)).floor() as u64;
    if check % 2 == 0 { 0x00 } else { 0xFF }
}

#[test]
fn mip_level_count_is_floor_log2_max_plus_one() {
    assert_eq!(mip_level_count(1, 1), 1);
    assert_eq!(mip_level_count(2, 1), 2);
    assert_eq!(mip_level_count(3, 2), 2);
    assert_eq!(mip_level_count(4, 4), 3);
    assert_eq!(mip_level_count(968, 2048), 12);
    assert_eq!(mip_level_count(CHECKER_WIDTH, CHECKER_HEIGHT), 12);
}

#[test]
fn level_dimensions_halve_with_floor_at_one() {
    let levels = checkerboard_mips(8, 3, 2, 2);
    assert_eq!(levels.len(), 4);
    // Dims per level: (8, 3), (4, 1), (2, 1), (1, 1) — height floors at 1.
    let texel_counts: Vec<usize> = levels.iter().map(|level| level.len() / 4).collect();
    assert_eq!(texel_counts, vec![24, 4, 2, 1]);
}

#[test]
fn buffers_are_tightly_packed_rgba8() {
    let levels = checkerboard_mips(16, 8, 4, 2);
    assert_eq!(levels[0].len(), 16 * 8 * 4);
    for (i, level) in levels.iter().enumerate() {
        let level_w = (16_u32 >> i).max(1);
        let level_h = (8_u32 >> i).max(1);
        assert_eq!(level.len(), (level_w * level_h * 4) as usize);
    }
}

#[test]
fn texel_parities_match_the_analytic_function() {
    let (width, height, checks_u, checks_v) = (16, 8, 4, 2);
    let level0 = &checkerboard_mips(width, height, checks_u, checks_v)[0];
    let texel = |x: u32, y: u32| level0[((y * width + x) * 4) as usize];
    // Texel (0, 0): floor(0.5/16 * 4) + floor(0.5/8 * 2) = 0 + 0 — even, so
    // black.
    assert_eq!(texel(0, 0), 0x00);
    // Texel (4, 0): floor(4.5/16 * 4) = 1 — odd, so white.
    assert_eq!(texel(4, 0), 0xFF);
    for y in 0..height {
        for x in 0..width {
            assert_eq!(
                texel(x, y),
                expected_value(x, y, width, height, checks_u, checks_v),
                "texel ({x}, {y})",
            );
        }
    }
}

#[test]
fn alpha_is_always_opaque() {
    for level in checkerboard_mips(32, 16, 4, 2) {
        assert!(level.chunks_exact(4).all(|pixel| pixel[3] == 0xFF));
    }
    // Black and white texels are pure (R = G = B).
    for level in checkerboard_mips(32, 16, 4, 2) {
        assert!(
            level
                .chunks_exact(4)
                .all(|pixel| pixel[0] == pixel[1] && pixel[1] == pixel[2])
        );
    }
}

#[test]
fn deep_mip_levels_are_analytic_not_downsampled() {
    let levels = checkerboard_mips(64, 64, 4, 4);
    // The deepest level is 1x1: floor(0.5 * 4) + floor(0.5 * 4) = 4 — even,
    // so pure black.
    let deep = levels.last().unwrap();
    assert_eq!(deep.len(), 4);
    assert_eq!(deep[0], 0x00);
    // A naive box-average of level 0 would mix black and white texels into
    // gray; the analytic level differs from it.
    let level0 = &levels[0];
    let average = (level0
        .chunks_exact(4)
        .map(|pixel| pixel[0] as u64)
        .sum::<u64>()
        / (64 * 64)) as u8;
    assert!(average != 0x00 && average != 0xFF);
    assert_ne!(deep[0], average);
}

#[test]
fn default_constants_generate_a_valid_full_chain() {
    let levels = checkerboard_mips(CHECKER_WIDTH, CHECKER_HEIGHT, CHECKS_U, CHECKS_V);
    assert_eq!(
        levels.len() as u32,
        mip_level_count(CHECKER_WIDTH, CHECKER_HEIGHT)
    );
    assert_eq!(
        levels[0].len(),
        (CHECKER_WIDTH * CHECKER_HEIGHT * 4) as usize
    );
    assert_eq!(levels.last().unwrap().len(), 4);
}
