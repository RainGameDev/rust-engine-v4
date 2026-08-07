use anyhow::Result;
use macros::component;
use nalgebra::{Matrix4, UnitQuaternion, Vector3};

use crate::ecs::{
    World,
    components::engine_components::hierarchy::{Children, Parent},
    entities::Entity,
    query::{filter::Without, query::Query},
};

#[component(networked)]
pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub scale: Vector3<f32>,

    pub global_position: Vector3<f32>,
    pub global_rotation: UnitQuaternion<f32>,
    pub global_scale: Vector3<f32>,
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            position: Vector3::zeros(),
            rotation: UnitQuaternion::identity(),
            scale: Vector3::new(1.0, 1.0, 1.0),

            global_position: Vector3::zeros(),
            global_rotation: UnitQuaternion::identity(),
            global_scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn from_position(position: Vector3<f32>) -> Self {
        Self {
            position,
            ..Self::identity()
        }
    }

    pub fn from_rotation(rotation: UnitQuaternion<f32>) -> Self {
        Self {
            rotation,
            ..Self::identity()
        }
    }

    pub fn from_scale(scale: Vector3<f32>) -> Self {
        Self {
            scale,
            ..Self::identity()
        }
    }

    pub fn looking_at(mut self, target: Vector3<f32>, up: Vector3<f32>) -> Self {
        let dir = target - self.position;
        self.rotation = UnitQuaternion::face_towards(&dir, &up);
        self
    }

    pub fn with_position(mut self, position: Vector3<f32>) -> Self {
        self.position = position;
        self
    }

    pub fn with_rotation(mut self, rotation: UnitQuaternion<f32>) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_scale(mut self, scale: Vector3<f32>) -> Self {
        self.scale = scale;
        self
    }

    pub fn forward(&self) -> Vector3<f32> {
        self.rotation * -Vector3::z()
    }

    pub fn right(&self) -> Vector3<f32> {
        self.rotation * Vector3::x()
    }

    pub fn up(&self) -> Vector3<f32> {
        self.rotation * Vector3::y()
    }

    pub fn translate(&mut self, delta: Vector3<f32>) {
        self.position += delta;
    }

    pub fn rotate(&mut self, delta: UnitQuaternion<f32>) {
        self.rotation = delta * self.rotation;
    }

    pub fn to_matrix(&self) -> Matrix4<f32> {
        Matrix4::new_translation(&self.position)
            * self.rotation.to_homogeneous()
            * Matrix4::new_nonuniform_scaling(&self.scale)
    }

    /// Combines `self` as a parent transform with `child`, producing child's
    /// effective transform in the same space `self` is defined in.
    pub fn mul_transform(&self, child: &Transform) -> Transform {
        let position = self.position + self.rotation * self.scale.component_mul(&child.position);
        let rotation = self.rotation * child.rotation;
        let scale = self.scale.component_mul(&child.scale);

        let global_position = self.position
            + self.global_rotation * self.global_scale.component_mul(&child.global_position);
        let global_rotation = self.global_rotation * child.global_rotation;
        let global_scale = self.global_scale.component_mul(&child.global_scale);
        Transform {
            position,
            rotation,
            scale,
            global_position,
            global_rotation,
            global_scale,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

pub fn transform_update(world: &mut World) -> Result<()> {
    let roots: Vec<Entity> = {
        let query: Query<Entity, Without<Parent>> = Query::new(world);
        query.iter().collect()
    };

    for root in roots {
        propagate_root(world, root);
        propagate_children(world, root);
    }

    Ok(())
}

/// Roots have no parent, so their global transform is just their local transform.
fn propagate_root(world: &mut World, entity: Entity) {
    let Some(transform) = world.get_component_mut::<Transform>(entity) else {
        return;
    };
    transform.global_position = transform.position;
    transform.global_rotation = transform.rotation;
    transform.global_scale = transform.scale;
}

fn propagate_children(world: &mut World, parent: Entity) {
    let Some((parent_position, parent_rotation, parent_scale)) = world
        .get_component::<Transform>(parent)
        .map(|t| (t.global_position, t.global_rotation, t.global_scale))
    else {
        return;
    };

    let children = world.get_component::<Children>(parent).map(|c| c.0.clone());

    let Some(children) = children else { return };

    for child in children {
        let Some(local) = world.get_component::<Transform>(child).cloned() else {
            continue;
        };

        let global_position =
            parent_position + parent_rotation * parent_scale.component_mul(&local.position);
        let global_rotation = parent_rotation * local.rotation;
        let global_scale = parent_scale.component_mul(&local.scale);

        if let Some(transform) = world.get_component_mut::<Transform>(child) {
            transform.global_position = global_position;
            transform.global_rotation = global_rotation;
            transform.global_scale = global_scale;
        }

        propagate_children(world, child);
    }
}
