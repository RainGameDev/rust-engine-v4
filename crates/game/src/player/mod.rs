use anyhow::Result;
use engine_core::{
    component,
    ecs::{
        components::engine_components::transform::Transform, entities::Entity,
        query::single::Single, systems::param::Res,
    },
    fixed_update,
    input::InputManager,
    physics::velocity::Velocity,
};

use crate::GameState;

#[component]
pub struct Player;

#[fixed_update]
pub fn movement(
    _delta: f32,
    game_state: Res<GameState>,
    input: Res<InputManager>,
    mut player: Single<(Entity, &mut Transform, &Player, &mut Velocity)>,
) -> Result<()> {
    if let GameState::MainMenu = *game_state {
        return Ok(());
    }

    let speed = 5.0;

    if input.pressed("MoveLeft") {
        player.3.linear_velocity.x = -speed;
    } else if input.pressed("MoveRight") {
        player.3.linear_velocity.x = speed;
    } else {
        player.3.linear_velocity.x = 0.0;
    }

    Ok(())
}
