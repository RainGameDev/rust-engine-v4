use std::os::linux::raw;

use anyhow::Result;
use ash::vk::{Buffer, DeviceMemory};

use crate::rendering::{
    core::vertex::Vertex,
    vulkan::{VulkanRenderer, context::VulkanRenderingContext},
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

#[derive(Debug, Clone, Default)]
pub struct GpuMesh {
    pub vertex_buffer: Buffer,
    pub vertex_buffer_memory: DeviceMemory,
    pub index_buffer: Buffer,
    pub index_buffer_memory: DeviceMemory,
    pub index_count: u32,
    pub material_name: String,
}

#[derive(Clone)]
pub struct ModelRenderer {
    pub loaded_model: String,
    // pub material_path: String,
}

pub fn cube_mesh(offset: [f32; 3], material_name: impl Into<String>) -> RawMesh {
    let [ox, oy, oz] = offset;

    let vertices = vec![
        // +X face
        Vertex {
            position: [ox + 1.0, oy - 1.0, oz - 1.0],
            normal: [1.0, 0.0, 0.0],
            uv: [0.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy + 1.0, oz - 1.0],
            normal: [1.0, 0.0, 0.0],
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy + 1.0, oz + 1.0],
            normal: [1.0, 0.0, 0.0],
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [ox + 1.0, oy - 1.0, oz + 1.0],
            normal: [1.0, 0.0, 0.0],
            uv: [0.0, 1.0],
        },
        // -X face
        Vertex {
            position: [ox - 1.0, oy - 1.0, oz + 1.0],
            normal: [-1.0, 0.0, 0.0],
            uv: [0.0, 0.0],
        },
        Vertex {
            position: [ox - 1.0, oy + 1.0, oz + 1.0],
            normal: [-1.0, 0.0, 0.0],
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [ox - 1.0, oy + 1.0, oz - 1.0],
            normal: [-1.0, 0.0, 0.0],
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [ox - 1.0, oy - 1.0, oz - 1.0],
            normal: [-1.0, 0.0, 0.0],
            uv: [0.0, 1.0],
        },
        // +Y face
        Vertex {
            position: [ox - 1.0, oy + 1.0, oz - 1.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
        },
        Vertex {
            position: [ox - 1.0, oy + 1.0, oz + 1.0],
            normal: [0.0, 1.0, 0.0],
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy + 1.0, oz + 1.0],
            normal: [0.0, 1.0, 0.0],
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [ox + 1.0, oy + 1.0, oz - 1.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 1.0],
        },
        // -Y face
        Vertex {
            position: [ox - 1.0, oy - 1.0, oz + 1.0],
            normal: [0.0, -1.0, 0.0],
            uv: [0.0, 0.0],
        },
        Vertex {
            position: [ox - 1.0, oy - 1.0, oz - 1.0],
            normal: [0.0, -1.0, 0.0],
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy - 1.0, oz - 1.0],
            normal: [0.0, -1.0, 0.0],
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [ox + 1.0, oy - 1.0, oz + 1.0],
            normal: [0.0, -1.0, 0.0],
            uv: [0.0, 1.0],
        },
        // +Z face
        Vertex {
            position: [ox - 1.0, oy - 1.0, oz + 1.0],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy - 1.0, oz + 1.0],
            normal: [0.0, 0.0, 1.0],
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [ox + 1.0, oy + 1.0, oz + 1.0],
            normal: [0.0, 0.0, 1.0],
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [ox - 1.0, oy + 1.0, oz + 1.0],
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 1.0],
        },
        // -Z face
        Vertex {
            position: [ox + 1.0, oy - 1.0, oz - 1.0],
            normal: [0.0, 0.0, -1.0],
            uv: [0.0, 0.0],
        },
        Vertex {
            position: [ox - 1.0, oy - 1.0, oz - 1.0],
            normal: [0.0, 0.0, -1.0],
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [ox - 1.0, oy + 1.0, oz - 1.0],
            normal: [0.0, 0.0, -1.0],
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [ox + 1.0, oy + 1.0, oz - 1.0],
            normal: [0.0, 0.0, -1.0],
            uv: [0.0, 1.0],
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
    })
}
