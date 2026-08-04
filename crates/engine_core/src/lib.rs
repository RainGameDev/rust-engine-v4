pub mod assets;
pub mod ecs;
pub mod ffi;
pub mod input;
pub mod logging;
pub mod rendering;
pub mod time;
pub mod window;

#[cfg(test)]
mod tests;

pub use inventory;
pub use macros::{Component, Resource};
pub use macros::{fixed_update, late_update, update};

use anyhow::Result;
use nalgebra::Vector3;

use crate::ecs::query::filter::With;
use crate::ecs::query::single::Single;
use crate::ecs::systems::param::{Res, ResMut};
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
        let camera = ecs_world.spawn();
        ecs_world.insert_component(
            camera,
            Box::new(Transform::from_position(Vector3::new(0.0, 0.0, 100.0))),
        );
        ecs_world.insert_component(
            camera,
            Box::new(Camera::perspective(60.0, 16.0 / 9.0, 0.1, 1000.0)),
        );
        ecs_world.insert_component(camera, Box::new(GameCamera));

        ecs_world.add_resource(FrameCounter { count: 0 });

        for registration in inventory::iter::<AssetRegistration> {
            let type_id = (registration.type_id)();
            let asset_map = (registration.create_asset_map)();
            ecs_world.assets.insert(type_id, asset_map);
        }

        Self { ecs_world }
    }

    pub fn return_renderable(&self) -> Option<FrameInfo> {
        let camera_query: Query<(&Camera, &Transform)> = Query::new(&self.ecs_world);
        let (camera, camera_global) = camera_query.iter().find(|(c, _)| c.is_active)?;
        let view_projection = camera.view_projection_matrix(camera_global);

        Some(FrameInfo {
            view_projection,
            meshes: Vec::new(),
        })
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

pub fn init_core() -> Result<()> {
    let engine = Engine::new();
    window::run(engine)
}

#[derive(Resource, Clone, Debug)]
pub struct FrameCounter {
    pub count: u64,
}

#[update]
fn debug_print_scene_info(
    transforms: Query<&Transform>,
    mut frame_counter: ResMut<FrameCounter>,
    camera: Single<(&Camera, &Transform), With<GameCamera>>,
) -> Result<()> {
    println!("=== frame {} ===", frame_counter.count);

    for transform in transforms.iter() {
        println!(
            "transform — pos: {:?}, rot: {:?}, scale: {:?}",
            transform.position, transform.rotation, transform.scale
        );
    }

    let (cam, cam_global) = &*camera;
    println!(
        "camera — active: {}, world pos: {:?}",
        cam.is_active, cam_global.position
    );

    frame_counter.count += 1;

    Ok(())
}
