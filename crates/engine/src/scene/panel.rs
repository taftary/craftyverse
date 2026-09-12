//! Checkbox panel of the display options: a top-left anchored pixel-space
//! overlay — one row per attribute with its box, fill and label, plus the
//! hit rectangles the viewer clicks against.

use glam::Vec2;

use super::colors::{DIRECTION_COLORS, LABEL_COLOR, UI_COLOR};
use super::geometry::{push_line, push_triangle};
use super::options::{ATTRIBUTES, EFFECTS, PanelItem};
use super::{PanelRow, SceneBuilder, TextRun, Vertex};

/// Checkbox panel metrics (pixels, y-down); top-left anchored.
const PANEL_PAD: f32 = 8.0;
/// Edge length of a checkbox box.
const CHECKBOX_SIZE: f32 = 12.0;
/// Vertical pitch of the panel rows.
const CHECKBOX_ROW_HEIGHT: f32 = 18.0;
/// Pixel size of the checkbox labels.
const CHECKBOX_LABEL_SIZE: f32 = 11.0;
/// Horizontal gap between a checkbox box and its label.
const CHECKBOX_LABEL_GAP: f32 = 6.0;
/// Extra left indent of the per-port sub-switches under their group master.
const CHECKBOX_SUB_INDENT: f32 = 16.0;

/// Appends the four edges of the `min`–`max` rectangle (z = 0).
fn rect_outline(buf: &mut Vec<Vertex>, min: Vec2, max: Vec2, color: [f32; 3]) {
    let corners = [
        min.extend(0.0),
        Vec2::new(max.x, min.y).extend(0.0),
        max.extend(0.0),
        Vec2::new(min.x, max.y).extend(0.0),
    ];
    for edge in 0..4 {
        push_line(buf, corners[edge], corners[(edge + 1) % 4], color);
    }
}

/// Appends the two triangles filling the `min`–`max` rectangle (z = 0).
fn fill_rect(buf: &mut Vec<Vertex>, min: Vec2, max: Vec2, color: [f32; 3]) {
    let corners = [
        min.extend(0.0),
        Vec2::new(max.x, min.y).extend(0.0),
        max.extend(0.0),
        Vec2::new(min.x, max.y).extend(0.0),
    ];
    push_triangle(buf, corners[0], corners[1], corners[2], color);
    push_triangle(buf, corners[0], corners[2], corners[3], color);
}

impl SceneBuilder {
    /// Checkbox panel (top-left, pixel space): one row per attribute — a box,
    /// filled when the attribute is on, plus its label — then one radio row
    /// per texture effect, filled for the active effect. Per-port
    /// sub-switches are indented under their group master and labeled with
    /// the port's direction color. Always generated so the options stay
    /// reachable when every attribute is off.
    pub(super) fn add_checkbox_panel(&mut self) {
        for (row, &(attribute, label)) in ATTRIBUTES.iter().enumerate() {
            let indent = attribute.port().map_or(0.0, |_| CHECKBOX_SUB_INDENT);
            let label_color = attribute
                .port()
                .map_or(LABEL_COLOR, |port| DIRECTION_COLORS[port.index()]);
            let on = self.options.value(attribute);
            self.add_panel_row(
                row,
                PanelItem::Attribute(attribute),
                label,
                indent,
                label_color,
                on,
            );
        }
        for (index, &(effect, label)) in EFFECTS.iter().enumerate() {
            let on = self.options.effect == effect;
            self.add_panel_row(
                ATTRIBUTES.len() + index,
                PanelItem::Effect(effect),
                label,
                0.0,
                LABEL_COLOR,
                on,
            );
        }
    }

    /// One panel row for `item` with text `label` at row index `row`: box
    /// outline, inset fill when `on`, label and clickable rectangle.
    fn add_panel_row(
        &mut self,
        row: usize,
        item: PanelItem,
        label: &'static str,
        indent: f32,
        label_color: [f32; 3],
        on: bool,
    ) {
        let min = Vec2::new(
            PANEL_PAD + indent,
            PANEL_PAD + row as f32 * CHECKBOX_ROW_HEIGHT,
        );
        let max = min + Vec2::splat(CHECKBOX_SIZE);
        rect_outline(&mut self.ui_lines, min, max, UI_COLOR);
        if on {
            fill_rect(
                &mut self.ui_triangles,
                min + Vec2::splat(3.0),
                max - Vec2::splat(3.0),
                UI_COLOR,
            );
        }
        let anchor = Vec2::new(
            max.x + CHECKBOX_LABEL_GAP,
            min.y + (CHECKBOX_SIZE - CHECKBOX_LABEL_SIZE) / 2.0,
        );
        self.ui_labels.push(TextRun {
            text: label,
            anchor,
            size: CHECKBOX_LABEL_SIZE,
            color: label_color,
            centered: false,
        });
        // The clickable rectangle covers the box and the label (width
        // estimated from the monospace advance).
        let label_width = label.len() as f32 * CHECKBOX_LABEL_SIZE * 0.6;
        self.panel_rows.push(PanelRow {
            item,
            min,
            max: Vec2::new(anchor.x + label_width, max.y),
        });
    }
}
