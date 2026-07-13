//! P17 - N-D affine, projective, convex, Lie, and smooth geometry.

use super::primitives::Point3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AffineFrame3 {
    pub origin: Point3,
    pub e0: Point3,
    pub e1: Point3,
    pub e2: Point3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeparatingPlane {
    pub normal: Point3,
    pub offset: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HomogeneousPoint3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hyperplane3 {
    pub normal: Point3,
    pub offset: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConvexityCertificate {
    pub is_convex: bool,
    pub witness: [u32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadraticSolution {
    pub x: [f64; 3],
    pub objective: f64,
    pub stationarity_residual: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveDifferential {
    pub tangent: Point3,
    pub curvature: f64,
    pub torsion: f64,
    pub arc_speed: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceDifferential {
    pub normal: Point3,
    pub first_form: [[f64; 2]; 2],
    pub gaussian: f64,
    pub mean: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathGeometryError {
    DegenerateFrame,
    DegenerateSimplex,
    CountMismatch,
    EmptyInput,
}

pub fn projective_from_point(p: Point3) -> HomogeneousPoint3 {
    HomogeneousPoint3 {
        x: p.x,
        y: p.y,
        z: p.z,
        w: 1.0,
    }
}

pub fn point_from_projective(p: HomogeneousPoint3) -> Result<Point3, MathGeometryError> {
    if p.w.abs() < 1e-14 {
        return Err(MathGeometryError::DegenerateFrame);
    }
    Ok(Point3::new(p.x / p.w, p.y / p.w, p.z / p.w))
}

pub fn hyperplane_eval(plane: Hyperplane3, p: Point3) -> f64 {
    dot(plane.normal, p) + plane.offset
}

pub fn cross_ratio_1d(a: f64, b: f64, c: f64, d: f64) -> Result<f64, MathGeometryError> {
    let denom = (a - d) * (b - c);
    if denom.abs() < 1e-14 {
        return Err(MathGeometryError::DegenerateSimplex);
    }
    Ok(((a - c) * (b - d)) / denom)
}

pub fn frame_to_world(frame: AffineFrame3, coords: [f64; 3]) -> Point3 {
    add(
        frame.origin,
        add(
            scale(frame.e0, coords[0]),
            add(scale(frame.e1, coords[1]), scale(frame.e2, coords[2])),
        ),
    )
}

pub fn world_to_frame(frame: AffineFrame3, p: Point3) -> Result<[f64; 3], MathGeometryError> {
    let v = sub(p, frame.origin);
    let det = dot(frame.e0, cross(frame.e1, frame.e2));
    if det.abs() < 1e-14 {
        return Err(MathGeometryError::DegenerateFrame);
    }
    Ok([
        dot(v, cross(frame.e1, frame.e2)) / det,
        dot(frame.e0, cross(v, frame.e2)) / det,
        dot(frame.e0, cross(frame.e1, v)) / det,
    ])
}

pub fn barycentric_tetra(
    p: Point3,
    a: Point3,
    b: Point3,
    c: Point3,
    d: Point3,
) -> Result<[f64; 4], MathGeometryError> {
    let det = signed_volume6(a, b, c, d);
    if det.abs() < 1e-14 {
        return Err(MathGeometryError::DegenerateSimplex);
    }
    let l0 = signed_volume6(p, b, c, d) / det;
    let l1 = signed_volume6(a, p, c, d) / det;
    let l2 = signed_volume6(a, b, p, d) / det;
    let l3 = signed_volume6(a, b, c, p) / det;
    Ok([l0, l1, l2, l3])
}

pub fn quaternion_normalize(q: Quaternion) -> Quaternion {
    let n = (q.w * q.w + q.x * q.x + q.y * q.y + q.z * q.z).sqrt();
    if n == 0.0 {
        Quaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    } else {
        Quaternion {
            w: q.w / n,
            x: q.x / n,
            y: q.y / n,
            z: q.z / n,
        }
    }
}

pub fn quaternion_slerp(a: Quaternion, b: Quaternion, t: f64) -> Quaternion {
    let a = quaternion_normalize(a);
    let mut b = quaternion_normalize(b);
    let mut cos = a.w * b.w + a.x * b.x + a.y * b.y + a.z * b.z;
    if cos < 0.0 {
        cos = -cos;
        b = Quaternion {
            w: -b.w,
            x: -b.x,
            y: -b.y,
            z: -b.z,
        };
    }
    if cos > 0.9995 {
        return quaternion_normalize(Quaternion {
            w: a.w + t * (b.w - a.w),
            x: a.x + t * (b.x - a.x),
            y: a.y + t * (b.y - a.y),
            z: a.z + t * (b.z - a.z),
        });
    }
    let theta = cos.acos();
    let s0 = ((1.0 - t) * theta).sin() / theta.sin();
    let s1 = (t * theta).sin() / theta.sin();
    Quaternion {
        w: s0 * a.w + s1 * b.w,
        x: s0 * a.x + s1 * b.x,
        y: s0 * a.y + s1 * b.y,
        z: s0 * a.z + s1 * b.z,
    }
}

pub fn best_translation_registration(
    source: &[Point3],
    target: &[Point3],
) -> Result<Point3, MathGeometryError> {
    if source.is_empty() {
        return Err(MathGeometryError::EmptyInput);
    }
    if source.len() != target.len() {
        return Err(MathGeometryError::CountMismatch);
    }
    Ok(sub(centroid(target), centroid(source)))
}

pub fn caratheodory_reduce_3d(
    points: &[Point3],
    target: Point3,
    out_indices: &mut [u32; 4],
    out_weights: &mut [f64; 4],
) -> Result<usize, MathGeometryError> {
    if points.is_empty() {
        return Err(MathGeometryError::EmptyInput);
    }
    for i in 0..points.len() {
        if distance_sq(points[i], target) <= 1e-18 {
            out_indices[0] = i as u32;
            out_weights[0] = 1.0;
            return Ok(1);
        }
    }
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            let ab = sub(points[j], points[i]);
            let denom = dot(ab, ab);
            if denom > 0.0 {
                let t = (dot(sub(target, points[i]), ab) / denom).clamp(0.0, 1.0);
                let p = add(points[i], scale(ab, t));
                if distance_sq(p, target) <= 1e-12 {
                    out_indices[0] = i as u32;
                    out_indices[1] = j as u32;
                    out_weights[0] = 1.0 - t;
                    out_weights[1] = t;
                    return Ok(2);
                }
            }
        }
    }
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            for k in j + 1..points.len() {
                if let Some(w) = barycentric_triangle(target, points[i], points[j], points[k]) {
                    if w.iter().all(|x| *x >= -1e-10) {
                        out_indices[..3].copy_from_slice(&[i as u32, j as u32, k as u32]);
                        out_weights[..3].copy_from_slice(&w);
                        return Ok(3);
                    }
                }
            }
        }
    }
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            for k in j + 1..points.len() {
                for l in k + 1..points.len() {
                    let w = barycentric_tetra(target, points[i], points[j], points[k], points[l])?;
                    if w.iter().all(|x| *x >= -1e-10) {
                        out_indices.copy_from_slice(&[i as u32, j as u32, k as u32, l as u32]);
                        out_weights.copy_from_slice(&w);
                        return Ok(4);
                    }
                }
            }
        }
    }
    Err(MathGeometryError::DegenerateSimplex)
}

pub fn convexity_certificate_polyline(points: &[Point3]) -> ConvexityCertificate {
    if points.len() < 4 {
        return ConvexityCertificate {
            is_convex: true,
            witness: [0, 0, 0, 0],
        };
    }
    let base_normal = cross(sub(points[1], points[0]), sub(points[2], points[1]));
    let mut sign = 0.0;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let c = points[(i + 2) % points.len()];
        let s = dot(cross(sub(b, a), sub(c, b)), base_normal);
        if s.abs() > 1e-12 {
            if sign == 0.0 {
                sign = s.signum();
            } else if sign * s < 0.0 {
                return ConvexityCertificate {
                    is_convex: false,
                    witness: [
                        i as u32,
                        ((i + 1) % points.len()) as u32,
                        ((i + 2) % points.len()) as u32,
                        0,
                    ],
                };
            }
        }
    }
    ConvexityCertificate {
        is_convex: true,
        witness: [0, 0, 0, 0],
    }
}

pub fn householder_reflect(v: Point3, normal: Point3) -> Point3 {
    let n = normalize(normal);
    sub(v, scale(n, 2.0 * dot(v, n)))
}

pub fn quaternion_to_matrix(q: Quaternion) -> [[f64; 3]; 3] {
    let q = quaternion_normalize(q);
    let (w, x, y, z) = (q.w, q.x, q.y, q.z);
    [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - z * w),
            2.0 * (x * z + y * w),
        ],
        [
            2.0 * (x * y + z * w),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - x * w),
        ],
        [
            2.0 * (x * z - y * w),
            2.0 * (y * z + x * w),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ]
}

pub fn solve_diagonal_quadratic(
    diag: [f64; 3],
    linear: [f64; 3],
) -> Result<QuadraticSolution, MathGeometryError> {
    let mut x = [0.0; 3];
    let mut residual: f64 = 0.0;
    let mut objective = 0.0;
    for i in 0..3 {
        if diag[i] <= 0.0 || !diag[i].is_finite() {
            return Err(MathGeometryError::DegenerateFrame);
        }
        x[i] = -linear[i] / diag[i];
        objective += 0.5 * diag[i] * x[i] * x[i] + linear[i] * x[i];
        residual = residual.max((diag[i] * x[i] + linear[i]).abs());
    }
    Ok(QuadraticSolution {
        x,
        objective,
        stationarity_residual: residual,
    })
}

pub fn schur_complement_2x2(
    a00: f64,
    a01: f64,
    a11: f64,
    b: [f64; 2],
) -> Result<f64, MathGeometryError> {
    let det = a00 * a11 - a01 * a01;
    if det.abs() < 1e-14 {
        return Err(MathGeometryError::DegenerateFrame);
    }
    Ok(b[0] * b[0] * a11 / det - 2.0 * b[0] * b[1] * a01 / det + b[1] * b[1] * a00 / det)
}

pub fn so3_exp(axis_angle: Point3) -> Quaternion {
    let theta = norm(axis_angle);
    if theta < 1e-12 {
        return quaternion_normalize(Quaternion {
            w: 1.0,
            x: 0.5 * axis_angle.x,
            y: 0.5 * axis_angle.y,
            z: 0.5 * axis_angle.z,
        });
    }
    let axis = scale(axis_angle, 1.0 / theta);
    let half = theta * 0.5;
    Quaternion {
        w: half.cos(),
        x: axis.x * half.sin(),
        y: axis.y * half.sin(),
        z: axis.z * half.sin(),
    }
}

pub fn so3_log(q: Quaternion) -> Point3 {
    let q = quaternion_normalize(q);
    let v = Point3::new(q.x, q.y, q.z);
    let s = norm(v);
    if s < 1e-12 {
        return scale(v, 2.0);
    }
    let theta = 2.0 * s.atan2(q.w);
    scale(v, theta / s)
}

pub fn curve_differential(
    p0: Point3,
    p1: Point3,
    p2: Point3,
    p3: Point3,
    dt: f64,
) -> Result<CurveDifferential, MathGeometryError> {
    if !(dt.is_finite() && dt > 0.0) {
        return Err(MathGeometryError::DegenerateSimplex);
    }
    let velocity = scale(sub(p2, p0), 0.5 / dt);
    let accel = scale(add(sub(p2, scale(p1, 2.0)), p0), 1.0 / (dt * dt));
    let jerk = scale(
        add(sub(p3, scale(p2, 3.0)), sub(scale(p1, 3.0), p0)),
        1.0 / (dt * dt * dt),
    );
    let speed = norm(velocity);
    if speed < 1e-14 {
        return Err(MathGeometryError::DegenerateSimplex);
    }
    let cross_va = cross(velocity, accel);
    let curvature = norm(cross_va) / speed.powi(3);
    let torsion = if norm(cross_va) > 1e-14 {
        dot(cross_va, jerk) / dot(cross_va, cross_va)
    } else {
        0.0
    };
    Ok(CurveDifferential {
        tangent: scale(velocity, 1.0 / speed),
        curvature,
        torsion,
        arc_speed: speed,
    })
}

pub fn surface_patch_differential(
    center: Point3,
    du: Point3,
    dv: Point3,
    duu: Point3,
    duv: Point3,
    dvv: Point3,
) -> Result<SurfaceDifferential, MathGeometryError> {
    let normal = normalize(cross(du, dv));
    if norm(normal) < 1e-14 {
        return Err(MathGeometryError::DegenerateSimplex);
    }
    let e = dot(du, du);
    let f = dot(du, dv);
    let g = dot(dv, dv);
    let l = dot(duu, normal);
    let m = dot(duv, normal);
    let n = dot(dvv, normal);
    let denom = e * g - f * f;
    if denom.abs() < 1e-14 || !center.x.is_finite() {
        return Err(MathGeometryError::DegenerateSimplex);
    }
    let gaussian = (l * n - m * m) / denom;
    let mean = (e * n - 2.0 * f * m + g * l) / (2.0 * denom);
    Ok(SurfaceDifferential {
        normal,
        first_form: [[e, f], [f, g]],
        gaussian,
        mean,
    })
}

pub fn project_points_to_plane(
    points: &[Point3],
    plane: Hyperplane3,
    out: &mut [Point3],
) -> Result<usize, MathGeometryError> {
    if out.len() < points.len() {
        return Err(MathGeometryError::CountMismatch);
    }
    let n = normalize(plane.normal);
    for (slot, &p) in out.iter_mut().zip(points.iter()) {
        *slot = sub(p, scale(n, dot(n, p) + plane.offset));
    }
    Ok(points.len())
}

pub fn separating_plane_aabb(
    a_min: Point3,
    a_max: Point3,
    b_min: Point3,
    b_max: Point3,
) -> Option<SeparatingPlane> {
    if a_max.x < b_min.x {
        return Some(SeparatingPlane {
            normal: Point3::new(1.0, 0.0, 0.0),
            offset: -0.5 * (a_max.x + b_min.x),
        });
    }
    if b_max.x < a_min.x {
        return Some(SeparatingPlane {
            normal: Point3::new(-1.0, 0.0, 0.0),
            offset: 0.5 * (b_max.x + a_min.x),
        });
    }
    if a_max.y < b_min.y {
        return Some(SeparatingPlane {
            normal: Point3::new(0.0, 1.0, 0.0),
            offset: -0.5 * (a_max.y + b_min.y),
        });
    }
    if b_max.y < a_min.y {
        return Some(SeparatingPlane {
            normal: Point3::new(0.0, -1.0, 0.0),
            offset: 0.5 * (b_max.y + a_min.y),
        });
    }
    if a_max.z < b_min.z {
        return Some(SeparatingPlane {
            normal: Point3::new(0.0, 0.0, 1.0),
            offset: -0.5 * (a_max.z + b_min.z),
        });
    }
    if b_max.z < a_min.z {
        return Some(SeparatingPlane {
            normal: Point3::new(0.0, 0.0, -1.0),
            offset: 0.5 * (b_max.z + a_min.z),
        });
    }
    None
}

fn centroid(points: &[Point3]) -> Point3 {
    let mut c = Point3::new(0.0, 0.0, 0.0);
    for &p in points {
        c = add(c, p);
    }
    scale(c, 1.0 / points.len() as f64)
}

fn signed_volume6(a: Point3, b: Point3, c: Point3, d: Point3) -> f64 {
    dot(sub(b, a), cross(sub(c, a), sub(d, a)))
}

fn barycentric_triangle(p: Point3, a: Point3, b: Point3, c: Point3) -> Option<[f64; 3]> {
    let v0 = sub(b, a);
    let v1 = sub(c, a);
    let v2 = sub(p, a);
    let d00 = dot(v0, v0);
    let d01 = dot(v0, v1);
    let d11 = dot(v1, v1);
    let d20 = dot(v2, v0);
    let d21 = dot(v2, v1);
    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-14 {
        return None;
    }
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    Some([1.0 - v - w, v, w])
}

fn add(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn scale(a: Point3, s: f64) -> Point3 {
    Point3::new(a.x * s, a.y * s, a.z * s)
}

fn dot(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn cross(a: Point3, b: Point3) -> Point3 {
    Point3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

fn norm(a: Point3) -> f64 {
    dot(a, a).sqrt()
}

fn normalize(a: Point3) -> Point3 {
    let n = norm(a);
    if n > 0.0 {
        scale(a, 1.0 / n)
    } else {
        Point3::new(0.0, 0.0, 0.0)
    }
}

fn distance_sq(a: Point3, b: Point3) -> f64 {
    dot(sub(a, b), sub(a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_round_trip() {
        let f = AffineFrame3 {
            origin: Point3::new(1.0, 2.0, 3.0),
            e0: Point3::new(1.0, 0.0, 0.0),
            e1: Point3::new(0.0, 2.0, 0.0),
            e2: Point3::new(0.0, 0.0, 3.0),
        };
        let p = frame_to_world(f, [0.25, 0.5, 0.75]);
        let q = world_to_frame(f, p).unwrap();
        assert!((q[0] - 0.25).abs() < 1e-12);
        assert!((q[1] - 0.5).abs() < 1e-12);
        assert!((q[2] - 0.75).abs() < 1e-12);
    }

    #[test]
    fn barycentric_sums_to_one() {
        let b = barycentric_tetra(
            Point3::new(0.25, 0.25, 0.25),
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        )
        .unwrap();
        assert!((b.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn slerp_outputs_unit_quaternion() {
        let q = quaternion_slerp(
            Quaternion {
                w: 1.0,
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Quaternion {
                w: 0.0,
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            0.5,
        );
        let n = (q.w * q.w + q.x * q.x + q.y * q.y + q.z * q.z).sqrt();
        assert!((n - 1.0).abs() < 1e-12);
    }
}
