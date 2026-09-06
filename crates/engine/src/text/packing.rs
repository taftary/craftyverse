//! Shelf packing of glyph bitmaps into the atlas texture: atlas dimensions,
//! the per-glyph metrics record, rectangle reservation, bitmap copies and
//! atlas-space UV rects.

use glam::Vec2;

/// Width of the glyph atlas texture in pixels.
pub const ATLAS_WIDTH: usize = 512;
/// Height of the glyph atlas texture in pixels.
pub const ATLAS_HEIGHT: usize = 512;
/// Padding in pixels reserved around each packed glyph rectangle.
pub const GLYPH_PADDING: usize = 2;

/// Metrics and atlas placement of one rasterized glyph.
#[derive(Clone, Copy, Default)]
pub struct Glyph {
    /// Atlas-space UV of the glyph rectangle's top-left corner.
    pub uv_min: Vec2,
    /// Atlas-space UV of the glyph rectangle's bottom-right corner.
    pub uv_max: Vec2,
    /// Bitmap size in pixels at the atlas rasterization size.
    pub size: Vec2,
    /// Bearing at the atlas rasterization size: `x` is the left side
    /// bearing, `y` is baseline→bottom (y-up).
    pub bearing: Vec2,
    /// Horizontal advance in pixels at the atlas rasterization size.
    pub advance: f32,
    /// Whether the glyph produced a bitmap (whitespace has none).
    pub has_bitmap: bool,
}

/// Hands out glyph rectangles left-to-right, wrapping to a new shelf when the
/// current one runs out of width.
pub struct ShelfPacker {
    x: usize,
    y: usize,
    row_height: usize,
}

impl Default for ShelfPacker {
    fn default() -> Self {
        Self::new()
    }
}

impl ShelfPacker {
    /// Starts packing at the top-left padding offset.
    pub fn new() -> Self {
        ShelfPacker {
            x: GLYPH_PADDING,
            y: GLYPH_PADDING,
            row_height: 0,
        }
    }

    /// Reserves a `w` x `h` rectangle; returns `None` when the atlas is full.
    pub fn reserve(&mut self, w: usize, h: usize) -> Option<(usize, usize)> {
        if self.x + w + GLYPH_PADDING > ATLAS_WIDTH {
            self.x = GLYPH_PADDING;
            self.y += self.row_height + GLYPH_PADDING;
            self.row_height = 0;
        }
        if self.y + h + GLYPH_PADDING > ATLAS_HEIGHT {
            return None;
        }
        let (x, y) = (self.x, self.y);
        self.x += w + GLYPH_PADDING;
        self.row_height = self.row_height.max(h);
        Some((x, y))
    }
}

/// Copies the `w` x `h` single-channel `bitmap` into `pixels` at (`x`, `y`).
pub(crate) fn blit(
    pixels: &mut [u8],
    atlas_width: usize,
    x: usize,
    y: usize,
    bitmap: &[u8],
    w: usize,
    h: usize,
) {
    for row in 0..h {
        let dst = (y + row) * atlas_width + x;
        pixels[dst..dst + w].copy_from_slice(&bitmap[row * w..(row + 1) * w]);
    }
}

/// UV rect of the `w` x `h` glyph at (`x`, `y`). The half-texel inset avoids
/// bleeding between glyphs.
pub fn uv_rect(x: usize, y: usize, w: usize, h: usize) -> (Vec2, Vec2) {
    let uv_min = Vec2::new(
        (x as f32 + 0.5) / ATLAS_WIDTH as f32,
        (y as f32 + 0.5) / ATLAS_HEIGHT as f32,
    );
    let uv_max = Vec2::new(
        ((x + w) as f32 - 0.5) / ATLAS_WIDTH as f32,
        ((y + h) as f32 - 0.5) / ATLAS_HEIGHT as f32,
    );
    (uv_min, uv_max)
}
