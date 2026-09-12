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
/// direction and ring-field value.
#[derive(BufferContents, Vertex, Clone, Copy)]
#[repr(C)]
pub(crate) struct TexVertexGpu {
    #[format(R32G32B32_SFLOAT)]
    pub(crate) pos: [f32; 3],
    #[format(R32G32_SFLOAT)]
    pub(crate) uv: [f32; 2],
    #[format(R32G32B32_SFLOAT)]
    pub(crate) bary: [f32; 3],
    #[format(R32_SFLOAT)]
    pub(crate) parity: f32,
    #[format(R32G32B32_SFLOAT)]
    pub(crate) radial: [f32; 3],
    #[format(R32_SFLOAT)]
    pub(crate) ring: f32,
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

/// View-projection matrix, camera world position and fragment mode of the
/// textured pipeline: mode 0 samples the checkerboard texture (the UV-map
/// view), modes 1..=11 select a procedural effect (see
/// `scene::TextureEffect::shader_mode`). Both textured shader stages declare
/// the same block; the vertex stage reads only `mvp`, the fragment stage
/// reads `mode` (plus `camera_pos` for the fresnel effect). The field order
/// keeps the GLSL offsets matching the `repr(C)` layout: `vec3 camera_pos`
/// at 64, `uint mode` at 76, 80 bytes total.
#[derive(BufferContents, Clone, Copy)]
#[repr(C)]
pub struct PushTex {
    /// View-projection matrix in column-major order.
    pub mvp: [[f32; 4]; 4],
    /// Camera eye position in world space (fresnel view direction).
    pub camera_pos: [f32; 3],
    /// Fragment mode: 0 = sample the checkerboard, 1..=11 = procedural
    /// effect.
    pub mode: u32,
}

impl PushTex {
    /// Packs the view-projection matrix (column-major), the camera eye
    /// position and the fragment `mode`.
    pub fn new(mvp: [[f32; 4]; 4], camera_pos: [f32; 3], mode: u32) -> Self {
        PushTex {
            mvp,
            camera_pos,
            mode,
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
