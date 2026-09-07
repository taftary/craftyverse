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
//! The viewer switches between three [`ViewMode`]s: [`ViewMode::Mesh`] (the
//! attribute/line debug view described above), [`ViewMode::Textured`] (filled
//! world-space node triangles carrying per-corner UVs, drawn with a texture)
//! and [`ViewMode::UvMap`] (the UV net laid flat as a world-space z = 0
//! plane, textured, with a wireframe overlay and vertex-distribution dots).
//! The textured and UV-map batches (`tex_world`, `tex_uv`, `uv_lines`) are
//! emitted only by their mode; the checkbox panel is emitted in every mode.
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
//! use planet_crafter_engine::scene::{ViewMode, build_scene, DisplayOptions};
//!
//! let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
//! let scene = build_scene(&[node], &DisplayOptions::default(), ViewMode::Mesh);
//! assert!(!scene.lines.is_empty());
//! ```

mod camera;
mod colors;
mod geometry;
mod options;
mod panel;

use glam::{Vec2, Vec3};

use crate::node::NodeRef;

pub use camera::{OrbitCamera, project_labels};
pub use options::{Attribute, Checkbox, DisplayOptions, Port};

#[cfg(feature = "test-internals")]
pub use camera::{MAX_PITCH, MAX_ZOOM, MIN_ZOOM};
#[cfg(feature = "test-internals")]
pub use colors::{
    DIRECTION_COLORS, LEVEL_COLORS, UV_DOT_COLOR, UV_LINE_COLOR, VIOLATION_COLOR, hex_rgb,
    level_color,
};
#[cfg(feature = "test-internals")]
pub use geometry::{DOT_SEGMENTS, plane_basis, push_arrowhead, push_disc};
#[cfg(feature = "test-internals")]
pub use options::ATTRIBUTES;

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

/// Which visualization [`build_scene`] emits.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ViewMode {
    /// Attribute/line debug view (default): outlines, arrows, markers and
    /// labels, toggled by [`DisplayOptions`].
    #[default]
    Mesh,
    /// Filled world-space node triangles carrying UVs (drawn by the renderer
    /// with a texture).
    Textured,
    /// The UV net laid flat as a world-space z = 0 plane, textured, plus a
    /// wireframe overlay with vertex-distribution dots.
    UvMap,
}

impl ViewMode {
    /// Next mode in the cycle Mesh → Textured → UvMap → Mesh.
    pub fn next(self) -> ViewMode {
        match self {
            ViewMode::Mesh => ViewMode::Textured,
            ViewMode::Textured => ViewMode::UvMap,
            ViewMode::UvMap => ViewMode::Mesh,
        }
    }
}

/// Textured vertex: a world-space position plus its texture coordinate. In
/// [`ViewMode::UvMap`] the position lies on the z = 0 UV plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UvVertex {
    /// Position in world space (y-up); on the z = 0 UV plane in
    /// [`ViewMode::UvMap`].
    pub pos: Vec3,
    /// Texture coordinate.
    pub uv: Vec2,
}

/// Edge length of the world-space square the [0, 1]² UV space is laid out on
/// in [`ViewMode::UvMap`].
pub const UV_PLANE_SIZE: f32 = 2.0;

/// Maps a UV coordinate to its world-space position on the z = 0 UV plane
/// used by [`ViewMode::UvMap`].
pub(crate) fn uv_plane_pos(uv: Vec2) -> Vec3 {
    Vec3::new(uv.x * UV_PLANE_SIZE, uv.y * UV_PLANE_SIZE, 0.0)
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
    /// Filled world-space node triangles with per-corner UVs, in the same
    /// A/B/C order as the nodes' vertices. Emitted only in
    /// [`ViewMode::Textured`] mode (empty otherwise).
    pub tex_world: Vec<UvVertex>,
    /// The node triangles laid flat on the z = 0 UV plane, textured. Emitted
    /// only in [`ViewMode::UvMap`] mode (empty otherwise).
    pub tex_uv: Vec<UvVertex>,
    /// UV-net wireframe plus vertex-distribution dots, on the z = 0 UV plane.
    /// Emitted only in [`ViewMode::UvMap`] mode (empty otherwise).
    pub uv_lines: Vec<Vertex>,
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
    /// `fit_center` over the emitted world-space vertices of the active view
    /// mode and, in [`ViewMode::Mesh`] mode, the world label anchors, so the
    /// whole scene fits at any camera angle (`1.0` for an empty scene).
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
/// - `options` — which node attributes to display (Mesh mode only; the
///   textured and UV-map modes ignore them and never emit the attribute
///   geometry).
/// - `view` — which visualization to emit.
///
/// # Example
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::node::Node;
/// use planet_crafter_engine::scene::{ViewMode, build_scene, Attribute, DisplayOptions};
///
/// let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
/// let mut options = DisplayOptions::default();
/// options.toggle(Attribute::Labels);
/// let scene = build_scene(&[node], &options, ViewMode::Mesh);
/// assert!(scene.labels.is_empty());
/// assert!(scene.checkboxes.iter().any(|c| c.attribute == Attribute::Labels));
/// ```
///
/// # View-fit invariant
///
/// ```
/// use glam::{Vec2, Vec3};
/// use planet_crafter_engine::node::Node;
/// use planet_crafter_engine::scene::{ViewMode, build_scene, OrbitCamera};
///
/// let node = Node::new("root", [Vec3::new(0.0, 2.0 / 3.0, 0.0), Vec3::new(0.5, -1.0 / 3.0, 0.0), Vec3::new(-0.5, -1.0 / 3.0, 0.0)], Vec3::ZERO);
/// let mesh = build_scene(&[node], &Default::default(), ViewMode::Mesh);
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
pub fn build_scene(nodes: &[NodeRef], options: &DisplayOptions, view: ViewMode) -> SceneMesh {
    let mut builder = SceneBuilder {
        options: *options,
        view,
        ..Default::default()
    };
    for node in nodes {
        match view {
            ViewMode::Mesh => builder.add_node(node),
            ViewMode::Textured => builder.add_textured_triangle(&node.borrow()),
            ViewMode::UvMap => builder.add_uv_triangle(&node.borrow()),
        }
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
    /// Filled world-space triangles with UVs (Textured mode).
    tex_world: Vec<UvVertex>,
    /// UV-net triangles on the z = 0 plane (UvMap mode).
    tex_uv: Vec<UvVertex>,
    /// UV-net wireframe and vertex-distribution dots (UvMap mode).
    uv_lines: Vec<Vertex>,
    labels: Vec<WorldLabel>,
    /// Checkbox panel geometry in pixel space.
    ui_lines: Vec<Vertex>,
    ui_triangles: Vec<Vertex>,
    /// Checkbox labels, already anchored in pixel space.
    ui_labels: Vec<TextRun<'static>>,
    checkboxes: Vec<Checkbox>,
    options: DisplayOptions,
    view: ViewMode,
    bounds: Bounds,
}

impl SceneBuilder {
    /// Moves the buffers out and computes the content bounding sphere from
    /// the active view mode's world-space batches.
    fn finish(self) -> SceneMesh {
        let positions: Vec<Vec3> = match self.view {
            ViewMode::Mesh => self
                .lines
                .iter()
                .chain(&self.triangles)
                .map(|vertex| vertex.pos)
                .chain(self.labels.iter().map(|label| label.world_pos))
                .collect(),
            ViewMode::Textured => self.tex_world.iter().map(|vertex| vertex.pos).collect(),
            ViewMode::UvMap => self
                .tex_uv
                .iter()
                .map(|vertex| vertex.pos)
                .chain(self.uv_lines.iter().map(|vertex| vertex.pos))
                .collect(),
        };
        let (fit_center, fit_radius) = bounding_sphere(&self.bounds, positions.into_iter());
        SceneMesh {
            lines: self.lines,
            triangles: self.triangles,
            tex_world: self.tex_world,
            tex_uv: self.tex_uv,
            uv_lines: self.uv_lines,
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
/// distance from the center over the active view mode's world-space positions
/// (in Mesh mode: the emitted world-space vertices plus the world label
/// anchors, tracked in the bounds but not emitted as vertices), so every
/// point the scene can draw fits at any camera angle — including a
/// labels-only scene. Empty scene → (origin, 1.0).
fn bounding_sphere(bounds: &Bounds, positions: impl Iterator<Item = Vec3>) -> (Vec3, f32) {
    if !bounds.has_content {
        return (Vec3::ZERO, 1.0);
    }
    let center = (bounds.min + bounds.max) * 0.5;
    let radius = positions
        .map(|pos| (pos - center).length())
        .fold(0.0_f32, f32::max);
    (center, radius.max(camera::MIN_FIT_RADIUS))
}
