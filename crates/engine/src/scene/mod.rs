//! CPU-side scene generation for the Vulkan debug viewer.
//!
//! This module turns a collection of [`Node`](crate::node::Node) instances into
//! render-agnostic vertex data: colored line and triangle lists in a y-down
//! world space, plus pixel-space text runs. The renderer uploads the buffers and
//! applies the two clip transforms.
//!
//! The generated visualization mirrors what the former SVG viewer produced:
//! triangle outlines, direction arrows (I/J/K and `direction_of_node`), a dashed
//! origin arrow, child links, open-port markers, center dots, and text labels.
//!
//! Each displayed node attribute can be toggled at runtime through
//! [`DisplayOptions`]. The scene also carries a pixel-space checkbox panel
//! (geometry plus hit rectangles) that the viewer uses to flip the options.
//!
//! Colors live in `colors`, display options in `options`, the per-node geometry
//! builders in `geometry`, and the checkbox panel in `panel`. The full contract
//! is specified in `docs/book/specs/scene.md`.
//!
//! # Example
//!
//! ```
//! use glam::{Vec2, Vec3};
//! use planet_crafter_engine::node::Node;
//! use planet_crafter_engine::scene::{build_scene, DisplayOptions};
//!
//! let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
//! let scene = build_scene(&[node], Vec2::new(800.0, 600.0), &DisplayOptions::default());
//! assert!(!scene.lines.is_empty());
//! ```

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
    /// Position in mapped world space. The y-axis points down to match the
    /// former SVG coordinate system.
    pub pos: Vec2,
    /// RGB color with components in the range `[0.0, 1.0]`.
    pub color: [f32; 3],
}

/// A text label anchored in pixel space.
///
/// `anchor` is the top-left of the text block, or the top-center when
/// `centered` is `true`. Glyphs are laid out at a constant pixel size
/// regardless of the world-space view fit.
#[derive(Clone, Debug)]
pub struct TextRun {
    /// Text content of the label.
    pub text: String,
    /// Anchor point in pixels (y-down).
    pub anchor: Vec2,
    /// Font size in pixels.
    pub size: f32,
    /// RGB color.
    pub color: [f32; 3],
    /// When `true`, `anchor` is the top-center of the text block.
    pub centered: bool,
}

/// Affine transform from pixel/world space into Vulkan clip space.
///
/// The mapping is `clip = pos * scale + offset`.
#[derive(Clone, Copy, Debug)]
pub struct ClipTransform {
    /// Scale component of the affine mapping.
    pub scale: Vec2,
    /// Offset component of the affine mapping.
    pub offset: Vec2,
}

/// Everything the renderer needs to draw one frame of the debug viewer.
pub struct SceneMesh {
    /// Colored line list in mapped world space (y-down). Includes outlines,
    /// arrow shafts, dashed lines, and child links.
    pub lines: Vec<Vertex>,
    /// Colored triangle list in mapped world space. Includes arrowheads,
    /// center dots, and checkbox fills.
    pub triangles: Vec<Vertex>,
    /// Checkbox panel line geometry in pixel space, drawn with `pixel_to_clip`.
    pub ui_lines: Vec<Vertex>,
    /// Checkbox panel triangle geometry in pixel space, drawn with `pixel_to_clip`.
    pub ui_triangles: Vec<Vertex>,
    /// Text labels in pixel space (node labels and checkbox labels).
    pub texts: Vec<TextRun>,
    /// Checkbox hit rectangles, in the same order as the panel rows.
    pub checkboxes: Vec<Checkbox>,
    /// Maps `lines` and `triangles` world positions to clip space.
    pub world_to_clip: ClipTransform,
    /// Maps text and UI pixel positions to clip space.
    pub pixel_to_clip: ClipTransform,
}

/// Builds a visualization of `nodes` fitted into `viewport` pixels.
///
/// Only the attributes enabled in `options` are generated. The returned
/// [`SceneMesh`] also contains the checkbox panel and the transforms required
/// to render it.
///
/// # Parameters
///
/// - `nodes` — nodes to visualize.
/// - `viewport` — window size in pixels.
/// - `options` — which node attributes to display.
///
/// # Example
///
/// ```
/// use glam::{Vec2, Vec3};
/// use planet_crafter_engine::node::Node;
/// use planet_crafter_engine::scene::{build_scene, Attribute, DisplayOptions};
///
/// let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
/// let mut options = DisplayOptions::default();
/// options.toggle(Attribute::Labels);
/// let scene = build_scene(&[node], Vec2::new(800.0, 600.0), &options);
/// assert!(scene.checkboxes.iter().any(|c| c.attribute == Attribute::Labels));
/// ```
///
/// # View-fit invariant
///
/// ```
/// use glam::{Vec2, Vec3};
/// use planet_crafter_engine::node::Node;
/// use planet_crafter_engine::scene::build_scene;
///
/// let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
/// let mesh = build_scene(&[node], Vec2::new(800.0, 600.0), &Default::default());
///
/// // The world-to-clip transform maps every world-space vertex into NDC.
/// for vertex in mesh.lines.iter().chain(&mesh.triangles) {
///     let clip = vertex.pos * mesh.world_to_clip.scale + mesh.world_to_clip.offset;
///     assert!(clip.x.abs() <= 1.0, "clip.x out of range: {}", clip.x);
///     assert!(clip.y.abs() <= 1.0, "clip.y out of range: {}", clip.y);
/// }
/// ```
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
        scale: Vec2::new(scale.x * 2.0 / viewport.x, -scale.y * 2.0 / viewport.y),
        offset: Vec2::new(
            offset.x * 2.0 / viewport.x - 1.0,
            1.0 - offset.y * 2.0 / viewport.y,
        ),
    }
}
