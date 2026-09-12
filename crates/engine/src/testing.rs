//! Test-only surface: engine internals re-exported for the external
//! `planet-crafter-tests` crate (the workspace-root `tests/` package).
//!
//! Available only with the `test-internals` feature. Nothing here is part of
//! the public API contract: the module exists so the consolidated test suite
//! can keep its whitebox assertions without widening the engine's public API.

use crate::text::TextAtlas;

pub use crate::node::link;
pub use crate::render::{
    CHECKER_CELLS, CHECKER_HEIGHT, CHECKER_WIDTH, CHECKS_U, CHECKS_V, GEOM_FRAG, GEOM_VERT,
    LATITUDE_BANDS, LIGHT_DIR, MASK_EDGE_WIDTH, PushMatrix, PushTex, PushTransform, STRIPE_BANDS,
    TEX_FRAG, TEX_VERT, TEXT_FRAG, TEXT_VERT, checker, checkerboard_mips, compile_spirv,
    device_type_rank, diffuse, edge_mask, fresnel, gradient, latitude, mip_level_count,
    panel_item_at, pixel_matrix, radial_rgb, required_capacity, scenario_index_of, stripes,
};
pub use crate::scene::{
    ATTRIBUTES, DIRECTION_COLORS, DOT_SEGMENTS, EFFECTS, LEVEL_COLORS, MAX_PITCH, MAX_ZOOM,
    MIN_ZOOM, UV_DOT_COLOR, UV_LINE_COLOR, UV_PLANE_SIZE, VIOLATION_COLOR, hex_rgb, level_color,
    plane_basis, push_arrowhead, push_disc,
};
pub use crate::text::{ATLAS_HEIGHT, ATLAS_WIDTH, GLYPH_PADDING, Glyph, ShelfPacker, uv_rect};

/// Glyph for `ch` in `atlas`; characters outside printable ASCII fall back to
/// `'?'`. Test-only wrapper around the crate-private accessor.
pub fn glyph(atlas: &TextAtlas, ch: char) -> Glyph {
    atlas.glyph(ch)
}
