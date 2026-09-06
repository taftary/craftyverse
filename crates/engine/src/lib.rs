//! PlanetCrafter engine crate: reusable geometry, topology, renderer-neutral
//! scene data, text layout, and the Vulkan debug viewer.

#![warn(missing_docs)]

pub mod node;
pub mod render;
pub mod scene;
pub mod text;

#[cfg(feature = "test-internals")]
pub mod testing;
