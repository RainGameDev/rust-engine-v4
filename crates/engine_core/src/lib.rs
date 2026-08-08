pub mod assets;
pub mod ecs;
pub mod ffi;
pub mod input;
pub mod logging;
pub mod networking;
pub mod rendering;
pub mod time;
pub mod utils;
pub mod window;

#[cfg(test)]
mod tests;

pub use inventory;
pub use macros::{Component, Resource};
pub use macros::{fixed_update, late_update, start, update};

use anyhow::Result;
use nalgebra::Vector3;

use crate::ecs::commands::Commands;
use crate::ecs::systems::param::ResMut;
use crate::rendering::egui::context::EguiContext;
use crate::{
    assets::AssetRegistration,
    ecs::{
        World,
        components::engine_components::{
            camera::{Camera, GameCamera},
            transform::Transform,
        },
        query::query::Query,
    },
    rendering::core::frame_info::FrameInfo,
};

/// Engine handler
pub struct Engine {
    pub ecs_world: World,
}

impl Engine {
    pub fn new() -> Self {
        let mut ecs_world = World::default();
        for registration in inventory::iter::<AssetRegistration> {
            let type_id = (registration.type_id)();
            let asset_map = (registration.create_asset_map)();
            ecs_world.assets.insert(type_id, asset_map);
        }
        ecs_world.add_resource(input::InputManager::new());
        Self { ecs_world }
    }

    pub fn return_renderable(&self) -> Option<FrameInfo> {
        let camera_query: Query<(&Camera, &Transform)> = Query::new(&self.ecs_world);
        let (camera, camera_global) = camera_query.iter().find(|(c, _)| c.is_active)?;
        let view_projection = camera.view_projection_matrix(camera_global);

        Some(FrameInfo {
            view_projection,
            draws: Vec::new(),
        })
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

pub fn init_core(auto_connect_addr: Option<std::net::SocketAddr>) -> Result<()> {
    let engine = Engine::new();
    window::run(engine, auto_connect_addr)
}

#[update]
pub fn test(context: ResMut<EguiContext>) -> Result<()> {
    egui::Window::new("").show(&context.0, |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
            });
        });
    });
    Ok(())
}

#[start]
pub fn start(commands: &mut Commands) -> Result<()> {
    let camera = commands.spawn();
    commands.add_component(
        camera,
        Transform::from_position(Vector3::new(0.0, 0.0, 100.0)),
    );
    commands.add_component(camera, Camera::perspective(60.0, 16.0 / 9.0, 0.1, 1000.0));
    commands.add_component(camera, GameCamera);

    Ok(())
}
