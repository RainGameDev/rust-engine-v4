use core::f32;

use anyhow::Result;
use macros::{component, fixed_update};

use crate::{
    ecs::query::{filter::Without, query::Query},
    physics::{collider::IgnoreCollisions, velocity::Velocity},
};

#[component]
pub struct Gravity {
    pub force: f32,
    pub weight: f32,
}

#[fixed_update]
pub fn gravity_update(
    delta: f32,
    query: Query<(&Gravity, &mut Velocity), Without<IgnoreCollisions>>,
) -> Result<()> {
    for (gravity, velocity) in query.iter() {
        velocity.linear_velocity.y += gravity.force * gravity.weight * delta;
        velocity.linear_velocity.y = velocity
            .linear_velocity
            .y
            .clamp(-32.0 * gravity.weight, f32::INFINITY);
    }

    Ok(())
}
