//! Color palette of the visualization: compile-time "#rrggbb" parsing, the
//! level palette cycled by `level % 8` and the fixed element colors.

/// Colors of the level palette, cycled by `level % 8` (same as the SVG one).
pub const LEVEL_COLORS: [[f32; 3]; 8] = [
    hex_rgb("#1f77b4"),
    hex_rgb("#ff7f0e"),
    hex_rgb("#2ca02c"),
    hex_rgb("#d62728"),
    hex_rgb("#9467bd"),
    hex_rgb("#8c564b"),
    hex_rgb("#e377c2"),
    hex_rgb("#7f7f7f"),
];

/// Colors of the I/J/K direction vectors (red/green/blue); also used for the
/// per-port child links, open-port markers and sub-switch labels.
pub const DIRECTION_COLORS: [[f32; 3]; 3] =
    [hex_rgb("#d32f2f"), hex_rgb("#388e3c"), hex_rgb("#1976d2")];
/// Color of the `direction_of_node` arrow.
pub(crate) const NODE_DIRECTION_COLOR: [f32; 3] = hex_rgb("#8e24aa");
/// Color of the dashed origin arrow.
pub(crate) const ORIGIN_COLOR: [f32; 3] = hex_rgb("#424242");
/// Color of the name/level label and the master checkbox labels.
pub(crate) const LABEL_COLOR: [f32; 3] = hex_rgb("#212121");
/// Color of the A/B/C corner labels.
pub(crate) const CORNER_LABEL_COLOR: [f32; 3] = hex_rgb("#757575");
/// Color of the checkbox panel geometry (box outlines and fills).
pub(crate) const UI_COLOR: [f32; 3] = hex_rgb("#424242");
/// Color of the reciprocal-port-rule violation highlights.
pub const VIOLATION_COLOR: [f32; 3] = hex_rgb("#ff5722");
/// Color of the UV-net wireframe in the UV-map view.
pub const UV_LINE_COLOR: [f32; 3] = hex_rgb("#000000");
/// Color of the UV vertex-distribution dots in the UV-map view: a red that
/// stays readable on both the black and the white checkerboard squares.
pub const UV_DOT_COLOR: [f32; 3] = hex_rgb("#d92626");

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
pub const fn hex_rgb(color: &str) -> [f32; 3] {
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
