//! CPU-side scene generation for the Vulkan debug viewer.
//!
//! Generates the same visualization the SVG viewer produced — triangle
//! outlines, direction arrows, dashed origin arrow, child links, open-port
//! markers, center dots and text labels — but as plain vertex data: colored
//! line and triangle lists in a y-down world space, plus pixel-space text
//! runs. The renderer only uploads the buffers and applies the two clip
//! transforms.
//!
//! Each displayed node attribute can be toggled through [`DisplayOptions`];
//! the scene also carries a pixel-space checkbox panel (geometry plus hit
//! rectangles) that the viewer uses to flip them at runtime.
//!
//! Colors live in `colors`, display options in `options`, the per-node
//! geometry builders in `geometry` and the checkbox panel in `panel`.

mod colors;
mod geometry;
mod options;
mod panel;

#[cfg(test)]
mod tests;

use glam::Vec2;

use crate::node::NodeRef;

pub use options::{Attribute, Checkbox, DisplayOptions};

/// Colored vertex in mapped world space (y-down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub pos: Vec2,
    pub color: [f32; 3],
}

/// A text label with a pixel-space anchor (top-left, or top-center when
/// `centered`). Glyphs keep a constant pixel size regardless of the view fit.
#[derive(Clone, Debug)]
pub struct TextRun {
    pub text: String,
    pub anchor: Vec2,
    pub size: f32,
    pub color: [f32; 3],
    pub centered: bool,
}

/// Affine transform `clip = pos * scale + offset` into Vulkan clip space.
#[derive(Clone, Copy, Debug)]
pub struct ClipTransform {
    pub scale: Vec2,
    pub offset: Vec2,
}

/// Everything the renderer needs for one scene.
pub struct SceneMesh {
    /// Colored line list in mapped world space.
    pub lines: Vec<Vertex>,
    /// Colored triangle list in mapped world space (arrowheads, center dots).
    pub triangles: Vec<Vertex>,
    /// Checkbox panel geometry in pixel space (drawn with `pixel_to_clip`).
    pub ui_lines: Vec<Vertex>,
    pub ui_triangles: Vec<Vertex>,
    /// Text labels, anchored in pixel space (node labels and checkbox labels).
    pub texts: Vec<TextRun>,
    /// Checkbox hit rectangles, same order as the panel rows.
    pub checkboxes: Vec<Checkbox>,
    /// Maps `lines`/`triangles` positions to clip space.
    pub world_to_clip: ClipTransform,
    /// Maps text and UI pixel positions to clip space.
    pub pixel_to_clip: ClipTransform,
}

/// Builds the visualization of `nodes` fitted into `viewport` pixels,
/// displaying the attributes enabled in `options`.
pub fn build_scene(nodes: &[NodeRef], viewport: Vec2, options: &DisplayOptions) -> SceneMesh {
    let mut builder = SceneBuilder {
        options: *options,
        ..Default::default()
    };
    for node in nodes {
        builder.add_node(&node.borrow());
    }
    builder.add_checkbox_panel();
    builder.finish(viewport)
}

/// Label whose final pixel position is computed once the view fit is known.
struct LabelRequest {
    text: String,
    world_pos: Vec2,
    offset_px: Vec2,
    size_px: f32,
    color: [f32; 3],
    centered: bool,
}

/// Content bounds in mapped world space (y-down), accumulated while the
/// scene is built; the view fit is computed from them.
#[derive(Default)]
struct Bounds {
    min: Vec2,
    max: Vec2,
    has_content: bool,
}

impl Bounds {
    /// Extends the bounds to include `point` (already mapped to y-down).
    fn track(&mut self, point: Vec2) {
        if self.has_content {
            self.min = self.min.min(point);
            self.max = self.max.max(point);
        } else {
            self.min = point;
            self.max = point;
            self.has_content = true;
        }
    }
}

#[derive(Default)]
struct SceneBuilder {
    lines: Vec<Vertex>,
    triangles: Vec<Vertex>,
    labels: Vec<LabelRequest>,
    /// Checkbox panel geometry in pixel space.
    ui_lines: Vec<Vertex>,
    ui_triangles: Vec<Vertex>,
    /// Checkbox labels, already anchored in pixel space.
    ui_labels: Vec<TextRun>,
    checkboxes: Vec<Checkbox>,
    options: DisplayOptions,
    bounds: Bounds,
}

impl SceneBuilder {
    /// Computes the view fit (content bounds + 5% margin, aspect preserved)
    /// and resolves the label anchors to pixel positions.
    fn finish(self, viewport: Vec2) -> SceneMesh {
        let viewport = viewport.max(Vec2::ONE);
        let (origin, scale_px, pad) = fit_view(&self.bounds, viewport);
        let world_to_pixel = |point: Vec2| (point - origin) * scale_px + pad;

        let mut texts: Vec<TextRun> = self
            .labels
            .into_iter()
            .map(|label| TextRun {
                text: label.text,
                anchor: world_to_pixel(label.world_pos) + label.offset_px,
                size: label.size_px,
                color: label.color,
                centered: label.centered,
            })
            .collect();
        texts.extend(self.ui_labels);

        SceneMesh {
            lines: self.lines,
            triangles: self.triangles,
            ui_lines: self.ui_lines,
            ui_triangles: self.ui_triangles,
            texts,
            checkboxes: self.checkboxes,
            world_to_clip: to_clip_transform(
                Vec2::splat(scale_px),
                pad - origin * scale_px,
                viewport,
            ),
            pixel_to_clip: to_clip_transform(Vec2::ONE, Vec2::ZERO, viewport),
        }
    }
}

/// View fit of the content `bounds` into `viewport` pixels: bounds + 5%
/// margin, aspect preserved, centered; an empty scene fills the viewport.
/// Returns the world→pixel mapping pieces `origin`, `scale_px` and `pad`,
/// with `pixel = (world - origin) * scale_px + pad`. `viewport` must be
/// non-zero (clamped by the caller).
fn fit_view(bounds: &Bounds, viewport: Vec2) -> (Vec2, f32, Vec2) {
    let (origin, content_size) = if bounds.has_content {
        let extent = (bounds.max - bounds.min).max(Vec2::ONE);
        (bounds.min - extent * 0.05, extent * 1.1)
    } else {
        (Vec2::ZERO, viewport)
    };
    let scale_px = (viewport.x / content_size.x).min(viewport.y / content_size.y);
    let pad = (viewport - content_size * scale_px) * 0.5;
    (origin, scale_px, pad)
}

/// Vulkan clip transform of the affine mapping `pixel = pos * scale + offset`
/// (pixel space, y-down, `viewport` pixels). The identity mapping
/// (`Vec2::ONE`, `Vec2::ZERO`) is the degenerate case: raw pixel positions
/// mapped straight to clip space (`pixel_to_clip`).
fn to_clip_transform(scale: Vec2, offset: Vec2, viewport: Vec2) -> ClipTransform {
    ClipTransform {
        scale: Vec2::new(
            scale.x * 2.0 / viewport.x,
            -scale.y * 2.0 / viewport.y,
        ),
        offset: Vec2::new(
            offset.x * 2.0 / viewport.x - 1.0,
            1.0 - offset.y * 2.0 / viewport.y,
        ),
    }
}
