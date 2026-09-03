//! Per-node geometry builders: pure vertex-push helpers over `Vec<Vertex>`
//! buffers plus the `SceneBuilder` attribute builders that compose them.
//! Builders receive the node's center already mapped (y-down) and track every
//! emitted point in the content bounds used for the view fit.

use glam::Vec2;

use crate::node::Node;

use super::colors::{
    level_color, CORNER_LABEL_COLOR, DIRECTION_COLORS, LABEL_COLOR, NODE_DIRECTION_COLOR,
    ORIGIN_COLOR,
};
use super::{Bounds, LabelRequest, SceneBuilder, Vertex};

/// Arrow length factor relative to the node's own size (same as `Svg::new`).
const ARROW_SCALE: f32 = 0.5;

/// Triangle-fan segment count of the filled discs (center dot, open-port
/// markers).
pub(crate) const DOT_SEGMENTS: usize = 16;

/// Pixel size of the name/level label.
const LABEL_SIZE_PX: f32 = 12.0;
/// Pixel size of the A/B/C corner labels.
const CORNER_LABEL_SIZE_PX: f32 = 10.0;

/// Node geometry is 2D and y-up; the view space is y-down. Pure `(x, -y)`
/// flip (same mapping the SVG viewer used); bounds tracking stays explicit
/// at the call sites.
pub(crate) fn flip_y(point: Vec2) -> Vec2 {
    Vec2::new(point.x, -point.y)
}

/// Appends one line segment; endpoints are already mapped.
pub(crate) fn push_line(buf: &mut Vec<Vertex>, from: Vec2, to: Vec2, color: [f32; 3]) {
    buf.push(Vertex { pos: from, color });
    buf.push(Vertex { pos: to, color });
}

/// Appends one triangle; corners are already mapped.
pub(crate) fn push_triangle(buf: &mut Vec<Vertex>, a: Vec2, b: Vec2, c: Vec2, color: [f32; 3]) {
    for pos in [a, b, c] {
        buf.push(Vertex { pos, color });
    }
}

/// Appends a dashed line segment; endpoints are already mapped. The gap is
/// half the dash, like the SVG `stroke-dasharray="4 2"`.
pub(crate) fn push_dashed_line(
    buf: &mut Vec<Vertex>,
    from: Vec2,
    to: Vec2,
    dash: f32,
    color: [f32; 3],
) {
    let gap = dash / 2.0;
    let dir = (to - from).normalize();
    let total = (to - from).length();
    let mut d = 0.0;
    while d < total {
        let seg_end = (d + dash).min(total);
        push_line(buf, from + dir * d, from + dir * seg_end, color);
        d += dash + gap;
    }
}

/// Appends a filled disc as a `segments`-triangle fan around `center`;
/// `center` is already mapped.
pub(crate) fn push_disc(
    buf: &mut Vec<Vertex>,
    center: Vec2,
    radius: f32,
    segments: usize,
    color: [f32; 3],
) {
    for i in 0..segments {
        let a0 = i as f32 / segments as f32 * std::f32::consts::TAU;
        let a1 = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
        push_triangle(
            buf,
            center,
            center + radius * Vec2::new(a0.cos(), a0.sin()),
            center + radius * Vec2::new(a1.cos(), a1.sin()),
            color,
        );
    }
}

/// Appends a small filled triangle at `tip`, pointing along `dir` (mapped
/// space).
pub(crate) fn push_arrowhead(
    buf: &mut Vec<Vertex>,
    tip: Vec2,
    dir: Vec2,
    size: f32,
    color: [f32; 3],
) {
    let back = dir * size;
    let side = Vec2::new(-dir.y, dir.x) * size * 0.45;
    push_triangle(buf, tip, tip - back + side, tip - back - side, color);
}

/// Appends one arrow: shaft `from` → `to` (solid, or dashed with the given
/// dash length) into `lines`, plus a `head_size` arrowhead at `to` into
/// `triangles`.
pub(crate) fn push_arrow(
    lines: &mut Vec<Vertex>,
    triangles: &mut Vec<Vertex>,
    from: Vec2,
    to: Vec2,
    color: [f32; 3],
    head_size: f32,
    dash: Option<f32>,
) {
    match dash {
        Some(dash) => push_dashed_line(lines, from, to, dash, color),
        None => push_line(lines, from, to, color),
    }
    push_arrowhead(triangles, to, (to - from).normalize(), head_size, color);
}

/// Flips `point` into y-down view space, tracks it in the content `bounds`
/// and returns it (`flip_y` + `Bounds::track`).
fn map_track(bounds: &mut Bounds, point: Vec2) -> Vec2 {
    let mapped = flip_y(point);
    bounds.track(mapped);
    mapped
}

/// Arrows scale with the node's own triangle so they stay readable after
/// repeated `split()` calls (same rule as the SVG viewer).
pub(crate) fn arrow_length(node: &Node) -> f32 {
    let min_corner_distance = node
        .points
        .iter()
        .map(|point| (*point - node.center).length())
        .fold(f32::INFINITY, f32::min);
    ARROW_SCALE * min_corner_distance
}

impl SceneBuilder {
    /// Appends the visual representation of one node, limited to the
    /// attributes enabled in `options`. Never alters the node. The node's
    /// center is mapped once here and passed to the enabled builders; each
    /// builder tracks it in the content bounds only when it actually emits.
    pub(super) fn add_node(&mut self, node: &Node) {
        let arrow_len = arrow_length(node);
        let center = flip_y(node.center);
        if self.options.child_links {
            self.add_child_links(node, center, arrow_len);
        }
        if self.options.open_ports {
            self.add_open_port_markers(node, arrow_len);
        }
        if self.options.outline {
            self.add_triangle_outline(node);
        }
        if self.options.directions {
            self.add_direction_arrows(node, center, arrow_len);
        }
        if self.options.direction_of_node {
            self.add_direction_of_node_arrow(node, center, arrow_len);
        }
        if self.options.origin {
            self.add_origin_arrow(node, center, arrow_len);
        }
        if self.options.center_dot {
            self.add_center_dot(node, center, arrow_len);
        }
        if self.options.labels {
            self.add_labels(node, center);
        }
    }

    /// Medium dashed line from the node's center to each non-empty child's
    /// center, colored with the direction color of the child slot (I/J/K).
    /// Each port's link is gated by its own switch on top of the group master.
    fn add_child_links(&mut self, node: &Node, center: Vec2, arrow_len: f32) {
        self.bounds.track(center);
        for (index, child) in node.children.iter().enumerate() {
            let Some(child) = child else { continue };
            if !self.options.child_links_ijk[index] {
                continue;
            }
            let to = map_track(&mut self.bounds, child.borrow().center);
            push_dashed_line(
                &mut self.lines,
                center,
                to,
                arrow_len / 6.0,
                DIRECTION_COLORS[index],
            );
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
            let center = map_track(&mut self.bounds, edge_midpoints[index] + outward * radius * 1.4);
            // Extend the content bounds to the disc rim so the view fit never
            // clips a marker.
            self.bounds
                .track(flip_y(edge_midpoints[index] + outward * radius * 2.4));
            push_disc(
                &mut self.triangles,
                center,
                radius,
                DOT_SEGMENTS,
                DIRECTION_COLORS[index],
            );
        }
    }

    /// Triangle outline through the corner points A → B → C → A, colored by level.
    fn add_triangle_outline(&mut self, node: &Node) {
        let color = level_color(node.level);
        let [a, b, c] = node.points.map(|point| map_track(&mut self.bounds, point));
        push_line(&mut self.lines, a, b, color);
        push_line(&mut self.lines, b, c, color);
        push_line(&mut self.lines, c, a, color);
    }

    /// One arrow per direction vector, starting at the node's center. Each
    /// port's arrow is gated by its own switch on top of the group master.
    fn add_direction_arrows(&mut self, node: &Node, center: Vec2, arrow_len: f32) {
        self.bounds.track(center);
        for (index, direction) in node.directions.iter().enumerate() {
            if !self.options.directions_ijk[index] {
                continue;
            }
            let end = map_track(&mut self.bounds, node.center + direction.normalize() * arrow_len);
            let color = DIRECTION_COLORS[index];
            push_arrow(
                &mut self.lines,
                &mut self.triangles,
                center,
                end,
                color,
                arrow_len * 0.25,
                None,
            );
        }
    }

    /// One arrow along `direction_of_node` (base BC → apex A), starting at
    /// the node's center.
    fn add_direction_of_node_arrow(&mut self, node: &Node, center: Vec2, arrow_len: f32) {
        self.bounds.track(center);
        let end = map_track(&mut self.bounds, node.center + node.direction_of_node * arrow_len);
        push_arrow(
            &mut self.lines,
            &mut self.triangles,
            center,
            end,
            NODE_DIRECTION_COLOR,
            arrow_len * 0.25,
            None,
        );
    }

    /// Dashed arrow from the node's center along the normalized
    /// `direction_to_origin`. Skipped when the vector has zero length.
    fn add_origin_arrow(&mut self, node: &Node, center: Vec2, arrow_len: f32) {
        if node.direction_to_origin.length() == 0.0 {
            return;
        }
        self.bounds.track(center);
        let end = map_track(
            &mut self.bounds,
            node.center + node.direction_to_origin.normalize() * arrow_len,
        );
        push_arrow(
            &mut self.lines,
            &mut self.triangles,
            center,
            end,
            ORIGIN_COLOR,
            arrow_len * 0.25,
            Some(arrow_len / 6.0),
        );
    }

    /// Filled dot on top of all lines, colored by level.
    fn add_center_dot(&mut self, node: &Node, center: Vec2, arrow_len: f32) {
        self.bounds.track(center);
        push_disc(
            &mut self.triangles,
            center,
            arrow_len * 0.08,
            DOT_SEGMENTS,
            level_color(node.level),
        );
    }

    /// Name/level label near the center plus corner labels A, B, C.
    fn add_labels(&mut self, node: &Node, center: Vec2) {
        self.bounds.track(center);
        self.labels.push(LabelRequest {
            text: format!("{} L{}", node.name, node.level),
            world_pos: center,
            offset_px: Vec2::new(6.0, -16.0),
            size_px: LABEL_SIZE_PX,
            color: LABEL_COLOR,
            centered: false,
        });
        for (point, corner_label) in node.points.iter().zip(['A', 'B', 'C']) {
            let corner = map_track(&mut self.bounds, *point);
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
}
