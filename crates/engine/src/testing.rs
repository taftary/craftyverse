//! Test-only surface: engine internals re-exported for the external
//! `planet-crafter-tests` crate (the workspace-root `tests/` package).
//!
//! Available only with the `test-internals` feature. Nothing here is part of
//! the public API contract: the module exists so the consolidated test suite
//! can keep its whitebox assertions without widening the engine's public API.

use crate::text::TextAtlas;

pub use crate::node::link;
pub use crate::render::{
    GEOM_FRAG, GEOM_VERT, PushMatrix, PushTransform, TEXT_FRAG, TEXT_VERT, checkbox_at,
    compile_spirv, device_type_rank, scenario_index_of,
};
pub use crate::scene::{
    ATTRIBUTES, DIRECTION_COLORS, DOT_SEGMENTS, LEVEL_COLORS, MAX_PITCH, MAX_ZOOM, MIN_ZOOM,
    VIOLATION_COLOR, hex_rgb, level_color, plane_basis, push_arrowhead, push_disc,
};
pub use crate::text::{ATLAS_HEIGHT, ATLAS_WIDTH, GLYPH_PADDING, Glyph, ShelfPacker, uv_rect};

/// Glyph for `ch` in `atlas`; characters outside printable ASCII fall back to
/// `'?'`. Test-only wrapper around the crate-private accessor.
pub fn glyph(atlas: &TextAtlas, ch: char) -> Glyph {
    atlas.glyph(ch)
}
