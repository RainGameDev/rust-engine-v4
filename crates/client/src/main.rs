use anyhow::Result;
use engine_core::{
    ecs::{
        commands::Commands,
        components::engine_components::{
            camera::{Camera, GameCamera},
            transform::Transform,
        },
    },
    init_core, start,
};
use game::{GameState, components::TempCamera};

use crate::ui::settings::SettingsState;

pub mod ui;

fn main() -> anyhow::Result<()> {
    init_core(None)
}

#[start]
pub fn init(commands: &mut Commands) -> Result<()> {
    // Init the game, runs funcitons ect ect ect
    commands.add_resource(game::init());
    commands.add_resource(GameState::MainMenu);
    commands.add_resource(SettingsState::default());

    // make the main menu camera
    let menu_camera = commands.spawn();
    commands.add_component(menu_camera, Transform::default());
    commands.add_component(menu_camera, TempCamera);
    commands.add_component(menu_camera, GameCamera);
    commands.add_component(menu_camera, Camera::perspective(90.0, 1.0, 0.001, 1000.0));

    Ok(())
}
