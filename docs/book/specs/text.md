## Text Module Definition (`crates/engine/src/text/`)

### Overview

The `text` module provides bitmap-font text for the Vulkan debug viewer.
Printable ASCII glyphs are rasterized once with
[`fontdue`](https://crates.io/crates/fontdue) into a single-channel (R8) texture
atlas; `layout()` then converts a text run into textured quads positioned in
pixel space. The renderer uploads `pixels` as a texture and draws the quads
with alpha blending. The module contains no GPU code.

### Structure

- **`TextAtlas`** — rasterized glyph atlas plus layout metrics:
  - `pixels: Vec<u8>` — R8 coverage values, row-major, `width * height` bytes.
  - `width`, `height: u32` — atlas dimensions (512 × 512).
  - `glyphs` (private) — per-glyph UV rect, size, bearing and advance.
  - `ascent` (private) — baseline distance from the top of the line box.
- **`TextVertex { pos: Vec2, uv: Vec2, color: [f32; 3] }`** — one text quad
  vertex, positioned in pixels (y-down).

### Methods

- `TextAtlas::new(font_bytes: &[u8]) -> Result<TextAtlas, String>` — parses the
  font and rasterizes printable ASCII (`' '..='~'`) at `ATLAS_SIZE` (48 px) into
  a shelf-packed atlas (2 px padding, half-texel UV inset to avoid bleeding).
  Returns `Err` when the font is invalid or the atlas overflows.
- `layout(text, anchor, size, color, centered) -> Vec<TextVertex>` — emits two
  triangles (6 vertices) per glyph at `size` px, scaling quad geometry down
  from `ATLAS_SIZE`:
  - `anchor` — top-left of the text block, or top-center when `centered`
    (total advance width is used for centering).
  - whitespace produces no quads (advance only); unknown characters fall back
    to `'?'`.
  - glyph quads are placed with fontdue metrics: left side bearing `xmin`,
    baseline-to-bottom bearing `ymin`, line `ascent`.

### Font Asset

- `assets/fonts/JetBrainsMono-Regular.ttf` — bundled monospace font (SIL Open
  Font License), embedded into the binary with `include_bytes!`, so the viewer
  has no runtime file dependency.

### Rules

- One atlas serves all text sizes: rasterize big (48 px), scale quads down —
  linear sampling keeps small labels readable.
- All positions are raw pixels; the world→pixel mapping of label anchors is the
  caller's job (see `scene` module).

### Files

Folder module `crates/engine/src/text/`:

- **`mod.rs`** — `TextAtlas`, `TextVertex`, `Glyph`, the atlas constants and
  `layout()`; `TextAtlas::new()` orchestrates the packing helpers.
- **`packing.rs`** — atlas-construction internals: `ShelfPacker` (shelf
  packing), `blit()` (bitmap copy) and `uv_rect()` (half-texel UV inset).
