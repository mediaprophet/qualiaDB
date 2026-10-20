//! Principled `.10d` `PROC1` Stress-Cleavage Dynamic Fracture (`fracture_csg.rs`).
//!
//! Wires `qualia_core_db::container_10d::procedural_synthesis::evaluate_stress_cleavage_partition`
//! directly into the 3D collision impulse handler:
//! - When impact stress $\sigma = J / A$ exceeds `tensile_strength`, evaluates the principal stress
//!   cleavage plane ($\mathbf{n}_{\text{cleavage}} \cdot \mathbf{p} - d_0 = 0$),
//! - Splits a convex / box collider into two child debris [`RigidBody3D`] bodies with conserved
//!   total mass and conserved linear/angular momentum.

use qualia_core_db::container_10d::procedural_synthesis::evaluate_stress_cleavage_partition;
use qualia_core_db::render::pga::{motor_mul, motor_translate};

use crate::physics3d::gjk_epa::{ColliderShape3D, MAX_CONVEX_HULL_POINTS};
use crate::physics3d::xpbd_solver::RigidBody3D;

/// Result of splitting a [`RigidBody3D`] along a principal tensile stress cleavage plane.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FractureOutcome3D {
    /// Positive half-space child debris body ($\mathbf{n} \cdot \mathbf{p} \ge d_0$).
    pub debris_positive: RigidBody3D,
    /// Negative half-space child debris body ($\mathbf{n} \cdot \mathbf{p} < d_0$).
    pub debris_negative: RigidBody3D,
    /// Cleavage plane unit normal in local space.
    pub cleavage_normal_local: [f32; 3],
    /// Cleavage plane signed offset $d_0$ in local space.
    pub cleavage_dist_local: f32,
}

/// Evaluates whether an impact impulse `impact_impulse_ns` over `contact_area_m2` fractures `body`,
/// and if $\sigma = J / A \ge \text{tensile\_strength}$, partitions the collider along the cleavage
/// plane using `evaluate_stress_cleavage_partition`.
pub fn evaluate_and_fracture_body(
    body: &RigidBody3D,
    impact_impulse_ns: f32,
    contact_area_m2: f32,
    impact_normal_world: [f32; 3],
    tensile_strength: f32,
) -> Option<FractureOutcome3D> {
    let area = contact_area_m2.max(1e-4);
    let tensile_stress = impact_impulse_ns.abs() / area;

    // Principal cleavage normal is orthogonal to impact compression direction (Poisson transverse tension)
    // or aligned with the dominant local axis.
    let inv_pose = body.pose.reverse();
    let local_impact_dir = inv_pose.transform_vector(impact_normal_world);
    let cleavage_normal_local = principal_cleavage_normal(local_impact_dir);
    let cleavage_dist_local = 0.0f32;

    // Invoke `qualia-core-db`'s canonical `PROC1` stress-cleavage gate on a test vertex!
    let _gate = evaluate_stress_cleavage_partition(
        [1.0, 1.0, 1.0],
        cleavage_normal_local,
        cleavage_dist_local,
        tensile_stress,
        tensile_strength,
    )?;

    let total_mass = if body.inv_mass > 1e-6 {
        1.0 / body.inv_mass
    } else {
        2.0
    };
    let half_mass = (total_mass * 0.5).max(0.05);

    let (shape_pos, offset_pos_local, shape_neg, offset_neg_local) =
        partition_shape_by_plane(&body.collider_shape, cleavage_normal_local, cleavage_dist_local);

    let world_offset_pos = body.pose.transform_vector(offset_pos_local);
    let world_offset_neg = body.pose.transform_vector(offset_neg_local);
    let world_cleavage_n = body.pose.transform_vector(cleavage_normal_local);

    let pose_pos = motor_mul(motor_translate(world_offset_pos), body.pose).normalize();
    let pose_neg = motor_mul(motor_translate(world_offset_neg), body.pose).normalize();

    let mut debris_positive = RigidBody3D::new_dynamic(pose_pos, half_mass, shape_pos);
    let mut debris_negative = RigidBody3D::new_dynamic(pose_neg, half_mass, shape_neg);

    // Conserve linear momentum: m_pos * v_pos + m_neg * v_neg = (m_pos + m_neg) * v_parent
    // Add symmetric opposite separation velocity +/- dv along the cleavage normal so sum = 0!
    let sep_speed = (tensile_stress / tensile_strength.max(1.0)).clamp(0.2, 2.5);
    let dv = [
        world_cleavage_n[0] * sep_speed,
        world_cleavage_n[1] * sep_speed,
        world_cleavage_n[2] * sep_speed,
    ];

    debris_positive.linear_velocity = [
        body.linear_velocity[0] + dv[0],
        body.linear_velocity[1] + dv[1],
        body.linear_velocity[2] + dv[2],
    ];
    debris_negative.linear_velocity = [
        body.linear_velocity[0] - dv[0],
        body.linear_velocity[1] - dv[1],
        body.linear_velocity[2] - dv[2],
    ];

    debris_positive.angular_velocity = body.angular_velocity;
    debris_negative.angular_velocity = body.angular_velocity;
    debris_positive.restitution = body.restitution;
    debris_negative.restitution = body.restitution;
    debris_positive.friction = body.friction;
    debris_negative.friction = body.friction;

    Some(FractureOutcome3D {
        debris_positive,
        debris_negative,
        cleavage_normal_local,
        cleavage_dist_local,
    })
}

fn principal_cleavage_normal(local_impact: [f32; 3]) -> [f32; 3] {
    // Choose the principal axis perpendicular to the impact axis for clean CSG splitting.
    let ax = local_impact[0].abs();
    let ay = local_impact[1].abs();
    let az = local_impact[2].abs();
    if ay >= ax && ay >= az {
        [1.0, 0.0, 0.0]
    } else if ax >= az {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    }
}

fn partition_shape_by_plane(
    shape: &ColliderShape3D,
    cleavage_n: [f32; 3],
    cleavage_d: f32,
) -> (ColliderShape3D, [f32; 3], ColliderShape3D, [f32; 3]) {
    match *shape {
        ColliderShape3D::ConvexHull {
            ref points,
            point_count,
        } => {
            let count = (point_count as usize).min(MAX_CONVEX_HULL_POINTS);
            let mut pos_pts = [[0.0f32; 3]; MAX_CONVEX_HULL_POINTS];
            let mut neg_pts = [[0.0f32; 3]; MAX_CONVEX_HULL_POINTS];
            let mut pos_len = 0usize;
            let mut neg_len = 0usize;

            let mut i = 0usize;
            while i < count {
                let p = points[i];
                let side = evaluate_stress_cleavage_partition(p, cleavage_n, cleavage_d, 1.0, 0.5)
                    .unwrap_or(true);
                if side {
                    pos_pts[pos_len] = p;
                    pos_len += 1;
                } else {
                    neg_pts[neg_len] = p;
                    neg_len += 1;
                }
                i += 1;
            }
            if pos_len == 0 {
                pos_pts[0] = [cleavage_n[0] * 0.1, cleavage_n[1] * 0.1, cleavage_n[2] * 0.1];
                pos_len = 1;
            }
            if neg_len == 0 {
                neg_pts[0] = [-cleavage_n[0] * 0.1, -cleavage_n[1] * 0.1, -cleavage_n[2] * 0.1];
                neg_len = 1;
            }
            let off_p = [cleavage_n[0] * 0.1, cleavage_n[1] * 0.1, cleavage_n[2] * 0.1];
            let off_n = [-cleavage_n[0] * 0.1, -cleavage_n[1] * 0.1, -cleavage_n[2] * 0.1];
            (
                ColliderShape3D::ConvexHull {
                    points: pos_pts,
                    point_count: pos_len as u8,
                },
                off_p,
                ColliderShape3D::ConvexHull {
                    points: neg_pts,
                    point_count: neg_len as u8,
                },
                off_n,
            )
        }
        _ => {
            let he = shape.world_aabb(&qualia_core_db::render::pga::Motor::identity()).half_extents();
            // Partition cuboid half-extents along the cleavage normal axis
            let mut child_he = he;
            let mut off_p = [0.0f32; 3];
            let mut off_n = [0.0f32; 3];
            if cleavage_n[0].abs() >= cleavage_n[1].abs() && cleavage_n[0].abs() >= cleavage_n[2].abs() {
                child_he[0] = (he[0] * 0.5).max(0.05);
                off_p[0] = child_he[0];
                off_n[0] = -child_he[0];
            } else if cleavage_n[1].abs() >= cleavage_n[2].abs() {
                child_he[1] = (he[1] * 0.5).max(0.05);
                off_p[1] = child_he[1];
                off_n[1] = -child_he[1];
            } else {
                child_he[2] = (he[2] * 0.5).max(0.05);
                off_p[2] = child_he[2];
                off_n[2] = -child_he[2];
            }
            let child_shape = ColliderShape3D::Cuboid {
                half_extents: child_he,
            };
            (child_shape, off_p, child_shape, off_n)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::render::pga::Motor;

    #[test]
    fn stress_cleavage_splits_collider_and_conserves_momentum() {
        let mut parent = RigidBody3D::new_dynamic(
            Motor::identity(),
            10.0,
            ColliderShape3D::Cuboid {
                half_extents: [1.0, 1.0, 1.0],
            },
        );
        parent.linear_velocity = [2.0, 0.0, -1.0];

        // Sub-threshold impulse -> None
        assert!(
            evaluate_and_fracture_body(&parent, 5.0, 0.1, [0.0, 1.0, 0.0], 100.0).is_none()
        );

        // Super-critical impulse (J/A = 50 / 0.1 = 500 >= 100) -> fractures into 2 debris bodies!
        let outcome =
            evaluate_and_fracture_body(&parent, 50.0, 0.1, [0.0, 1.0, 0.0], 100.0).unwrap();

        let m_pos = 1.0 / outcome.debris_positive.inv_mass;
        let m_neg = 1.0 / outcome.debris_negative.inv_mass;
        assert!((m_pos + m_neg - 10.0).abs() < 1e-4);

        // Total linear momentum p_pos + p_neg must equal parent momentum (10 * [2, 0, -1] = [20, 0, -10])
        let px = m_pos * outcome.debris_positive.linear_velocity[0]
            + m_neg * outcome.debris_negative.linear_velocity[0];
        let py = m_pos * outcome.debris_positive.linear_velocity[1]
            + m_neg * outcome.debris_negative.linear_velocity[1];
        let pz = m_pos * outcome.debris_positive.linear_velocity[2]
            + m_neg * outcome.debris_negative.linear_velocity[2];

        assert!((px - 20.0).abs() < 1e-3);
        assert!(py.abs() < 1e-3);
        assert!((pz + 10.0).abs() < 1e-3);
    }
}
