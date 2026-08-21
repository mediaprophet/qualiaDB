//! Inverse kinematics — look-at and CCD (Cyclic Coordinate Descent).
//!
//! Simple IK solvers for kinematic chains. These operate on 3D joint
//! positions and produce rotation corrections.

/// IK solver result.
#[derive(Debug, Clone)]
pub struct IkResult {
    /// Updated joint positions after solving.
    pub joint_positions: Vec<[f32; 3]>,
    /// Whether the target was reached within tolerance.
    pub converged: bool,
    /// Number of iterations used.
    pub iterations: u32,
    /// Final distance to target.
    pub final_distance: f32,
}

/// Look-at IK: rotate a chain so the end effector points toward a target.
///
/// This rotates the root joint so the chain's end points toward `target`.
/// For a single-segment chain, this is a direct look-at. For multi-segment,
/// only the root is adjusted (use `ccd_ik` for full-chain solving).
pub fn look_at_ik(joint_positions: &[[f32; 3]], target: [f32; 3]) -> IkResult {
    if joint_positions.len() < 2 {
        return IkResult {
            joint_positions: joint_positions.to_vec(),
            converged: false,
            iterations: 0,
            final_distance: f32::INFINITY,
        };
    }

    let mut positions = joint_positions.to_vec();
    let root = positions[0];
    let end = positions[positions.len() - 1];

    // Current direction and desired direction.
    let current_dir = normalize3(sub3(end, root));
    let desired_dir = normalize3(sub3(target, root));

    // Rotation axis = cross product.
    let axis = cross3(current_dir, desired_dir);
    let axis_len = norm3(axis);
    if axis_len < 1e-6 {
        return IkResult {
            joint_positions: positions,
            converged: true,
            iterations: 0,
            final_distance: dist3(end, target),
        };
    }

    let angle = dot3(current_dir, desired_dir).clamp(-1.0, 1.0).acos();
    let axis = normalize3(axis);

    // Rotate all joints after root around the axis.
    let chain_len = positions.len();
    for i in 1..chain_len {
        let relative = sub3(positions[i], root);
        let rotated = rotate_axis_angle(relative, axis, angle);
        positions[i] = add3(root, rotated);
    }

    let end = positions[chain_len - 1];
    let dist = dist3(end, target);
    IkResult {
        joint_positions: positions,
        converged: dist < 0.01,
        iterations: 1,
        final_distance: dist,
    }
}

/// CCD (Cyclic Coordinate Descent) IK solver.
///
/// Iteratively rotates each joint starting from the end effector backward
/// to the root, aligning the end effector with the target.
pub fn ccd_ik(
    joint_positions: &[[f32; 3]],
    target: [f32; 3],
    max_iterations: u32,
    tolerance: f32,
) -> IkResult {
    if joint_positions.len() < 2 {
        return IkResult {
            joint_positions: joint_positions.to_vec(),
            converged: false,
            iterations: 0,
            final_distance: f32::INFINITY,
        };
    }

    let mut positions = joint_positions.to_vec();
    let n = positions.len();
    let end_idx = n - 1;

    let mut iter = 0u32;
    let mut dist = dist3(positions[end_idx], target);

    while dist > tolerance && iter < max_iterations {
        // Iterate from the joint before the end effector back to the root.
        for i in (0..end_idx).rev() {
            let joint = positions[i];
            let end = positions[end_idx];

            let to_end = normalize3(sub3(end, joint));
            let to_target = normalize3(sub3(target, joint));

            let axis = cross3(to_end, to_target);
            let axis_len = norm3(axis);
            if axis_len < 1e-6 {
                continue;
            }

            let angle = dot3(to_end, to_target).clamp(-1.0, 1.0).acos();
            let axis = normalize3(axis);

            // Rotate all joints after i around the axis at positions[i].
            for j in (i + 1)..n {
                let relative = sub3(positions[j], joint);
                let rotated = rotate_axis_angle(relative, axis, angle);
                positions[j] = add3(joint, rotated);
            }
        }

        dist = dist3(positions[end_idx], target);
        iter += 1;
    }

    IkResult {
        joint_positions: positions,
        converged: dist <= tolerance,
        iterations: iter,
        final_distance: dist,
    }
}

// ── Vector helpers ───────────────────────────────────────────────────────────

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm3(a: [f32; 3]) -> f32 {
    dot3(a, a).sqrt()
}

fn normalize3(a: [f32; 3]) -> [f32; 3] {
    let n = norm3(a);
    if n < 1e-10 {
        return [0.0; 3];
    }
    [a[0] / n, a[1] / n, a[2] / n]
}

fn dist3(a: [f32; 3], b: [f32; 3]) -> f32 {
    norm3(sub3(a, b))
}

/// Rotate vector `v` around `axis` by `angle` (Rodrigues' formula).
fn rotate_axis_angle(v: [f32; 3], axis: [f32; 3], angle: f32) -> [f32; 3] {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let cross = cross3(axis, v);
    let dot = dot3(axis, v);
    [
        v[0] * cos_a + cross[0] * sin_a + axis[0] * dot * (1.0 - cos_a),
        v[1] * cos_a + cross[1] * sin_a + axis[1] * dot * (1.0 - cos_a),
        v[2] * cos_a + cross[2] * sin_a + axis[2] * dot * (1.0 - cos_a),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn look_at_basic() {
        // Chain along X axis, target along Y axis.
        let joints = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
        let target = [0.0, 1.0, 0.0];
        let result = look_at_ik(&joints, target);
        assert!(result.converged || result.final_distance < 2.0);
        // End should be closer to target than before.
        let original_dist = dist3(joints[1], target);
        assert!(result.final_distance <= original_dist);
    }

    #[test]
    fn look_at_already_aligned() {
        let joints = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
        let target = [2.0, 0.0, 0.0];
        let result = look_at_ik(&joints, target);
        assert!(result.converged);
    }

    #[test]
    fn ccd_converges() {
        // Simple 3-joint chain along X, target up in Y.
        let joints = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
        let target = [0.0, 2.0, 0.0];
        let result = ccd_ik(&joints, target, 50, 0.01);
        assert!(
            result.final_distance < 0.1,
            "CCD should get close to target: dist={}",
            result.final_distance
        );
    }

    #[test]
    fn ccd_reachable_target() {
        // Chain of length 2 along X, target slightly off-axis (reachable).
        let joints = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
        let target = [1.0, 1.0, 0.0];
        let result = ccd_ik(&joints, target, 50, 0.01);
        assert!(
            result.converged,
            "CCD should converge to reachable target: dist={}",
            result.final_distance
        );
    }

    #[test]
    fn ccd_unreachable_target() {
        // Target way beyond chain length — should get as close as possible.
        let joints = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
        let target = [100.0, 0.0, 0.0];
        let result = ccd_ik(&joints, target, 10, 0.01);
        // Should not converge but should point in the right direction.
        assert!(!result.converged);
        // End should be at distance ~1 from root (chain length).
        let end = result.joint_positions[1];
        let dist_from_root = norm3(end);
        assert!((dist_from_root - 1.0).abs() < 0.1);
    }

    #[test]
    fn ik_single_joint_returns_error() {
        let joints = [[0.0, 0.0, 0.0]];
        let result = look_at_ik(&joints, [1.0, 0.0, 0.0]);
        assert!(!result.converged);
    }

    #[test]
    fn ccd_preserves_chain_length() {
        let joints = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
        let original_len1 = dist3(joints[0], joints[1]);
        let original_len2 = dist3(joints[1], joints[2]);
        let result = ccd_ik(&joints, [0.0, 2.0, 0.0], 50, 0.01);
        let new_len1 = dist3(result.joint_positions[0], result.joint_positions[1]);
        let new_len2 = dist3(result.joint_positions[1], result.joint_positions[2]);
        assert!((new_len1 - original_len1).abs() < 0.01);
        assert!((new_len2 - original_len2).abs() < 0.01);
    }
}
