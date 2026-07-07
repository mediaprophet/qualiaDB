//! Real return-based portfolio risk metrics.
//!
//! This is the genuine computation behind `RiskAnalyzer::calculate_risk_metrics`
//! (`super::RiskAnalyzer`). It replaces the prior stub, which — before being made
//! to refuse — returned a default `RiskMetrics` with every field fabricated
//! (Sharpe 0.75, a hardcoded VaR/volatility/drawdown) regardless of the portfolio.
//!
//! The metrics are ordinary sample statistics of the portfolio's **return time
//! series**, which is built from each `Asset`'s `price_history`:
//!
//! 1. Per-asset simple returns `r_{i,t} = (p_{i,t} − p_{i,t-1}) / p_{i,t-1}`.
//! 2. Value weights `w_i = market_value_i / Σ market_value` over the assets that
//!    carry an (equal-length) history.
//! 3. Portfolio return `R_t = Σ_i w_i · r_{i,t}`.
//!
//! From `R` it computes (all genuine, all tested against hand calculation):
//! - **volatility** — sample standard deviation of `R` (via the engine's
//!   `solvers::statistics::descriptive`, Modality-First Composition).
//! - **VaR(95%)** and **CVaR(95%)** — historical: the 5th-percentile loss and the
//!   mean of the losses beyond it (expected shortfall).
//! - **Sharpe** = `(mean R − r_f)/volatility`, **Sortino** = `(mean R − r_f)/downside
//!   deviation`, with `r_f = 0` per period (a stated, standard simplification).
//! - **max drawdown** — the largest peak-to-trough decline of the cumulative
//!   wealth path `∏(1+R_t)`.
//!
//! Honesty boundaries (never fabricated):
//! - No usable price history (need ≥ 3 prices ⇒ ≥ 2 returns) ⇒ `InsufficientData`.
//! - Misaligned histories (different lengths, no dates to align on) ⇒ `InsufficientData`.
//! - **beta** and **alpha** are reported as `NaN` when no benchmark return series
//!   is supplied, because they are defined only relative to one. When a benchmark
//!   series is supplied (via the optional `benchmark_returns` argument), beta is
//!   `Cov(R_p, R_b)/Var(R_b)` and alpha is `mean(R_p) − beta·mean(R_b)`. `NaN` (in
//!   the no-benchmark case) is an unmistakable "not computed", not a plausible fake.

use super::{FinancialError, Portfolio, RiskMetrics};
use crate::solvers::statistics::descriptive;

/// Per-period risk-free rate used for Sharpe/Sortino. Zero is a standard, stated
/// simplification (the metrics then measure excess return over a zero baseline).
const RISK_FREE: f64 = 0.0;

/// Simple returns from a price series (oldest first): `(p[t] − p[t-1])/p[t-1]`.
fn returns_from_prices(prices: &[f64]) -> Vec<f64> {
    let mut r = Vec::with_capacity(prices.len().saturating_sub(1));
    for t in 1..prices.len() {
        let prev = prices[t - 1];
        if prev != 0.0 {
            r.push((prices[t] - prev) / prev);
        } else {
            r.push(0.0);
        }
    }
    r
}

/// Linear-interpolated quantile of an ascending-sorted slice (numpy "linear").
fn quantile_sorted(sorted: &[f64], q: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return f64::NAN;
    }
    if n == 1 {
        return sorted[0];
    }
    let pos = q * (n - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    let frac = pos - lo as f64;
    sorted[lo] + (sorted[hi] - sorted[lo]) * frac
}

/// Build the value-weighted portfolio return series from the portfolio's assets.
fn portfolio_returns(portfolio: &Portfolio) -> Result<Vec<f64>, FinancialError> {
    // Assets that carry at least two prices (⇒ at least one return).
    let with_history: Vec<&super::Asset> = portfolio
        .assets
        .iter()
        .filter(|a| a.price_history.len() >= 2)
        .collect();
    if with_history.is_empty() {
        return Err(FinancialError::InsufficientData(
            "portfolio risk: no asset carries a price history (need ≥ 3 prices per asset for ≥ 2 \
             returns); refusing to fabricate risk numbers."
                .to_string(),
        ));
    }

    // Histories must be aligned (same length) — without dates we cannot align
    // mismatched series, so refuse rather than silently mis-pair returns.
    let t_len = with_history[0].price_history.len();
    if with_history.iter().any(|a| a.price_history.len() != t_len) {
        return Err(FinancialError::InsufficientData(
            "portfolio risk: asset price histories have different lengths and carry no dates to \
             align on; provide equal-length aligned histories."
                .to_string(),
        ));
    }
    if t_len < 3 {
        return Err(FinancialError::InsufficientData(
            "portfolio risk: need at least 3 prices per asset (≥ 2 returns) to compute sample \
             statistics."
                .to_string(),
        ));
    }

    // Value weights over the included assets (renormalised to sum to 1).
    let total_value: f64 = with_history.iter().map(|a| a.market_value).sum();
    if !(total_value > 0.0) {
        return Err(FinancialError::InsufficientData(
            "portfolio risk: total market value of assets with history is not positive; cannot \
             form portfolio weights."
                .to_string(),
        ));
    }

    let n_returns = t_len - 1;
    let mut portfolio_r = vec![0.0; n_returns];
    for asset in &with_history {
        let w = asset.market_value / total_value;
        let r = returns_from_prices(&asset.price_history);
        for t in 0..n_returns {
            portfolio_r[t] += w * r[t];
        }
    }
    Ok(portfolio_r)
}

/// Compute the real risk metrics for `portfolio`. See the module docs for the
/// definitions and the honesty boundaries.
///
/// `benchmark_returns`, when supplied, enables genuine beta/alpha:
/// - `beta  = Cov(R_p, R_b) / Var(R_b)`
/// - `alpha = mean(R_p) − beta · mean(R_b)`
///
/// The benchmark series must be the same length as the portfolio return series
/// (i.e. one benchmark return per period). A length mismatch, or zero benchmark
/// variance, leaves beta/alpha as `NaN` (undefined) rather than fabricating them.
pub fn compute_risk_metrics(
    portfolio: &Portfolio,
    benchmark_returns: Option<&[f64]>,
) -> Result<RiskMetrics, FinancialError> {
    let r = portfolio_returns(portfolio)?;
    let n = r.len();

    let mean = descriptive::mean(&r).unwrap_or(0.0);
    let volatility = descriptive::std_dev(&r, true).unwrap_or(0.0);

    // Historical VaR / CVaR at 95%.
    let mut sorted = r.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let q05 = quantile_sorted(&sorted, 0.05);
    let var_95 = -q05; // express a loss as a positive number
    let tail: Vec<f64> = sorted.iter().copied().filter(|&x| x <= q05).collect();
    let cvar_95 = if tail.is_empty() {
        -sorted[0]
    } else {
        -descriptive::mean(&tail).unwrap_or(sorted[0])
    };

    // Sharpe and Sortino (per period, r_f = 0).
    let sharpe_ratio = if volatility > 0.0 {
        (mean - RISK_FREE) / volatility
    } else {
        0.0
    };
    let downside_var = r
        .iter()
        .map(|&x| {
            let d = (x - RISK_FREE).min(0.0);
            d * d
        })
        .sum::<f64>()
        / n as f64;
    let downside_dev = downside_var.sqrt();
    let sortino_ratio = if downside_dev > 0.0 {
        (mean - RISK_FREE) / downside_dev
    } else {
        0.0
    };

    // Maximum drawdown of the cumulative wealth path ∏(1+R_t).
    let mut wealth = 1.0;
    let mut peak = 1.0;
    let mut max_drawdown = 0.0;
    for &x in &r {
        wealth *= 1.0 + x;
        if wealth > peak {
            peak = wealth;
        }
        if peak > 0.0 {
            let dd = (peak - wealth) / peak;
            if dd > max_drawdown {
                max_drawdown = dd;
            }
        }
    }

    // Beta / alpha against an optional benchmark return series. Defined only
    // when the benchmark is supplied, length-aligned to the portfolio returns,
    // and has positive variance. Otherwise NaN (never a fabricated 1.0/0.0).
    let (beta, alpha) = match benchmark_returns {
        Some(br) if br.len() == n => {
            let var_b = descriptive::variance(br, true).unwrap_or(0.0);
            if var_b > 0.0 {
                let cov = descriptive::covariance(&r, br, true).unwrap_or(0.0);
                let b = cov / var_b;
                let mean_b = descriptive::mean(br).unwrap_or(0.0);
                let a = mean - b * mean_b;
                (b, a)
            } else {
                (f64::NAN, f64::NAN)
            }
        }
        _ => (f64::NAN, f64::NAN),
    };

    Ok(RiskMetrics {
        portfolio_id: portfolio.portfolio_id.clone(),
        var_95,
        cvar_95,
        volatility,
        beta,
        alpha,
        sharpe_ratio,
        sortino_ratio,
        max_drawdown,
        // Headline single-number risk: the 95% historical VaR (a real, defined
        // loss fraction), used as the operation's risk_score.
        overall_risk_score: var_95,
        // Filled in by the caller (RiskAnalyzer) against the portfolio's declared
        // risk profile; the raw computation has no opinion on tolerance fit.
        risk_profile_assessment: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::financial_modeling::{
        Asset, AssetType, InvestmentStrategy, LiquidityNeeds, Portfolio, RiskProfile,
        RiskTolerance, TimeHorizon,
    };

    fn asset(symbol: &str, market_value: f64, prices: Vec<f64>) -> Asset {
        Asset {
            asset_id: symbol.to_string(),
            symbol: symbol.to_string(),
            asset_type: AssetType::Stock,
            quantity: 1.0,
            average_cost: 0.0,
            current_price: *prices.last().unwrap_or(&0.0),
            market_value,
            currency: "USD".to_string(),
            exchange: "TEST".to_string(),
            last_updated: 0,
            price_history: prices,
        }
    }

    fn portfolio(assets: Vec<Asset>) -> Portfolio {
        Portfolio {
            portfolio_id: "p".to_string(),
            portfolio_name: "p".to_string(),
            owner_id: "o".to_string(),
            assets,
            cash_balance: 0.0,
            total_value: 0.0,
            created_at: 0,
            last_updated: 0,
            risk_profile: RiskProfile {
                risk_tolerance: RiskTolerance::Moderate,
                risk_capacity: 0.0,
                time_horizon: TimeHorizon::LongTerm,
                liquidity_needs: LiquidityNeeds::Low,
            },
            investment_strategy: InvestmentStrategy::Balanced,
        }
    }

    #[test]
    fn single_asset_matches_hand_computation() {
        // prices 100→110→99→108.9 ⇒ returns 0.1, -0.1, 0.1.
        let p = portfolio(vec![asset("A", 1000.0, vec![100.0, 110.0, 99.0, 108.9])]);
        let m = compute_risk_metrics(&p, None).unwrap();

        // mean = 0.1/3; sample stddev computed by hand below.
        assert!(
            (m.volatility - 0.11547005).abs() < 1e-6,
            "vol {}",
            m.volatility
        );
        // Sharpe = mean/vol with rf=0.
        let mean = 0.1 / 3.0;
        assert!((m.sharpe_ratio - mean / 0.11547005).abs() < 1e-5);
        // Wealth path 1→1.1→0.99→1.089: peak 1.1, trough 0.99 ⇒ DD = 0.1.
        assert!((m.max_drawdown - 0.1).abs() < 1e-9, "dd {}", m.max_drawdown);
        // VaR/CVaR are real numbers (the worst return is -0.1 ⇒ positive loss).
        assert!(m.var_95 > 0.0 && m.cvar_95 > 0.0);
        // beta/alpha are explicitly not computed without a benchmark.
        assert!(m.beta.is_nan() && m.alpha.is_nan());
    }

    #[test]
    fn value_weighting_blends_drawdown_between_components() {
        // Asset A rises monotonically (no drawdown); asset B falls monotonically
        // (real drawdown). A 50/50 value blend's drawdown must sit strictly between.
        let a = asset("A", 500.0, vec![100.0, 101.0, 102.0, 103.0]);
        let b = asset("B", 500.0, vec![100.0, 99.0, 98.0, 97.0]);
        let blended = compute_risk_metrics(&portfolio(vec![a.clone(), b.clone()]), None).unwrap();
        let only_a = compute_risk_metrics(&portfolio(vec![a]), None).unwrap();
        let only_b = compute_risk_metrics(&portfolio(vec![b]), None).unwrap();
        assert!(
            only_a.max_drawdown < 1e-12,
            "rising asset should have no drawdown: {}",
            only_a.max_drawdown
        );
        assert!(
            only_b.max_drawdown > 0.0,
            "falling asset must have drawdown"
        );
        assert!(
            blended.max_drawdown > 0.0 && blended.max_drawdown < only_b.max_drawdown,
            "blend {} should sit between {} and {}",
            blended.max_drawdown,
            only_a.max_drawdown,
            only_b.max_drawdown
        );
    }

    #[test]
    fn refuses_without_history() {
        let mut a = asset("A", 1000.0, Vec::new());
        a.price_history.clear();
        let r = compute_risk_metrics(&portfolio(vec![a]), None);
        assert!(matches!(r, Err(FinancialError::InsufficientData(_))));
    }

    #[test]
    fn refuses_misaligned_histories() {
        let a = asset("A", 500.0, vec![100.0, 101.0, 102.0, 103.0]);
        let b = asset("B", 500.0, vec![100.0, 99.0]); // shorter
        let r = compute_risk_metrics(&portfolio(vec![a, b]), None);
        assert!(matches!(r, Err(FinancialError::InsufficientData(_))));
    }

    #[test]
    fn refuses_too_short_history() {
        let a = asset("A", 1000.0, vec![100.0, 101.0]); // only 1 return
        let r = compute_risk_metrics(&portfolio(vec![a]), None);
        assert!(matches!(r, Err(FinancialError::InsufficientData(_))));
    }

    #[test]
    fn benchmark_supplies_real_beta_and_alpha() {
        // Portfolio returns (from prices 100→110→99→108.9): 0.1, -0.1, 0.1.
        let p = portfolio(vec![asset("A", 1000.0, vec![100.0, 110.0, 99.0, 108.9])]);
        // Benchmark returns aligned to the same 3 periods — same sign pattern as
        // the portfolio but half the magnitude ⇒ beta = 2.0, alpha ≈ 0.0.
        let benchmark = [0.05, -0.05, 0.05];

        let m = compute_risk_metrics(&p, Some(&benchmark)).unwrap();

        // beta/alpha must now be real numbers, not NaN.
        assert!(!m.beta.is_nan(), "beta should be computed with a benchmark");
        assert!(
            !m.alpha.is_nan(),
            "alpha should be computed with a benchmark"
        );
        assert!((m.beta - 2.0).abs() < 1e-9, "beta {} should be 2.0", m.beta);
        assert!(m.alpha.abs() < 1e-9, "alpha {} should be ~0.0", m.alpha);

        // Mismatched benchmark length ⇒ beta/alpha stay NaN (no fabrication).
        let m_mis = compute_risk_metrics(&p, Some(&[0.05, 0.05])).unwrap();
        assert!(m_mis.beta.is_nan() && m_mis.alpha.is_nan());

        // No benchmark ⇒ beta/alpha stay NaN (the honesty boundary).
        let m_none = compute_risk_metrics(&p, None).unwrap();
        assert!(m_none.beta.is_nan() && m_none.alpha.is_nan());
    }
}
