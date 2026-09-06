//! Vulkan object setup: instance, physical/logical device, swapchain, render
//! pass, framebuffers, pipelines, glyph-atlas upload and vertex buffers.
//! Free functions so `Renderer::new` stays a short orchestration over them.

use std::sync::Arc;

use vulkano::VulkanLibrary;
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{DescriptorSet, WriteDescriptorSet};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::format::Format;
use vulkano::image::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::graphics::color_blend::{
    AttachmentBlend, ColorBlendAttachmentState, ColorBlendState,
};
use vulkano::pipeline::graphics::depth_stencil::{DepthState, DepthStencilState};
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{VertexBufferDescription, VertexDefinition};
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    DynamicState, GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::shader::EntryPoint;
use vulkano::swapchain::{PresentMode, Surface, Swapchain, SwapchainCreateInfo};
use vulkano::sync::{self, GpuFuture};
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::text::TextAtlas;

/// Creates the Vulkan instance with the surface extensions `event_loop`
/// requires (`ENUMERATE_PORTABILITY` for MoltenVK).
pub(crate) fn create_instance(event_loop: &EventLoop<()>) -> Arc<Instance> {
    let library = VulkanLibrary::new().expect("no local Vulkan library");
    let required_extensions =
        Surface::required_extensions(event_loop).expect("failed to query surface extensions");
    Instance::new(
        library,
        InstanceCreateInfo {
            flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
            enabled_extensions: required_extensions,
            ..Default::default()
        },
    )
    .expect("failed to create Vulkan instance")
}

/// Picks the physical device to use: the best-ranked one (by
/// `device_type_rank`) that supports `device_extensions` and has a graphics
/// queue family able to present to `surface`. Returns it with that queue
/// family's index.
pub(crate) fn pick_physical_device(
    instance: &Arc<Instance>,
    surface: &Arc<Surface>,
    device_extensions: &DeviceExtensions,
) -> (Arc<PhysicalDevice>, u32) {
    let (physical_device, queue_family_index) = instance
        .enumerate_physical_devices()
        .expect("failed to enumerate physical devices")
        .filter(|p| p.supported_extensions().contains(device_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    q.queue_flags.intersects(QueueFlags::GRAPHICS)
                        && p.surface_support(i as u32, surface).unwrap_or(false)
                })
                .map(|i| (p, i as u32))
        })
        .min_by_key(|(p, _)| device_type_rank(p.properties().device_type))
        .expect("no suitable Vulkan physical device found");
    println!(
        "Using device: {} (type: {:?})",
        physical_device.properties().device_name,
        physical_device.properties().device_type,
    );
    (physical_device, queue_family_index)
}

/// Preference rank of a physical device kind: discrete GPU first, then
/// integrated, virtual, CPU, anything else last.
pub(crate) fn device_type_rank(device_type: PhysicalDeviceType) -> u8 {
    match device_type {
        PhysicalDeviceType::DiscreteGpu => 0,
        PhysicalDeviceType::IntegratedGpu => 1,
        PhysicalDeviceType::VirtualGpu => 2,
        PhysicalDeviceType::Cpu => 3,
        _ => 4,
    }
}

/// Creates the logical device (with `device_extensions`) and returns it with
/// its queue from `queue_family_index`.
pub(crate) fn create_device_and_queue(
    physical_device: &Arc<PhysicalDevice>,
    device_extensions: &DeviceExtensions,
    queue_family_index: u32,
) -> (Arc<Device>, Arc<Queue>) {
    let (device, mut queues) = Device::new(
        physical_device.clone(),
        DeviceCreateInfo {
            enabled_extensions: *device_extensions,
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            ..Default::default()
        },
    )
    .expect("failed to create device");
    (device, queues.next().unwrap())
}

/// Creates the swapchain for `window` (FIFO present mode, i.e. vsync) and
/// returns it with its images.
pub(crate) fn create_swapchain(
    physical_device: &Arc<PhysicalDevice>,
    device: &Arc<Device>,
    surface: Arc<Surface>,
    window: &Arc<Window>,
) -> (Arc<Swapchain>, Vec<Arc<Image>>) {
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
    Swapchain::new(
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
    .expect("failed to create swapchain")
}

/// Single render pass: one color attachment (the swapchain image), cleared
/// then stored, plus a depth attachment (cleared, not stored).
pub(crate) fn create_render_pass(
    device: &Arc<Device>,
    swapchain: &Arc<Swapchain>,
) -> Arc<RenderPass> {
    vulkano::single_pass_renderpass!(
        device.clone(),
        attachments: {
            color: {
                format: swapchain.image_format(),
                samples: 1,
                load_op: Clear,
                store_op: Store,
            },
            depth: {
                format: Format::D32_SFLOAT,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            },
        },
        pass: { color: [color], depth_stencil: {depth} },
    )
    .expect("failed to create render pass")
}

/// Depth buffer image view (D32_SFLOAT) at the swapchain extent; recreated
/// together with the swapchain on window resize.
pub(crate) fn create_depth_view(
    memory_allocator: &Arc<StandardMemoryAllocator>,
    extent: [u32; 2],
) -> Arc<ImageView> {
    let image = Image::new(
        memory_allocator.clone(),
        ImageCreateInfo {
            image_type: ImageType::Dim2d,
            format: Format::D32_SFLOAT,
            extent: [extent[0], extent[1], 1],
            usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
    )
    .expect("failed to create depth image");
    ImageView::new_default(image).expect("failed to create depth image view")
}

/// One framebuffer per swapchain image, each with the shared depth view;
/// recreated together with the swapchain on window resize.
pub(crate) fn create_framebuffers(
    images: &[Arc<Image>],
    depth_view: &Arc<ImageView>,
    render_pass: &Arc<RenderPass>,
) -> Vec<Arc<Framebuffer>> {
    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view, depth_view.clone()],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect()
}

/// A graphics pipeline drawing `vertex_input` as `topology` with the given
/// shaders; `blend` enables alpha blending (text quads) when `Some`, and
/// `depth` enables depth testing and writes (world geometry) — UI and text
/// pipelines leave depth off so they always draw on top. The viewport is
/// dynamic, set per frame.
#[allow(clippy::too_many_arguments)]
pub(crate) fn graphics_pipeline(
    device: &Arc<Device>,
    vs: EntryPoint,
    fs: EntryPoint,
    vertex_input: VertexBufferDescription,
    topology: PrimitiveTopology,
    blend: Option<AttachmentBlend>,
    depth: bool,
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
    let depth_stencil_state = if depth {
        DepthStencilState {
            depth: Some(DepthState::simple()),
            ..Default::default()
        }
    } else {
        DepthStencilState::default()
    };
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
            depth_stencil_state: Some(depth_stencil_state),
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

/// Rasterizes the glyph atlas, uploads it as an R8 texture through a one-shot
/// staging copy, and returns the atlas with the descriptor set (linear
/// sampler + texture) the text pipeline samples.
#[allow(clippy::too_many_arguments)]
pub(crate) fn upload_atlas(
    device: &Arc<Device>,
    queue: &Arc<Queue>,
    queue_family_index: u32,
    memory_allocator: &Arc<StandardMemoryAllocator>,
    command_buffer_allocator: &Arc<StandardCommandBufferAllocator>,
    descriptor_set_allocator: &Arc<StandardDescriptorSetAllocator>,
    text_pipeline: &Arc<GraphicsPipeline>,
) -> (TextAtlas, Arc<DescriptorSet>) {
    // Rasterize the glyph atlas and upload it as an R8 texture.
    let atlas = TextAtlas::new(include_bytes!(
        "../../../../assets/fonts/JetBrainsMono-Regular.ttf"
    ))
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
    let text_layout = text_pipeline
        .layout()
        .set_layouts()
        .first()
        .unwrap()
        .clone();
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
    (atlas, text_descriptor_set)
}

/// Uploads `data` to a device-local vertex buffer; `None` when `data` is
/// empty (the matching draw batch is then skipped).
pub(crate) fn vertex_buffer<T: BufferContents + Copy>(
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
