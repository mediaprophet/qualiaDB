//! Stochastic finance/economics kernels.
//!
//! The legacy public helpers are convenience APIs. The seeded `*_into` variants
//! are the deterministic, caller-buffered kernels to prefer for tests, WASM, and
//! any future hot-path integration.

use rand_distr::{Distribution, StandardNormal};

#[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
use rayon::prelude::*;

/// Default seed for deterministic Monte Carlo callers that do not supply their
/// own scenario seed.
pub const DEFAULT_MONTE_CARLO_SEED: u64 = 0x5144_4245_434f_4e31;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StochasticError {
    InvalidSteps,
    InvalidPaths,
    OutputBufferTooSmall,
    NonFiniteInput,
}

#[derive(Debug, Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn unit_open(&mut self) -> f64 {
        // Map to (0, 1) with 53 random mantissa bits.
        let bits = self.next_u64() >> 11;
        ((bits as f64) + 0.5) * (1.0 / ((1u64 << 53) as f64))
    }

    fn gaussian(&mut self) -> f64 {
        let u1 = self.unit_open();
        let u2 = self.unit_open();
        (-2.0 * u1.ln()).sqrt() * (core::f64::consts::TAU * u2).cos()
    }
}

fn valid_gbm_inputs(
    initial_price: f64,
    drift: f64,
    volatility: f64,
    time_horizon: f64,
    steps: usize,
) -> bool {
    steps > 0
        && initial_price.is_finite()
        && drift.is_finite()
        && volatility.is_finite()
        && time_horizon.is_finite()
        && initial_price >= 0.0
        && volatility >= 0.0
        && time_horizon >= 0.0
}

fn gbm_step(current_price: f64, drift: f64, volatility: f64, dt: f64, z: f64) -> f64 {
    current_price
        * f64::exp((drift - 0.5 * volatility * volatility) * dt + volatility * dt.sqrt() * z)
}

/// Simulates a single path of Geometric Brownian Motion (GBM) using the ambient
/// random source. Kept for compatibility with existing CLI/WASM surfaces.
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
        current_price = gbm_step(current_price, drift, volatility, dt, z);
    }

    current_price
}

/// Deterministic single-path GBM simulation.
pub fn simulate_gbm_path_seeded(
    initial_price: f64,
    drift: f64,
    volatility: f64,
    time_horizon: f64,
    steps: usize,
    seed: u64,
) -> Result<f64, StochasticError> {
    if !valid_gbm_inputs(initial_price, drift, volatility, time_horizon, steps) {
        return if steps == 0 {
            Err(StochasticError::InvalidSteps)
        } else {
            Err(StochasticError::NonFiniteInput)
        };
    }
    let dt = time_horizon / steps as f64;
    let mut current_price = initial_price;
    let mut rng = SplitMix64::new(seed);

    for _ in 0..steps {
        current_price = gbm_step(current_price, drift, volatility, dt, rng.gaussian());
    }
    Ok(current_price)
}

/// Writes each GBM step price into `out`. Returns the number of prices written.
pub fn simulate_gbm_steps_into(
    initial_price: f64,
    drift: f64,
    volatility: f64,
    time_horizon: f64,
    steps: usize,
    seed: u64,
    out: &mut [f64],
) -> Result<usize, StochasticError> {
    if steps == 0 {
        return Err(StochasticError::InvalidSteps);
    }
    if out.len() < steps {
        return Err(StochasticError::OutputBufferTooSmall);
    }
    if !valid_gbm_inputs(initial_price, drift, volatility, time_horizon, steps) {
        return Err(StochasticError::NonFiniteInput);
    }

    let dt = time_horizon / steps as f64;
    let mut current_price = initial_price;
    let mut rng = SplitMix64::new(seed);
    for slot in out.iter_mut().take(steps) {
        current_price = gbm_step(current_price, drift, volatility, dt, rng.gaussian());
        *slot = current_price;
    }
    Ok(steps)
}

/// Runs a deterministic Monte Carlo VaR calculation into a caller-owned buffer.
///
/// `final_prices_out` is sorted ascending before returning, so callers may reuse
/// it for quantile diagnostics. Returns `(paths, mean_final_price, var_95_loss)`.
pub fn run_monte_carlo_var_seeded_into(
    initial_price: f64,
    drift: f64,
    volatility: f64,
    time_horizon: f64,
    steps: usize,
    paths: usize,
    seed: u64,
    final_prices_out: &mut [f64],
) -> Result<(usize, f64, f64), StochasticError> {
    if paths == 0 {
        return Err(StochasticError::InvalidPaths);
    }
    if final_prices_out.len() < paths {
        return Err(StochasticError::OutputBufferTooSmall);
    }
    if !valid_gbm_inputs(initial_price, drift, volatility, time_horizon, steps) {
        return if steps == 0 {
            Err(StochasticError::InvalidSteps)
        } else {
            Err(StochasticError::NonFiniteInput)
        };
    }

    let mut sum = 0.0;
    for (i, slot) in final_prices_out.iter_mut().take(paths).enumerate() {
        let path_seed = seed ^ ((i as u64).wrapping_mul(0xD1B5_4A32_D192_ED03));
        let final_price = simulate_gbm_path_seeded(
            initial_price,
            drift,
            volatility,
            time_horizon,
            steps,
            path_seed,
        )?;
        *slot = final_price;
        sum += final_price;
    }

    let prices = &mut final_prices_out[..paths];
    prices.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let mean = sum / paths as f64;
    let var_index = ((paths as f64 * 0.05).floor() as usize).min(paths - 1);
    let var_95 = initial_price - prices[var_index];
    Ok((paths, mean, var_95))
}

/// Runs a Monte Carlo simulation to calculate the expected end value and the
/// Value at Risk (VaR) at a 95% confidence interval.
///
/// This is the compatibility facade. Prefer `run_monte_carlo_var_seeded_into`
/// for deterministic, caller-buffered execution.
pub fn run_monte_carlo_var(
    initial_price: f64,
    drift: f64,
    volatility: f64,
    time_horizon: f64,
    steps: usize,
    paths: usize,
) -> (f64, f64) {
    if paths == 0 || steps == 0 {
        return (f64::NAN, f64::NAN);
    }

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

    final_prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let mean: f64 = final_prices.iter().sum::<f64>() / paths as f64;
    let var_index = ((paths as f64 * 0.05).floor() as usize).min(paths.saturating_sub(1));
    let var_95 = initial_price - final_prices[var_index];

    (mean, var_95)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_gbm_is_reproducible() {
        let a = simulate_gbm_path_seeded(100.0, 0.05, 0.2, 1.0, 32, 7).unwrap();
        let b = simulate_gbm_path_seeded(100.0, 0.05, 0.2, 1.0, 32, 7).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn caller_buffered_path_writes_steps() {
        let mut out = [0.0f64; 4];
        let n = simulate_gbm_steps_into(100.0, 0.0, 0.0, 1.0, 4, 1, &mut out).unwrap();
        assert_eq!(n, 4);
        for price in out {
            assert!(price > 0.0);
        }
    }

    #[test]
    fn seeded_var_uses_caller_buffer() {
        let mut prices = [0.0f64; 16];
        let (n, mean, var) =
            run_monte_carlo_var_seeded_into(100.0, 0.02, 0.1, 1.0, 12, 16, 42, &mut prices)
                .unwrap();
        assert_eq!(n, 16);
        assert!(mean.is_finite());
        assert!(var.is_finite());
        assert!(prices.windows(2).all(|w| w[0] <= w[1]));
    }
}
