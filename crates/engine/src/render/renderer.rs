//! The renderer: owns the Vulkan objects and the scene vertex buffers, and
//! draws frames in a fixed order — world geometry (per view mode: mesh lines
//! and triangles, textured 3D triangles, or the flat UV map plus its
//! wireframe overlay), then the checkbox panel (lines, triangles), then
//! alpha-blended text. World geometry is transformed on the GPU by the
//! orbit-camera view-projection matrix (a push constant), so camera changes
//! never rebuild the geometry buffers; only the label anchors are
//! re-projected on the CPU.

use std::sync::Arc;

use glam::{Mat4, Vec2, Vec3, Vec4};
use vulkano::buffer::{BufferContents, Subbuffer};
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo,
    SubpassBeginInfo, SubpassEndInfo,
};
use vulkano::descriptor_set::DescriptorSet;
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::image::view::ImageView;
use vulkano::instance::Instance;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::pipeline::graphics::color_blend::AttachmentBlend;
use vulkano::pipeline::graphics::input_assembly::PrimitiveTopology;
use vulkano::pipeline::graphics::vertex_input::Vertex;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::render_pass::{Framebuffer, RenderPass, Subpass};
use vulkano::swapchain::{
    Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo, acquire_next_image,
};
use vulkano::sync::{self, GpuFuture};
use vulkano::{Validated, VulkanError};
use winit::window::Window;

use crate::node::NodeRef;
use crate::scene::{self, Attribute, Checkbox, DisplayOptions, OrbitCamera, TextRun, WorldLabel};
use crate::text::TextAtlas;

use super::buffers::VertexBuffer;
use super::setup;
use super::shaders::{GEOM_FRAG, GEOM_VERT, TEX_FRAG, TEX_VERT, TEXT_FRAG, TEXT_VERT, load_shader};
use super::vertices::{GeomVertex, PushMatrix, PushTransform, TextVertexGpu, UvVertexGpu};

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
    depth_view: Arc<ImageView>,
    line_pipeline: Arc<GraphicsPipeline>,
    tri_pipeline: Arc<GraphicsPipeline>,
    ui_line_pipeline: Arc<GraphicsPipeline>,
    ui_tri_pipeline: Arc<GraphicsPipeline>,
    text_pipeline: Arc<GraphicsPipeline>,
    tex_pipeline: Arc<GraphicsPipeline>,
    text_descriptor_set: Arc<DescriptorSet>,
    tex_descriptor_set: Arc<DescriptorSet>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    atlas: TextAtlas,
    line_buffer: VertexBuffer<GeomVertex>,
    tri_buffer: VertexBuffer<GeomVertex>,
    ui_line_buffer: VertexBuffer<GeomVertex>,
    ui_tri_buffer: VertexBuffer<GeomVertex>,
    text_buffer: Option<Subbuffer<[TextVertexGpu]>>,
    tex_world_buffer: VertexBuffer<UvVertexGpu>,
    tex_uv_buffer: VertexBuffer<UvVertexGpu>,
    uv_line_buffer: VertexBuffer<GeomVertex>,
    /// View mode the scene was built with (drives the world batches drawn).
    view_mode: scene::ViewMode,
    /// Checkbox hit rectangles of the current scene (pixel space).
    checkboxes: Vec<Checkbox>,
    /// World-anchored labels of the current scene, re-projected on camera
    /// changes.
    labels: Vec<WorldLabel>,
    /// Checkbox labels of the current scene (pixel space).
    ui_texts: Vec<TextRun<'static>>,
    /// Content bounding sphere of the current scene, the camera fit target.
    fit_center: Vec3,
    fit_radius: f32,
    /// Camera applied to the current scene (updated by `set_camera`).
    camera: OrbitCamera,
    world_mvp: PushMatrix,
    pixel_mvp: PushMatrix,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    window_resized: bool,
}

impl Renderer {
    /// Full Vulkan setup: device, swapchain, render pass, pipelines and the
    /// glyph atlas (see `setup`). Scene buffers stay empty until `set_scene`.
    pub(crate) fn new(instance: Arc<Instance>, window: Arc<Window>) -> Self {
        let surface = Surface::from_window(instance.clone(), window.clone())
            .expect("failed to create surface");
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
        let (physical_device, queue_family_index) =
            setup::pick_physical_device(&instance, &surface, &device_extensions);
        let (device, queue) = setup::create_device_and_queue(
            &physical_device,
            &device_extensions,
            queue_family_index,
        );

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
        let depth_view = setup::create_depth_view(&memory_allocator, swapchain.image_extent());
        let framebuffers = setup::create_framebuffers(&images, &depth_view, &render_pass);

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
            true,
            &subpass,
        );
        let tri_pipeline = setup::graphics_pipeline(
            &device,
            geom_vs.clone(),
            geom_fs.clone(),
            GeomVertex::per_vertex(),
            PrimitiveTopology::TriangleList,
            None,
            true,
            &subpass,
        );
        let ui_line_pipeline = setup::graphics_pipeline(
            &device,
            geom_vs.clone(),
            geom_fs.clone(),
            GeomVertex::per_vertex(),
            PrimitiveTopology::LineList,
            None,
            false,
            &subpass,
        );
        let ui_tri_pipeline = setup::graphics_pipeline(
            &device,
            geom_vs,
            geom_fs,
            GeomVertex::per_vertex(),
            PrimitiveTopology::TriangleList,
            None,
            false,
            &subpass,
        );
        let text_pipeline = setup::graphics_pipeline(
            &device,
            text_vs,
            text_fs,
            TextVertexGpu::per_vertex(),
            PrimitiveTopology::TriangleList,
            Some(AttachmentBlend::alpha()),
            false,
            &subpass,
        );
        let tex_vs = load_shader(&device, TEX_VERT, naga::ShaderStage::Vertex);
        let tex_fs = load_shader(&device, TEX_FRAG, naga::ShaderStage::Fragment);
        let tex_pipeline = setup::graphics_pipeline(
            &device,
            tex_vs,
            tex_fs,
            UvVertexGpu::per_vertex(),
            PrimitiveTopology::TriangleList,
            None,
            true,
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
        let tex_descriptor_set = setup::upload_checkerboard(
            &device,
            &queue,
            queue_family_index,
            &memory_allocator,
            &command_buffer_allocator,
            &descriptor_set_allocator,
            &tex_pipeline,
        );

        Renderer {
            window,
            device: device.clone(),
            queue,
            queue_family_index,
            swapchain,
            render_pass,
            framebuffers,
            depth_view,
            line_pipeline,
            tri_pipeline,
            ui_line_pipeline,
            ui_tri_pipeline,
            text_pipeline,
            tex_pipeline,
            text_descriptor_set,
            tex_descriptor_set,
            memory_allocator,
            command_buffer_allocator,
            atlas,
            line_buffer: VertexBuffer::new(),
            tri_buffer: VertexBuffer::new(),
            ui_line_buffer: VertexBuffer::new(),
            ui_tri_buffer: VertexBuffer::new(),
            text_buffer: None,
            tex_world_buffer: VertexBuffer::new(),
            tex_uv_buffer: VertexBuffer::new(),
            uv_line_buffer: VertexBuffer::new(),
            view_mode: scene::ViewMode::Mesh,
            checkboxes: Vec::new(),
            labels: Vec::new(),
            ui_texts: Vec::new(),
            fit_center: Vec3::ZERO,
            fit_radius: 1.0,
            camera: OrbitCamera::default(),
            world_mvp: PushMatrix::IDENTITY,
            pixel_mvp: PushMatrix::IDENTITY,
            previous_frame_end: Some(sync::now(device).boxed()),
            window_resized: false,
        }
    }

    /// Window size in physical pixels.
    fn viewport(&self) -> Vec2 {
        let size = self.window.inner_size();
        Vec2::new(size.width as f32, size.height as f32)
    }

    /// Rebuilds the current scene's mesh in `view_mode` and re-uploads the
    /// geometry vertex buffers, reusing their allocations: batches that fit
    /// the existing capacity are rewritten in place and only growth
    /// reallocates (see `buffers`). Runs on scene switch, on view-mode
    /// switch, on resize and on checkbox toggle — never on camera changes
    /// (see `set_camera`).
    pub(crate) fn set_scene(
        &mut self,
        scenario: &[NodeRef],
        options: &DisplayOptions,
        view_mode: scene::ViewMode,
    ) {
        self.view_mode = view_mode;
        let mesh = scene::build_scene(scenario, options, view_mode);

        // The in-place buffer rewrites below must not race a frame still in
        // flight. SAFETY: the renderer is single-threaded — `set_scene` and
        // the command submission in `draw_frame` both run on the winit event
        // loop thread, so nothing submits to the device's queues while this
        // waits. `set_scene` runs only on discrete events, never per frame.
        unsafe { self.device.wait_idle() }.expect("failed to wait for the device");
        // The wait only idles the GPU: vulkano's per-buffer bookkeeping
        // still counts the previous frame's reads until its fence future is
        // cleaned. The fence is known-signaled after the wait, so this
        // deterministically propagates `signal_finished` and unlocks the
        // buffers for host writes.
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        let to_geom = |v: &scene::Vertex| GeomVertex {
            pos: v.pos.to_array(),
            color: v.color,
        };
        let to_uv = |v: &scene::UvVertex| UvVertexGpu {
            pos: v.pos.to_array(),
            uv: v.uv.to_array(),
        };
        self.line_buffer.update(
            &self.memory_allocator,
            &mesh.lines.iter().map(to_geom).collect::<Vec<_>>(),
        );
        self.tri_buffer.update(
            &self.memory_allocator,
            &mesh.triangles.iter().map(to_geom).collect::<Vec<_>>(),
        );
        self.ui_line_buffer.update(
            &self.memory_allocator,
            &mesh.ui_lines.iter().map(to_geom).collect::<Vec<_>>(),
        );
        self.ui_tri_buffer.update(
            &self.memory_allocator,
            &mesh.ui_triangles.iter().map(to_geom).collect::<Vec<_>>(),
        );
        self.tex_world_buffer.update(
            &self.memory_allocator,
            &mesh.tex_world.iter().map(to_uv).collect::<Vec<_>>(),
        );
        self.tex_uv_buffer.update(
            &self.memory_allocator,
            &mesh.tex_uv.iter().map(to_uv).collect::<Vec<_>>(),
        );
        self.uv_line_buffer.update(
            &self.memory_allocator,
            &mesh.uv_lines.iter().map(to_geom).collect::<Vec<_>>(),
        );

        self.checkboxes = mesh.checkboxes;
        self.labels = mesh.labels;
        self.ui_texts = mesh.texts;
        self.fit_center = mesh.fit_center;
        self.fit_radius = mesh.fit_radius;
        self.apply_camera();
    }

    /// Applies a new orbit camera: recomputes the view-projection push
    /// constant and re-anchors the text labels — the only per-camera CPU
    /// work. The geometry buffers are untouched.
    pub(crate) fn set_camera(&mut self, camera: OrbitCamera) {
        self.camera = camera;
        self.apply_camera();
    }

    /// Recomputes the world/pixel transforms from the stored camera and
    /// viewport, and rebuilds the text buffer for them.
    fn apply_camera(&mut self) {
        let viewport = self.viewport();
        let mvp = self
            .camera
            .view_projection(self.fit_center, self.fit_radius, viewport);
        self.world_mvp = PushMatrix::from(mvp);
        self.pixel_mvp = PushMatrix::from(pixel_matrix(viewport));

        let mut text_data = Vec::new();
        for run in &scene::project_labels(&self.labels, &mvp, viewport) {
            self.atlas.layout(
                run.text,
                run.anchor,
                run.size,
                run.color,
                run.centered,
                &mut text_data,
            );
        }
        for run in &self.ui_texts {
            self.atlas.layout(
                run.text,
                run.anchor,
                run.size,
                run.color,
                run.centered,
                &mut text_data,
            );
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

    /// Draws one frame: the world batches of the current view mode, the
    /// checkbox panel (lines then triangles), then text. Recreates the
    /// swapchain and rebuilds the scene first when the window was resized.
    pub(crate) fn draw_frame(&mut self, scenario: &[NodeRef], options: &DisplayOptions) {
        let window_size = self.window.inner_size();
        if window_size.width == 0 || window_size.height == 0 {
            return;
        }
        if self.window_resized {
            self.window_resized = false;
            self.recreate_swapchain();
            // The view fit and the text anchors depend on the viewport.
            let view_mode = self.view_mode;
            self.set_scene(scenario, options, view_mode);
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
                    clear_values: vec![Some([1.0, 1.0, 1.0, 1.0].into()), Some(1.0.into())],
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

        // World-space geometry, per view mode. The checkbox panel and text
        // below are drawn in all modes.
        match self.view_mode {
            scene::ViewMode::Mesh => {
                // Child links, outlines, arrow shafts, dashes…
                if let Some((lines, count)) = self.line_buffer.batch() {
                    record_draw(
                        &mut builder,
                        &self.line_pipeline,
                        self.world_mvp,
                        &lines,
                        count,
                        None,
                    );
                }
                // …then filled triangles (arrowheads, dots) — depth-tested
                // with the lines.
                if let Some((triangles, count)) = self.tri_buffer.batch() {
                    record_draw(
                        &mut builder,
                        &self.tri_pipeline,
                        self.world_mvp,
                        &triangles,
                        count,
                        None,
                    );
                }
            }
            scene::ViewMode::Textured => {
                // Filled node triangles sampled from the checkerboard.
                if let Some((tex_world, count)) = self.tex_world_buffer.batch() {
                    record_draw(
                        &mut builder,
                        &self.tex_pipeline,
                        self.world_mvp,
                        &tex_world,
                        count,
                        Some(&self.tex_descriptor_set),
                    );
                }
            }
            scene::ViewMode::UvMap => {
                // The textured UV net laid flat on the z = 0 world plane…
                if let Some((tex_uv, count)) = self.tex_uv_buffer.batch() {
                    record_draw(
                        &mut builder,
                        &self.tex_pipeline,
                        self.world_mvp,
                        &tex_uv,
                        count,
                        Some(&self.tex_descriptor_set),
                    );
                }
                // …then the net wireframe and vertex dots, drawn depthless so
                // the overlay floats on top of the plane (no z-fighting).
                if let Some((uv_lines, count)) = self.uv_line_buffer.batch() {
                    record_draw(
                        &mut builder,
                        &self.ui_line_pipeline,
                        self.world_mvp,
                        &uv_lines,
                        count,
                        None,
                    );
                }
            }
        }
        // Checkbox panel, in pixel space (no depth test: always on top).
        if let Some((ui_lines, count)) = self.ui_line_buffer.batch() {
            record_draw(
                &mut builder,
                &self.ui_line_pipeline,
                self.pixel_mvp,
                &ui_lines,
                count,
                None,
            );
        }
        if let Some((ui_triangles, count)) = self.ui_tri_buffer.batch() {
            record_draw(
                &mut builder,
                &self.ui_tri_pipeline,
                self.pixel_mvp,
                &ui_triangles,
                count,
                None,
            );
        }
        // …finally alpha-blended text in pixel space.
        if let Some(text) = &self.text_buffer {
            record_draw(
                &mut builder,
                &self.text_pipeline,
                PushTransform::for_viewport(self.viewport()),
                text,
                text.len() as u32,
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
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
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

    /// Recreates the swapchain, the depth view and the framebuffers at the
    /// current window size.
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
        self.depth_view = setup::create_depth_view(&self.memory_allocator, image_extent);
        self.framebuffers =
            setup::create_framebuffers(&images, &self.depth_view, &self.render_pass);
    }
}

/// Pixel-space → clip matrix for `viewport` pixels (y-down, z → 0.5); the
/// UI counterpart of the world view-projection matrix. Uses the project's
/// y-up NDC convention: pixel `(0, 0)` maps to clip `(-1, 1)`, the top-left
/// of the window.
pub fn pixel_matrix(viewport: Vec2) -> Mat4 {
    let viewport = viewport.max(Vec2::ONE);
    Mat4::from_cols(
        Vec4::new(2.0 / viewport.x, 0.0, 0.0, 0.0),
        Vec4::new(0.0, -2.0 / viewport.y, 0.0, 0.0),
        Vec4::ZERO,
        Vec4::new(-1.0, 1.0, 0.5, 1.0),
    )
}

/// Attribute of the first checkbox containing `point`, if any.
pub fn checkbox_at(checkboxes: &[Checkbox], point: Vec2) -> Option<Attribute> {
    checkboxes
        .iter()
        .find(|checkbox| checkbox.contains(point))
        .map(|checkbox| checkbox.attribute)
}

/// Records one draw batch: binds `pipeline`, `transform` as push constants
/// and `buffer` as vertex buffer — plus `descriptor_set` for the textured
/// batches (checkerboard) and the text batch (glyph atlas) — then draws
/// `vertex_count` vertices. The count comes from the batch, not the buffer
/// length: reusable buffers may carry spare capacity (see `buffers`).
fn record_draw<T, Pc: BufferContents + Copy>(
    builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    pipeline: &Arc<GraphicsPipeline>,
    transform: Pc,
    buffer: &Subbuffer<[T]>,
    vertex_count: u32,
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
    // SAFETY: pipeline, vertex buffer, push constants and, for the batches
    // that sample a texture, the descriptor set bound above satisfy the
    // draw's requirements; `vertex_count` stays within the live batch range.
    unsafe { builder.draw(vertex_count, 1, 0, 0) }.unwrap();
}
