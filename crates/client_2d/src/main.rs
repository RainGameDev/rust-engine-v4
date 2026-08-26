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
    init_core, log_error,
    rendering::egui::{context::EguiContext, fonts::FontRegistry},
    start, update,
};
use game_2d::{GameState, components::TempCamera};

use crate::bindings::registered_inputs;
use crate::ui::settings::{SettingsState, bindings_path};

pub mod bindings;
pub mod ui;

fn main() -> anyhow::Result<()> {
    init_core(None)
}

#[start]
pub fn register_default_bindings(
    mut input: ResMut<engine_core::input::InputManager>,
) -> Result<()> {
    input.register_inputs(registered_inputs());

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
    commands.add_resource(game_2d::init());
    commands.add_resource(GameState::MainMenu);
    commands.add_resource(SettingsState::default());

    let mut fonts = FontRegistry::default();
    fonts.load_dir(std::path::Path::new(&format!(
        "{}/{}",
        env!("CARGO_MANIFEST_DIR"),
        "res/fonts/"
    )));
    commands.add_resource(fonts);

    // make the main menu camera
    let menu_camera = commands.spawn();
    commands.add_component(menu_camera, Transform::default());
    commands.add_component(menu_camera, TempCamera);
    commands.add_component(menu_camera, GameCamera);
    commands.add_component(menu_camera, Camera::perspective(90.0, 1.0, 0.001, 1000.0));

    Ok(())
}

#[update]
pub fn apply_fonts(mut fonts: ResMut<FontRegistry>, context: ResMut<EguiContext>) -> Result<()> {
    fonts.apply_if_needed(&context.0);
    Ok(())
}
