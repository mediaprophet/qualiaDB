//! Bounded, zero-heap vegetation instance pool and dynamic collision bending.
//!
//! Evaluates collider displacement and wind response on vegetation instances,
//! writing directly into caller-provided `GpuInstanceRecord` slices for mesh
//! instancing without heap allocation.

use crate::render::instance_culling::GpuInstanceRecord;

/// Maximum number of dynamic colliders evaluated against vegetation per step.
pub const MAX_VEGETATION_COLLIDERS: usize = 16;

/// A dynamic physical collider (character, vehicle, projectile) interacting with foliage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VegetationCollider {
    pub position: [f32; 3],
    pub radius: f32,
    pub velocity: [f32; 3],
}

impl Default for VegetationCollider {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            radius: 0.5,
            velocity: [0.0; 3],
        }
    }
}

/// Physical parameters controlling vegetation stiffness, recovery, and wind reaction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VegetationBendingParams {
    /// Elastic resistance to bending (0.0 = completely limp, 1.0 = rigid).
    pub stiffness: f32,
    /// Recovery rate towards rest position per second (e.g. 5.0 to 10.0).
    pub recovery_rate: f32,
    /// Maximum bend angle in radians (e.g. 1.2 rad ~ 70 deg).
    pub max_bend_angle: f32,
    /// Authoritative horizontal wind direction vector [x, z].
    pub wind_direction: [f32; 2],
    /// Wind strength multiplier.
    pub wind_strength: f32,
}

impl Default for VegetationBendingParams {
    fn default() -> Self {
        Self {
            stiffness: 0.5,
            recovery_rate: 6.0,
            max_bend_angle: 1.2,
            wind_direction: [1.0, 0.0],
            wind_strength: 0.2,
        }
    }
}

/// Persistent state for a single vegetation instance in the pool.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VegetationInstanceState {
    pub base_transform: [[f32; 4]; 4],
    pub semantic_id: u64,
    pub height: f32,
    pub current_bend_offset: [f32; 2],
    pub current_bend_velocity: [f32; 2],
}

impl VegetationInstanceState {
    pub fn new(base_transform: [[f32; 4]; 4], height: f32, semantic_id: u64) -> Self {
        Self {
            base_transform,
            semantic_id,
            height: height.max(0.1),
            current_bend_offset: [0.0, 0.0],
            current_bend_velocity: [0.0, 0.0],
        }
    }
}

/// Zero-heap evaluator for dynamic vegetation instances.
pub struct VegetationEvaluator;

impl VegetationEvaluator {
    /// Update vegetation instances against dynamic colliders and wind, writing the resulting
    /// deformed instances into caller-supplied `out_records`.
    ///
    /// Guaranteed zero heap allocation in this hot path.
    pub fn update_instances(
        instances: &mut [VegetationInstanceState],
        colliders: &[VegetationCollider],
        params: &VegetationBendingParams,
        dt: f32,
        time: f32,
        out_records: &mut [GpuInstanceRecord],
    ) -> usize {
        let count = instances.len().min(out_records.len());
        let dt = dt.clamp(0.0, 0.1);

        for (i, instance) in instances.iter_mut().take(count).enumerate() {
            let inst_x = instance.base_transform[3][0];
            let inst_y = instance.base_transform[3][1];
            let inst_z = instance.base_transform[3][2];

            let mut target_bend_x = 0.0f32;
            let mut target_bend_z = 0.0f32;

            // 1. Collider interaction: Push away from colliders within range.
            for collider in colliders.iter().take(MAX_VEGETATION_COLLIDERS) {
                let dx = inst_x - collider.position[0];
                let dy = inst_y - collider.position[1];
                let dz = inst_z - collider.position[2];

                // Check vertical reach
                if dy < -instance.height || dy > collider.radius * 2.0 {
                    continue;
                }

                let horiz_dist_sq = dx * dx + dz * dz;
                let effective_radius = collider.radius + 0.3;
                let eff_rad_sq = effective_radius * effective_radius;

                if horiz_dist_sq < eff_rad_sq && horiz_dist_sq > 1e-6 {
                    let horiz_dist = horiz_dist_sq.sqrt();
                    let overlap = (effective_radius - horiz_dist) / effective_radius;
                    let push_magnitude = overlap * (1.0 - params.stiffness).max(0.1);

                    let dir_x = dx / horiz_dist;
                    let dir_z = dz / horiz_dist;

                    target_bend_x += dir_x * push_magnitude * params.max_bend_angle;
                    target_bend_z += dir_z * push_magnitude * params.max_bend_angle;
                }
            }

            // 2. Wind contribution
            let wind_oscillation = (time * 3.0 + inst_x * 0.5 + inst_z * 0.5).sin();
            let wind_force = params.wind_strength * (0.8 + 0.2 * wind_oscillation);
            target_bend_x += params.wind_direction[0] * wind_force;
            target_bend_z += params.wind_direction[1] * wind_force;

            // 3. Spring-damper integration towards target bend
            let spring_k = params.recovery_rate * params.recovery_rate;
            let damping = 2.0 * params.recovery_rate;

            let force_x = spring_k * (target_bend_x - instance.current_bend_offset[0])
                - damping * instance.current_bend_velocity[0];
            let force_z = spring_k * (target_bend_z - instance.current_bend_offset[1])
                - damping * instance.current_bend_velocity[1];

            instance.current_bend_velocity[0] += force_x * dt;
            instance.current_bend_velocity[1] += force_z * dt;
            instance.current_bend_offset[0] += instance.current_bend_velocity[0] * dt;
            instance.current_bend_offset[1] += instance.current_bend_velocity[1] * dt;

            // Clamp total bend angle
            let bend_sq = instance.current_bend_offset[0] * instance.current_bend_offset[0]
                + instance.current_bend_offset[1] * instance.current_bend_offset[1];
            if bend_sq > params.max_bend_angle * params.max_bend_angle {
                let bend = bend_sq.sqrt();
                instance.current_bend_offset[0] =
                    (instance.current_bend_offset[0] / bend) * params.max_bend_angle;
                instance.current_bend_offset[1] =
                    (instance.current_bend_offset[1] / bend) * params.max_bend_angle;
                instance.current_bend_velocity = [0.0, 0.0];
            }

            // 4. Construct deformed world_from_local matrix:
            // Apply shear/rotation in Y direction according to bend offsets
            let mut world_mat = instance.base_transform;
            world_mat[1][0] += instance.current_bend_offset[0];
            world_mat[1][2] += instance.current_bend_offset[1];

            out_records[i] = GpuInstanceRecord::new(world_mat, instance.semantic_id);
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_transform(x: f32, y: f32, z: f32) -> [[f32; 4]; 4] {
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [x, y, z, 1.0],
        ]
    }

    #[test]
    fn vegetation_stays_undisplaced_without_colliders_and_wind() {
        let mut instances = [VegetationInstanceState::new(identity_transform(0.0, 0.0, 0.0), 1.0, 42)];
        let colliders = [];
        let params = VegetationBendingParams {
            stiffness: 0.5,
            recovery_rate: 6.0,
            max_bend_angle: 1.0,
            wind_direction: [0.0, 0.0],
            wind_strength: 0.0,
        };
        let mut records = [GpuInstanceRecord::new(identity_transform(0.0, 0.0, 0.0), 0)];

        let updated = VegetationEvaluator::update_instances(
            &mut instances,
            &colliders,
            &params,
            0.016,
            0.0,
            &mut records,
        );

        assert_eq!(updated, 1);
        assert_eq!(instances[0].current_bend_offset, [0.0, 0.0]);
        assert_eq!(records[0].world_from_local, instances[0].base_transform);
        assert_eq!(records[0].semantic_id_words[0], 42);
    }

    #[test]
    fn vegetation_bends_away_from_intruding_collider() {
        let mut instances = [VegetationInstanceState::new(identity_transform(1.0, 0.0, 0.0), 1.0, 101)];
        // Collider placed at origin, overlapping instance at (1.0, 0, 0)
        let colliders = [VegetationCollider {
            position: [0.0, 0.0, 0.0],
            radius: 1.5,
            velocity: [1.0, 0.0, 0.0],
        }];
        let params = VegetationBendingParams {
            stiffness: 0.1,
            recovery_rate: 10.0,
            max_bend_angle: 1.2,
            wind_direction: [0.0, 0.0],
            wind_strength: 0.0,
        };
        let mut records = [GpuInstanceRecord::new(identity_transform(0.0, 0.0, 0.0), 0)];

        // Run several simulation steps
        for step in 0..10 {
            VegetationEvaluator::update_instances(
                &mut instances,
                &colliders,
                &params,
                0.016,
                step as f32 * 0.016,
                &mut records,
            );
        }

        // Instance should bend in positive X direction (away from collider at 0.0)
        assert!(
            instances[0].current_bend_offset[0] > 0.1,
            "vegetation must bend away from collider along positive X, got {}",
            instances[0].current_bend_offset[0]
        );
        assert!(
            records[0].world_from_local[1][0] > 0.1,
            "gpu instance record must carry deformed bend matrix"
        );
    }
}
