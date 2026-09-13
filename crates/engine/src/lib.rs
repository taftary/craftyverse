//! PlanetCrafter engine crate: reusable geometry, topology, renderer-neutral
//! scene data, text layout, the headless planet runtime manager, LOD
//! scheduler and visibility culling, and the Vulkan debug viewer.

#![warn(missing_docs)]

pub mod lod;
pub mod node;
pub mod render;
pub mod runtime;
pub mod scene;
pub mod text;
pub mod visibility;

#[cfg(feature = "test-internals")]
pub mod testing;
