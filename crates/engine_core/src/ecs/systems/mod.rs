use std::sync::OnceLock;

use crate::ecs::World;
use anyhow::Result;
use macros::Resource;

pub mod param;
pub mod scheduler;

pub trait HasPriority {
    fn priority(&self) -> u32;
}

pub trait ScheduledSystem: Copy + 'static {
    fn name(&self) -> &'static str;
    fn func(&self) -> fn(&mut World) -> Result<()>;
    fn priority(&self) -> u32;
}

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

impl ScheduledSystem for UpdateSystem {
    fn name(&self) -> &'static str {
        self.name
    }
    fn func(&self) -> fn(&mut World) -> Result<()> {
        self.func
    }
    fn priority(&self) -> u32 {
        self.priority
    }
}

static UPDATE_SYSTEMS_CACHE: OnceLock<Vec<UpdateSystem>> = OnceLock::new();

impl UpdateSystem {
    pub fn sorted() -> &'static [UpdateSystem] {
        UPDATE_SYSTEMS_CACHE.get_or_init(|| {
            let mut systems: Vec<UpdateSystem> =
                inventory::iter::<UpdateSystem>().copied().collect();
            systems.sort_by_key(|s| s.priority);
            systems
        })
    }
}

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

impl ScheduledSystem for LateUpdateSystem {
    fn name(&self) -> &'static str {
        self.name
    }
    fn func(&self) -> fn(&mut World) -> Result<()> {
        self.func
    }
    fn priority(&self) -> u32 {
        self.priority
    }
}

static LATE_UPDATE_SYSTEMS_CACHE: OnceLock<Vec<LateUpdateSystem>> = OnceLock::new();

impl LateUpdateSystem {
    pub fn sorted() -> &'static [LateUpdateSystem] {
        LATE_UPDATE_SYSTEMS_CACHE.get_or_init(|| {
            let mut systems: Vec<LateUpdateSystem> =
                inventory::iter::<LateUpdateSystem>().copied().collect();
            systems.sort_by_key(|s| s.priority);
            systems
        })
    }
}

/// A system that runs at a fixed timestep (60 by default).
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

static FIXED_UPDATE_SYSTEMS_CACHE: OnceLock<Vec<FixedUpdateSystem>> = OnceLock::new();

impl FixedUpdateSystem {
    pub fn name(&self) -> &'static str {
        self.name
    }
    pub fn priority(&self) -> u32 {
        self.priority
    }

    pub fn sorted() -> &'static [FixedUpdateSystem] {
        FIXED_UPDATE_SYSTEMS_CACHE.get_or_init(|| {
            let mut systems: Vec<FixedUpdateSystem> =
                inventory::iter::<FixedUpdateSystem>().copied().collect();
            systems.sort_by_key(|s| s.priority);
            systems
        })
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct DeltaTime(pub f32);

#[derive(Resource, Debug, Clone, Default)]
pub struct EngineTimer(pub f32);

#[derive(Resource, Debug, Clone, Default)]
pub struct FixedUpdateTimer {
    pub accumulator: f32,
    /// Target seconds per fixed tick (default: 1/60 = 0.0166...s).
    pub fixed_timestep: f32,
    pub last_time: Option<std::time::Instant>,
}

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

impl ScheduledSystem for StartSystem {
    fn name(&self) -> &'static str {
        self.name
    }
    fn func(&self) -> fn(&mut World) -> Result<()> {
        self.func
    }
    fn priority(&self) -> u32 {
        self.priority
    }
}

static START_SYSTEMS_CACHE: OnceLock<Vec<StartSystem>> = OnceLock::new();

impl StartSystem {
    pub fn sorted() -> &'static [StartSystem] {
        START_SYSTEMS_CACHE.get_or_init(|| {
            let mut systems: Vec<StartSystem> = inventory::iter::<StartSystem>().copied().collect();
            systems.sort_by_key(|s| s.priority);
            systems
        })
    }
}

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

pub fn run_system<S: ScheduledSystem>(world: &mut World, systems: &[S]) -> Result<()> {
    for system in systems {
        if let Err(_err) = (system.func())(world) {
            // crate::log_error!(reason: "system returned an error", "'{}': {err:?}", system.name());
        }
    }
    Ok(())
}

pub fn run_fixed_update(
    world: &mut World,
    systems: &[FixedUpdateSystem],
    delta: f32,
) -> Result<()> {
    for system in systems {
        if let Err(_err) = (system.func)(world, delta) {
            // crate::log_error!(reason: "fixed system returned an error", "'{}': {err:?}", system.name());
        }
    }
    Ok(())
}
