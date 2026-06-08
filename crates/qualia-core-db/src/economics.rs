use rand_distr::{Distribution, StandardNormal};

#[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
use rayon::prelude::*;

use crate::ode_solver::{rk4_step, PhysicalState};

/// Simulates a single path of Geometric Brownian Motion (GBM)
/// dS = mu * S * dt + sigma * S * dW
pub fn simulate_gbm_path(
    initial_price: f64,
    drift: f64,
    volatility: f64,
    time_horizon: f64,
    steps: usize,
) -> f64 {
    let dt = time_horizon / steps as f64;
    let mut current_price = initial_price;
    let mut rng = rand::rng();

    for _ in 0..steps {
        let z: f64 = StandardNormal.sample(&mut rng);
        // Discrete GBM approximation
        current_price *=
            f64::exp((drift - 0.5 * volatility.powi(2)) * dt + volatility * f64::sqrt(dt) * z);
    }

    current_price
}

/// Runs a Monte Carlo simulation to calculate the expected end value
/// and the Value at Risk (VaR) at a 95% confidence interval.
/// Automatically utilizes Rayon parallel execution on Desktop/Server builds,
/// while degrading gracefully to a single thread on Android to preserve battery.
pub fn run_monte_carlo_var(
    initial_price: f64,
    drift: f64,
    volatility: f64,
    time_horizon: f64,
    steps: usize,
    paths: usize,
) -> (f64, f64) {
    // Abstract execution iterator based on OS target
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    let mut final_prices: Vec<f64> = (0..paths)
        .into_par_iter()
        .map(|_| simulate_gbm_path(initial_price, drift, volatility, time_horizon, steps))
        .collect();

    #[cfg(any(target_os = "android", target_arch = "wasm32"))]
    let mut final_prices: Vec<f64> = (0..paths)
        .into_iter()
        .map(|_| simulate_gbm_path(initial_price, drift, volatility, time_horizon, steps))
        .collect();

    // Sort to find percentiles
    final_prices.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Calculate Expected Value (Mean)
    let mean: f64 = final_prices.iter().sum::<f64>() / paths as f64;

    // Calculate 95% VaR (Value at 5th percentile)
    let var_index = (paths as f64 * 0.05).floor() as usize;
    let var_95 = initial_price - final_prices[var_index]; // Potential loss

    (mean, var_95)
}

/// Evaluates a simple macroeconomic System Dynamics flow.
/// Models the Equation of Exchange (M*V = P*Q).
/// - state[0] = Money Supply (M)
/// - state[1] = Price Level (P)
pub fn simulate_macroeconomic_flow(
    initial_m: f64,
    initial_p: f64,
    velocity: f64,
    real_gdp: f64, // Q
    time_horizon: f64,
    steps: usize,
) -> PhysicalState {
    let dt = time_horizon / steps as f64;
    let mut state = PhysicalState {
        time: 0.0,
        values: vec![initial_m, initial_p],
    };

    // dy/dt
    let macro_derivative = |_t: f64, y: &[f64]| -> Vec<f64> {
        let current_m = y[0];
        let current_p = y[1];

        // M increases slowly due to inflation/printing (e.g. 2% growth)
        let dm_dt = current_m * 0.02;

        // P adapts based on P = (M * V) / Q
        // dP/dt nudges P towards the target price level
        let target_p = (current_m * velocity) / real_gdp;
        let dp_dt = 0.5 * (target_p - current_p); // Adjustment rate

        vec![dm_dt, dp_dt]
    };

    for _ in 0..steps {
        rk4_step(&mut state, dt, &macro_derivative);
    }

    state
}

/// Context regarding the physical state of the node
pub struct SystemContext {
    pub current_battery_level: f32,
    pub cpu_temperature: f32,
    pub network_congestion_index: f64,
}

/// Get mock system context
pub fn get_current_system_context() -> SystemContext {
    SystemContext {
        current_battery_level: 0.8,
        cpu_temperature: 45.0,
        network_congestion_index: 0.2,
    }
}

/// Calculates bandwidth liability in USD based on gb routed and context.
pub fn calculate_bandwidth_liability(bytes: usize, context: &SystemContext) -> f64 {
    let gb_routed = bytes as f64 / 1_073_741_824.0;
    let mut base_rate = 0.05; // .05 per GB

    // Dynamically adjust based on system context
    base_rate += context.network_congestion_index * 0.05;

    // Low battery -> demands higher compensation
    if context.current_battery_level < 0.2 {
        base_rate += 0.05;
    }

    // High temperature -> throttling penalty
    if context.cpu_temperature > 70.0 {
        base_rate += 0.02;
    }

    gb_routed * base_rate
}
