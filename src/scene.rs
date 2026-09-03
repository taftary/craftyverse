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

use glam::Vec2;

use crate::node::{Node, NodeRef};

/// Arrow length factor relative to the node's own size (same as `Svg::new`).
const ARROW_SCALE: f32 = 0.5;

/// Colors of the level palette, cycled by `level % 8` (same as the SVG one).
const LEVEL_COLORS: [[f32; 3]; 8] = [
    hex_rgb("#1f77b4"),
    hex_rgb("#ff7f0e"),
    hex_rgb("#2ca02c"),
    hex_rgb("#d62728"),
    hex_rgb("#9467bd"),
    hex_rgb("#8c564b"),
    hex_rgb("#e377c2"),
    hex_rgb("#7f7f7f"),
];

const DIRECTION_COLORS: [[f32; 3]; 3] = [
    hex_rgb("#d32f2f"),
    hex_rgb("#388e3c"),
    hex_rgb("#1976d2"),
];
const NODE_DIRECTION_COLOR: [f32; 3] = hex_rgb("#8e24aa");
const ORIGIN_COLOR: [f32; 3] = hex_rgb("#424242");
const LABEL_COLOR: [f32; 3] = hex_rgb("#212121");
const CORNER_LABEL_COLOR: [f32; 3] = hex_rgb("#757575");
const UI_COLOR: [f32; 3] = hex_rgb("#424242");

const LABEL_SIZE_PX: f32 = 12.0;
const CORNER_LABEL_SIZE_PX: f32 = 10.0;
const DOT_SEGMENTS: usize = 16;

/// Checkbox panel metrics (pixels, y-down); top-left anchored.
const PANEL_PAD: f32 = 8.0;
const CHECKBOX_SIZE: f32 = 12.0;
const CHECKBOX_ROW_HEIGHT: f32 = 18.0;
const CHECKBOX_LABEL_SIZE: f32 = 11.0;
const CHECKBOX_LABEL_GAP: f32 = 6.0;
const CHECKBOX_SUB_INDENT: f32 = 16.0;

/// One hex digit → value; invalid digits map to 0.
const fn hex_channel(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => 0,
    }
}

/// "#rrggbb" → RGB floats in [0, 1].
const fn hex_rgb(color: &str) -> [f32; 3] {
    let bytes = color.as_bytes();
    [
        (hex_channel(bytes[1]) * 16 + hex_channel(bytes[2])) as f32 / 255.0,
        (hex_channel(bytes[3]) * 16 + hex_channel(bytes[4])) as f32 / 255.0,
        (hex_channel(bytes[5]) * 16 + hex_channel(bytes[6])) as f32 / 255.0,
    ]
}

/// Level palette, cycled by `level % 8`.
pub fn level_color(level: u32) -> [f32; 3] {
    LEVEL_COLORS[(level % 8) as usize]
}

/// One toggleable node attribute of the visualization; each variant has a
/// checkbox in the display-options panel. `ChildLinks`, `OpenPorts` and
/// `Directions` are group masters gating their per-port sub-switches
/// (`ChildLink(i)`, `OpenPort(i)`, `Direction(i)`; 0 = I, 1 = J, 2 = K): an
/// element is drawn only when both the master and its per-port switch are on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Attribute {
    ChildLinks,
    ChildLink(usize),
    OpenPorts,
    OpenPort(usize),
    Outline,
    Directions,
    Direction(usize),
    DirectionOfNode,
    Origin,
    CenterDot,
    Labels,
}

impl Attribute {
    /// Port index when the attribute is a per-port sub-switch of a group.
    fn port(self) -> Option<usize> {
        match self {
            Attribute::ChildLink(i) | Attribute::OpenPort(i) | Attribute::Direction(i) => Some(i),
            _ => None,
        }
    }
}

/// Which node attributes the visualization displays. Toggled at runtime
/// through the checkbox panel; everything is on by default.
#[derive(Clone, Copy, Debug)]
pub struct DisplayOptions {
    pub child_links: bool,
    pub child_links_ijk: [bool; 3],
    pub open_ports: bool,
    pub open_ports_ijk: [bool; 3],
    pub outline: bool,
    pub directions: bool,
    pub directions_ijk: [bool; 3],
    pub direction_of_node: bool,
    pub origin: bool,
    pub center_dot: bool,
    pub labels: bool,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            child_links: true,
            child_links_ijk: [true; 3],
            open_ports: true,
            open_ports_ijk: [true; 3],
            outline: true,
            directions: true,
            directions_ijk: [true; 3],
            direction_of_node: true,
            origin: true,
            center_dot: true,
            labels: true,
        }
    }
}

impl DisplayOptions {
    /// Current display state of one attribute.
    pub fn value(&self, attribute: Attribute) -> bool {
        match attribute {
            Attribute::ChildLinks => self.child_links,
            Attribute::ChildLink(i) => self.child_links_ijk[i],
            Attribute::OpenPorts => self.open_ports,
            Attribute::OpenPort(i) => self.open_ports_ijk[i],
            Attribute::Outline => self.outline,
            Attribute::Directions => self.directions,
            Attribute::Direction(i) => self.directions_ijk[i],
            Attribute::DirectionOfNode => self.direction_of_node,
            Attribute::Origin => self.origin,
            Attribute::CenterDot => self.center_dot,
            Attribute::Labels => self.labels,
        }
    }

    /// Flips one attribute (checkbox click).
    pub fn toggle(&mut self, attribute: Attribute) {
        match attribute {
            Attribute::ChildLinks => self.child_links = !self.child_links,
            Attribute::ChildLink(i) => self.child_links_ijk[i] = !self.child_links_ijk[i],
            Attribute::OpenPorts => self.open_ports = !self.open_ports,
            Attribute::OpenPort(i) => self.open_ports_ijk[i] = !self.open_ports_ijk[i],
            Attribute::Outline => self.outline = !self.outline,
            Attribute::Directions => self.directions = !self.directions,
            Attribute::Direction(i) => self.directions_ijk[i] = !self.directions_ijk[i],
            Attribute::DirectionOfNode => self.direction_of_node = !self.direction_of_node,
            Attribute::Origin => self.origin = !self.origin,
            Attribute::CenterDot => self.center_dot = !self.center_dot,
            Attribute::Labels => self.labels = !self.labels,
        }
    }
}

/// Checkboxes of the display-options panel, in display order. Per-port
/// sub-switches sit right under their group master.
const ATTRIBUTES: [(Attribute, &str); 17] = [
    (Attribute::ChildLinks, "child links"),
    (Attribute::ChildLink(0), "link I"),
    (Attribute::ChildLink(1), "link J"),
    (Attribute::ChildLink(2), "link K"),
    (Attribute::OpenPorts, "open ports"),
    (Attribute::OpenPort(0), "port I"),
    (Attribute::OpenPort(1), "port J"),
    (Attribute::OpenPort(2), "port K"),
    (Attribute::Outline, "outline"),
    (Attribute::Directions, "directions ijk"),
    (Attribute::Direction(0), "dir I"),
    (Attribute::Direction(1), "dir J"),
    (Attribute::Direction(2), "dir K"),
    (Attribute::DirectionOfNode, "direction of node"),
    (Attribute::Origin, "origin arrow"),
    (Attribute::CenterDot, "center dot"),
    (Attribute::Labels, "labels"),
];

/// Clickable area of one checkbox (pixel space, y-down). The viewer
/// hit-tests mouse clicks against these.
#[derive(Clone, Copy, Debug)]
pub struct Checkbox {
    pub attribute: Attribute,
    /// Top-left and bottom-right corners of the clickable rectangle (checkbox
    /// box plus label), in pixels.
    pub min: Vec2,
    pub max: Vec2,
}

impl Checkbox {
    /// Whether `point` (pixels, y-down) is inside the clickable rectangle.
    pub fn contains(&self, point: Vec2) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }
}

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
    min: Vec2,
    max: Vec2,
    has_content: bool,
}

impl SceneBuilder {
    /// Node geometry is 2D and y-up; the view space is y-down. The flip is
    /// applied per point (same mapping the SVG viewer used) and the point
    /// contributes to the content bounds used for the view fit.
    fn map(&mut self, point: Vec2) -> Vec2 {
        let mapped = Vec2::new(point.x, -point.y);
        if self.has_content {
            self.min = self.min.min(mapped);
            self.max = self.max.max(mapped);
        } else {
            self.min = mapped;
            self.max = mapped;
            self.has_content = true;
        }
        mapped
    }

    /// Appends one line segment; endpoints are already mapped.
    fn line(&mut self, from: Vec2, to: Vec2, color: [f32; 3]) {
        self.lines.push(Vertex { pos: from, color });
        self.lines.push(Vertex { pos: to, color });
    }

    /// Appends a dashed line segment; endpoints are already mapped. The gap is
    /// half the dash, like the SVG `stroke-dasharray="4 2"`.
    fn dashed_line(&mut self, from: Vec2, to: Vec2, dash: f32, color: [f32; 3]) {
        let gap = dash / 2.0;
        let dir = (to - from).normalize();
        let total = (to - from).length();
        let mut d = 0.0;
        while d < total {
            let seg_end = (d + dash).min(total);
            self.line(from + dir * d, from + dir * seg_end, color);
            d += dash + gap;
        }
    }

    /// Appends one triangle; corners are already mapped.
    fn triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: [f32; 3]) {
        for pos in [a, b, c] {
            self.triangles.push(Vertex { pos, color });
        }
    }

    /// Appends one pixel-space UI line segment (checkbox panel).
    fn ui_line(&mut self, from: Vec2, to: Vec2, color: [f32; 3]) {
        self.ui_lines.push(Vertex { pos: from, color });
        self.ui_lines.push(Vertex { pos: to, color });
    }

    /// Appends one pixel-space UI triangle (checkbox panel).
    fn ui_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: [f32; 3]) {
        for pos in [a, b, c] {
            self.ui_triangles.push(Vertex { pos, color });
        }
    }

    /// Appends the visual representation of one node, limited to the
    /// attributes enabled in `options`. Never alters the node.
    fn add_node(&mut self, node: &Node) {
        let arrow_len = arrow_length(node);
        if self.options.child_links {
            self.add_child_links(node, arrow_len);
        }
        if self.options.open_ports {
            self.add_open_port_markers(node, arrow_len);
        }
        if self.options.outline {
            self.add_triangle_outline(node);
        }
        if self.options.directions {
            self.add_direction_arrows(node, arrow_len);
        }
        if self.options.direction_of_node {
            self.add_direction_of_node_arrow(node, arrow_len);
        }
        if self.options.origin {
            self.add_origin_arrow(node, arrow_len);
        }
        if self.options.center_dot {
            self.add_center_dot(node, arrow_len);
        }
        if self.options.labels {
            self.add_labels(node);
        }
    }

    /// Medium dashed line from the node's center to each non-empty child's
    /// center, colored with the direction color of the child slot (I/J/K).
    /// Each port's link is gated by its own switch on top of the group master.
    fn add_child_links(&mut self, node: &Node, arrow_len: f32) {
        let from = self.map(node.center);
        for (index, child) in node.children.iter().enumerate() {
            let Some(child) = child else { continue };
            if !self.options.child_links_ijk[index] {
                continue;
            }
            let to = self.map(child.borrow().center);
            self.dashed_line(from, to, arrow_len / 6.0, DIRECTION_COLORS[index]);
        }
    }

    /// Bold filled disc just outside the edge of every open (null) port,
    /// colored with the port's direction color. Open ports get a solid
    /// marker — not a thin line — so they stand out instead of being the
    /// mere absence of a child link. Port I/J/K faces edge AB/BC/CA.
    fn add_open_port_markers(&mut self, node: &Node, arrow_len: f32) {
        let [a, b, c] = node.points;
        let edge_midpoints = [(a + b) / 2.0, (b + c) / 2.0, (c + a) / 2.0];
        let radius = arrow_len * 0.18;
        for (index, child) in node.children.iter().enumerate() {
            if child.is_some() || !self.options.open_ports_ijk[index] {
                continue;
            }
            let outward = node.directions[index];
            let center = self.map(edge_midpoints[index] + outward * radius * 1.4);
            // Extend the content bounds to the disc rim so the view fit never
            // clips a marker.
            self.map(edge_midpoints[index] + outward * radius * 2.4);
            self.disc(center, radius, DIRECTION_COLORS[index]);
        }
    }

    /// Triangle outline through the corner points A → B → C → A, colored by level.
    fn add_triangle_outline(&mut self, node: &Node) {
        let color = level_color(node.level);
        let [a, b, c] = node.points.map(|point| self.map(point));
        self.line(a, b, color);
        self.line(b, c, color);
        self.line(c, a, color);
    }

    /// One arrow per direction vector, starting at the node's center. Each
    /// port's arrow is gated by its own switch on top of the group master.
    fn add_direction_arrows(&mut self, node: &Node, arrow_len: f32) {
        let start = self.map(node.center);
        for (index, direction) in node.directions.iter().enumerate() {
            if !self.options.directions_ijk[index] {
                continue;
            }
            let end = self.map(node.center + direction.normalize() * arrow_len);
            let color = DIRECTION_COLORS[index];
            self.line(start, end, color);
            self.add_arrowhead(end, (end - start).normalize(), arrow_len * 0.25, color);
        }
    }

    /// One arrow along `direction_of_node` (base BC → apex A), starting at
    /// the node's center.
    fn add_direction_of_node_arrow(&mut self, node: &Node, arrow_len: f32) {
        let start = self.map(node.center);
        let end = self.map(node.center + node.direction_of_node * arrow_len);
        self.line(start, end, NODE_DIRECTION_COLOR);
        self.add_arrowhead(
            end,
            (end - start).normalize(),
            arrow_len * 0.25,
            NODE_DIRECTION_COLOR,
        );
    }

    /// Dashed arrow from the node's center along the normalized
    /// `direction_to_origin`. Skipped when the vector has zero length.
    fn add_origin_arrow(&mut self, node: &Node, arrow_len: f32) {
        if node.direction_to_origin.length() == 0.0 {
            return;
        }
        let start = self.map(node.center);
        let end = self.map(node.center + node.direction_to_origin.normalize() * arrow_len);
        let dir = (end - start).normalize();
        self.dashed_line(start, end, arrow_len / 6.0, ORIGIN_COLOR);
        self.add_arrowhead(end, dir, arrow_len * 0.25, ORIGIN_COLOR);
    }

    /// Filled dot on top of all lines, colored by level.
    fn add_center_dot(&mut self, node: &Node, arrow_len: f32) {
        let center = self.map(node.center);
        self.disc(center, arrow_len * 0.08, level_color(node.level));
    }

    /// Filled disc of `DOT_SEGMENTS` triangles; `center` is already mapped.
    fn disc(&mut self, center: Vec2, radius: f32, color: [f32; 3]) {
        for i in 0..DOT_SEGMENTS {
            let a0 = i as f32 / DOT_SEGMENTS as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / DOT_SEGMENTS as f32 * std::f32::consts::TAU;
            self.triangle(
                center,
                center + radius * Vec2::new(a0.cos(), a0.sin()),
                center + radius * Vec2::new(a1.cos(), a1.sin()),
                color,
            );
        }
    }

    /// Name/level label near the center plus corner labels A, B, C.
    fn add_labels(&mut self, node: &Node) {
        let center = self.map(node.center);
        self.labels.push(LabelRequest {
            text: format!("{} L{}", node.name, node.level),
            world_pos: center,
            offset_px: Vec2::new(6.0, -16.0),
            size_px: LABEL_SIZE_PX,
            color: LABEL_COLOR,
            centered: false,
        });
        for (point, corner_label) in node.points.iter().zip(['A', 'B', 'C']) {
            let corner = self.map(*point);
            let outward = (corner - center).normalize();
            self.labels.push(LabelRequest {
                text: corner_label.to_string(),
                world_pos: corner,
                offset_px: outward * 8.0,
                size_px: CORNER_LABEL_SIZE_PX,
                color: CORNER_LABEL_COLOR,
                centered: true,
            });
        }
    }

    /// Small filled triangle at `tip`, pointing along `dir` (mapped space).
    fn add_arrowhead(&mut self, tip: Vec2, dir: Vec2, size: f32, color: [f32; 3]) {
        let back = dir * size;
        let side = Vec2::new(-dir.y, dir.x) * size * 0.45;
        self.triangle(tip, tip - back + side, tip - back - side, color);
    }

    /// Checkbox panel (top-left, pixel space): one row per attribute — a box,
    /// filled when the attribute is on, plus its label. Per-port sub-switches
    /// are indented under their group master and labeled with the port's
    /// direction color. Always generated so the options stay reachable when
    /// every attribute is off.
    fn add_checkbox_panel(&mut self) {
        for (row, (attribute, label)) in ATTRIBUTES.iter().enumerate() {
            let indent = attribute.port().map_or(0.0, |_| CHECKBOX_SUB_INDENT);
            let label_color = attribute.port().map_or(LABEL_COLOR, |i| DIRECTION_COLORS[i]);
            let min = Vec2::new(PANEL_PAD + indent, PANEL_PAD + row as f32 * CHECKBOX_ROW_HEIGHT);
            let max = min + Vec2::splat(CHECKBOX_SIZE);
            self.ui_line(min, Vec2::new(max.x, min.y), UI_COLOR);
            self.ui_line(Vec2::new(max.x, min.y), max, UI_COLOR);
            self.ui_line(max, Vec2::new(min.x, max.y), UI_COLOR);
            self.ui_line(Vec2::new(min.x, max.y), min, UI_COLOR);
            if self.options.value(*attribute) {
                let fill_min = min + Vec2::splat(3.0);
                let fill_max = max - Vec2::splat(3.0);
                self.ui_triangle(fill_min, Vec2::new(fill_max.x, fill_min.y), fill_max, UI_COLOR);
                self.ui_triangle(fill_min, fill_max, Vec2::new(fill_min.x, fill_max.y), UI_COLOR);
            }
            let anchor = Vec2::new(
                max.x + CHECKBOX_LABEL_GAP,
                min.y + (CHECKBOX_SIZE - CHECKBOX_LABEL_SIZE) / 2.0,
            );
            self.ui_labels.push(TextRun {
                text: label.to_string(),
                anchor,
                size: CHECKBOX_LABEL_SIZE,
                color: label_color,
                centered: false,
            });
            // The clickable rectangle covers the box and the label (width
            // estimated from the monospace advance).
            let label_width = label.len() as f32 * CHECKBOX_LABEL_SIZE * 0.6;
            self.checkboxes.push(Checkbox {
                attribute: *attribute,
                min,
                max: Vec2::new(anchor.x + label_width, max.y),
            });
        }
    }

    /// Computes the view fit (content bounds + 5% margin, aspect preserved)
    /// and resolves the label anchors to pixel positions.
    fn finish(self, viewport: Vec2) -> SceneMesh {
        let viewport = viewport.max(Vec2::ONE);
        let (origin, content_size) = if self.has_content {
            let extent = (self.max - self.min).max(Vec2::ONE);
            (self.min - extent * 0.05, extent * 1.1)
        } else {
            (Vec2::ZERO, viewport)
        };
        let scale_px = (viewport.x / content_size.x).min(viewport.y / content_size.y);
        let pad = (viewport - content_size * scale_px) * 0.5;
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
            world_to_clip: ClipTransform {
                scale: Vec2::new(
                    scale_px * 2.0 / viewport.x,
                    -scale_px * 2.0 / viewport.y,
                ),
                offset: Vec2::new(
                    (pad.x - origin.x * scale_px) * 2.0 / viewport.x - 1.0,
                    1.0 - (pad.y - origin.y * scale_px) * 2.0 / viewport.y,
                ),
            },
            pixel_to_clip: ClipTransform {
                scale: Vec2::new(2.0 / viewport.x, -2.0 / viewport.y),
                offset: Vec2::new(-1.0, 1.0),
            },
        }
    }
}

/// Arrows scale with the node's own triangle so they stay readable after
/// repeated `split()` calls (same rule as the SVG viewer).
fn arrow_length(node: &Node) -> f32 {
    let min_corner_distance = node
        .points
        .iter()
        .map(|point| (*point - node.center).length())
        .fold(f32::INFINITY, f32::min);
    ARROW_SCALE * min_corner_distance
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::Labeling;

    /// Equilateral test node (base 300, apex up, height = base * √3 / 2).
    fn test_node() -> NodeRef {
        Node::new(
            Vec2::Y,
            Vec2::ZERO,
            Vec2::new(0.0, 1000.0),
            300.0,
            300.0 * 3.0_f32.sqrt() / 2.0,
            "root",
            Labeling::Normal,
        )
    }

    #[test]
    fn single_node_emits_all_element_kinds() {
        let node = test_node();
        let mesh = build_scene(&[node], Vec2::new(800.0, 800.0), &DisplayOptions::default());

        // 3 outline + 4 arrow shafts + at least one dashed origin segment.
        assert!(mesh.lines.len() >= (3 + 4 + 1) * 2);
        assert_eq!(mesh.lines.len() % 2, 0);
        // 5 arrowheads + 16 dot segments + 3 open-port discs = 69 triangles.
        assert_eq!(mesh.triangles.len(), 69 * 3);
        // 1 name label + 3 corner labels + checkbox labels.
        assert_eq!(mesh.texts.len(), 4 + ATTRIBUTES.len());
        assert_eq!(mesh.texts[0].text, "root L0");
        // One checkbox row per attribute, all ticked by default.
        assert_eq!(mesh.checkboxes.len(), ATTRIBUTES.len());
        assert_eq!(mesh.ui_lines.len(), ATTRIBUTES.len() * 4 * 2);
        assert_eq!(mesh.ui_triangles.len(), ATTRIBUTES.len() * 2 * 3);
    }

    #[test]
    fn origin_arrow_is_skipped_when_node_is_at_origin() {
        let node = Node::new(Vec2::Y, Vec2::ZERO, Vec2::ZERO, 300.0, 200.0, "at_origin", Labeling::Normal);
        let mesh = build_scene(&[node], Vec2::new(800.0, 800.0), &DisplayOptions::default());

        // Only outline + arrow shafts remain.
        assert_eq!(mesh.lines.len(), (3 + 4) * 2);
        // 4 arrowheads + dot segments + 3 open-port discs.
        assert_eq!(mesh.triangles.len(), (4 + DOT_SEGMENTS + 3 * DOT_SEGMENTS) * 3);
    }

    #[test]
    fn split_scene_links_children_and_labels_all_nodes() {
        let node = test_node();
        let center = node.borrow().split();
        let mut nodes = vec![center.clone()];
        for corner in center.borrow().children.iter().flatten() {
            nodes.push(corner.clone());
        }
        let mesh = build_scene(&nodes, Vec2::new(800.0, 800.0), &DisplayOptions::default());

        // 4 nodes × (3 outline + 4 shafts) + dashed origin segments +
        // 6 child links (3 center→corner, 3 corner→center).
        assert!(mesh.lines.len() >= (4 * 7 + 6) * 2);
        // 4 nodes × (5 arrowheads + 16 dot segments) + 6 open-port discs (two
        // per corner node; the center node is fully linked).
        assert_eq!(mesh.triangles.len(), (4 * 21 + 6 * DOT_SEGMENTS) * 3);
        // 4 nodes × (1 name label + 3 corner labels) + checkbox labels.
        assert_eq!(mesh.texts.len(), 16 + ATTRIBUTES.len());
        assert_eq!(mesh.texts[0].text, "root.C L1");
    }

    #[test]
    fn child_links_are_dashed_and_colored_by_direction() {
        let node = test_node();
        let center = node.borrow().split();
        // Child links only: every emitted line is part of a dashed link.
        let options = DisplayOptions {
            child_links: true,
            child_links_ijk: [true; 3],
            open_ports: false,
            open_ports_ijk: [false; 3],
            outline: false,
            directions: false,
            directions_ijk: [false; 3],
            direction_of_node: false,
            origin: false,
            center_dot: false,
            labels: false,
        };
        let mesh = build_scene(&[center], Vec2::new(800.0, 800.0), &options);

        // Dashed: the three links are split into more than one segment each.
        assert!(mesh.lines.len() > 3 * 2);
        assert_eq!(mesh.lines.len() % 2, 0);
        // Every segment carries one of the I/J/K direction colors, and all
        // three are used.
        assert!(mesh
            .lines
            .iter()
            .all(|vertex| DIRECTION_COLORS.contains(&vertex.color)));
        for color in DIRECTION_COLORS {
            assert!(mesh.lines.iter().any(|vertex| vertex.color == color));
        }
    }

    #[test]
    fn open_ports_emit_bold_disc_per_null_port() {
        let node = test_node();
        // Open ports only: every emitted triangle is part of a port marker.
        let options = DisplayOptions {
            child_links: false,
            child_links_ijk: [false; 3],
            open_ports: true,
            open_ports_ijk: [true; 3],
            outline: false,
            directions: false,
            directions_ijk: [false; 3],
            direction_of_node: false,
            origin: false,
            center_dot: false,
            labels: false,
        };
        let mesh = build_scene(&[node.clone()], Vec2::new(800.0, 800.0), &options);

        // One disc per open port (all three are null), no lines.
        assert_eq!(mesh.triangles.len(), 3 * DOT_SEGMENTS * 3);
        assert!(mesh.lines.is_empty());
        // Every disc carries the direction color of its port; all three used.
        for color in DIRECTION_COLORS {
            assert!(mesh.triangles.iter().any(|vertex| vertex.color == color));
        }

        // The split center node is fully linked: no markers.
        let center = node.borrow().split();
        let mesh = build_scene(&[center.clone()], Vec2::new(800.0, 800.0), &options);
        assert!(mesh.triangles.is_empty());

        // Each corner node has two open ports: two discs.
        let node_i = center.borrow().children[0].clone().unwrap();
        let mesh = build_scene(&[node_i], Vec2::new(800.0, 800.0), &options);
        assert_eq!(mesh.triangles.len(), 2 * DOT_SEGMENTS * 3);
    }

    #[test]
    fn per_port_switches_gate_each_group() {
        let node = test_node();
        let mut options = DisplayOptions {
            child_links: false,
            child_links_ijk: [false; 3],
            open_ports: true,
            open_ports_ijk: [true; 3],
            outline: false,
            directions: false,
            directions_ijk: [false; 3],
            direction_of_node: false,
            origin: false,
            center_dot: false,
            labels: false,
        };

        // One port off: two discs remain, none in the disabled port's color.
        options.toggle(Attribute::OpenPort(1));
        let mesh = build_scene(&[node.clone()], Vec2::new(800.0, 800.0), &options);
        assert_eq!(mesh.triangles.len(), 2 * DOT_SEGMENTS * 3);
        assert!(mesh
            .triangles
            .iter()
            .all(|vertex| vertex.color != DIRECTION_COLORS[1]));

        // The master gates the whole group without touching the per-port
        // switches: no markers while off, the same selection returns when on.
        options.toggle(Attribute::OpenPorts);
        let mesh = build_scene(&[node.clone()], Vec2::new(800.0, 800.0), &options);
        assert!(mesh.triangles.is_empty());
        options.toggle(Attribute::OpenPorts);
        let mesh = build_scene(&[node.clone()], Vec2::new(800.0, 800.0), &options);
        assert_eq!(mesh.triangles.len(), 2 * DOT_SEGMENTS * 3);

        // Directions: only port I enabled — one shaft and one arrowhead, red.
        options.open_ports = false;
        options.directions = true;
        options.directions_ijk = [true, false, false];
        let mesh = build_scene(&[node.clone()], Vec2::new(800.0, 800.0), &options);
        assert_eq!(mesh.lines.len(), 2);
        assert_eq!(mesh.triangles.len(), 3);
        assert!(mesh
            .lines
            .iter()
            .chain(&mesh.triangles)
            .all(|vertex| vertex.color == DIRECTION_COLORS[0]));

        // Child links: the split center has three links; keep only I and K.
        let center = node.borrow().split();
        options.directions = false;
        options.child_links = true;
        options.child_links_ijk = [true, false, true];
        let mesh = build_scene(&[center], Vec2::new(800.0, 800.0), &options);
        assert!(!mesh.lines.is_empty());
        assert!(mesh
            .lines
            .iter()
            .all(|vertex| vertex.color != DIRECTION_COLORS[1]));
        for color in [DIRECTION_COLORS[0], DIRECTION_COLORS[2]] {
            assert!(mesh.lines.iter().any(|vertex| vertex.color == color));
        }
    }

    #[test]
    fn disabled_attributes_emit_no_geometry() {
        let node = test_node();
        let options = DisplayOptions {
            child_links: false,
            child_links_ijk: [false; 3],
            open_ports: false,
            open_ports_ijk: [false; 3],
            outline: false,
            directions: false,
            directions_ijk: [false; 3],
            direction_of_node: false,
            origin: false,
            center_dot: false,
            labels: false,
        };
        let mesh = build_scene(&[node], Vec2::new(800.0, 800.0), &options);

        // Only the checkbox panel remains, so options can be turned back on.
        assert!(mesh.lines.is_empty() && mesh.triangles.is_empty());
        assert_eq!(mesh.texts.len(), ATTRIBUTES.len());
        assert_eq!(mesh.checkboxes.len(), ATTRIBUTES.len());
        // Unticked boxes: outlines but no fills.
        assert_eq!(mesh.ui_lines.len(), ATTRIBUTES.len() * 4 * 2);
        assert!(mesh.ui_triangles.is_empty());
    }

    #[test]
    fn toggle_gates_each_attribute() {
        let node = test_node();
        let mut options = DisplayOptions::default();
        options.toggle(Attribute::Directions);
        options.toggle(Attribute::DirectionOfNode);
        assert!(!options.value(Attribute::Directions));
        assert!(!options.value(Attribute::DirectionOfNode));
        let mesh = build_scene(&[node], Vec2::new(800.0, 800.0), &options);

        // No direction arrowheads left; only the origin arrowhead + dot + the
        // three open-port discs.
        assert_eq!(mesh.triangles.len(), (1 + DOT_SEGMENTS + 3 * DOT_SEGMENTS) * 3);
        // Two of the checkboxes are unticked.
        assert_eq!(mesh.ui_triangles.len(), (ATTRIBUTES.len() - 2) * 2 * 3);
    }

    #[test]
    fn checkbox_contains_hit_tests_rectangle() {
        let node = test_node();
        let mesh = build_scene(&[node], Vec2::new(800.0, 800.0), &DisplayOptions::default());

        let checkbox = mesh.checkboxes[0];
        let center = (checkbox.min + checkbox.max) / 2.0;
        assert!(checkbox.contains(center));
        assert!(!checkbox.contains(checkbox.min - Vec2::ONE));
        assert!(!checkbox.contains(checkbox.max + Vec2::ONE));
    }

    #[test]
    fn level_color_cycles_through_palette() {
        assert_eq!(level_color(0), LEVEL_COLORS[0]);
        assert_eq!(level_color(7), LEVEL_COLORS[7]);
        assert_eq!(level_color(8), LEVEL_COLORS[0]);
    }

    #[test]
    fn clip_transform_maps_all_geometry_inside_clip_space() {
        let node = test_node();
        let mesh = build_scene(&[node], Vec2::new(800.0, 800.0), &DisplayOptions::default());

        for vertex in mesh.lines.iter().chain(&mesh.triangles) {
            let clip = vertex.pos * mesh.world_to_clip.scale + mesh.world_to_clip.offset;
            assert!(clip.x.abs() <= 1.0, "clip.x {} out of range", clip.x);
            assert!(clip.y.abs() <= 1.0, "clip.y {} out of range", clip.y);
        }
    }

    #[test]
    fn empty_scene_produces_identity_like_transform() {
        let mesh = build_scene(&[], Vec2::new(800.0, 800.0), &DisplayOptions::default());
        // No node geometry, but the checkbox panel is always generated.
        assert!(mesh.lines.is_empty() && mesh.triangles.is_empty());
        assert_eq!(mesh.texts.len(), ATTRIBUTES.len());
        assert_eq!(mesh.checkboxes.len(), ATTRIBUTES.len());
        assert_eq!(mesh.world_to_clip.scale, Vec2::new(2.0 / 800.0, -2.0 / 800.0));
    }

    #[test]
    fn hex_rgb_parses_channels() {
        assert_eq!(hex_rgb("#000000"), [0.0; 3]);
        assert_eq!(hex_rgb("#ffffff"), [1.0; 3]);
        assert_eq!(hex_rgb("#ff0000"), [1.0, 0.0, 0.0]);
    }
}

