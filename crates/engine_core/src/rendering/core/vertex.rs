use ash::vk;
use macros::vertex;

/// Trait that allows for multiple types of vertex;
pub trait VertexDefinition {
    fn get_binding_description() -> vk::VertexInputBindingDescription;
    fn get_attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription>;
    fn vertex_type_name() -> &'static str;
}

pub struct VertexTypeInfo {
    pub name: &'static str,
    pub binding_description: fn() -> vk::VertexInputBindingDescription,
    pub attribute_descriptions: fn() -> Vec<vk::VertexInputAttributeDescription>,
    pub size: usize,
    /// Optional vertex shader override. When `None` the renderer falls back to
    /// `RenderingSettings::default_vertex_shader`.
    pub vertex_shader: Option<&'static str>,
}

inventory::collect!(VertexTypeInfo);

/// Default model vertex
#[vertex]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    #[format(R16G16B16A16_UINT)]
    pub joints: [u16; 4], // offset 32
    pub weights: [f32; 4], // offset 40
}
