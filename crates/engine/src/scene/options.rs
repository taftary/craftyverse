//! Display options: the toggleable node attributes, the flag set backing
//! them, and the checkbox hit rectangles.

use glam::Vec2;

/// One of the three node ports, labeling the I/J/K direction slots.
///
/// `I` is perpendicular to edge AB, `J` to BC, `K` to CA. Carrying a `Port`
/// instead of a raw index makes invalid ports unrepresentable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Port {
    /// Port I (perpendicular to edge AB).
    I,
    /// Port J (perpendicular to edge BC).
    J,
    /// Port K (perpendicular to edge CA).
    K,
}

impl Port {
    /// Position of the port in I/J/K order, for array indexing.
    pub(crate) fn index(self) -> usize {
        match self {
            Port::I => 0,
            Port::J => 1,
            Port::K => 2,
        }
    }
}

/// One toggleable node attribute of the visualization.
///
/// Each variant corresponds to a row in the display-options panel.
/// `ChildLinks`, `OpenPorts`, and `Directions` are group masters that gate
/// their per-port sub-switches `ChildLink(port)`, `OpenPort(port)`, and
/// `Direction(port)`.
///
/// An element is drawn only when both its group master and its per-port switch
/// are enabled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Attribute {
    /// Master switch for child-link lines.
    ChildLinks,
    /// Per-port child-link switch.
    ChildLink(Port),
    /// Master switch for open-port markers.
    OpenPorts,
    /// Per-port open-port switch.
    OpenPort(Port),
    /// Triangle outline.
    Outline,
    /// Master switch for I/J/K direction arrows.
    Directions,
    /// Per-port direction arrow switch.
    Direction(Port),
    /// Node direction arrow (base to apex).
    DirectionOfNode,
    /// Dashed origin arrow.
    Origin,
    /// Filled center dot.
    CenterDot,
    /// Node name/level and corner letter labels.
    Labels,
    /// Highlight of links that violate the reciprocal port rule.
    LinkViolations,
}

impl Attribute {
    /// Port when the attribute is a per-port sub-switch of a group.
    pub(crate) fn port(self) -> Option<Port> {
        match self {
            Attribute::ChildLink(port) | Attribute::OpenPort(port) | Attribute::Direction(port) => {
                Some(port)
            }
            _ => None,
        }
    }
}

/// One procedural texture effect of the textured view mode.
///
/// Each variant corresponds to a radio row in the display-options panel:
/// exactly one effect is active at a time. The effect is evaluated per
/// pixel from the triangle-local barycentric coordinates, the node's
/// topology parity and its radial direction (see `render::procedural` for
/// the CPU reference); it never reads UVs, so it is independent of UV
/// seams.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TextureEffect {
    /// Barycentric gradient: `uA` → red, `uB` → green, `uC` → blue.
    Gradient,
    /// Sub-triangle checkerboard, phase-flipped by parity (default).
    #[default]
    Checkerboard,
    /// Stripes along `uC`: iso-lines parallel to edge `AB` (direction `I`),
    /// phase-flipped by parity.
    StripesI,
    /// Stripes along `uA` (the altitude coordinate): iso-lines parallel to
    /// edge `BC` (direction `J`), phase-flipped by parity.
    StripesJ,
    /// Stripes along `uB`: iso-lines parallel to edge `CA` (direction `K`),
    /// phase-flipped by parity.
    StripesK,
    /// Band mask along one edge, edge-flipped by parity.
    EdgeMask,
    /// Outward radial direction as RGB (normal visualization).
    RadialRgb,
    /// Diffuse lighting: `dot(outward radial, light dir)` grayscale.
    Diffuse,
    /// Latitude bands around the poles (stripes of `dot(radial, Y)`).
    Latitude,
    /// Fresnel rim: bright silhouette edges (`1 - |dot(radial, view)|`).
    Fresnel,
}

impl TextureEffect {
    /// Fragment-shader mode selecting this effect (1..=10). Mode 0 is
    /// reserved: it samples the checkerboard texture (the UV-map view).
    pub fn shader_mode(self) -> u32 {
        match self {
            TextureEffect::Gradient => 1,
            TextureEffect::Checkerboard => 2,
            TextureEffect::StripesI => 3,
            TextureEffect::StripesJ => 4,
            TextureEffect::StripesK => 5,
            TextureEffect::EdgeMask => 6,
            TextureEffect::RadialRgb => 7,
            TextureEffect::Diffuse => 8,
            TextureEffect::Latitude => 9,
            TextureEffect::Fresnel => 10,
        }
    }
}

/// The panel row item a click hit: an attribute checkbox (toggles) or a
/// texture-effect radio row (selects).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PanelItem {
    /// Attribute checkbox row.
    Attribute(Attribute),
    /// Texture-effect radio row.
    Effect(TextureEffect),
}

/// Which node attributes the visualization displays.
///
/// Toggled at runtime through the checkbox panel; everything is on by default.
#[derive(Clone, Copy, Debug)]
pub struct DisplayOptions {
    /// Master switch for child-link lines.
    pub child_links: bool,
    /// Per-port child-link switches, indexed `[I, J, K]`.
    pub child_links_ijk: [bool; 3],
    /// Master switch for open-port markers.
    pub open_ports: bool,
    /// Per-port open-port switches, indexed `[I, J, K]`.
    pub open_ports_ijk: [bool; 3],
    /// Triangle outline switch.
    pub outline: bool,
    /// Master switch for I/J/K direction arrows.
    pub directions: bool,
    /// Per-port direction arrow switches, indexed `[I, J, K]`.
    pub directions_ijk: [bool; 3],
    /// Node direction arrow switch.
    pub direction_of_node: bool,
    /// Dashed origin arrow switch.
    pub origin: bool,
    /// Center dot switch.
    pub center_dot: bool,
    /// Label switch.
    pub labels: bool,
    /// Highlight switch for links that violate the reciprocal port rule
    /// (`child.children[2 - port]` does not point back).
    pub link_violations: bool,
    /// Active procedural texture effect of the textured view mode (a radio
    /// selection, not a toggle; ignored by the other view modes).
    pub effect: TextureEffect,
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
            link_violations: true,
            effect: TextureEffect::default(),
        }
    }
}

/// Maps an [`Attribute`] to its backing flag: `attribute_field!(self, attr)`
/// yields `&flag`, `attribute_field!(self, attr, mut)` yields `&mut flag`.
macro_rules! attribute_field {
    ($self:ident, $attribute:ident $(, $mut:tt)?) => {
        match $attribute {
            Attribute::ChildLinks => &$($mut)? $self.child_links,
            Attribute::ChildLink(port) => &$($mut)? $self.child_links_ijk[port.index()],
            Attribute::OpenPorts => &$($mut)? $self.open_ports,
            Attribute::OpenPort(port) => &$($mut)? $self.open_ports_ijk[port.index()],
            Attribute::Outline => &$($mut)? $self.outline,
            Attribute::Directions => &$($mut)? $self.directions,
            Attribute::Direction(port) => &$($mut)? $self.directions_ijk[port.index()],
            Attribute::DirectionOfNode => &$($mut)? $self.direction_of_node,
            Attribute::Origin => &$($mut)? $self.origin,
            Attribute::CenterDot => &$($mut)? $self.center_dot,
            Attribute::Labels => &$($mut)? $self.labels,
            Attribute::LinkViolations => &$($mut)? $self.link_violations,
        }
    };
}

impl DisplayOptions {
    /// Returns an option set with every attribute disabled.
    ///
    /// Test-only: gated behind the `test-internals` feature to keep the
    /// default build free of dead code.
    #[cfg(feature = "test-internals")]
    pub fn none() -> Self {
        Self {
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
            link_violations: false,
            effect: TextureEffect::default(),
        }
    }

    /// Returns the current display state of `attribute`.
    pub fn value(&self, attribute: Attribute) -> bool {
        *self.field(attribute)
    }

    /// Toggles `attribute` (used on checkbox click).
    pub fn toggle(&mut self, attribute: Attribute) {
        let field = self.field_mut(attribute);
        *field = !*field;
    }

    /// The flag backing `attribute`.
    fn field(&self, attribute: Attribute) -> &bool {
        attribute_field!(self, attribute)
    }

    /// Mutable variant of `field`; the shared macro keeps the two mappings
    /// in sync by construction.
    fn field_mut(&mut self, attribute: Attribute) -> &mut bool {
        attribute_field!(self, attribute, mut)
    }
}

/// Checkboxes of the display-options panel, in display order. Per-port
/// sub-switches sit right under their group master.
pub const ATTRIBUTES: [(Attribute, &str); 18] = [
    (Attribute::ChildLinks, "child links"),
    (Attribute::ChildLink(Port::I), "link I"),
    (Attribute::ChildLink(Port::J), "link J"),
    (Attribute::ChildLink(Port::K), "link K"),
    (Attribute::OpenPorts, "open ports"),
    (Attribute::OpenPort(Port::I), "port I"),
    (Attribute::OpenPort(Port::J), "port J"),
    (Attribute::OpenPort(Port::K), "port K"),
    (Attribute::Outline, "outline"),
    (Attribute::Directions, "directions ijk"),
    (Attribute::Direction(Port::I), "dir I"),
    (Attribute::Direction(Port::J), "dir J"),
    (Attribute::Direction(Port::K), "dir K"),
    (Attribute::DirectionOfNode, "direction of node"),
    (Attribute::Origin, "origin arrow"),
    (Attribute::CenterDot, "center dot"),
    (Attribute::Labels, "labels"),
    (Attribute::LinkViolations, "link violations"),
];

/// Texture-effect radio rows of the display-options panel, in display order
/// (right below the attribute checkboxes). Exactly one is active at a time.
pub const EFFECTS: [(TextureEffect, &str); 10] = [
    (TextureEffect::Gradient, "fx gradient"),
    (TextureEffect::Checkerboard, "fx checkerboard"),
    (TextureEffect::StripesI, "fx stripes I"),
    (TextureEffect::StripesJ, "fx stripes J"),
    (TextureEffect::StripesK, "fx stripes K"),
    (TextureEffect::EdgeMask, "fx edge mask"),
    (TextureEffect::RadialRgb, "fx radial rgb"),
    (TextureEffect::Diffuse, "fx diffuse"),
    (TextureEffect::Latitude, "fx latitude"),
    (TextureEffect::Fresnel, "fx fresnel"),
];

/// Clickable area of one panel row (pixel space, y-down).
///
/// The viewer hit-tests mouse clicks against these rectangles; clicking
/// toggles an attribute checkbox or selects a texture-effect radio row.
#[derive(Clone, Copy, Debug)]
pub struct PanelRow {
    /// Panel item controlled by this row.
    pub item: PanelItem,
    /// Top-left corner of the clickable rectangle (checkbox box plus label),
    /// in pixels.
    pub min: Vec2,
    /// Bottom-right corner of the clickable rectangle, in pixels.
    pub max: Vec2,
}

impl PanelRow {
    /// Returns `true` if `point` (pixels, y-down) is inside the clickable
    /// rectangle.
    pub fn contains(&self, point: Vec2) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }
}
