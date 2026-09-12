//! Vulkan debug viewer.
//!
//! This module opens a window and renders the node scenes produced by the
//! `scene` module. All Vulkan ([`vulkano`](https://crates.io/crates/vulkano))
//! and windowing ([`winit`](https://crates.io/crates/winit)) code lives here;
//! the rest of the crate stays GPU-independent.
//!
//! Rendering model: one render pass with a depth buffer, six pipelines —
//! colored world lines, colored world triangles (arrowheads, discs),
//! textured world triangles (`tex_pipeline`, procedural per-triangle
//! effects in the `Textured` view and the generated `checkerboard` texture
//! in the `UvMap` view, switched by a push-constant fragment mode), the
//! pixel-space checkbox panel (lines and triangles, no depth test), and
//! alpha-blended text quads sampled from the `text` glyph atlas. The world
//! batches depend on the view mode (the T
//! key cycles them): `Mesh` draws the attribute/line debug view, `Textured`
//! the 3D node triangles shaded with the selected procedural effect
//! (barycentric coordinates, topology parity and radial direction, never
//! UVs), `UvMap` the
//! checkerboard-textured UV net laid flat plus its wireframe overlay.
//! World-space geometry stays 3D and is placed
//! on the GPU via a view-projection matrix push constant driven by the orbit
//! camera, so camera changes never rebuild the geometry buffers; only the
//! label anchors are re-projected on the CPU. Text and the checkbox panel
//! are laid out in pixel space and mapped with separate transforms.
//! Left-clicking a checkbox toggles the display of the matching node
//! attribute; left-clicking an `fx` radio row selects the procedural effect.
//!
//! GPU vertex and push-constant layouts live in `vertices`, the shaders and
//! their runtime compilation in `shaders`, the checkerboard debug texture in
//! `checkerboard`, the CPU reference of the procedural texture effects in
//! `procedural`, the reusable scene vertex buffers in `buffers`, Vulkan
//! object setup in `setup`, the renderer in `renderer`, and the winit event
//! handling in `viewer`.
//!
//! The full contract is specified in `docs/book/specs/render.md`.

mod buffers;
mod checkerboard;
#[cfg(feature = "test-internals")]
mod procedural;
mod renderer;
mod setup;
mod shaders;
mod vertices;
mod viewer;

use winit::event_loop::{ControlFlow, EventLoop};

use glam::Vec3;

use crate::node::{
    IcosphereMesh, NodeRef, build_icosphere, destroy_mesh, split_nodes, unsplit_nodes,
};
use viewer::Viewer;

#[cfg(feature = "test-internals")]
pub use buffers::required_capacity;
#[cfg(feature = "test-internals")]
pub use checkerboard::{
    CHECKER_HEIGHT, CHECKER_WIDTH, CHECKS_U, CHECKS_V, checkerboard_mips, mip_level_count,
};
#[cfg(feature = "test-internals")]
pub use procedural::{
    CHECKER_CELLS, LATITUDE_BANDS, LIGHT_DIR, MASK_EDGE_WIDTH, STRIPE_BANDS, checker, diffuse,
    edge_mask, fresnel, gradient, latitude, radial_rgb, stripes,
};
#[cfg(feature = "test-internals")]
pub use renderer::panel_item_at;
#[cfg(feature = "test-internals")]
pub use renderer::pixel_matrix;
#[cfg(feature = "test-internals")]
pub use setup::device_type_rank;
#[cfg(feature = "test-internals")]
pub use shaders::{GEOM_FRAG, GEOM_VERT, TEX_FRAG, TEX_VERT, TEXT_FRAG, TEXT_VERT, compile_spirv};
#[cfg(feature = "test-internals")]
pub use vertices::{PushMatrix, PushTex, PushTransform};
#[cfg(feature = "test-internals")]
pub use viewer::scenario_index_of;

/// Maximum subdivision level an [`Scenario::Icosphere`] can be adjusted to
/// from the viewer (bounds the per-rebuild CPU and GPU upload cost).
pub const MAX_ICOSPHERE_SUBDIVISIONS: u32 = 5;

/// A viewer scenario: a fixed node list or a parametric icosphere.
pub enum Scenario {
    /// Nodes displayed as-is.
    Static(Vec<NodeRef>),
    /// A geodesic sphere rebuilt from its parameters, adjustable at runtime.
    Icosphere(IcosphereConfig),
}

/// Parameters and current mesh of an [`Scenario::Icosphere`].
///
/// The mesh is rebuilt (and the previous generation graph destroyed) whenever
/// the parameters change through [`Scenario::adjust_subdivisions`] or
/// [`Scenario::scale_radius`].
pub struct IcosphereConfig {
    name_prefix: String,
    radius: f32,
    subdivisions: u32,
    origin: Vec3,
    /// Whether the north/south hemispheres are currently displayed apart.
    hemisphere_split: bool,
    mesh: IcosphereMesh,
}

impl Scenario {
    /// Creates an icosphere scenario from its construction parameters.
    pub fn icosphere(
        name_prefix: impl Into<String>,
        radius: f32,
        subdivisions: u32,
        origin: Vec3,
    ) -> Self {
        let name_prefix = name_prefix.into();
        let mesh = build_icosphere(&name_prefix, radius, subdivisions, origin);
        Scenario::Icosphere(IcosphereConfig {
            name_prefix,
            radius,
            subdivisions,
            origin,
            hemisphere_split: false,
            mesh,
        })
    }

    /// Current node list to display.
    ///
    /// Returns the stored nodes.
    pub fn nodes(&self) -> Vec<NodeRef> {
        match self {
            Scenario::Static(nodes) => nodes.clone(),
            Scenario::Icosphere(config) => config.mesh.faces.clone(),
        }
    }

    /// Current radius of an icosphere scenario, `None` for static scenarios.
    pub fn radius(&self) -> Option<f32> {
        match self {
            Scenario::Static(_) => None,
            Scenario::Icosphere(config) => Some(config.radius),
        }
    }

    /// Current subdivision level of an icosphere scenario, `None` for static
    /// scenarios.
    pub fn subdivisions(&self) -> Option<u32> {
        match self {
            Scenario::Static(_) => None,
            Scenario::Icosphere(config) => Some(config.subdivisions),
        }
    }

    /// Adjusts the subdivision level by `delta` (clamped to
    /// `0..=MAX_ICOSPHERE_SUBDIVISIONS`) and rebuilds the mesh. Returns
    /// `true` if the level changed; always `false` for static scenarios.
    pub fn adjust_subdivisions(&mut self, delta: i32) -> bool {
        let Scenario::Icosphere(config) = self else {
            return false;
        };
        let new_level =
            (config.subdivisions as i32 + delta).clamp(0, MAX_ICOSPHERE_SUBDIVISIONS as i32) as u32;
        if new_level == config.subdivisions {
            return false;
        }
        config.subdivisions = new_level;
        config.rebuild();
        true
    }

    /// Scales the radius by `factor` (clamped away from degenerate sizes)
    /// and rebuilds the mesh. Returns `true` if the radius changed; always
    /// `false` for static scenarios.
    pub fn scale_radius(&mut self, factor: f32) -> bool {
        let Scenario::Icosphere(config) = self else {
            return false;
        };
        let new_radius = (config.radius * factor).clamp(1.0, 1e6);
        if new_radius == config.radius {
            return false;
        }
        config.radius = new_radius;
        config.rebuild();
        true
    }

    /// Whether the icosphere scenario currently displays its north and
    /// south hemispheres side by side; `None` for static scenarios.
    pub fn hemisphere_split(&self) -> Option<bool> {
        match self {
            Scenario::Static(_) => None,
            Scenario::Icosphere(config) => Some(config.hemisphere_split),
        }
    }

    /// Splits the whole scenario one generation deeper. Static scenarios
    /// split and re-weld their connected mesh in place (see
    /// [`split_nodes`]); icosphere scenarios rebuild with one more
    /// subdivision level so the new midpoints stay projected on the sphere
    /// (same as the Right-arrow key). Returns `true` if the scenario
    /// changed; `false` for an empty static scenario or a clamped level.
    pub fn split(&mut self) -> bool {
        match self {
            Scenario::Static(nodes) => {
                if nodes.is_empty() {
                    return false;
                }
                *nodes = split_nodes(&nodes[0]);
                true
            }
            Scenario::Icosphere(_) => self.adjust_subdivisions(1),
        }
    }

    /// Merges the scenario one generation back — the reverse of
    /// [`Scenario::split`]. Static scenarios group their split nodes back
    /// into their parents (see [`unsplit_nodes`]); icosphere scenarios
    /// rebuild with one less subdivision level (same as the Left-arrow
    /// key). Returns `true` if the scenario changed; `false` when there is
    /// nothing to merge (base mesh, empty scenario, or clamped level).
    pub fn unsplit(&mut self) -> bool {
        match self {
            Scenario::Static(nodes) => {
                if nodes.is_empty() {
                    return false;
                }
                let merged = unsplit_nodes(&nodes[0]);
                if merged.len() == nodes.len() {
                    return false;
                }
                *nodes = merged;
                true
            }
            Scenario::Icosphere(_) => self.adjust_subdivisions(-1),
        }
    }

    /// Toggles the hemisphere split display: the north half (face centers at
    /// `z >= origin.z`, i.e. facing the viewer in the XY projection) is moved
    /// right and the south half left so the two halves can be inspected
    /// without overlap. Returns `true` if the state changed; always `false`
    /// for static scenarios.
    pub fn toggle_hemisphere_split(&mut self) -> bool {
        let Scenario::Icosphere(config) = self else {
            return false;
        };
        config.hemisphere_split = !config.hemisphere_split;
        let sign = if config.hemisphere_split { 1.0 } else { -1.0 };
        config.shift_hemispheres(sign);
        true
    }
}

impl IcosphereConfig {
    /// Destroys the current mesh graph and rebuilds it from the parameters.
    fn rebuild(&mut self) {
        destroy_mesh(&self.mesh.faces[0]);
        self.mesh = build_icosphere(
            &self.name_prefix,
            self.radius,
            self.subdivisions,
            self.origin,
        );
        if self.hemisphere_split {
            self.shift_hemispheres(1.0);
        }
    }

    /// Horizontal separation applied to each hemisphere when split.
    fn hemisphere_gap(&self) -> f32 {
        1.2 * self.radius
    }

    /// Translates every face by `sign * gap` along X — north faces
    /// (`center.z >= origin.z`) right, south faces left. The gap scales with
    /// the radius so the halves stay separated when the radius changes.
    fn shift_hemispheres(&mut self, sign: f32) {
        let gap = sign * self.hemisphere_gap();
        for face in &self.mesh.faces {
            let north = face.borrow().center.z >= self.origin.z;
            translate_node(face, Vec3::new(if north { gap } else { -gap }, 0.0, 0.0));
        }
    }
}

/// Moves a node by `delta`, keeping its derived geometry consistent:
/// `vertices` and `center` shift, `direction_to_origin` (`origin - center`)
/// compensates, and the normalized directions are translation-invariant.
fn translate_node(node: &NodeRef, delta: Vec3) {
    let mut node = node.borrow_mut();
    node.vertices = node.vertices.map(|p| p + delta);
    node.center += delta;
    node.direction_to_origin -= delta;
}

impl Drop for IcosphereConfig {
    fn drop(&mut self) {
        destroy_mesh(&self.mesh.faces[0]);
    }
}

/// Opens the viewer window and runs the event loop.
///
/// # Interaction
///
/// - **Number keys 1..N** — switch between scenarios.
/// - **E / Q** — split the whole scene one generation deeper / merge it
///   back (all scenarios; icosphere scenarios rebuild like the arrow keys).
/// - **Left / Right arrows** — decrease / increase the icosphere subdivision
///   level (icosphere scenarios only).
/// - **Down / Up arrows** — shrink / grow the icosphere radius (icosphere
///   scenarios only).
/// - **H** — toggle the north/south hemisphere split display (icosphere
///   scenarios only).
/// - **T** — cycle the view: mesh attributes → textured 3D → UV map.
/// - **V** — toggle the broken-link highlight (same as
///   the "link violations" checkbox).
/// - **Left drag** (outside the checkbox panel) or **W / A / S / D** — orbit
///   the 3D camera around the scene.
/// - **Mouse wheel** — zoom the camera.
/// - **R** — reset the camera to the head-on view.
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
