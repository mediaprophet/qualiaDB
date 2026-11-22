//! Rest-anchored leaf motor composition shared by native motion backends.

use crate::render::instance_culling::GpuInstanceRecord;

use super::{Motor, SemanticMotionError};

/// Apply a local motion delta before the fitted left-hand instance transform.
/// The rest transform can include uniform authored scale and portal framing.
pub fn compose_leaf_delta(
    rest: &GpuInstanceRecord,
    motor: Motor,
) -> Result<GpuInstanceRecord, SemanticMotionError> {
    if !super::super::valid_motor(&motor) {
        return Err(SemanticMotionError::InvalidCommand);
    }
    let translation = motor.transform_point([0.0; 3]);
    let transform: [[f64; 4]; 4] = std::array::from_fn(|column| {
        if column == 3 {
            return [translation[0], translation[1], translation[2], 1.0];
        }
        let mut axis = [0.0; 3];
        axis[column] = 1.0;
        let v = motor.transform_point(axis);
        [v[0] - translation[0], v[1] - translation[1], v[2] - translation[2], 0.0]
    });
    let world = std::array::from_fn(|column| std::array::from_fn(|row| {
        (0..4).map(|lane| f64::from(rest.world_from_local[lane][row]) * transform[column][lane])
            .sum::<f64>() as f32
    }));
    let [lo, hi] = rest.semantic_id_words;
    let instance = GpuInstanceRecord::new(world, (u64::from(hi) << 32) | u64::from(lo));
    if instance.world_from_local.iter().flatten().chain(instance.normal_from_local.iter().flatten())
        .any(|v| !v.is_finite())
    {
        return Err(SemanticMotionError::InvalidCommand);
    }
    Ok(instance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_local_motion_once_under_the_authored_pose_and_keeps_semantic_id() {
        let rest = GpuInstanceRecord::new([
            [2.0, 0.0, 0.0, 0.0], [0.0, 2.0, 0.0, 0.0], [0.0, 0.0, 2.0, 0.0], [1.0, 3.0, 4.0, 1.0],
        ], 0xfedcba9876543210);
        let motor = Motor::from_rotation_translation([1.0, 0.0, 0.0, 0.0], [0.25, 0.0, 0.0]);
        let first = compose_leaf_delta(&rest, motor).unwrap();
        let again = compose_leaf_delta(&rest, motor).unwrap();
        assert_eq!(first.world_from_local[3], [1.5, 3.0, 4.0, 1.0]);
        assert_eq!(first.world_from_local, again.world_from_local);
        assert_eq!(first.semantic_id_words, rest.semantic_id_words);
    }
}
