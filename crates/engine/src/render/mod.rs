//! Vulkan debug viewer.
//!
//! This module opens a window and renders the node scenes produced by the
//! `scene` module. All Vulkan ([`vulkano`](https://crates.io/crates/vulkano))
//! and windowing ([`winit`](https://crates.io/crates/winit)) code lives here;
//! the rest of the crate stays GPU-independent.
//!
//! Rendering model: one render pass, three pipelines — colored lines, colored
//! triangles (arrowheads, center dots), and alpha-blended text quads sampled
//! from the `text` glyph atlas. World-space geometry is placed via a
//! `scale`/`offset` push-constant transform computed by `scene`; text and the
//! checkbox panel are laid out in pixel space and mapped with a second
//! transform. Left-clicking a checkbox toggles the display of the matching node
//! attribute.
//!
//! GPU vertex and push-constant layouts live in `vertices`, the shaders and
//! their runtime compilation in `shaders`, Vulkan object setup in `setup`, the
//! renderer in `renderer`, and the winit event handling in `viewer`.
//!
//! The full contract is specified in `docs/book/specs/render.md`.

mod renderer;
mod setup;
mod shaders;
mod vertices;
mod viewer;

#[cfg(test)]
mod tests;

use winit::event_loop::{ControlFlow, EventLoop};

use crate::icosahedron_plan::IcosahedronPlan;
use crate::node::{NodeRef, collect_nodes};
use viewer::Viewer;

/// A viewer scenario: a fixed node list, or a live plan that can be
/// subdivided interactively.
pub enum Scenario {
    /// Fixed node list. Not splittable; displayed as-is.
    Static(Vec<NodeRef>),
    /// Live plan scenario.
    ///
    /// `IcosahedronPlan::split()` subdivides the plan one level when the user presses **S**.
    /// `rebuild` regenerates the plan's initial mesh when the user presses **R**.
    Plan {
        /// Current plan state; mutated by `IcosahedronPlan::split()` and reset by `rebuild`.
        plan: IcosahedronPlan,
        /// Function that returns the plan's initial mesh.
        rebuild: fn() -> IcosahedronPlan,
    },
}

impl Scenario {
    /// Current node list to display.
    ///
    /// For a static scenario this returns the stored nodes. For a plan scenario
    /// it collects the nodes reachable from the plan's current `root_node`.
    pub fn nodes(&self) -> Vec<NodeRef> {
        match self {
            Scenario::Static(nodes) => nodes.clone(),
            Scenario::Plan { plan, .. } => {
                collect_nodes(plan.root_node.as_ref().expect("generated plan"))
            }
        }
    }
}

/// Opens the viewer window and runs the event loop.
///
/// # Interaction
///
/// - **Number keys 1..N** — switch between scenarios.
/// - **S** — split the current `Scenario::Plan` one level.
/// - **R** — regenerate the current `Scenario::Plan` via its `rebuild` function.
/// - **Left click** — toggle the display attribute of the clicked checkbox.
/// - **Close window** — exit the event loop.
///
/// # Panics
///
/// Panics if the winit event loop or the Vulkan instance cannot be created.
///
/// # Platform notes
///
/// This function requires a Vulkan-capable GPU/driver and opens a window. It
/// cannot be used in headless tests.
pub fn run(scenarios: Vec<Scenario>) {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let instance = setup::create_instance(&event_loop);

    let mut viewer = Viewer::new(instance, scenarios);
    event_loop.run_app(&mut viewer).expect("event loop error");
}
