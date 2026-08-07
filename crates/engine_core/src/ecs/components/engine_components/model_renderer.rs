use macros::component;

use crate::{assets::core::handle::Handle, rendering::core::model::GpuMesh};

/// TODO: Give a reference to a model asset handle
#[component]
pub struct ModelRenderer {
    pub model: Handle<GpuMesh>,
}
