use anyhow::Result;
use engine_core::ecs::{
    commands::Commands,
    components::engine_components::{
        camera::{Camera, GameCamera},
        transform::Transform,
    },
    systems::param::{Res, ResMut},
};
use engine_core::log_info;
use engine_core::networking::client::IncomingRegistry;
use engine_core::{Resource, start, update};
use game_data::registry::GameRegistry;

#[derive(Debug, Resource)]
pub struct ClientRegistry(pub GameRegistry);

#[derive(Debug, Resource)]
pub struct AppliedRegistryVersion(pub u32);

#[start]
pub fn init_registry(commands: &mut Commands) -> Result<()> {
    commands.add_resource(AppliedRegistryVersion(0));
    let camera = commands.spawn();
    commands.add_component(camera, Camera::perspective(70.0, 1.0, 0.01, 1000.0));
    commands.add_component(camera, GameCamera);
    commands.add_component(camera, Transform::default());
    Ok(())
}

#[update]
pub fn receive_registry(
    incoming: Res<IncomingRegistry>,
    mut applied: ResMut<AppliedRegistryVersion>,
    commands: &mut Commands,
) -> Result<()> {
    if incoming.version <= applied.0 {
        return Ok(());
    }
    let registry: GameRegistry = bincode::deserialize(&incoming.bytes)?;
    for item in registry.items.values() {
        log_info!("Loaded item asset, name: {}", item.name);
    }
    commands.add_resource(ClientRegistry(registry));
    applied.0 = incoming.version;
    Ok(())
}
