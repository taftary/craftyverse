use glam::Vec2;
use planet_crafter_engine::testing::{
    ATLAS_HEIGHT, ATLAS_WIDTH, GLYPH_PADDING, ShelfPacker, uv_rect,
};

#[test]
fn shelf_packer_wraps_to_a_new_shelf_when_full() {
    let mut packer = ShelfPacker::new();
    // A shelf-filling rectangle starts at the padding offset.
    let wide = ATLAS_WIDTH - 2 * GLYPH_PADDING;
    assert_eq!(
        packer.reserve(wide, 10),
        Some((GLYPH_PADDING, GLYPH_PADDING))
    );
    // The next rectangle wraps one shelf down: past the first shelf's
    // height plus padding.
    assert_eq!(
        packer.reserve(10, 10),
        Some((GLYPH_PADDING, GLYPH_PADDING + 10 + GLYPH_PADDING))
    );
    // A same-width rectangle continues on the same shelf.
    assert_eq!(
        packer.reserve(10, 10),
        Some((
            GLYPH_PADDING + 10 + GLYPH_PADDING,
            GLYPH_PADDING + 10 + GLYPH_PADDING
        ))
    );
}

#[test]
fn shelf_packer_reports_atlas_overflow() {
    // A rectangle taller than the atlas never fits.
    let mut packer = ShelfPacker::new();
    assert_eq!(packer.reserve(10, ATLAS_HEIGHT), None);

    // Filling the shelves eventually exhausts the atlas.
    let mut packer = ShelfPacker::new();
    let wide = ATLAS_WIDTH - 2 * GLYPH_PADDING;
    assert!(
        packer
            .reserve(wide, ATLAS_HEIGHT - 2 * GLYPH_PADDING)
            .is_some()
    );
    assert_eq!(packer.reserve(10, 10), None);
}

#[test]
fn uv_rect_insets_by_half_a_texel() {
    let (uv_min, uv_max) = uv_rect(10, 20, 30, 40);
    assert_eq!(
        uv_min,
        Vec2::new(10.5 / ATLAS_WIDTH as f32, 20.5 / ATLAS_HEIGHT as f32)
    );
    assert_eq!(
        uv_max,
        Vec2::new(39.5 / ATLAS_WIDTH as f32, 59.5 / ATLAS_HEIGHT as f32)
    );
}
