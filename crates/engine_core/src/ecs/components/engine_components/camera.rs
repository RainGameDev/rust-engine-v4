use macros::{Component, component};
use nalgebra::{Matrix4, Orthographic3, Perspective3, Point3, Vector3};

use crate::ecs::components::engine_components::transform::Transform;

#[derive(Clone, Debug, PartialEq)]
pub enum Projection {
    Perspective {
        /// Vertical field of view.
        fov_y: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32,
    },
    Orthographic {
        half_height: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32,
    },
}

impl Projection {
    pub fn perspective(fov_y_degrees: f32, aspect_ratio: f32, near: f32, far: f32) -> Self {
        debug_assert!(
            fov_y_degrees > 0.0 && fov_y_degrees < 180.0,
            "fov_y_degrees must be in (0, 180), got {fov_y_degrees}"
        );
        Self::Perspective {
            fov_y: fov_y_degrees.to_radians(),
            aspect_ratio,
            near,
            far,
        }
    }

    pub fn orthographic(half_height: f32, aspect_ratio: f32, near: f32, far: f32) -> Self {
        Self::Orthographic {
            half_height,
            aspect_ratio,
            near,
            far,
        }
    }

    pub fn set_aspect_ratio(&mut self, new_aspect: f32) {
        match self {
            Self::Perspective { aspect_ratio, .. } => *aspect_ratio = new_aspect,
            Self::Orthographic { aspect_ratio, .. } => *aspect_ratio = new_aspect,
        }
    }

    pub fn matrix(&self) -> Matrix4<f32> {
        match *self {
            Self::Perspective {
                fov_y,
                aspect_ratio,
                near,
                far,
            } => Perspective3::new(aspect_ratio, fov_y, near, far).to_homogeneous(),
            Self::Orthographic {
                half_height,
                aspect_ratio,
                near,
                far,
            } => {
                let half_width = half_height * aspect_ratio;
                Orthographic3::new(
                    -half_width,
                    half_width,
                    -half_height,
                    half_height,
                    near,
                    far,
                )
                .to_homogeneous()
            }
        }
    }
}

#[component]
pub struct Camera {
    pub projection: Projection,
    /// Whether this camera is currently used to render.
    /// Lets multiple `Camera` entities exist with only one active at a time.
    pub is_active: bool,
}

impl Camera {
    /// Creates a new perspective Camera
    pub fn perspective(fov_y_degrees: f32, aspect_ratio: f32, near: f32, far: f32) -> Self {
        Self {
            projection: Projection::perspective(fov_y_degrees, aspect_ratio, near, far),
            is_active: true,
        }
    }

    /// Creates a new orthographic camera
    pub fn orthographic(half_height: f32, aspect_ratio: f32, near: f32, far: f32) -> Self {
        Self {
            projection: Projection::orthographic(half_height, aspect_ratio, near, far),
            is_active: true,
        }
    }

    /// Gets the cameras projection matrix
    pub fn projection_matrix(&self) -> Matrix4<f32> {
        self.projection.matrix()
    }

    /// Builds the view matrix from this camera entity's world position/rotation.
    pub fn view_matrix(&self, global: &Transform) -> Matrix4<f32> {
        let eye = Point3::from(global.global_position);
        let forward = global.rotation * -Vector3::z();
        let up = global.rotation * Vector3::y();
        Matrix4::look_at_rh(&eye, &(eye + forward), &up)
    }

    pub fn view_projection_matrix(&self, global: &Transform) -> Matrix4<f32> {
        self.projection_matrix() * self.view_matrix(global)
    }
}

#[component]
pub struct GameCamera;

#[component]
pub struct EditorCamera;
