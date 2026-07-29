use anyhow::Result;

use crate::ecs::{World, systems::HasPriority};

/// A system that runs every frame.
#[derive(Clone, Copy)]
pub struct UpdateSystem {
    pub name: &'static str,
    pub func: fn(&mut World) -> Result<()>,
    pub priority: u32,
}
inventory::collect!(UpdateSystem);

impl HasPriority for UpdateSystem {
    fn priority(&self) -> u32 {
        self.priority
    }
}
