//! The `Plan` module: central manager of the node hierarchy.
//!
//! A [`Plan`] encapsulates the primary root node that anchors a generated mesh.
//! It orchestrates pentagonal base generation and the dual-pentagon interlocked
//! mesh, and it can subdivide the whole mesh one level at a time.
//!
//! Pentagonal base construction lives in `pentagon`; whole-mesh subdivision
//! lives in `subdivide`. The full contract is specified in
//! `docs/rust/book/specs/plan.md`.
//!
//! # Example
//!
//! ```
//! use planet_crafter_engine::plan::Plan;
//!
//! let mut plan = Plan::default();
//! let root = plan.generate(1.0);
//! assert!(plan.root_node.is_some());
//! assert_eq!(root.borrow().level, 0);
//! ```

mod pentagon;
mod subdivide;

#[cfg(test)]
mod tests;

use std::rc::Rc;

use glam::Vec2;

use crate::node::{Labeling, NodeRef};

pub use pentagon::generate_base;

/// Central manager of the node hierarchy.
///
/// Holds a reference to the primary base node that anchors the generated mesh.
/// After [`Plan::generate`] the root node is the North base root of the
/// dual-pentagon interlocked mesh.
#[derive(Default)]
pub struct Plan {
    /// Reference to the primary base node.
    ///
    /// `None` before [`Plan::generate`] is called. After generation it points
    /// to the North base root, and after [`Plan::split`] it is re-anchored on
    /// the old root's center node.
    pub root_node: Option<NodeRef>,
}

impl Plan {
    /// Generates the dual-pentagon interlocked mesh.
    ///
    /// This method builds a North pentagonal base and a South pentagonal base,
    /// then connects the inner (reverted) nodes of the North base to the inner
    /// nodes of the South base through reciprocal I <-> K links.
    ///
    /// The South base is offset along the positive Y-axis and keeps the same
    /// orientation as the North base, but it is generated with mirrored
    /// labeling so every South node's I/K direction vectors are swapped
    /// relative to the North convention.
    ///
    /// # Parameters
    ///
    /// - `side_length` — length of each pentagon edge.
    ///
    /// # Returns
    ///
    /// The North base root node, which becomes the primary root of the plan.
    ///
    /// # Example
    ///
    /// ```
    /// use planet_crafter_engine::plan::Plan;
    ///
    /// let mut plan = Plan::default();
    /// let root = plan.generate(2.0);
    /// assert_eq!(root.borrow().name, "north_base_node_0");
    /// ```
    pub fn generate(&mut self, side_length: f32) -> NodeRef {
        let north_center = Vec2::new(side_length * 3.0, side_length * 3.0);
        let north_dir = Vec2::Y;
        let r = pentagon::apothem(side_length);
        let south_center = north_center + Vec2::new(0.0, 4.0 * r);

        let north_root = pentagon::generate_base(
            "north_",
            side_length,
            north_dir,
            north_center,
            Labeling::Normal,
        );
        let south_root = pentagon::generate_base(
            "south_",
            side_length,
            north_dir,
            south_center,
            Labeling::Mirrored,
        );

        let north_reverted = pentagon::reverted_nodes(&north_root);
        let south_reverted = pentagon::reverted_nodes(&south_root);

        pentagon::wire_interlock(&north_reverted, &south_reverted);

        self.root_node = Some(Rc::clone(&north_root));
        north_root
    }
}
