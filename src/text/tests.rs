use super::*;

const FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");

fn test_atlas() -> TextAtlas {
    TextAtlas::new(FONT_BYTES).expect("load bundled font")
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
fn layout_emits_two_triangles_per_glyph() {
    let atlas = test_atlas();
    let vertices = atlas.layout("AB", Vec2::new(10.0, 20.0), 12.0, [0.0; 3], false);

    assert_eq!(vertices.len(), 2 * 6);
    for vertex in &vertices {
        assert!(vertex.uv.cmpge(Vec2::ZERO).all());
        assert!(vertex.uv.cmple(Vec2::ONE).all());
        assert!(vertex.pos.x >= 10.0);
        assert!(vertex.pos.y >= 20.0);
    }
}

#[test]
fn layout_skips_whitespace_quads() {
    let atlas = test_atlas();
    assert!(atlas.layout(" ", Vec2::ZERO, 12.0, [0.0; 3], false).is_empty());
}

#[test]
fn centered_layout_is_symmetric_around_anchor() {
    let atlas = test_atlas();
    let vertices = atlas.layout("AA", Vec2::new(100.0, 0.0), 12.0, [0.0; 3], true);

    let min_x = vertices.iter().map(|v| v.pos.x).fold(f32::INFINITY, f32::min);
    let max_x = vertices
        .iter()
        .map(|v| v.pos.x)
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(((min_x + max_x) / 2.0 - 100.0).abs() < 0.5);
}
