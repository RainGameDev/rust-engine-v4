use anyhow::Result;
use std::time::{Duration, Instant};

use crate::ecs::World;
use crate::ecs::components::engine_components::transform::transform_update;
use crate::ecs::systems::{
    DeltaTime, FixedUpdateSystem, LateUpdateSystem, UpdateSystem, run_fixed_update, run_system,
};

pub struct Schedule {
    fixed_timestep: Duration,
    accumulator: Duration,
    last_frame: Instant,
}

impl Schedule {
    pub fn new(fixed_hz: f64) -> Self {
        Self {
            fixed_timestep: Duration::from_secs_f64(1.0 / fixed_hz),
            accumulator: Duration::ZERO,
            last_frame: Instant::now(),
        }
    }

    /// Call once per rendered frame.
    pub fn tick(&mut self, world: &mut World) -> Result<()> {
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame);
        self.last_frame = now;
        self.accumulator += delta;

        let fixed_delta = self.fixed_timestep.as_secs_f32();

        let mut steps = 0;
        while self.accumulator >= self.fixed_timestep {
            run_fixed_update(world, FixedUpdateSystem::sorted(), fixed_delta)?;
            self.accumulator -= self.fixed_timestep;
            steps += 1;
            if steps >= 8 {
                self.accumulator = Duration::ZERO;
                break;
            }
        }

        // Publish the render-frame delta so `#[update]` systems can read it.
        if !world.has_resource::<DeltaTime>() {
            world.add_resource(DeltaTime::default());
        }
        if let Ok(mut delta_time) = world.get_resource_mut::<DeltaTime>() {
            delta_time.0 = delta.as_secs_f32();
        }

        // Refresh global transforms before update systems run so collider/raycast
        // code sees current world-space positions (snapshots may carry stale globals).
        transform_update(world)?;

        run_system(world, UpdateSystem::sorted())?;
        transform_update(world)?;
        run_system(world, LateUpdateSystem::sorted())?;

        transform_update(world)?;

        Ok(())
    }
}
