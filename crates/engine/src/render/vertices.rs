//! GPU-side vertex and push-constant layouts shared by the pipelines.

use glam::Mat4;
use vulkano::buffer::BufferContents;
use vulkano::pipeline::graphics::vertex_input::Vertex;

/// Colored 3D vertex of the geometry (line/triangle) pipelines.
#[derive(BufferContents, Vertex, Clone, Copy)]
#[repr(C)]
pub(crate) struct GeomVertex {
    #[format(R32G32B32_SFLOAT)]
    pub(crate) pos: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    pub(crate) color: [f32; 3],
}

/// Vertex of the text pipeline: pixel-space position, atlas UV and color.
#[derive(BufferContents, Vertex, Clone, Copy)]
#[repr(C)]
pub(crate) struct TextVertexGpu {
    #[format(R32G32_SFLOAT)]
    pub(crate) pos: [f32; 2],
    #[format(R32G32_SFLOAT)]
    pub(crate) uv: [f32; 2],
    #[format(R32G32B32_SFLOAT)]
    pub(crate) color: [f32; 3],
}

/// Vertex of the textured pipeline: world-space position, texture
/// coordinate, barycentric corner coordinate, topology parity sign, radial
/// direction and ring-field value. `pub` inside the private `vertices`
/// module; it only escapes under the `test-internals` feature for the mesh
/// pool tests.
#[derive(BufferContents, Vertex, Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct TexVertexGpu {
    /// World-space position.
    #[format(R32G32B32_SFLOAT)]
    pub pos: [f32; 3],
    /// Texture coordinate.
    #[format(R32G32_SFLOAT)]
    pub uv: [f32; 2],
    /// Barycentric corner coordinate (unit basis per corner).
    #[format(R32G32B32_SFLOAT)]
    pub bary: [f32; 3],
    /// Topology parity sign (+1/-1).
    #[format(R32_SFLOAT)]
    pub parity: f32,
    /// Normalized `direction_to_origin` of the node.
    #[format(R32G32B32_SFLOAT)]
    pub radial: [f32; 3],
    /// Ring-field value of the corner.
    #[format(R32_SFLOAT)]
    pub ring: f32,
}

/// View-projection matrix push constant of the geometry pipelines (one per
/// space: `world_mvp` and `pixel_mvp`).
#[derive(BufferContents, Clone, Copy)]
#[repr(C)]
pub struct PushMatrix {
    /// View-projection matrix in column-major order.
    pub mvp: [[f32; 4]; 4],
}

impl PushMatrix {
    /// Identity matrix; placeholder of both transforms before the first
    /// scene layout.
    pub const IDENTITY: Self = PushMatrix {
        mvp: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };
}

impl From<Mat4> for PushMatrix {
    fn from(matrix: Mat4) -> Self {
        PushMatrix {
            mvp: matrix.to_cols_array_2d(),
        }
    }
}

/// View-projection matrix, camera position, fragment mode and
/// ground-flattening state of the textured pipeline: mode 0 samples the
/// checkerboard texture (the UV-map view), modes 1..=11 select a procedural
/// effect (see `scene::TextureEffect::shader_mode`). Both textured shader
/// stages declare the same block; the vertex stage reads `mvp` plus the
/// flattening fields, the fragment stage reads `mode` (plus `camera_pos`
/// for the fresnel effect). The field order and padding keep the GLSL
/// offsets (vec3 members are 16-byte aligned) matching the `repr(C)`
/// layout: `vec3 camera_pos` at 64, `uint mode` at 76, `vec3 anchor` at 80,
/// `vec3 anchor_up` at 96, `float flatten` at 112, 116 bytes total.
///
/// `anchor` is the floating origin: the vertex shader subtracts it before
/// transforming, so `mvp` and `camera_pos` are anchor-relative while the
/// pooled vertex data stays in spherical world coordinates (feature 5,
/// Decision 3 of `plan/RELATED.md`). `anchor_up` is the outward radial at
/// the anchor (the tangent-plane normal) and `flatten` the authoritative
/// blend factor: 0 = spherical (identity morph), 1 = flat tangent plane.
#[derive(BufferContents, Clone, Copy)]
#[repr(C)]
pub struct PushTex {
    /// View-projection matrix in column-major order (anchor-relative frame
    /// when `anchor` is nonzero).
    pub mvp: [[f32; 4]; 4],
    /// Camera eye position (fresnel view direction); anchor-relative when
    /// `anchor` is nonzero.
    pub camera_pos: [f32; 3],
    /// Fragment mode: 0 = sample the checkerboard, 1..=11 = procedural
    /// effect.
    pub mode: u32,
    /// Floating-origin anchor in world coordinates; the vertex shader
    /// subtracts it from every vertex before the morph and transform.
    pub anchor: [f32; 3],
    pad0: f32,
    /// Outward radial at the anchor: the normal of the tangent plane the
    /// terrain morphs toward.
    pub anchor_up: [f32; 3],
    /// Authoritative flattening blend factor, 0 (spherical) to 1 (flat).
    /// Packs into the `anchor_up` vec3 tail, matching the GLSL block layout
    /// (offset 108, 112 bytes total).
    pub flatten: f32,
}

impl PushTex {
    /// Packs the view-projection matrix (column-major), the camera eye
    /// position and the fragment `mode`, with no flattening: a zero anchor
    /// and factor 0 make the vertex-shader morph the identity, so positions
    /// stay in plain world space (the debug viewer's usage).
    pub fn new(mvp: [[f32; 4]; 4], camera_pos: [f32; 3], mode: u32) -> Self {
        PushTex {
            mvp,
            camera_pos,
            mode,
            anchor: [0.0; 3],
            pad0: 0.0,
            anchor_up: [0.0, 1.0, 0.0],
            flatten: 0.0,
        }
    }

    /// Packs the full ground-flattening state: the anchor-relative
    /// view-projection matrix and camera eye, the fragment `mode`, the
    /// floating-origin `anchor` (world coordinates), the outward radial at
    /// the anchor and the authoritative `flatten` factor.
    pub fn morph(
        mvp: [[f32; 4]; 4],
        camera_pos: [f32; 3],
        mode: u32,
        anchor: [f32; 3],
        anchor_up: [f32; 3],
        flatten: f32,
    ) -> Self {
        PushTex {
            mvp,
            camera_pos,
            mode,
            anchor,
            pad0: 0.0,
            anchor_up,
            flatten,
        }
    }
}

/// Vertex of the atmosphere pipeline: world-space position on the shell
/// sphere and the outward radial direction (the sphere normal). `pub`
/// inside the private `vertices` module; it only escapes under the
/// `test-internals` feature for the atmosphere tests.
#[derive(BufferContents, Vertex, Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct AtmoVertexGpu {
    /// World-space position, exactly on the shell sphere.
    #[format(R32G32B32_SFLOAT)]
    pub pos: [f32; 3],
    /// Outward radial direction (unit length).
    #[format(R32G32B32_SFLOAT)]
    pub dir: [f32; 3],
}

/// Push-constant block of the atmosphere pipeline (feature 6): the
/// anchor-relative view-projection matrix and camera eye, the
/// floating-origin anchor, and the two normalized factors the appearance is
/// driven by. Both atmosphere shader stages declare the same block; the
/// vertex stage reads `mvp` and `anchor`, the fragment stage reads the
/// factors and `camera_pos`. The field order keeps the GLSL offsets (vec3
/// members are 16-byte aligned) matching the `repr(C)` layout: `vec3
/// camera_pos` at 64, `vec3 anchor` at 80, `float atmosphere_factor` at 92,
/// `float rim_factor` at 96, 100 bytes total.
///
/// `anchor` is the floating origin: the vertex shader subtracts it before
/// transforming (the shell vertex data stays in spherical world coordinates
/// and is never rewritten per frame). Unlike the terrain block there is no
/// flatten factor: the shell stays curved at all times (Decision 3 of
/// `plan/RELATED.md`). `atmosphere_factor` is the manager's normalized
/// factor (0 at/beyond the shell edge, 1 at the surface; `> 0` iff the
/// camera is inside the shell) and `rim_factor` the orbit-layer rim ramp
/// (`render::atmosphere::rim_factor`).
#[derive(BufferContents, Clone, Copy)]
#[repr(C)]
pub struct PushAtmosphere {
    /// View-projection matrix in column-major order (anchor-relative frame).
    pub mvp: [[f32; 4]; 4],
    /// Camera eye position, anchor-relative (the view direction).
    pub camera_pos: [f32; 3],
    pad0: f32,
    /// Floating-origin anchor in world coordinates; the vertex shader
    /// subtracts it from every shell vertex before the transform.
    pub anchor: [f32; 3],
    /// Manager's atmosphere factor: 0 at/beyond the shell edge, 1 at the
    /// surface. Also flags the inside view (`> 0` iff under the shell).
    pub atmosphere_factor: f32,
    /// Orbit-layer rim ramp: 0 in space, 1 at and inside the shell.
    pub rim_factor: f32,
}

impl PushAtmosphere {
    /// Packs the anchor-relative view-projection matrix (column-major) and
    /// camera eye, the floating-origin `anchor` (world coordinates) and the
    /// two appearance factors.
    pub fn new(
        mvp: [[f32; 4]; 4],
        camera_pos: [f32; 3],
        anchor: [f32; 3],
        atmosphere_factor: f32,
        rim_factor: f32,
    ) -> Self {
        PushAtmosphere {
            mvp,
            camera_pos,
            pad0: 0.0,
            anchor,
            atmosphere_factor,
            rim_factor,
        }
    }
}

/// Push-constant affine transform `clip = pos * scale + offset` of the text
/// pipeline (pixel space).
#[derive(BufferContents, Clone, Copy)]
#[repr(C)]
pub struct PushTransform {
    /// Scale factor of the transform (clip units per pixel).
    pub scale: [f32; 2],
    /// Offset of the transform (clip units).
    pub offset: [f32; 2],
}

impl PushTransform {
    /// Pixel-space → clip transform for `viewport` pixels (y-down), in the
    /// project's y-up NDC convention: pixel `(0, 0)` maps to clip `(-1, 1)`,
    /// the top-left of the window, and pixel `(w, h)` to clip `(1, -1)`.
    pub fn for_viewport(viewport: glam::Vec2) -> Self {
        PushTransform {
            scale: [2.0 / viewport.x, -2.0 / viewport.y],
            offset: [-1.0, 1.0],
        }
    }
}
