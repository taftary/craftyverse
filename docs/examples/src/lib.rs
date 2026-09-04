//! Compile-tested examples for the architecture handbook.
//!
//! Each module demonstrates one boundary or rule from `docs/book`. The
//! matching binary in `examples/` calls the demonstration function so the
//! example is also runnable with `cargo run --example <name>`.

#![warn(missing_docs)]

pub mod engine_api;
pub mod message_flow;
pub mod renderer_neutral_scene;
pub mod state_transitions;
pub mod validated_resource;
