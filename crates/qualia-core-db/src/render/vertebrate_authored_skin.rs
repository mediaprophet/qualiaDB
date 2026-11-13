//! Portable anatomy binds use the scaffold's world coordinates, never archetype dimensions.
use crate::render::skeletal::fauna_rig::{
    FaunaBiomechanicalArchetype, FaunaSkeletalRig10d, CLIP_CRUISE_TROT_GLIDE, CLIP_SPRINT_HOP_DIVE,
    FAUNA_PARENT_INDICES,
};
use crate::render::skeletal::{validate_joints, Joint, Motor};

pub const AUTHORED_JOINT_COUNT: usize = 21;
pub const RIGHT_SHOULDER: usize = 18;
pub const RIGHT_ELBOW: usize = 19;
pub const RIGHT_WRIST: usize = 20;

/// Indices 0..18 retain the fauna ABI; the independent right forelimb is appended.
#[derive(Clone, Debug)]
pub struct AuthoredSkinBinding {
    pub joints: Vec<Joint>,
    pub cruise: Vec<Motor>,
    pub sprint: Vec<Motor>,
}

impl AuthoredSkinBinding {
    /// Landmarks are joint pivots in the same metre/world frame as the authored mesh.
    /// Rotations from the archetype poses are retargeted onto these local binds.
    pub fn from_landmarks(
        archetype: FaunaBiomechanicalArchetype,
        landmarks: &[[f64; 3]; AUTHORED_JOINT_COUNT],
    ) -> Result<Self, String> {
        if landmarks
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 10_000.0)
        {
            return Err("bind landmarks must be finite world positions within 10000 metres".into());
        }
        let baseline = FaunaSkeletalRig10d::new(archetype);
        let mut result = Self {
            joints: Vec::with_capacity(AUTHORED_JOINT_COUNT),
            cruise: Vec::with_capacity(AUTHORED_JOINT_COUNT),
            sprint: Vec::with_capacity(AUTHORED_JOINT_COUNT),
        };
        for i in 0..AUTHORED_JOINT_COUNT {
            let parent = match i {
                RIGHT_SHOULDER => Some(2),
                RIGHT_ELBOW => Some(RIGHT_SHOULDER as u16),
                RIGHT_WRIST => Some(RIGHT_ELBOW as u16),
                _ => (FAUNA_PARENT_INDICES[i] >= 0).then_some(FAUNA_PARENT_INDICES[i] as u16),
            };
            let p = landmarks[i];
            let origin = parent.map_or([0.0; 3], |j| landmarks[j as usize]);
            let local =
                Motor::from_translation(p[0] - origin[0], p[1] - origin[1], p[2] - origin[2]);
            result.joints.push(Joint {
                parent,
                rest_local: local,
                inverse_bind: Motor::from_translation(-p[0], -p[1], -p[2]),
            });
            let source = if i >= RIGHT_SHOULDER {
                i - RIGHT_SHOULDER + 12
            } else {
                i
            };
            let delta = |clip: usize| {
                baseline.rest_pose[source]
                    .conjugate()
                    .mul(&baseline.clip_poses[clip][source])
                    .normalize()
            };
            result
                .cruise
                .push(local.mul(&delta(CLIP_CRUISE_TROT_GLIDE)).normalize());
            result
                .sprint
                .push(local.mul(&delta(CLIP_SPRINT_HOP_DIVE)).normalize());
        }
        validate_joints(&result.joints).map_err(|e| format!("invalid authored bind: {e:?}"))?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::skeletal::compose_palette_into;

    fn wallaby_landmarks() -> [[f64; 3]; AUTHORED_JOINT_COUNT] {
        [
            [0.0, 0.335, -0.115],
            [0.0, 0.395, -0.055],
            [0.0, 0.470, 0.090],
            [0.0, 0.520, 0.135],
            [0.0, 0.565, 0.165],
            [0.0, 0.545, 0.200],
            [0.068, 0.328, -0.115],
            [0.076, 0.225, -0.022],
            [0.066, 0.088, -0.135],
            [-0.068, 0.328, -0.115],
            [-0.076, 0.225, -0.022],
            [-0.066, 0.088, -0.135],
            [0.052, 0.438, 0.102],
            [0.058, 0.365, 0.088],
            [0.050, 0.324, 0.142],
            [0.0, 0.318, -0.140],
            [0.0, 0.115, -0.355],
            [0.0, 0.022, -0.600],
            [-0.052, 0.438, 0.102],
            [-0.058, 0.365, 0.088],
            [-0.050, 0.324, 0.142],
        ]
    }

    #[test]
    fn authored_pivots_preserve_joint_attachment_in_both_bent_poses() {
        let landmarks = wallaby_landmarks();
        let binding = AuthoredSkinBinding::from_landmarks(
            FaunaBiomechanicalArchetype::MacropodSaltatorial,
            &landmarks,
        )
        .unwrap();
        let rest: Vec<_> = binding.joints.iter().map(|j| j.rest_local).collect();
        for pose in [&rest, &binding.cruise, &binding.sprint] {
            let mut globals = vec![Motor::identity(); AUTHORED_JOINT_COUNT];
            let mut palette = globals.clone();
            compose_palette_into(
                &binding.joints,
                pose,
                Motor::identity(),
                &mut globals,
                &mut palette,
            )
            .unwrap();
            for (i, joint) in binding.joints.iter().enumerate() {
                if let Some(parent) = joint.parent {
                    // A shared anatomical endpoint cannot separate when either adjacent
                    // bone owns it: rotation must occur about that authored endpoint.
                    let a = palette[i].transform_point(landmarks[i]);
                    let b = palette[parent as usize].transform_point(landmarks[i]);
                    for axis in 0..3 {
                        assert!((a[axis] - b[axis]).abs() < 1e-8, "joint {i} detached");
                    }
                    let p = globals[parent as usize].transform_point([0.0; 3]);
                    let length2 = |a: [f64; 3], b: [f64; 3]| {
                        (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>()
                    };
                    assert!(
                        (length2(a, p) - length2(landmarks[i], landmarks[parent as usize])).abs()
                            < 1e-8
                    );
                }
            }
        }
        assert_eq!(
            binding.joints[RIGHT_ELBOW].parent,
            Some(RIGHT_SHOULDER as u16)
        );
        assert_eq!(binding.joints[RIGHT_WRIST].parent, Some(RIGHT_ELBOW as u16));
        // The old generic skeleton must fail this species-coordinate endpoint check.
        let generic = FaunaSkeletalRig10d::new(FaunaBiomechanicalArchetype::MacropodSaltatorial);
        let mut globals = vec![Motor::identity(); 18];
        let mut palette = globals.clone();
        compose_palette_into(
            &generic.joints,
            &generic.clip_poses[CLIP_CRUISE_TROT_GLIDE],
            Motor::identity(),
            &mut globals,
            &mut palette,
        )
        .unwrap();
        let moved = palette[6].transform_point(landmarks[6]);
        assert!((moved[2] - landmarks[6][2]).abs() > 0.1);
    }
}
