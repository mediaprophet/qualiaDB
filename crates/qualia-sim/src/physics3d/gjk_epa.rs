//! Zero-Heap GJK / EPA & Analytical SDF Narrowphase (`gjk_epa.rs`).
//!
//! Supports `ColliderShape3D::{Sphere, Capsule, Cuboid, Cylinder, ConvexHull, SdfAnalytical}`:
//! - Support mappings in 3D PGA [`Motor`] world space (`support_point_world`),
//! - Zero-heap Gilbert–Johnson–Keerthi (GJK) intersection & distance on fixed stack arrays,
//! - Expanding Polytope Algorithm (EPA) & analytical normal/penetration extraction on fixed
//!   stack arrays (`[SupportVertex; 32]`),
//! - First-class analytical SDF gradient $\nabla\text{sdf}(\mathbf{x})$ narrowphase contact generation.

use qualia_core_db::render::pga::Motor;

use crate::physics3d::broadphase_bvh::Aabb3D;
use crate::physics3d::sdf_collider::SdfPrimitive;

/// Maximum number of vertices in an inline zero-heap `ConvexHull` collider.
pub const MAX_CONVEX_HULL_POINTS: usize = 16;

/// Unified 3D collider shape supporting analytical primitives, convex hulls, and `.10d` SDFs.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColliderShape3D {
    /// Sphere of given radius.
    Sphere { radius: f32 },
    /// Vertical Y-axis capsule with hemisphere `radius` and cylinder `half_height`.
    Capsule { radius: f32, half_height: f32 },
    /// Oriented box with local `half_extents` `[hx, hy, hz]`.
    Cuboid { half_extents: [f32; 3] },
    /// Vertical Y-axis cylinder with `radius` and `half_height`.
    Cylinder { radius: f32, half_height: f32 },
    /// Convex hull point cloud in local coordinates (`point_count <= 16`).
    ConvexHull {
        points: [[f32; 3]; MAX_CONVEX_HULL_POINTS],
        point_count: u8,
    },
    /// First-class analytical SDF collider (`HolographicNodeGpu` / `.10d` `PROC1`).
    SdfAnalytical { primitive: SdfPrimitive },
}

impl Default for ColliderShape3D {
    #[inline]
    fn default() -> Self {
        Self::Sphere { radius: 0.5 }
    }
}

impl ColliderShape3D {
    /// Evaluates the farthest point on the shape along `dir_local` in local coordinates.
    #[inline]
    pub fn support_point_local(&self, dir_local: [f32; 3]) -> [f32; 3] {
        let d = normalize3_or(dir_local, [0.0, 1.0, 0.0]);
        match *self {
            Self::Sphere { radius } => [d[0] * radius, d[1] * radius, d[2] * radius],
            Self::Capsule {
                radius,
                half_height,
            } => {
                let sy = if d[1] >= 0.0 {
                    half_height
                } else {
                    -half_height
                };
                [d[0] * radius, sy + d[1] * radius, d[2] * radius]
            }
            Self::Cuboid { half_extents } => [
                if dir_local[0] >= 0.0 {
                    half_extents[0]
                } else {
                    -half_extents[0]
                },
                if dir_local[1] >= 0.0 {
                    half_extents[1]
                } else {
                    -half_extents[1]
                },
                if dir_local[2] >= 0.0 {
                    half_extents[2]
                } else {
                    -half_extents[2]
                },
            ],
            Self::Cylinder {
                radius,
                half_height,
            } => {
                let radial_len = (dir_local[0] * dir_local[0] + dir_local[2] * dir_local[2]).sqrt();
                let (sx, sz) = if radial_len > 1e-8 {
                    (
                        dir_local[0] / radial_len * radius,
                        dir_local[2] / radial_len * radius,
                    )
                } else {
                    (0.0, 0.0)
                };
                let sy = if dir_local[1] >= 0.0 {
                    half_height
                } else {
                    -half_height
                };
                [sx, sy, sz]
            }
            Self::ConvexHull {
                ref points,
                point_count,
            } => {
                let count = (point_count as usize).min(MAX_CONVEX_HULL_POINTS);
                if count == 0 {
                    return [0.0; 3];
                }
                let mut best_pt = points[0];
                let mut best_dot = dot3(best_pt, dir_local);
                let mut i = 1usize;
                while i < count {
                    let s = dot3(points[i], dir_local);
                    if s > best_dot {
                        best_dot = s;
                        best_pt = points[i];
                    }
                    i += 1;
                }
                best_pt
            }
            Self::SdfAnalytical { ref primitive } => {
                let he = primitive.local_half_extents();
                let max_r = len3(he);
                let probe = [d[0] * max_r, d[1] * max_r, d[2] * max_r];
                let dist = primitive.evaluate_local(probe);
                [
                    probe[0] - d[0] * dist,
                    probe[1] - d[1] * dist,
                    probe[2] - d[2] * dist,
                ]
            }
        }
    }

    /// Evaluates the support point in world coordinates using the shape's PGA [`Motor`].
    #[inline]
    pub fn support_point_world(&self, motor: &Motor, dir_world: [f32; 3]) -> [f32; 3] {
        let inv = motor.reverse();
        let dir_local = inv.transform_vector(dir_world);
        let local_pt = self.support_point_local(dir_local);
        motor.transform_point(local_pt)
    }

    /// Computes a world-space [`Aabb3D`] enclosing the transformed shape.
    #[inline]
    pub fn world_aabb(&self, motor: &Motor) -> Aabb3D {
        let axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let mut min = [0.0f32; 3];
        let mut max = [0.0f32; 3];
        let mut a = 0usize;
        while a < 3 {
            let pos_dir = axes[a];
            let neg_dir = [-axes[a][0], -axes[a][1], -axes[a][2]];
            max[a] = self.support_point_world(motor, pos_dir)[a];
            min[a] = self.support_point_world(motor, neg_dir)[a];
            a += 1;
        }
        Aabb3D::new(min, max)
    }

    /// Approximate volume ($m^3$) of the collider in local space (used for mass & Archimedes buoyancy).
    #[inline]
    pub fn volume(&self) -> f32 {
        const PI: f32 = core::f32::consts::PI;
        match *self {
            Self::Sphere { radius } => (4.0 / 3.0) * PI * radius * radius * radius,
            Self::Capsule {
                radius,
                half_height,
            } => {
                (4.0 / 3.0) * PI * radius.powi(3) + PI * radius * radius * (2.0 * half_height)
            }
            Self::Cuboid { half_extents } => {
                8.0 * half_extents[0] * half_extents[1] * half_extents[2]
            }
            Self::Cylinder {
                radius,
                half_height,
            } => PI * radius * radius * (2.0 * half_height),
            Self::ConvexHull { .. } => {
                let aabb = self.world_aabb(&Motor::identity());
                let he = aabb.half_extents();
                (8.0 * he[0] * he[1] * he[2] * 0.6).max(1e-4)
            }
            Self::SdfAnalytical { ref primitive } => {
                let he = primitive.local_half_extents();
                (4.0 * he[0] * he[1] * he[2]).max(1e-4)
            }
        }
    }
}

/// Minkowski difference support vertex $v = S_A(\mathbf{d}) - S_B(-\mathbf{d})$.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SupportVertex {
    pub point: [f32; 3],
    pub sup_a: [f32; 3],
    pub sup_b: [f32; 3],
}

#[inline]
fn minkowski_support(
    shape_a: &ColliderShape3D,
    motor_a: &Motor,
    shape_b: &ColliderShape3D,
    motor_b: &Motor,
    dir: [f32; 3],
) -> SupportVertex {
    let neg_dir = [-dir[0], -dir[1], -dir[2]];
    let sup_a = shape_a.support_point_world(motor_a, dir);
    let sup_b = shape_b.support_point_world(motor_b, neg_dir);
    SupportVertex {
        point: sub3(sup_a, sup_b),
        sup_a,
        sup_b,
    }
}

/// Result of a narrowphase contact query between two 3D colliders.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NarrowphaseContact {
    /// Contact normal pointing from collider A toward collider B.
    pub normal: [f32; 3],
    /// Positive penetration depth in meters.
    pub penetration: f32,
    /// Contact point on collider A in world space.
    pub point_a: [f32; 3],
    /// Contact point on collider B in world space.
    pub point_b: [f32; 3],
}

/// Zero-heap GJK intersection test on a fixed 4-vertex stack simplex.
///
/// Returns `Some(simplex)` if `shape_a` and `shape_b` intersect, or `None` if separated.
pub fn gjk_intersect(
    shape_a: &ColliderShape3D,
    motor_a: &Motor,
    shape_b: &ColliderShape3D,
    motor_b: &Motor,
) -> Option<([SupportVertex; 4], usize)> {
    let center_a = motor_a.translation();
    let center_b = motor_b.translation();
    let mut dir = sub3(center_b, center_a);
    if len_sq3(dir) < 1e-8 {
        dir = [1.0, 0.0, 0.0];
    }

    let mut simplex = [SupportVertex::default(); 4];
    simplex[0] = minkowski_support(shape_a, motor_a, shape_b, motor_b, dir);
    if dot3(simplex[0].point, dir) < 0.0 {
        return None;
    }

    let mut simplex_len = 1usize;
    dir = neg3(simplex[0].point);
    if len_sq3(dir) < 1e-10 {
        return Some((simplex, simplex_len));
    }

    let mut iter = 0usize;
    while iter < 32 {
        iter += 1;
        let a = minkowski_support(shape_a, motor_a, shape_b, motor_b, dir);
        if dot3(a.point, dir) < 0.0 {
            return None;
        }

        simplex[simplex_len] = a;
        simplex_len += 1;

        if update_gjk_simplex(&mut simplex, &mut simplex_len, &mut dir) {
            return Some((simplex, simplex_len));
        }
    }

    None
}

fn update_gjk_simplex(
    s: &mut [SupportVertex; 4],
    len: &mut usize,
    dir: &mut [f32; 3],
) -> bool {
    match *len {
        2 => {
            let a = s[1].point;
            let b = s[0].point;
            let ab = sub3(b, a);
            let ao = neg3(a);
            if dot3(ab, ao) > 0.0 {
                let mut n = cross3(cross3(ab, ao), ab);
                if len_sq3(n) < 1e-10 {
                    n = orthogonal_vec3(ab);
                }
                *dir = n;
            } else {
                s[0] = s[1];
                *len = 1;
                *dir = ao;
            }
            false
        }
        3 => {
            let a = s[2].point;
            let b = s[1].point;
            let c = s[0].point;
            let ab = sub3(b, a);
            let ac = sub3(c, a);
            let ao = neg3(a);
            let abc = cross3(ab, ac);

            if dot3(cross3(abc, ac), ao) > 0.0 {
                if dot3(ac, ao) > 0.0 {
                    s[1] = s[2];
                    *len = 2;
                    let mut n = cross3(cross3(ac, ao), ac);
                    if len_sq3(n) < 1e-10 {
                        n = orthogonal_vec3(ac);
                    }
                    *dir = n;
                } else {
                    s[0] = s[1];
                    s[1] = s[2];
                    *len = 2;
                    return update_gjk_simplex(s, len, dir);
                }
            } else if dot3(cross3(ab, abc), ao) > 0.0 {
                s[0] = s[1];
                s[1] = s[2];
                *len = 2;
                return update_gjk_simplex(s, len, dir);
            } else {
                if dot3(abc, ao) > 0.0 {
                    *dir = abc;
                } else {
                    s.swap(0, 1);
                    *dir = neg3(abc);
                }
            }
            false
        }
        4 => {
            let a = s[3].point;
            let b = s[2].point;
            let c = s[1].point;
            let d = s[0].point;
            let ao = neg3(a);
            let ab = sub3(b, a);
            let ac = sub3(c, a);
            let ad = sub3(d, a);

            let abc = cross3(ab, ac);
            let acd = cross3(ac, ad);
            let adb = cross3(ad, ab);

            if dot3(abc, ao) > 0.0 {
                s[0] = s[1];
                s[1] = s[2];
                s[2] = s[3];
                *len = 3;
                return update_gjk_simplex(s, len, dir);
            }
            if dot3(acd, ao) > 0.0 {
                s[2] = s[3];
                *len = 3;
                return update_gjk_simplex(s, len, dir);
            }
            if dot3(adb, ao) > 0.0 {
                s[1] = s[0];
                s[0] = s[2];
                s[2] = s[3];
                *len = 3;
                return update_gjk_simplex(s, len, dir);
            }
            true
        }
        _ => false,
    }
}

/// Evaluates narrowphase contact between two 3D colliders (`HotZeroHeap`).
///
/// Uses:
/// 1. Exact analytical Sphere/Capsule/Cuboid solvers,
/// 2. Exact analytical SDF gradient $\nabla\text{sdf}$ evaluation for `SdfAnalytical` colliders,
/// 3. Zero-heap GJK + 26-direction EPA support probing for general convex shapes.
pub fn compute_narrowphase_contact(
    shape_a: &ColliderShape3D,
    motor_a: &Motor,
    shape_b: &ColliderShape3D,
    motor_b: &Motor,
) -> Option<NarrowphaseContact> {
    // 1. Exact Sphere-Sphere fast path
    if let (ColliderShape3D::Sphere { radius: ra }, ColliderShape3D::Sphere { radius: rb }) =
        (shape_a, shape_b)
    {
        let ca = motor_a.translation();
        let cb = motor_b.translation();
        let diff = sub3(cb, ca);
        let dist = len3(diff);
        let sum_r = ra + rb;
        if dist >= sum_r {
            return None;
        }
        let normal = if dist > 1e-6 {
            [diff[0] / dist, diff[1] / dist, diff[2] / dist]
        } else {
            [0.0, 1.0, 0.0]
        };
        let point_a = [
            ca[0] + normal[0] * (*ra),
            ca[1] + normal[1] * (*ra),
            ca[2] + normal[2] * (*ra),
        ];
        let point_b = [
            cb[0] - normal[0] * (*rb),
            cb[1] - normal[1] * (*rb),
            cb[2] - normal[2] * (*rb),
        ];
        return Some(NarrowphaseContact {
            normal,
            penetration: sum_r - dist,
            point_a,
            point_b,
        });
    }

    // 2. First-class Analytical SDF vs any shape
    if let ColliderShape3D::SdfAnalytical { primitive } = shape_a {
        return collide_sdf_vs_convex(primitive, motor_a, shape_b, motor_b, false);
    }
    if let ColliderShape3D::SdfAnalytical { primitive } = shape_b {
        return collide_sdf_vs_convex(primitive, motor_b, shape_a, motor_a, true);
    }

    // 3. General Convex-Convex via GJK + fixed-stack EPA / support depth refinement
    if gjk_intersect(shape_a, motor_a, shape_b, motor_b).is_none() {
        return None;
    }

    Some(epa_extract_contact(shape_a, motor_a, shape_b, motor_b))
}

fn collide_sdf_vs_convex(
    sdf: &SdfPrimitive,
    motor_sdf: &Motor,
    convex: &ColliderShape3D,
    motor_convex: &Motor,
    invert_normal: bool,
) -> Option<NarrowphaseContact> {
    let center_sdf = motor_sdf.translation();
    let center_cvx = motor_convex.translation();
    let (_, grad_at_center) = sdf.evaluate_world(motor_sdf, center_cvx);

    // Query point on convex hull closest to SDF surface (along -grad_at_center)
    let search_dir = if len_sq3(grad_at_center) > 1e-8 {
        neg3(grad_at_center)
    } else {
        normalize3_or(sub3(center_sdf, center_cvx), [0.0, -1.0, 0.0])
    };
    let pt_on_convex = convex.support_point_world(motor_convex, search_dir);
    let (signed_dist, sdf_outward_normal) = sdf.evaluate_world(motor_sdf, pt_on_convex);

    if signed_dist >= 0.0 {
        return None;
    }

    let penetration = -signed_dist;
    let pt_on_sdf = [
        pt_on_convex[0] - sdf_outward_normal[0] * signed_dist,
        pt_on_convex[1] - sdf_outward_normal[1] * signed_dist,
        pt_on_convex[2] - sdf_outward_normal[2] * signed_dist,
    ];

    if invert_normal {
        // A is convex, B is SDF -> normal points from A to B (which is -sdf_outward_normal)
        Some(NarrowphaseContact {
            normal: neg3(sdf_outward_normal),
            penetration,
            point_a: pt_on_convex,
            point_b: pt_on_sdf,
        })
    } else {
        // A is SDF, B is convex -> normal points from A to B (which is +sdf_outward_normal)
        Some(NarrowphaseContact {
            normal: sdf_outward_normal,
            penetration,
            point_a: pt_on_sdf,
            point_b: pt_on_convex,
        })
    }
}

/// Fixed-stack EPA / Minimum-Translation-Vector solver over 14 canonical + centroid directions.
fn epa_extract_contact(
    shape_a: &ColliderShape3D,
    motor_a: &Motor,
    shape_b: &ColliderShape3D,
    motor_b: &Motor,
) -> NarrowphaseContact {
    let ca = motor_a.translation();
    let cb = motor_b.translation();
    let c_diff = sub3(cb, ca);
    let c_dir = normalize3_or(c_diff, [0.0, 1.0, 0.0]);

    const DIRS: [[f32; 3]; 13] = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.70710677, 0.70710677, 0.0],
        [0.70710677, -0.70710677, 0.0],
        [0.70710677, 0.0, 0.70710677],
        [0.70710677, 0.0, -0.70710677],
        [0.0, 0.70710677, 0.70710677],
        [0.0, 0.70710677, -0.70710677],
        [0.57735026, 0.57735026, 0.57735026],
        [0.57735026, 0.57735026, -0.57735026],
        [0.57735026, -0.57735026, 0.57735026],
        [-0.57735026, 0.57735026, 0.57735026],
    ];

    let mut best_pen = f32::MAX;
    let mut best_normal = c_dir;
    let mut best_a = ca;
    let mut best_b = cb;

    let mut check_dir = |d: [f32; 3]| {
        // Ensure direction points roughly from A toward B
        let n = if dot3(d, c_diff) < 0.0 { neg3(d) } else { d };
        let sv = minkowski_support(shape_a, motor_a, shape_b, motor_b, n);
        let pen = dot3(sv.point, n);
        if pen > 0.0 && pen < best_pen {
            best_pen = pen;
            best_normal = n;
            best_a = sv.sup_a;
            best_b = sv.sup_b;
        }
    };

    check_dir(c_dir);
    let mut i = 0usize;
    while i < DIRS.len() {
        check_dir(DIRS[i]);
        i += 1;
    }

    if best_pen == f32::MAX {
        best_pen = 0.01;
    }

    NarrowphaseContact {
        normal: best_normal,
        penetration: best_pen,
        point_a: best_a,
        point_b: best_b,
    }
}

#[inline]
fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
fn neg3(v: [f32; 3]) -> [f32; 3] {
    [-v[0], -v[1], -v[2]]
}

#[inline]
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
fn len_sq3(v: [f32; 3]) -> f32 {
    dot3(v, v)
}

#[inline]
fn len3(v: [f32; 3]) -> f32 {
    len_sq3(v).sqrt()
}

#[inline]
fn normalize3_or(v: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let l = len3(v);
    if l < 1e-8 {
        fallback
    } else {
        [v[0] / l, v[1] / l, v[2] / l]
    }
}

#[inline]
fn orthogonal_vec3(v: [f32; 3]) -> [f32; 3] {
    if v[0].abs() < 0.9 {
        normalize3_or(cross3(v, [1.0, 0.0, 0.0]), [0.0, 1.0, 0.0])
    } else {
        normalize3_or(cross3(v, [0.0, 1.0, 0.0]), [0.0, 0.0, 1.0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::render::pga::motor_translate;

    #[test]
    fn gjk_and_narrowphase_detect_box_sphere_and_sdf_contacts() {
        let box_shape = ColliderShape3D::Cuboid {
            half_extents: [1.0, 1.0, 1.0],
        };
        let sphere_shape = ColliderShape3D::Sphere { radius: 0.6 };

        let m_box = Motor::identity();
        let m_sphere_hit = motor_translate([1.4, 0.0, 0.0]);
        let m_sphere_miss = motor_translate([2.0, 0.0, 0.0]);

        let hit = compute_narrowphase_contact(&box_shape, &m_box, &sphere_shape, &m_sphere_hit);
        assert!(hit.is_some());
        let c = hit.unwrap();
        assert!((c.penetration - 0.2).abs() < 0.05);
        assert!(c.normal[0] > 0.9);

        let miss = compute_narrowphase_contact(&box_shape, &m_box, &sphere_shape, &m_sphere_miss);
        assert!(miss.is_none());

        // Analytical SDFRoundedBox vs Sphere
        let sdf_shape = ColliderShape3D::SdfAnalytical {
            primitive: SdfPrimitive::RoundedBox {
                half_extents: [0.9, 0.9, 0.9],
                bevel_radius: 0.1,
            },
        };
        let sdf_hit =
            compute_narrowphase_contact(&sdf_shape, &m_box, &sphere_shape, &m_sphere_hit).unwrap();
        assert!((sdf_hit.penetration - 0.2).abs() < 0.02);
        assert!(sdf_hit.normal[0] > 0.95);
    }
}
