//! Bitmap-font text layout for the Vulkan debug viewer.
//!
//! Printable ASCII glyphs are rasterized once with `fontdue` into a
//! single-channel (R8) texture atlas. [`TextAtlas::layout`] then turns a text
//! run into textured quads positioned in pixel space. The renderer uploads
//! [`TextAtlas::pixels`] as a texture and draws the quads with alpha blending.
//!
//! The module contains no GPU code. All positions are raw pixels; the
//! world→pixel mapping of label anchors is the caller's responsibility (see the
//! `scene` module). The full contract is specified in
//! `docs/book/specs/text.md`.
//!
//! # Example
//!
//! ```
//! use planet_crafter_engine::text::TextAtlas;
//!
//! let font = include_bytes!("../../../../assets/fonts/JetBrainsMono-Regular.ttf");
//! let atlas = TextAtlas::new(font).expect("valid bundled font");
//! assert!(!atlas.pixels.is_empty());
//! ```

mod packing;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use fontdue::{Font, FontSettings};
use glam::Vec2;

use packing::{ShelfPacker, blit, uv_rect};

/// Glyph rasterization size in the atlas. Layouts scale quads down from this
/// size; linear sampling keeps small labels readable.
const ATLAS_SIZE: f32 = 48.0;
/// Width of the glyph atlas texture in pixels.
const ATLAS_WIDTH: usize = 512;
/// Height of the glyph atlas texture in pixels.
const ATLAS_HEIGHT: usize = 512;
/// Padding in pixels reserved around each packed glyph rectangle.
const GLYPH_PADDING: usize = 2;

/// Vertex of a text quad, positioned in pixels (y-down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextVertex {
    /// Pixel position (y-down).
    pub pos: Vec2,
    /// Texture coordinates into the glyph atlas.
    pub uv: Vec2,
    /// RGB color.
    pub color: [f32; 3],
}

#[derive(Clone, Copy, Default)]
struct Glyph {
    uv_min: Vec2,
    uv_max: Vec2,
    /// Bitmap size in pixels at [`ATLAS_SIZE`].
    size: Vec2,
    /// Bearing at [`ATLAS_SIZE`]: `x` is the left side bearing, `y` is
    /// baseline→bottom (y-up).
    bearing: Vec2,
    advance: f32,
    has_bitmap: bool,
}

/// Rasterized glyph atlas plus the metrics needed to lay out text runs.
pub struct TextAtlas {
    glyphs: HashMap<char, Glyph>,
    /// Baseline distance from the top of the line box at [`ATLAS_SIZE`].
    ascent: f32,
    /// Atlas width in pixels.
    pub width: u32,
    /// Atlas height in pixels.
    pub height: u32,
    /// R8 coverage values, row-major, `width * height` bytes.
    pub pixels: Vec<u8>,
}

impl TextAtlas {
    /// Rasterizes printable ASCII into a shelf-packed atlas.
    ///
    /// Parses `font_bytes` with `fontdue`, then rasterizes every printable
    /// ASCII character (`' '..='~'`) at `ATLAS_SIZE` into a 512×512 R8 atlas.
    ///
    /// # Errors
    ///
    /// Returns `Err` if:
    /// - `font_bytes` is not a valid font.
    /// - the font has no horizontal line metrics.
    /// - the glyph atlas overflows its fixed size.
    ///
    /// # Example
    ///
    /// ```
    /// use planet_crafter_engine::text::TextAtlas;
    ///
    /// let font = include_bytes!("../../../../assets/fonts/JetBrainsMono-Regular.ttf");
    /// let atlas = TextAtlas::new(font).expect("bundled font is valid");
    /// assert_eq!(atlas.width, 512);
    /// ```
    pub fn new(font_bytes: &[u8]) -> Result<Self, String> {
        let font = Font::from_bytes(font_bytes, FontSettings::default())
            .map_err(|err| format!("failed to load font: {err}"))?;
        let ascent = font
            .horizontal_line_metrics(ATLAS_SIZE)
            .ok_or("font has no horizontal line metrics")?
            .ascent;

        let mut pixels = vec![0u8; ATLAS_WIDTH * ATLAS_HEIGHT];
        let mut glyphs = HashMap::new();
        let mut packer = ShelfPacker::new();
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
            let (x, y) = packer
                .reserve(metrics.width, metrics.height)
                .ok_or("text atlas overflow")?;
            blit(
                &mut pixels,
                ATLAS_WIDTH,
                x,
                y,
                &bitmap,
                metrics.width,
                metrics.height,
            );
            let (uv_min, uv_max) = uv_rect(x, y, metrics.width, metrics.height);
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

    /// Total advance width of `text` at `scale`, used to center a text run.
    fn measure(&self, text: &str, scale: f32) -> f32 {
        text.chars().map(|ch| self.glyph(ch).advance * scale).sum()
    }

    /// Lays out `text` into two triangles per glyph.
    ///
    /// # Parameters
    ///
    /// - `text` — the string to lay out.
    /// - `anchor` — top-left of the text block, or top-center when `centered`
    ///   is `true`.
    /// - `size` — font size in pixels.
    /// - `color` — RGB color.
    /// - `centered` — when `true`, the anchor is the top-center of the block.
    ///
    /// # Returns
    ///
    /// Six [`TextVertex`] instances per glyph (two triangles), ready to be
    /// uploaded as a vertex buffer.
    ///
    /// Whitespace produces no quads; unknown characters fall back to `'?'`.
    ///
    /// # Example
    ///
    /// ```
    /// use glam::Vec2;
    /// use planet_crafter_engine::text::TextAtlas;
    ///
    /// let font = include_bytes!("../../../../assets/fonts/JetBrainsMono-Regular.ttf");
    /// let atlas = TextAtlas::new(font).expect("bundled font is valid");
    /// let vertices = atlas.layout("Hi", Vec2::new(10.0, 10.0), 16.0, [0.0, 0.0, 0.0], false);
    /// assert_eq!(vertices.len(), 2 * 6);
    /// ```
    pub fn layout(
        &self,
        text: &str,
        anchor: Vec2,
        size: f32,
        color: [f32; 3],
        centered: bool,
    ) -> Vec<TextVertex> {
        let scale = size / ATLAS_SIZE;
        let width = self.measure(text, scale);
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
                push_quad(
                    &mut vertices,
                    [
                        Vec2::new(x0, y0),
                        Vec2::new(x1, y0),
                        Vec2::new(x1, y1),
                        Vec2::new(x0, y1),
                    ],
                    (glyph.uv_min, glyph.uv_max),
                    color,
                );
            }
            pen_x += glyph.advance * scale;
        }
        vertices
    }
}

/// Pushes the two triangles (indices `[0,1,2,0,2,3]`) of a glyph quad from its
/// position corners (top-left, top-right, bottom-right, bottom-left) and its
/// UV rect.
fn push_quad(out: &mut Vec<TextVertex>, corners: [Vec2; 4], uvs: (Vec2, Vec2), color: [f32; 3]) {
    let (uv_min, uv_max) = uvs;
    let corners = [
        (corners[0], Vec2::new(uv_min.x, uv_min.y)),
        (corners[1], Vec2::new(uv_max.x, uv_min.y)),
        (corners[2], Vec2::new(uv_max.x, uv_max.y)),
        (corners[3], Vec2::new(uv_min.x, uv_max.y)),
    ];
    for index in [0usize, 1, 2, 0, 2, 3] {
        let (pos, uv) = corners[index];
        out.push(TextVertex { pos, uv, color });
    }
}
