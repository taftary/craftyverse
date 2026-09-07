//! Winit event handling: owns the scenarios, the display options, the orbit
//! camera and the cursor, creates the window and drives the renderer in
//! response to window events.

use std::sync::Arc;

use glam::Vec2;
use vulkano::instance::Instance;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::scene::{Attribute, DisplayOptions, OrbitCamera, ViewMode};

use super::Scenario;
use super::renderer::Renderer;

/// Orbit speed of mouse drags, in radians per pixel.
const RADIANS_PER_PIXEL: f32 = 0.01;
/// Orbit step of the WASD keys, in radians (5°).
const KEY_ORBIT_STEP: f32 = std::f32::consts::PI / 36.0;
/// Zoom factor applied per mouse-wheel notch.
const WHEEL_ZOOM_FACTOR: f32 = 1.1;

pub(crate) struct Viewer {
    instance: Arc<Instance>,
    scenarios: Vec<Scenario>,
    current_scene: usize,
    /// Display state of the node attributes, toggled via the checkbox panel.
    options: DisplayOptions,
    /// Which visualization the world batches show, cycled with the T key.
    view_mode: ViewMode,
    /// Orbit camera of the 3D view, driven by mouse drag, wheel and WASD.
    camera: OrbitCamera,
    /// Last cursor position, in physical pixels.
    cursor: Vec2,
    /// Whether the left button is currently dragging an orbit (started
    /// outside the checkbox panel).
    dragging: bool,
    renderer: Option<Renderer>,
}

impl Viewer {
    pub(crate) fn new(instance: Arc<Instance>, scenarios: Vec<Scenario>) -> Self {
        Viewer {
            instance,
            scenarios,
            current_scene: 0,
            options: DisplayOptions::default(),
            view_mode: ViewMode::default(),
            camera: OrbitCamera::default(),
            cursor: Vec2::ZERO,
            dragging: false,
            renderer: None,
        }
    }

    /// Rebuilds the current scene's mesh and asks for a redraw — the shared
    /// tail of scene switch, split, reset and checkbox toggle.
    fn refresh_scene(&mut self) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let nodes = self.scenarios[self.current_scene].nodes();
        renderer.set_scene(&nodes, &self.options, self.view_mode);
        renderer.request_redraw();
    }

    /// Applies the current camera and asks for a redraw — the shared tail of
    /// drag, wheel and camera keys. The geometry buffers are untouched.
    fn refresh_camera(&mut self) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        renderer.set_camera(self.camera);
        renderer.request_redraw();
    }

    /// Flags the swapchain for recreation and asks for a redraw.
    fn on_resized(&mut self) {
        let renderer = self.renderer.as_mut().unwrap();
        renderer.mark_resized();
        renderer.request_redraw();
    }

    /// Left-click toggles the checkbox under the cursor; a left press
    /// anywhere else starts an orbit drag.
    fn on_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        if button != MouseButton::Left {
            return;
        }
        match state {
            ElementState::Pressed => {
                let Some(renderer) = self.renderer.as_mut() else {
                    return;
                };
                if let Some(attribute) = renderer.checkbox_at(self.cursor) {
                    self.options.toggle(attribute);
                    self.refresh_scene();
                } else {
                    self.dragging = true;
                }
            }
            ElementState::Released => self.dragging = false,
        }
    }

    /// Tracks the cursor; while dragging, orbits the camera with the delta.
    fn on_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let cursor = Vec2::new(position.x as f32, position.y as f32);
        let delta = cursor - self.cursor;
        self.cursor = cursor;
        if self.dragging {
            self.camera
                .orbit(-delta.x * RADIANS_PER_PIXEL, delta.y * RADIANS_PER_PIXEL);
            self.refresh_camera();
        }
    }

    /// Mouse wheel zooms the camera (one notch = ×1.1 magnification).
    fn on_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        let steps = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            // Trackpads report pixels; scale them to notch-like steps.
            MouseScrollDelta::PixelDelta(position) => position.y as f32 / 50.0,
        };
        if steps == 0.0 {
            return;
        }
        self.camera.zoom_by(WHEEL_ZOOM_FACTOR.powf(steps));
        self.refresh_camera();
    }

    /// Digit keys switch scenarios; arrow keys adjust icosphere scenarios;
    /// E/Q split and merge any scenario; T cycles the view mode; WASD
    /// orbits, R resets the camera.
    fn on_key_input(&mut self, event: KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }
        let key = event.physical_key;
        // Camera keys accept key repeat for a smooth orbit.
        let orbit_step = match key {
            PhysicalKey::Code(KeyCode::KeyW) => Some((0.0, -KEY_ORBIT_STEP)),
            PhysicalKey::Code(KeyCode::KeyS) => Some((0.0, KEY_ORBIT_STEP)),
            PhysicalKey::Code(KeyCode::KeyA) => Some((KEY_ORBIT_STEP, 0.0)),
            PhysicalKey::Code(KeyCode::KeyD) => Some((-KEY_ORBIT_STEP, 0.0)),
            _ => None,
        };
        if let Some((delta_yaw, delta_pitch)) = orbit_step {
            self.camera.orbit(delta_yaw, delta_pitch);
            self.refresh_camera();
            return;
        }
        if key == PhysicalKey::Code(KeyCode::KeyR) {
            self.camera.reset();
            self.refresh_camera();
            return;
        }
        if event.repeat {
            return;
        }
        if key == PhysicalKey::Code(KeyCode::KeyT) {
            self.view_mode = self.view_mode.next();
            self.refresh_scene();
            return;
        }
        if key == PhysicalKey::Code(KeyCode::KeyV) {
            self.options.toggle(Attribute::LinkViolations);
            self.refresh_scene();
            return;
        }
        if let Some(index) =
            scenario_index_of(&key, self.scenarios.len()).filter(|i| *i != self.current_scene)
        {
            self.current_scene = index;
            self.refresh_scene();
            return;
        }
        let scenario = &mut self.scenarios[self.current_scene];
        let changed = match key {
            PhysicalKey::Code(KeyCode::ArrowRight) => scenario.adjust_subdivisions(1),
            PhysicalKey::Code(KeyCode::ArrowLeft) => scenario.adjust_subdivisions(-1),
            PhysicalKey::Code(KeyCode::ArrowUp) => scenario.scale_radius(1.25),
            PhysicalKey::Code(KeyCode::ArrowDown) => scenario.scale_radius(0.8),
            PhysicalKey::Code(KeyCode::KeyH) => scenario.toggle_hemisphere_split(),
            PhysicalKey::Code(KeyCode::KeyE) => scenario.split(),
            PhysicalKey::Code(KeyCode::KeyQ) => scenario.unsplit(),
            _ => false,
        };
        if changed {
            self.refresh_scene();
        }
    }

    /// Draws the current scene.
    fn on_redraw_requested(&mut self) {
        let renderer = self.renderer.as_mut().unwrap();
        renderer.draw_frame(&self.scenarios[self.current_scene].nodes(), &self.options);
    }
}

impl ApplicationHandler for Viewer {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("PlanetCrafter node viewer — 1-N: scenes, arrows: sphere, E/Q: split/unsplit, T: view, drag/WASD: orbit, wheel: zoom, R: reset"),
                    )
                    .expect("failed to create window"),
            );
            let mut renderer = Renderer::new(self.instance.clone(), window);
            renderer.set_scene(
                &self.scenarios[self.current_scene].nodes(),
                &self.options,
                self.view_mode,
            );
            self.renderer = Some(renderer);
        }
        self.renderer.as_ref().unwrap().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.renderer.is_none() {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) => self.on_resized(),
            WindowEvent::CursorMoved { position, .. } => self.on_cursor_moved(position),
            WindowEvent::MouseInput { state, button, .. } => self.on_mouse_input(state, button),
            WindowEvent::MouseWheel { delta, .. } => self.on_mouse_wheel(delta),
            WindowEvent::KeyboardInput { event, .. } => self.on_key_input(event),
            WindowEvent::RedrawRequested => self.on_redraw_requested(),
            _ => {}
        }
    }
}

/// Scenario index selected by a digit key (Digit1 → 0, Digit2 → 1, …), or
/// `None` for other keys and out-of-range digits (`count` scenarios).
pub fn scenario_index_of(key: &PhysicalKey, count: usize) -> Option<usize> {
    let index = match key {
        PhysicalKey::Code(KeyCode::Digit1) => Some(0),
        PhysicalKey::Code(KeyCode::Digit2) => Some(1),
        PhysicalKey::Code(KeyCode::Digit3) => Some(2),
        PhysicalKey::Code(KeyCode::Digit4) => Some(3),
        _ => None,
    };
    index.filter(|i| *i < count)
}
