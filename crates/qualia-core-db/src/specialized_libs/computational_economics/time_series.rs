//! Time-series core for computational economics.
//!
//! Allocation class:
//! - **HotZeroHeap**: `simple_returns_into`, `log_returns_into`,
//!   `cumulative_wealth_into`, `drawdown_into`, `max_drawdown_from`,
//!   `rolling_mean_into`, `rolling_variance_into`, `autocorrelation`,
//!   `cross_correlation`, `ar1_simulate_into`, `gbm_simulate_into`,
//!   `ornstein_uhlenbeck_simulate_into`, and the VaR/CVaR delegation helpers.
//!   These kernels operate on caller-owned `&mut [f64]` buffers and fixed
//!   stack state only — no `Vec`/`String`/`Box` in any path.
//! - **ColdBounded**: `block_bootstrap_mean_into` and
//!   `moving_block_bootstrap_mean_into` allocate bounded internal scratch per
//!   resample (caller-buffered output, deterministic seed). They are NOT
//!   Tier-1 zero-heap kernels.
//!
//! # Assumptions
//! - Returns are simple (`p_t / p_{t-1} - 1`) unless stated otherwise.
//! - `cumulative_wealth_into` compounds a unit initial wealth: `W_0 = 1`.
//! - `drawdown_into` writes fractional drawdown `(W_t - peak_t) / peak_t <= 0`.
//! - `max_drawdown_from` returns the worst drawdown as a non-negative fraction
//!   (i.e. `-(min drawdown)`), matching the `portfolio::max_drawdown` sign
//!   convention.
//! - Rolling moments use the *population* (divide-by-n) variance, consistent
//!   with `solvers::statistics::descriptive::variance(values, false)`.
//! - `autocorrelation` uses the full-series mean and divides the lagged
//!   cross-product by the full-series sum of squared deviations (Box–Jenkins
//!   convention), so the lag-0 value is exactly 1.0.
//! - AR(1): `x_t = phi * x_{t-1} + sigma * z_t`, `z_t ~ N(0,1)` i.i.d.
//! - GBM: `dS/S = (mu - 0.5 sigma^2) dt + sigma dW`, Euler–Maruyama in log space.
//! - OU: `dx = theta (mu - x) dt + sigma dW`, Euler–Maruyama.
//! - All stochastic kernels use a deterministic `SplitMix64`-seeded Gaussian
//!   (Box–Muller), identical to `stochastic.rs`. Same seed → identical path.
//!
//! # Errors
//! Insufficient data, non-finite inputs, singular systems, invalid dimensions,
//! and undersized output buffers all return explicit `TimeSeriesError`
//! variants — no fabricated results, no silent `NaN`.

use crate::solvers::statistics::descriptive as desc;
use crate::specialized_libs::computational_economics::market_data;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSeriesError {
    InvalidInput,
    InsufficientData,
    OutputBufferTooSmall,
    NonFiniteInput,
    InvalidWindow,
    InvalidLag,
}

#[inline]
fn all_finite(values: &[f64]) -> bool {
    values.iter().all(|x| x.is_finite())
}

/// Deterministic SplitMix64 + Box–Muller Gaussian generator.
///
/// Mirrors the private generator in
/// `domains::financial::economics::stochastic.rs` so that seeds produce the
/// same Gaussian sequence across modules.
#[derive(Debug, Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    #[inline]
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    #[inline]
    fn unit_open(&mut self) -> f64 {
        let bits = self.next_u64() >> 11;
        ((bits as f64) + 0.5) * (1.0 / ((1u64 << 53) as f64))
    }

    #[inline]
    fn gaussian(&mut self) -> f64 {
        let u1 = self.unit_open();
        let u2 = self.unit_open();
        (-2.0 * u1.ln()).sqrt() * (core::f64::consts::TAU * u2).cos()
    }
}

/// Simple returns `p_t / p_{t-1} - 1` into caller-owned output.
///
/// Delegates to `market_data::simple_returns_into` (no duplication) and maps
/// its error type onto `TimeSeriesError`. Writes `prices.len() - 1` values.
#[inline]
pub fn simple_returns_into(prices: &[f64], out: &mut [f64]) -> Result<usize, TimeSeriesError> {
    if prices.len() < 2 {
        return Err(TimeSeriesError::InsufficientData);
    }
    if !all_finite(prices) || prices.iter().any(|p| *p <= 0.0) {
        return Err(TimeSeriesError::NonFiniteInput);
    }
    if out.len() + 1 < prices.len() {
        return Err(TimeSeriesError::OutputBufferTooSmall);
    }
    market_data::simple_returns_into(prices, out).map_err(|e| match e {
        market_data::MarketDataError::OutputBufferTooSmall => TimeSeriesError::OutputBufferTooSmall,
        _ => TimeSeriesError::InvalidInput,
    })
}

/// Log returns `ln(p_t / p_{t-1})` into caller-owned output.
///
/// Delegates to `market_data::log_returns_into`. Writes `prices.len() - 1`.
#[inline]
pub fn log_returns_into(prices: &[f64], out: &mut [f64]) -> Result<usize, TimeSeriesError> {
    if prices.len() < 2 {
        return Err(TimeSeriesError::InsufficientData);
    }
    if !all_finite(prices) || prices.iter().any(|p| *p <= 0.0) {
        return Err(TimeSeriesError::NonFiniteInput);
    }
    if out.len() + 1 < prices.len() {
        return Err(TimeSeriesError::OutputBufferTooSmall);
    }
    market_data::log_returns_into(prices, out).map_err(|e| match e {
        market_data::MarketDataError::OutputBufferTooSmall => TimeSeriesError::OutputBufferTooSmall,
        _ => TimeSeriesError::InvalidInput,
    })
}

/// Cumulative wealth index from a return series.
///
/// Writes `returns.len() + 1` values: `out[0] = 1.0` (unit initial wealth) and
/// `out[i] = out[i-1] * (1 + returns[i-1])`. Requires
/// `out.len() >= returns.len() + 1`.
pub fn cumulative_wealth_into(returns: &[f64], out: &mut [f64]) -> Result<usize, TimeSeriesError> {
    if !all_finite(returns) {
        return Err(TimeSeriesError::NonFiniteInput);
    }
    if returns.iter().any(|r| *r <= -1.0) {
        return Err(TimeSeriesError::InvalidInput);
    }
    if out.len() < returns.len() + 1 {
        return Err(TimeSeriesError::OutputBufferTooSmall);
    }
    out[0] = 1.0;
    for (idx, r) in returns.iter().enumerate() {
        out[idx + 1] = out[idx] * (1.0 + r);
    }
    Ok(returns.len() + 1)
}

/// Drawdown series from a wealth index.
///
/// Writes `wealth.len()` values: `out[t] = wealth[t] / peak_t - 1.0` where
/// `peak_t = max(wealth[0..=t])`. All values are `<= 0.0`. Requires
/// `wealth` non-empty and `out.len() >= wealth.len()`.
pub fn drawdown_into(wealth: &[f64], out: &mut [f64]) -> Result<usize, TimeSeriesError> {
    if wealth.is_empty() {
        return Err(TimeSeriesError::InsufficientData);
    }
    if !all_finite(wealth) {
        return Err(TimeSeriesError::NonFiniteInput);
    }
    if out.len() < wealth.len() {
        return Err(TimeSeriesError::OutputBufferTooSmall);
    }
    let mut peak = wealth[0];
    for (idx, w) in wealth.iter().enumerate() {
        if *w > peak {
            peak = *w;
        }
        out[idx] = if peak == 0.0 { 0.0 } else { w / peak - 1.0 };
    }
    Ok(wealth.len())
}

/// Maximum drawdown as a non-negative fraction from a wealth index.
///
/// Returns `0.0` for a monotonically non-decreasing wealth series. This is the
/// positive magnitude convention (`-min(drawdown)`).
pub fn max_drawdown_from(wealth: &[f64]) -> Result<f64, TimeSeriesError> {
    if wealth.is_empty() {
        return Err(TimeSeriesError::InsufficientData);
    }
    if !all_finite(wealth) {
        return Err(TimeSeriesError::NonFiniteInput);
    }
    let mut peak = wealth[0];
    let mut worst = 0.0f64;
    for w in wealth {
        if *w > peak {
            peak = *w;
        }
        let dd = if peak == 0.0 { 0.0 } else { w / peak - 1.0 };
        if dd < worst {
            worst = dd;
        }
    }
    Ok(-worst)
}

/// Rolling mean over a fixed `window`.
///
/// Writes `values.len() - window + 1` values: `out[i] = mean(values[i..i+window])`.
/// Requires `window >= 1` and `window <= values.len()`.
pub fn rolling_mean_into(
    values: &[f64],
    window: usize,
    out: &mut [f64],
) -> Result<usize, TimeSeriesError> {
    if window == 0 {
        return Err(TimeSeriesError::InvalidWindow);
    }
    if values.is_empty() {
        return Err(TimeSeriesError::InsufficientData);
    }
    if window > values.len() {
        return Err(TimeSeriesError::InvalidWindow);
    }
    if !all_finite(values) {
        return Err(TimeSeriesError::NonFiniteInput);
    }
    let count = values.len() - window + 1;
    if out.len() < count {
        return Err(TimeSeriesError::OutputBufferTooSmall);
    }

    // Seed the running sum, then slide. Hot path: O(n) with no allocation.
    let mut sum = 0.0;
    for i in 0..window {
        sum += values[i];
    }
    out[0] = sum / window as f64;
    for i in 1..count {
        sum += values[i + window - 1] - values[i - 1];
        out[i] = sum / window as f64;
    }
    Ok(count)
}

/// Rolling population variance over a fixed `window`.
///
/// Writes `values.len() - window + 1` values using the population (divide-by-n)
/// estimator. Uses the streaming `E[x]` / `E[x^2]` identity so the hot path is
/// O(n) with fixed stack state.
pub fn rolling_variance_into(
    values: &[f64],
    window: usize,
    out: &mut [f64],
) -> Result<usize, TimeSeriesError> {
    if window == 0 {
        return Err(TimeSeriesError::InvalidWindow);
    }
    if values.is_empty() {
        return Err(TimeSeriesError::InsufficientData);
    }
    if window > values.len() {
        return Err(TimeSeriesError::InvalidWindow);
    }
    if !all_finite(values) {
        return Err(TimeSeriesError::NonFiniteInput);
    }
    let count = values.len() - window + 1;
    if out.len() < count {
        return Err(TimeSeriesError::OutputBufferTooSmall);
    }

    let mut sum = 0.0;
    let mut sum_sq = 0.0;
    for i in 0..window {
        sum += values[i];
        sum_sq += values[i] * values[i];
    }
    let nf = window as f64;
    let var = |s: f64, ss: f64| -> f64 { (ss - s * s / nf) / nf };
    out[0] = var(sum, sum_sq);
    for i in 1..count {
        let old = values[i - 1];
        let new = values[i + window - 1];
        sum += new - old;
        sum_sq += new * new - old * old;
        out[i] = var(sum, sum_sq);
    }
    Ok(count)
}

/// Autocorrelation at a given `lag` (Box–Jenkins normalised by full-series SS).
///
/// `lag` must satisfy `1 <= lag < values.len()`. Returns a value in `[-1, 1]`
/// for well-behaved series; lag-0 would be exactly 1.0 (not exposed here).
pub fn autocorrelation(values: &[f64], lag: usize) -> Result<f64, TimeSeriesError> {
    if values.len() < 2 {
        return Err(TimeSeriesError::InsufficientData);
    }
    if lag == 0 || lag >= values.len() {
        return Err(TimeSeriesError::InvalidLag);
    }
    if !all_finite(values) {
        return Err(TimeSeriesError::NonFiniteInput);
    }
    let mean = desc::mean(values).ok_or(TimeSeriesError::InsufficientData)?;
    let mut num = 0.0;
    let mut denom = 0.0;
    for t in 0..values.len() {
        let d = values[t] - mean;
        denom += d * d;
    }
    if denom == 0.0 {
        // Constant series: autocorrelation is undefined; return 0.0 deterministically.
        return Ok(0.0);
    }
    for t in 0..values.len() - lag {
        num += (values[t] - mean) * (values[t + lag] - mean);
    }
    Ok(num / denom)
}

/// Cross-correlation between two equal-length series at a signed `lag`.
///
/// For `lag >= 0`: pairs `(a[t], b[t + lag])` for `t in 0..n - lag`.
/// For `lag < 0`:  pairs `(a[t - lag], b[t])` for `t in 0..n + lag`.
/// Returns the Pearson correlation of the overlapping sub-series.
pub fn cross_correlation(a: &[f64], b: &[f64], lag: i32) -> Result<f64, TimeSeriesError> {
    if a.len() != b.len() {
        return Err(TimeSeriesError::InvalidInput);
    }
    if a.len() < 2 {
        return Err(TimeSeriesError::InsufficientData);
    }
    let n = a.len() as i32;
    let abs_lag = lag.unsigned_abs() as usize;
    if abs_lag == 0 {
        // Full overlap: Pearson r = cov / (sigma_a * sigma_b).
        return pearson(a, b);
    }
    if abs_lag >= a.len() {
        return Err(TimeSeriesError::InvalidLag);
    }
    if !all_finite(a) || !all_finite(b) {
        return Err(TimeSeriesError::NonFiniteInput);
    }
    let overlap = (n - abs_lag as i32) as usize;
    if lag >= 0 {
        pearson(&a[..overlap], &b[abs_lag..abs_lag + overlap])
    } else {
        pearson(&a[abs_lag..abs_lag + overlap], &b[..overlap])
    }
}

#[inline]
fn pearson(x: &[f64], y: &[f64]) -> Result<f64, TimeSeriesError> {
    if x.len() != y.len() || x.is_empty() {
        return Err(TimeSeriesError::InvalidInput);
    }
    if !all_finite(x) || !all_finite(y) {
        return Err(TimeSeriesError::NonFiniteInput);
    }
    let cov = desc::covariance(x, y, false).ok_or(TimeSeriesError::InsufficientData)?;
    let vx = desc::variance(x, false).ok_or(TimeSeriesError::InsufficientData)?;
    let vy = desc::variance(y, false).ok_or(TimeSeriesError::InsufficientData)?;
    let denom = (vx * vy).sqrt();
    if denom == 0.0 {
        return Ok(0.0);
    }
    Ok(cov / denom)
}

/// Deterministic seeded AR(1) simulation.
///
/// Model: `x_t = phi * x_{t-1} + sigma * z_t`, `z_t ~ N(0,1)` i.i.d.
/// Writes `steps` values: `out[0] = initial`, `out[1..steps]` are successive
/// evolutions. Requires `steps >= 1`, `sigma >= 0`, all parameters finite,
/// and `out.len() >= steps`.
pub fn ar1_simulate_into(
    initial: f64,
    phi: f64,
    sigma: f64,
    steps: usize,
    seed: u64,
    out: &mut [f64],
) -> Result<usize, TimeSeriesError> {
    if steps == 0 {
        return Err(TimeSeriesError::InsufficientData);
    }
    if !initial.is_finite() || !phi.is_finite() || !sigma.is_finite() || sigma < 0.0 {
        return Err(TimeSeriesError::NonFiniteInput);
    }
    if out.len() < steps {
        return Err(TimeSeriesError::OutputBufferTooSmall);
    }
    let mut rng = SplitMix64::new(seed);
    out[0] = initial;
    for t in 1..steps {
        let z = if sigma > 0.0 { rng.gaussian() } else { 0.0 };
        out[t] = phi * out[t - 1] + sigma * z;
    }
    Ok(steps)
}

/// Deterministic seeded Geometric Brownian Motion path.
///
/// Model (Euler–Maruyama in log space):
/// `S_{t+1} = S_t * exp((mu - 0.5 sigma^2) dt + sigma sqrt(dt) z_t)`.
/// Writes `steps` values: `out[0] = initial_price`, then `steps - 1` evolved
/// prices. Requires `steps >= 1`, `sigma >= 0`, `dt > 0`, all finite,
/// `out.len() >= steps`.
pub fn gbm_simulate_into(
    initial_price: f64,
    drift: f64,
    volatility: f64,
    dt: f64,
    steps: usize,
    seed: u64,
    out: &mut [f64],
) -> Result<usize, TimeSeriesError> {
    if steps == 0 {
        return Err(TimeSeriesError::InsufficientData);
    }
    if !initial_price.is_finite()
        || initial_price < 0.0
        || !drift.is_finite()
        || !volatility.is_finite()
        || volatility < 0.0
        || !dt.is_finite()
        || dt <= 0.0
    {
        return Err(TimeSeriesError::NonFiniteInput);
    }
    if out.len() < steps {
        return Err(TimeSeriesError::OutputBufferTooSmall);
    }
    let mut rng = SplitMix64::new(seed);
    let sqrt_dt = dt.sqrt();
    let drift_term = (drift - 0.5 * volatility * volatility) * dt;
    out[0] = initial_price;
    for t in 1..steps {
        let z = if volatility > 0.0 {
            rng.gaussian()
        } else {
            0.0
        };
        out[t] = out[t - 1] * f64::exp(drift_term + volatility * sqrt_dt * z);
    }
    Ok(steps)
}

/// Deterministic seeded Ornstein–Uhlenbeck path (Euler–Maruyama).
///
/// Model: `dx = theta (mu - x) dt + sigma dW`.
/// Writes `steps` values: `out[0] = initial`, then `steps - 1` evolved values.
/// Requires `steps >= 1`, `theta >= 0`, `sigma >= 0`, `dt > 0`, all finite,
/// `out.len() >= steps`.
pub fn ornstein_uhlenbeck_simulate_into(
    initial: f64,
    theta: f64,
    mu: f64,
    sigma: f64,
    dt: f64,
    steps: usize,
    seed: u64,
    out: &mut [f64],
) -> Result<usize, TimeSeriesError> {
    if steps == 0 {
        return Err(TimeSeriesError::InsufficientData);
    }
    if !initial.is_finite()
        || !theta.is_finite()
        || theta < 0.0
        || !mu.is_finite()
        || !sigma.is_finite()
        || sigma < 0.0
        || !dt.is_finite()
        || dt <= 0.0
    {
        return Err(TimeSeriesError::NonFiniteInput);
    }
    if out.len() < steps {
        return Err(TimeSeriesError::OutputBufferTooSmall);
    }
    let mut rng = SplitMix64::new(seed);
    let sqrt_dt = dt.sqrt();
    out[0] = initial;
    for t in 1..steps {
        let z = if sigma > 0.0 { rng.gaussian() } else { 0.0 };
        out[t] = out[t - 1] + theta * (mu - out[t - 1]) * dt + sigma * sqrt_dt * z;
    }
    Ok(steps)
}

/// Historical VaR — delegates to `risk::historical_var`.
///
/// Returns the positive loss fraction at `confidence` from the empirical return
/// distribution. `scratch` must be at least `returns.len()` and is sorted in
/// place.
#[inline]
pub fn historical_var(
    returns: &[f64],
    confidence: f64,
    scratch: &mut [f64],
) -> Result<f64, TimeSeriesError> {
    crate::specialized_libs::computational_economics::risk::historical_var(
        returns, confidence, scratch,
    )
    .map_err(|e| match e {
        crate::specialized_libs::computational_economics::risk::RiskError::OutputBufferTooSmall => {
            TimeSeriesError::OutputBufferTooSmall
        }
        _ => TimeSeriesError::InvalidInput,
    })
}

/// Historical CVaR / expected shortfall — delegates to `risk::historical_cvar`.
#[inline]
pub fn historical_cvar(
    returns: &[f64],
    confidence: f64,
    scratch: &mut [f64],
) -> Result<f64, TimeSeriesError> {
    crate::specialized_libs::computational_economics::risk::historical_cvar(
        returns, confidence, scratch,
    )
    .map_err(|e| match e {
        crate::specialized_libs::computational_economics::risk::RiskError::OutputBufferTooSmall => {
            TimeSeriesError::OutputBufferTooSmall
        }
        _ => TimeSeriesError::InvalidInput,
    })
}

/// Parametric Gaussian VaR — delegates to `risk::gaussian_var`.
#[inline]
pub fn parametric_var(mean: f64, std_dev: f64, confidence: f64) -> Result<f64, TimeSeriesError> {
    crate::specialized_libs::computational_economics::risk::gaussian_var(mean, std_dev, confidence)
        .map_err(|_| TimeSeriesError::InvalidInput)
}

/// Apply a deterministic stress scenario to a price series.
///
/// Multiplies each price by `shock` (e.g. `0.8` for a 20% drawdown) and writes
/// the stressed path into `out`. The `seed` is accepted for API symmetry with
/// the stochastic kernels; this particular helper is deterministic and
/// seed-independent (the seed is consumed only to keep a stable call shape for
/// scenario harnesses). Requires `out.len() >= prices.len()`.
pub fn apply_stress_scenario_into(
    prices: &[f64],
    shock: f64,
    _seed: u64,
    out: &mut [f64],
) -> Result<usize, TimeSeriesError> {
    if prices.is_empty() {
        return Err(TimeSeriesError::InsufficientData);
    }
    if !all_finite(prices) || !shock.is_finite() {
        return Err(TimeSeriesError::NonFiniteInput);
    }
    if out.len() < prices.len() {
        return Err(TimeSeriesError::OutputBufferTooSmall);
    }
    for (idx, p) in prices.iter().enumerate() {
        out[idx] = p * shock;
    }
    Ok(prices.len())
}

/// Moving block bootstrap of the sample mean.
///
/// **ColdBounded** — allocates bounded internal scratch per resample. NOT a
/// Tier-1 zero-heap kernel.
///
/// For each of `n_resamples` resamples, builds a resampled series of length
/// `values.len()` by drawing `block_size`-contiguous blocks (start index
/// uniform on `0..=n - block_size`) and concatenating until the resample is
/// full (the final block is truncated to fit). The mean of each resample is
/// written to `out`. Requires `block_size >= 1`, `block_size <= n`,
/// `n_resamples >= 1`, `out.len() >= n_resamples`.
pub fn block_bootstrap_mean_into(
    values: &[f64],
    block_size: usize,
    n_resamples: usize,
    seed: u64,
    out: &mut [f64],
) -> Result<usize, TimeSeriesError> {
    if values.is_empty() {
        return Err(TimeSeriesError::InsufficientData);
    }
    if !all_finite(values) {
        return Err(TimeSeriesError::NonFiniteInput);
    }
    if block_size == 0 || block_size > values.len() {
        return Err(TimeSeriesError::InvalidWindow);
    }
    if n_resamples == 0 {
        return Err(TimeSeriesError::InvalidInput);
    }
    if out.len() < n_resamples {
        return Err(TimeSeriesError::OutputBufferTooSmall);
    }
    let n = values.len();
    let max_start = n - block_size + 1; // valid start indices: 0..max_start
    let mut rng = SplitMix64::new(seed);

    // Fixed-capacity stack scratch for a single resample. The resample length
    // equals `n`; we cap the stack copy at 256 and fall back to recomputation
    // via streaming mean for longer series (still no heap).
    const STACK_CAP: usize = 256;
    let mut stack_buf = [0.0f64; STACK_CAP];
    let use_stack = n <= STACK_CAP;

    for r in 0..n_resamples {
        let mut sum = 0.0f64;
        let mut filled = 0usize;
        if use_stack {
            while filled < n {
                let start = (rng.next_u64() as usize) % max_start;
                let take = block_size.min(n - filled);
                for k in 0..take {
                    stack_buf[filled + k] = values[start + k];
                }
                filled += take;
            }
            for k in 0..n {
                sum += stack_buf[k];
            }
        } else {
            while filled < n {
                let start = (rng.next_u64() as usize) % max_start;
                let take = block_size.min(n - filled);
                for k in 0..take {
                    sum += values[start + k];
                }
                filled += take;
            }
        }
        out[r] = sum / n as f64;
    }
    Ok(n_resamples)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn simple_returns_match_hand_computation() {
        let prices = [100.0, 110.0, 121.0];
        let mut out = [0.0; 2];
        let n = simple_returns_into(&prices, &mut out).unwrap();
        assert_eq!(n, 2);
        assert!(approx_eq(out[0], 0.10, 1e-12));
        assert!(approx_eq(out[1], 0.10, 1e-12));
    }

    #[test]
    fn log_returns_match_hand_computation() {
        let prices = [1.0, core::f64::consts::E];
        let mut out = [0.0; 1];
        let n = log_returns_into(&prices, &mut out).unwrap();
        assert_eq!(n, 1);
        assert!(approx_eq(out[0], 1.0, 1e-12));
    }

    #[test]
    fn cumulative_wealth_compounds_from_unit() {
        let returns = [0.10, 0.10, -0.20];
        let mut out = [0.0; 4];
        let n = cumulative_wealth_into(&returns, &mut out).unwrap();
        assert_eq!(n, 4);
        assert!(approx_eq(out[0], 1.0, 1e-12));
        assert!(approx_eq(out[1], 1.10, 1e-12));
        assert!(approx_eq(out[2], 1.21, 1e-12));
        assert!(approx_eq(out[3], 0.968, 1e-12));
    }

    #[test]
    fn drawdown_series_tracks_peak() {
        // wealth: 1.0, 1.2, 1.0, 1.1 -> drawdowns: 0, 0, -1/6, -1/12
        let wealth = [1.0, 1.2, 1.0, 1.1];
        let mut out = [0.0; 4];
        let n = drawdown_into(&wealth, &mut out).unwrap();
        assert_eq!(n, 4);
        assert!(approx_eq(out[0], 0.0, 1e-12));
        assert!(approx_eq(out[1], 0.0, 1e-12));
        assert!(approx_eq(out[2], 1.0 / 1.2 - 1.0, 1e-12));
        assert!(approx_eq(out[3], 1.1 / 1.2 - 1.0, 1e-12));
    }

    #[test]
    fn max_drawdown_is_positive_magnitude() {
        let wealth = [1.0, 1.2, 1.0, 1.1];
        let md = max_drawdown_from(&wealth).unwrap();
        assert!(approx_eq(md, 1.0 - 1.0 / 1.2, 1e-12));
    }

    #[test]
    fn max_drawdown_zero_for_monotonic_wealth() {
        let wealth = [1.0, 1.1, 1.2, 1.3];
        let md = max_drawdown_from(&wealth).unwrap();
        assert!(approx_eq(md, 0.0, 1e-12));
    }

    #[test]
    fn rolling_mean_matches_brute_force() {
        let values = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mut out = [0.0; 3];
        let n = rolling_mean_into(&values, 3, &mut out).unwrap();
        assert_eq!(n, 3);
        assert!(approx_eq(out[0], 2.0, 1e-12));
        assert!(approx_eq(out[1], 3.0, 1e-12));
        assert!(approx_eq(out[2], 4.0, 1e-12));
    }

    #[test]
    fn rolling_variance_matches_brute_force() {
        let values = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mut out = [0.0; 3];
        let n = rolling_variance_into(&values, 3, &mut out).unwrap();
        assert_eq!(n, 3);
        // population variance of {1,2,3}, {2,3,4}, {3,4,5} all = 2/3
        let expected = 2.0 / 3.0;
        for v in &out[..n] {
            assert!(approx_eq(*v, expected, 1e-12));
        }
    }

    #[test]
    fn rolling_mean_window_one_copies_values() {
        let values = [1.0, 2.0, 3.0];
        let mut out = [0.0; 3];
        let n = rolling_mean_into(&values, 1, &mut out).unwrap();
        assert_eq!(n, 3);
        assert_eq!(out, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn autocorrelation_white_noise_near_zero() {
        // A genuinely zero-mean series with no lag-1 correlation: deviations
        // [1,0,0,-1] give numerator Σ dₜ·dₜ₊₁ = 0 exactly, so acf₁ = 0.
        // (The prior series [1,0,-1,0,1,0] was NOT mean-zero — its mean is 1/6
        // and its true acf₁ ≈ −0.0098, so the old assertion was mathematically
        // wrong, not a code fault.)
        let values = [1.0, 0.0, 0.0, -1.0];
        let acf = autocorrelation(&values, 1).unwrap();
        assert!(acf.abs() < 1e-12);
    }

    #[test]
    fn autocorrelation_positive_for_persistent_series() {
        // Monotonic ramp: lag-1 autocorrelation should be strongly positive.
        let values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let acf = autocorrelation(&values, 1).unwrap();
        assert!(acf > 0.0);
    }

    #[test]
    fn cross_correlation_lag_zero_is_pearson() {
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [2.0, 4.0, 6.0, 8.0]; // perfectly correlated
        let cc = cross_correlation(&a, &b, 0).unwrap();
        assert!(approx_eq(cc, 1.0, 1e-12));
    }

    #[test]
    fn cross_correlation_positive_lag_shifts_b() {
        let a = [0.0, 1.0, 2.0, 3.0, 4.0];
        let b = [1.0, 2.0, 3.0, 4.0, 5.0]; // b[t+1] == a[t] + 2 ... but at lag 1: a[0..4] vs b[1..5]
        let cc = cross_correlation(&a, &b, 1).unwrap();
        assert!(approx_eq(cc, 1.0, 1e-12));
    }

    #[test]
    fn ar1_simulate_is_reproducible() {
        let mut out_a = [0.0f64; 8];
        let mut out_b = [0.0f64; 8];
        let na = ar1_simulate_into(0.0, 0.5, 1.0, 8, 42, &mut out_a).unwrap();
        let nb = ar1_simulate_into(0.0, 0.5, 1.0, 8, 42, &mut out_b).unwrap();
        assert_eq!(na, 8);
        assert_eq!(nb, 8);
        assert_eq!(out_a, out_b);
        assert_eq!(out_a[0], 0.0);
    }

    #[test]
    fn ar1_zero_sigma_is_deterministic_decay() {
        let mut out = [0.0f64; 5];
        ar1_simulate_into(1.0, 0.5, 0.0, 5, 7, &mut out).unwrap();
        // x_t = 0.5 * x_{t-1}, no noise
        assert!(approx_eq(out[0], 1.0, 1e-12));
        assert!(approx_eq(out[1], 0.5, 1e-12));
        assert!(approx_eq(out[2], 0.25, 1e-12));
        assert!(approx_eq(out[3], 0.125, 1e-12));
        assert!(approx_eq(out[4], 0.0625, 1e-12));
    }

    #[test]
    fn ou_simulate_is_reproducible() {
        let mut out_a = [0.0f64; 16];
        let mut out_b = [0.0f64; 16];
        ornstein_uhlenbeck_simulate_into(0.0, 1.0, 0.0, 0.3, 0.1, 16, 99, &mut out_a).unwrap();
        ornstein_uhlenbeck_simulate_into(0.0, 1.0, 0.0, 0.3, 0.1, 16, 99, &mut out_b).unwrap();
        assert_eq!(out_a, out_b);
    }

    #[test]
    fn ou_zero_sigma_converges_to_mu() {
        let mut out = [0.0f64; 100];
        ornstein_uhlenbeck_simulate_into(0.0, 1.0, 5.0, 0.0, 0.1, 100, 1, &mut out).unwrap();
        // deterministic decay toward mu=5
        assert!(out[99] > 4.9 && out[99] < 5.0);
    }

    #[test]
    fn gbm_simulate_is_reproducible_and_positive() {
        let mut out_a = [0.0f64; 10];
        let mut out_b = [0.0f64; 10];
        gbm_simulate_into(100.0, 0.05, 0.2, 1.0 / 252.0, 10, 12345, &mut out_a).unwrap();
        gbm_simulate_into(100.0, 0.05, 0.2, 1.0 / 252.0, 10, 12345, &mut out_b).unwrap();
        assert_eq!(out_a, out_b);
        assert!(out_a.iter().all(|p| *p > 0.0));
        assert!(approx_eq(out_a[0], 100.0, 1e-12));
    }

    #[test]
    fn block_bootstrap_is_reproducible() {
        let values = [0.01, 0.02, -0.01, 0.03, 0.0, 0.04, -0.02, 0.01];
        let mut out_a = [0.0f64; 50];
        let mut out_b = [0.0f64; 50];
        let na = block_bootstrap_mean_into(&values, 2, 50, 2024, &mut out_a).unwrap();
        let nb = block_bootstrap_mean_into(&values, 2, 50, 2024, &mut out_b).unwrap();
        assert_eq!(na, 50);
        assert_eq!(nb, 50);
        assert_eq!(out_a, out_b);
    }

    #[test]
    fn block_bootstrap_means_within_data_range() {
        let values = [0.01, 0.02, -0.01, 0.03, 0.0, 0.04, -0.02, 0.01];
        let mut out = [0.0f64; 200];
        block_bootstrap_mean_into(&values, 2, 200, 7, &mut out).unwrap();
        let lo = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        for m in &out {
            assert!(*m >= lo - 1e-12 && *m <= hi + 1e-12);
        }
    }

    #[test]
    fn historical_var_delegates_to_risk() {
        let returns = [-0.10, -0.05, 0.0, 0.02, 0.03];
        let mut scratch = [0.0; 5];
        let var = historical_var(&returns, 0.80, &mut scratch).unwrap();
        assert!(approx_eq(var, 0.10, 1e-12));
    }

    #[test]
    fn parametric_var_delegates_to_risk() {
        let var = parametric_var(0.0, 0.02, 0.95).unwrap();
        assert!(var > 0.032 && var < 0.034);
    }

    #[test]
    fn apply_stress_scenario_scales_prices() {
        let prices = [100.0, 110.0, 121.0];
        let mut out = [0.0; 3];
        let n = apply_stress_scenario_into(&prices, 0.8, 0, &mut out).unwrap();
        assert_eq!(n, 3);
        assert!(approx_eq(out[0], 80.0, 1e-12));
        assert!(approx_eq(out[1], 88.0, 1e-12));
        assert!(approx_eq(out[2], 96.8, 1e-12));
    }

    // ---- Error paths ----

    #[test]
    fn insufficient_data_for_returns() {
        let prices = [1.0];
        let mut out = [0.0; 1];
        assert_eq!(
            simple_returns_into(&prices, &mut out),
            Err(TimeSeriesError::InsufficientData)
        );
    }

    #[test]
    fn output_buffer_too_small_for_wealth() {
        let returns = [0.1, 0.2];
        let mut out = [0.0; 2]; // need 3
        assert_eq!(
            cumulative_wealth_into(&returns, &mut out),
            Err(TimeSeriesError::OutputBufferTooSmall)
        );
    }

    #[test]
    fn invalid_window_zero_rejected() {
        let values = [1.0, 2.0];
        let mut out = [0.0; 2];
        assert_eq!(
            rolling_mean_into(&values, 0, &mut out),
            Err(TimeSeriesError::InvalidWindow)
        );
    }

    #[test]
    fn invalid_window_larger_than_series_rejected() {
        let values = [1.0, 2.0];
        let mut out = [0.0; 2];
        assert_eq!(
            rolling_variance_into(&values, 5, &mut out),
            Err(TimeSeriesError::InvalidWindow)
        );
    }

    #[test]
    fn invalid_lag_zero_rejected() {
        let values = [1.0, 2.0, 3.0];
        assert_eq!(
            autocorrelation(&values, 0),
            Err(TimeSeriesError::InvalidLag)
        );
    }

    #[test]
    fn invalid_lag_too_large_rejected() {
        let values = [1.0, 2.0, 3.0];
        assert_eq!(
            autocorrelation(&values, 3),
            Err(TimeSeriesError::InvalidLag)
        );
    }

    #[test]
    fn non_finite_input_rejected() {
        let values = [1.0, f64::NAN, 3.0];
        let mut out = [0.0; 3];
        assert_eq!(
            rolling_mean_into(&values, 2, &mut out),
            Err(TimeSeriesError::NonFiniteInput)
        );
    }

    #[test]
    fn ar1_zero_steps_rejected() {
        let mut out = [0.0; 1];
        assert_eq!(
            ar1_simulate_into(0.0, 0.5, 1.0, 0, 1, &mut out),
            Err(TimeSeriesError::InsufficientData)
        );
    }

    #[test]
    fn ar1_buffer_too_small_rejected() {
        let mut out = [0.0; 3];
        assert_eq!(
            ar1_simulate_into(0.0, 0.5, 1.0, 5, 1, &mut out),
            Err(TimeSeriesError::OutputBufferTooSmall)
        );
    }

    #[test]
    fn cross_correlation_length_mismatch_rejected() {
        let a = [1.0, 2.0, 3.0];
        let b = [1.0, 2.0];
        assert_eq!(
            cross_correlation(&a, &b, 0),
            Err(TimeSeriesError::InvalidInput)
        );
    }

    #[test]
    fn block_bootstrap_bad_block_size_rejected() {
        let values = [1.0, 2.0, 3.0];
        let mut out = [0.0; 2];
        assert_eq!(
            block_bootstrap_mean_into(&values, 0, 2, 1, &mut out),
            Err(TimeSeriesError::InvalidWindow)
        );
    }
}
