use nalgebra::{Quaternion, UnitQuaternion, Vector3};

/// Rotates a vector by a quaternion
pub fn rotate_vector(q: UnitQuaternion<f32>, v: Vector3<f32>) -> Vector3<f32> {
    let qv = Vector3::new(q.i, q.j, q.k);
    let t = qv.cross(&v) * 2.0;
    v + t * q.w + qv.cross(&t)
}

/// Calculated the AABB of a triangle.
pub fn triangle_aabb(tris: &[[Vector3<f32>; 3]]) -> (Vector3<f32>, Vector3<f32>) {
    let mut min = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Vector3::new(f32::MIN, f32::MIN, f32::MIN);
    for tri in tris {
        for v in tri {
            min.x = min.x.min(v.x);
            min.y = min.y.min(v.y);
            min.z = min.z.min(v.z);
            max.x = max.x.max(v.x);
            max.y = max.y.max(v.y);
            max.z = max.z.max(v.z);
        }
    }
    (min, max)
}

pub fn closest_points_segment_segment(
    p1: Vector3<f32>,
    p2: Vector3<f32>,
    p3: Vector3<f32>,
    p4: Vector3<f32>,
) -> (Vector3<f32>, Vector3<f32>) {
    let d1 = p2 - p1;
    let d2 = p4 - p3;
    let r = p1 - p3;
    let a = d1.norm_squared();
    let e = d2.norm_squared();
    let f = d2.dot(&r);
    let (s, t) = if a <= 1e-10 {
        (0.0f32, (f / e).clamp(0.0, 1.0))
    } else {
        let c = d1.dot(&r);
        if e <= 1e-10 {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b_dot = d1.dot(&d2);
            let denom = a * e - b_dot * b_dot;
            let s = if denom > 1e-10 {
                ((b_dot * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let t = (b_dot * s + f) / e;
            if t < 0.0 {
                ((-c / a).clamp(0.0, 1.0), 0.0)
            } else if t > 1.0 {
                (((b_dot - c) / a).clamp(0.0, 1.0), 1.0)
            } else {
                (s, t)
            }
        }
    };
    (p1 + d1 * s, p3 + d2 * t)
}

// Returns (closest point on segment, closest point on triangle)
pub fn closest_points_segment_triangle(
    seg_a: Vector3<f32>,
    seg_b: Vector3<f32>,
    tri: &[Vector3<f32>; 3],
) -> (Vector3<f32>, Vector3<f32>) {
    let mut best_dist2 = f32::MAX;
    let mut best = (seg_a, tri[0]);

    // capsule endpoint A vs triangle
    let q = closest_point_on_triangle(seg_a, tri);
    let d2 = (seg_a - q).norm_squared();
    if d2 < best_dist2 {
        best_dist2 = d2;
        best = (seg_a, q);
    }

    // capsule endpoint B vs triangle
    let q = closest_point_on_triangle(seg_b, tri);
    let d2 = (seg_b - q).norm_squared();
    if d2 < best_dist2 {
        best_dist2 = d2;
        best = (seg_b, q);
    }

    // capsule segment vs each triangle edge
    let edges = [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])];
    for (e0, e1) in edges {
        let (ps, pt) = closest_points_segment_segment(seg_a, seg_b, e0, e1);
        let d2 = (ps - pt).norm_squared();
        if d2 < best_dist2 {
            best_dist2 = d2;
            best = (ps, pt);
        }
    }

    best
}

// Christer Ericson's closest point on triangle algorithm
pub fn closest_point_on_triangle(p: Vector3<f32>, tri: &[Vector3<f32>; 3]) -> Vector3<f32> {
    let (a, b, c) = (tri[0], tri[1], tri[2]);
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(&ap);
    let d2 = ac.dot(&ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let d3 = ab.dot(&bp);
    let d4 = ac.dot(&bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let cp = p - c;
    let d5 = ab.dot(&cp);
    let d6 = ac.dot(&cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denom = 1.0 / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}

pub fn triangle_normal(tri: &[Vector3<f32>; 3]) -> Vector3<f32> {
    let n = (tri[1] - tri[0]).cross(&(tri[2] - tri[0]));
    if n.norm_squared() > 1e-10 {
        n.normalize()
    } else {
        Vector3::new(0.0, 1.0, 0.0)
    }
}
