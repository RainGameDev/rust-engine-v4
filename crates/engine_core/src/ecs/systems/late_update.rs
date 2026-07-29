use anyhow::Result;

use crate::ecs::{World, systems::HasPriority};

/// A system that runs at the end of every frame.
#[derive(Clone, Copy)]
pub struct LateUpdateSystem {
    pub name: &'static str,
    pub func: fn(&mut World) -> Result<()>,
    pub priority: u32,
}
inventory::collect!(LateUpdateSystem);

impl HasPriority for LateUpdateSystem {
    fn priority(&self) -> u32 {
        self.priority
    }
}
