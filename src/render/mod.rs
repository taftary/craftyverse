//! Vulkan debug viewer: opens a window and renders the node scenes generated
//! by `scene`. All Vulkan and winit code lives in this module; the rest of
//! the crate stays GPU-independent.
//!
//! Rendering model: one render pass, three pipelines — colored lines,
//! colored triangles (arrowheads, center dots) and alpha-blended text quads
//! sampled from the `text` glyph atlas. World-space geometry is placed via
//! a `scale`/`offset` push-constant transform computed by `scene`; text and
//! the checkbox panel are laid out in pixel space and mapped with a second
//! transform. Left-clicking a checkbox toggles the display of the matching
//! node attribute.
//!
//! GPU vertex and push-constant layouts live in `vertices`, the shaders and
//! their runtime compilation in `shaders`, Vulkan object setup in `setup`,
//! the renderer in `renderer` and the winit event handling in `viewer`.

mod renderer;
mod setup;
mod shaders;
mod vertices;
mod viewer;

#[cfg(test)]
mod tests;

use winit::event_loop::{ControlFlow, EventLoop};

use crate::node::{collect_nodes, NodeRef};
use crate::plan::Plan;
use viewer::Viewer;

/// A viewer scenario: a fixed node list, or a live plan that can be
/// subdivided interactively.
pub enum Scenario {
    /// Fixed node list (not splittable).
    Static(Vec<NodeRef>),
    /// Live plan: `Plan::split()` subdivides it one level, `rebuild`
    /// regenerates its initial mesh.
    Plan { plan: Plan, rebuild: fn() -> Plan },
}

impl Scenario {
    /// Current node list to display (collected fresh from the plan when
    /// plan-backed).
    pub(crate) fn nodes(&self) -> Vec<NodeRef> {
        match self {
            Scenario::Static(nodes) => nodes.clone(),
            Scenario::Plan { plan, .. } => {
                collect_nodes(plan.root_node.as_ref().expect("generated plan"))
            }
        }
    }
}

/// Opens the viewer window and runs the event loop. Keys 1..N switch between
/// the given scenarios, S subdivides the current plan-backed scenario one
/// level and R regenerates its initial mesh; left-clicking the checkbox panel
/// toggles the display of each node attribute; the window closes the loop.
pub fn run(scenarios: Vec<Scenario>) {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let instance = setup::create_instance(&event_loop);

    let mut viewer = Viewer::new(instance, scenarios);
    event_loop.run_app(&mut viewer).expect("event loop error");
}
