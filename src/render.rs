//! Vulkan debug viewer: opens a window and renders the node scenes generated
//! by `scene.rs`. All Vulkan and winit code lives in this module; the rest of
//! the crate stays GPU-independent.
//!
//! Rendering model: one render pass, three pipelines — colored lines,
//! colored triangles (arrowheads, center dots) and alpha-blended text quads
//! sampled from the `text.rs` glyph atlas. World-space geometry is placed via
//! a `scale`/`offset` push-constant transform computed by `scene.rs`; text and
//! the checkbox panel are laid out in pixel space and mapped with a second
//! transform. Left-clicking a checkbox toggles the display of the matching
//! node attribute.

use std::sync::Arc;

use glam::Vec2;
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo, RenderPassBeginInfo,
    SubpassBeginInfo, SubpassEndInfo,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::format::Format;
use vulkano::image::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::memory::allocator::{
    AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator,
};
use vulkano::pipeline::graphics::color_blend::{
    AttachmentBlend, ColorBlendAttachmentState, ColorBlendState,
};
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{
    Vertex, VertexBufferDescription, VertexDefinition,
};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout,
    PipelineShaderStageCreateInfo,
};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::shader::{EntryPoint, ShaderModule, ShaderModuleCreateInfo};
use vulkano::swapchain::{
    acquire_next_image, PresentMode, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo,
};
use vulkano::sync::{self, GpuFuture};
use vulkano::{Validated, VulkanError, VulkanLibrary};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::node::NodeRef;
use crate::scene::{self, Checkbox, DisplayOptions, SceneMesh};
use crate::text::TextAtlas;

#[derive(BufferContents, Vertex, Clone, Copy)]
#[repr(C)]
struct GeomVertex {
    #[format(R32G32_SFLOAT)]
    pos: [f32; 2],
    #[format(R32G32B32_SFLOAT)]
    color: [f32; 3],
}

#[derive(BufferContents, Vertex, Clone, Copy)]
#[repr(C)]
struct TextVertexGpu {
    #[format(R32G32_SFLOAT)]
    pos: [f32; 2],
    #[format(R32G32_SFLOAT)]
    uv: [f32; 2],
    #[format(R32G32B32_SFLOAT)]
    color: [f32; 3],
}

#[derive(BufferContents, Clone, Copy)]
#[repr(C)]
struct PushTransform {
    scale: [f32; 2],
    offset: [f32; 2],
}

const GEOM_VERT: &str = r#"
#version 450
layout(location = 0) in vec2 pos;
layout(location = 1) in vec3 color;
layout(location = 0) out vec3 out_color;
layout(push_constant) uniform PushTransform { vec2 scale; vec2 offset; } pc;
void main() {
    gl_Position = vec4(pos * pc.scale + pc.offset, 0.0, 1.0);
    out_color = color;
}
"#;

const GEOM_FRAG: &str = r#"
#version 450
layout(location = 0) in vec3 color;
layout(location = 0) out vec4 out_color;
void main() {
    out_color = vec4(color, 1.0);
}
"#;

const TEXT_VERT: &str = r#"
#version 450
layout(location = 0) in vec2 pos;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec3 color;
layout(location = 0) out vec2 out_uv;
layout(location = 1) out vec3 out_color;
layout(push_constant) uniform PushTransform { vec2 scale; vec2 offset; } pc;
void main() {
    gl_Position = vec4(pos * pc.scale + pc.offset, 0.0, 1.0);
    out_uv = uv;
    out_color = color;
}
"#;

const TEXT_FRAG: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 1) in vec3 color;
layout(location = 0) out vec4 out_color;
// Note: naga's GLSL frontend supports neither `layout(set = ...)` (resources
// default to set 0) nor combined `sampler2D` uniforms, so the glyph atlas is
// bound as a separate texture and sampler.
layout(binding = 0) uniform texture2D atlas_texture;
layout(binding = 1) uniform sampler atlas_sampler;
void main() {
    out_color = vec4(color, texture(sampler2D(atlas_texture, atlas_sampler), uv).r);
}
"#;

/// Opens the viewer window and runs the event loop. Keys 1..N switch between
/// the given scenarios, left-clicking the checkbox panel toggles the display
/// of each node attribute; the window closes the loop.
pub fn run(scenarios: Vec<Vec<NodeRef>>) {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let library = VulkanLibrary::new().expect("no local Vulkan library");
    let required_extensions =
        Surface::required_extensions(&event_loop).expect("failed to query surface extensions");
    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
            enabled_extensions: required_extensions,
            ..Default::default()
        },
    )
    .expect("failed to create Vulkan instance");

    let mut viewer = Viewer {
        instance,
        scenarios,
        current_scene: 0,
        options: DisplayOptions::default(),
        cursor: Vec2::ZERO,
        renderer: None,
    };
    event_loop.run_app(&mut viewer).expect("event loop error");
}

struct Viewer {
    instance: Arc<Instance>,
    scenarios: Vec<Vec<NodeRef>>,
    current_scene: usize,
    /// Display state of the node attributes, toggled via the checkbox panel.
    options: DisplayOptions,
    /// Last cursor position, in physical pixels.
    cursor: Vec2,
    renderer: Option<Renderer>,
}

impl ApplicationHandler for Viewer {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_none() {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("PlanetCrafter node viewer — 1: node, 2: split, 3: base, 4: dual mesh"),
                    )
                    .expect("failed to create window"),
            );
            let mut renderer = Renderer::new(self.instance.clone(), window);
            renderer.set_scene(&self.scenarios[self.current_scene], &self.options);
            self.renderer = Some(renderer);
        }
        self.renderer.as_ref().unwrap().window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) => {
                renderer.window_resized = true;
                renderer.window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Vec2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput { state, button, .. }
                if state == ElementState::Pressed && button == MouseButton::Left =>
            {
                if let Some(attribute) = renderer.checkbox_at(self.cursor) {
                    self.options.toggle(attribute);
                    renderer.set_scene(&self.scenarios[self.current_scene], &self.options);
                    renderer.window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && !event.repeat =>
            {
                let index = match event.physical_key {
                    PhysicalKey::Code(KeyCode::Digit1) => Some(0),
                    PhysicalKey::Code(KeyCode::Digit2) => Some(1),
                    PhysicalKey::Code(KeyCode::Digit3) => Some(2),
                    PhysicalKey::Code(KeyCode::Digit4) => Some(3),
                    _ => None,
                };
                if let Some(index) =
                    index.filter(|i| *i < self.scenarios.len() && *i != self.current_scene)
                {
                    self.current_scene = index;
                    renderer.set_scene(&self.scenarios[index], &self.options);
                    renderer.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                renderer.draw_frame(&self.scenarios[self.current_scene], &self.options)
            }
            _ => {}
        }
    }
}

struct Renderer {
    window: Arc<Window>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    queue_family_index: u32,
    swapchain: Arc<Swapchain>,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    line_pipeline: Arc<GraphicsPipeline>,
    tri_pipeline: Arc<GraphicsPipeline>,
    text_pipeline: Arc<GraphicsPipeline>,
    text_descriptor_set: Arc<DescriptorSet>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    atlas: TextAtlas,
    line_buffer: Option<Subbuffer<[GeomVertex]>>,
    tri_buffer: Option<Subbuffer<[GeomVertex]>>,
    ui_line_buffer: Option<Subbuffer<[GeomVertex]>>,
    ui_tri_buffer: Option<Subbuffer<[GeomVertex]>>,
    text_buffer: Option<Subbuffer<[TextVertexGpu]>>,
    /// Checkbox hit rectangles of the current scene (pixel space).
    checkboxes: Vec<Checkbox>,
    world_to_clip: PushTransform,
    pixel_to_clip: PushTransform,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    window_resized: bool,
}

impl Renderer {
    fn new(instance: Arc<Instance>, window: Arc<Window>) -> Self {
        let surface =
            Surface::from_window(instance.clone(), window.clone()).expect("failed to create surface");
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .expect("failed to enumerate physical devices")
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.intersects(QueueFlags::GRAPHICS)
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                _ => 4,
            })
            .expect("no suitable Vulkan physical device found");
        println!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .expect("failed to create device");
        let queue = queues.next().unwrap();

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        ));
        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let capabilities = physical_device
            .surface_capabilities(&surface, Default::default())
            .expect("failed to query surface capabilities");
        let composite_alpha = capabilities
            .supported_composite_alpha
            .into_iter()
            .next()
            .unwrap();
        let image_format = physical_device
            .surface_formats(&surface, Default::default())
            .expect("failed to query surface formats")[0]
            .0;
        let (swapchain, images) = Swapchain::new(
            device.clone(),
            surface,
            SwapchainCreateInfo {
                min_image_count: capabilities.min_image_count.max(2),
                image_format,
                image_extent: window.inner_size().into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT,
                composite_alpha,
                present_mode: PresentMode::Fifo,
                ..Default::default()
            },
        )
        .expect("failed to create swapchain");

        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    format: swapchain.image_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
            },
            pass: { color: [color], depth_stencil: {} },
        )
        .expect("failed to create render pass");
        let framebuffers = window_size_dependent_setup(&images, &render_pass);

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        let geom_vs = load_shader(&device, GEOM_VERT, naga::ShaderStage::Vertex);
        let geom_fs = load_shader(&device, GEOM_FRAG, naga::ShaderStage::Fragment);
        let text_vs = load_shader(&device, TEXT_VERT, naga::ShaderStage::Vertex);
        let text_fs = load_shader(&device, TEXT_FRAG, naga::ShaderStage::Fragment);
        let line_pipeline = graphics_pipeline(
            &device,
            geom_vs.clone(),
            geom_fs.clone(),
            GeomVertex::per_vertex(),
            PrimitiveTopology::LineList,
            None,
            &subpass,
        );
        let tri_pipeline = graphics_pipeline(
            &device,
            geom_vs,
            geom_fs,
            GeomVertex::per_vertex(),
            PrimitiveTopology::TriangleList,
            None,
            &subpass,
        );
        let text_pipeline = graphics_pipeline(
            &device,
            text_vs,
            text_fs,
            TextVertexGpu::per_vertex(),
            PrimitiveTopology::TriangleList,
            Some(AttachmentBlend::alpha()),
            &subpass,
        );

        // Rasterize the glyph atlas and upload it as an R8 texture.
        let atlas = TextAtlas::new(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"))
            .expect("failed to load bundled font");
        let atlas_image = Image::new(
            memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8_UNORM,
                extent: [atlas.width, atlas.height, 1],
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .expect("failed to create atlas image");
        let staging = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            atlas.pixels.iter().copied(),
        )
        .expect("failed to create staging buffer");
        let mut upload = AutoCommandBufferBuilder::primary(
            command_buffer_allocator.clone(),
            queue_family_index,
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
        upload
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(
                staging,
                atlas_image.clone(),
            ))
            .unwrap();
        sync::now(device.clone())
            .then_execute(queue.clone(), upload.build().unwrap())
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();

        let sampler = Sampler::new(
            device.clone(),
            SamplerCreateInfo {
                mag_filter: Filter::Linear,
                min_filter: Filter::Linear,
                address_mode: [SamplerAddressMode::ClampToEdge; 3],
                ..Default::default()
            },
        )
        .unwrap();
        let atlas_view = ImageView::new_default(atlas_image).unwrap();
        let text_layout = text_pipeline.layout().set_layouts().first().unwrap().clone();
        let text_descriptor_set = DescriptorSet::new(
            descriptor_set_allocator.clone(),
            text_layout,
            [
                WriteDescriptorSet::image_view(0, atlas_view),
                WriteDescriptorSet::sampler(1, sampler),
            ],
            [],
        )
        .unwrap();

        Renderer {
            window,
            device: device.clone(),
            queue,
            queue_family_index,
            swapchain,
            render_pass,
            framebuffers,
            line_pipeline,
            tri_pipeline,
            text_pipeline,
            text_descriptor_set,
            memory_allocator,
            command_buffer_allocator,
            atlas,
            line_buffer: None,
            tri_buffer: None,
            ui_line_buffer: None,
            ui_tri_buffer: None,
            text_buffer: None,
            checkboxes: Vec::new(),
            world_to_clip: PushTransform {
                scale: [1.0; 2],
                offset: [0.0; 2],
            },
            pixel_to_clip: PushTransform {
                scale: [1.0; 2],
                offset: [0.0; 2],
            },
            previous_frame_end: Some(sync::now(device).boxed()),
            window_resized: false,
        }
    }

    /// Rebuilds the scene mesh for `scenario` and re-uploads all vertex
    /// buffers. Cheap enough to run on scene switch, on resize and on
    /// checkbox toggle.
    fn set_scene(&mut self, scenario: &[NodeRef], options: &DisplayOptions) {
        let size = self.window.inner_size();
        let viewport = Vec2::new(size.width as f32, size.height as f32);
        let mesh = scene::build_scene(scenario, viewport, options);

        let to_geom = |v: &scene::Vertex| GeomVertex {
            pos: v.pos.to_array(),
            color: v.color,
        };
        self.line_buffer = vertex_buffer(
            &self.memory_allocator,
            mesh.lines.iter().map(to_geom).collect(),
        );
        self.tri_buffer = vertex_buffer(
            &self.memory_allocator,
            mesh.triangles.iter().map(to_geom).collect(),
        );
        self.ui_line_buffer = vertex_buffer(
            &self.memory_allocator,
            mesh.ui_lines.iter().map(to_geom).collect(),
        );
        self.ui_tri_buffer = vertex_buffer(
            &self.memory_allocator,
            mesh.ui_triangles.iter().map(to_geom).collect(),
        );

        let mut text_data = Vec::new();
        for run in &mesh.texts {
            text_data.extend(self.atlas.layout(
                &run.text,
                run.anchor,
                run.size,
                run.color,
                run.centered,
            ));
        }
        self.text_buffer = vertex_buffer(
            &self.memory_allocator,
            text_data
                .into_iter()
                .map(|v| TextVertexGpu {
                    pos: v.pos.to_array(),
                    uv: v.uv.to_array(),
                    color: v.color,
                })
                .collect(),
        );

        let SceneMesh {
            world_to_clip,
            pixel_to_clip,
            checkboxes,
            ..
        } = mesh;
        self.checkboxes = checkboxes;
        self.world_to_clip = PushTransform {
            scale: world_to_clip.scale.to_array(),
            offset: world_to_clip.offset.to_array(),
        };
        self.pixel_to_clip = PushTransform {
            scale: pixel_to_clip.scale.to_array(),
            offset: pixel_to_clip.offset.to_array(),
        };
    }

    /// Attribute whose checkbox contains `point` (physical pixels), if any.
    fn checkbox_at(&self, point: Vec2) -> Option<scene::Attribute> {
        self.checkboxes
            .iter()
            .find(|checkbox| checkbox.contains(point))
            .map(|checkbox| checkbox.attribute)
    }

    fn draw_frame(&mut self, scenario: &[NodeRef], options: &DisplayOptions) {
        let window_size = self.window.inner_size();
        if window_size.width == 0 || window_size.height == 0 {
            return;
        }
        if self.window_resized {
            self.window_resized = false;
            self.recreate_swapchain();
            // The view fit and the text anchors depend on the viewport.
            self.set_scene(scenario, options);
        }
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        let (image_index, suboptimal, acquire_future) =
            match acquire_next_image(self.swapchain.clone(), None).map_err(Validated::unwrap) {
                Ok(result) => result,
                Err(VulkanError::OutOfDate) => {
                    self.window_resized = true;
                    return;
                }
                Err(err) => panic!("failed to acquire next image: {err}"),
            };
        if suboptimal {
            self.window_resized = true;
        }

        let mut builder = AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.queue_family_index,
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([1.0, 1.0, 1.0, 1.0].into())],
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[image_index as usize].clone(),
                    )
                },
                SubpassBeginInfo::default(),
            )
            .unwrap()
            .set_viewport(
                0,
                [Viewport {
                    offset: [0.0; 2],
                    extent: [window_size.width as f32, window_size.height as f32],
                    depth_range: 0.0..=1.0,
                }]
                .into_iter()
                .collect(),
            )
            .unwrap();

        // World-space geometry: child links, outlines, arrow shafts, dashes…
        if let Some(lines) = &self.line_buffer {
            builder
                .bind_pipeline_graphics(self.line_pipeline.clone())
                .unwrap()
                .push_constants(self.line_pipeline.layout().clone(), 0, self.world_to_clip)
                .unwrap()
                .bind_vertex_buffers(0, lines.clone())
                .unwrap();
            // SAFETY: pipeline, vertex buffer and push constants bound above
            // satisfy the draw's requirements.
            unsafe { builder.draw(lines.len() as u32, 1, 0, 0) }.unwrap();
        }
        // …then filled triangles (arrowheads, dots) on top of the lines.
        if let Some(triangles) = &self.tri_buffer {
            builder
                .bind_pipeline_graphics(self.tri_pipeline.clone())
                .unwrap()
                .push_constants(self.tri_pipeline.layout().clone(), 0, self.world_to_clip)
                .unwrap()
                .bind_vertex_buffers(0, triangles.clone())
                .unwrap();
            // SAFETY: pipeline, vertex buffer and push constants bound above
            // satisfy the draw's requirements.
            unsafe { builder.draw(triangles.len() as u32, 1, 0, 0) }.unwrap();
        }
        // Checkbox panel, in pixel space.
        if let Some(ui_lines) = &self.ui_line_buffer {
            builder
                .bind_pipeline_graphics(self.line_pipeline.clone())
                .unwrap()
                .push_constants(self.line_pipeline.layout().clone(), 0, self.pixel_to_clip)
                .unwrap()
                .bind_vertex_buffers(0, ui_lines.clone())
                .unwrap();
            // SAFETY: pipeline, vertex buffer and push constants bound above
            // satisfy the draw's requirements.
            unsafe { builder.draw(ui_lines.len() as u32, 1, 0, 0) }.unwrap();
        }
        if let Some(ui_triangles) = &self.ui_tri_buffer {
            builder
                .bind_pipeline_graphics(self.tri_pipeline.clone())
                .unwrap()
                .push_constants(self.tri_pipeline.layout().clone(), 0, self.pixel_to_clip)
                .unwrap()
                .bind_vertex_buffers(0, ui_triangles.clone())
                .unwrap();
            // SAFETY: pipeline, vertex buffer and push constants bound above
            // satisfy the draw's requirements.
            unsafe { builder.draw(ui_triangles.len() as u32, 1, 0, 0) }.unwrap();
        }
        // …finally alpha-blended text in pixel space.
        if let Some(text) = &self.text_buffer {
            builder
                .bind_pipeline_graphics(self.text_pipeline.clone())
                .unwrap()
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    self.text_pipeline.layout().clone(),
                    0,
                    self.text_descriptor_set.clone(),
                )
                .unwrap()
                .push_constants(self.text_pipeline.layout().clone(), 0, self.pixel_to_clip)
                .unwrap()
                .bind_vertex_buffers(0, text.clone())
                .unwrap();
            // SAFETY: pipeline, vertex buffer, descriptor set and push
            // constants bound above satisfy the draw's requirements.
            unsafe { builder.draw(text.len() as u32, 1, 0, 0) }.unwrap();
        }

        builder.end_render_pass(SubpassEndInfo::default()).unwrap();
        let command_buffer = builder.build().unwrap();

        let future = self
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(
                    self.swapchain.clone(),
                    image_index,
                ),
            )
            .then_signal_fence_and_flush();
        match future.map_err(Validated::unwrap) {
            Ok(future) => self.previous_frame_end = Some(future.boxed()),
            Err(VulkanError::OutOfDate) => {
                self.window_resized = true;
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
            Err(err) => panic!("failed to flush future: {err}"),
        }
    }

    fn recreate_swapchain(&mut self) {
        let image_extent: [u32; 2] = self.window.inner_size().into();
        let (swapchain, images) = self
            .swapchain
            .recreate(SwapchainCreateInfo {
                image_extent,
                ..self.swapchain.create_info().clone()
            })
            .expect("failed to recreate swapchain");
        self.swapchain = swapchain;
        self.framebuffers = window_size_dependent_setup(&images, &self.render_pass);
    }
}

fn window_size_dependent_setup(
    images: &[Arc<Image>],
    render_pass: &Arc<RenderPass>,
) -> Vec<Arc<Framebuffer>> {
    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect()
}

fn vertex_buffer<T: BufferContents + Copy>(
    allocator: &Arc<StandardMemoryAllocator>,
    data: Vec<T>,
) -> Option<Subbuffer<[T]>> {
    if data.is_empty() {
        return None;
    }
    Some(
        Buffer::from_iter(
            allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            data,
        )
        .expect("failed to create vertex buffer"),
    )
}

#[allow(clippy::too_many_arguments)]
fn graphics_pipeline(
    device: &Arc<Device>,
    vs: EntryPoint,
    fs: EntryPoint,
    vertex_input: VertexBufferDescription,
    topology: PrimitiveTopology,
    blend: Option<AttachmentBlend>,
    subpass: &Subpass,
) -> Arc<GraphicsPipeline> {
    let stages = [
        PipelineShaderStageCreateInfo::new(vs.clone()),
        PipelineShaderStageCreateInfo::new(fs),
    ];
    let layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(device.clone())
            .unwrap(),
    )
    .unwrap();
    GraphicsPipeline::new(
        device.clone(),
        None,
        GraphicsPipelineCreateInfo {
            stages: stages.into_iter().collect(),
            vertex_input_state: Some(vertex_input.definition(&vs).unwrap()),
            input_assembly_state: Some(InputAssemblyState {
                topology,
                ..Default::default()
            }),
            viewport_state: Some(ViewportState::default()),
            rasterization_state: Some(RasterizationState::default()),
            multisample_state: Some(MultisampleState::default()),
            color_blend_state: Some(ColorBlendState::with_attachment_states(
                subpass.num_color_attachments(),
                ColorBlendAttachmentState {
                    blend,
                    ..Default::default()
                },
            )),
            dynamic_state: [DynamicState::Viewport].into_iter().collect(),
            subpass: Some(subpass.clone().into()),
            ..GraphicsPipelineCreateInfo::layout(layout)
        },
    )
    .expect("failed to create graphics pipeline")
}

/// Compiles GLSL to a `ShaderModule` at runtime through naga (pure Rust, no
/// native shader toolchain needed).
fn load_shader(
    device: &Arc<Device>,
    source: &str,
    stage: naga::ShaderStage,
) -> EntryPoint {
    let module = naga::front::glsl::Frontend::default()
        .parse(&naga::front::glsl::Options::from(stage), source)
        .expect("GLSL parse failed");
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("shader validation failed");
    let words = naga::back::spv::write_vec(&module, &info, &naga::back::spv::Options::default(), None)
        .expect("SPIR-V generation failed");
    // SAFETY: the SPIR-V comes from naga's validated output, so it satisfies
    // the validity invariants `ShaderModule::new` requires.
    unsafe { ShaderModule::new(device.clone(), ShaderModuleCreateInfo::new(&words)) }
    .expect("failed to create shader module")
    .entry_point("main")
    .expect("shader has no `main` entry point")
}
