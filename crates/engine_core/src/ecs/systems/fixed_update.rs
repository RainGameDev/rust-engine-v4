use anyhow::Result;
use macros::Resource;

use crate::ecs::{World, systems::HasPriority};

/// A system that runs at a fixed timestep (20 Hz by default).
#[derive(Clone, Copy)]
pub struct FixedUpdateSystem {
    pub name: &'static str,
    pub func: fn(&mut World, delta: f32) -> Result<()>,
    pub priority: u32,
}
inventory::collect!(FixedUpdateSystem);

impl HasPriority for FixedUpdateSystem {
    fn priority(&self) -> u32 {
        self.priority
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct DeltaTime(pub f32);

#[derive(Resource, Debug, Clone, Default)]
pub struct EngineTimer(pub f32);

#[derive(Resource, Debug, Clone, Default)]
pub struct FixedUpdateTimer {
    pub accumulator: f32,
    /// Target seconds per fixed tick (default: 1/20 = 0.05s).
    pub fixed_timestep: f32,
    pub last_time: Option<std::time::Instant>,
}
