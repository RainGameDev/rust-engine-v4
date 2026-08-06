use macros::Component;

use crate::{assets::core::handle::Handle, rendering::core::model::GpuMesh};

/// TODO: Give a reference to a model asset handle
#[derive(Component, Clone, Debug)]
pub struct ModelRenderer {
    pub model: Handle<GpuMesh>,
}
