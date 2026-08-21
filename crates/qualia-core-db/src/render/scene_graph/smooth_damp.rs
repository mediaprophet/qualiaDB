//! Smooth damp — critically-damped spring smoothing (Unity-style).
//!
//! Smoothly moves a current value toward a target, with a maximum speed
//! and smooth time. Based on Game Programming Gems 1, Chapter 3.9.

/// Smoothly damp a scalar value toward a target.
///
/// - `current`: Current value.
/// - `target`: Target value.
/// - `velocity`: Current velocity (mutated in place).
/// - `smooth_time`: Approximate time to reach the target (seconds).
/// - `max_speed`: Maximum speed.
/// - `delta_time`: Frame time (seconds).
///
/// Returns the new value and updates `velocity`.
pub fn smooth_damp(
    current: f32,
    target: f32,
    velocity: &mut f32,
    smooth_time: f32,
    max_speed: f32,
    delta_time: f32,
) -> f32 {
    let smooth_time = smooth_time.max(0.0001);
    let omega = 2.0 / smooth_time;
    let x = omega * delta_time;
    let exp = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);

    let mut change = current - target;
    let max_change = max_speed * smooth_time;
    change = change.clamp(-max_change, max_change);

    // Adjust target after clamping — this is the key step that prevents
    // the result from snapping to the original target when the gap is large.
    let adjusted_target = current - change;

    let temp = (*velocity + omega * change) * delta_time;
    *velocity = (*velocity - omega * temp) * exp;
    let result = adjusted_target + (change + temp) * exp;
    result
}

/// Smoothly damp a 3D vector toward a target.
pub fn smooth_damp_vec3(
    current: [f32; 3],
    target: [f32; 3],
    velocity: &mut [f32; 3],
    smooth_time: f32,
    max_speed: f32,
    delta_time: f32,
) -> [f32; 3] {
    [
        smooth_damp(
            current[0],
            target[0],
            &mut velocity[0],
            smooth_time,
            max_speed,
            delta_time,
        ),
        smooth_damp(
            current[1],
            target[1],
            &mut velocity[1],
            smooth_time,
            max_speed,
            delta_time,
        ),
        smooth_damp(
            current[2],
            target[2],
            &mut velocity[2],
            smooth_time,
            max_speed,
            delta_time,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smooth_damp_approaches_target() {
        let mut vel = 0.0;
        let mut current = 0.0;
        let target = 10.0;
        for _ in 0..100 {
            current = smooth_damp(current, target, &mut vel, 0.3, f32::INFINITY, 0.016);
        }
        assert!(
            (current - target).abs() < 0.1,
            "should be close to target: {current}"
        );
    }

    #[test]
    fn smooth_damp_at_target_stays() {
        let mut vel = 0.0;
        let current = 5.0;
        let target = 5.0;
        let result = smooth_damp(current, target, &mut vel, 0.3, f32::INFINITY, 0.016);
        assert!((result - target).abs() < 0.01);
        assert!(vel.abs() < 0.01);
    }

    #[test]
    fn smooth_damp_respects_max_speed() {
        // max_speed limits the velocity, so over many frames the position
        // should not approach the target faster than max_speed allows.
        let mut vel = 0.0;
        let mut current = 0.0;
        let target = 10.0;
        let max_speed = 1.0;
        let dt = 0.016;
        // Run for 0.5 seconds — at max_speed=1.0, we should not have moved
        // more than ~0.5 units (max_speed * elapsed_time).
        for _ in 0..31 {
            current = smooth_damp(current, target, &mut vel, 1.0, max_speed, dt);
        }
        let elapsed = 31.0 * dt;
        let max_expected = max_speed * elapsed + 1.0; // +1.0 tolerance for smoothing
        assert!(
            current.abs() < max_expected,
            "should respect max speed: current={current}, max_expected={max_expected}"
        );
    }

    #[test]
    fn smooth_damp_vec3_approaches() {
        let mut vel = [0.0; 3];
        let mut current = [0.0; 3];
        let target = [10.0, 0.0, 5.0];
        for _ in 0..100 {
            current = smooth_damp_vec3(current, target, &mut vel, 0.3, f32::INFINITY, 0.016);
        }
        assert!((current[0] - 10.0).abs() < 0.1);
        assert!((current[2] - 5.0).abs() < 0.1);
    }

    #[test]
    fn smooth_damp_oscillates_not() {
        // Critically damped — should not overshoot significantly.
        let mut vel = 0.0;
        let mut current = 0.0;
        let target = 10.0;
        let mut max_value = 0.0f32;
        for _ in 0..200 {
            current = smooth_damp(current, target, &mut vel, 0.5, f32::INFINITY, 0.016);
            if current > max_value {
                max_value = current;
            }
        }
        // Should not overshoot by more than a small amount.
        assert!(
            max_value < target + 1.0,
            "should not overshoot much: max={max_value}"
        );
    }

    #[test]
    fn smooth_damp_min_smooth_time() {
        let mut vel = 0.0;
        // Very small smooth_time should still work.
        let result = smooth_damp(0.0, 10.0, &mut vel, 0.0, f32::INFINITY, 0.016);
        // Should not panic or produce NaN.
        assert!(result.is_finite());
    }
}
