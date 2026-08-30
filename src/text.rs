//! Bitmap-font text layout for the Vulkan debug viewer.
//!
//! Printable ASCII glyphs are rasterized once with `fontdue` into a
//! single-channel (R8) texture atlas; `layout()` then turns a text run into
//! textured quads in pixel space. The renderer uploads `pixels` as a texture
//! and draws the quads with alpha blending.

use std::collections::HashMap;

use fontdue::{Font, FontSettings};
use glam::Vec2;

/// Glyph rasterization size in the atlas; layouts scale quads down from it.
const ATLAS_SIZE: f32 = 48.0;
const ATLAS_WIDTH: usize = 512;
const ATLAS_HEIGHT: usize = 512;
const GLYPH_PADDING: usize = 2;

/// Vertex of a text quad, positioned in pixels (y-down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextVertex {
    pub pos: Vec2,
    pub uv: Vec2,
    pub color: [f32; 3],
}

#[derive(Clone, Copy, Default)]
struct Glyph {
    uv_min: Vec2,
    uv_max: Vec2,
    /// Bitmap size in pixels at `ATLAS_SIZE`.
    size: Vec2,
    /// Bearing at `ATLAS_SIZE`: x = left side bearing, y = baseline→bottom (y-up).
    bearing: Vec2,
    advance: f32,
    has_bitmap: bool,
}

/// Rasterized glyph atlas plus the metrics needed to lay out text runs.
pub struct TextAtlas {
    glyphs: HashMap<char, Glyph>,
    /// Baseline distance from the top of the line box at `ATLAS_SIZE`.
    ascent: f32,
    pub width: u32,
    pub height: u32,
    /// R8 coverage values, row-major, `width * height` bytes.
    pub pixels: Vec<u8>,
}

impl TextAtlas {
    /// Rasterizes printable ASCII into a shelf-packed atlas.
    pub fn new(font_bytes: &[u8]) -> Result<Self, String> {
        let font = Font::from_bytes(font_bytes, FontSettings::default())
            .map_err(|err| format!("failed to load font: {err}"))?;
        let ascent = font
            .horizontal_line_metrics(ATLAS_SIZE)
            .ok_or("font has no horizontal line metrics")?
            .ascent;

        let mut pixels = vec![0u8; ATLAS_WIDTH * ATLAS_HEIGHT];
        let mut glyphs = HashMap::new();
        let mut x = GLYPH_PADDING;
        let mut y = GLYPH_PADDING;
        let mut row_height = 0usize;
        for ch in ' '..='~' {
            let (metrics, bitmap) = font.rasterize(ch, ATLAS_SIZE);
            if metrics.width == 0 || metrics.height == 0 {
                // Whitespace: advance only, no quad.
                glyphs.insert(
                    ch,
                    Glyph {
                        advance: metrics.advance_width,
                        ..Default::default()
                    },
                );
                continue;
            }
            if x + metrics.width + GLYPH_PADDING > ATLAS_WIDTH {
                x = GLYPH_PADDING;
                y += row_height + GLYPH_PADDING;
                row_height = 0;
            }
            if y + metrics.height + GLYPH_PADDING > ATLAS_HEIGHT {
                return Err("text atlas overflow".to_string());
            }
            for row in 0..metrics.height {
                let dst = (y + row) * ATLAS_WIDTH + x;
                pixels[dst..dst + metrics.width]
                    .copy_from_slice(&bitmap[row * metrics.width..(row + 1) * metrics.width]);
            }
            // Half-texel inset avoids bleeding between glyphs.
            let uv_min = Vec2::new(
                (x as f32 + 0.5) / ATLAS_WIDTH as f32,
                (y as f32 + 0.5) / ATLAS_HEIGHT as f32,
            );
            let uv_max = Vec2::new(
                ((x + metrics.width) as f32 - 0.5) / ATLAS_WIDTH as f32,
                ((y + metrics.height) as f32 - 0.5) / ATLAS_HEIGHT as f32,
            );
            glyphs.insert(
                ch,
                Glyph {
                    uv_min,
                    uv_max,
                    size: Vec2::new(metrics.width as f32, metrics.height as f32),
                    bearing: Vec2::new(metrics.xmin as f32, metrics.ymin as f32),
                    advance: metrics.advance_width,
                    has_bitmap: true,
                },
            );
            x += metrics.width + GLYPH_PADDING;
            row_height = row_height.max(metrics.height);
        }

        Ok(TextAtlas {
            glyphs,
            ascent,
            width: ATLAS_WIDTH as u32,
            height: ATLAS_HEIGHT as u32,
            pixels,
        })
    }

    fn glyph(&self, ch: char) -> Glyph {
        self.glyphs
            .get(&ch)
            .or_else(|| self.glyphs.get(&'?'))
            .copied()
            .unwrap_or_default()
    }

    /// Lays out `text` at `size` px into two triangles per glyph. `anchor` is
    /// the top-left of the text block (top-center when `centered`), in pixels.
    pub fn layout(
        &self,
        text: &str,
        anchor: Vec2,
        size: f32,
        color: [f32; 3],
        centered: bool,
    ) -> Vec<TextVertex> {
        let scale = size / ATLAS_SIZE;
        let width: f32 = text.chars().map(|ch| self.glyph(ch).advance * scale).sum();
        let mut pen_x = anchor.x - if centered { width / 2.0 } else { 0.0 };
        let baseline = anchor.y + self.ascent * scale;

        let mut vertices = Vec::with_capacity(text.len() * 6);
        for ch in text.chars() {
            let glyph = self.glyph(ch);
            if glyph.has_bitmap {
                let x0 = pen_x + glyph.bearing.x * scale;
                let y0 = baseline - (glyph.bearing.y + glyph.size.y) * scale;
                let x1 = x0 + glyph.size.x * scale;
                let y1 = y0 + glyph.size.y * scale;
                let corners = [
                    (Vec2::new(x0, y0), Vec2::new(glyph.uv_min.x, glyph.uv_min.y)),
                    (Vec2::new(x1, y0), Vec2::new(glyph.uv_max.x, glyph.uv_min.y)),
                    (Vec2::new(x1, y1), Vec2::new(glyph.uv_max.x, glyph.uv_max.y)),
                    (Vec2::new(x0, y1), Vec2::new(glyph.uv_min.x, glyph.uv_max.y)),
                ];
                for index in [0usize, 1, 2, 0, 2, 3] {
                    let (pos, uv) = corners[index];
                    vertices.push(TextVertex { pos, uv, color });
                }
            }
            pen_x += glyph.advance * scale;
        }
        vertices
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");

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
}
