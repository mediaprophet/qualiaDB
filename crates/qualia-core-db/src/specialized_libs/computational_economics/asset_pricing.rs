//! Asset pricing: CAPM, dividend discount model, CCAPM, and Lucas asset
//! pricing.
//!
//! Allocation class: **HotZeroHeap**. No `Vec`/`String`/`Box` in any kernel.
//!
//! Assumptions:
//! - CAPM: single-factor linear model, mean-variance investors, normally
//!   distributed returns.
//! - CCAPM: CRRA utility, log-normal consumption growth.
//! - Lucas: representative agent, complete markets, rational expectations.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetPricingError {
    InvalidInput,
    NonFinite,
    NonConverged,
    BufferTooSmall,
    InsufficientData,
}

fn require_finite(x: f64) -> Result<(), AssetPricingError> {
    if x.is_finite() {
        Ok(())
    } else {
        Err(AssetPricingError::NonFinite)
    }
}

// ---------------------------------------------------------------------------
// CAPM
// ---------------------------------------------------------------------------

/// CAPM expected return: `E[R] = rf + beta * (E[Rm] - rf)`.
///
/// Here `market_premium = E[Rm] - rf`.
pub fn capm_expected_return(
    rf: f64,
    beta: f64,
    market_premium: f64,
) -> Result<f64, AssetPricingError> {
    require_finite(rf)?;
    require_finite(beta)?;
    require_finite(market_premium)?;
    Ok(rf + beta * market_premium)
}

/// CAPM beta: `beta = Cov(Ra, Rm) / Var(Rm)`.
///
/// Zero-heap over caller slices. Returns error if `Var(Rm) == 0`.
pub fn capm_beta(
    asset_returns: &[f64],
    market_returns: &[f64],
) -> Result<f64, AssetPricingError> {
    if asset_returns.len() != market_returns.len() || asset_returns.len() < 2 {
        return Err(AssetPricingError::InsufficientData);
    }
    let n = asset_returns.len() as f64;
    let mut mean_a = 0.0;
    let mut mean_m = 0.0;
    for i in 0..asset_returns.len() {
        if !asset_returns[i].is_finite() || !market_returns[i].is_finite() {
            return Err(AssetPricingError::NonFinite);
        }
        mean_a += asset_returns[i];
        mean_m += market_returns[i];
    }
    mean_a /= n;
    mean_m /= n;
    let mut cov = 0.0;
    let mut var_m = 0.0;
    for i in 0..asset_returns.len() {
        let da = asset_returns[i] - mean_a;
        let dm = market_returns[i] - mean_m;
        cov += da * dm;
        var_m += dm * dm;
    }
    cov /= n;
    var_m /= n;
    if var_m <= 0.0 {
        return Err(AssetPricingError::InvalidInput);
    }
    Ok(cov / var_m)
}

// ---------------------------------------------------------------------------
// Dividend discount model
// ---------------------------------------------------------------------------

/// Gordon growth model: `P = D1 / (r - g)`.
///
/// Refuses if `g >= r` (no finite price).
pub fn gordon_growth_price(
    next_dividend: f64,
    required_return: f64,
    growth_rate: f64,
) -> Result<f64, AssetPricingError> {
    require_finite(next_dividend)?;
    require_finite(required_return)?;
    require_finite(growth_rate)?;
    if next_dividend < 0.0 || required_return <= 0.0 || growth_rate < 0.0 {
        return Err(AssetPricingError::InvalidInput);
    }
    if growth_rate >= required_return {
        return Err(AssetPricingError::InvalidInput);
    }
    Ok(next_dividend / (required_return - growth_rate))
}

/// Multi-period DDM: PV of dividends for `n_periods` + terminal value.
///
/// Terminal value = `D_{n+1} / (r - g)` discounted back `n_periods`. Returns
/// total price.
pub fn multi_period_ddm(
    dividends: &[f64],
    discount_rate: f64,
    terminal_growth: f64,
) -> Result<f64, AssetPricingError> {
    if dividends.is_empty() {
        return Err(AssetPricingError::InsufficientData);
    }
    require_finite(discount_rate)?;
    require_finite(terminal_growth)?;
    if discount_rate <= 0.0 || terminal_growth < 0.0 || terminal_growth >= discount_rate {
        return Err(AssetPricingError::InvalidInput);
    }
    let n = dividends.len();
    let r = 1.0 + discount_rate;
    let mut price = 0.0;
    for (t, d) in dividends.iter().enumerate() {
        require_finite(*d)?;
        if *d < 0.0 {
            return Err(AssetPricingError::InvalidInput);
        }
        price += d / r.powf((t + 1) as f64);
    }
    // Terminal value: D_{n+1} = D_n * (1 + g), TV = D_{n+1} / (r - g)
    let d_next = dividends[n - 1] * (1.0 + terminal_growth);
    let tv = d_next / (discount_rate - terminal_growth);
    price += tv / r.powf(n as f64);
    Ok(price)
}

// ---------------------------------------------------------------------------
// CCAPM
// ---------------------------------------------------------------------------

/// CCAPM equity premium (log-normal approximation):
/// `E[Rm - Rf] = gamma * sigma_c * sigma_m + 0.5 * gamma^2 * sigma_c^2`
///
/// where `sigma_c` is consumption growth volatility and `sigma_m` is market
/// return volatility. This is the Breeden-Lucas consumption-beta form.
pub fn ccapm_equity_premium(
    gamma: f64,
    consumption_growth_std: f64,
    market_return_std: f64,
) -> Result<f64, AssetPricingError> {
    require_finite(gamma)?;
    require_finite(consumption_growth_std)?;
    require_finite(market_return_std)?;
    if gamma <= 0.0 || consumption_growth_std < 0.0 || market_return_std < 0.0 {
        return Err(AssetPricingError::InvalidInput);
    }
    Ok(gamma * consumption_growth_std * market_return_std
        + 0.5 * gamma * gamma * consumption_growth_std * consumption_growth_std)
}

/// CCAPM stochastic discount factor: `m = beta * (c_{t+1}/c_t)^(-gamma)`.
pub fn ccapm_stochastic_discount_factor(
    consumption_growth: f64,
    gamma: f64,
    beta: f64,
) -> Result<f64, AssetPricingError> {
    require_finite(consumption_growth)?;
    require_finite(gamma)?;
    require_finite(beta)?;
    if consumption_growth <= 0.0 || gamma <= 0.0 || !(0.0..=1.0).contains(&beta) {
        return Err(AssetPricingError::InvalidInput);
    }
    Ok(beta * consumption_growth.powf(-gamma))
}

// ---------------------------------------------------------------------------
// Lucas asset pricing
// ---------------------------------------------------------------------------

/// Lucas asset price: `P = E[sum_t beta^t * m_t * d_t]` where
/// `m_t = (c_t / c_0)^(-gamma)`.
///
/// `dividend_paths` and `consumption_paths` are `n_paths x n_periods` row-major.
/// Returns the mean price across paths. Writes per-path prices into `out` if
/// provided (length >= n_paths).
pub fn lucas_asset_price(
    dividend_paths: &[f64],
    consumption_paths: &[f64],
    n_paths: usize,
    n_periods: usize,
    beta: f64,
    gamma: f64,
    mut out: Option<&mut [f64]>,
) -> Result<f64, AssetPricingError> {
    if n_paths == 0 || n_periods == 0 {
        return Err(AssetPricingError::InsufficientData);
    }
    if dividend_paths.len() < n_paths * n_periods || consumption_paths.len() < n_paths * n_periods {
        return Err(AssetPricingError::BufferTooSmall);
    }
    require_finite(beta)?;
    require_finite(gamma)?;
    if !(0.0..=1.0).contains(&beta) || gamma <= 0.0 {
        return Err(AssetPricingError::InvalidInput);
    }
    let mut total = 0.0;
    for p in 0..n_paths {
        let mut price = 0.0;
        let c0 = consumption_paths[p * n_periods];
        if c0 <= 0.0 {
            return Err(AssetPricingError::InvalidInput);
        }
        for t in 0..n_periods {
            let d = dividend_paths[p * n_periods + t];
            let c = consumption_paths[p * n_periods + t];
            if !d.is_finite() || !c.is_finite() || c <= 0.0 {
                return Err(AssetPricingError::NonFinite);
            }
            let m = (c / c0).powf(-gamma);
            price += beta.powi(t as i32) * m * d;
        }
        if let Some(ref mut buf) = out {
            if p < buf.len() {
                buf[p] = price;
            }
        }
        total += price;
    }
    Ok(total / n_paths as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn capm_expected_return_basic() {
        let r = capm_expected_return(0.02, 1.2, 0.05).unwrap();
        assert!(approx(r, 0.08));
    }

    #[test]
    fn capm_beta_unit_correlated() {
        // Asset = market → beta = 1
        let m = [0.01, 0.02, 0.03, 0.04, 0.05];
        let a = [0.01, 0.02, 0.03, 0.04, 0.05];
        let b = capm_beta(&a, &m).unwrap();
        assert!(approx(b, 1.0));
    }

    #[test]
    fn capm_beta_two_times_market() {
        let m = [0.01, 0.02, 0.03, 0.04, 0.05];
        let a = [0.02, 0.04, 0.06, 0.08, 0.10];
        let b = capm_beta(&a, &m).unwrap();
        assert!(approx(b, 2.0));
    }

    #[test]
    fn gordon_growth_basic() {
        // D1=10, r=0.1, g=0.05 → P = 10/0.05 = 200
        let p = gordon_growth_price(10.0, 0.1, 0.05).unwrap();
        assert!(approx(p, 200.0));
    }

    #[test]
    fn gordon_growth_refuses_g_geq_r() {
        assert_eq!(
            gordon_growth_price(10.0, 0.05, 0.05).unwrap_err(),
            AssetPricingError::InvalidInput
        );
        assert_eq!(
            gordon_growth_price(10.0, 0.05, 0.06).unwrap_err(),
            AssetPricingError::InvalidInput
        );
    }

    #[test]
    fn multi_period_ddm_basic() {
        // 2 dividends [10, 10], r=0.1, g=0 → PV = 10/1.1 + 10/1.21 + TV
        // TV = 10*1/(0.1-0) = 100, discounted 2 periods: 100/1.21
        let p = multi_period_ddm(&[10.0, 10.0], 0.1, 0.0).unwrap();
        let expected = 10.0 / 1.1 + 10.0 / 1.21 + 100.0 / 1.21;
        assert!(approx(p, expected));
    }

    #[test]
    fn ccapm_premium_positive() {
        let p = ccapm_equity_premium(2.0, 0.02, 0.16).unwrap();
        assert!(p > 0.0);
    }

    #[test]
    fn ccapm_sdf_decreases_with_growth() {
        let m1 = ccapm_stochastic_discount_factor(1.0, 2.0, 0.99).unwrap();
        let m2 = ccapm_stochastic_discount_factor(1.1, 2.0, 0.99).unwrap();
        assert!(m2 < m1); // higher consumption growth → lower SDF
    }

    #[test]
    fn lucas_price_positive_and_finite() {
        // 1 path, 3 periods. Dividends [1, 1, 1], consumption [1, 1.01, 1.02].
        let d = [1.0, 1.0, 1.0];
        let c = [1.0, 1.01, 1.02];
        let p = lucas_asset_price(&d, &c, 1, 3, 0.99, 2.0, None).unwrap();
        assert!(p > 0.0 && p.is_finite());
    }

    #[test]
    fn lucas_price_writes_per_path() {
        let d = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let c = [1.0, 1.01, 1.02, 1.0, 1.01, 1.02];
        let mut out = [0.0f64; 2];
        let mean = lucas_asset_price(&d, &c, 2, 3, 0.99, 2.0, Some(&mut out)).unwrap();
        assert!(approx(mean, (out[0] + out[1]) / 2.0));
    }

    #[test]
    fn empty_data_rejected() {
        assert_eq!(
            capm_beta(&[], &[]).unwrap_err(),
            AssetPricingError::InsufficientData
        );
    }

    #[test]
    fn invalid_gamma_rejected() {
        assert_eq!(
            ccapm_equity_premium(-1.0, 0.02, 0.16).unwrap_err(),
            AssetPricingError::InvalidInput
        );
    }
}
