//! Advanced control algorithms — adaptive PID tuning, Model Predictive Control, and MIMO
//! state-space — complementing the PID/anti-windup loops in the parent module. Pure `f64` math,
//! zero-heap (caller-supplied buffers; bounded loops).

// ─── Adaptive PID tuning (gain scheduling + MIT-rule adaptation) ───────────────────

/// Scale a PID gain triple by a scalar `adaptation` factor — the simplest gain-scheduling step
/// (retune as the operating regime changes). Returns `(kp, ki, kd)`.
#[inline]
pub fn adaptive_gains(kp: f64, ki: f64, kd: f64, adaptation: f64) -> (f64, f64, f64) {
    (kp * adaptation, ki * adaptation, kd * adaptation)
}

/// **MIT-rule** online gain adaptation (model-reference adaptive control): nudge `gain` to reduce
/// the tracking `error` — `gain + adaptation_rate · error · signal`. One adaptation step.
#[inline]
pub fn mit_rule_adapt(gain: f64, adaptation_rate: f64, error: f64, signal: f64) -> f64 {
    gain + adaptation_rate * error * signal
}

/// **Gain scheduling**: linearly interpolate the gain between `low_gain` (at `op_min`) and
/// `high_gain` (at `op_max`) for the current `op_point` — adaptive tuning across the operating
/// envelope. Clamps outside `[op_min, op_max]`.
pub fn scheduled_gain(
    op_point: f64,
    op_min: f64,
    op_max: f64,
    low_gain: f64,
    high_gain: f64,
) -> f64 {
    if op_max <= op_min {
        return low_gain;
    }
    let t = ((op_point - op_min) / (op_max - op_min)).clamp(0.0, 1.0);
    low_gain + t * (high_gain - low_gain)
}

// ─── Model Predictive Control (receding horizon) ──────────────────────────────────

/// One-step **Model Predictive Control** for a scalar LTI plant `x_{k+1} = a·x_k + b·u`: search
/// candidate controls across `[u_min, u_max]` (a grid of `steps`+1 points), simulate `horizon`
/// steps holding `u`, and return the `u` minimising `Σ (setpoint − x_k)² + control_penalty·u²`.
/// The optimal FIRST move of the receding horizon. Zero-heap (bounded loops, no allocation).
pub fn mpc_control(
    a: f64,
    b: f64,
    state: f64,
    setpoint: f64,
    horizon: u32,
    u_min: f64,
    u_max: f64,
    steps: u32,
    control_penalty: f64,
) -> f64 {
    let n = steps.max(1);
    let mut best_u = u_min;
    let mut best_cost = f64::INFINITY;
    for i in 0..=n {
        let u = u_min + (u_max - u_min) * (i as f64) / (n as f64);
        let mut x = state;
        let mut cost = 0.0;
        for _ in 0..horizon {
            x = a * x + b * u;
            let e = setpoint - x;
            cost += e * e;
        }
        cost += control_penalty * u * u;
        if cost < best_cost {
            best_cost = cost;
            best_u = u;
        }
    }
    best_u
}

// ─── MIMO state-space (multi-input multi-output) ──────────────────────────────────

/// MIMO state transition `x' = A·x + B·u` for a system with `n` states and `m` inputs. `a` is the
/// `n×n` state matrix (row-major), `b` the `n×m` input matrix (row-major), `x` the state (len n),
/// `u` the input (len m); the next state is written to `out` (len n). Zero-heap. Returns `false`
/// on a dimension mismatch.
pub fn mimo_step(a: &[f64], b: &[f64], x: &[f64], u: &[f64], out: &mut [f64]) -> bool {
    let n = x.len();
    let m = u.len();
    if a.len() < n * n || b.len() < n * m || out.len() < n {
        return false;
    }
    for i in 0..n {
        let mut s = 0.0;
        for j in 0..n {
            s += a[i * n + j] * x[j];
        }
        for j in 0..m {
            s += b[i * m + j] * u[j];
        }
        out[i] = s;
    }
    true
}

/// MIMO output equation `y = C·x + D·u` with `p` outputs: `c` is `p×n` (row-major), `d` is `p×m`
/// (row-major); the output is written to `out` (len p). Zero-heap. Returns `false` on a mismatch.
pub fn mimo_output(c: &[f64], d: &[f64], x: &[f64], u: &[f64], p: usize, out: &mut [f64]) -> bool {
    let n = x.len();
    let m = u.len();
    if c.len() < p * n || d.len() < p * m || out.len() < p {
        return false;
    }
    for i in 0..p {
        let mut s = 0.0;
        for j in 0..n {
            s += c[i * n + j] * x[j];
        }
        for j in 0..m {
            s += d[i * m + j] * u[j];
        }
        out[i] = s;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn adaptive_tuning_schedules_and_adapts_gains() {
        // Scalar adaptation scales all three gains.
        let (kp, ki, kd) = adaptive_gains(1.0, 0.5, 0.1, 2.0);
        assert!(close(kp, 2.0) && close(ki, 1.0) && close(kd, 0.2));
        // MIT rule nudges the gain to cut tracking error.
        assert!(close(mit_rule_adapt(1.0, 0.1, 2.0, 3.0), 1.6)); // 1 + 0.1*2*3
                                                                 // Gain scheduling interpolates across the envelope and clamps.
        assert!(close(scheduled_gain(5.0, 0.0, 10.0, 1.0, 3.0), 2.0)); // midpoint
        assert!(close(scheduled_gain(-5.0, 0.0, 10.0, 1.0, 3.0), 1.0)); // clamp low
        assert!(close(scheduled_gain(99.0, 0.0, 10.0, 1.0, 3.0), 3.0)); // clamp high
    }

    #[test]
    fn mpc_drives_state_toward_setpoint() {
        // Integrator x' = x + u, from 0 to setpoint 10, no control penalty → u ≈ 10 over horizon 1.
        let u = mpc_control(1.0, 1.0, 0.0, 10.0, 1, 0.0, 20.0, 200, 0.0);
        assert!(
            (u - 10.0).abs() <= 0.1,
            "MPC picks u≈10 to reach the setpoint, got {u}"
        );
        // A control penalty pulls the optimal move below the unpenalised value.
        let u_pen = mpc_control(1.0, 1.0, 0.0, 10.0, 1, 0.0, 20.0, 200, 1.0);
        assert!(u_pen < u, "control penalty reduces the aggressive move");
    }

    #[test]
    fn mimo_state_space_step_and_output() {
        // 2-state, 1-input system. A = [[1,1],[0,1]] (a double integrator), B = [[0],[1]].
        let a = [1.0, 1.0, 0.0, 1.0];
        let b = [0.0, 1.0];
        let x = [0.0, 0.0];
        let u = [2.0];
        let mut nx = [0.0; 2];
        assert!(mimo_step(&a, &b, &x, &u, &mut nx));
        // x' = A·x + B·u = [0, 2].
        assert!(close(nx[0], 0.0) && close(nx[1], 2.0));
        // Output y = C·x + D·u with C = [[1,0]] (observe position), D = [[0]].
        let c = [1.0, 0.0];
        let d = [0.0];
        let mut y = [0.0; 1];
        assert!(mimo_output(&c, &d, &nx, &u, 1, &mut y));
        assert!(close(y[0], 0.0)); // position still 0 after one step
                                   // Dimension mismatch refuses.
        assert!(!mimo_step(&a, &b, &x, &u, &mut [0.0; 1]));
    }
}
