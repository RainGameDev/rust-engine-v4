use anyhow::Result;
use engine_core::{
    Resource,
    assets::register_extra_asset_dir,
    ecs::{
        commands::Commands,
        components::engine_components::{
            camera::{Camera, GameCamera},
            sprite::Sprite,
            transform::Transform,
        },
        entities::Entity,
        query::query::Query,
        systems::param::Res,
    },
    log_debug,
    nalgebra::Vector3,
    physics::{collider::Collider, velocity::Velocity},
    update,
};
use game_data::registry::GameRegistry;

use crate::{components::TempCamera, player::Player};

pub mod components;
pub mod player;

#[derive(Debug, Resource)]
pub enum GameState {
    MainMenu,
    Playing,
}
impl GameState {
    pub fn is_playing(&self) -> bool {
        matches!(self, GameState::Playing)
    }
}

#[derive(Debug, Resource)]
pub struct GameContext {
    /// The data registry for the game (items, quests, npcs ect ect ect)
    pub registry: GameRegistry,
}

#[update]
pub fn update() -> Result<()> {
    Ok(())
}

pub fn init() -> Result<GameContext> {
    log_debug!("Initing game");

    register_extra_asset_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/res"));

    let registry = GameRegistry::load_from_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data"))?;

    Ok(GameContext { registry })
}

#[update]
pub fn start_game(
    commands: &mut Commands,
    game_state: Res<GameState>,
    temp_cameras: Query<(Entity, &TempCamera)>,
) -> Result<()> {
    if let GameState::MainMenu = *game_state {
        return Ok(());
    }

    let has_temp = temp_cameras.iter().next().is_some();
    if !has_temp {
        return Ok(());
    }

    log_debug!("hi");

    for (entity, _cam) in temp_cameras.iter() {
        commands.despawn(entity);
    }

    let cam = commands.spawn();
    commands.add_component(
        cam,
        Transform {
            position: Vector3::new(0.0, 0.0, 1.0),

            ..Default::default()
        },
    );
    commands.add_component(cam, GameCamera);
    commands.add_component(cam, Camera::orthographic(5.0, 16.0 / 9.0, -1000.0, 1000.0));

    // Player sprite
    let player = commands.spawn();
    commands.add_component(player, Transform::default());
    commands.add_component(player, Velocity::zero_2d());
    commands.add_component(player, Player);
    commands.add_component(player, Collider::rect_2d(0.5, 1.0));
    commands.add_component(
        player,
        Sprite {
            path: "images/ui/player.png".into(),
        },
    );

    let wall = commands.spawn();
    let mut collider = Collider::rect_2d(0.5, 1.0);
    collider.is_static = true;
    commands.add_component(wall, collider);
    let mut wall_vel = Velocity::zero_2d();
    wall_vel.mass = 0.0;
    commands.add_component(wall, wall_vel);
    commands.add_component(
        wall,
        Transform {
            position: Vector3::new(1.0, 0.0, 0.0),
            ..Default::default()
        },
    );
    commands.add_component(
        wall,
        Sprite {
            path: "images/ui/player.png".into(),
        },
    );

    Ok(())
}
