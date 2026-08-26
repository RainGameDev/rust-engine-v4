use std::path::Path;

use anyhow::Result;
use engine_core::{
    ecs::{
        commands::Commands,
        components::engine_components::{
            camera::{Camera, EditorCamera},
            transform::Transform,
        },
    },
    init_core,
    nalgebra::Vector3,
    start,
};
use game_data::registry::GameRegistry;

pub mod ui;

fn main() -> Result<()> {
    init_core(None)
}

#[start]
pub fn init(commands: &mut Commands) -> Result<()> {
    // The game's shared content registry, layered with the editor's own defs.
    let editor_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let game_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("game_data")
        .join("data");

    let mut registry = GameRegistry::load_from_dir(&editor_dir)?;
    if let Ok(mut game_registry) = GameRegistry::load_from_dir(&game_dir) {
        game_registry.entities.extend(registry.entities);
        registry = game_registry;
        commands.add_resource(registry);
    }

    // Editor flycam.
    let camera = commands.spawn();
    commands.add_component(
        camera,
        Transform::from_position(Vector3::new(6.0, 8.0, 12.0)),
    );
    commands.add_component(camera, EditorCamera);
    commands.add_component(camera, Camera::perspective(60.0, 1.0, 0.01, 1000.0));

    Ok(())
}
