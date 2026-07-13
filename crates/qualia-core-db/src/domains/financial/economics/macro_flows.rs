//! Macroeconomic flow models.

use crate::ode_solver::{rk4_step, PhysicalState};

/// Evaluates a simple macroeconomic System Dynamics flow.
/// Models the Equation of Exchange (M*V = P*Q).
/// - state[0] = Money Supply (M)
/// - state[1] = Price Level (P)
pub fn simulate_macroeconomic_flow(
    initial_m: f64,
    initial_p: f64,
    velocity: f64,
    real_gdp: f64,
    time_horizon: f64,
    steps: usize,
) -> PhysicalState {
    let dt = time_horizon / steps as f64;
    let mut state = PhysicalState {
        time: 0.0,
        values: vec![initial_m, initial_p],
    };

    let macro_derivative = |_t: f64, y: &[f64]| -> Vec<f64> {
        let current_m = y[0];
        let current_p = y[1];
        let dm_dt = current_m * 0.02;
        let target_p = (current_m * velocity) / real_gdp;
        let dp_dt = 0.5 * (target_p - current_p);
        vec![dm_dt, dp_dt]
    };

    for _ in 0..steps {
        rk4_step(&mut state, dt, &macro_derivative);
    }

    state
}
