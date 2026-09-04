//! Display options: the toggleable node attributes, the flag set backing
//! them, and the checkbox hit rectangles.

use glam::Vec2;

/// One toggleable node attribute of the visualization.
///
/// Each variant corresponds to a row in the display-options panel.
/// `ChildLinks`, `OpenPorts`, and `Directions` are group masters that gate
/// their per-port sub-switches:
///
/// - `ChildLink(i)`, `OpenPort(i)`, `Direction(i)` where `i` is `0` for I,
///   `1` for J, and `2` for K.
///
/// An element is drawn only when both its group master and its per-port switch
/// are enabled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Attribute {
    /// Master switch for child-link lines.
    ChildLinks,
    /// Per-port child-link switch.
    ChildLink(usize),
    /// Master switch for open-port markers.
    OpenPorts,
    /// Per-port open-port switch.
    OpenPort(usize),
    /// Triangle outline.
    Outline,
    /// Master switch for I/J/K direction arrows.
    Directions,
    /// Per-port direction arrow switch.
    Direction(usize),
    /// Node direction arrow (base to apex).
    DirectionOfNode,
    /// Dashed origin arrow.
    Origin,
    /// Filled center dot.
    CenterDot,
    /// Node name/level and corner letter labels.
    Labels,
}

impl Attribute {
    /// Port index when the attribute is a per-port sub-switch of a group.
    pub(crate) fn port(self) -> Option<usize> {
        match self {
            Attribute::ChildLink(i) | Attribute::OpenPort(i) | Attribute::Direction(i) => Some(i),
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
    /// Returns an option set with every attribute disabled.
    ///
    /// Test-only: gated to keep the non-test build free of dead code.
    #[cfg(test)]
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

    /// The flag backing `attribute`; the single attribute → field mapping,
    /// mirrored by `field_mut`.
    fn field(&self, attribute: Attribute) -> &bool {
        match attribute {
            Attribute::ChildLinks => &self.child_links,
            Attribute::ChildLink(i) => &self.child_links_ijk[i],
            Attribute::OpenPorts => &self.open_ports,
            Attribute::OpenPort(i) => &self.open_ports_ijk[i],
            Attribute::Outline => &self.outline,
            Attribute::Directions => &self.directions,
            Attribute::Direction(i) => &self.directions_ijk[i],
            Attribute::DirectionOfNode => &self.direction_of_node,
            Attribute::Origin => &self.origin,
            Attribute::CenterDot => &self.center_dot,
            Attribute::Labels => &self.labels,
        }
    }

    /// Mutable variant of `field`; the two matches must stay in sync.
    fn field_mut(&mut self, attribute: Attribute) -> &mut bool {
        match attribute {
            Attribute::ChildLinks => &mut self.child_links,
            Attribute::ChildLink(i) => &mut self.child_links_ijk[i],
            Attribute::OpenPorts => &mut self.open_ports,
            Attribute::OpenPort(i) => &mut self.open_ports_ijk[i],
            Attribute::Outline => &mut self.outline,
            Attribute::Directions => &mut self.directions,
            Attribute::Direction(i) => &mut self.directions_ijk[i],
            Attribute::DirectionOfNode => &mut self.direction_of_node,
            Attribute::Origin => &mut self.origin,
            Attribute::CenterDot => &mut self.center_dot,
            Attribute::Labels => &mut self.labels,
        }
    }
}

/// Checkboxes of the display-options panel, in display order. Per-port
/// sub-switches sit right under their group master.
pub(crate) const ATTRIBUTES: [(Attribute, &str); 17] = [
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
