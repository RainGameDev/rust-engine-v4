use anyhow::Result;

use crate::ecs::{World, systems::HasPriority};

/// A system that runs before each frame's render pass.
#[derive(Clone, Copy)]
pub struct PreRenderSystem {
    pub name: &'static str,
    pub func: fn(&mut World) -> Result<()>,
    pub priority: u32,
}
inventory::collect!(PreRenderSystem);

impl HasPriority for PreRenderSystem {
    fn priority(&self) -> u32 {
        self.priority
    }
}
