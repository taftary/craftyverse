//! Display options: the toggleable node attributes, the flag set backing
//! them and the checkbox hit rectangles.

use glam::Vec2;

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
    pub(crate) fn port(self) -> Option<usize> {
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
    /// All attributes off — complement of the all-on `Default`. Test-only so
    /// far: gated to keep the non-test build free of dead code.
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

    /// Current display state of one attribute.
    pub fn value(&self, attribute: Attribute) -> bool {
        *self.field(attribute)
    }

    /// Flips one attribute (checkbox click).
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
