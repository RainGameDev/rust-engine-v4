use macros::Component;
use nalgebra::{Matrix4, UnitQuaternion, Vector3};

#[derive(Component, Clone, Debug)]
pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub scale: Vector3<f32>,
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            position: Vector3::zeros(),
            rotation: UnitQuaternion::identity(),
            scale: Vector3::new(1.0, 1.0, 1.0),
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
        Transform {
            position,
            rotation,
            scale,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

#[derive(Component, Clone, Debug)]
pub struct GlobalTransform {
    pub matrix: Matrix4<f32>,
    pub translation: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
}

impl GlobalTransform {
    pub fn from_matrix(matrix: Matrix4<f32>) -> Self {
        let translation = matrix.column(3).xyz();
        let rotation = UnitQuaternion::from_matrix(&matrix.fixed_view::<3, 3>(0, 0).into());
        Self {
            matrix,
            translation,
            rotation,
        }
    }
}
