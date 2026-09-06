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
    /// Pixel-space → clip transform for `viewport` pixels (y-down).
    pub fn for_viewport(viewport: glam::Vec2) -> Self {
        PushTransform {
            scale: [2.0 / viewport.x, -2.0 / viewport.y],
            offset: [-1.0, 1.0],
        }
    }
}
