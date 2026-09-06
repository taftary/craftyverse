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

/// Clickable area of one checkbox (pixel space, y-down).
///
/// The viewer hit-tests mouse clicks against these rectangles.
#[derive(Clone, Copy, Debug)]
pub struct Checkbox {
    /// Attribute controlled by this checkbox.
    pub attribute: Attribute,
    /// Top-left corner of the clickable rectangle (checkbox box plus label),
    /// in pixels.
    pub min: Vec2,
    /// Bottom-right corner of the clickable rectangle, in pixels.
    pub max: Vec2,
}

impl Checkbox {
    /// Returns `true` if `point` (pixels, y-down) is inside the clickable
    /// rectangle.
    pub fn contains(&self, point: Vec2) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }
}
