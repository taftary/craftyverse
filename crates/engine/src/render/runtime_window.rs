//! The planet runtime window: a second winit window dedicated to the planet
//! runtime (Decision 6 of `plan/RELATED.md`), alongside the unchanged debug
//! viewer window.
//!
//! The window renders the active LOD chunks of the planet through the mesh
//! pool (`super::pool`): the LOD scheduler ([`LodScheduler`]) splits, merges,
//! loads and unloads chunks as the player moves, and every change only
//! rewrites vertex data inside the fixed pool slots - no GPU mesh object is
//! ever created at runtime. Each chunk is one node triangle plus a short
//! skirt quad per non-welded border (the T-junction crack mask), drawn with
//! the textured pipeline (diffuse effect), plus the text overlay. The
//! free-fly player camera doubles as the player proxy: its position is fed
//! each frame into [`PlanetRuntimeManager::update`] and
//! [`LodScheduler::update`], and the resulting readouts are shown as a text
//! overlay (same glyph-atlas pattern as the viewer's labels). **F** toggles
//! the navigation camera: a free-fly spectator camera for navigating space
//! that changes nothing but the viewpoint - LOD/loading keep following the
//! player and culling keeps following the PLAYER camera (Decision 1 of
//! `plan/RELATED.md`), so the navigation camera sees whatever the player
//! camera would, gaps included. A small world-space cross marks the player
//! position in both camera modes. Only the chunks that
//! survive the visibility pass
//! ([`cull_chunks`](crate::visibility::cull_chunks): frustum plus
//! conservative horizon culling over the active chunk bounding spheres)
//! are drawn - one draw batch per visible live slot. World rendering is
//! anchor-relative (feature 5, Decision 3 of `plan/RELATED.md`): the vertex
//! shader subtracts the authoritative floating-origin anchor from every
//! (spherical, unmodified) pooled vertex and morphs it toward the tangent
//! plane at the anchor by the authoritative flatten factor, and the draw
//! view-projection is built from the anchor-relative camera, so f32
//! precision error far from the planet center stays contained. The culling
//! pass tests the same morphed geometry: while the flatten factor is
//! nonzero the chunk bounds are built from the morphed corners and the
//! horizon occlusion body shrinks to the sphere inscribed in the morphed
//! ellipsoid ([`PlanetHorizon::morphed`]), so chunks are never culled by
//! their unmorphed spherical volumes. After the
//! opaque terrain, the curved atmosphere shell (feature 6) is drawn every
//! frame through its own alpha-blended, depth-tested (no depth writes)
//! pipeline: the shell is a fixed lat-long sphere at the atmosphere shell
//! radius, never morphed (it stays curved at all times), shifted into the
//! anchor-relative frame in its vertex shader; the single atmosphere
//! fragment shader is driven by the manager's `atmosphere_factor` and the
//! CPU-side orbit-layer rim ramp only (no independent distance math), so
//! the appearance goes from invisible in space through the orbit rim, the
//! curved scattering layer and the horizon fog to the full sky dome over
//! the flattened ground with no hard cuts. The window
//! owns its own device, swapchain and pipelines — built from the shared
//! `setup` pieces — so the two windows render independently.
//!
//! Controls: left drag = mouse look, W/A/S/D = move in the view plane,
//! Space/C = rise/sink, Shift = speed boost, mouse wheel = speed multiplier,
//! F = player/navigation camera toggle, R = respawn beyond the
//! orbit threshold (player camera). The base fly speed scales with
//! altitude so the full space-to-ground sweep stays comfortable
//! ([`fly_speed`]).

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use glam::{Mat4, Vec2, Vec3};
use vulkano::buffer::Subbuffer;
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassBeginInfo,
    SubpassEndInfo,
};
use vulkano::descriptor_set::DescriptorSet;
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::image::view::ImageView;
use vulkano::instance::Instance;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::graphics::color_blend::AttachmentBlend;
use vulkano::pipeline::graphics::input_assembly::PrimitiveTopology;
use vulkano::pipeline::graphics::vertex_input::Vertex;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{Framebuffer, RenderPass, Subpass};
use vulkano::swapchain::{
    Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo, acquire_next_image,
};
use vulkano::sync::{self, GpuFuture};
use vulkano::{Validated, VulkanError};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::lod::{LodConfig, LodScheduler};
use crate::node::{NodeRef, build_icosphere, destroy_mesh};
use crate::runtime::{PlanetConfig, PlanetRuntimeManager, RuntimeState};
use crate::scene::TextureEffect;
use crate::text::TextAtlas;
use crate::visibility::{ChunkBounds, Frustum, PlanetHorizon, cull_chunks};

use super::atmosphere::{self, SHELL_SLICES, SHELL_STACKS};
use super::buffers::VertexBuffer;
use super::pool::{MAX_CHUNK_VERTICES, MeshPool, PoolConfig, PoolStats, SKIRT_DEPTH_FACTOR};
use super::renderer::record_draw;
use super::setup;
use super::shaders::{ATMO_FRAG, ATMO_VERT, TEX_FRAG, TEX_VERT, TEXT_FRAG, TEXT_VERT, load_shader};
use super::vertices::{
    AtmoVertexGpu, PushAtmosphere, PushTex, PushTransform, TexVertexGpu, TextVertexGpu,
};

/// Number of mesh-pool slots of the runtime window, fixed at creation. The
/// LOD configuration keeps the active chunk count far below this bound; if
/// it is ever reached, assignments queue until slots are released (they
/// never grow the pool).
const POOL_SLOTS: usize = 1024;

/// Number of vertex worker threads of the mesh pool.
fn default_workers() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1))
        .unwrap_or(1)
        .clamp(1, 4)
}

/// The LOD configuration of the runtime window's planet, derived from the
/// planet radius: splits begin at 1.5 planet radii (geometric x2 per level
/// from there), the active zone is a sphere of 0.75 radii around the player.
fn lod_config(config: &PlanetConfig) -> LodConfig {
    LodConfig {
        base_split_distance: 1.5 * config.planet_radius,
        hysteresis_ratio: 1.3,
        max_level: 4,
        operations_per_frame: 2,
        active_distance: 0.75 * config.planet_radius,
        min_active_meshes: 20,
    }
}

/// Mouse-look sensitivity, in radians per pixel of drag.
const LOOK_RADIANS_PER_PIXEL: f32 = 0.005;

/// Speed multiplier applied while Shift is held.
const BOOST_FACTOR: f32 = 8.0;

/// Speed multiplier step per mouse-wheel notch.
const WHEEL_SPEED_STEP: f32 = 1.25;

/// Bounds of the user speed multiplier.
const MIN_SPEED_FACTOR: f32 = 1.0 / 32.0;
const MAX_SPEED_FACTOR: f32 = 32.0;

/// Pitch clamp of the fly camera: it never quite reaches the poles, where
/// the up vector would be parallel to the view direction.
const MAX_FLY_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.01;

/// Vertical field of view of the fly camera (45°), matching the viewer.
const FIELD_OF_VIEW: f32 = std::f32::consts::FRAC_PI_4;

/// Maximum frame delta applied to movement, so a stalled event loop (window
/// drag, breakpoint) never teleports the player.
const MAX_FRAME_DT: f32 = 0.1;

/// Base fly speed in world units per second for `altitude` (signed height
/// above the surface): proportional to the altitude magnitude, clamped away
/// from zero so ground-level flight stays possible and from extreme values.
/// The result is scaled by the boost and the user wheel multiplier.
pub fn fly_speed(altitude: f32) -> f32 {
    altitude.abs().clamp(1.0, 100_000.0)
}

/// Clip planes of the fly camera for a frame: the near plane scales with the
/// distance to the planet center (depth precision from orbit down to the
/// ground), the far plane always covers the whole planet.
pub fn clip_planes(distance_to_center: f32, planet_radius: f32) -> (f32, f32) {
    let near = (distance_to_center * 1e-3).clamp(0.05, 100.0);
    let far = distance_to_center + 4.0 * planet_radius;
    (near, far)
}

/// Free-fly camera of the runtime window. Two instances exist: the player
/// camera, which doubles as the player proxy (its position is published to
/// the runtime manager and drives culling), and the navigation camera, a
/// spectator that only changes the viewpoint.
///
/// Yaw rotates around the world Y axis, pitch is the elevation above the
/// horizon (clamped just short of the poles). At identity angles the camera
/// sits looking down world -Z with up +Y, matching the viewer's head-on
/// frame. Pure glam math — no GPU code — so every piece is unit-testable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlyCamera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    /// User speed multiplier, adjusted with the mouse wheel.
    speed_factor: f32,
}

impl FlyCamera {
    /// Spawns the camera beyond the orbit threshold (`orbit_radius * 1.1`
    /// from the planet center, along +Z), facing the planet.
    pub fn spawn(config: &PlanetConfig) -> Self {
        let position =
            config.planet_origin + Vec3::Z * config.orbit_radius() * SPAWN_DISTANCE_FACTOR;
        let mut camera = FlyCamera {
            position,
            yaw: 0.0,
            pitch: 0.0,
            speed_factor: 1.0,
        };
        camera.face_toward(config.planet_origin);
        camera
    }

    /// Current camera (= player) position in world units.
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Current user speed multiplier.
    pub fn speed_factor(&self) -> f32 {
        self.speed_factor
    }

    /// Unit forward direction of the camera.
    pub fn forward(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        Vec3::new(sin_yaw * cos_pitch, sin_pitch, -cos_yaw * cos_pitch)
    }

    /// Unit right direction of the camera (horizontal, perpendicular to the
    /// forward direction).
    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    /// Aims the camera so its forward direction points at `target`.
    pub fn face_toward(&mut self, target: Vec3) {
        let direction = (target - self.position).normalize_or_zero();
        if direction == Vec3::ZERO {
            return;
        }
        self.yaw = direction.x.atan2(-direction.z);
        self.pitch = direction.y.asin().clamp(-MAX_FLY_PITCH, MAX_FLY_PITCH);
    }

    /// Adds `delta_yaw` / `delta_pitch` (radians) to the look angles; the
    /// pitch is clamped to just short of the poles.
    pub fn look(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch).clamp(-MAX_FLY_PITCH, MAX_FLY_PITCH);
    }

    /// Moves the camera by `offset` expressed in the camera frame: `x` along
    /// the camera right, `y` along world up, `z` along the camera forward.
    pub fn move_local(&mut self, offset: Vec3) {
        self.position += self.right() * offset.x + Vec3::Y * offset.y + self.forward() * offset.z;
    }

    /// Multiplies the user speed multiplier by `factor`, clamped to
    /// `1/32..=32`.
    pub fn adjust_speed_factor(&mut self, factor: f32) {
        self.speed_factor = (self.speed_factor * factor).clamp(MIN_SPEED_FACTOR, MAX_SPEED_FACTOR);
    }

    /// View-projection matrix of the camera for `viewport` pixels and the
    /// given clip planes, in the project's y-up NDC convention (same as
    /// [`OrbitCamera::view_projection`](crate::scene::OrbitCamera::view_projection)).
    pub fn view_projection(&self, viewport: Vec2, near: f32, far: f32) -> Mat4 {
        self.view_projection_relative(viewport, near, far, Vec3::ZERO)
    }

    /// View-projection matrix in the frame of the floating origin `anchor`:
    /// the camera eye is translated to `position - anchor`, so the rendered
    /// coordinates stay small (the vertex shader subtracts the same anchor
    /// from every vertex before transforming - feature 5, Decision 3 of
    /// `plan/RELATED.md`). At a zero anchor this is [`Self::view_projection`].
    pub fn view_projection_relative(
        &self,
        viewport: Vec2,
        near: f32,
        far: f32,
        anchor: Vec3,
    ) -> Mat4 {
        let viewport = viewport.max(Vec2::ONE);
        let aspect = viewport.x / viewport.y;
        let eye = self.position - anchor;
        let view = glam::camera::rh::view::look_at_mat4(eye, eye + self.forward(), Vec3::Y);
        glam::camera::rh::proj::directx::perspective(FIELD_OF_VIEW, aspect, near, far) * view
    }
}

/// Spawn distance of the fly camera, as a factor of the orbit radius
/// (beyond the space/orbit threshold, so the sweep starts in space).
const SPAWN_DISTANCE_FACTOR: f32 = 1.1;

/// The active camera of the runtime window (feature 4 camera model,
/// Decision 1 of `plan/RELATED.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraMode {
    /// The player camera: first-person, attached to the player. Flying
    /// moves the player; LOD/loading follow the player and culling follows
    /// this camera.
    Player,
    /// The navigation camera: a free-fly spectator for navigating space.
    /// Only the viewpoint changes - LOD/loading still follow the player
    /// and culling still follows the player camera.
    Navigation,
}

impl CameraMode {
    /// The overlay readout name.
    pub fn name(self) -> &'static str {
        match self {
            CameraMode::Player => "player",
            CameraMode::Navigation => "navigation",
        }
    }
}

/// The camera the visibility pass culls against: always the player camera,
/// in every mode (the navigation camera never re-culls). Headless policy,
/// unit-tested.
pub fn culling_camera<'a>(
    player_camera: &'a FlyCamera,
    _nav_camera: &'a FlyCamera,
    _mode: CameraMode,
) -> &'a FlyCamera {
    player_camera
}

/// The camera whose viewpoint is rendered in `mode`.
pub fn draw_camera<'a>(
    player_camera: &'a FlyCamera,
    nav_camera: &'a FlyCamera,
    mode: CameraMode,
) -> &'a FlyCamera {
    match mode {
        CameraMode::Player => player_camera,
        CameraMode::Navigation => nav_camera,
    }
}

/// The live readout lines of the debug overlay, formatted from one frame's
/// [`RuntimeState`] plus the current fly speed. Pure formatting, unit-tested
/// headless; the controls hint is appended by the window (static).
pub fn overlay_lines(state: &RuntimeState, fly_speed: f32) -> Vec<String> {
    let player = state.anchor + state.local_player;
    vec![
        format!(
            "player position:    ({:.1}, {:.1}, {:.1})",
            player.x, player.y, player.z
        ),
        format!("distance to center: {:.1}", state.distance_to_center),
        format!("altitude:           {:.1}", state.altitude),
        format!("layer:              {}", state.layer.name()),
        format!("atmosphere factor:  {:.3}", state.atmosphere_factor),
        format!("flatten factor:     {:.3}", state.flatten_factor),
        format!(
            "anchor:             ({:.1}, {:.1}, {:.1})",
            state.anchor.x, state.anchor.y, state.anchor.z
        ),
        format!("fly speed:          {fly_speed:.1}"),
    ]
}

/// Static controls hint shown below the readouts.
const CONTROLS_HINT: &str =
    "drag: look  WASD: move  Space/C: up/down  Shift: boost  wheel: speed  F: nav cam  R: respawn";

/// Lateral distance from the anchor (as a factor of the planet radius) at
/// which [`flattening_lines`] samples the morphed surface for the
/// world-flatten readout.
const WORLD_FLATTEN_SAMPLE_FACTOR: f32 = 0.25;

/// The live ground-flattening readout lines of the debug overlay (feature
/// 5): the gravity blend, the blended gravity direction, the world flatten
/// factor and the f32 precision-error bound. Pure formatting over the
/// authoritative [`RuntimeState`], unit-tested headless.
///
/// The world flatten factor is the morph factor recovered from the
/// surface-height query ([`surface_height`](crate::runtime::surface_height))
/// sampled at `0.25` planet radii lateral distance from the anchor - the
/// same query future picking will use, so the readout demonstrates that the
/// rendered surface and the headless query agree at the current blend.
///
/// The precision bound is the maximum f32 rounding error of one operation
/// on a coordinate of magnitude `distance(player, anchor)`
/// ([`precision_error_bound`](crate::runtime::precision_error_bound)): the
/// error the floating origin keeps contained near the player.
pub fn flattening_lines(state: &RuntimeState, planet_radius: f32) -> Vec<String> {
    let up = crate::runtime::anchor_up(state);
    let gravity = crate::runtime::gravity_direction(state, state.anchor + state.local_player);
    let lateral = WORLD_FLATTEN_SAMPLE_FACTOR * planet_radius;
    let sphere_depth = planet_radius - (planet_radius * planet_radius - lateral * lateral).sqrt();
    // Any tangent direction works (the setup is radially symmetric); pick
    // the axis least aligned with `up` for a stable cross product.
    let axis = if up.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let tangent = up.cross(axis).normalize_or(Vec3::Z);
    let sample = state.anchor + tangent * lateral;
    let world_flatten = match crate::runtime::surface_height(state, sample) {
        // The sampled height is `-sphere_depth * (1 - factor)`.
        Some(height) => format!("{:.3}", (1.0 + height / sphere_depth).clamp(0.0, 1.0)),
        None => "-".to_string(),
    };
    vec![
        format!(
            "gravity:            {:.0}% flat",
            state.flatten_factor * 100.0
        ),
        format!(
            "gravity direction:  ({:.3}, {:.3}, {:.3})",
            gravity.x, gravity.y, gravity.z
        ),
        format!("world flatten:      {world_flatten}"),
        format!(
            "f32 error bound:    {:.2e}",
            crate::runtime::precision_error_bound(state)
        ),
    ]
}

/// The live atmosphere readout lines of the debug overlay (feature 6): the
/// active atmosphere appearance state (`none` / `rim` / `scattering` /
/// `fog` / `sky dome`, derived from the layer) and the configured shell
/// radius multiplier. The normalized distance factor itself is already part
/// of [`overlay_lines`] (`atmosphere factor`). Pure formatting over the
/// authoritative [`RuntimeState`], unit-tested headless.
pub fn atmosphere_lines(state: &RuntimeState, atmosphere_multiplier: f32) -> Vec<String> {
    vec![
        format!(
            "atmosphere state:   {}",
            atmosphere::atmosphere_state_name(state.layer)
        ),
        format!("shell multiplier:   {atmosphere_multiplier:.2}"),
    ]
}

/// The live LOD scheduler readouts of one frame (feature 2), collected by
/// the window from the scheduler and its last [`FrameReport`](crate::lod::FrameReport).
pub struct LodReadout {
    /// Number of currently active (loaded) chunks.
    pub active_chunks: usize,
    /// Active chunk count per subdivision level, sorted by level.
    pub level_histogram: Vec<(u32, usize)>,
    /// Split/merge operations queued for budget.
    pub queued_operations: usize,
    /// Per-frame operation budget.
    pub operations_budget: usize,
    /// Operations executed this frame.
    pub operations_used: usize,
    /// Total splits executed since spawn.
    pub total_splits: u64,
    /// Total merges executed since spawn.
    pub total_merges: u64,
}

/// The LOD readout lines of the debug overlay. Pure formatting, unit-tested
/// headless.
pub fn lod_lines(readout: &LodReadout) -> Vec<String> {
    let histogram = if readout.level_histogram.is_empty() {
        "-".to_string()
    } else {
        readout
            .level_histogram
            .iter()
            .map(|(level, count)| format!("L{level}:{count}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    vec![
        format!("loaded chunks:      {}", readout.active_chunks),
        format!("chunk levels:       {histogram}"),
        format!("queued operations:  {}", readout.queued_operations),
        format!(
            "budget used:        {}/{}",
            readout.operations_used, readout.operations_budget
        ),
        format!(
            "splits/merges:      {}/{}",
            readout.total_splits, readout.total_merges
        ),
    ]
}

/// The conservative bounding volume of one chunk for the visibility pass:
/// [`ChunkBounds::from_node`] with the border-skirt depth as the margin,
/// so the crack-masking skirts stay inside the culled volume. Headless and
/// unit-tested.
pub fn chunk_bounds(node: &NodeRef) -> ChunkBounds {
    let node_ref = node.borrow();
    let [a, b, c] = node_ref.vertices;
    let shortest_edge = (b - a).length().min((c - b).length()).min((a - c).length());
    drop(node_ref);
    ChunkBounds::from_node(node, SKIRT_DEPTH_FACTOR * shortest_edge)
}

/// The morph-aware bounding volume of one chunk (feature 5 fix): the
/// corners displaced by the authoritative morph
/// ([`morph_point`](crate::runtime::morph_point)) at the state's flatten
/// factor, with the same skirt margin as [`chunk_bounds`]. The vertex
/// shader displaces the rendered vertices by up to `flatten_factor x
/// height above the anchor plane` - far beyond the spherical bounding
/// radius near the active-zone edge - so culling against the unmorphed
/// volume drops chunks that are visibly rendered (the disappearing-mesh
/// bug). The morph is affine: the morphed chunk is exactly the triangle
/// through the morphed corners, the skirt displacement only contracts
/// under it, and the morphed center is the morph of the center, so this
/// volume conservatively covers the rendered chunk at every factor.
/// Headless and unit-tested.
pub fn morphed_chunk_bounds(node: &NodeRef, state: &RuntimeState) -> ChunkBounds {
    let node_ref = node.borrow();
    let [a, b, c] = node_ref.vertices;
    let shortest_edge = (b - a).length().min((c - b).length()).min((a - c).length());
    let center = crate::runtime::morph_point(node_ref.center, state);
    let vertices = node_ref
        .vertices
        .map(|vertex| crate::runtime::morph_point(vertex, state));
    drop(node_ref);
    ChunkBounds::from_triangle(center, vertices, SKIRT_DEPTH_FACTOR * shortest_edge)
}

/// Half-length of the player marker arms as a factor of the draw camera's
/// distance to the player (keeps the marker at a roughly constant screen
/// size), with a floor so it never degenerates.
const MARKER_SIZE_FACTOR: f32 = 0.02;
const MARKER_MIN_SIZE: f32 = 0.5;

/// Arm width of the player marker, as a factor of the arm half-length.
const MARKER_WIDTH_FACTOR: f32 = 0.12;

/// The vertex data of the player position marker: a 3-bar cross (one bar
/// along `up`, two along the tangent directions) centered on the player,
/// drawn through the textured pipeline with the same anchor-relative morph
/// constants as the terrain, so it stays glued to the rendered world at
/// every flatten factor. `half_size` is the arm half-length in world
/// units. Headless and unit-tested.
pub fn player_marker_vertices(player: Vec3, half_size: f32, up: Vec3) -> Vec<TexVertexGpu> {
    let half_size = half_size.max(MARKER_MIN_SIZE * MARKER_WIDTH_FACTOR);
    let width = half_size * MARKER_WIDTH_FACTOR;
    let up = up.normalize_or(Vec3::Y);
    // Any tangent direction works; pick the axis least aligned with `up`
    // for a stable cross product.
    let axis = if up.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let tangent_u = up.cross(axis).normalize_or(Vec3::Z);
    let tangent_v = up.cross(tangent_u).normalize_or(Vec3::X);
    let mut out = Vec::with_capacity(18);
    for (bar, across) in [(up, tangent_u), (tangent_u, up), (tangent_v, up)] {
        let a = bar * half_size;
        let w = across * width;
        let corners = [
            player - a - w,
            player + a - w,
            player + a + w,
            player - a + w,
        ];
        let vertex = |pos: Vec3| TexVertexGpu {
            pos: pos.to_array(),
            uv: [0.0, 0.0],
            bary: [1.0, 0.0, 0.0],
            parity: 1.0,
            radial: up.to_array(),
            ring: 0.0,
        };
        out.extend_from_slice(&[
            vertex(corners[0]),
            vertex(corners[1]),
            vertex(corners[2]),
            vertex(corners[0]),
            vertex(corners[2]),
            vertex(corners[3]),
        ]);
    }
    out
}

/// The visibility readout lines of the debug overlay (feature 4).
/// Pure formatting, unit-tested headless.
pub struct VisibilityReadout {
    /// The active camera (player or navigation spectator).
    pub camera: CameraMode,
    /// Chunks tested by the culling pass (the active set).
    pub tested: usize,
    /// Chunks surviving both culling tests.
    pub visible: usize,
    /// Chunks culled by the camera frustum.
    pub frustum_culled: usize,
    /// Chunks culled by the planet horizon.
    pub horizon_culled: usize,
    /// Terrain draw batches issued this frame.
    pub draw_calls: usize,
}

/// The visibility readout lines of the debug overlay. Pure formatting,
/// unit-tested headless.
pub fn visibility_lines(readout: &VisibilityReadout) -> Vec<String> {
    vec![
        format!("camera:             {}", readout.camera.name()),
        format!("chunks visible:     {}/{}", readout.visible, readout.tested),
        format!("frustum culled:     {}", readout.frustum_culled),
        format!("horizon culled:     {}", readout.horizon_culled),
        format!("draw calls:         {}", readout.draw_calls),
    ]
}

/// The mesh-pool readout lines of the debug overlay (feature 3).
/// `vertex_writes` is the number of slot vertices rewritten this frame.
/// Pure formatting, unit-tested headless.
pub fn pool_lines(stats: &PoolStats, vertex_writes: usize) -> Vec<String> {
    vec![
        format!("pool capacity:      {}", stats.capacity),
        format!(
            "slots used/free:    {}/{}",
            stats.used,
            stats.capacity - stats.used
        ),
        format!("queued assignments: {}", stats.queued_assignments),
        format!("vertex writes:      {vertex_writes}"),
        format!("pending jobs:       {}", stats.pending_jobs),
        format!(
            "workers:            {} threads, {} jobs done",
            stats.workers, stats.completed_jobs
        ),
    ]
}

/// The runtime window: owns the planet mesh graph, the LOD scheduler, the
/// mesh pool, the player and navigation cameras, the pressed-key state and
/// its renderer; fed window events by the viewer's `ApplicationHandler`
/// (routed by window id).
pub(crate) struct RuntimeWindow {
    manager: PlanetRuntimeManager,
    /// The chunk LOD scheduler; owns the traversal roots of the planet
    /// graph (built at subdivision 0, refined by the scheduler).
    scheduler: LodScheduler,
    /// The fixed-capacity mesh pool; slot GPU buffers live in the renderer.
    pool: MeshPool,
    /// The player camera: the player proxy. Its position feeds the runtime
    /// manager and the LOD scheduler every frame, and the visibility pass
    /// always culls against it.
    player_camera: FlyCamera,
    /// The navigation camera: a free-fly spectator that only changes the
    /// rendered viewpoint (active in `CameraMode::Navigation`).
    nav_camera: FlyCamera,
    /// Which camera is active (F toggles).
    mode: CameraMode,
    spawn_config: PlanetConfig,
    /// Total splits/merges executed since spawn, and the operations used by
    /// the last scheduler update (overlay readouts).
    total_splits: u64,
    total_merges: u64,
    operations_used: usize,
    /// Slot vertices rewritten by the last pool poll (overlay readout).
    vertex_writes: usize,
    /// Currently held movement/boost keys.
    forward: bool,
    back: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    boost: bool,
    /// Last cursor position, in physical pixels.
    cursor: Vec2,
    /// Whether the left button is currently dragging a look.
    dragging: bool,
    /// Timestamp of the previous redraw, for frame deltas.
    last_frame: Instant,
    /// The fixed atmosphere shell vertex data (feature 6), built once at
    /// spawn at the atmosphere shell radius; uploaded once to the GPU and
    /// never rewritten.
    shell: Vec<AtmoVertexGpu>,
    renderer: Option<RuntimeRenderer>,
}

impl RuntimeWindow {
    pub(crate) fn new(config: PlanetConfig) -> Self {
        let manager =
            PlanetRuntimeManager::new(config).expect("invalid planet runtime configuration");
        let mesh = build_icosphere("planet", config.planet_radius, 0, config.planet_origin);
        let scheduler = LodScheduler::new(lod_config(&config), mesh.faces.clone())
            .expect("invalid LOD configuration");
        let pool = MeshPool::new(PoolConfig {
            slots: POOL_SLOTS,
            workers: default_workers(),
        });
        let camera = FlyCamera::spawn(&config);
        // The curved atmosphere shell (feature 6): a fixed lat-long sphere
        // at the atmosphere shell radius (planet radius x the configured
        // multiplier), built once and never morphed or rebuilt.
        let shell = atmosphere::shell_vertices(
            config.planet_origin,
            config.atmosphere_radius(),
            SHELL_SLICES,
            SHELL_STACKS,
        )
        .into_iter()
        .map(|vertex| AtmoVertexGpu {
            pos: vertex.pos.to_array(),
            dir: vertex.dir.to_array(),
        })
        .collect();
        let mut window = RuntimeWindow {
            manager,
            scheduler,
            pool,
            player_camera: camera,
            nav_camera: camera,
            mode: CameraMode::Player,
            spawn_config: config,
            total_splits: 0,
            total_merges: 0,
            operations_used: 0,
            vertex_writes: 0,
            forward: false,
            back: false,
            left: false,
            right: false,
            up: false,
            down: false,
            boost: false,
            cursor: Vec2::ZERO,
            dragging: false,
            last_frame: Instant::now(),
            shell,
            renderer: None,
        };
        // Prime the scheduler and the pool from the spawn position so the
        // initial active set (and its slot assignments) exists before the
        // first frame.
        let report = window.scheduler.update(window.player_camera.position());
        window.pool.apply_report(&report);
        window
    }

    /// Creates the window and its renderer; called from
    /// `ApplicationHandler::resumed` after the viewer window exists.
    pub(crate) fn resumed(&mut self, event_loop: &ActiveEventLoop, instance: Arc<Instance>) {
        if self.renderer.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("PlanetCrafter planet runtime — drag: look, WASD/Space/C: fly, Shift: boost, wheel: speed, F: nav cam, R: respawn"),
                )
                .expect("failed to create runtime window"),
        );
        let shell = std::mem::take(&mut self.shell);
        self.renderer = Some(RuntimeRenderer::new(instance, window, POOL_SLOTS, &shell));
        self.last_frame = Instant::now();
        self.request_redraw();
    }

    /// The runtime window's id, once created.
    pub(crate) fn window_id(&self) -> Option<WindowId> {
        self.renderer.as_ref().map(|renderer| renderer.window.id())
    }

    /// Handles one window event addressed to the runtime window.
    pub(crate) fn handle_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.mark_resized();
                    renderer.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => self.on_cursor_moved(position),
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    self.dragging = state == ElementState::Pressed;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => self.on_mouse_wheel(delta),
            WindowEvent::KeyboardInput { event, .. } => self.on_key_input(event),
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }

    /// The camera the fly controls currently drive (player camera in
    /// player mode, navigation camera in navigation mode).
    fn active_camera_mut(&mut self) -> &mut FlyCamera {
        match self.mode {
            CameraMode::Player => &mut self.player_camera,
            CameraMode::Navigation => &mut self.nav_camera,
        }
    }

    /// Tracks the cursor; while dragging, rotates the active fly camera.
    fn on_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let cursor = Vec2::new(position.x as f32, position.y as f32);
        let delta = cursor - self.cursor;
        self.cursor = cursor;
        if self.dragging {
            self.active_camera_mut().look(
                -delta.x * LOOK_RADIANS_PER_PIXEL,
                -delta.y * LOOK_RADIANS_PER_PIXEL,
            );
            self.request_redraw();
        }
    }

    /// Mouse wheel scales the user speed multiplier (x1.25 per notch).
    fn on_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let steps = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(position) => position.y as f32 / 50.0,
        };
        if steps == 0.0 {
            return;
        }
        self.active_camera_mut()
            .adjust_speed_factor(WHEEL_SPEED_STEP.powf(steps));
        self.request_redraw();
    }

    /// Tracks the held movement/boost keys; R respawns beyond the orbit
    /// threshold.
    fn on_key_input(&mut self, event: KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        let key = match event.physical_key {
            PhysicalKey::Code(code) => code,
            _ => return,
        };
        match key {
            KeyCode::KeyW => self.forward = pressed,
            KeyCode::KeyS => self.back = pressed,
            KeyCode::KeyA => self.left = pressed,
            KeyCode::KeyD => self.right = pressed,
            KeyCode::Space => self.up = pressed,
            KeyCode::KeyC => self.down = pressed,
            KeyCode::ShiftLeft | KeyCode::ShiftRight => self.boost = pressed,
            KeyCode::KeyF if pressed && !event.repeat => {
                // Toggle the navigation camera: entering spectator mode
                // continues from the current view; the player camera (and
                // the player) stays put while the navigation camera flies.
                // LOD/loading follow the player and culling follows the
                // player camera in every mode (Decision 1).
                self.mode = match self.mode {
                    CameraMode::Player => {
                        self.nav_camera = self.player_camera;
                        CameraMode::Navigation
                    }
                    CameraMode::Navigation => CameraMode::Player,
                };
                self.request_redraw();
            }
            KeyCode::KeyR if pressed && !event.repeat => {
                let spawn = FlyCamera::spawn(&self.spawn_config);
                self.player_camera = spawn;
                self.nav_camera = spawn;
                self.mode = CameraMode::Player;
                self.request_redraw();
            }
            _ => {}
        }
        if pressed && self.moving() {
            self.request_redraw();
        }
    }

    /// Whether any movement key is currently held (drives the continuous
    /// redraw loop while flying).
    fn moving(&self) -> bool {
        self.forward || self.back || self.left || self.right || self.up || self.down
    }

    fn request_redraw(&self) {
        if let Some(renderer) = self.renderer.as_ref() {
            renderer.request_redraw();
        }
    }

    /// One frame: applies the held movement keys over the frame delta to
    /// the active camera (the player camera moves the player; the
    /// navigation camera is a spectator), feeds the player position to the
    /// runtime manager and the LOD scheduler, applies the scheduler's
    /// report to the mesh pool, culls the active chunks against the player
    /// camera, and draws the surviving chunks plus the player marker, the
    /// atmosphere shell and the overlay readouts. Redraws continue while
    /// a movement key is held, worker jobs are in flight, or the scheduler
    /// queue holds operations (smooth flight and prompt async completion
    /// under `ControlFlow::Wait`).
    fn redraw(&mut self) {
        let moving = self.moving();
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(MAX_FRAME_DT);
        self.last_frame = now;

        let config = self.manager.config();
        let mut state = self.manager.update(self.player_camera.position());
        if moving {
            // The navigation camera flies with its own altitude-based
            // speed (it can be far from the player); the player camera
            // flies with the player's altitude and moves the player.
            let nav_altitude =
                self.nav_camera.position().distance(config.planet_origin) - config.planet_radius;
            let (altitude, camera) = match self.mode {
                CameraMode::Player => (state.altitude, &mut self.player_camera),
                CameraMode::Navigation => (nav_altitude, &mut self.nav_camera),
            };
            let intent = Vec3::new(
                (self.right as i8 - self.left as i8) as f32,
                (self.up as i8 - self.down as i8) as f32,
                (self.forward as i8 - self.back as i8) as f32,
            )
            .normalize_or_zero();
            let boost = if self.boost { BOOST_FACTOR } else { 1.0 };
            let speed = fly_speed(altitude) * boost * camera.speed_factor();
            camera.move_local(intent * speed * dt);
            state = self.manager.update(self.player_camera.position());
        }

        // LOD + mesh pool: drain completed worker results, run the
        // scheduler for this frame, and dispatch the new vertex jobs. LOD
        // and loading follow the player, never a camera (Decision 1).
        let outcome = self.pool.poll();
        self.vertex_writes = outcome.vertex_writes;
        let report = self.scheduler.update(self.player_camera.position());
        self.operations_used = report.splits.len() + report.merges.len();
        self.total_splits += report.splits.len() as u64;
        self.total_merges += report.merges.len() as u64;
        self.pool.apply_report(&report);
        let stats = self.pool.stats();

        let viewport = renderer.viewport();
        let draw_camera = *draw_camera(&self.player_camera, &self.nav_camera, self.mode);
        let draw_distance = draw_camera.position().distance(config.planet_origin);
        let (near, far) = clip_planes(draw_distance, config.planet_radius);
        // Floating-origin rendering (feature 5): the draw transform and the
        // camera eye are anchor-relative; the vertex shader subtracts the
        // same anchor from every (spherical, unmodified) pooled vertex.
        let render_mvp = draw_camera.view_projection_relative(viewport, near, far, state.anchor);

        // Visibility pass (feature 4): frustum plus conservative horizon
        // culling over the active chunk set. Culling always follows the
        // PLAYER camera, in every mode (the navigation camera never
        // re-culls). While the flatten factor is nonzero the chunk bounds
        // are built from the morphed corners and the horizon occluder is
        // the sphere inscribed in the morphed ellipsoid, so the pass tests
        // the rendered (morphed) geometry, never the unmorphed sphere.
        // Only the surviving slots are drawn.
        let cull_camera = *culling_camera(&self.player_camera, &self.nav_camera, self.mode);
        let chunks = self.scheduler.active_chunks();
        let bounds: Vec<ChunkBounds> = if state.flatten_factor > 0.0 {
            chunks
                .iter()
                .map(|node| morphed_chunk_bounds(node, &state))
                .collect()
        } else {
            chunks.iter().map(chunk_bounds).collect()
        };
        let (cull_near, cull_far) = clip_planes(state.distance_to_center, config.planet_radius);
        let cull_mvp = cull_camera.view_projection(viewport, cull_near, cull_far);
        let frustum = Frustum::from_view_projection(cull_mvp);
        let horizon = PlanetHorizon::morphed(
            config.planet_origin,
            config.planet_radius,
            crate::runtime::anchor_up(&state),
            state.flatten_factor,
        );
        let visibility = cull_chunks(&bounds, cull_camera.position(), &frustum, &horizon);
        let mut visible_slots = vec![false; POOL_SLOTS];
        let mut draw_calls = 0usize;
        for &index in &visibility.visible {
            let name = chunks[index].borrow().name.clone();
            if let Some(slot) = self.pool.slot_of(&name) {
                visible_slots[slot] = true;
                if !self.pool.slot_vertices(slot).is_empty() {
                    draw_calls += 1;
                }
            }
        }

        // The player position marker: a small world-space cross, drawn in
        // both camera modes, sized by the draw camera's distance so it
        // stays visible from far away.
        let player = self.player_camera.position();
        let marker_size =
            (draw_camera.position().distance(player) * MARKER_SIZE_FACTOR).max(MARKER_MIN_SIZE);
        let marker = player_marker_vertices(player, marker_size, crate::runtime::anchor_up(&state));

        let active_speed_factor = match self.mode {
            CameraMode::Player => self.player_camera.speed_factor(),
            CameraMode::Navigation => self.nav_camera.speed_factor(),
        };
        let speed = fly_speed(state.altitude)
            * if self.boost { BOOST_FACTOR } else { 1.0 }
            * active_speed_factor;
        let mut lines = overlay_lines(&state, speed);
        lines.extend(flattening_lines(&state, config.planet_radius));
        lines.extend(atmosphere_lines(&state, config.atmosphere_multiplier));
        let mut histogram: BTreeMap<u32, usize> = BTreeMap::new();
        for chunk in chunks {
            *histogram.entry(chunk.borrow().level).or_default() += 1;
        }
        lines.extend(lod_lines(&LodReadout {
            active_chunks: chunks.len(),
            level_histogram: histogram.into_iter().collect(),
            queued_operations: self.scheduler.queued_operations(),
            operations_budget: self.scheduler.config().operations_per_frame,
            operations_used: self.operations_used,
            total_splits: self.total_splits,
            total_merges: self.total_merges,
        }));
        lines.extend(pool_lines(&stats, self.vertex_writes));
        lines.extend(visibility_lines(&VisibilityReadout {
            camera: self.mode,
            tested: visibility.tested,
            visible: visibility.visible.len(),
            frustum_culled: visibility.frustum_culled,
            horizon_culled: visibility.horizon_culled,
            draw_calls,
        }));
        lines.push(CONTROLS_HINT.to_string());
        renderer.draw_frame(
            render_mvp,
            draw_camera.position(),
            &state,
            atmosphere::rim_factor(
                state.distance_to_center,
                config.orbit_radius(),
                config.atmosphere_radius(),
            ),
            &marker,
            &lines,
            &mut self.pool,
            &visible_slots,
        );

        if moving || stats.pending_jobs > 0 || self.scheduler.queued_operations() > 0 {
            renderer.request_redraw();
        }
    }
}

impl Drop for RuntimeWindow {
    fn drop(&mut self) {
        // Destroy the planet graph: the pool only holds chunk `Rc`s (no
        // cycles), but the bidirectional node links must be severed
        // explicitly. The active set is never empty after the initial
        // update (`min_active_meshes` >= 1).
        if let Some(root) = self.scheduler.active_chunks().first() {
            destroy_mesh(root);
        }
    }
}

/// Renderer of the runtime window: its own device, swapchain, render pass
/// and three pipelines (textured world triangles, the atmosphere shell and
/// overlay text), orchestrated
/// from the shared `setup` functions. The terrain is drawn from the mesh
/// pool: one pre-allocated slot vertex buffer per pool slot (stable GPU
/// allocations, sized to `MAX_CHUNK_VERTICES`; never created or resized at
/// runtime), each visible live chunk slot one draw batch (the visibility
/// pass of the `visibility` module marks the surviving slots). The
/// atmosphere shell is one fixed vertex buffer (built once at startup,
/// never rewritten) drawn after the opaque terrain through its own
/// alpha-blended, depth-tested (no depth writes) pipeline. The overlay
/// text buffer is rebuilt every frame.
struct RuntimeRenderer {
    window: Arc<Window>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    queue_family_index: u32,
    swapchain: Arc<Swapchain>,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    depth_view: Arc<ImageView>,
    tex_pipeline: Arc<GraphicsPipeline>,
    text_pipeline: Arc<GraphicsPipeline>,
    atmo_pipeline: Arc<GraphicsPipeline>,
    text_descriptor_set: Arc<DescriptorSet>,
    tex_descriptor_set: Arc<DescriptorSet>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    atlas: TextAtlas,
    /// One stable GPU allocation per mesh-pool slot, indexed by slot.
    slot_buffers: Vec<VertexBuffer<TexVertexGpu>>,
    /// The fixed atmosphere shell vertex buffer (feature 6).
    shell_buffer: Option<Subbuffer<[AtmoVertexGpu]>>,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    window_resized: bool,
}

impl RuntimeRenderer {
    fn new(
        instance: Arc<Instance>,
        window: Arc<Window>,
        pool_slots: usize,
        shell: &[AtmoVertexGpu],
    ) -> Self {
        let surface = Surface::from_window(instance.clone(), window.clone())
            .expect("failed to create runtime surface");
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
        let (physical_device, queue_family_index) =
            setup::pick_physical_device(&instance, &surface, &device_extensions);
        let (device, queue) = setup::create_device_and_queue(
            &physical_device,
            &device_extensions,
            queue_family_index,
        );

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        ));
        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let (swapchain, images) =
            setup::create_swapchain(&physical_device, &device, surface, &window);
        let render_pass = setup::create_render_pass(&device, &swapchain);
        let depth_view = setup::create_depth_view(&memory_allocator, swapchain.image_extent());
        let framebuffers = setup::create_framebuffers(&images, &depth_view, &render_pass);

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        let tex_vs = load_shader(&device, TEX_VERT, naga::ShaderStage::Vertex);
        let tex_fs = load_shader(&device, TEX_FRAG, naga::ShaderStage::Fragment);
        let tex_pipeline = setup::graphics_pipeline(
            &device,
            tex_vs,
            tex_fs,
            TexVertexGpu::per_vertex(),
            PrimitiveTopology::TriangleList,
            None,
            true,
            true,
            &subpass,
        );
        // The atmosphere shell pipeline (feature 6): alpha-blended, drawn
        // after the opaque terrain, depth-tested against it but writing no
        // depth of its own (like the text pipeline blends, but in the world
        // depth buffer).
        let atmo_vs = load_shader(&device, ATMO_VERT, naga::ShaderStage::Vertex);
        let atmo_fs = load_shader(&device, ATMO_FRAG, naga::ShaderStage::Fragment);
        let atmo_pipeline = setup::graphics_pipeline(
            &device,
            atmo_vs,
            atmo_fs,
            AtmoVertexGpu::per_vertex(),
            PrimitiveTopology::TriangleList,
            Some(AttachmentBlend::alpha()),
            true,
            false,
            &subpass,
        );
        let text_vs = load_shader(&device, TEXT_VERT, naga::ShaderStage::Vertex);
        let text_fs = load_shader(&device, TEXT_FRAG, naga::ShaderStage::Fragment);
        let text_pipeline = setup::graphics_pipeline(
            &device,
            text_vs,
            text_fs,
            TextVertexGpu::per_vertex(),
            PrimitiveTopology::TriangleList,
            Some(AttachmentBlend::alpha()),
            false,
            false,
            &subpass,
        );

        let (atlas, text_descriptor_set) = setup::upload_atlas(
            &device,
            &queue,
            queue_family_index,
            &memory_allocator,
            &command_buffer_allocator,
            &descriptor_set_allocator,
            &text_pipeline,
        );
        let tex_descriptor_set = setup::upload_checkerboard(
            &device,
            &queue,
            queue_family_index,
            &memory_allocator,
            &command_buffer_allocator,
            &descriptor_set_allocator,
            &tex_pipeline,
        );

        // One stable GPU allocation per mesh-pool slot: created once, never
        // resized or recreated during any LOD transition.
        let slot_buffers = (0..pool_slots)
            .map(|_| VertexBuffer::with_capacity(&memory_allocator, MAX_CHUNK_VERTICES))
            .collect();
        // The atmosphere shell: one fixed upload, never rewritten.
        let shell_buffer = setup::vertex_buffer(&memory_allocator, shell.to_vec());

        RuntimeRenderer {
            window,
            device: device.clone(),
            queue,
            queue_family_index,
            swapchain,
            render_pass,
            framebuffers,
            depth_view,
            tex_pipeline,
            text_pipeline,
            atmo_pipeline,
            text_descriptor_set,
            tex_descriptor_set,
            memory_allocator,
            command_buffer_allocator,
            atlas,
            slot_buffers,
            shell_buffer,
            previous_frame_end: Some(sync::now(device).boxed()),
            window_resized: false,
        }
    }

    /// Window size in physical pixels.
    fn viewport(&self) -> Vec2 {
        let size = self.window.inner_size();
        Vec2::new(size.width as f32, size.height as f32)
    }

    fn request_redraw(&self) {
        self.window.request_redraw();
    }

    fn mark_resized(&mut self) {
        self.window_resized = true;
    }

    /// Draws one frame: uploads the pool's dirty slots (behind a device
    /// wait, the same in-place rewrite discipline as the viewer), draws
    /// every live chunk slot marked in `visible_slots` through the textured
    /// pipeline (diffuse effect), then the player marker (a small
    /// world-space cross through the same pipeline), then the atmosphere
    /// shell (blended,
    /// depth-tested, no depth writes), then the overlay text laid out in
    /// pixel space. `render_mvp` is the anchor-relative view-projection: the
    /// vertex shaders subtract `state.anchor` from every (spherical,
    /// unmodified) vertex; the terrain morphs by `state.flatten_factor`,
    /// the shell never morphs. `rim_factor` is the CPU-side orbit-layer rim
    /// ramp ([`atmosphere::rim_factor`]).
    #[allow(clippy::too_many_arguments)]
    fn draw_frame(
        &mut self,
        render_mvp: Mat4,
        eye: Vec3,
        state: &RuntimeState,
        rim_factor: f32,
        marker: &[TexVertexGpu],
        overlay: &[String],
        pool: &mut MeshPool,
        visible_slots: &[bool],
    ) {
        let window_size = self.window.inner_size();
        if window_size.width == 0 || window_size.height == 0 {
            return;
        }
        if self.window_resized {
            self.window_resized = false;
            self.recreate_swapchain();
        }
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        // Upload the slots whose vertex data changed since the last frame.
        // The updates rewrite the live range of the stable slot
        // allocations in place; a device wait first keeps the rewrites from
        // racing a frame still in flight.
        let dirty = pool.take_dirty();
        if !dirty.is_empty() {
            // SAFETY: the renderer is single-threaded — the pool updates and
            // the command submission below both run on the winit event loop
            // thread, so nothing submits to the device's queues while this
            // waits. The pool workers only touch plain CPU vertex data,
            // never the device.
            unsafe { self.device.wait_idle() }.expect("failed to wait for the device");
            // The wait only idles the GPU: vulkano's per-buffer bookkeeping
            // still counts the previous frame's reads until its fence future
            // is cleaned. The fence is known-signaled after the wait, so
            // this deterministically propagates `signal_finished` and
            // unlocks the buffers for host writes (same pattern as
            // `Renderer::set_scene`).
            self.previous_frame_end.as_mut().unwrap().cleanup_finished();
            for (slot, vertices) in dirty {
                self.slot_buffers[slot].update(&self.memory_allocator, vertices);
            }
        }

        let (image_index, suboptimal, acquire_future) =
            match acquire_next_image(self.swapchain.clone(), None).map_err(Validated::unwrap) {
                Ok(result) => result,
                Err(VulkanError::OutOfDate) => {
                    self.window_resized = true;
                    return;
                }
                Err(err) => panic!("failed to acquire next image: {err}"),
            };
        if suboptimal {
            self.window_resized = true;
        }

        // The overlay changes every frame; rebuild the small text buffer.
        let mut text_data = Vec::new();
        for (row, line) in overlay.iter().enumerate() {
            self.atlas.layout(
                line,
                Vec2::new(10.0, 10.0 + row as f32 * 18.0),
                16.0,
                [0.85, 0.9, 1.0],
                false,
                &mut text_data,
            );
        }
        let text_buffer = setup::vertex_buffer(
            &self.memory_allocator,
            text_data
                .into_iter()
                .map(|v| TextVertexGpu {
                    pos: v.pos.to_array(),
                    uv: v.uv.to_array(),
                    color: v.color,
                })
                .collect(),
        );

        let mut builder = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.queue_family_index,
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    // Dark space background, in contrast to the viewer's white.
                    clear_values: vec![Some([0.05, 0.07, 0.12, 1.0].into()), Some(1.0.into())],
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[image_index as usize].clone(),
                    )
                },
                SubpassBeginInfo::default(),
            )
            .unwrap()
            .set_viewport(
                0,
                [Viewport {
                    offset: [0.0; 2],
                    extent: [window_size.width as f32, window_size.height as f32],
                    depth_range: 0.0..=1.0,
                }]
                .into_iter()
                .collect(),
            )
            .unwrap();

        for (slot, slot_buffer) in self.slot_buffers.iter().enumerate() {
            if !visible_slots.get(slot).copied().unwrap_or(false) {
                continue;
            }
            if let Some((chunk, count)) = slot_buffer.batch() {
                record_draw(
                    &mut builder,
                    &self.tex_pipeline,
                    PushTex::morph(
                        render_mvp.to_cols_array_2d(),
                        (eye - state.anchor).to_array(),
                        TextureEffect::Diffuse.shader_mode(),
                        state.anchor.to_array(),
                        crate::runtime::anchor_up(state).to_array(),
                        state.flatten_factor,
                    ),
                    &chunk,
                    count,
                    Some(&self.tex_descriptor_set),
                );
            }
        }
        // The player position marker: a tiny world-space cross, drawn
        // through the same textured pipeline with the same morph constants
        // (depth-tested like the terrain), rebuilt every frame like the
        // text buffer.
        if !marker.is_empty() {
            let marker_buffer = setup::vertex_buffer(&self.memory_allocator, marker.to_vec());
            if let Some(marker_buffer) = marker_buffer {
                record_draw(
                    &mut builder,
                    &self.tex_pipeline,
                    PushTex::morph(
                        render_mvp.to_cols_array_2d(),
                        (eye - state.anchor).to_array(),
                        TextureEffect::Diffuse.shader_mode(),
                        state.anchor.to_array(),
                        crate::runtime::anchor_up(state).to_array(),
                        state.flatten_factor,
                    ),
                    &marker_buffer,
                    marker.len() as u32,
                    Some(&self.tex_descriptor_set),
                );
            }
        }
        // The curved atmosphere shell (feature 6): drawn every frame after
        // the opaque terrain, blended and depth-tested against it (no depth
        // writes). The push constants carry the manager's atmosphere factor
        // and the CPU-side orbit rim ramp; the shader does no independent
        // distance math.
        if let Some(shell) = &self.shell_buffer {
            record_draw(
                &mut builder,
                &self.atmo_pipeline,
                PushAtmosphere::new(
                    render_mvp.to_cols_array_2d(),
                    (eye - state.anchor).to_array(),
                    state.anchor.to_array(),
                    state.atmosphere_factor,
                    rim_factor,
                ),
                shell,
                shell.len() as u32,
                None,
            );
        }
        if let Some(text) = &text_buffer {
            record_draw(
                &mut builder,
                &self.text_pipeline,
                PushTransform::for_viewport(self.viewport()),
                text,
                text.len() as u32,
                Some(&self.text_descriptor_set),
            );
        }

        builder.end_render_pass(SubpassEndInfo::default()).unwrap();
        let command_buffer = builder.build().unwrap();

        let future = self
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush();
        match future.map_err(Validated::unwrap) {
            Ok(future) => self.previous_frame_end = Some(future.boxed()),
            Err(VulkanError::OutOfDate) => {
                self.window_resized = true;
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
            Err(err) => panic!("failed to flush future: {err}"),
        }
    }

    /// Recreates the swapchain, the depth view and the framebuffers at the
    /// current window size.
    fn recreate_swapchain(&mut self) {
        let image_extent: [u32; 2] = self.window.inner_size().into();
        let (swapchain, images) = self
            .swapchain
            .recreate(SwapchainCreateInfo {
                image_extent,
                ..self.swapchain.create_info().clone()
            })
            .expect("failed to recreate swapchain");
        self.swapchain = swapchain;
        self.depth_view = setup::create_depth_view(&self.memory_allocator, image_extent);
        self.framebuffers =
            setup::create_framebuffers(&images, &self.depth_view, &self.render_pass);
    }
}
