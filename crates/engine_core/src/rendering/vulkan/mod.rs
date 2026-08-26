use crate::{
    log_debug,
    rendering::{
        core::frame_info::{FrameInfo, PushConstants, matrix_to_push_constant},
        egui::renderer::UIRenderer,
        rendering_settings::RenderingSettings,
        vulkan::{
            context::{RenderingContextAttributes, VulkanRenderingContext, depth_image_aspect},
            frame::VulkanFrame,
            image::ImageLayouts,
            queue::queue_family_picker,
            swapchain::VulkanSwapchain,
        },
    },
};
use anyhow::Result;
use ash::vk::{
    self, ClearColorValue, CommandBufferResetFlags, CommandPool, Pipeline, PipelineLayout,
    PipelineLayoutCreateInfo, Semaphore,
};
use egui::{Context, TextureId, epaint::ImageDelta};
use std::{fs, sync::Arc};
use winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};

pub mod context;
pub mod debug;
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
    pub settings: RenderingSettings,
}

impl RenderingInfo {
    pub fn new(event_loop: &ActiveEventLoop) -> Self {
        Self::with_settings(event_loop, RenderingSettings::default())
    }

    pub fn with_settings(event_loop: &ActiveEventLoop, settings: RenderingSettings) -> Self {
        let window = Arc::new(event_loop.create_window(Default::default()).unwrap());

        RenderingInfo {
            context: VulkanRenderingContext::new(RenderingContextAttributes {
                compatability_window: &window,
                queue_family_picker: queue_family_picker::single_queue_family,
            })
            .unwrap(),
            window,
            settings,
        }
    }
}

#[allow(unused)]
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
    pub settings: RenderingSettings,
    joint_buffer: vk::Buffer,
    joint_buffer_memory: vk::DeviceMemory,
    joint_descriptor_set_layout: vk::DescriptorSetLayout,
    joint_descriptor_pool: vk::DescriptorPool,
    joint_descriptor_set: vk::DescriptorSet,
    pub texture_descriptor_set_layout: vk::DescriptorSetLayout,
    pub texture_descriptor_pool: vk::DescriptorPool,
    fallback_texture_descriptor_set: vk::DescriptorSet,
    present_semaphores: Vec<Semaphore>,
    context: Arc<VulkanRenderingContext>,
    current_image_index: u32,
}

/// Maximum number of joints supported per skinned draw.
const MAX_JOINTS: usize = 256;

// TODO: replace with asset loader
const SHADER_DIR: &str = "res/shaders/";
fn load_shader_module(
    context: &VulkanRenderingContext,
    path: &str,
) -> Result<ash::vk::ShaderModule> {
    let code = fs::read(format!("{SHADER_DIR}{path}"))?;
    Ok(context.create_shader_module(&code)?)
}

fn create_graphics_pipeline(
    context: &VulkanRenderingContext,
    pipeline_layout: vk::PipelineLayout,
    swapchain: &VulkanSwapchain,
    settings: &RenderingSettings,
) -> Result<vk::Pipeline> {
    let vertex_shader = load_shader_module(context, &settings.default_vertex_shader)?;
    let fragment_shader = load_shader_module(context, &settings.default_fragment_shader)?;

    log_debug!("Creating Graphics Pipeline");
    let pipeline = context.create_graphics_pipeline(
        vertex_shader,
        fragment_shader,
        swapchain.extent,
        swapchain.format,
        pipeline_layout,
        swapchain.depth.format,
        settings.rasterization_settings,
        settings.depth_settings,
    )?;

    unsafe {
        context.device.destroy_shader_module(vertex_shader, None);
        context.device.destroy_shader_module(fragment_shader, None);
    }
    Ok(pipeline)
}

impl VulkanRenderer {
    pub fn new(
        rendering_info: RenderingInfo,
        rendering_settings: &RenderingSettings,
    ) -> anyhow::Result<Self> {
        let settings = rendering_info.settings.clone();
        let swapchain = VulkanSwapchain::new(
            rendering_info.context.clone().into(),
            rendering_info.window.clone(),
        )?;
        // swapchain.resize()?;

        // TODO: Replace this with an asset loader
        let context = rendering_info.context.clone();

        unsafe {
            // Skinning: a storage buffer holding the per-entity joint matrices.
            let joint_binding = vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX);
            let joint_descriptor_set_layout =
                context.create_descriptor_set_layout(&[joint_binding])?;

            let joint_descriptor_pool = context.create_descriptor_pool(
                &[vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)],
                1,
            )?;

            let joint_descriptor_set = context
                .allocate_descriptor_set(joint_descriptor_pool, joint_descriptor_set_layout)?;

            let joint_buffer_size =
                (MAX_JOINTS * std::mem::size_of::<nalgebra::Matrix4<f32>>()) as vk::DeviceSize;
            let (joint_buffer, joint_buffer_memory) = context.create_buffer(
                joint_buffer_size,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;

            context.device.update_descriptor_sets(
                &[vk::WriteDescriptorSet::default()
                    .dst_set(joint_descriptor_set)
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo::default()
                        .buffer(joint_buffer)
                        .offset(0)
                        .range(joint_buffer_size)])],
                &[],
            );

            let push_constant_range = vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .offset(0)
                .size(std::mem::size_of::<PushConstants>() as u32);

            // Texture descriptor set layout (set 1, binding 0)
            let texture_binding = vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT);
            let texture_descriptor_set_layout =
                context.create_descriptor_set_layout(&[texture_binding])?;

            let texture_descriptor_pool = context.create_descriptor_pool(
                &[vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(1024)],
                1024,
            )?;

            let pipeline_layout = context.device.create_pipeline_layout(
                &PipelineLayoutCreateInfo::default()
                    .set_layouts(&[joint_descriptor_set_layout, texture_descriptor_set_layout])
                    .push_constant_ranges(&[push_constant_range]),
                None,
            )?;

            let pipeline =
                create_graphics_pipeline(&context, pipeline_layout, &swapchain, &settings)?;

            let command_pool = context.device.create_command_pool(
                &ash::vk::CommandPoolCreateInfo::default()
                    .queue_family_index(context.queue_families.graphics)
                    .flags(ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
                None,
            )?;

            // Create a 1x1 white fallback texture for meshes without textures
            let white_pixel = [255u8, 255, 255, 255];
            let tex_format = vk::Format::R8G8B8A8_SRGB;
            let (fallback_image, _fallback_memory) = context.create_image(
                vk::Extent2D {
                    width: 1,
                    height: 1,
                },
                tex_format,
                vk::ImageTiling::OPTIMAL,
                vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?;
            let (staging_buf, staging_mem) = context.create_buffer(
                4,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            let ptr = context
                .device
                .map_memory(staging_mem, 0, 4, vk::MemoryMapFlags::empty())?
                as *mut u8;
            ptr.copy_from_nonoverlapping(white_pixel.as_ptr(), 4);
            context.device.unmap_memory(staging_mem);
            let cmd = context.begin_single_time_commands(command_pool);
            context.transition_image_layout(
                cmd,
                fallback_image,
                image::ImageLayoutState {
                    layout: vk::ImageLayout::UNDEFINED,
                    access_mask: vk::AccessFlags::empty(),
                    stage_mask: vk::PipelineStageFlags::TOP_OF_PIPE,
                    queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                },
                image::ImageLayoutState {
                    layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    access_mask: vk::AccessFlags::TRANSFER_WRITE,
                    stage_mask: vk::PipelineStageFlags::TRANSFER,
                    queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                },
                vk::ImageAspectFlags::COLOR,
            );
            context.copy_buffer_to_image(cmd, staging_buf, fallback_image, 1, 1);
            context.transition_image_layout(
                cmd,
                fallback_image,
                image::ImageLayoutState {
                    layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    access_mask: vk::AccessFlags::TRANSFER_WRITE,
                    stage_mask: vk::PipelineStageFlags::TRANSFER,
                    queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                },
                image::ImageLayoutState {
                    layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    access_mask: vk::AccessFlags::SHADER_READ,
                    stage_mask: vk::PipelineStageFlags::FRAGMENT_SHADER,
                    queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                },
                vk::ImageAspectFlags::COLOR,
            );
            let fallback_queue = context.queues[context.queue_families.transfer as usize];
            context.end_single_time_commands(cmd, fallback_queue, command_pool);
            context.device.destroy_buffer(staging_buf, None);
            context.device.free_memory(staging_mem, None);
            let fallback_view = context.create_image_view(
                fallback_image,
                tex_format,
                vk::ImageAspectFlags::COLOR,
            )?;
            let fallback_sampler = context.create_sampler(rendering_settings)?;
            let fallback_texture_descriptor_set = context
                .allocate_descriptor_set(texture_descriptor_pool, texture_descriptor_set_layout)?;
            context.device.update_descriptor_sets(
                &[vk::WriteDescriptorSet::default()
                    .dst_set(fallback_texture_descriptor_set)
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&[vk::DescriptorImageInfo::default()
                        .sampler(fallback_sampler)
                        .image_view(fallback_view)
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)])],
                &[],
            );

            let in_flight_frames_count = 1;

            let command_buffers = context.device.allocate_command_buffers(
                &ash::vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .level(ash::vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(in_flight_frames_count as u32),
            )?;

            let mut frames = Vec::with_capacity(command_buffers.len());
            for &command_buffer in command_buffers.iter() {
                let image_available_semaphore = context
                    .device
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?;
                let in_flight_fence = context.device.create_fence(
                    &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                    None,
                )?;

                frames.push(VulkanFrame {
                    command_buffer,
                    image_available_semaphore,
                    in_flight_fence,
                });
            }

            let ui_renderer = UIRenderer::new(
                context.clone(),
                &swapchain,
                rendering_info.window,
                settings.image_settings,
            )?;
            let renderer = VulkanRenderer {
                in_flight_frames_count,
                current_frame: 0,
                frames,
                command_pool,
                image_layouts: ImageLayouts::default(),
                pipeline,
                pipeline_layout,
                joint_buffer,
                joint_buffer_memory,
                joint_descriptor_set_layout,
                joint_descriptor_pool,
                joint_descriptor_set,
                texture_descriptor_set_layout,
                texture_descriptor_pool,
                fallback_texture_descriptor_set,
                present_semaphores: Vec::new(),
                context: Arc::new(rendering_info.context.clone()),
                swapchain,
                ui_renderer,
                current_image_index: 0,
                settings,
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

            self.context.device.reset_fences(&[frame.in_flight_fence])?;

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

            // transition depth image to a writable transfer layout and clear it
            self.context.transition_image_layout(
                frame.command_buffer,
                self.swapchain.depth.image,
                self.image_layouts.undefined,
                self.image_layouts.transfer_dst,
                depth_image_aspect(self.swapchain.depth.format),
            );

            self.context.device.cmd_clear_depth_stencil_image(
                frame.command_buffer,
                self.swapchain.depth.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
                &[vk::ImageSubresourceRange::default()
                    .aspect_mask(depth_image_aspect(self.swapchain.depth.format))
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1)],
            );

            // transition depth image layout for depth attachment use
            self.context.transition_image_layout(
                frame.command_buffer,
                self.swapchain.depth.image,
                self.image_layouts.transfer_dst,
                self.image_layouts.depth,
                depth_image_aspect(self.swapchain.depth.format),
            );

            self.context.begin_rendering(
                frame.command_buffer,
                self.swapchain.views[image_index as usize],
                self.swapchain.depth.image_view,
                ClearColorValue {
                    float32: [0.0, 0.2, 0.8, 1.0],
                },
                vk::Rect2D::default().extent(self.swapchain.extent),
            );

            self.context.device.cmd_set_viewport(
                frame.command_buffer,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: self.swapchain.extent.width as f32,
                    height: self.swapchain.extent.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
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

            for draw in frame_info.draws {
                let mesh = &draw.mesh;

                // Upload this entity's joint matrices to the storage buffer.
                let joints = draw.joint_matrices.as_ref();
                let (joint_count, write_identity) = match joints {
                    Some(j) if !j.is_empty() => (j.len().min(MAX_JOINTS), false),
                    _ => (1, true),
                };
                let bytes =
                    (joint_count * std::mem::size_of::<nalgebra::Matrix4<f32>>()) as vk::DeviceSize;

                let joint_ptr = self.context.device.map_memory(
                    self.joint_buffer_memory,
                    0,
                    bytes,
                    vk::MemoryMapFlags::empty(),
                )? as *mut nalgebra::Matrix4<f32>;
                if write_identity {
                    joint_ptr.write(nalgebra::Matrix4::identity());
                } else {
                    joint_ptr.copy_from_nonoverlapping(joints.unwrap().as_ptr(), joint_count);
                }
                self.context.device.unmap_memory(self.joint_buffer_memory);

                self.context.device.cmd_bind_descriptor_sets(
                    frame.command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline_layout,
                    0,
                    &[self.joint_descriptor_set],
                    &[],
                );

                // Bind texture descriptor set (set 1)
                let tex_set = mesh
                    .texture_descriptor_set
                    .unwrap_or(self.fallback_texture_descriptor_set);
                self.context.device.cmd_bind_descriptor_sets(
                    frame.command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline_layout,
                    1,
                    &[tex_set],
                    &[],
                );

                let mvp = frame_info.view_projection * draw.model;
                let push_constants = PushConstants {
                    mvp: matrix_to_push_constant(&mvp),
                    model: matrix_to_push_constant(&draw.model),
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

            self.context.device.cmd_end_rendering(frame.command_buffer);
            self.current_image_index = image_index;
        }

        Ok(())
    }
    pub fn update_command_buffer(&mut self) {}

    /// Applies new render settings, rebuilding the pipeline or updating sampler
    /// options where the corresponding settings have changed.
    pub fn update_render_settings(
        &mut self,
        new_settings: RenderingSettings,
    ) -> anyhow::Result<()> {
        let image_changed = new_settings.image_settings != self.settings.image_settings;
        let pipeline_changed = new_settings.depth_settings != self.settings.depth_settings
            || new_settings.rasterization_settings != self.settings.rasterization_settings
            || new_settings.default_vertex_shader != self.settings.default_vertex_shader
            || new_settings.default_fragment_shader != self.settings.default_fragment_shader;

        self.settings = new_settings;

        if image_changed {
            self.ui_renderer
                .update_sampler_options(self.settings.image_settings);
        }
        if pipeline_changed {
            self.recreate_pipeline()?;
        }
        Ok(())
    }

    /// Rebuilds the graphics pipeline from the current render settings.
    pub fn recreate_pipeline(&mut self) -> anyhow::Result<()> {
        unsafe {
            // Make sure no in-flight work references the old pipeline.
            self.context.device.device_wait_idle()?;
            self.context.device.destroy_pipeline(self.pipeline, None);
        }
        self.pipeline = create_graphics_pipeline(
            &self.context,
            self.pipeline_layout,
            &self.swapchain,
            &self.settings,
        )?;
        Ok(())
    }

    pub fn recreate_swapchain(&mut self) {}

    pub fn resize(&mut self) -> anyhow::Result<()> {
        self.swapchain.resize()?;
        self.recreate_present_semaphores()?;
        Ok(())
    }

    /// (Re)creates one present semaphore per swapchain image.
    /// The old semaphores are safe to destroy
    fn recreate_present_semaphores(&mut self) -> anyhow::Result<()> {
        unsafe {
            for semaphore in self.present_semaphores.drain(..) {
                self.context.device.destroy_semaphore(semaphore, None);
            }
            for _ in 0..self.swapchain.images.len() {
                self.present_semaphores.push(
                    self.context
                        .device
                        .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?,
                );
            }
        }
        Ok(())
    }

    // --- UI ---

    pub fn begin_ui(&mut self) {
        let mut state = self.ui_renderer.state.lock().unwrap();
        let raw_input = state.take_egui_input(&self.ui_renderer.window);
        self.ui_renderer.context.begin_pass(raw_input);
    }

    pub fn end_ui(&mut self) -> Result<()> {
        if self.present_semaphores.len() != self.swapchain.images.len() {
            self.recreate_present_semaphores()?;
        }

        let frame = &self.frames[self.current_frame];
        let mut full_output = self.ui_renderer.context.end_pass();
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
                .drain()
                .flat_map(|(id, deltas)| deltas.into_iter().map(move |delta| (id, delta)))
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

            self.context.transition_image_layout(
                frame.command_buffer,
                self.swapchain.images[self.current_image_index as usize],
                self.image_layouts.renderable,
                self.image_layouts.present,
                vk::ImageAspectFlags::COLOR,
            );

            self.context
                .device
                .end_command_buffer(frame.command_buffer)?;

            let present_semaphore = self.present_semaphores[self.current_image_index as usize];

            self.context.device.queue_submit(
                self.context.queues[self.context.queue_families.graphics as usize],
                &[ash::vk::SubmitInfo::default()
                    .wait_semaphores(&[frame.image_available_semaphore])
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                    .command_buffers(&[frame.command_buffer])
                    .signal_semaphores(&[present_semaphore])],
                frame.in_flight_fence,
            )?;

            self.swapchain
                .present_image(self.current_image_index, present_semaphore)?;
        }

        // Defer these frees until next frame (see `ui_pending_texture_frees`).
        self.ui_renderer.pending_texture_frees = full_output.textures_delta.free.drain().collect();
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

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.context.device.device_wait_idle();
        }
    }
}
