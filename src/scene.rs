//! CPU-side scene generation for the Vulkan debug viewer.
//!
//! Generates the same visualization the SVG viewer produced — triangle
//! outlines, direction arrows, dashed origin arrow, child links, center dots
//! and text labels — but as plain vertex data: colored line and triangle
//! lists in a y-down world space, plus pixel-space text runs. The renderer
//! only uploads the buffers and applies the two clip transforms.

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

const CHILD_LINK_COLOR: [f32; 3] = hex_rgb("#9e9e9e");
const DIRECTION_COLORS: [[f32; 3]; 3] = [
    hex_rgb("#d32f2f"),
    hex_rgb("#388e3c"),
    hex_rgb("#1976d2"),
];
const ORIGIN_COLOR: [f32; 3] = hex_rgb("#424242");
const LABEL_COLOR: [f32; 3] = hex_rgb("#212121");
const CORNER_LABEL_COLOR: [f32; 3] = hex_rgb("#757575");

const LABEL_SIZE_PX: f32 = 12.0;
const CORNER_LABEL_SIZE_PX: f32 = 10.0;
const DOT_SEGMENTS: usize = 16;

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
    /// Text labels, anchored in pixel space.
    pub texts: Vec<TextRun>,
    /// Maps `lines`/`triangles` positions to clip space.
    pub world_to_clip: ClipTransform,
    /// Maps text pixel positions to clip space.
    pub pixel_to_clip: ClipTransform,
}

/// Builds the visualization of `nodes` fitted into `viewport` pixels.
pub fn build_scene(nodes: &[NodeRef], viewport: Vec2) -> SceneMesh {
    let mut builder = SceneBuilder::default();
    for node in nodes {
        builder.add_node(&node.borrow());
    }
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

    /// Appends one triangle; corners are already mapped.
    fn triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: [f32; 3]) {
        for pos in [a, b, c] {
            self.triangles.push(Vertex { pos, color });
        }
    }

    /// Appends the visual representation of one node. Never alters the node.
    fn add_node(&mut self, node: &Node) {
        self.add_child_links(node);
        self.add_triangle_outline(node);
        let arrow_len = arrow_length(node);
        self.add_direction_arrows(node, arrow_len);
        self.add_origin_arrow(node, arrow_len);
        self.add_center_dot(node, arrow_len);
        self.add_labels(node);
    }

    /// Thin line from the node's center to each non-empty child's center.
    fn add_child_links(&mut self, node: &Node) {
        let from = self.map(node.center);
        for child in node.children.iter().flatten() {
            let to = self.map(child.borrow().center);
            self.line(from, to, CHILD_LINK_COLOR);
        }
    }

    /// Triangle outline through the UV corners A → B → C → A, colored by level.
    fn add_triangle_outline(&mut self, node: &Node) {
        let color = level_color(node.level);
        let [a, b, c] = node.uvs.map(|uv| self.map(uv));
        self.line(a, b, color);
        self.line(b, c, color);
        self.line(c, a, color);
    }

    /// One arrow per direction vector, starting at the node's center.
    fn add_direction_arrows(&mut self, node: &Node, arrow_len: f32) {
        let start = self.map(node.center);
        for (index, direction) in node.directions.iter().enumerate() {
            let end = self.map(node.center + direction.normalize() * arrow_len);
            let color = DIRECTION_COLORS[index];
            self.line(start, end, color);
            self.add_arrowhead(end, (end - start).normalize(), arrow_len * 0.25, color);
        }
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
        // Dashed shaft, like the SVG `stroke-dasharray="4 2"`.
        let dash = arrow_len / 6.0;
        let gap = dash / 2.0;
        let total = (end - start).length();
        let mut d = 0.0;
        while d < total {
            let seg_end = (d + dash).min(total);
            self.line(start + dir * d, start + dir * seg_end, ORIGIN_COLOR);
            d += dash + gap;
        }
        self.add_arrowhead(end, dir, arrow_len * 0.25, ORIGIN_COLOR);
    }

    /// Filled dot on top of all lines, colored by level.
    fn add_center_dot(&mut self, node: &Node, arrow_len: f32) {
        let center = self.map(node.center);
        let radius = arrow_len * 0.08;
        let color = level_color(node.level);
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

    /// Name/level/direction-set label near the center plus corner labels A, B, C.
    fn add_labels(&mut self, node: &Node) {
        let center = self.map(node.center);
        self.labels.push(LabelRequest {
            text: format!(
                "{} L{} {}",
                node.name,
                node.level,
                node.direction_of_node.label()
            ),
            world_pos: center,
            offset_px: Vec2::new(6.0, -16.0),
            size_px: LABEL_SIZE_PX,
            color: LABEL_COLOR,
            centered: false,
        });
        for (uv, corner_label) in node.uvs.iter().zip(['A', 'B', 'C']) {
            let corner = self.map(*uv);
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

        let texts = self
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

        SceneMesh {
            lines: self.lines,
            triangles: self.triangles,
            texts,
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
        .uvs
        .iter()
        .map(|uv| (*uv - node.center).length())
        .fold(f32::INFINITY, f32::min);
    ARROW_SCALE * min_corner_distance
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::DirectionSet;

    fn test_node() -> NodeRef {
        Node::new(
            DirectionSet::Normal,
            Vec2::ZERO,
            Vec2::new(0.0, 1000.0),
            300.0,
            "root",
        )
    }

    #[test]
    fn single_node_emits_all_element_kinds() {
        let node = test_node();
        let mesh = build_scene(&[node], Vec2::new(800.0, 800.0));

        // 3 outline + 3 arrow shafts + at least one dashed origin segment.
        assert!(mesh.lines.len() >= (3 + 3 + 1) * 2);
        assert_eq!(mesh.lines.len() % 2, 0);
        // 4 arrowheads + 16 dot segments = 20 triangles.
        assert_eq!(mesh.triangles.len(), 20 * 3);
        // 1 name label + 3 corner labels.
        assert_eq!(mesh.texts.len(), 4);
        assert_eq!(mesh.texts[0].text, "root L0 N");
    }

    #[test]
    fn origin_arrow_is_skipped_when_node_is_at_origin() {
        let node = Node::new(
            DirectionSet::Normal,
            Vec2::ZERO,
            Vec2::ZERO,
            300.0,
            "at_origin",
        );
        let mesh = build_scene(&[node], Vec2::new(800.0, 800.0));

        // Only outline + direction arrow shafts remain.
        assert_eq!(mesh.lines.len(), (3 + 3) * 2);
        // 3 arrowheads + dot segments.
        assert_eq!(mesh.triangles.len(), (3 + DOT_SEGMENTS) * 3);
    }

    #[test]
    fn split_scene_links_children_and_labels_all_nodes() {
        let node = test_node();
        let center = node.borrow().split();
        let mut nodes = vec![center.clone()];
        for corner in center.borrow().children.iter().flatten() {
            nodes.push(corner.clone());
        }
        let mesh = build_scene(&nodes, Vec2::new(800.0, 800.0));

        // 4 nodes × (3 outline + 3 shafts) + dashed origin segments +
        // 6 child links (3 center→corner, 3 corner→center).
        assert!(mesh.lines.len() >= (4 * 6 + 6) * 2);
        // 4 nodes × (4 arrowheads + 16 dot segments).
        assert_eq!(mesh.triangles.len(), 4 * 20 * 3);
        // 4 nodes × (1 name label + 3 corner labels).
        assert_eq!(mesh.texts.len(), 16);
        assert_eq!(mesh.texts[0].text, "root.C L1 R");
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
        let mesh = build_scene(&[node], Vec2::new(800.0, 800.0));

        for vertex in mesh.lines.iter().chain(&mesh.triangles) {
            let clip = vertex.pos * mesh.world_to_clip.scale + mesh.world_to_clip.offset;
            assert!(clip.x.abs() <= 1.0, "clip.x {} out of range", clip.x);
            assert!(clip.y.abs() <= 1.0, "clip.y {} out of range", clip.y);
        }
    }

    #[test]
    fn empty_scene_produces_identity_like_transform() {
        let mesh = build_scene(&[], Vec2::new(800.0, 800.0));
        assert!(mesh.lines.is_empty() && mesh.triangles.is_empty() && mesh.texts.is_empty());
        assert_eq!(mesh.world_to_clip.scale, Vec2::new(2.0 / 800.0, -2.0 / 800.0));
    }

    #[test]
    fn hex_rgb_parses_channels() {
        assert_eq!(hex_rgb("#000000"), [0.0; 3]);
        assert_eq!(hex_rgb("#ffffff"), [1.0; 3]);
        assert_eq!(hex_rgb("#ff0000"), [1.0, 0.0, 0.0]);
    }
}

