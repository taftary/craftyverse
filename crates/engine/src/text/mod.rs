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

use fontdue::{Font, FontSettings};
use glam::Vec2;

use packing::blit;

#[cfg(feature = "test-internals")]
pub use packing::{ATLAS_HEIGHT, ATLAS_WIDTH, GLYPH_PADDING, Glyph, ShelfPacker, uv_rect};
#[cfg(not(feature = "test-internals"))]
use packing::{ATLAS_HEIGHT, ATLAS_WIDTH, Glyph, ShelfPacker, uv_rect};

/// Glyph rasterization size in the atlas. Layouts scale quads down from this
/// size; linear sampling keeps small labels readable.
const ATLAS_SIZE: f32 = 48.0;

/// Number of rasterized glyphs: the printable ASCII range `' '..='~'`.
const GLYPH_COUNT: usize = ('~' as usize) - (' ' as usize) + 1;
/// Glyph index of the fallback character for text outside printable ASCII.
const FALLBACK_GLYPH: usize = ('?' as usize) - (' ' as usize);

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

/// Failure modes of [`TextAtlas::new`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextAtlasError {
    /// The font bytes could not be parsed (carries the parser's message).
    InvalidFont(String),
    /// The font provides no horizontal line metrics.
    NoHorizontalLineMetrics,
    /// The rasterized glyphs do not fit the fixed-size atlas.
    AtlasOverflow,
}

impl std::fmt::Display for TextAtlasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextAtlasError::InvalidFont(message) => write!(f, "failed to load font: {message}"),
            TextAtlasError::NoHorizontalLineMetrics => {
                write!(f, "font has no horizontal line metrics")
            }
            TextAtlasError::AtlasOverflow => write!(f, "text atlas overflow"),
        }
    }
}

impl std::error::Error for TextAtlasError {}

/// Rasterized glyph atlas plus the metrics needed to lay out text runs.
pub struct TextAtlas {
    /// One glyph per printable ASCII character, indexed by
    /// `ch as usize - ' ' as usize`.
    glyphs: [Glyph; GLYPH_COUNT],
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
    /// - [`TextAtlasError::InvalidFont`] — `font_bytes` is not a valid font.
    /// - [`TextAtlasError::NoHorizontalLineMetrics`] — the font has no
    ///   horizontal line metrics.
    /// - [`TextAtlasError::AtlasOverflow`] — the glyph atlas overflows its
    ///   fixed size.
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
    pub fn new(font_bytes: &[u8]) -> Result<Self, TextAtlasError> {
        let font = Font::from_bytes(font_bytes, FontSettings::default())
            .map_err(|err| TextAtlasError::InvalidFont(err.to_string()))?;
        let ascent = font
            .horizontal_line_metrics(ATLAS_SIZE)
            .ok_or(TextAtlasError::NoHorizontalLineMetrics)?
            .ascent;

        let mut pixels = vec![0u8; ATLAS_WIDTH * ATLAS_HEIGHT];
        let mut glyphs = [Glyph::default(); GLYPH_COUNT];
        let mut packer = ShelfPacker::new();
        for ch in ' '..='~' {
            let (metrics, bitmap) = font.rasterize(ch, ATLAS_SIZE);
            if metrics.width == 0 || metrics.height == 0 {
                // Whitespace: advance only, no quad.
                glyphs[ch as usize - ' ' as usize] = Glyph {
                    advance: metrics.advance_width,
                    ..Default::default()
                };
                continue;
            }
            let (x, y) = packer
                .reserve(metrics.width, metrics.height)
                .ok_or(TextAtlasError::AtlasOverflow)?;
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
            glyphs[ch as usize - ' ' as usize] = Glyph {
                uv_min,
                uv_max,
                size: Vec2::new(metrics.width as f32, metrics.height as f32),
                bearing: Vec2::new(metrics.xmin as f32, metrics.ymin as f32),
                advance: metrics.advance_width,
                has_bitmap: true,
            };
        }

        Ok(TextAtlas {
            glyphs,
            ascent,
            width: ATLAS_WIDTH as u32,
            height: ATLAS_HEIGHT as u32,
            pixels,
        })
    }

    /// Glyph for `ch`; characters outside printable ASCII fall back to `'?'`.
    pub(crate) fn glyph(&self, ch: char) -> Glyph {
        let index = (ch as usize).wrapping_sub(' ' as usize);
        self.glyphs
            .get(index)
            .copied()
            .unwrap_or(self.glyphs[FALLBACK_GLYPH])
    }

    /// Total advance width of `text` at `scale`, used to center a text run.
    fn measure(&self, text: &str, scale: f32) -> f32 {
        text.chars().map(|ch| self.glyph(ch).advance * scale).sum()
    }

    /// Lays out `text` into two triangles per glyph, appending to `out`.
    ///
    /// # Parameters
    ///
    /// - `text` — the string to lay out.
    /// - `anchor` — top-left of the text block, or top-center when `centered`
    ///   is `true`.
    /// - `size` — font size in pixels.
    /// - `color` — RGB color.
    /// - `centered` — when `true`, the anchor is the top-center of the block.
    /// - `out` — buffer the six [`TextVertex`] instances per glyph (two
    ///   triangles) are appended to; clear it first to reuse it.
    ///
    /// Whitespace produces no quads; characters outside printable ASCII fall
    /// back to `'?'`.
    ///
    /// # Example
    ///
    /// ```
    /// use glam::Vec2;
    /// use planet_crafter_engine::text::TextAtlas;
    ///
    /// let font = include_bytes!("../../../../assets/fonts/JetBrainsMono-Regular.ttf");
    /// let atlas = TextAtlas::new(font).expect("bundled font is valid");
    /// let mut vertices = Vec::new();
    /// atlas.layout("Hi", Vec2::new(10.0, 10.0), 16.0, [0.0, 0.0, 0.0], false, &mut vertices);
    /// assert_eq!(vertices.len(), 2 * 6);
    /// ```
    pub fn layout(
        &self,
        text: &str,
        anchor: Vec2,
        size: f32,
        color: [f32; 3],
        centered: bool,
        out: &mut Vec<TextVertex>,
    ) {
        let scale = size / ATLAS_SIZE;
        let width = self.measure(text, scale);
        let mut pen_x = anchor.x - if centered { width / 2.0 } else { 0.0 };
        let baseline = anchor.y + self.ascent * scale;

        out.reserve(text.len() * 6);
        for ch in text.chars() {
            let glyph = self.glyph(ch);
            if glyph.has_bitmap {
                let x0 = pen_x + glyph.bearing.x * scale;
                let y0 = baseline - (glyph.bearing.y + glyph.size.y) * scale;
                let x1 = x0 + glyph.size.x * scale;
                let y1 = y0 + glyph.size.y * scale;
                push_quad(
                    out,
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
