//! Shelf packing of glyph bitmaps into the atlas texture: rectangle
//! reservation, bitmap copies and atlas-space UV rects.

use glam::Vec2;

use super::{ATLAS_HEIGHT, ATLAS_WIDTH, GLYPH_PADDING};

/// Hands out glyph rectangles left-to-right, wrapping to a new shelf when the
/// current one runs out of width.
pub(crate) struct ShelfPacker {
    x: usize,
    y: usize,
    row_height: usize,
}

impl ShelfPacker {
    /// Starts packing at the top-left padding offset.
    pub(crate) fn new() -> Self {
        ShelfPacker {
            x: GLYPH_PADDING,
            y: GLYPH_PADDING,
            row_height: 0,
        }
    }

    /// Reserves a `w` x `h` rectangle; returns `None` when the atlas is full.
    pub(crate) fn reserve(&mut self, w: usize, h: usize) -> Option<(usize, usize)> {
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
pub(crate) fn uv_rect(x: usize, y: usize, w: usize, h: usize) -> (Vec2, Vec2) {
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
