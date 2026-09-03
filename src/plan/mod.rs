//! The `Plan` class: central manager of the spatial hierarchy. Encapsulates a
//! primary root node and orchestrates pentagonal base generation and the
//! dual-pentagon interlocked mesh. Pentagonal base construction lives in
//! `pentagon`, mesh subdivision in `subdivide`.
//! See `docs/classes-definitions/plan.md`.

mod pentagon;
mod subdivide;

#[cfg(test)]
mod tests;

use std::rc::Rc;

use glam::Vec2;

use crate::node::{Labeling, NodeRef};

pub use pentagon::generate_base;

/// Central manager of the node hierarchy. Holds a reference to the primary
/// base node that anchors the generated mesh.
#[derive(Default)]
pub struct Plan {
    /// Reference to the primary base node (North base root after `generate`).
    pub root_node: Option<NodeRef>,
}

impl Plan {
    /// Generates North and South pentagonal base structures and connects North
    /// inner nodes to South inner nodes via their remaining open ports
    /// (reciprocal I <-> K links). The South base is offset along the Y-axis
    /// and keeps the same orientation as the North base, but is generated with
    /// mirrored labeling so every South node's I/K direction vectors are
    /// swapped relative to the North convention. Returns the primary root
    /// node (North base root).
    ///
    /// - `side_length` — length of each pentagon edge.
    pub fn generate(&mut self, side_length: f32) -> NodeRef {
        let north_center = Vec2::new(side_length * 3.0, side_length * 3.0);
        let north_dir = Vec2::Y;
        let r = pentagon::apothem(side_length);
        let south_center = north_center + Vec2::new(0.0, 4.0 * r);

        let north_root = pentagon::generate_base("north_", side_length, north_dir, north_center, Labeling::Normal);
        let south_root = pentagon::generate_base("south_", side_length, north_dir, south_center, Labeling::Mirrored);

        let north_reverted = pentagon::reverted_nodes(&north_root);
        let south_reverted = pentagon::reverted_nodes(&south_root);

        pentagon::wire_interlock(&north_reverted, &south_reverted);

        self.root_node = Some(Rc::clone(&north_root));
        north_root
    }
}
