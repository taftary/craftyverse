//! The `lod` module: the headless chunk LOD scheduler.
//!
//! Node-graph only - no GPU, window, or render dependency, so it is fully
//! unit-testable headless. Each frame the [`LodScheduler`] reads the player
//! position and decides which chunks split, merge, load, and unload, within
//! a hard per-frame operation budget. Topology changes go through the local
//! refinement operation ([`split_node_local`](crate::node::split_node_local))
//! and the local merge ([`unsplit_node`](crate::node::unsplit_node)); the
//! level difference across any shared edge never exceeds 1 (restricted
//! subdivision), and the residual T-junction seams are published as
//! [`BorderState`] metadata for the skirt shader of the runtime window.
//!
//! The full contract is specified in `docs/book/specs/lod.md`.
//!
//! # Example
//!
//! ```
//! use glam::Vec3;
//! use planet_crafter_engine::lod::{LodConfig, LodScheduler};
//! use planet_crafter_engine::node::{build_icosphere, destroy_mesh};
//!
//! let mesh = build_icosphere("planet", 300.0, 0, Vec3::ZERO);
//! let mut scheduler = LodScheduler::new(LodConfig::default(), mesh.faces.clone()).unwrap();
//!
//! // A player far beyond every threshold: no split or merge, but the
//! // active zone and its minimum are maintained.
//! let report = scheduler.update(Vec3::new(100_000.0, 0.0, 0.0));
//! assert!(report.splits.is_empty());
//! assert!(report.merges.is_empty());
//! assert_eq!(scheduler.active_chunks().len(), 20);
//! destroy_mesh(&scheduler.active_chunks()[0]);
//! ```

mod config;
mod scheduler;

pub use config::{LodConfig, LodConfigError};
pub use scheduler::{BorderState, FrameReport, LodScheduler, border_states};
