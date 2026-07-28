use anyhow::Result;
use ash::vk::{
    self, ClearColorValue, CommandBufferResetFlags, CommandPool, Pipeline, PipelineLayout,
    PipelineLayoutCreateInfo,
};
use std::{
    fs,
    sync::{Arc, Mutex},
};
use winit::{event_loop::ActiveEventLoop, window::Window};

use crate::rendering::{
    core::model::GpuMesh,
    vulkan::{
        context::{RenderingContextAttributes, VulkanRenderingContext},
        frame::VulkanFrame,
        image::ImageLayouts,
        queue::queue_family_picker,
        swapchain::VulkanSwapchain,
    },
};

pub mod context;
pub mod device;
pub mod frame;
pub mod image;
pub mod queue;
pub mod surface;
pub mod swapchain;

#[derive(Clone)]
pub struct RenderingInfo {
    pub context: VulkanRenderingContext,
    pub window: Arc<Window>,
    // pub settings: RenderingSettings,
}

impl RenderingInfo {
    pub fn new(event_loop: &ActiveEventLoop) -> Self {
        let window = Arc::new(event_loop.create_window(Default::default()).unwrap());

        let rendering_info = RenderingInfo {
            context: VulkanRenderingContext::new(RenderingContextAttributes {
                compatability_window: &window,
                queue_family_picker: queue_family_picker::single_queue_family,
            })
            .unwrap(),
            window,
        };

        rendering_info
    }
}

pub struct VulkanRenderer {
    pub in_flight_frames_count: usize,
    pub swapchain: VulkanSwapchain,
    pub frames: Vec<VulkanFrame>,
    pub current_frame: usize,
    pub command_pool: CommandPool,
    pub image_layouts: ImageLayouts,
    pub pipeline: Pipeline,
    pub pipeline_layout: PipelineLayout,
    context: Arc<VulkanRenderingContext>,
}

// TODO: replace with asset loader
const SHADER_DIR: &str = "res/shaders/";
fn load_shader_module(
    context: &Arc<VulkanRenderingContext>,
    path: &str,
) -> Result<ash::vk::ShaderModule> {
    let code = fs::read(format!("{SHADER_DIR}{path}"))?;
    Ok(context.create_shader_module(&code)?)
}

impl VulkanRenderer {
    pub fn new(rendering_info: RenderingInfo) -> anyhow::Result<Self> {
        let mut swapchain = VulkanSwapchain::new(
            rendering_info.context.clone().into(),
            rendering_info.window.clone(),
        )?;
        // swapchain.resize()?;

        // TODO: Replace this with an asset loader
        let vertex_shader =
            load_shader_module(&rendering_info.context.clone().into(), "shader.vert.spv")?;
        let fragment_shader =
            load_shader_module(&rendering_info.context.clone().into(), "shader.frag.spv")?;

        unsafe {
            let context = rendering_info.context.clone();
            let pipeline_layout = rendering_info
                .context
                .device
                .create_pipeline_layout(&PipelineLayoutCreateInfo::default(), None)?;

            let pipeline = context.create_graphics_pipeline(
                vertex_shader,
                fragment_shader,
                swapchain.extent,
                swapchain.format,
                pipeline_layout,
                Default::default(),
            )?;

            context.device.destroy_shader_module(vertex_shader, None);
            context.device.destroy_shader_module(fragment_shader, None);

            let command_pool = context.device.create_command_pool(
                &ash::vk::CommandPoolCreateInfo::default()
                    .queue_family_index(context.queue_families.graphics)
                    .flags(ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
                None,
            )?;

            let in_flight_frames_count = 1;

            let command_buffers = context.device.allocate_command_buffers(
                &ash::vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .level(ash::vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(in_flight_frames_count as u32),
            )?;

            let mut frames = Vec::with_capacity(command_buffers.len());
            for (_index, &command_buffer) in command_buffers.iter().enumerate() {
                let image_available_semaphore = context
                    .device
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?;
                let render_finished_semaphore = context
                    .device
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?;
                let in_flight_fence = context.device.create_fence(
                    &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                    None,
                )?;

                frames.push(VulkanFrame {
                    command_buffer,
                    image_available_semaphore,
                    render_finished_semaphore,
                    in_flight_fence,
                });
            }

            let renderer = VulkanRenderer {
                in_flight_frames_count,
                current_frame: 0,
                frames,
                command_pool,
                image_layouts: ImageLayouts::default(),
                pipeline,
                pipeline_layout,
                context: Arc::new(rendering_info.context.clone()),
                swapchain,
            };

            // rendering_info.renderer = Some(Box::new(renderer));
            Ok(renderer)
        }
    }

    pub fn render(&mut self, meshes: Vec<GpuMesh>) -> anyhow::Result<()> {
        let frame = &self.frames[self.current_frame];

        unsafe {
            self.context
                .device
                .wait_for_fences(&[frame.in_flight_fence], true, u64::MAX)?;

            self.context
                .device
                .reset_command_buffer(frame.command_buffer, CommandBufferResetFlags::empty())?;

            let image_index = self
                .swapchain
                .acquire_next_image(frame.image_available_semaphore)?;

            self.context.device.begin_command_buffer(
                frame.command_buffer,
                &ash::vk::CommandBufferBeginInfo::default(),
            )?;

            // transition image layout to be used for color attachmant
            self.context.transition_image_layout(
                frame.command_buffer,
                self.swapchain.images[image_index as usize],
                self.image_layouts.undefined,
                self.image_layouts.renderable,
                vk::ImageAspectFlags::COLOR,
            );

            // transition image layout to be presented for rendering
            self.context.transition_image_layout(
                frame.command_buffer,
                self.swapchain.images[image_index as usize],
                self.image_layouts.renderable,
                self.image_layouts.present,
                vk::ImageAspectFlags::COLOR,
            );

            self.context.begin_rendering(
                frame.command_buffer,
                self.swapchain.views[image_index as usize],
                ClearColorValue {
                    float32: [0.0, 0.2, 0.8, 1.0],
                },
                vk::Rect2D::default().extent(self.swapchain.extent),
            );

            self.context.device.cmd_set_viewport(
                frame.command_buffer,
                0,
                &[vk::Viewport::default()
                    .width(self.swapchain.extent.width as f32)
                    .height(self.swapchain.extent.height as f32)],
            );

            self.context.device.cmd_set_scissor(
                frame.command_buffer,
                0,
                &[vk::Rect2D::default().extent(self.swapchain.extent)],
            );
            self.context.device.cmd_bind_pipeline(
                frame.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            for mesh in meshes {
                self.context.device.cmd_bind_vertex_buffers(
                    frame.command_buffer,
                    0,
                    &[mesh.vertex_buffer],
                    &[0],
                );
                self.context.device.cmd_bind_index_buffer(
                    frame.command_buffer,
                    mesh.index_buffer,
                    0,
                    vk::IndexType::UINT32,
                );
                self.context.device.cmd_draw_indexed(
                    frame.command_buffer,
                    mesh.index_count,
                    1,
                    0,
                    0,
                    0,
                );
            }

            self.context
                .device
                .cmd_draw(frame.command_buffer, 3, 1, 0, 0);

            self.context.device.cmd_end_rendering(frame.command_buffer);

            self.context
                .device
                .end_command_buffer(frame.command_buffer)?;

            self.context.device.queue_submit(
                self.context.queues[self.context.queue_families.graphics as usize],
                &[ash::vk::SubmitInfo::default()
                    .wait_semaphores(&[frame.image_available_semaphore])
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                    .command_buffers(&[frame.command_buffer])
                    .signal_semaphores(&[frame.render_finished_semaphore])],
                frame.in_flight_fence,
            )?;

            self.swapchain
                .present_image(image_index, frame.render_finished_semaphore)?;
        }

        Ok(())
    }
    pub fn update_command_buffer(&mut self) {}

    pub fn recreate_swapchain(&mut self) {}

    pub fn resize(&mut self) -> anyhow::Result<()> {
        self.swapchain.resize()
    }
}
