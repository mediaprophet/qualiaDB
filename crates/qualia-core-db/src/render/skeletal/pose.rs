use super::{
    valid_motor, validate_joints, Clip, Joint, Motor, SkeletalError, MAX_JOINTS, MAX_KEYS,
};

fn validate_clip(clip: Clip<'_>, count: usize) -> Result<(), SkeletalError> {
    if clip.keys.len() > MAX_KEYS || clip.tracks.len() > MAX_JOINTS {
        return Err(SkeletalError::BudgetExceeded);
    }
    if !clip.duration.is_finite() || clip.duration <= 0.0 {
        return Err(SkeletalError::InvalidInput);
    }
    let mut seen = [false; MAX_JOINTS];
    for track in clip.tracks {
        let joint = track.joint as usize;
        let start = track.first_key as usize;
        let end = start
            .checked_add(track.key_count as usize)
            .ok_or(SkeletalError::InvalidInput)?;
        if joint >= count || seen[joint] || track.key_count == 0 || end > clip.keys.len() {
            return Err(SkeletalError::InvalidInput);
        }
        seen[joint] = true;
        let keys = &clip.keys[start..end];
        if keys.iter().any(|k| {
            !k.time.is_finite() || k.time < 0.0 || k.time > clip.duration || !valid_motor(&k.local)
        }) || keys.windows(2).any(|pair| pair[0].time >= pair[1].time)
        {
            return Err(SkeletalError::InvalidInput);
        }
        // Loops declare their seam explicitly, avoiding an implicit snap or
        // interpolating a missing end pose across the entire duration.
        if clip.looping && (keys[0].time != 0.0 || keys[keys.len() - 1].time != clip.duration) {
            return Err(SkeletalError::InvalidInput);
        }
        if clip.looping {
            let (a, ta) = keys[0].local.to_rotation_translation();
            let (b, tb) = keys[keys.len() - 1].local.to_rotation_translation();
            let dot = a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>().abs();
            if (dot - 1.0).abs() > 1e-5 || ta.iter().zip(tb).any(|(x, y)| (x - y).abs() > 1e-5) {
                return Err(SkeletalError::InvalidInput);
            }
        }
    }
    Ok(())
}

/// Samples a local pose, optionally transitioning from a caller-owned pose.
/// `blend` is target influence [0,1]. Missing tracks retain the rig rest pose.
/// Validation finishes before modifying any output. Time is visual, not a game day.
pub fn sample_pose_into(
    joints: &[Joint],
    clip: Clip<'_>,
    time: f64,
    previous: Option<&[Motor]>,
    blend: f64,
    out: &mut [Motor],
) -> Result<usize, SkeletalError> {
    validate_joints(joints)?;
    validate_clip(clip, joints.len())?;
    if !time.is_finite()
        || !blend.is_finite()
        || !(0.0..=1.0).contains(&blend)
        || previous.is_some_and(|p| p.len() != joints.len() || p.iter().any(|m| !valid_motor(m)))
    {
        return Err(SkeletalError::InvalidInput);
    }
    if out.len() < joints.len() {
        return Err(SkeletalError::OutputTooSmall);
    }
    let t = if clip.looping {
        time.rem_euclid(clip.duration)
    } else {
        time.clamp(0.0, clip.duration)
    };
    for (i, joint) in joints.iter().enumerate() {
        out[i] = joint.rest_local;
    }
    for track in clip.tracks {
        let start = track.first_key as usize;
        let keys = &clip.keys[start..start + track.key_count as usize];
        let upper = keys.partition_point(|k| k.time <= t);
        out[track.joint as usize] = if upper == 0 {
            keys[0].local
        } else if upper == keys.len() {
            keys[keys.len() - 1].local
        } else {
            let a = keys[upper - 1];
            let b = keys[upper];
            Motor::sclerp(&a.local, &b.local, (t - a.time) / (b.time - a.time))
        };
    }
    if let Some(previous) = previous {
        for i in 0..joints.len() {
            out[i] = Motor::sclerp(&previous[i], &out[i], blend);
        }
    }
    Ok(joints.len())
}

/// Iterative parent-before-child composition and inverse bind application.
/// `globals` doubles as reusable attachment transforms for tools and clothing.
pub fn compose_palette_into(
    joints: &[Joint],
    local: &[Motor],
    root: Motor,
    globals: &mut [Motor],
    palette: &mut [Motor],
) -> Result<usize, SkeletalError> {
    validate_joints(joints)?;
    if local.len() != joints.len() || !valid_motor(&root) || local.iter().any(|m| !valid_motor(m)) {
        return Err(SkeletalError::InvalidInput);
    }
    if globals.len() < joints.len() || palette.len() < joints.len() {
        return Err(SkeletalError::OutputTooSmall);
    }
    for (i, joint) in joints.iter().enumerate() {
        globals[i] = match joint.parent {
            Some(parent) => globals[parent as usize].mul(&local[i]),
            None => root.mul(&local[i]),
        }
        .normalize();
        palette[i] = globals[i].mul(&joint.inverse_bind).normalize();
    }
    Ok(joints.len())
}
