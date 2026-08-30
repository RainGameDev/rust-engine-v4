use anyhow::Result;
use macros::{component, fixed_update};
use nalgebra::{UnitQuaternion, Vector3};

use crate::ecs::{components::engine_components::transform::Transform, query::query::Query};

#[component]
pub struct Velocity {
    pub angular_velocity: Vector3<f32>,
    pub linear_velocity: Vector3<f32>,
    pub mass: f32,
    pub is_grounded: bool,
    pub process: bool,

    pub inertia_tensor: Vector3<f32>,
    pub mu_static: f32,
    pub mu_kinetic: f32,
    pub restitution: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
}

impl Velocity {
    pub fn zero() -> Self {
        Self {
            linear_velocity: Vector3::zeros(),
            angular_velocity: Vector3::zeros(),
            mass: 1.0,
            is_grounded: false,
            process: true,
            inertia_tensor: Vector3::new(1.0, 1.0, 1.0),
            mu_static: 0.5,
            mu_kinetic: 0.3,
            restitution: 0.0,
            linear_damping: 0.0,
            angular_damping: 0.05,
        }
    }
    pub fn zero_2d() -> Self {
        Self {
            linear_velocity: Vector3::zeros(),
            angular_velocity: Vector3::zeros(),
            mass: 1.0,
            is_grounded: false,
            process: true,
            inertia_tensor: Vector3::new(0.0, 0.0, 1.0),
            mu_static: 0.5,
            mu_kinetic: 0.3,
            restitution: 0.0,
            linear_damping: 0.0,
            angular_damping: 0.05,
        }
    }

    pub fn lock_to_2d(&mut self) {
        self.linear_velocity.z = 0.0;
        self.angular_velocity.x = 0.0;
        self.angular_velocity.y = 0.0;
    }
}

#[fixed_update]
pub fn physics_integration(
    delta: f32,
    entities: Query<(&mut Transform, &mut Velocity)>,
) -> Result<()> {
    for (tf, vel) in entities.iter() {
        if !vel.process {
            continue;
        }

        // Integrate linear velocity
        tf.position += vel.linear_velocity * delta;
        tf.global_position += vel.linear_velocity * delta;

        // Integrate angular velocity (axis-angle to quaternion)
        let angle = vel.angular_velocity.norm() * delta;
        if angle > 1e-8 {
            let axis = vel.angular_velocity.normalize();
            let dq = UnitQuaternion::from_axis_angle(&nalgebra::Unit::new_normalize(axis), angle);
            tf.rotation = dq * tf.rotation;
            tf.global_rotation = dq * tf.global_rotation;
        }

        // Apply damping
        let linear_damping = vel.linear_damping;
        let angular_damping = vel.angular_damping;
        vel.linear_velocity *= 1.0 - linear_damping * delta;
        vel.angular_velocity *= 1.0 - angular_damping * delta;
    }

    Ok(())
}
