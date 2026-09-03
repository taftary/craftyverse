//! GPU-side vertex and push-constant layouts shared by the pipelines.

use vulkano::buffer::BufferContents;
use vulkano::pipeline::graphics::vertex_input::Vertex;

use crate::scene::ClipTransform;

/// Colored 2D vertex of the geometry (line/triangle) pipelines.
#[derive(BufferContents, Vertex, Clone, Copy)]
#[repr(C)]
pub(crate) struct GeomVertex {
    #[format(R32G32_SFLOAT)]
    pub(crate) pos: [f32; 2],
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

/// Push-constant affine transform `clip = pos * scale + offset`; one per
/// space (`world_to_clip` and `pixel_to_clip`).
#[derive(BufferContents, Clone, Copy)]
#[repr(C)]
pub(crate) struct PushTransform {
    pub(crate) scale: [f32; 2],
    pub(crate) offset: [f32; 2],
}

impl PushTransform {
    /// Identity transform; placeholder of both clip transforms before the
    /// first scene layout.
    pub(crate) const IDENTITY: Self = PushTransform {
        scale: [1.0; 2],
        offset: [0.0; 2],
    };
}

impl From<&ClipTransform> for PushTransform {
    fn from(transform: &ClipTransform) -> Self {
        PushTransform {
            scale: transform.scale.to_array(),
            offset: transform.offset.to_array(),
        }
    }
}
