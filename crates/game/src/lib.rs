use anyhow::Result;
use engine_core::{
    Resource,
    ecs::{
        commands::Commands,
        components::engine_components::{
            camera::{Camera, GameCamera},
            transform::Transform,
        },
        entities::Entity,
        query::query::Query,
        systems::param::Res,
    },
    log_debug, update,
};
use game_data::registry::GameRegistry;

use crate::components::TempCamera;

pub mod components;

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

    let registry = GameRegistry::load_from_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/data"))?;

    Ok(GameContext { registry })
}

#[update]
pub fn start_game(
    commands: &mut Commands,
    game_state: Res<GameState>,
    temp_cameras: Query<(Entity, &TempCamera)>,
) -> Result<()> {
    if let GameState::MainMenu = *game_state { return Ok(()) }

    for (entity, _cam) in temp_cameras.iter() {
        commands.despawn(entity);
    }

    let menu_camera = commands.spawn();
    commands.add_component(menu_camera, Transform::default());
    commands.add_component(menu_camera, GameCamera);
    commands.add_component(menu_camera, Camera::perspective(90.0, 1.0, 0.001, 1000.0));

    Ok(())
}
