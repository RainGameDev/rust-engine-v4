use anyhow::Result;
use ash::vk::{self, Buffer, DescriptorSet, DeviceMemory, ImageView, Sampler};
use macros::Asset;

use crate::rendering::{
    core::vertex::Vertex,
    rendering_settings::RenderingSettings,
    vulkan::{VulkanRenderer, context::VulkanRenderingContext, image::ImageLayoutState},
};

#[derive(Clone, Debug)]
pub struct RawMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material_name: String,
}

#[derive(Clone, Debug)]
pub struct GpuModel {
    pub meshes: Vec<GpuMesh>,
    pub name: String,
}

#[derive(Asset, Debug, Clone, Default)]
pub struct GpuMesh {
    pub vertex_buffer: Buffer,
    pub vertex_buffer_memory: DeviceMemory,
    pub index_buffer: Buffer,
    pub index_buffer_memory: DeviceMemory,
    pub index_count: u32,
    pub material_name: String,
    pub texture_image: Option<vk::Image>,
    pub texture_memory: Option<DeviceMemory>,
    pub texture_image_view: Option<ImageView>,
    pub texture_sampler: Option<Sampler>,
    pub texture_descriptor_set: Option<DescriptorSet>,
}
pub fn cube_mesh(offset: [f32; 3], material_name: impl Into<String>) -> RawMesh {
    let [ox, oy, oz] = offset;

    let vertices = vec![
        // +X face
        Vertex {
            position: [ox + 1.0, oy - 1.0, oz - 1.0],
            normal: [1.0, 0.0, 0.0],
            uv: [0.0, 0.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy + 1.0, oz - 1.0],
            normal: [1.0, 0.0, 0.0],
            uv: [1.0, 0.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy + 1.0, oz + 1.0],
            normal: [1.0, 0.0, 0.0],
            uv: [1.0, 1.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy - 1.0, oz + 1.0],
            normal: [1.0, 0.0, 0.0],
            uv: [0.0, 1.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        // -X face
        Vertex {
            position: [ox - 1.0, oy - 1.0, oz + 1.0],
            normal: [-1.0, 0.0, 0.0],
            uv: [0.0, 0.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox - 1.0, oy + 1.0, oz + 1.0],
            normal: [-1.0, 0.0, 0.0],
            uv: [1.0, 0.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox - 1.0, oy + 1.0, oz - 1.0],
            normal: [-1.0, 0.0, 0.0],
            uv: [1.0, 1.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox - 1.0, oy - 1.0, oz - 1.0],
            normal: [-1.0, 0.0, 0.0],
            uv: [0.0, 1.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        // +Y face
        Vertex {
            position: [ox - 1.0, oy + 1.0, oz - 1.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox - 1.0, oy + 1.0, oz + 1.0],
            normal: [0.0, 1.0, 0.0],
            uv: [1.0, 0.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy + 1.0, oz + 1.0],
            normal: [0.0, 1.0, 0.0],
            uv: [1.0, 1.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy + 1.0, oz - 1.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 1.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        // -Y face
        Vertex {
            position: [ox - 1.0, oy - 1.0, oz + 1.0],
            normal: [0.0, -1.0, 0.0],
            uv: [0.0, 0.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox - 1.0, oy - 1.0, oz - 1.0],
            normal: [0.0, -1.0, 0.0],
            uv: [1.0, 0.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy - 1.0, oz - 1.0],
            normal: [0.0, -1.0, 0.0],
            uv: [1.0, 1.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy - 1.0, oz + 1.0],
            normal: [0.0, -1.0, 0.0],
            uv: [0.0, 1.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        // +Z face
        Vertex {
            position: [ox - 1.0, oy - 1.0, oz + 1.0],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy - 1.0, oz + 1.0],
            normal: [0.0, 0.0, 1.0],
            uv: [1.0, 0.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy + 1.0, oz + 1.0],
            normal: [0.0, 0.0, 1.0],
            uv: [1.0, 1.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox - 1.0, oy + 1.0, oz + 1.0],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 1.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        // -Z face
        Vertex {
            position: [ox + 1.0, oy - 1.0, oz - 1.0],
            normal: [0.0, 0.0, -1.0],
            uv: [0.0, 0.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox - 1.0, oy - 1.0, oz - 1.0],
            normal: [0.0, 0.0, -1.0],
            uv: [1.0, 0.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox - 1.0, oy + 1.0, oz - 1.0],
            normal: [0.0, 0.0, -1.0],
            uv: [1.0, 1.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy + 1.0, oz - 1.0],
            normal: [0.0, 0.0, -1.0],
            uv: [0.0, 1.0],
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        },
    ];

    let indices: Vec<u32> = vec![
        0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16, 17,
        18, 16, 18, 19, 20, 21, 22, 20, 22, 23,
    ];

    RawMesh {
        vertices,
        indices,
        material_name: material_name.into(),
    }
}

pub fn raw_mesh_to_gpu_mesh(
    raw_mesh: RawMesh,
    renderer: &VulkanRenderer,
    context: &VulkanRenderingContext,
) -> Result<GpuMesh> {
    let vertex_buffer =
        context.create_vertex_buffer(raw_mesh.vertices.as_slice(), renderer.command_pool)?;

    let index_buffer = context.create_index_buffer(&raw_mesh.indices, renderer.command_pool)?;

    Ok(GpuMesh {
        vertex_buffer: vertex_buffer.0,
        vertex_buffer_memory: vertex_buffer.1,
        index_buffer: index_buffer.0,
        index_buffer_memory: index_buffer.1,
        index_count: raw_mesh.indices.len() as u32,
        material_name: raw_mesh.material_name,
        texture_image: None,
        texture_memory: None,
        texture_image_view: None,
        texture_sampler: None,
        texture_descriptor_set: None,
    })
}

/// Creates a GpuMesh with a texture uploaded to the GPU.
pub fn raw_mesh_to_gpu_mesh_with_texture(
    raw_mesh: RawMesh,
    pixels: &[u8],
    width: u32,
    height: u32,
    renderer: &VulkanRenderer,
    context: &VulkanRenderingContext,
    settings: &RenderingSettings,
) -> Result<GpuMesh> {
    let vertex_buffer =
        context.create_vertex_buffer(raw_mesh.vertices.as_slice(), renderer.command_pool)?;
    let index_buffer = context.create_index_buffer(&raw_mesh.indices, renderer.command_pool)?;

    // Create texture image
    let format = vk::Format::R8G8B8A8_SRGB;
    let (texture_image, texture_memory) = context.create_image(
        vk::Extent2D { width, height },
        format,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    // Create staging buffer with pixel data
    let image_size = (width * height * 4) as vk::DeviceSize;
    let (staging_buffer, staging_memory) = context.create_buffer(
        image_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    unsafe {
        let ptr =
            context
                .device
                .map_memory(staging_memory, 0, image_size, vk::MemoryMapFlags::empty())?
                as *mut u8;
        ptr.copy_from_nonoverlapping(pixels.as_ptr(), pixels.len());
        context.device.unmap_memory(staging_memory);
    }

    // Transition to transfer dst, copy, transition to shader read
    let cmd = context.begin_single_time_commands(renderer.command_pool);
    context.transition_image_layout(
        cmd,
        texture_image,
        ImageLayoutState {
            layout: vk::ImageLayout::UNDEFINED,
            access_mask: vk::AccessFlags::empty(),
            stage_mask: vk::PipelineStageFlags::TOP_OF_PIPE,
            queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        },
        ImageLayoutState {
            layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            access_mask: vk::AccessFlags::TRANSFER_WRITE,
            stage_mask: vk::PipelineStageFlags::TRANSFER,
            queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        },
        vk::ImageAspectFlags::COLOR,
    );
    context.copy_buffer_to_image(cmd, staging_buffer, texture_image, width, height);
    context.transition_image_layout(
        cmd,
        texture_image,
        ImageLayoutState {
            layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            access_mask: vk::AccessFlags::TRANSFER_WRITE,
            stage_mask: vk::PipelineStageFlags::TRANSFER,
            queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        },
        ImageLayoutState {
            layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            access_mask: vk::AccessFlags::SHADER_READ,
            stage_mask: vk::PipelineStageFlags::FRAGMENT_SHADER,
            queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        },
        vk::ImageAspectFlags::COLOR,
    );
    let queue = context.queues[context.queue_families.transfer as usize];
    context.end_single_time_commands(cmd, queue, renderer.command_pool);

    // Cleanup staging buffer
    unsafe {
        context.device.destroy_buffer(staging_buffer, None);
        context.device.free_memory(staging_memory, None);
    }

    let texture_image_view =
        context.create_image_view(texture_image, format, vk::ImageAspectFlags::COLOR)?;
    let texture_sampler = context.create_sampler(settings)?;

    // Allocate and write descriptor set for this texture
    let texture_descriptor_set = context.allocate_descriptor_set(
        renderer.texture_descriptor_pool,
        renderer.texture_descriptor_set_layout,
    )?;

    unsafe {
        context.device.update_descriptor_sets(
            &[vk::WriteDescriptorSet::default()
                .dst_set(texture_descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&[vk::DescriptorImageInfo::default()
                    .sampler(texture_sampler)
                    .image_view(texture_image_view)
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)])],
            &[],
        );
    }

    Ok(GpuMesh {
        vertex_buffer: vertex_buffer.0,
        vertex_buffer_memory: vertex_buffer.1,
        index_buffer: index_buffer.0,
        index_buffer_memory: index_buffer.1,
        index_count: raw_mesh.indices.len() as u32,
        material_name: raw_mesh.material_name,
        texture_image: Some(texture_image),
        texture_memory: Some(texture_memory),
        texture_image_view: Some(texture_image_view),
        texture_sampler: Some(texture_sampler),
        texture_descriptor_set: Some(texture_descriptor_set),
    })
}
