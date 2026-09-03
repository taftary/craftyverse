//! The renderer: owns the Vulkan objects and the scene vertex buffers, and
//! draws frames in a fixed order — world lines, world triangles, UI lines,
//! UI triangles, then alpha-blended text.

use std::sync::Arc;

use glam::Vec2;
use vulkano::buffer::Subbuffer;
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo,
    SubpassBeginInfo, SubpassEndInfo,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::DescriptorSet;
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::instance::Instance;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::pipeline::graphics::color_blend::AttachmentBlend;
use vulkano::pipeline::graphics::input_assembly::PrimitiveTopology;
use vulkano::pipeline::graphics::vertex_input::Vertex;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::{Framebuffer, RenderPass, Subpass};
use vulkano::swapchain::{
    acquire_next_image, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo,
};
use vulkano::sync::{self, GpuFuture};
use vulkano::{Validated, VulkanError};
use winit::window::Window;

use crate::node::NodeRef;
use crate::scene::{self, Attribute, Checkbox, DisplayOptions, SceneMesh};
use crate::text::TextAtlas;

use super::setup;
use super::shaders::{load_shader, GEOM_FRAG, GEOM_VERT, TEXT_FRAG, TEXT_VERT};
use super::vertices::{GeomVertex, PushTransform, TextVertexGpu};

/// Owns the window, the Vulkan objects and the current scene's vertex
/// buffers; redraws on request from the viewer.
pub(crate) struct Renderer {
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
    /// Full Vulkan setup: device, swapchain, render pass, pipelines and the
    /// glyph atlas (see `setup`). Scene buffers stay empty until `set_scene`.
    pub(crate) fn new(instance: Arc<Instance>, window: Arc<Window>) -> Self {
        let surface =
            Surface::from_window(instance.clone(), window.clone()).expect("failed to create surface");
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
        let (physical_device, queue_family_index) =
            setup::pick_physical_device(&instance, &surface, &device_extensions);
        let (device, queue) =
            setup::create_device_and_queue(&physical_device, &device_extensions, queue_family_index);

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        ));
        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let (swapchain, images) =
            setup::create_swapchain(&physical_device, &device, surface, &window);
        let render_pass = setup::create_render_pass(&device, &swapchain);
        let framebuffers = setup::create_framebuffers(&images, &render_pass);

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        let geom_vs = load_shader(&device, GEOM_VERT, naga::ShaderStage::Vertex);
        let geom_fs = load_shader(&device, GEOM_FRAG, naga::ShaderStage::Fragment);
        let text_vs = load_shader(&device, TEXT_VERT, naga::ShaderStage::Vertex);
        let text_fs = load_shader(&device, TEXT_FRAG, naga::ShaderStage::Fragment);
        let line_pipeline = setup::graphics_pipeline(
            &device,
            geom_vs.clone(),
            geom_fs.clone(),
            GeomVertex::per_vertex(),
            PrimitiveTopology::LineList,
            None,
            &subpass,
        );
        let tri_pipeline = setup::graphics_pipeline(
            &device,
            geom_vs,
            geom_fs,
            GeomVertex::per_vertex(),
            PrimitiveTopology::TriangleList,
            None,
            &subpass,
        );
        let text_pipeline = setup::graphics_pipeline(
            &device,
            text_vs,
            text_fs,
            TextVertexGpu::per_vertex(),
            PrimitiveTopology::TriangleList,
            Some(AttachmentBlend::alpha()),
            &subpass,
        );

        let (atlas, text_descriptor_set) = setup::upload_atlas(
            &device,
            &queue,
            queue_family_index,
            &memory_allocator,
            &command_buffer_allocator,
            &descriptor_set_allocator,
            &text_pipeline,
        );

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
            world_to_clip: PushTransform::IDENTITY,
            pixel_to_clip: PushTransform::IDENTITY,
            previous_frame_end: Some(sync::now(device).boxed()),
            window_resized: false,
        }
    }

    /// Rebuilds the scene mesh for `scenario` and re-uploads all vertex
    /// buffers. Cheap enough to run on scene switch, on resize and on
    /// checkbox toggle.
    pub(crate) fn set_scene(&mut self, scenario: &[NodeRef], options: &DisplayOptions) {
        let size = self.window.inner_size();
        let viewport = Vec2::new(size.width as f32, size.height as f32);
        let mesh = scene::build_scene(scenario, viewport, options);

        let to_geom = |v: &scene::Vertex| GeomVertex {
            pos: v.pos.to_array(),
            color: v.color,
        };
        self.line_buffer = setup::vertex_buffer(
            &self.memory_allocator,
            mesh.lines.iter().map(to_geom).collect(),
        );
        self.tri_buffer = setup::vertex_buffer(
            &self.memory_allocator,
            mesh.triangles.iter().map(to_geom).collect(),
        );
        self.ui_line_buffer = setup::vertex_buffer(
            &self.memory_allocator,
            mesh.ui_lines.iter().map(to_geom).collect(),
        );
        self.ui_tri_buffer = setup::vertex_buffer(
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
        self.text_buffer = setup::vertex_buffer(
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
        self.world_to_clip = PushTransform::from(&world_to_clip);
        self.pixel_to_clip = PushTransform::from(&pixel_to_clip);
    }

    /// Attribute whose checkbox contains `point` (physical pixels), if any.
    pub(crate) fn checkbox_at(&self, point: Vec2) -> Option<Attribute> {
        checkbox_at(&self.checkboxes, point)
    }

    /// Asks winit for a redraw (delivered as a `RedrawRequested` event).
    pub(crate) fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Flags the swapchain for recreation at the next `draw_frame`.
    pub(crate) fn mark_resized(&mut self) {
        self.window_resized = true;
    }

    /// Draws one frame: world lines, world triangles, checkbox panel (lines
    /// then triangles), then text. Recreates the swapchain and rebuilds the
    /// scene first when the window was resized.
    pub(crate) fn draw_frame(&mut self, scenario: &[NodeRef], options: &DisplayOptions) {
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
            record_draw(&mut builder, &self.line_pipeline, self.world_to_clip, lines, None);
        }
        // …then filled triangles (arrowheads, dots) on top of the lines.
        if let Some(triangles) = &self.tri_buffer {
            record_draw(&mut builder, &self.tri_pipeline, self.world_to_clip, triangles, None);
        }
        // Checkbox panel, in pixel space.
        if let Some(ui_lines) = &self.ui_line_buffer {
            record_draw(&mut builder, &self.line_pipeline, self.pixel_to_clip, ui_lines, None);
        }
        if let Some(ui_triangles) = &self.ui_tri_buffer {
            record_draw(&mut builder, &self.tri_pipeline, self.pixel_to_clip, ui_triangles, None);
        }
        // …finally alpha-blended text in pixel space.
        if let Some(text) = &self.text_buffer {
            record_draw(
                &mut builder,
                &self.text_pipeline,
                self.pixel_to_clip,
                text,
                Some(&self.text_descriptor_set),
            );
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

    /// Recreates the swapchain and its framebuffers at the current window
    /// size.
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
        self.framebuffers = setup::create_framebuffers(&images, &self.render_pass);
    }
}

/// Attribute of the first checkbox containing `point`, if any.
pub(crate) fn checkbox_at(checkboxes: &[Checkbox], point: Vec2) -> Option<Attribute> {
    checkboxes
        .iter()
        .find(|checkbox| checkbox.contains(point))
        .map(|checkbox| checkbox.attribute)
}

/// Records one draw batch: binds `pipeline`, `transform` as push constants
/// and `buffer` as vertex buffer — plus the atlas `descriptor_set` for the
/// text batch — then draws the whole buffer.
fn record_draw<T>(
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    pipeline: &Arc<GraphicsPipeline>,
    transform: PushTransform,
    buffer: &Subbuffer<[T]>,
    descriptor_set: Option<&Arc<DescriptorSet>>,
) {
    builder.bind_pipeline_graphics(pipeline.clone()).unwrap();
    if let Some(descriptor_set) = descriptor_set {
        builder
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                pipeline.layout().clone(),
                0,
                descriptor_set.clone(),
            )
            .unwrap();
    }
    builder
        .push_constants(pipeline.layout().clone(), 0, transform)
        .unwrap()
        .bind_vertex_buffers(0, buffer.clone())
        .unwrap();
    // SAFETY: pipeline, vertex buffer, push constants and, for the text
    // batch, the descriptor set bound above satisfy the draw's requirements.
    unsafe { builder.draw(buffer.len() as u32, 1, 0, 0) }.unwrap();
}
