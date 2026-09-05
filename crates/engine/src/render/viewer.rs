//! Winit event handling: owns the scenarios, the display options and the
//! cursor, creates the window and drives the renderer in response to window
//! events.

use std::sync::Arc;

use glam::Vec2;
use vulkano::instance::Instance;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::scene::DisplayOptions;

use super::Scenario;
use super::renderer::Renderer;

pub(crate) struct Viewer {
    instance: Arc<Instance>,
    scenarios: Vec<Scenario>,
    current_scene: usize,
    /// Display state of the node attributes, toggled via the checkbox panel.
    options: DisplayOptions,
    /// Last cursor position, in physical pixels.
    cursor: Vec2,
    renderer: Option<Renderer>,
}

impl Viewer {
    pub(crate) fn new(instance: Arc<Instance>, scenarios: Vec<Scenario>) -> Self {
        Viewer {
            instance,
            scenarios,
            current_scene: 0,
            options: DisplayOptions::default(),
            cursor: Vec2::ZERO,
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
        renderer.set_scene(&nodes, &self.options);
        renderer.request_redraw();
    }

    /// Flags the swapchain for recreation and asks for a redraw.
    fn on_resized(&mut self) {
        let renderer = self.renderer.as_mut().unwrap();
        renderer.mark_resized();
        renderer.request_redraw();
    }

    /// Left-click toggles the checkbox under the cursor.
    fn on_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        if state != ElementState::Pressed || button != MouseButton::Left {
            return;
        }
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        if let Some(attribute) = renderer.checkbox_at(self.cursor) {
            self.options.toggle(attribute);
            self.refresh_scene();
        }
    }

    /// Digit keys switch scenarios.
    fn on_key_input(&mut self, event: KeyEvent) {
        if event.state != ElementState::Pressed || event.repeat {
            return;
        }
        let key = event.physical_key;
        if let Some(index) =
            scenario_index_of(&key, self.scenarios.len()).filter(|i| *i != self.current_scene)
        {
            self.current_scene = index;
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
                            .with_title("PlanetCrafter node viewer — 1-N: scenes"),
                    )
                    .expect("failed to create window"),
            );
            let mut renderer = Renderer::new(self.instance.clone(), window);
            renderer.set_scene(&self.scenarios[self.current_scene].nodes(), &self.options);
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
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Vec2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput { state, button, .. } => self.on_mouse_input(state, button),
            WindowEvent::KeyboardInput { event, .. } => self.on_key_input(event),
            WindowEvent::RedrawRequested => self.on_redraw_requested(),
            _ => {}
        }
    }
}

/// Scenario index selected by a digit key (Digit1 → 0, Digit2 → 1, …), or
/// `None` for other keys and out-of-range digits (`count` scenarios).
pub(crate) fn scenario_index_of(key: &PhysicalKey, count: usize) -> Option<usize> {
    let index = match key {
        PhysicalKey::Code(KeyCode::Digit1) => Some(0),
        PhysicalKey::Code(KeyCode::Digit2) => Some(1),
        PhysicalKey::Code(KeyCode::Digit3) => Some(2),
        PhysicalKey::Code(KeyCode::Digit4) => Some(3),
        _ => None,
    };
    index.filter(|i| *i < count)
}
