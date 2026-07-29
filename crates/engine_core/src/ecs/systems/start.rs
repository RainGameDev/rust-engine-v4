use anyhow::Result;

use crate::ecs::{World, systems::HasPriority};

/// A system that runs once at startup.
#[derive(Clone, Copy)]
pub struct StartSystem {
    pub name: &'static str,
    pub func: fn(&mut World) -> Result<()>,
    pub priority: u32,
}
inventory::collect!(StartSystem);

impl HasPriority for StartSystem {
    fn priority(&self) -> u32 {
        self.priority
    }
}
