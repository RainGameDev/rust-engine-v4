use std::{collections::HashSet, io};

use anyhow::Result;
use ash::{
    Device, Entry, Instance,
    khr::*,
    vk::{
        self, AttachmentLoadOp, AttachmentStoreOp, ClearValue, CompareOp, ImageLayout, Queue,
        RenderingAttachmentInfo,
    },
};
use winit::raw_window_handle::HasWindowHandle;
use winit::{raw_window_handle::HasDisplayHandle, window::Window};

use crate::rendering::{
    core::vertex::{Vertex, VertexDefinition},
    vulkan::{
        debug::{DebugUtils, VALIDATION_LAYER_NAME, is_validation_layer_available},
        device::PhysicalDevice,
        image::ImageLayoutState,
        queue::{QueueFamilies, QueueFamily, QueueFamilyPicker},
        surface::Surface,
    },
};

pub struct RenderingContextAttributes<'window> {
    pub compatability_window: &'window Window,
    pub queue_family_picker: QueueFamilyPicker,
}

#[derive(Clone)]
pub struct VulkanRenderingContext {
    // What queues are present.
    pub queues: Vec<Queue>,

    /// Whats the current device.
    pub device: Device,
    /// Whats the current physical device
    pub physical_device: PhysicalDevice,

    /// Queue family indices
    pub queue_family_indices: HashSet<u32>,

    pub queue_families: QueueFamilies,
    /// The current vulkan surface instance.
    pub surface_extension: surface::Instance,
    /// The current vulkan instance.
    pub instance: Instance,
    /// The vulkan entry point.
    pub entry: Entry,
    /// The debug utils extension and messenger, when validation is available.
    pub debug_utils: Option<DebugUtils>,
    /// The vulkan swapchain device.
    pub swapchain_extension: swapchain::Device,
}

impl VulkanRenderingContext {
    pub fn new(attributes: RenderingContextAttributes) -> Result<VulkanRenderingContext> {
        unsafe {
            let entry = Entry::load()?;

            // get the windows raw handle info.
            let raw_display_handle = attributes.compatability_window.display_handle()?.as_raw();
            let raw_window_handle = attributes.compatability_window.window_handle()?.as_raw();

            // create a vulkan instance
            let mut extensions =
                ash_window::enumerate_required_extensions(raw_display_handle)?.to_vec();
            extensions.push(ash::vk::EXT_DEBUG_UTILS_NAME.as_ptr());

            let validation_layer_available = is_validation_layer_available(&entry);

            // must outlive `create_instance`, as the instance create info references it.
            let mut messenger_create_info = DebugUtils::messenger_create_info();
            let application_info = &vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_3);
            let validation_layers = [VALIDATION_LAYER_NAME.as_ptr()];

            let mut instance_create_info = vk::InstanceCreateInfo::default()
                .application_info(application_info)
                .enabled_extension_names(&extensions);

            if validation_layer_available {
                instance_create_info = instance_create_info
                    .enabled_layer_names(&validation_layers)
                    .push_next(&mut messenger_create_info);
            }

            let instance = entry.create_instance(&instance_create_info, None)?;

            // install the debug messenger, if the validation layer is available.
            let debug_utils = DebugUtils::new(&entry, &instance);

            // get the surface info
            let surface_extension = ash::khr::surface::Instance::new(&entry, &instance);
            let compatability_surface = ash_window::create_surface(
                &entry,
                &instance,
                raw_display_handle,
                raw_window_handle,
                None,
            )?;

            // pick the most suitable physical device.
            let mut physical_devices = instance
                .enumerate_physical_devices()?
                .into_iter()
                .map(|handle| {
                    let properties = instance.get_physical_device_properties(handle);
                    let features = instance.get_physical_device_features(handle);
                    let memory_properties = instance.get_physical_device_memory_properties(handle);
                    let queue_family_properties =
                        instance.get_physical_device_queue_family_properties(handle);

                    let queue_families = queue_family_properties
                        .into_iter()
                        .enumerate()
                        .map(|(index, properties)| QueueFamily {
                            index: index as u32,
                            properties,
                        })
                        .collect::<Vec<_>>();

                    PhysicalDevice {
                        handle,
                        properties,
                        features,
                        memory_properties,
                        queue_families,
                    }
                })
                .collect::<Vec<_>>();

            physical_devices.retain(|device| {
                surface_extension
                    .get_physical_device_surface_support(device.handle, 0, compatability_surface)
                    .unwrap_or(false)
            });

            // destroy the compatability surface.
            surface_extension.destroy_surface(compatability_surface, None);

            // get the physical device
            let (physical_device, queue_family) =
                (attributes.queue_family_picker)(physical_devices)?;

            // create the queue indices.
            let queue_family_indices = HashSet::from([
                queue_family.graphics,
                queue_family.present,
                queue_family.transfer,
                queue_family.compute,
            ]);

            // create the queues
            let queue_create_infos = queue_family_indices
                .iter()
                .copied()
                .map(|index| {
                    vk::DeviceQueueCreateInfo::default()
                        .queue_family_index(index)
                        .queue_priorities(&[1.0])
                })
                .collect::<Vec<_>>();

            let device = instance.create_device(
                physical_device.handle,
                &vk::DeviceCreateInfo::default()
                    .queue_create_infos(&queue_create_infos)
                    .enabled_extension_names(&[ash::khr::swapchain::NAME.as_ptr()])
                    .push_next(
                        &mut vk::PhysicalDeviceDynamicRenderingFeatures::default()
                            .dynamic_rendering(true),
                    )
                    .push_next(
                        &mut vk::PhysicalDeviceBufferDeviceAddressFeatures::default()
                            .buffer_device_address(true),
                    ),
                None,
            )?;

            // create the swapchain extension
            let swapchain_extension = ash::khr::swapchain::Device::new(&instance, &device);

            let queues = queue_family_indices
                .iter()
                .map(|&index| device.get_device_queue(index, 0))
                .collect::<Vec<_>>();

            Ok(Self {
                queues,
                device,
                queue_family_indices,
                queue_families: queue_family,
                physical_device,
                surface_extension,
                instance,
                entry,
                debug_utils,
                swapchain_extension,
            })
        }
    }

    /// Creates a new surface,
    /// UNSAFE: unsafe due to the window needing to outlast it's surface,
    pub fn create_surface(&self, window: &Window) -> Result<Surface> {
        unsafe {
            // get the raw info of the winit window
            let raw_display_handle = window.display_handle()?.as_raw();
            let raw_window_handle = window.window_handle()?.as_raw();

            // create the vulkan surface
            let handle = ash_window::create_surface(
                &self.entry,
                &self.instance,
                raw_display_handle,
                raw_window_handle,
                None,
            )?;

            // get the capabilities of the surface
            let capabilities = self
                .surface_extension
                .get_physical_device_surface_capabilities(self.physical_device.handle, handle)?;

            // get the formats of the surface
            let formats = self
                .surface_extension
                .get_physical_device_surface_formats(self.physical_device.handle, handle)?;

            // get the present modes for the surface
            let present_modes = self
                .surface_extension
                .get_physical_device_surface_present_modes(self.physical_device.handle, handle)?;

            Ok(Surface {
                handle,
                capabilities,
                formats,
                present_modes,
            })
        }
    }

    pub fn create_graphics_pipeline(
        &self,
        vertex_shader: vk::ShaderModule,
        fragment_shader: vk::ShaderModule,
        image_extent: vk::Extent2D,
        image_format: vk::Format,
        pipeline_layout: vk::PipelineLayout,
        depth_format: vk::Format,
    ) -> Result<vk::Pipeline> {
        let entry_point = std::ffi::CString::new("main").unwrap();

        let bindings = vec![Vertex::get_binding_description()];

        let attributes = Vertex::get_attribute_descriptions();

        unsafe {
            Ok(self
                .device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[vk::GraphicsPipelineCreateInfo::default()
                        .stages(&[
                            vk::PipelineShaderStageCreateInfo::default()
                                .stage(vk::ShaderStageFlags::VERTEX)
                                .module(vertex_shader)
                                .name(&entry_point),
                            vk::PipelineShaderStageCreateInfo::default()
                                .stage(vk::ShaderStageFlags::FRAGMENT)
                                .module(fragment_shader)
                                .name(&entry_point),
                        ])
                        .vertex_input_state(
                            &vk::PipelineVertexInputStateCreateInfo::default()
                                .vertex_binding_descriptions(&bindings)
                                .vertex_attribute_descriptions(&attributes),
                        )
                        .input_assembly_state(
                            &vk::PipelineInputAssemblyStateCreateInfo::default()
                                .topology(vk::PrimitiveTopology::TRIANGLE_LIST),
                        )
                        .viewport_state(
                            &vk::PipelineViewportStateCreateInfo::default()
                                .viewports(&[vk::Viewport {
                                    x: 0.0,
                                    y: 0.0,
                                    width: image_extent.width as f32,
                                    height: image_extent.height as f32,
                                    min_depth: 0.0,
                                    max_depth: 1.0,
                                }])
                                .scissors(&[vk::Rect2D {
                                    offset: vk::Offset2D { x: 0, y: 0 },
                                    extent: image_extent,
                                }]),
                        )
                        .rasterization_state(
                            &vk::PipelineRasterizationStateCreateInfo::default()
                                .depth_clamp_enable(false)
                                .rasterizer_discard_enable(false)
                                .polygon_mode(vk::PolygonMode::FILL)
                                .cull_mode(vk::CullModeFlags::NONE)
                                .front_face(vk::FrontFace::CLOCKWISE)
                                .depth_bias_enable(false)
                                .line_width(1.0),
                        )
                        .multisample_state(
                            &vk::PipelineMultisampleStateCreateInfo::default()
                                .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                                .sample_shading_enable(false),
                        )
                        .color_blend_state(
                            &vk::PipelineColorBlendStateCreateInfo::default().attachments(&[
                                vk::PipelineColorBlendAttachmentState::default()
                                    .color_write_mask(vk::ColorComponentFlags::RGBA)
                                    .blend_enable(false),
                            ]),
                        )
                        .dynamic_state(
                            &vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&[
                                vk::DynamicState::VIEWPORT,
                                vk::DynamicState::SCISSOR,
                            ]),
                        )
                        .depth_stencil_state(
                            &vk::PipelineDepthStencilStateCreateInfo::default()
                                .depth_test_enable(true)
                                .depth_write_enable(true)
                                .depth_compare_op(CompareOp::LESS),
                        )
                        .layout(pipeline_layout)
                        .render_pass(vk::RenderPass::null())
                        .push_next(
                            &mut vk::PipelineRenderingCreateInfo::default()
                                .color_attachment_formats(&[image_format])
                                .depth_attachment_format(depth_format),
                        )],
                    None,
                )
                .unwrap()
                .into_iter()
                .next()
                .unwrap())
        }
    }

    pub fn create_shader_module(&self, code: &[u8]) -> Result<vk::ShaderModule, vk::Result> {
        let mut code = io::Cursor::new(code);
        let code = ash::util::read_spv(&mut code).unwrap();
        let create_info = vk::ShaderModuleCreateInfo::default().code(&code);
        let shader_module = unsafe { self.device.create_shader_module(&create_info, None) }?;
        Ok(shader_module)
    }

    /// Creates a descriptor set layout from the given bindings.
    pub fn create_descriptor_set_layout(
        &self,
        bindings: &[vk::DescriptorSetLayoutBinding],
    ) -> Result<vk::DescriptorSetLayout> {
        let layout = unsafe {
            self.device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default().bindings(bindings),
                None,
            )?
        };
        Ok(layout)
    }

    /// Creates a descriptor pool with the given sizes, able to allocate `max_sets` sets.
    pub fn create_descriptor_pool(
        &self,
        pool_sizes: &[vk::DescriptorPoolSize],
        max_sets: u32,
    ) -> Result<vk::DescriptorPool> {
        let pool = unsafe {
            self.device.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::default()
                    .pool_sizes(pool_sizes)
                    .max_sets(max_sets),
                None,
            )?
        };
        Ok(pool)
    }

    /// Allocates a single descriptor set from `pool` using `layout`.
    pub fn allocate_descriptor_set(
        &self,
        pool: vk::DescriptorPool,
        layout: vk::DescriptorSetLayout,
    ) -> Result<vk::DescriptorSet> {
        let set = unsafe {
            self.device
                .allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::default()
                        .descriptor_pool(pool)
                        .set_layouts(&[layout]),
                )?
                .into_iter()
                .next()
                .unwrap()
        };
        Ok(set)
    }

    pub fn create_image_view(
        &self,
        image: vk::Image,
        format: vk::Format,
        aspect_flags: vk::ImageAspectFlags,
    ) -> Result<vk::ImageView> {
        let image_view = unsafe {
            self.device.create_image_view(
                &vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format)
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(aspect_flags)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    ),
                None,
            )
        }?;
        Ok(image_view)
    }

    /// Creates a vulkan image
    pub fn create_image(
        &self,
        extent: vk::Extent2D,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Image, vk::DeviceMemory)> {
        // create the Create info.
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(tiling)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .samples(vk::SampleCountFlags::TYPE_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        // use the device to create it.
        let image = unsafe { self.device.create_image(&image_info, None).unwrap() };

        // get the images memory requiremetns
        let mem_reqs = unsafe { self.device.get_image_memory_requirements(image) };

        //get the allocation info of the image.
        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_reqs.size)
            .memory_type_index(self.find_memory_type(mem_reqs.memory_type_bits, properties)?);

        // allocate the memory.
        let memory = unsafe { self.device.allocate_memory(&alloc_info, None).unwrap() };
        unsafe { self.device.bind_image_memory(image, memory, 0).unwrap() };

        Ok((image, memory))
    }

    /// Transitions the render image type from one to the new one.
    pub fn transition_image_layout(
        &self,
        command_buffer: vk::CommandBuffer,
        image: vk::Image,
        old_state: ImageLayoutState,
        new_state: ImageLayoutState,
        aspect_mask: vk::ImageAspectFlags,
    ) {
        let image_memory_barrier = vk::ImageMemoryBarrier::default()
            .old_layout(old_state.layout)
            .new_layout(new_state.layout)
            .image(image)
            .src_access_mask(old_state.access_mask)
            .dst_access_mask(new_state.access_mask)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(aspect_mask)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1),
            );

        // bind the new image layout.
        unsafe {
            self.device.cmd_pipeline_barrier(
                command_buffer,
                old_state.stage_mask,
                new_state.stage_mask,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_memory_barrier],
            );
        }
    }

    /// Begins vulkan rendering.
    pub fn begin_rendering(
        &self,
        command_buffer: vk::CommandBuffer,
        view: vk::ImageView,
        depth_view: vk::ImageView,
        clear_color: vk::ClearColorValue,
        render_area: vk::Rect2D,
    ) {
        unsafe {
            self.device.cmd_begin_rendering(
                command_buffer,
                &vk::RenderingInfo::default()
                    .layer_count(1)
                    .color_attachments(&[RenderingAttachmentInfo::default()
                        .image_view(view)
                        .image_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .clear_value(ClearValue { color: clear_color })
                        .load_op(AttachmentLoadOp::CLEAR)
                        .store_op(AttachmentStoreOp::STORE)])
                    .depth_attachment(
                        &RenderingAttachmentInfo::default()
                            .image_view(depth_view)
                            .image_layout(ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
                            .load_op(AttachmentLoadOp::LOAD)
                            .store_op(AttachmentStoreOp::STORE),
                    )
                    .render_area(render_area),
            );
        }
    }

    /// Finds a memory type based on the filter.
    pub fn find_memory_type(
        &self,
        filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<u32> {
        // try to find an exact match
        for i in 0..self.physical_device.memory_properties.memory_type_count {
            if (filter & (1 << i)) != 0
                && (self.physical_device.memory_properties.memory_types[i as usize].property_flags
                    & properties)
                    == properties
            {
                return Ok(i);
            }
        }

        // find any memory type that matches the filter
        for i in 0..self.physical_device.memory_properties.memory_type_count {
            if (filter & (1 << i)) != 0 {
                return Ok(i);
            }
        }

        Err(anyhow::anyhow!(
            "Failed to find suitable memory type with filter: {}",
            filter
        ))
    }

    /// Finds the first candidate format that supports the given features for
    /// the given tiling on the current physical device.
    pub fn find_supported_format(
        &self,
        candidates: &[vk::Format],
        tiling: vk::ImageTiling,
        features: vk::FormatFeatureFlags,
    ) -> Result<vk::Format> {
        candidates
            .iter()
            .copied()
            .find(|&format| {
                let props = unsafe {
                    self.instance
                        .get_physical_device_format_properties(self.physical_device.handle, format)
                };
                let supported = match tiling {
                    vk::ImageTiling::LINEAR => props.linear_tiling_features,
                    vk::ImageTiling::OPTIMAL => props.optimal_tiling_features,
                    _ => props.linear_tiling_features | props.optimal_tiling_features,
                };
                supported.contains(features)
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Failed to find a format supporting the required features for tiling {:?}",
                    tiling
                )
            })
    }

    pub fn begin_single_time_commands(&self, command_pool: vk::CommandPool) -> vk::CommandBuffer {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool)
            .command_buffer_count(1);

        let cmd_buf = unsafe { self.device.allocate_command_buffers(&alloc_info).unwrap()[0] };
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.device
                .begin_command_buffer(cmd_buf, &begin_info)
                .unwrap()
        };
        cmd_buf
    }

    pub fn end_single_time_commands(
        &self,
        cmd_buf: vk::CommandBuffer,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
    ) {
        unsafe {
            self.device.end_command_buffer(cmd_buf).unwrap();

            let buffer = &[cmd_buf];
            let submit_info = vk::SubmitInfo::default().command_buffers(buffer);
            self.device
                .queue_submit(queue, &[submit_info], vk::Fence::null())
                .unwrap();
            self.device.queue_wait_idle(queue).unwrap();
            self.device.free_command_buffers(command_pool, &[cmd_buf]);
        }
    }
    pub fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { self.device.create_buffer(&buffer_info, None)? };
        let mem_requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(
                self.find_memory_type(mem_requirements.memory_type_bits, properties)?,
            );

        let memory = unsafe { self.device.allocate_memory(&alloc_info, None)? };

        unsafe { self.device.bind_buffer_memory(buffer, memory, 0)? };

        Ok((buffer, memory))
    }

    pub fn create_vertex_buffer<T: VertexDefinition>(
        &self,
        vertices: &[T],
        command_pool: vk::CommandPool,
    ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        let buffer_size = (size_of::<T>() * vertices.len()) as vk::DeviceSize;

        // Create staging buffer
        let (staging_buffer, staging_memory) = self.create_buffer(
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        unsafe {
            let data_ptr = self.device.map_memory(
                staging_memory,
                0,
                buffer_size,
                vk::MemoryMapFlags::empty(),
            )? as *mut T;
            data_ptr.copy_from_nonoverlapping(vertices.as_ptr(), vertices.len());
            self.device.unmap_memory(staging_memory);
        }

        // Create device local buffer
        let buffer_info = vk::BufferCreateInfo::default()
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { self.device.create_buffer(&buffer_info, None)? };
        let mem_requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(self.find_memory_type(
                mem_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?);

        let buffer_memory = unsafe { self.device.allocate_memory(&alloc_info, None)? };
        unsafe { self.device.bind_buffer_memory(buffer, buffer_memory, 0)? };

        // Copy staging -> device local
        let cmd = self.begin_single_time_commands(command_pool);
        unsafe {
            let copy_region = vk::BufferCopy::default().size(buffer_size);
            self.device
                .cmd_copy_buffer(cmd, staging_buffer, buffer, &[copy_region]);
        }
        let queue = self.queues[self.queue_families.transfer as usize];
        self.end_single_time_commands(cmd, queue, command_pool);

        unsafe {
            self.device.destroy_buffer(staging_buffer, None);
            self.device.free_memory(staging_memory, None);
        }

        Ok((buffer, buffer_memory))
    }

    /// Creates an index buffer from a slice of indices
    pub fn create_index_buffer(
        &self,
        indices: &[u32],
        command_pool: vk::CommandPool,
    ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        let buffer_size = (std::mem::size_of::<u32>() * indices.len()) as vk::DeviceSize;

        // staging
        let (staging_buffer, staging_memory) = self.create_buffer(
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        unsafe {
            let data_ptr = self.device.map_memory(
                staging_memory,
                0,
                buffer_size,
                vk::MemoryMapFlags::empty(),
            )? as *mut u32;
            data_ptr.copy_from_nonoverlapping(indices.as_ptr(), indices.len());
            self.device.unmap_memory(staging_memory);
        }

        // device local
        let buffer_info = vk::BufferCreateInfo::default()
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { self.device.create_buffer(&buffer_info, None)? };
        let mem_requirements = unsafe { self.device.get_buffer_memory_requirements(buffer) };

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_requirements.size)
            .memory_type_index(self.find_memory_type(
                mem_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )?);

        let buffer_memory = unsafe { self.device.allocate_memory(&alloc_info, None)? };
        unsafe { self.device.bind_buffer_memory(buffer, buffer_memory, 0)? };

        // copy
        let cmd = self.begin_single_time_commands(command_pool);
        unsafe {
            let copy_region = vk::BufferCopy::default().size(buffer_size);
            self.device
                .cmd_copy_buffer(cmd, staging_buffer, buffer, &[copy_region]);
        }
        let queue = self.queues[self.queue_families.transfer as usize];
        self.end_single_time_commands(cmd, queue, command_pool);

        unsafe {
            self.device.destroy_buffer(staging_buffer, None);
            self.device.free_memory(staging_memory, None);
        }

        Ok((buffer, buffer_memory))
    }
}

pub fn depth_image_aspect(format: vk::Format) -> vk::ImageAspectFlags {
    if matches!(
        format,
        vk::Format::D32_SFLOAT_S8_UINT
            | vk::Format::D24_UNORM_S8_UINT
            | vk::Format::D16_UNORM_S8_UINT
    ) {
        vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
    } else {
        vk::ImageAspectFlags::DEPTH
    }
}
