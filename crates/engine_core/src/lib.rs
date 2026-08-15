pub mod assets;
pub mod ecs;
pub mod ffi;
pub mod input;
pub mod logging;
pub mod networking;
pub mod physics;
pub mod rendering;
pub mod tiles;
pub mod time;
pub mod utils;
pub mod window;

#[cfg(test)]
mod tests;

pub use ash;
pub use egui;
pub use inventory;
pub use macros::{Component, Resource, component};
pub use macros::{fixed_update, late_update, start, update};
pub use nalgebra;

use crate::{
    assets::AssetRegistration,
    ecs::{
        World,
        components::engine_components::{camera::Camera, transform::Transform},
        query::query::Query,
    },
    rendering::core::frame_info::FrameInfo,
};
use anyhow::Result;

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
