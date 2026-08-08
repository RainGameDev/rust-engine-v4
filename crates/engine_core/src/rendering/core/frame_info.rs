use nalgebra::Matrix4;

use crate::{
    ecs::{
        World,
        components::engine_components::camera::{Camera, GameCamera},
        query::{filter::With, query::Query},
    },
    rendering::core::model::GpuMesh,
};

/// All info needed for a frame to render.
#[derive(Clone, Debug)]
pub struct FrameInfo {
    pub view_projection: Matrix4<f32>,
    pub draws: Vec<DrawInfo>,
}

/// One mesh to draw this frame, plus optional skinning data.
#[derive(Clone, Debug)]
pub struct DrawInfo {
    pub mesh: GpuMesh,
    /// model matrix for this entity.
    pub model: Matrix4<f32>,
    /// Per-entity joint matrices (in the `Skeleton`'s joint order). `None`
    /// for static meshes, which bind an identity joint buffer.
    pub joint_matrices: Option<Vec<Matrix4<f32>>>,
}

#[repr(C)]
pub struct PushConstants {
    pub mvp: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
}

pub fn matrix_to_push_constant(m: &Matrix4<f32>) -> [[f32; 4]; 4] {
    let data: [f32; 16] = m.as_slice().try_into().unwrap();
    [
        [data[0], data[1], data[2], data[3]],
        [data[4], data[5], data[6], data[7]],
        [data[8], data[9], data[10], data[11]],
        [data[12], data[13], data[14], data[15]],
    ]
}

pub fn update_camera_aspect_ratio(world: &mut World, width: u32, height: u32) {
    let aspect = width as f32 / height as f32;

    let query: Query<&mut Camera, With<GameCamera>> = Query::new(world);
    for camera in query.iter() {
        camera.projection.set_aspect_ratio(aspect);
    }
}
