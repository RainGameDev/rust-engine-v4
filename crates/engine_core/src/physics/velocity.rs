use macros::component;
use nalgebra::Vector3;

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
