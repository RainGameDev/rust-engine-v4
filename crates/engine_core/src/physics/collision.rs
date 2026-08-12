use nalgebra::{UnitQuaternion, Vector3};

use crate::{
    ecs::{World, components::engine_components::transform::Transform, entities::Entity},
    physics::{
        bvh::Bvh,
        math::{
            closest_point_on_triangle, closest_points_segment_triangle, triangle_normal,
        },
        velocity::Velocity,
    },
};

pub fn apply_position_correction(world: &mut World, id: Entity, offset: Vector3<f32>) {
    if offset.norm_squared() < 1e-6 {
        return;
    }
    let max_correction = 3.0;
    let offset = if offset.norm_squared() > max_correction * max_correction {
        offset.normalize() * max_correction
    } else {
        offset
    };
    // Compute local-space delta before taking mutable borrows on `world`.
    let local_delta = if let Some(parent_id) = world.get_parent_id(id) {
        if let Some(parent_t) = world.get_component::<Transform>(parent_id) {
            let pr = parent_t.global_rotation;
            let inv_pr = pr.conjugate();
            inv_pr * offset
        } else {
            offset
        }
    } else {
        offset
    };

    if let Some(t) = world.get_component_mut::<Transform>(id) {
        // Apply correction in world space
        t.global_position += offset;
        // Apply the precomputed local delta
        t.position += local_delta;
    }
}

pub fn resolve_impulse(
    vel_a: &mut Velocity,
    vel_b: &mut Velocity,
    r_a: Vector3<f32>,
    r_b: Vector3<f32>,
    normal: Vector3<f32>,
) {
    let inv_mass_a = if vel_a.mass > 0.0 {
        1.0 / vel_a.mass
    } else {
        0.0
    };
    let inv_mass_b = if vel_b.mass > 0.0 {
        1.0 / vel_b.mass
    } else {
        0.0
    };
    let inv_i_a = vel_a
        .inertia_tensor
        .map(|v| if v > 0.0 { 1.0 / v } else { 0.0 });
    let inv_i_b = vel_b
        .inertia_tensor
        .map(|v| if v > 0.0 { 1.0 / v } else { 0.0 });

    // Velocity at contact point including angular contribution
    let va_contact = vel_a.linear_velocity + vel_a.angular_velocity.cross(&r_a);
    let vb_contact = vel_b.linear_velocity + vel_b.angular_velocity.cross(&r_b);
    let relative_vel = va_contact - vb_contact;

    let vel_along_normal = relative_vel.dot(&normal);

    // Already separating
    if vel_along_normal > 0.0 {
        return;
    }

    // Angular contribution
    let ang_term = |inv_i: Vector3<f32>, r: Vector3<f32>| {
        let rxn = r.cross(&normal);
        // apply diagonal inverse inertia tensor
        let i_inv_rxn = Vector3::new(inv_i.x * rxn.x, inv_i.y * rxn.y, inv_i.z * rxn.z);
        i_inv_rxn.cross(&r).dot(&normal)
    };

    let restitution = if vel_along_normal > -0.5 {
        0.0
    } else {
        vel_a.restitution.min(vel_b.restitution)
    };
    let j_denom = inv_mass_a + inv_mass_b + ang_term(inv_i_a, r_a) + ang_term(inv_i_b, r_b);
    if j_denom <= 1e-6 {
        return;
    }
    let j = -(1.0 + restitution) * vel_along_normal / j_denom;

    // Apply normal impulse
    vel_a.linear_velocity += normal * (j * inv_mass_a);
    vel_b.linear_velocity -= normal * (j * inv_mass_b);
    vel_a.angular_velocity += apply_inv_inertia(inv_i_a, r_a.cross(&(normal * j)));
    vel_b.angular_velocity -= apply_inv_inertia(inv_i_b, r_b.cross(&(normal * j)));

    // Friction
    let va_contact = vel_a.linear_velocity + vel_a.angular_velocity.cross(&r_a);
    let vb_contact = vel_b.linear_velocity + vel_b.angular_velocity.cross(&r_b);
    let relative_vel = va_contact - vb_contact;

    let tangential = relative_vel - normal * relative_vel.dot(&normal);

    // No sliding
    if tangential.norm_squared() < 1e-8 {
        return;
    }
    let tangent = tangential.normalize();

    // Same denominator structure but along tangent
    let ang_term_t = |inv_i: Vector3<f32>, r: Vector3<f32>| {
        let rxt = r.cross(&tangent);
        let i_inv_rxt = Vector3::new(inv_i.x * rxt.x, inv_i.y * rxt.y, inv_i.z * rxt.z);
        i_inv_rxt.cross(&r).dot(&tangent)
    };

    let jt_denom = inv_mass_a + inv_mass_b + ang_term_t(inv_i_a, r_a) + ang_term_t(inv_i_b, r_b);
    let jt = -relative_vel.dot(&tangent) / jt_denom;

    // clamp static vs kinetic
    let mu_s = (vel_a.mu_static * vel_b.mu_static).sqrt();
    let mu_k = (vel_a.mu_kinetic * vel_b.mu_kinetic).sqrt();

    let friction_impulse = if jt.abs() <= j * mu_s {
        tangent * jt
    } else {
        tangent * (j * mu_k * -jt.signum())
    };

    // Don't let friction increase tangential speed.
    let orig_va_contact = vel_a.linear_velocity + vel_a.angular_velocity.cross(&r_a);
    let orig_vb_contact = vel_b.linear_velocity + vel_b.angular_velocity.cross(&r_b);
    let orig_rel_tang = orig_va_contact
        - orig_vb_contact
        - normal * (orig_va_contact - orig_vb_contact).dot(&normal);
    let orig_tang_mag = orig_rel_tang.norm();

    // Tentatively apply linear part of friction impulse to test effect on tangential velocity
    let test_va_lin = vel_a.linear_velocity + friction_impulse * inv_mass_a;
    let test_vb_lin = vel_b.linear_velocity - friction_impulse * inv_mass_b;
    let test_va_contact = test_va_lin + vel_a.angular_velocity.cross(&r_a);
    let test_vb_contact = test_vb_lin + vel_b.angular_velocity.cross(&r_b);
    let test_rel_tang = test_va_contact
        - test_vb_contact
        - normal * (test_va_contact - test_vb_contact).dot(&normal);
    let test_tang_mag = test_rel_tang.norm();

    let final_friction = if test_tang_mag > orig_tang_mag + 1e-6 {
        // scale down impulse to not increase tangential speed
        if test_tang_mag.abs() > 1e-9 {
            friction_impulse * (orig_tang_mag / test_tang_mag)
        } else {
            Vector3::new(0.0, 0.0, 0.0)
        }
    } else {
        friction_impulse
    };

    vel_a.linear_velocity += final_friction * inv_mass_a;
    vel_b.linear_velocity -= final_friction * inv_mass_b;
    vel_a.angular_velocity += apply_inv_inertia(inv_i_a, r_a.cross(&final_friction));
    vel_b.angular_velocity -= apply_inv_inertia(inv_i_b, r_b.cross(&final_friction));
}

pub fn apply_inv_inertia(inv_i: Vector3<f32>, torque: Vector3<f32>) -> Vector3<f32> {
    Vector3::new(inv_i.x * torque.x, inv_i.y * torque.y, inv_i.z * torque.z)
}

pub fn sphere_vs_sphere(
    center_a: Vector3<f32>,
    ra: f32,
    center_b: Vector3<f32>,
    rb: f32,
) -> Option<Vector3<f32>> {
    let d = center_a - center_b;
    let dist2 = d.norm_squared();
    let sum = ra + rb;
    if dist2 >= sum * sum {
        return None;
    }
    let dist = dist2.sqrt();
    let normal = if dist > 1e-10 {
        d / dist
    } else {
        Vector3::new(0.0, 1.0, 0.0)
    };
    Some(normal * (sum - dist))
}

pub fn sphere_vs_obb(
    sphere_center: Vector3<f32>,
    radius: f32,
    obb_center: Vector3<f32>,
    obb_axes: &[Vector3<f32>; 3],
    obb_half: Vector3<f32>,
) -> Option<Vector3<f32>> {
    let d = sphere_center - obb_center;
    // Find closest point on OBB to sphere center
    let closest = obb_center
        + obb_axes[0] * d.dot(&obb_axes[0]).clamp(-obb_half.x, obb_half.x)
        + obb_axes[1] * d.dot(&obb_axes[1]).clamp(-obb_half.y, obb_half.y)
        + obb_axes[2] * d.dot(&obb_axes[2]).clamp(-obb_half.z, obb_half.z);

    let diff = sphere_center - closest;
    let dist2 = diff.norm_squared();
    if dist2 >= radius * radius {
        return None;
    }
    let dist = dist2.sqrt();
    let normal = if dist > 1e-10 {
        diff / dist
    } else {
        Vector3::new(0.0, 1.0, 0.0)
    };
    Some(normal * (radius - dist))
}

pub fn capsule_vs_mesh(
    seg_a: Vector3<f32>,
    seg_b: Vector3<f32>,
    radius: f32,
    bvh: &Bvh,
    mesh_pos: Vector3<f32>,
    mesh_rot: UnitQuaternion<f32>,
) -> Option<Vector3<f32>> {
    if bvh.nodes.is_empty() {
        return None;
    }
    let inv_rot = mesh_rot.conjugate();
    let local_a = inv_rot * (seg_a - mesh_pos);
    let local_b = inv_rot * (seg_b - mesh_pos);

    // AABB of the full capsule for BVH pruning
    let cap_min = Vector3::new(
        local_a.x.min(local_b.x) - radius,
        local_a.y.min(local_b.y) - radius,
        local_a.z.min(local_b.z) - radius,
    );
    let cap_max = Vector3::new(
        local_a.x.max(local_b.x) + radius,
        local_a.y.max(local_b.y) + radius,
        local_a.z.max(local_b.z) + radius,
    );

    let mut stack = vec![0u32];
    let mut deepest: Option<Vector3<f32>> = None;
    let mut max_depth = 0.0f32;

    while let Some(idx) = stack.pop() {
        let node = &bvh.nodes[idx as usize];

        if !aabb_overlaps_aabb(node.aabb_min, node.aabb_max, cap_min, cap_max) {
            continue;
        }

        if node.left == 0 {
            for i in node.tri_start..node.tri_start + node.tri_count {
                let tri = &bvh.triangles[i as usize];
                let (closest_seg, closest_tri) =
                    closest_points_segment_triangle(local_a, local_b, tri);
                let diff = closest_seg - closest_tri;
                let dist2 = diff.norm_squared();
                if dist2 < radius * radius {
                    let dist = dist2.sqrt();
                    let depth = radius - dist;
                    if depth > max_depth {
                        max_depth = depth;
                        let face_normal = triangle_normal(tri);
                        let mut local_normal = if dist > 1e-10 {
                            diff / dist
                        } else {
                            face_normal
                        };
                        if local_normal.dot(&face_normal) < 0.0 {
                            local_normal = face_normal;
                        }
                        deepest = Some(mesh_rot * local_normal * depth);
                    }
                }
            }
        } else {
            stack.push(node.left);
            stack.push(node.right);
        }
    }

    deepest
}

pub fn sphere_vs_mesh(
    center: Vector3<f32>,
    radius: f32,
    bvh: &Bvh,
    mesh_pos: Vector3<f32>,
    mesh_rot: UnitQuaternion<f32>,
) -> Option<Vector3<f32>> {
    if bvh.nodes.is_empty() {
        return None;
    }
    let inv_rot = mesh_rot.conjugate();
    let local_center = inv_rot * (center - mesh_pos);

    let mut stack = vec![0u32];
    let mut deepest: Option<Vector3<f32>> = None;
    let mut max_depth = 0.0f32;

    while let Some(idx) = stack.pop() {
        let node = &bvh.nodes[idx as usize];

        if !aabb_overlaps_sphere(node.aabb_min, node.aabb_max, local_center, radius) {
            continue;
        }

        if node.left == 0 {
            for i in node.tri_start..node.tri_start + node.tri_count {
                let tri = &bvh.triangles[i as usize];
                let closest = closest_point_on_triangle(local_center, tri);
                let diff = local_center - closest;
                let dist2 = diff.norm_squared();
                if dist2 < radius * radius {
                    let dist = dist2.sqrt();
                    let depth = radius - dist;
                    if depth > max_depth {
                        max_depth = depth;
                        let local_normal = if dist > 1e-10 {
                            diff / dist
                        } else {
                            triangle_normal(tri)
                        };
                        deepest = Some(mesh_rot * local_normal * depth);
                    }
                }
            }
        } else {
            stack.push(node.left);
            stack.push(node.right);
        }
    }

    deepest
}

pub fn aabb_overlaps_sphere(
    min: Vector3<f32>,
    max: Vector3<f32>,
    center: Vector3<f32>,
    radius: f32,
) -> bool {
    let closest = Vector3::new(
        center.x.clamp(min.x, max.x),
        center.y.clamp(min.y, max.y),
        center.z.clamp(min.z, max.z),
    );
    (center - closest).norm_squared() <= radius * radius
}

pub fn aabb_overlaps_aabb(
    min_a: Vector3<f32>,
    max_a: Vector3<f32>,
    min_b: Vector3<f32>,
    max_b: Vector3<f32>,
) -> bool {
    min_a.x <= max_b.x
        && max_a.x >= min_b.x
        && min_a.y <= max_b.y
        && max_a.y >= min_b.y
        && min_a.z <= max_b.z
        && max_a.z >= min_b.z
}
