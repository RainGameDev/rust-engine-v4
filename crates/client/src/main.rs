use anyhow::Result;
use engine_core::{
    ecs::{
        commands::Commands,
        components::engine_components::{
            camera::{Camera, GameCamera},
            transform::Transform,
        },
        systems::param::ResMut,
    },
    init_core, log_error, start,
};
use game::{GameState, components::TempCamera};

use crate::ui::settings::{SettingsState, bindings_path, default_bindings};

pub mod ui;

fn main() -> anyhow::Result<()> {
    init_core(None)
}

#[start]
pub fn register_default_bindings(mut input: ResMut<engine_core::input::InputManager>) -> Result<()> {
    for (name, binding) in default_bindings() {
        input.bind_action(name, binding);
    }

    let path = bindings_path();
    if path.exists() {
        match std::fs::read(&path) {
            Ok(bytes) => {
                if let Err(err) = input.load_bindings(&bytes) {
                    log_error!(reason: "failed to load bindings", "{err}");
                }
            }
            Err(err) => {
                log_error!(reason: "failed to read bindings file", "{err}");
            }
        }
    }

    Ok(())
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
