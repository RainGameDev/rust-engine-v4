use std::sync::Arc;

use macros::component;
use nalgebra::{UnitQuaternion, Vector3};

use crate::physics::{
    bvh::Bvh,
    collision::{
        aabb_overlaps_aabb, capsule_vs_mesh, sphere_vs_mesh, sphere_vs_obb, sphere_vs_sphere,
    },
    math::{rotate_vector, triangle_normal},
};

#[component]
pub struct IgnoreCollisions;

/// The shape used for collision detection.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ColliderShape {
    Cuboid {
        size: Vector3<f32>,
    },
    Sphere {
        radius: f32,
    },
    Capsule {
        radius: f32,
        height: f32,
    },
    Cylinder {
        radius: f32,
        height: f32,
    },
    Mesh {
        triangles: Arc<Vec<[Vector3<f32>; 3]>>,
        bvh: Arc<Bvh>,
        model_path: String,
    },
}

impl ColliderShape {
    pub fn circle(radius: f32) -> Self {
        Self::Sphere { radius }
    }

    pub fn rect(half_width: f32, half_height: f32) -> Self {
        Self::Cuboid {
            size: Vector3::new(half_width, half_height, 0.1),
        }
    }

    pub fn aabb(width: f32, height: f32) -> Self {
        Self::Cuboid {
            size: Vector3::new(width * 0.5, height * 0.5, 0.0),
        }
    }
    pub fn half_extents(&self) -> Vector3<f32> {
        match self {
            ColliderShape::Cuboid { size } => *size,
            ColliderShape::Sphere { radius } => Vector3::new(*radius, *radius, *radius),
            ColliderShape::Capsule { radius, height } => {
                Vector3::new(*radius, height * 0.5 + radius, *radius)
            }
            ColliderShape::Cylinder { radius, height } => {
                Vector3::new(*radius, height * 0.5, *radius)
            }
            ColliderShape::Mesh { bvh, .. } => {
                if bvh.nodes.is_empty() {
                    Vector3::new(0.0, 0.0, 0.0)
                } else {
                    (bvh.nodes[0].aabb_max - bvh.nodes[0].aabb_min) * 0.5
                }
            }
        }
    }
}

impl std::fmt::Display for ColliderShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ColliderShape::Cuboid { .. } => "Cube",
            ColliderShape::Sphere { .. } => "Sphere",
            ColliderShape::Capsule { .. } => "Capsule",
            ColliderShape::Cylinder { .. } => "Cylinder",
            ColliderShape::Mesh { .. } => "Mesh",
        };
        f.write_str(name)
    }
}

#[component(networked)]
pub struct Collider {
    pub shape: ColliderShape,
    pub offset: Vector3<f32>,
    pub is_static: bool,
    pub is_area: bool,
}

impl Collider {
    /// Creates a dynamic collider.
    pub fn new(shape: ColliderShape, offset: Vector3<f32>) -> Self {
        Self {
            shape,
            offset,
            is_static: false,
            is_area: false,
        }
    }

    /// Creates a static collider.
    pub fn new_static(shape: ColliderShape, offset: Vector3<f32>) -> Self {
        Self {
            shape,
            offset,
            is_static: true,
            is_area: false,
        }
    }

    pub fn circle_2d(radius: f32) -> Self {
        Self::new(ColliderShape::circle(radius), Vector3::zeros())
    }

    pub fn rect_2d(half_width: f32, half_height: f32) -> Self {
        Self::new(
            ColliderShape::rect(half_width, half_height),
            Vector3::zeros(),
        )
    }

    /// Returns the world-space center of this collider (offset rotated by entity rotation).
    pub fn world_center(
        &self,
        position: Vector3<f32>,
        rotation: UnitQuaternion<f32>,
    ) -> Vector3<f32> {
        position + rotate_vector(rotation, self.offset)
    }

    /// Returns the three local axes of this OBB in world space.
    pub fn world_axes(&self, rotation: UnitQuaternion<f32>) -> [Vector3<f32>; 3] {
        [
            rotate_vector(rotation, Vector3::new(1.0, 0.0, 0.0)),
            rotate_vector(rotation, Vector3::new(0.0, 1.0, 0.0)),
            rotate_vector(rotation, Vector3::new(0.0, 0.0, 1.0)),
        ]
    }

    /// Returns the half-extents (collider_size is already treated as half-extents).
    pub fn half_extents(&self) -> Vector3<f32> {
        self.shape.half_extents()
    }

    /// Returns the minimum translation vector to separate this collider from `other`, or `None` if they are not overlapping.
    pub fn translation_vector_against(
        &self,
        pos_a: Vector3<f32>,
        rotation_a: UnitQuaternion<f32>,
        other: &Collider,
        pos_b: Vector3<f32>,
        rotation_b: UnitQuaternion<f32>,
    ) -> Option<Vector3<f32>> {
        let center_a = self.world_center(pos_a, rotation_a);
        let center_b = other.world_center(pos_b, rotation_b);
        let axes_a = self.world_axes(rotation_a);
        let axes_b = other.world_axes(rotation_b);
        let half_a = self.half_extents();
        let half_b = other.half_extents();

        let d = center_a - center_b;

        match (&self.shape, &other.shape) {
            (ColliderShape::Sphere { radius: ra }, ColliderShape::Sphere { radius: rb }) => {
                return sphere_vs_sphere(center_a, *ra, center_b, *rb);
            }

            (ColliderShape::Sphere { radius }, ColliderShape::Mesh { bvh, .. }) => {
                return sphere_vs_mesh(center_a, *radius, bvh, pos_b, rotation_b);
            }
            (ColliderShape::Capsule { radius, height }, ColliderShape::Mesh { bvh, .. }) => {
                let half_h = *height * 0.5;
                let up = rotation_a * Vector3::new(0.0, 1.0, 0.0);
                return capsule_vs_mesh(
                    center_a + up * half_h,
                    center_a - up * half_h,
                    *radius,
                    bvh,
                    pos_b,
                    rotation_b,
                );
            }
            (ColliderShape::Mesh { bvh, .. }, ColliderShape::Sphere { radius }) => {
                return sphere_vs_mesh(center_b, *radius, bvh, pos_a, rotation_a).map(|v| -v);
            }
            (ColliderShape::Mesh { bvh, .. }, ColliderShape::Capsule { radius, height }) => {
                let half_h = *height * 0.5;
                let up = rotation_b * Vector3::new(0.0, 1.0, 0.0);
                return capsule_vs_mesh(
                    center_b + up * half_h,
                    center_b - up * half_h,
                    *radius,
                    bvh,
                    pos_a,
                    rotation_a,
                )
                .map(|v| -v);
            }
            (
                ColliderShape::Cuboid { .. } | ColliderShape::Cylinder { .. },
                ColliderShape::Mesh { bvh, .. },
            ) => {
                return obb_vs_mesh(center_a, &axes_a, half_a, bvh, pos_b, rotation_b);
            }
            (
                ColliderShape::Mesh { bvh, .. },
                ColliderShape::Cuboid { .. } | ColliderShape::Cylinder { .. },
            ) => {
                return obb_vs_mesh(center_b, &axes_b, half_b, bvh, pos_a, rotation_a).map(|v| -v);
            }

            (ColliderShape::Sphere { radius }, _) => {
                return sphere_vs_obb(center_a, *radius, center_b, &axes_b, half_b);
            }
            (_, ColliderShape::Sphere { radius }) => {
                return sphere_vs_obb(center_b, *radius, center_a, &axes_a, half_a).map(|v| -v);
            }

            (ColliderShape::Mesh { bvh, .. }, ColliderShape::Mesh { bvh: bh2, .. }) => {
                return mesh_vs_mesh(bvh, pos_a, rotation_a, bh2, pos_b, rotation_b);
            }

            _ => {}
        }

        let mut min_overlap = f32::MAX;
        let mut min_axis = Vector3::new(0.0f32, 0.0, 0.0);
        let face_axes: [Vector3<f32>; 6] = [
            axes_a[0], axes_a[1], axes_a[2], axes_b[0], axes_b[1], axes_b[2],
        ];

        let mut edge_axes: Vec<Vector3<f32>> = Vec::new();
        for axis_a in &axes_a {
            for axis_b in &axes_b {
                let cross = axis_a.cross(axis_b);
                if cross.norm_squared() > 1e-10 {
                    edge_axes.push(cross.normalize());
                }
            }
        }

        let all_axes: Vec<Vector3<f32>> = face_axes.iter().copied().chain(edge_axes).collect();

        for axis in &all_axes {
            if axis.norm_squared() < 1e-10 {
                continue;
            }
            let axis = axis.normalize();

            let proj_a = project_obb(axis, &axes_a, half_a);
            let proj_b = project_obb(axis, &axes_b, half_b);
            let dist = d.dot(&axis).abs();
            let overlap = proj_a + proj_b - dist;

            if overlap <= 0.0 {
                return None; // Separating axis found, no collision
            }
            if overlap < min_overlap {
                min_overlap = overlap;
                // Ensure the MTV points from B toward A
                min_axis = if d.dot(&axis) >= 0.0 { axis } else { -axis };
            }
        }

        Some(min_axis * min_overlap)
    }

    /// Returns `true` if `point` (world space) lies inside this collider.
    pub fn contains_point(
        &self,
        position: Vector3<f32>,
        point: Vector3<f32>,
        rotation: UnitQuaternion<f32>,
    ) -> bool {
        let axes = self.world_axes(rotation);
        let half = self.half_extents();
        let center = self.world_center(position, rotation);
        let local = point - center;

        // Project the point onto each local axis and check against half-extent
        local.dot(&axes[0]).abs() <= half.x
            && local.dot(&axes[1]).abs() <= half.y
            && local.dot(&axes[2]).abs() <= half.z
    }
}

fn project_obb(axis: Vector3<f32>, obb_axes: &[Vector3<f32>; 3], half: Vector3<f32>) -> f32 {
    axis.dot(&obb_axes[0]).abs() * half.x
        + axis.dot(&obb_axes[1]).abs() * half.y
        + axis.dot(&obb_axes[2]).abs() * half.z
}

fn mesh_vs_mesh(
    bvh: &Bvh,
    mesh_pos: Vector3<f32>,
    mesh_rot: UnitQuaternion<f32>,
    bvh_2: &Bvh,
    mesh_pos_2: Vector3<f32>,
    mesh_rot_2: UnitQuaternion<f32>,
) -> Option<Vector3<f32>> {
    if bvh.nodes.is_empty() || bvh_2.nodes.is_empty() {
        return None;
    }

    let r = mesh_rot.conjugate() * mesh_rot_2;
    let t = mesh_rot.conjugate() * (mesh_pos_2 - mesh_pos);

    let rc = [
        r * Vector3::new(1.0, 0.0, 0.0),
        r * Vector3::new(0.0, 1.0, 0.0),
        r * Vector3::new(0.0, 0.0, 1.0),
    ];
    let transform_aabb = |min: Vector3<f32>, max: Vector3<f32>| {
        let center = r * ((min + max) * 0.5) + t;
        let e = (max - min) * 0.5;
        let extent = Vector3::new(
            rc[0].x.abs() * e.x + rc[1].x.abs() * e.y + rc[2].x.abs() * e.z,
            rc[0].y.abs() * e.x + rc[1].y.abs() * e.y + rc[2].y.abs() * e.z,
            rc[0].z.abs() * e.x + rc[1].z.abs() * e.y + rc[2].z.abs() * e.z,
        );
        (center - extent, center + extent)
    };

    let mut stack = vec![(0u32, 0u32)];
    let mut deepest: Option<Vector3<f32>> = None;
    let mut max_depth = 0.0f32;

    while let Some((idx, idx_2)) = stack.pop() {
        let node = &bvh.nodes[idx as usize];
        let node_2 = &bvh_2.nodes[idx_2 as usize];
        let (min_2, max_2) = transform_aabb(node_2.aabb_min, node_2.aabb_max);

        if !aabb_overlaps_aabb(node.aabb_min, node.aabb_max, min_2, max_2) {
            continue;
        }

        match (node.left == 0, node_2.left == 0) {
            (true, true) => {
                for i in node.tri_start..node.tri_start + node.tri_count {
                    let tri = &bvh.triangles[i as usize];
                    for x in node_2.tri_start..node_2.tri_start + node_2.tri_count {
                        let raw = &bvh_2.triangles[x as usize];
                        let tri_2 = [r * raw[0] + t, r * raw[1] + t, r * raw[2] + t];
                        if let Some((direction, depth)) = triangle_vs_triangle(tri, &tri_2)
                            && depth > max_depth
                        {
                            max_depth = depth;
                            deepest = Some(mesh_rot * direction * depth);
                        }
                    }
                }
            }
            (true, false) => {
                stack.push((idx, node_2.left));
                stack.push((idx, node_2.right));
            }
            (false, true) => {
                stack.push((node.left, idx_2));
                stack.push((node.right, idx_2));
            }
            (false, false) => {
                stack.push((node.left, node_2.left));
                stack.push((node.left, node_2.right));
                stack.push((node.right, node_2.left));
                stack.push((node.right, node_2.right));
            }
        }
    }

    deepest
}

fn obb_vs_mesh(
    center: Vector3<f32>,
    axes: &[Vector3<f32>; 3],
    half_extents: Vector3<f32>,
    bvh: &Bvh,
    mesh_pos: Vector3<f32>,
    mesh_rot: UnitQuaternion<f32>,
) -> Option<Vector3<f32>> {
    if bvh.nodes.is_empty() {
        return None;
    }

    // Transform the OBB into the mesh local space.
    let inv_rot = mesh_rot.conjugate();
    let local_center = inv_rot * (center - mesh_pos);
    let local_axes = [inv_rot * axes[0], inv_rot * axes[1], inv_rot * axes[2]];

    let extent = Vector3::new(
        project_obb(Vector3::new(1.0, 0.0, 0.0), &local_axes, half_extents),
        project_obb(Vector3::new(0.0, 1.0, 0.0), &local_axes, half_extents),
        project_obb(Vector3::new(0.0, 0.0, 1.0), &local_axes, half_extents),
    );
    let obb_min = local_center - extent;
    let obb_max = local_center + extent;

    let mut stack = vec![0u32];
    let mut deepest: Option<Vector3<f32>> = None;
    let mut max_depth = 0.0f32;

    while let Some(idx) = stack.pop() {
        let node = &bvh.nodes[idx as usize];

        if !aabb_overlaps_aabb(node.aabb_min, node.aabb_max, obb_min, obb_max) {
            continue;
        }

        if node.left == 0 {
            for i in node.tri_start..node.tri_start + node.tri_count {
                let tri = &bvh.triangles[i as usize];
                if let Some((direction, depth)) =
                    obb_vs_triangle(local_center, &local_axes, half_extents, tri)
                    && depth > max_depth
                {
                    max_depth = depth;
                    deepest = Some(mesh_rot * direction * depth);
                }
            }
        } else {
            stack.push(node.left);
            stack.push(node.right);
        }
    }

    deepest
}

fn triangle_vs_triangle(
    tri: &[Vector3<f32>; 3],
    tri_2: &[Vector3<f32>; 3],
) -> Option<(Vector3<f32>, f32)> {
    let edges = [tri[1] - tri[0], tri[2] - tri[1], tri[0] - tri[2]];
    let edges_2 = [
        tri_2[1] - tri_2[0],
        tri_2[2] - tri_2[1],
        tri_2[0] - tri_2[2],
    ];
    let n = edges[0].cross(&edges[1]);
    let n_2 = edges_2[0].cross(&edges_2[1]);

    let mut axes = [Vector3::zeros(); 17];
    axes[0] = n;
    axes[1] = n_2;
    let mut k = 2;
    for e in &edges {
        for e2 in &edges_2 {
            axes[k] = e.cross(e2);
            k += 1;
        }
    }
    for e in &edges {
        axes[k] = n.cross(e);
        k += 1;
    }
    for e2 in &edges_2 {
        axes[k] = n_2.cross(e2);
        k += 1;
    }

    let mut min_overlap = f32::MAX;
    let mut min_axis = Vector3::zeros();

    for axis in axes {
        if axis.norm_squared() < 1e-10 {
            continue;
        }
        let axis = axis.normalize();
        let (min_a, max_a) = project_triangle(axis, tri);
        let (min_b, max_b) = project_triangle(axis, tri_2);
        let overlap = (max_b - min_a).min(max_a - min_b);
        if overlap <= 0.0 {
            return None; // Separating axis found, no collision
        }
        if overlap < min_overlap {
            min_overlap = overlap;
            min_axis = axis;
        }
    }

    if min_overlap == f32::MAX {
        return None;
    }

    let centroid = (tri[0] + tri[1] + tri[2]) / 3.0;
    let centroid_2 = (tri_2[0] + tri_2[1] + tri_2[2]) / 3.0;
    if (centroid - centroid_2).dot(&min_axis) < 0.0 {
        min_axis = -min_axis;
    }

    Some((min_axis, min_overlap))
}

fn project_triangle(axis: Vector3<f32>, tri: &[Vector3<f32>; 3]) -> (f32, f32) {
    let d0 = tri[0].dot(&axis);
    let d1 = tri[1].dot(&axis);
    let d2 = tri[2].dot(&axis);
    (d0.min(d1).min(d2), d0.max(d1).max(d2))
}

fn obb_vs_triangle(
    center: Vector3<f32>,
    axes: &[Vector3<f32>; 3],
    half_extents: Vector3<f32>,
    tri: &[Vector3<f32>; 3],
) -> Option<(Vector3<f32>, f32)> {
    let tri_normal = triangle_normal(tri);
    if tri_normal.norm_squared() < 1e-10 {
        return None;
    }
    let tri_normal = tri_normal.normalize();

    let plane_value = tri[0].dot(&tri_normal);
    let box_center_proj = center.dot(&tri_normal);
    let box_radius = project_obb(tri_normal, axes, half_extents);
    let overlap = box_radius - (box_center_proj - plane_value).abs();
    if overlap <= 0.0 {
        return None;
    }

    let min_axis = if box_center_proj >= plane_value {
        tri_normal
    } else {
        -tri_normal
    };

    Some((min_axis, overlap))
}
