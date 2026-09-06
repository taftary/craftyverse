//! CPU-side scene generation for the Vulkan debug viewer.
//!
//! This module turns a collection of [`Node`](crate::node::Node) instances into
//! render-agnostic vertex data: colored line and triangle lists in a y-up 3D
//! world space, pixel-space UI geometry for the display-options panel, and
//! world-anchored text labels. The renderer uploads the buffers and applies
//! the camera transform on the GPU; only the label anchors are re-projected on
//! the CPU when the camera changes (see [`project_labels`]).
//!
//! The generated visualization mirrors what the former SVG viewer produced:
//! triangle outlines, direction arrows (I/J/K and `direction_of_node`), a dashed
//! origin arrow, child links, open-port markers, center dots, and text labels.
//!
//! Each displayed node attribute can be toggled at runtime through
//! [`DisplayOptions`]. The scene also carries a pixel-space checkbox panel
//! (geometry plus hit rectangles) that the viewer uses to flip the options.
//!
//! Camera math (orbit, zoom, bounding-sphere fit, label projection) lives in
//! `camera`, colors in `colors`, display options in `options`, the per-node
//! geometry builders in `geometry`, and the checkbox panel in `panel`. The
//! full contract is specified in `docs/book/specs/scene.md`.
//!
//! # Example
//!
//! ```
//! use glam::Vec3;
//! use planet_crafter_engine::node::Node;
//! use planet_crafter_engine::scene::{build_scene, DisplayOptions};
//!
//! let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
//! let scene = build_scene(&[node], &DisplayOptions::default());
//! assert!(!scene.lines.is_empty());
//! ```

mod camera;
mod colors;
mod geometry;
mod options;
mod panel;

#[cfg(test)]
mod tests;

use glam::{Vec2, Vec3};

use crate::node::NodeRef;

pub use camera::{OrbitCamera, project_labels};
pub use options::{Attribute, Checkbox, DisplayOptions, Port};

/// Colored vertex: world-space (y-up) for the `lines`/`triangles` batches,
/// pixel-space with `z = 0` for the UI batches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    /// Position in world space (y-up), or in pixel space with `z = 0` for the
    /// checkbox panel.
    pub pos: Vec3,
    /// RGB color with components in the range `[0.0, 1.0]`.
    pub color: [f32; 3],
}

/// A text label anchored in pixel space.
///
/// `anchor` is the top-left of the text block, or the top-center when
/// `centered` is `true`. Glyphs are laid out at a constant pixel size
/// regardless of the camera. The text is borrowed: panel labels borrow the
/// static attribute names, projected labels borrow their [`WorldLabel`].
#[derive(Clone, Debug)]
pub struct TextRun<'a> {
    /// Text content of the label.
    pub text: &'a str,
    /// Anchor point in pixels (y-down).
    pub anchor: Vec2,
    /// Font size in pixels.
    pub size: f32,
    /// RGB color.
    pub color: [f32; 3],
    /// When `true`, `anchor` is the top-center of the text block.
    pub centered: bool,
}

/// Pixel offset of a [`WorldLabel`], resolved at projection time.
#[derive(Clone, Copy, Debug)]
pub enum LabelOffset {
    /// Fixed pixel offset applied to the projected anchor.
    Fixed(Vec2),
    /// Push the label away from the projection of a world reference point
    /// (used for the A/B/C corner labels, pushed outward from the node
    /// center).
    Outward {
        /// World-space reference point the label is pushed away from.
        from: Vec3,
        /// Push distance in pixels.
        distance_px: f32,
    },
}

/// A text label anchored to a 3D world point.
///
/// The renderer re-projects the anchor with [`project_labels`] whenever the
/// camera changes; glyphs are then laid out at a constant pixel size.
#[derive(Clone, Debug)]
pub struct WorldLabel {
    /// Text content of the label.
    pub text: String,
    /// Anchor point in world space.
    pub world_pos: Vec3,
    /// Pixel offset applied after projection.
    pub offset: LabelOffset,
    /// Font size in pixels.
    pub size_px: f32,
    /// RGB color.
    pub color: [f32; 3],
    /// When `true`, the projected anchor is the top-center of the text block.
    pub centered: bool,
}

/// Everything the renderer needs to draw one frame of the debug viewer.
pub struct SceneMesh {
    /// Colored line list in world space (y-up). Includes outlines, arrow
    /// shafts, dashed lines, and child links.
    pub lines: Vec<Vertex>,
    /// Colored triangle list in world space. Includes arrowheads, center
    /// dots, and open-port markers.
    pub triangles: Vec<Vertex>,
    /// Checkbox panel line geometry in pixel space (`z = 0`).
    pub ui_lines: Vec<Vertex>,
    /// Checkbox panel triangle geometry in pixel space (`z = 0`).
    pub ui_triangles: Vec<Vertex>,
    /// Checkbox labels, anchored in pixel space.
    pub texts: Vec<TextRun<'static>>,
    /// World-anchored labels (node name/level and corner letters), projected
    /// to pixel space by [`project_labels`] when the camera changes.
    pub labels: Vec<WorldLabel>,
    /// Checkbox hit rectangles, in the same order as the panel rows.
    pub checkboxes: Vec<Checkbox>,
    /// Center of the content bounding sphere (world space).
    pub fit_center: Vec3,
    /// Radius of the content bounding sphere: the maximal distance from
    /// `fit_center` over all emitted world-space vertices and the world
    /// label anchors, so the whole scene fits at any camera angle (`1.0`
    /// for an empty scene).
    pub fit_radius: f32,
}

/// Builds a visualization of `nodes`, displaying the attributes enabled in
/// `options`.
///
/// The returned [`SceneMesh`] carries world-space geometry plus the content
/// bounding sphere (`fit_center` / `fit_radius`) that
/// [`OrbitCamera::view_projection`] fits into the viewport; it also contains
/// the pixel-space checkbox panel and its hit rectangles.
///
/// # Parameters
///
/// - `nodes` — nodes to visualize.
/// - `options` — which node attributes to display.
///
/// # Example
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::node::Node;
/// use planet_crafter_engine::scene::{build_scene, Attribute, DisplayOptions};
///
/// let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
/// let mut options = DisplayOptions::default();
/// options.toggle(Attribute::Labels);
/// let scene = build_scene(&[node], &options);
/// assert!(scene.labels.is_empty());
/// assert!(scene.checkboxes.iter().any(|c| c.attribute == Attribute::Labels));
/// ```
///
/// # View-fit invariant
///
/// ```
/// use glam::{Vec2, Vec3};
/// use planet_crafter_engine::node::Node;
/// use planet_crafter_engine::scene::{build_scene, OrbitCamera};
///
/// let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
/// let mesh = build_scene(&[node], &Default::default());
///
/// // The default camera fits every world-space vertex into clip space.
/// let mvp = OrbitCamera::default().view_projection(
///     mesh.fit_center,
///     mesh.fit_radius,
///     Vec2::new(800.0, 600.0),
/// );
/// for vertex in mesh.lines.iter().chain(&mesh.triangles) {
///     let clip = mvp * vertex.pos.extend(1.0);
///     let ndc = clip.truncate() / clip.w;
///     assert!(ndc.x.abs() <= 1.0, "ndc.x out of range: {}", ndc.x);
///     assert!(ndc.y.abs() <= 1.0, "ndc.y out of range: {}", ndc.y);
///     assert!((0.0..=1.0).contains(&ndc.z), "ndc.z out of range: {}", ndc.z);
/// }
/// ```
pub fn build_scene(nodes: &[NodeRef], options: &DisplayOptions) -> SceneMesh {
    let mut builder = SceneBuilder {
        options: *options,
        ..Default::default()
    };
    for node in nodes {
        builder.add_node(node);
    }
    builder.add_checkbox_panel();
    builder.finish()
}

/// Content bounds in world space, accumulated while the scene is built; the
/// bounding-sphere fit is computed from them.
#[derive(Default)]
struct Bounds {
    min: Vec3,
    max: Vec3,
    has_content: bool,
}

impl Bounds {
    /// Extends the bounds to include `point`.
    fn track(&mut self, point: Vec3) {
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
    labels: Vec<WorldLabel>,
    /// Checkbox panel geometry in pixel space.
    ui_lines: Vec<Vertex>,
    ui_triangles: Vec<Vertex>,
    /// Checkbox labels, already anchored in pixel space.
    ui_labels: Vec<TextRun<'static>>,
    checkboxes: Vec<Checkbox>,
    options: DisplayOptions,
    bounds: Bounds,
}

impl SceneBuilder {
    /// Moves the buffers out and computes the content bounding sphere.
    fn finish(self) -> SceneMesh {
        let (fit_center, fit_radius) =
            bounding_sphere(&self.bounds, &self.lines, &self.triangles, &self.labels);
        SceneMesh {
            lines: self.lines,
            triangles: self.triangles,
            ui_lines: self.ui_lines,
            ui_triangles: self.ui_triangles,
            texts: self.ui_labels,
            labels: self.labels,
            checkboxes: self.checkboxes,
            fit_center,
            fit_radius,
        }
    }
}

/// Content bounding sphere: center = bounds box center, radius = maximal
/// distance from the center over the emitted world-space vertices and the
/// world label anchors (tracked in the bounds but not emitted as
/// vertices), so every point the scene can draw fits at any camera angle —
/// including a labels-only scene. Empty scene → (origin, 1.0).
fn bounding_sphere(
    bounds: &Bounds,
    lines: &[Vertex],
    triangles: &[Vertex],
    labels: &[WorldLabel],
) -> (Vec3, f32) {
    if !bounds.has_content {
        return (Vec3::ZERO, 1.0);
    }
    let center = (bounds.min + bounds.max) * 0.5;
    let radius = lines
        .iter()
        .chain(triangles)
        .map(|vertex| vertex.pos)
        .chain(labels.iter().map(|label| label.world_pos))
        .map(|pos| (pos - center).length())
        .fold(0.0_f32, f32::max);
    (center, radius.max(camera::MIN_FIT_RADIUS))
}
