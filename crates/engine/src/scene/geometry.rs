//! Per-node geometry builders: pure vertex-push helpers over `Vec<Vertex>`
//! buffers plus the `SceneBuilder` attribute builders that compose them.
//! Builders work in the node's own 3D world space (y-up) and track every
//! emitted point in the content bounds used for the camera fit.

use glam::{Vec2, Vec3};

use std::rc::Rc;

use crate::node::{Node, NodeRef};

use super::colors::{
    CORNER_LABEL_COLOR, DIRECTION_COLORS, LABEL_COLOR, NODE_DIRECTION_COLOR, ORIGIN_COLOR,
    VIOLATION_COLOR, level_color,
};
use super::{Bounds, LabelOffset, SceneBuilder, Vertex, WorldLabel};

/// Arrow length factor relative to the node's own size (same as `Svg::new`).
const ARROW_SCALE: f32 = 0.5;

/// Triangle-fan segment count of the filled discs (center dot, open-port
/// markers).
pub(crate) const DOT_SEGMENTS: usize = 16;

/// Pixel size of the name/level label.
const LABEL_SIZE_PX: f32 = 12.0;
/// Pixel size of the A/B/C corner labels.
const CORNER_LABEL_SIZE_PX: f32 = 10.0;
/// Pixel distance the A/B/C corner labels are pushed outward from the node
/// center.
const CORNER_LABEL_OFFSET_PX: f32 = 8.0;

/// Appends one line segment.
pub(crate) fn push_line(buf: &mut Vec<Vertex>, from: Vec3, to: Vec3, color: [f32; 3]) {
    buf.push(Vertex { pos: from, color });
    buf.push(Vertex { pos: to, color });
}

/// Appends one triangle.
pub(crate) fn push_triangle(buf: &mut Vec<Vertex>, a: Vec3, b: Vec3, c: Vec3, color: [f32; 3]) {
    for pos in [a, b, c] {
        buf.push(Vertex { pos, color });
    }
}

/// Appends a dashed line segment. The gap is half the dash, like the SVG
/// `stroke-dasharray="4 2"`. Coincident endpoints emit nothing.
pub(crate) fn push_dashed_line(
    buf: &mut Vec<Vertex>,
    from: Vec3,
    to: Vec3,
    dash: f32,
    color: [f32; 3],
) {
    let gap = dash / 2.0;
    let Some(dir) = (to - from).try_normalize() else {
        return;
    };
    let total = (to - from).length();
    let mut d = 0.0;
    while d < total {
        let seg_end = (d + dash).min(total);
        push_line(buf, from + dir * d, from + dir * seg_end, color);
        d += dash + gap;
    }
}

/// Appends a filled disc as a `segments`-triangle fan around `center`, in the
/// plane perpendicular to `normal`.
pub(crate) fn push_disc(
    buf: &mut Vec<Vertex>,
    center: Vec3,
    normal: Vec3,
    radius: f32,
    segments: usize,
    color: [f32; 3],
) {
    let (u, v) = plane_basis(normal);
    for i in 0..segments {
        let a0 = i as f32 / segments as f32 * std::f32::consts::TAU;
        let a1 = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
        push_triangle(
            buf,
            center,
            center + radius * (u * a0.cos() + v * a0.sin()),
            center + radius * (u * a1.cos() + v * a1.sin()),
            color,
        );
    }
}

/// Orthonormal basis `(u, v)` of the plane perpendicular to `direction`;
/// degenerate directions fall back to the XY plane.
pub(crate) fn plane_basis(direction: Vec3) -> (Vec3, Vec3) {
    let n = direction.try_normalize().unwrap_or(Vec3::Z);
    let reference = if n.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let u = n.cross(reference).normalize();
    (u, n.cross(u))
}

/// Appends a two-fin arrowhead at `tip`, pointing along `dir`: two triangles
/// in perpendicular planes, so the head stays readable from any camera angle.
pub(crate) fn push_arrowhead(
    buf: &mut Vec<Vertex>,
    tip: Vec3,
    dir: Vec3,
    size: f32,
    color: [f32; 3],
) {
    let dir = dir.try_normalize().unwrap_or(Vec3::Z);
    let back = dir * size;
    let (u, v) = plane_basis(dir);
    for axis in [u, v] {
        let side = axis * size * 0.45;
        push_triangle(buf, tip, tip - back + side, tip - back - side, color);
    }
}

/// Appends one arrow: shaft `from` → `to` (solid, or dashed with the given
/// dash length) into `lines`, plus a `head_size` arrowhead at `to` into
/// `triangles`.
pub(crate) fn push_arrow(
    lines: &mut Vec<Vertex>,
    triangles: &mut Vec<Vertex>,
    from: Vec3,
    to: Vec3,
    color: [f32; 3],
    head_size: f32,
    dash: Option<f32>,
) {
    match dash {
        Some(dash) => push_dashed_line(lines, from, to, dash, color),
        None => push_line(lines, from, to, color),
    }
    push_arrowhead(triangles, to, to - from, head_size, color);
}

/// Tracks `point` in the content `bounds` and returns it unchanged.
fn tracked(bounds: &mut Bounds, point: Vec3) -> Vec3 {
    bounds.track(point);
    point
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

/// Outward normal of the node's triangle (A,B,C winding); falls back to +Z
/// for degenerate triangles. Discs (center dot, open-port markers, violation
/// markers) are built in the plane perpendicular to it — painted-on-surface
/// markers that stay stable under camera rotation.
fn node_normal(node: &Node) -> Vec3 {
    let [a, b, c] = node.points;
    (b - a).cross(c - a).try_normalize().unwrap_or(Vec3::Z)
}

impl SceneBuilder {
    /// Appends the visual representation of one node, limited to the
    /// attributes enabled in `options`. Never alters the node. The node's
    /// center is passed to the enabled builders; each builder tracks the
    /// points it emits in the content bounds.
    pub(super) fn add_node(&mut self, node_ref: &NodeRef) {
        let node = &*node_ref.borrow();
        let arrow_len = arrow_length(node);
        let center = node.center;
        let normal = node_normal(node);
        if self.options.child_links {
            self.add_child_links(node, center, arrow_len);
        }
        if self.options.open_ports {
            self.add_open_port_markers(node, normal, arrow_len);
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
            self.add_center_dot(node, center, normal, arrow_len);
        }
        if self.options.labels {
            self.add_labels(node, center);
        }
        if self.options.link_violations {
            self.add_link_violations(node_ref, node, normal, arrow_len);
        }
    }

    /// Highlights every broken link — an occupied port whose recorded
    /// back-port does not point back to this node — by overdrawing the
    /// port's edge with a bright line and a disc at its midpoint. Meshes
    /// whose links are all wired through `topology::link` emit nothing.
    fn add_link_violations(
        &mut self,
        node_ref: &NodeRef,
        node: &Node,
        normal: Vec3,
        arrow_len: f32,
    ) {
        let [a, b, c] = node.points;
        let edge_endpoints = [(a, b), (b, c), (c, a)];
        for (index, child) in node.children.iter().enumerate() {
            let Some(child) = child else { continue };
            let intact = match node.back_ports[index] {
                None => false,
                Some(back) => {
                    let child = child.borrow();
                    child.children[back]
                        .as_ref()
                        .is_some_and(|link| Rc::ptr_eq(link, node_ref))
                        && child.back_ports[back] == Some(index)
                }
            };
            if intact {
                continue;
            }
            let (u, v) = edge_endpoints[index];
            let u = tracked(&mut self.bounds, u);
            let v = tracked(&mut self.bounds, v);
            push_line(&mut self.lines, u, v, VIOLATION_COLOR);
            push_disc(
                &mut self.triangles,
                (u + v) / 2.0,
                normal,
                arrow_len * 0.12,
                DOT_SEGMENTS,
                VIOLATION_COLOR,
            );
        }
    }

    /// Medium dashed line from the node's center to each non-empty child's
    /// center, colored with the direction color of the child slot (I/J/K).
    /// Each port's link is gated by its own switch on top of the group master.
    fn add_child_links(&mut self, node: &Node, center: Vec3, arrow_len: f32) {
        self.bounds.track(center);
        for (index, child) in node.children.iter().enumerate() {
            let Some(child) = child else { continue };
            if !self.options.child_links_ijk[index] {
                continue;
            }
            let to = tracked(&mut self.bounds, child.borrow().center);
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
    fn add_open_port_markers(&mut self, node: &Node, normal: Vec3, arrow_len: f32) {
        let [a, b, c] = node.points;
        let edge_midpoints = [(a + b) / 2.0, (b + c) / 2.0, (c + a) / 2.0];
        let radius = arrow_len * 0.18;
        for (index, child) in node.children.iter().enumerate() {
            if child.is_some() || !self.options.open_ports_ijk[index] {
                continue;
            }
            let outward = node.directions[index];
            let center = tracked(
                &mut self.bounds,
                edge_midpoints[index] + outward * radius * 1.4,
            );
            push_disc(
                &mut self.triangles,
                center,
                normal,
                radius,
                DOT_SEGMENTS,
                DIRECTION_COLORS[index],
            );
        }
    }

    /// Triangle outline through the corner points A → B → C → A, colored by level.
    fn add_triangle_outline(&mut self, node: &Node) {
        let color = level_color(node.level);
        let [a, b, c] = node.points.map(|point| tracked(&mut self.bounds, point));
        push_line(&mut self.lines, a, b, color);
        push_line(&mut self.lines, b, c, color);
        push_line(&mut self.lines, c, a, color);
    }

    /// One arrow per direction vector, starting at the node's center. Each
    /// port's arrow is gated by its own switch on top of the group master.
    /// The directions are normalized by construction (see `Node::new`).
    fn add_direction_arrows(&mut self, node: &Node, center: Vec3, arrow_len: f32) {
        self.bounds.track(center);
        for (index, direction) in node.directions.iter().enumerate() {
            if !self.options.directions_ijk[index] {
                continue;
            }
            let end = tracked(&mut self.bounds, node.center + direction * arrow_len);
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
    fn add_direction_of_node_arrow(&mut self, node: &Node, center: Vec3, arrow_len: f32) {
        self.bounds.track(center);
        let end = tracked(
            &mut self.bounds,
            node.center + node.direction_of_node * arrow_len,
        );
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
    fn add_origin_arrow(&mut self, node: &Node, center: Vec3, arrow_len: f32) {
        if node.direction_to_origin.length() == 0.0 {
            return;
        }
        self.bounds.track(center);
        let end = tracked(
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
    fn add_center_dot(&mut self, node: &Node, center: Vec3, normal: Vec3, arrow_len: f32) {
        self.bounds.track(center);
        push_disc(
            &mut self.triangles,
            center,
            normal,
            arrow_len * 0.08,
            DOT_SEGMENTS,
            level_color(node.level),
        );
    }

    /// Name/level label near the center plus corner labels A, B, C, anchored
    /// in world space and projected when the camera is applied.
    fn add_labels(&mut self, node: &Node, center: Vec3) {
        self.bounds.track(center);
        self.labels.push(WorldLabel {
            text: format!("{} L{}", node.name, node.level),
            world_pos: center,
            offset: LabelOffset::Fixed(Vec2::new(6.0, -16.0)),
            size_px: LABEL_SIZE_PX,
            color: LABEL_COLOR,
            centered: false,
        });
        for (point, corner_label) in node.points.iter().zip(['A', 'B', 'C']) {
            let corner = tracked(&mut self.bounds, *point);
            self.labels.push(WorldLabel {
                text: corner_label.to_string(),
                world_pos: corner,
                offset: LabelOffset::Outward {
                    from: center,
                    distance_px: CORNER_LABEL_OFFSET_PX,
                },
                size_px: CORNER_LABEL_SIZE_PX,
                color: CORNER_LABEL_COLOR,
                centered: true,
            });
        }
    }
}
