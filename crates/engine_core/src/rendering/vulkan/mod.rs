use anyhow::Result;
use ash::vk::{
    self, ClearColorValue, CommandBufferResetFlags, CommandPool, Pipeline, PipelineLayout,
    PipelineLayoutCreateInfo,
};
use egui::{Context, TextureId, epaint::ImageDelta};
use std::{fs, sync::Arc};
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};

use crate::rendering::{
    core::frame_info::{FrameInfo, PushConstants, matrix_to_push_constant},
    egui::renderer::UIRenderer,
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

        RenderingInfo {
            context: VulkanRenderingContext::new(RenderingContextAttributes {
                compatability_window: &window,
                queue_family_picker: queue_family_picker::single_queue_family,
            })
            .unwrap(),
            window,
        }
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
    pub ui_renderer: UIRenderer,
    context: Arc<VulkanRenderingContext>,
    current_image_index: u32,
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
        let swapchain = VulkanSwapchain::new(
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

            let ui_renderer = UIRenderer::new(context.clone(), &swapchain, rendering_info.window)?;
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
                ui_renderer,
                current_image_index: 0,
            };

            // rendering_info.renderer = Some(Box::new(renderer));
            Ok(renderer)
        }
    }

    pub fn render(&mut self, frame_info: FrameInfo) -> anyhow::Result<()> {
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

            for mesh in frame_info.meshes {
                let mvp = frame_info.view_projection;
                let push_constants = PushConstants {
                    mvp: matrix_to_push_constant(&mvp),
                };

                self.context.device.cmd_push_constants(
                    frame.command_buffer,
                    self.pipeline_layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    std::slice::from_raw_parts(
                        &push_constants as *const PushConstants as *const u8,
                        std::mem::size_of::<PushConstants>(),
                    ),
                );

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

            self.current_image_index = image_index;
        }

        Ok(())
    }
    pub fn update_command_buffer(&mut self) {}

    pub fn recreate_swapchain(&mut self) {}

    pub fn resize(&mut self) -> anyhow::Result<()> {
        self.swapchain.resize()
    }

    // --- UI ---

    pub fn begin_ui(&mut self) {
        let mut state = self.ui_renderer.state.lock().unwrap();
        let raw_input = state.take_egui_input(&self.ui_renderer.window);
        self.ui_renderer.context.begin_pass(raw_input);
    }

    pub fn end_ui(&mut self) -> Result<()> {
        let frame = &self.frames[self.current_frame];
        let full_output = self.ui_renderer.context.end_pass();
        let mut state = self.ui_renderer.state.lock().unwrap();
        let mut renderer = self.ui_renderer.renderer.lock().unwrap();

        state.handle_platform_output(&self.ui_renderer.window, full_output.platform_output);

        if !self.ui_renderer.pending_texture_frees.is_empty() {
            let to_free = std::mem::take(&mut self.ui_renderer.pending_texture_frees);
            renderer.free_textures(&to_free)?;
        }

        let pixels_per_point = full_output.pixels_per_point;

        self.ui_renderer.cached_primitives = self
            .ui_renderer
            .context
            .tessellate(full_output.shapes, pixels_per_point);

        if !full_output.textures_delta.set.is_empty() {
            let texture_updates: Vec<(TextureId, ImageDelta)> = full_output
                .textures_delta
                .set
                .iter()
                .flat_map(|(id, deltas)| deltas.iter().map(move |delta| (*id, delta.clone())))
                .collect();
            renderer.set_textures(
                self.context.queues[self.context.queue_families.graphics as usize],
                self.command_pool,
                &texture_updates,
            )?;
        }

        unsafe {
            let color_attachment = vk::RenderingAttachmentInfo::default()
                .image_view(self.swapchain.views[self.current_image_index as usize])
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::LOAD) // Load existing content
                .store_op(vk::AttachmentStoreOp::STORE);

            let rendering_info = vk::RenderingInfo::default()
                .render_area(vk::Rect2D::default().extent(self.swapchain.extent))
                .layer_count(1)
                .color_attachments(std::slice::from_ref(&color_attachment));

            self.context
                .device
                .cmd_begin_rendering(frame.command_buffer, &rendering_info);

            // Set viewport and scissor for egui
            self.context.device.cmd_set_viewport(
                frame.command_buffer,
                0,
                &[vk::Viewport::default()
                    .width(self.swapchain.extent.width as f32)
                    .height(self.swapchain.extent.height as f32)
                    .min_depth(0.0)
                    .max_depth(1.0)],
            );

            self.context.device.cmd_set_scissor(
                frame.command_buffer,
                0,
                &[vk::Rect2D::default().extent(self.swapchain.extent)],
            );

            renderer.cmd_draw(
                frame.command_buffer,
                self.swapchain.extent,
                pixels_per_point,
                &self.ui_renderer.cached_primitives,
            )?;

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
                .present_image(self.current_image_index, frame.render_finished_semaphore)?;
        }

        // Defer these frees until next frame (see `ui_pending_texture_frees`).
        self.ui_renderer.pending_texture_frees =
            full_output.textures_delta.free.iter().copied().collect();
        Ok(())
    }

    pub fn handle_ui_event(&mut self, event: &WindowEvent) -> bool {
        let mut state = self.ui_renderer.state.lock().unwrap();
        state
            .on_window_event(&self.ui_renderer.window, event)
            .consumed
    }

    pub fn get_egui_context(&self) -> Context {
        self.ui_renderer.context.clone()
    }
}
