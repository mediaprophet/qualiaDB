//! 3D Spatial Raycasting & Swept Shape-Casting (`query.rs`).
//!
//! Provides exact zero-heap 3D spatial queries:
//! - [`cast_ray`]: analytical ray-sphere, ray-capsule, ray-cuboid/AABB, Möller–Trumbore
//!   ray-triangle (`ray_triangle_moller_trumbore`), and sphere-tracing against `.10d` /
//!   `HolographicNodeGpu` analytical SDFs (`SdfPrimitive`).
//! - [`cast_sphere`]: swept sphere time-of-impact query against scene colliders.
//! - [`cast_capsule`]: swept capsule time-of-impact query (used by `step_character_3d`).
//! - [`overlap_aabb`]: broadphase + narrowphase AABB overlap query.

use qualia_core_db::render::pga::Motor;

use crate::physics3d::broadphase_bvh::{Aabb3D, BroadphaseBvh};
use crate::physics3d::gjk_epa::ColliderShape3D;
use crate::physics3d::sdf_collider::SdfPrimitive;

/// Hit result returned by [`cast_ray`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit3D {
    /// Index of the hit collider.
    pub collider_id: u32,
    /// Distance / time-of-impact along the unit ray direction (`0.0 <= toi <= max_toi`).
    pub toi: f32,
    /// Exact world-space hit position `origin + dir * toi`.
    pub point: [f32; 3],
    /// Outward surface normal at the hit point in world space.
    pub normal: [f32; 3],
}

/// Hit result returned by [`cast_sphere`] and [`cast_capsule`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeCastHit3D {
    /// Index of the hit collider.
    pub collider_id: u32,
    /// Travel distance along `dir` at initial contact (`0.0 <= toi <= max_dist`).
    pub toi: f32,
    /// Contact point in world space.
    pub point: [f32; 3],
    /// Outward surface normal in world space.
    pub normal: [f32; 3],
}

/// Möller–Trumbore ray-triangle intersection test (`HotZeroHeap`).
///
/// Returns `Some((toi, normal))` if the ray `origin + t * dir` hits triangle `(v0, v1, v2)`
/// within `0.0 <= t <= max_toi`.
#[inline]
pub fn ray_triangle_moller_trumbore(
    origin: [f32; 3],
    dir: [f32; 3],
    v0: [f32; 3],
    v1: [f32; 3],
    v2: [f32; 3],
    max_toi: f32,
) -> Option<(f32, [f32; 3])> {
    const EPS: f32 = 1e-7;
    let e1 = sub3(v1, v0);
    let e2 = sub3(v2, v0);
    let h = cross3(dir, e2);
    let a = dot3(e1, h);
    if a.abs() < EPS {
        return None;
    }
    let f = 1.0 / a;
    let s = sub3(origin, v0);
    let u = f * dot3(s, h);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross3(s, e1);
    let v = f * dot3(dir, q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = f * dot3(e2, q);
    if t >= 0.0 && t <= max_toi {
        let mut n = normalize3(cross3(e1, e2));
        if dot3(n, dir) > 0.0 {
            n = [-n[0], -n[1], -n[2]];
        }
        Some((t, n))
    } else {
        None
    }
}

/// Analytical ray-sphere intersection in local space (sphere centered at origin).
#[inline]
pub fn ray_sphere_local(
    origin: [f32; 3],
    dir: [f32; 3],
    radius: f32,
    max_toi: f32,
) -> Option<(f32, [f32; 3])> {
    let b = dot3(origin, dir);
    let c = dot3(origin, origin) - radius * radius;
    if c > 0.0 && b > 0.0 {
        return None;
    }
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = (-b - disc.sqrt()).max(0.0);
    if t > max_toi {
        return None;
    }
    let hit = [
        origin[0] + dir[0] * t,
        origin[1] + dir[1] * t,
        origin[2] + dir[2] * t,
    ];
    Some((t, normalize3(hit)))
}

/// Sphere-tracing raycast against an analytical [`SdfPrimitive`] in local space.
#[inline]
pub fn ray_sdf_local(
    origin: [f32; 3],
    dir: [f32; 3],
    sdf: &SdfPrimitive,
    max_toi: f32,
) -> Option<(f32, [f32; 3])> {
    let mut t = 0.0f32;
    let mut step = 0usize;
    while step < 64 && t <= max_toi {
        let p = [
            origin[0] + dir[0] * t,
            origin[1] + dir[1] * t,
            origin[2] + dir[2] * t,
        ];
        let d = sdf.evaluate_local(p);
        if d <= 1e-4 {
            let n = sdf.gradient_normal_local(p);
            return Some((t, n));
        }
        t += d.max(1e-3);
        step += 1;
    }
    None
}

/// Raycast against a single [`ColliderShape3D`] transformed by `motor`.
pub fn raycast_collider(
    origin_world: [f32; 3],
    dir_world: [f32; 3],
    max_toi: f32,
    shape: &ColliderShape3D,
    motor: &Motor,
) -> Option<(f32, [f32; 3])> {
    let inv = motor.reverse();
    let o_local = inv.transform_point(origin_world);
    let d_local = normalize3(inv.transform_vector(dir_world));

    let local_hit = match *shape {
        ColliderShape3D::Sphere { radius } => ray_sphere_local(o_local, d_local, radius, max_toi),
        ColliderShape3D::Capsule {
            radius,
            half_height,
        } => {
            let sdf = SdfPrimitive::Capsule {
                radius,
                half_height,
            };
            ray_sdf_local(o_local, d_local, &sdf, max_toi)
        }
        ColliderShape3D::Cuboid { half_extents } => {
            let aabb = Aabb3D::from_center_half_extents([0.0; 3], half_extents);
            aabb.ray_intersect(o_local, d_local, max_toi).map(|(t, _)| {
                let hit = [
                    o_local[0] + d_local[0] * t,
                    o_local[1] + d_local[1] * t,
                    o_local[2] + d_local[2] * t,
                ];
                let dx = (hit[0].abs() - half_extents[0]).abs();
                let dy = (hit[1].abs() - half_extents[1]).abs();
                let dz = (hit[2].abs() - half_extents[2]).abs();
                let n = if dx <= dy && dx <= dz {
                    [hit[0].signum(), 0.0, 0.0]
                } else if dy <= dz {
                    [0.0, hit[1].signum(), 0.0]
                } else {
                    [0.0, 0.0, hit[2].signum()]
                };
                (t, n)
            })
        }
        ColliderShape3D::Cylinder {
            radius,
            half_height,
        } => {
            let sdf = SdfPrimitive::RoundedBox {
                half_extents: [radius, half_height, radius],
                bevel_radius: 0.0,
            };
            ray_sdf_local(o_local, d_local, &sdf, max_toi)
        }
        ColliderShape3D::ConvexHull { .. } => {
            let aabb = shape.world_aabb(&Motor::identity());
            aabb.ray_intersect(o_local, d_local, max_toi)
                .map(|(t, _)| (t, [-d_local[0], -d_local[1], -d_local[2]]))
        }
        ColliderShape3D::SdfAnalytical { ref primitive } => {
            ray_sdf_local(o_local, d_local, primitive, max_toi)
        }
    };

    local_hit.map(|(t, n_local)| (t, normalize3(motor.transform_vector(n_local))))
}

/// Casts a 3D ray through a [`BroadphaseBvh`] and collider slice, returning the closest [`RayHit3D`].
pub fn cast_ray<const MAX_LEAVES: usize>(
    bvh: &BroadphaseBvh<MAX_LEAVES>,
    shapes: &[ColliderShape3D],
    motors: &[Motor],
    origin: [f32; 3],
    dir: [f32; 3],
    max_toi: f32,
    ignore_collider: Option<u32>,
) -> Option<RayHit3D> {
    let unit_dir = normalize3(dir);
    let mut candidates = [0u32; 128];
    let cand_count = bvh.query_ray_candidates(origin, unit_dir, max_toi, &mut candidates);

    let mut best_hit: Option<RayHit3D> = None;
    let mut best_toi = max_toi;

    let mut i = 0usize;
    while i < cand_count {
        let cid = candidates[i];
        i += 1;
        if Some(cid) == ignore_collider {
            continue;
        }
        let idx = cid as usize;
        if idx >= shapes.len() || idx >= motors.len() {
            continue;
        }
        if let Some((toi, normal)) =
            raycast_collider(origin, unit_dir, best_toi, &shapes[idx], &motors[idx])
        {
            if toi <= best_toi {
                best_toi = toi;
                best_hit = Some(RayHit3D {
                    collider_id: cid,
                    toi,
                    point: [
                        origin[0] + unit_dir[0] * toi,
                        origin[1] + unit_dir[1] * toi,
                        origin[2] + unit_dir[2] * toi,
                    ],
                    normal,
                });
            }
        }
    }

    best_hit
}

/// Casts a sphere of `radius` along `dir` up to `max_dist` by inflating target primitives by `radius`.
pub fn cast_sphere<const MAX_LEAVES: usize>(
    bvh: &BroadphaseBvh<MAX_LEAVES>,
    shapes: &[ColliderShape3D],
    motors: &[Motor],
    center: [f32; 3],
    radius: f32,
    dir: [f32; 3],
    max_dist: f32,
    ignore_collider: Option<u32>,
) -> Option<ShapeCastHit3D> {
    let unit_dir = normalize3(dir);
    let sweep_aabb = Aabb3D::from_center_half_extents(center, [radius; 3]).union(
        &Aabb3D::from_center_half_extents(
            [
                center[0] + unit_dir[0] * max_dist,
                center[1] + unit_dir[1] * max_dist,
                center[2] + unit_dir[2] * max_dist,
            ],
            [radius; 3],
        ),
    );

    let mut candidates = [0u32; 128];
    let cand_count = bvh.overlap_aabb(&sweep_aabb, &mut candidates);

    let mut best_hit: Option<ShapeCastHit3D> = None;
    let mut best_toi = max_dist;

    let mut i = 0usize;
    while i < cand_count {
        let cid = candidates[i];
        i += 1;
        if Some(cid) == ignore_collider {
            continue;
        }
        let idx = cid as usize;
        if idx >= shapes.len() || idx >= motors.len() {
            continue;
        }

        // Inflate target shape by `radius` (Minkowski sum with sphere) and raycast!
        let inflated = match shapes[idx] {
            ColliderShape3D::Sphere { radius: r } => ColliderShape3D::Sphere {
                radius: r + radius,
            },
            ColliderShape3D::Capsule {
                radius: r,
                half_height,
            } => ColliderShape3D::Capsule {
                radius: r + radius,
                half_height,
            },
            ColliderShape3D::Cuboid { half_extents } => ColliderShape3D::SdfAnalytical {
                primitive: SdfPrimitive::RoundedBox {
                    half_extents,
                    bevel_radius: radius,
                },
            },
            ColliderShape3D::Cylinder {
                radius: r,
                half_height,
            } => ColliderShape3D::Cylinder {
                radius: r + radius,
                half_height: half_height + radius,
            },
            ColliderShape3D::ConvexHull { .. } => {
                let he = shapes[idx].world_aabb(&Motor::identity()).half_extents();
                ColliderShape3D::SdfAnalytical {
                    primitive: SdfPrimitive::RoundedBox {
                        half_extents: he,
                        bevel_radius: radius,
                    },
                }
            }
            ColliderShape3D::SdfAnalytical { primitive } => match primitive {
                SdfPrimitive::Sphere { radius: r } => ColliderShape3D::SdfAnalytical {
                    primitive: SdfPrimitive::Sphere {
                        radius: r + radius,
                    },
                },
                SdfPrimitive::RoundedBox {
                    half_extents,
                    bevel_radius,
                } => ColliderShape3D::SdfAnalytical {
                    primitive: SdfPrimitive::RoundedBox {
                        half_extents,
                        bevel_radius: bevel_radius + radius,
                    },
                },
                SdfPrimitive::Capsule {
                    radius: r,
                    half_height,
                } => ColliderShape3D::SdfAnalytical {
                    primitive: SdfPrimitive::Capsule {
                        radius: r + radius,
                        half_height,
                    },
                },
                SdfPrimitive::Torus {
                    major_radius,
                    minor_radius,
                } => ColliderShape3D::SdfAnalytical {
                    primitive: SdfPrimitive::Torus {
                        major_radius,
                        minor_radius: minor_radius + radius,
                    },
                },
                SdfPrimitive::SmoothUnionPair {
                    center_a,
                    radius_a,
                    center_b,
                    radius_b,
                    blend_radius,
                } => ColliderShape3D::SdfAnalytical {
                    primitive: SdfPrimitive::SmoothUnionPair {
                        center_a,
                        radius_a: radius_a + radius,
                        center_b,
                        radius_b: radius_b + radius,
                        blend_radius,
                    },
                },
            },
        };

        if let Some((toi, normal)) =
            raycast_collider(center, unit_dir, best_toi, &inflated, &motors[idx])
        {
            if toi <= best_toi {
                best_toi = toi;
                let sphere_center_at_hit = [
                    center[0] + unit_dir[0] * toi,
                    center[1] + unit_dir[1] * toi,
                    center[2] + unit_dir[2] * toi,
                ];
                best_hit = Some(ShapeCastHit3D {
                    collider_id: cid,
                    toi,
                    point: [
                        sphere_center_at_hit[0] - normal[0] * radius,
                        sphere_center_at_hit[1] - normal[1] * radius,
                        sphere_center_at_hit[2] - normal[2] * radius,
                    ],
                    normal,
                });
            }
        }
    }

    best_hit
}

/// Casts a vertical 3D capsule (`radius`, `half_height`) along `dir` up to `max_dist`
/// by sweeping 3 axial proxy spheres (bottom hemisphere, center, top hemisphere).
#[allow(clippy::too_many_arguments)]
pub fn cast_capsule<const MAX_LEAVES: usize>(
    bvh: &BroadphaseBvh<MAX_LEAVES>,
    shapes: &[ColliderShape3D],
    motors: &[Motor],
    center: [f32; 3],
    radius: f32,
    half_height: f32,
    dir: [f32; 3],
    max_dist: f32,
    ignore_collider: Option<u32>,
) -> Option<ShapeCastHit3D> {
    let offsets = [-half_height, 0.0, half_height];
    let mut best_hit: Option<ShapeCastHit3D> = None;
    let mut best_toi = max_dist;

    let mut i = 0usize;
    while i < 3 {
        let sample_center = [center[0], center[1] + offsets[i], center[2]];
        if let Some(hit) = cast_sphere(
            bvh,
            shapes,
            motors,
            sample_center,
            radius,
            dir,
            best_toi,
            ignore_collider,
        ) {
            if hit.toi <= best_toi {
                best_toi = hit.toi;
                best_hit = Some(hit);
            }
        }
        i += 1;
    }

    best_hit
}

/// Convenience wrapper returning all collider IDs overlapping `query_aabb`.
#[inline]
pub fn overlap_aabb<const MAX_LEAVES: usize>(
    bvh: &BroadphaseBvh<MAX_LEAVES>,
    query_aabb: &Aabb3D,
    out_collider_ids: &mut [u32],
) -> usize {
    bvh.overlap_aabb(query_aabb, out_collider_ids)
}

#[inline]
fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
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
fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let l = dot3(v, v).sqrt();
    if l < 1e-8 {
        [0.0, 1.0, 0.0]
    } else {
        [v[0] / l, v[1] / l, v[2] / l]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::render::pga::motor_translate;

    #[test]
    fn ray_and_capsule_casts_hit_analytical_sdf_and_triangle() {
        // 1. Möller-Trumbore triangle hit
        let tri_hit = ray_triangle_moller_trumbore(
            [0.2, 5.0, 0.2],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            10.0,
        );
        assert!(tri_hit.is_some());
        assert!((tri_hit.unwrap().0 - 5.0).abs() < 1e-5);

        // 2. BVH raycast & capsule cast against Analytical SDF sphere
        let shapes = [ColliderShape3D::SdfAnalytical {
            primitive: SdfPrimitive::Sphere { radius: 1.0 },
        }];
        let motors = [motor_translate([0.0, 0.0, 5.0])];
        let mut bvh = BroadphaseBvh::<16>::new(0.1);
        bvh.insert(0, shapes[0].world_aabb(&motors[0])).unwrap();

        let r_hit = cast_ray(&bvh, &shapes, &motors, [0.0, 0.0, 0.0], [0.0, 0.0, 1.0], 10.0, None)
            .unwrap();
        assert_eq!(r_hit.collider_id, 0);
        assert!((r_hit.toi - 4.0).abs() < 1e-2);

        let c_hit = cast_capsule(
            &bvh,
            &shapes,
            &motors,
            [0.0, 0.0, 0.0],
            0.5,
            0.5,
            [0.0, 0.0, 1.0],
            10.0,
            None,
        )
        .unwrap();
        assert!((c_hit.toi - 3.5).abs() < 1e-2);
    }
}
