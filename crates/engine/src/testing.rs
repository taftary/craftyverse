//! Test-only surface: engine internals re-exported for the external
//! `planet-crafter-tests` crate (the workspace-root `tests/` package).
//!
//! Available only with the `test-internals` feature. Nothing here is part of
//! the public API contract: the module exists so the consolidated test suite
//! can keep its whitebox assertions without widening the engine's public API.

use crate::text::TextAtlas;

pub use crate::node::link;
pub use crate::render::{
    ATMO_FRAG, ATMO_VERT, AtmoVertexGpu, AtmosphereSample, CHECKER_CELLS, CHECKER_HEIGHT,
    CHECKER_WIDTH, CHECKS_U, CHECKS_V, CameraMode, ChunkGeometry, DOME_HAZE, DOME_RISE_END,
    DOME_RISE_START, FlyCamera, GEOM_FRAG, GEOM_VERT, HORIZON_COLOR, LATITUDE_BANDS, LIGHT_DIR,
    LodReadout, MASK_EDGE_WIDTH, MAX_CHUNK_VERTICES, MIN_EYE_HEIGHT, MeshPool, PoolConfig,
    PoolStats, PushAtmosphere, PushMatrix, PushTex, PushTransform, RIM_COLOR, RIM_FADE_END,
    RIM_MAX_ALPHA, RIM_POWER, SCATTER_COLOR, SCATTER_FADE_END, SCATTER_FADE_START,
    SCATTER_MAX_ALPHA, SCATTER_RISE_END, SKY_COLOR, STRIPE_BANDS, ShellVertex, TEX_FRAG, TEX_VERT,
    TEXT_FRAG, TEXT_VERT, TexVertexGpu, VisibilityReadout, active_distance_for, appearance,
    atmosphere_lines, atmosphere_state_name, checker, checkerboard_mips, chunk_bounds,
    clamp_above_surface, clip_planes, compile_spirv, compute_chunk_vertices, culling_camera,
    device_type_rank, diffuse, dome_weight, draw_camera, edge_mask, flattening_lines, fly_speed,
    fresnel, gradient, hybrid_lines, latitude, lod_lines, mip_level_count, morph_vertex,
    morphed_chunk_bounds, overlay_lines, panel_item_at, pixel_matrix, player_marker_vertices,
    pool_lines, radial_rgb, required_capacity, rim_factor, rim_weight, rings, scatter_weight,
    scenario_index_of, shell_vertices, stripes, terrain_collision_applies, visibility_lines,
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
