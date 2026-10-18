//! Bounded skin influence authoring and seam remapping into caller buffers.
use super::{SkeletalError, SkinWeight, MAX_JOINTS, MAX_VERTICES};

#[derive(Debug, Clone, Copy)]
pub struct BoneSegment {
    pub joint: u16,
    pub start: [f32; 3],
    pub end: [f32; 3],
}

/// The authored influence set is specific to a mesh part. Nearest two segments
/// blend across hinges; rigid props should author a single attachment segment.
pub fn weights_from_segments_into(
    positions: &[[f32; 3]],
    segments: &[BoneSegment],
    joint_count: usize,
    influences: u8,
    out: &mut [SkinWeight],
) -> Result<usize, SkeletalError> {
    if positions.len() > MAX_VERTICES || segments.len() > MAX_JOINTS {
        return Err(SkeletalError::BudgetExceeded);
    }
    if positions.is_empty()
        || segments.is_empty()
        || joint_count == 0
        || joint_count > MAX_JOINTS
        || !(1..=2).contains(&influences)
        || positions
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 1e8)
        || segments.iter().any(|s| {
            s.joint as usize >= joint_count
                || s.start
                    .iter()
                    .chain(s.end.iter())
                    .any(|v| !v.is_finite() || v.abs() > 1e8)
        })
    {
        return Err(SkeletalError::InvalidInput);
    }
    if out.len() < positions.len() {
        return Err(SkeletalError::OutputTooSmall);
    }
    for (i, p) in positions.iter().enumerate() {
        let mut nearest = [(f64::INFINITY, 0usize); 2];
        for (s, bone) in segments.iter().enumerate() {
            let delta = std::array::from_fn::<_, 3, _>(|a| (bone.end[a] - bone.start[a]) as f64);
            let relative = std::array::from_fn::<_, 3, _>(|a| (p[a] - bone.start[a]) as f64);
            let length = delta.iter().map(|v| v * v).sum::<f64>();
            let t = if length < 1e-12 {
                0.0
            } else {
                (relative.iter().zip(delta).map(|(x, y)| x * y).sum::<f64>() / length)
                    .clamp(0.0, 1.0)
            };
            let distance = (0..3)
                .map(|a| (relative[a] - delta[a] * t).powi(2))
                .sum::<f64>();
            if distance < nearest[0].0 {
                nearest[1] = nearest[0];
                nearest[0] = (distance, s);
            } else if distance < nearest[1].0 {
                nearest[1] = (distance, s);
            }
        }
        let first = 1.0 / (nearest[0].0 + 0.0001);
        let second = if influences == 2 && nearest[1].0.is_finite() {
            1.0 / (nearest[1].0 + 0.0001)
        } else {
            0.0
        };
        let weight = (first / (first + second)) as f32;
        out[i] = SkinWeight {
            joints: [
                segments[nearest[0].1].joint,
                segments[nearest[1].1].joint,
                0,
                0,
            ],
            weights: [weight, 1.0 - weight, 0.0, 0.0],
        };
    }
    Ok(positions.len())
}

/// Remaps skin weights into `out`.
pub fn remap_skin_weights_into(
    source: &[SkinWeight],
    lineage: &[u32],
    out: &mut [SkinWeight],
) -> Result<usize, SkeletalError> {
    if lineage.len() > MAX_VERTICES {
        return Err(SkeletalError::BudgetExceeded);
    }
    if source.is_empty()
        || lineage.is_empty()
        || lineage.iter().any(|v| *v as usize >= source.len())
    {
        return Err(SkeletalError::InvalidInput);
    }
    if out.len() < lineage.len() {
        return Err(SkeletalError::OutputTooSmall);
    }
    for (i, original) in lineage.iter().enumerate() {
        out[i] = source[*original as usize];
    }
    Ok(lineage.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rigid_attachments_choose_one_joint_and_seams_keep_weights() {
        let bones = [
            BoneSegment {
                joint: 1,
                start: [-1.0, 0.0, 0.0],
                end: [-1.0, 1.0, 0.0],
            },
            BoneSegment {
                joint: 2,
                start: [1.0, 0.0, 0.0],
                end: [1.0, 1.0, 0.0],
            },
        ];
        let mut skin = [SkinWeight {
            joints: [0; 4],
            weights: [0.0; 4],
        }; 2];
        weights_from_segments_into(
            &[[-0.9, 0.5, 0.0], [0.9, 0.5, 0.0]],
            &bones,
            3,
            1,
            &mut skin,
        )
        .unwrap();
        assert_eq!(skin[0].joints[0], 1);
        assert_eq!(skin[1].joints[0], 2);
        assert_eq!(skin[0].weights, [1.0, 0.0, 0.0, 0.0]);
        let mut mapped = [skin[0]; 3];
        remap_skin_weights_into(&skin, &[1, 0, 1], &mut mapped).unwrap();
        assert_eq!(mapped[0].joints, skin[1].joints);
        assert_eq!(mapped[2].weights, skin[1].weights);
    }
    #[test]
    fn hinge_weights_are_normalized_and_bad_lineage_is_atomic() {
        let bones = [
            BoneSegment {
                joint: 1,
                start: [0.0, 0.0, 0.0],
                end: [0.0, 1.0, 0.0],
            },
            BoneSegment {
                joint: 2,
                start: [0.0, 1.0, 0.0],
                end: [0.0, 2.0, 0.0],
            },
        ];
        let mut skin = [SkinWeight {
            joints: [0; 4],
            weights: [0.0; 4],
        }];
        weights_from_segments_into(&[[0.0, 1.0, 0.0]], &bones, 3, 2, &mut skin).unwrap();
        assert_eq!(skin[0].weights, [0.5, 0.5, 0.0, 0.0]);
        let before = skin[0];
        let source = skin;
        assert_eq!(
            remap_skin_weights_into(&source, &[1], &mut skin),
            Err(SkeletalError::InvalidInput)
        );
        assert_eq!(skin[0].joints, before.joints);
        assert_eq!(skin[0].weights, before.weights);
    }
}
