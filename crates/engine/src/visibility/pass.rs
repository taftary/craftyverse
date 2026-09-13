//! The per-frame culling pass: frustum plus horizon tests over the active
//! chunk set, reporting the visible indices and per-test cull counts.

use glam::Vec3;

use crate::node::NodeRef;

use super::frustum::{BoundingSphere, Frustum};
use super::horizon::PlanetHorizon;

/// The conservative bounding volume of one active chunk, extracted from
/// its node. This is the culling input: the pass never touches the node
/// graph beyond this read-only extraction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChunkBounds {
    /// The chunk's bounding sphere (center plus all corners and the
    /// skirt margin).
    pub sphere: BoundingSphere,
}

impl ChunkBounds {
    /// A bounding volume from an explicit sphere.
    pub fn new(center: Vec3, radius: f32) -> Self {
        ChunkBounds {
            sphere: BoundingSphere::new(center, radius),
        }
    }

    /// Extracts the bounding volume of a chunk node: centered on
    /// `Node::center`, covering every corner (`Node::vertices`) plus
    /// `skirt_margin` (the border-skirt depth, so the downward
    /// crack-masking flanges stay inside the culled volume).
    pub fn from_node(node: &NodeRef, skirt_margin: f32) -> Self {
        let node_ref = node.borrow();
        ChunkBounds {
            sphere: BoundingSphere::from_triangle(node_ref.center, node_ref.vertices, skirt_margin),
        }
    }
}

/// What one [`cull_chunks`] pass decided, for the debug overlay and the
/// draw loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibilityReport {
    /// Number of chunks tested.
    pub tested: usize,
    /// Indices into the input slice of the visible chunks, in input order.
    pub visible: Vec<usize>,
    /// Chunks culled by the camera frustum.
    pub frustum_culled: usize,
    /// Chunks culled by the planet horizon (among those the frustum kept).
    pub horizon_culled: usize,
}

/// The per-frame culling pass: tests every chunk of `chunks` (typically
/// the LOD scheduler's active set, extracted as [`ChunkBounds`]) against
/// the camera `frustum` first and the planet `horizon` second, and
/// reports the visible indices. Read-only: the result never feeds back
/// into LOD or loading.
///
/// The horizon test consumes the camera position only; with a detached
/// camera (Decision 1 of `plan/RELATED.md`) this culls against the camera
/// while the tested set still reflects the player's active zone.
pub fn cull_chunks(
    chunks: &[ChunkBounds],
    camera: Vec3,
    frustum: &Frustum,
    horizon: &PlanetHorizon,
) -> VisibilityReport {
    let mut report = VisibilityReport {
        tested: chunks.len(),
        visible: Vec::with_capacity(chunks.len()),
        frustum_culled: 0,
        horizon_culled: 0,
    };
    for (index, chunk) in chunks.iter().enumerate() {
        if !frustum.contains_sphere(&chunk.sphere) {
            report.frustum_culled += 1;
        } else if horizon.occludes(camera, &chunk.sphere) {
            report.horizon_culled += 1;
        } else {
            report.visible.push(index);
        }
    }
    report
}
