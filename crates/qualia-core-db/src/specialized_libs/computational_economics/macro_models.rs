//! Macro models: Solow, Ramsey, RBC, New Keynesian, and OLG.
//!
//! Allocation class: **HotZeroHeap**. All scratch uses fixed-capacity stack
//! arrays. No `Vec`/`String`/`Box` in any kernel.
//!
//! Assumptions (common to all models unless overridden):
//! - Cobb-Douglas production: `Y = z * K^alpha * L^(1-alpha)`.
//! - CRRA utility: `U(c) = c^(1-sigma)/(1-sigma)`.
//! - Representative agent, infinite horizon, geometric discounting by `beta`.
//! - Discrete time.
//! - RBC uses an AR(1) technology shock with Gaussian innovations.

/// Maximum simulation periods.
pub const MAX_PERIODS: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroError {
    InvalidInput,
    NonFinite,
    NonConverged,
    BufferTooSmall,
}

/// SplitMix64 RNG for deterministic stochastic shocks (local copy to avoid
/// cross-module coupling).
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
        let bits = self.next_u64() >> 11;
        ((bits as f64) + 0.5) * (1.0 / ((1u64 << 53) as f64))
    }
    fn gaussian(&mut self) -> f64 {
        let u1 = self.unit_open();
        let u2 = self.unit_open();
        (-2.0 * u1.ln()).sqrt() * (core::f64::consts::TAU * u2).cos()
    }
}

fn require_finite(x: f64) -> Result<(), MacroError> {
    if x.is_finite() {
        Ok(())
    } else {
        Err(MacroError::NonFinite)
    }
}

// ---------------------------------------------------------------------------
// Solow growth model
// ---------------------------------------------------------------------------

/// Solow steady state: `k* = (s / (n + g + delta))^(1/(1-alpha))`.
///
/// Returns `(k_star, y_star)` where `y_star = k_star^alpha`.
pub fn solow_steady_state(
    s: f64,
    alpha: f64,
    delta: f64,
    n: f64,
    g: f64,
) -> Result<(f64, f64), MacroError> {
    if !(0.0..=1.0).contains(&s) || !(0.0..=1.0).contains(&alpha) || delta < 0.0 || n < 0.0 || g < 0.0 {
        return Err(MacroError::InvalidInput);
    }
    let denom = n + g + delta;
    if denom <= 0.0 {
        return Err(MacroError::InvalidInput);
    }
    let k_star = (s / denom).powf(1.0 / (1.0 - alpha));
    let y_star = k_star.powf(alpha);
    require_finite(k_star)?;
    require_finite(y_star)?;
    Ok((k_star, y_star))
}

/// Simulate discrete-time Solow: `k_{t+1} = (s*k_t^alpha + (1-delta)*k_t) / (1+n+g)`.
///
/// Writes `periods` per-capita capital values into `out`.
pub fn solow_simulate_into(
    k0: f64,
    s: f64,
    alpha: f64,
    delta: f64,
    n: f64,
    g: f64,
    periods: usize,
    out: &mut [f64],
) -> Result<usize, MacroError> {
    if periods == 0 || periods > MAX_PERIODS || out.len() < periods {
        return Err(MacroError::BufferTooSmall);
    }
    if k0 < 0.0 || !(0.0..=1.0).contains(&s) || !(0.0..=1.0).contains(&alpha) {
        return Err(MacroError::InvalidInput);
    }
    let denom = 1.0 + n + g;
    if denom <= 0.0 {
        return Err(MacroError::InvalidInput);
    }
    let mut k = k0;
    for t in 0..periods {
        out[t] = k;
        k = (s * k.powf(alpha) + (1.0 - delta) * k) / denom;
        require_finite(k)?;
    }
    Ok(periods)
}

// ---------------------------------------------------------------------------
// Ramsey model
// ---------------------------------------------------------------------------

/// Ramsey Euler equation residual.
///
/// `c_t = k^alpha - k_next + (1-delta)*k` (resource constraint).
/// Residual = `1 - beta * (alpha * k^(alpha-1) + 1 - delta) * (c_t / c_{t+1})^sigma`.
/// At the optimum this should be ~0.
pub fn ramsey_euler_residual(
    k: f64,
    k_next: f64,
    beta: f64,
    alpha: f64,
    delta: f64,
    sigma: f64,
) -> f64 {
    let c_t = k.powf(alpha) - k_next + (1.0 - delta) * k;
    // For a single pair, we treat c_{t+1} = c_t (steady-state check).
    let c_next = c_t;
    let r = alpha * k.powf(alpha - 1.0) + 1.0 - delta;
    1.0 - beta * r * (c_t / c_next).powf(sigma)
}

/// Ramsey steady-state capital: `k* = ((1/beta - 1 + delta) / alpha)^(1/(alpha-1))`.
pub fn ramsey_steady_state(
    alpha: f64,
    beta: f64,
    delta: f64,
) -> Result<f64, MacroError> {
    if !(0.0..=1.0).contains(&alpha) || !(0.0..=1.0).contains(&beta) || delta < 0.0 {
        return Err(MacroError::InvalidInput);
    }
    // 1/beta = alpha * k^(alpha-1) + 1 - delta
    // k^(alpha-1) = (1/beta - 1 + delta) / alpha
    let inner = (1.0 / beta - 1.0 + delta) / alpha;
    if inner <= 0.0 {
        return Err(MacroError::InvalidInput);
    }
    let k_star = inner.powf(1.0 / (alpha - 1.0));
    require_finite(k_star)?;
    Ok(k_star)
}

// ---------------------------------------------------------------------------
// RBC skeleton
// ---------------------------------------------------------------------------

/// Simulate an RBC model with AR(1) technology shock and a fixed savings-rate
/// consumption rule.
///
/// Production: `y = z * k^alpha`. Consumption: `c = (1-s) * y`. Capital:
/// `k_{t+1} = z*k^alpha + (1-delta)*k - c`. Shock: `z_t = rho*z_{t-1} + eps`,
/// `eps ~ N(0, sigma_z)`. Writes `k` and `z` series. Deterministic with seed.
pub fn rbc_simulate_into(
    k0: f64,
    z0: f64,
    alpha: f64,
    beta: f64,
    delta: f64,
    s: f64,
    rho_z: f64,
    sigma_z: f64,
    periods: usize,
    seed: u64,
    k_out: &mut [f64],
    z_out: &mut [f64],
) -> Result<usize, MacroError> {
    if periods == 0 || periods > MAX_PERIODS || k_out.len() < periods || z_out.len() < periods {
        return Err(MacroError::BufferTooSmall);
    }
    if k0 < 0.0 || !(0.0..=1.0).contains(&alpha) || !(0.0..=1.0).contains(&s) || delta < 0.0 {
        return Err(MacroError::InvalidInput);
    }
    let _ = beta; // beta not used in fixed-savings-rule skeleton
    let mut rng = SplitMix64::new(seed);
    let mut k = k0;
    let mut z = z0;
    for t in 0..periods {
        k_out[t] = k;
        z_out[t] = z;
        let y = z * k.powf(alpha);
        let c = (1.0 - s) * y;
        k = y + (1.0 - delta) * k - c;
        // AR(1) shock
        let eps = sigma_z * rng.gaussian();
        z = rho_z * z + eps;
        require_finite(k)?;
        require_finite(z)?;
        if k < 0.0 {
            return Err(MacroError::NonFinite);
        }
    }
    Ok(periods)
}

// ---------------------------------------------------------------------------
// New Keynesian 3-equation (linearized, 1-step perfect foresight)
// ---------------------------------------------------------------------------

/// Solve the linearized New Keynesian system for one period.
///
/// IS: `y_gap = E[y_gap_next] - (1/sigma)*(r - E[pi_next] - r_nat)`.
/// Phillips: `pi = beta * E[pi_next] + kappa * y_gap`.
/// Taylor: `r = rho_r * r_prev + (1-rho_r)*(phi_pi * pi + phi_y * y_gap)`.
///
/// For the deterministic 1-step case, expectations are set to 0. Returns
/// `(y_gap, pi, r)`.
pub fn new_keynesian_solve(
    r_prev: f64,
    beta: f64,
    kappa: f64,
    sigma: f64,
    phi_pi: f64,
    phi_y: f64,
    rho_r: f64,
    r_nat: f64,
) -> Result<(f64, f64, f64), MacroError> {
    if !(0.0..=1.0).contains(&beta) || kappa < 0.0 || sigma <= 0.0
        || phi_pi < 0.0 || phi_y < 0.0 || !(0.0..=1.0).contains(&rho_r)
    {
        return Err(MacroError::InvalidInput);
    }
    // With expectations = 0:
    // IS: y = -(1/sigma)*(r - r_nat)
    // Phillips: pi = kappa * y
    // Taylor: r = rho_r * r_prev + (1-rho_r)*(phi_pi * pi + phi_y * y)
    // Substitute:
    // r = rho_r * r_prev + (1-rho_r)*(phi_pi * kappa * y + phi_y * y)
    //   = rho_r * r_prev + (1-rho_r) * y * (phi_pi * kappa + phi_y)
    // y = -(1/sigma) * (r - r_nat)
    //   = -(1/sigma) * (rho_r * r_prev + (1-rho_r) * y * (phi_pi*kappa + phi_y) - r_nat)
    // Let A = (1-rho_r) * (phi_pi * kappa + phi_y)
    // y = -(1/sigma) * (rho_r * r_prev + A * y - r_nat)
    // y * (1 + A/sigma) = -(1/sigma) * (rho_r * r_prev - r_nat)
    // y = -(rho_r * r_prev - r_nat) / (sigma + A)
    let a = (1.0 - rho_r) * (phi_pi * kappa + phi_y);
    let denom = sigma + a;
    if denom.abs() < 1e-14 {
        return Err(MacroError::NonConverged);
    }
    let y_gap = -(rho_r * r_prev - r_nat) / denom;
    let pi = kappa * y_gap;
    let r = rho_r * r_prev + (1.0 - rho_r) * (phi_pi * pi + phi_y * y_gap);
    require_finite(y_gap)?;
    require_finite(pi)?;
    require_finite(r)?;
    Ok((y_gap, pi, r))
}

// ---------------------------------------------------------------------------
// Overlapping generations (2-period lived agents)
// ---------------------------------------------------------------------------

/// 2-period OLG steady state with log utility and Cobb-Douglas production.
///
/// Young agents supply 1 unit of labor, earn wage `w = (1-alpha)*y`, and
/// choose savings `s = w / (2 + rho)` where `rho` is the time-preference rate
/// (related to beta: `rho = (1-beta)/beta`). Capital evolves:
/// `(1+n) * k = s * w`. Returns `(k_star, y_star)`.
pub fn olg_steady_state(
    alpha: f64,
    beta: f64,
    n_pop_growth: f64,
) -> Result<(f64, f64), MacroError> {
    if !(0.0..=1.0).contains(&alpha) || !(0.0..=1.0).contains(&beta) || n_pop_growth < 0.0 {
        return Err(MacroError::InvalidInput);
    }
    let rho = (1.0 - beta) / beta;
    // s = w / (2 + rho), w = (1-alpha) * k^alpha
    // (1+n) * k = s = (1-alpha) * k^alpha / (2 + rho)
    // k^(1-alpha) = (1-alpha) / ((2 + rho) * (1 + n))
    let factor = (1.0 - alpha) / ((2.0 + rho) * (1.0 + n_pop_growth));
    if factor <= 0.0 {
        return Err(MacroError::InvalidInput);
    }
    let k_star = factor.powf(1.0 / (1.0 - alpha));
    let y_star = k_star.powf(alpha);
    require_finite(k_star)?;
    require_finite(y_star)?;
    Ok((k_star, y_star))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn solow_steady_state_formula() {
        // s=0.2, alpha=0.3, delta=0.05, n=0.01, g=0.02
        // denom = 0.08, k* = (0.2/0.08)^(1/0.7) = 2.5^(1/0.7)
        let (k, y) = solow_steady_state(0.2, 0.3, 0.05, 0.01, 0.02).unwrap();
        let expected = 2.5f64.powf(1.0 / 0.7);
        assert!(approx(k, expected, 1e-6));
        assert!(approx(y, expected.powf(0.3), 1e-6));
    }

    #[test]
    fn solow_simulation_converges() {
        let (k_star, _) = solow_steady_state(0.2, 0.3, 0.05, 0.01, 0.02).unwrap();
        let mut path = [0.0f64; 500];
        solow_simulate_into(1.0, 0.2, 0.3, 0.05, 0.01, 0.02, 500, &mut path).unwrap();
        // Should converge toward k_star from below.
        assert!(path[499] > path[0]);
        assert!(approx(path[499], k_star, 0.1));
    }

    #[test]
    fn ramsey_steady_state_formula() {
        // alpha=0.3, beta=0.96, delta=0.05
        // inner = (1/0.96 - 1 + 0.05) / 0.3 = (1.04167 - 0.95) / 0.3 = 0.09167/0.3 ≈ 0.3056
        // k = 0.3056^(1/(0.3-1)) = 0.3056^(-1/0.7)
        let k = ramsey_steady_state(0.3, 0.96, 0.05).unwrap();
        let inner: f64 = (1.0 / 0.96 - 1.0 + 0.05) / 0.3;
        let expected: f64 = inner.powf(1.0 / (0.3 - 1.0));
        assert!(approx(k, expected, 1e-6));
        assert!(k > 0.0);
    }

    #[test]
    fn ramsey_euler_residual_near_zero_at_steady_state() {
        let k = ramsey_steady_state(0.3, 0.96, 0.05).unwrap();
        let resid = ramsey_euler_residual(k, k, 0.96, 0.3, 0.05, 1.0);
        assert!(approx(resid, 0.0, 1e-6));
    }

    #[test]
    fn rbc_simulate_reproducible() {
        let mut k1 = [0.0f64; 100];
        let mut z1 = [0.0f64; 100];
        let mut k2 = [0.0f64; 100];
        let mut z2 = [0.0f64; 100];
        rbc_simulate_into(1.0, 1.0, 0.3, 0.96, 0.05, 0.2, 0.9, 0.01, 100, 42, &mut k1, &mut z1).unwrap();
        rbc_simulate_into(1.0, 1.0, 0.3, 0.96, 0.05, 0.2, 0.9, 0.01, 100, 42, &mut k2, &mut z2).unwrap();
        for t in 0..100 {
            assert_eq!(k1[t], k2[t]);
            assert_eq!(z1[t], z2[t]);
        }
    }

    #[test]
    fn rbc_stays_positive() {
        let mut k = [0.0f64; 50];
        let mut z = [0.0f64; 50];
        rbc_simulate_into(1.0, 1.0, 0.3, 0.96, 0.05, 0.2, 0.9, 0.01, 50, 7, &mut k, &mut z).unwrap();
        for t in 0..50 {
            assert!(k[t] > 0.0, "k[{}] = {}", t, k[t]);
        }
    }

    #[test]
    fn new_keynesian_neutral_returns_zero() {
        // r_prev = 0, r_nat = 0 → y_gap = 0, pi = 0, r = 0
        let (y, pi, r) = new_keynesian_solve(0.0, 0.99, 0.1, 1.0, 1.5, 0.5, 0.5, 0.0).unwrap();
        assert!(approx(y, 0.0, 1e-9));
        assert!(approx(pi, 0.0, 1e-9));
        assert!(approx(r, 0.0, 1e-9));
    }

    #[test]
    fn new_keynesian_positive_natural_rate() {
        // r_nat > 0 → positive output gap, positive inflation, positive r
        let (y, pi, r) = new_keynesian_solve(0.0, 0.99, 0.1, 1.0, 1.5, 0.5, 0.5, 0.02).unwrap();
        assert!(y > 0.0);
        assert!(pi > 0.0);
        assert!(r > 0.0);
    }

    #[test]
    fn olg_steady_state_positive() {
        let (k, y) = olg_steady_state(0.3, 0.96, 0.01).unwrap();
        assert!(k > 0.0);
        assert!(y > 0.0);
    }

    #[test]
    fn invalid_parameters_rejected() {
        assert_eq!(
            solow_steady_state(1.5, 0.3, 0.05, 0.01, 0.02).unwrap_err(),
            MacroError::InvalidInput
        );
        assert_eq!(
            ramsey_steady_state(1.5, 0.96, 0.05).unwrap_err(),
            MacroError::InvalidInput
        );
        assert_eq!(
            olg_steady_state(0.3, 1.5, 0.01).unwrap_err(),
            MacroError::InvalidInput
        );
    }

    #[test]
    fn buffer_too_small() {
        let mut path = [0.0f64; 10];
        let err = solow_simulate_into(1.0, 0.2, 0.3, 0.05, 0.01, 0.02, 20, &mut path).unwrap_err();
        assert_eq!(err, MacroError::BufferTooSmall);
    }
}
