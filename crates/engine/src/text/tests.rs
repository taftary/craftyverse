use super::packing::{ShelfPacker, uv_rect};
use super::*;

const FONT_BYTES: &[u8] = include_bytes!("../../../../assets/fonts/JetBrainsMono-Regular.ttf");

fn test_atlas() -> TextAtlas {
    TextAtlas::new(FONT_BYTES).expect("load bundled font")
}

/// Lays out `text` into a fresh buffer (test shorthand for `layout`).
fn layout(
    atlas: &TextAtlas,
    text: &str,
    anchor: Vec2,
    size: f32,
    centered: bool,
) -> Vec<TextVertex> {
    let mut vertices = Vec::new();
    atlas.layout(text, anchor, size, [0.0; 3], centered, &mut vertices);
    vertices
}

#[test]
fn atlas_contains_rasterized_glyphs() {
    let atlas = test_atlas();
    assert!(atlas.pixels.iter().any(|&px| px > 0));
    let glyph = atlas.glyph('A');
    assert!(glyph.has_bitmap && glyph.advance > 0.0);
    // Every stored UV stays inside the atlas.
    assert!(glyph.uv_min.cmpge(Vec2::ZERO).all());
    assert!(glyph.uv_max.cmple(Vec2::ONE).all());
}

#[test]
fn invalid_font_is_a_typed_error() {
    let Err(err) = TextAtlas::new(b"not a font") else {
        panic!("invalid font bytes must be rejected");
    };
    assert!(matches!(err, TextAtlasError::InvalidFont(_)));
    assert!(err.to_string().starts_with("failed to load font: "));
    assert_eq!(
        TextAtlasError::AtlasOverflow.to_string(),
        "text atlas overflow"
    );
    assert_eq!(
        TextAtlasError::NoHorizontalLineMetrics.to_string(),
        "font has no horizontal line metrics"
    );
}

#[test]
fn layout_emits_two_triangles_per_glyph() {
    let atlas = test_atlas();
    let vertices = layout(&atlas, "AB", Vec2::new(10.0, 20.0), 12.0, false);

    assert_eq!(vertices.len(), 2 * 6);
    for vertex in &vertices {
        assert!(vertex.uv.cmpge(Vec2::ZERO).all());
        assert!(vertex.uv.cmple(Vec2::ONE).all());
        assert!(vertex.pos.x >= 10.0);
        assert!(vertex.pos.y >= 20.0);
    }
}

#[test]
fn layout_appends_to_the_out_buffer() {
    let atlas = test_atlas();
    let mut vertices = Vec::new();
    atlas.layout("A", Vec2::ZERO, 12.0, [0.0; 3], false, &mut vertices);
    atlas.layout("B", Vec2::ZERO, 12.0, [0.0; 3], false, &mut vertices);

    assert_eq!(vertices.len(), 2 * 6);
}

#[test]
fn layout_skips_whitespace_quads() {
    let atlas = test_atlas();
    assert!(layout(&atlas, " ", Vec2::ZERO, 12.0, false).is_empty());
}

#[test]
fn layout_falls_back_to_question_mark_outside_printable_ascii() {
    let atlas = test_atlas();
    // A non-ASCII character lays out exactly like the '?' fallback.
    let fallback = layout(&atlas, "?", Vec2::ZERO, 12.0, false);
    for text in ["\u{e9}", "\u{1f600}", "\n"] {
        assert_eq!(layout(&atlas, text, Vec2::ZERO, 12.0, false), fallback);
    }
}

#[test]
fn centered_layout_is_symmetric_around_anchor() {
    let atlas = test_atlas();
    let vertices = layout(&atlas, "AA", Vec2::new(100.0, 0.0), 12.0, true);

    let min_x = vertices
        .iter()
        .map(|v| v.pos.x)
        .fold(f32::INFINITY, f32::min);
    let max_x = vertices
        .iter()
        .map(|v| v.pos.x)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(((min_x + max_x) / 2.0 - 100.0).abs() < 0.5);
}

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
