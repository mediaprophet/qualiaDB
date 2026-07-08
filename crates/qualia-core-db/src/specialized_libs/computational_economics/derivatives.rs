//! Derivatives pricing kernels.
//!
//! This module contains deterministic, dependency-free option analytics. All
//! inputs are supplied by the caller; no market data, calendars, or allocation
//! are used by the pricing routines.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionKind {
    Call,
    Put,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionStyle {
    European,
    American,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivativesError {
    InvalidInput,
    InvalidSteps,
    NonRecombiningTree,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlackScholesResult {
    pub price: f64,
    pub delta: f64,
    pub gamma: f64,
    pub vega: f64,
    pub theta: f64,
    pub rho: f64,
}

pub const MAX_BINOMIAL_STEPS: usize = 1024;

fn finite_positive(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

fn finite_nonnegative(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}

fn validate_option_inputs(
    spot: f64,
    strike: f64,
    risk_free_rate: f64,
    dividend_yield: f64,
    volatility: f64,
    time_years: f64,
) -> Result<(), DerivativesError> {
    if !finite_positive(spot)
        || !finite_positive(strike)
        || !risk_free_rate.is_finite()
        || !dividend_yield.is_finite()
        || !finite_positive(volatility)
        || !finite_nonnegative(time_years)
    {
        return Err(DerivativesError::InvalidInput);
    }
    Ok(())
}

fn payoff(kind: OptionKind, underlying: f64, strike: f64) -> f64 {
    match kind {
        OptionKind::Call => (underlying - strike).max(0.0),
        OptionKind::Put => (strike - underlying).max(0.0),
    }
}

fn normal_pdf(x: f64) -> f64 {
    const INV_SQRT_2PI: f64 = 0.398_942_280_401_432_7;
    INV_SQRT_2PI * (-0.5 * x * x).exp()
}

/// Standard normal CDF using the Abramowitz-Stegun 7.1.26 approximation.
pub fn normal_cdf(x: f64) -> f64 {
    if x >= 8.0 {
        return 1.0;
    }
    if x <= -8.0 {
        return 0.0;
    }

    let z = x.abs();
    let t = 1.0 / (1.0 + 0.231_641_9 * z);
    let poly = (((((1.330_274_429 * t - 1.821_255_978) * t) + 1.781_477_937) * t - 0.356_563_782)
        * t
        + 0.319_381_530)
        * t;
    let cdf = 1.0 - normal_pdf(z) * poly;
    if x >= 0.0 {
        cdf
    } else {
        1.0 - cdf
    }
}

/// Black-Scholes-Merton option price and Greeks with continuous dividend yield.
///
/// `vega` is per 1.0 absolute volatility change, not per volatility point.
/// `theta` is annualized decay under the standard closed-form convention.
pub fn black_scholes_price_and_greeks(
    kind: OptionKind,
    spot: f64,
    strike: f64,
    risk_free_rate: f64,
    dividend_yield: f64,
    volatility: f64,
    time_years: f64,
) -> Result<BlackScholesResult, DerivativesError> {
    validate_option_inputs(
        spot,
        strike,
        risk_free_rate,
        dividend_yield,
        volatility,
        time_years,
    )?;

    if time_years == 0.0 {
        return Ok(BlackScholesResult {
            price: payoff(kind, spot, strike),
            delta: match kind {
                OptionKind::Call if spot > strike => 1.0,
                OptionKind::Call => 0.0,
                OptionKind::Put if spot < strike => -1.0,
                OptionKind::Put => 0.0,
            },
            gamma: 0.0,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
        });
    }

    let sqrt_t = time_years.sqrt();
    let sigma_sqrt_t = volatility * sqrt_t;
    let d1 = ((spot / strike).ln()
        + (risk_free_rate - dividend_yield + 0.5 * volatility * volatility) * time_years)
        / sigma_sqrt_t;
    let d2 = d1 - sigma_sqrt_t;
    let discount_r = (-risk_free_rate * time_years).exp();
    let discount_q = (-dividend_yield * time_years).exp();
    let nd1 = normal_cdf(d1);
    let nd2 = normal_cdf(d2);
    let pdf_d1 = normal_pdf(d1);

    let common_theta = -(spot * discount_q * pdf_d1 * volatility) / (2.0 * sqrt_t);
    let result = match kind {
        OptionKind::Call => BlackScholesResult {
            price: spot * discount_q * nd1 - strike * discount_r * nd2,
            delta: discount_q * nd1,
            gamma: discount_q * pdf_d1 / (spot * sigma_sqrt_t),
            vega: spot * discount_q * pdf_d1 * sqrt_t,
            theta: common_theta - risk_free_rate * strike * discount_r * nd2
                + dividend_yield * spot * discount_q * nd1,
            rho: strike * time_years * discount_r * nd2,
        },
        OptionKind::Put => {
            let n_minus_d1 = normal_cdf(-d1);
            let n_minus_d2 = normal_cdf(-d2);
            BlackScholesResult {
                price: strike * discount_r * n_minus_d2 - spot * discount_q * n_minus_d1,
                delta: -discount_q * n_minus_d1,
                gamma: discount_q * pdf_d1 / (spot * sigma_sqrt_t),
                vega: spot * discount_q * pdf_d1 * sqrt_t,
                theta: common_theta + risk_free_rate * strike * discount_r * n_minus_d2
                    - dividend_yield * spot * discount_q * n_minus_d1,
                rho: -strike * time_years * discount_r * n_minus_d2,
            }
        }
    };

    if result.price.is_finite()
        && result.delta.is_finite()
        && result.gamma.is_finite()
        && result.vega.is_finite()
        && result.theta.is_finite()
        && result.rho.is_finite()
    {
        Ok(result)
    } else {
        Err(DerivativesError::InvalidInput)
    }
}

/// Returns the put-call parity residual:
/// `call - put - (spot * exp(-qT) - strike * exp(-rT))`.
pub fn put_call_parity(
    call_price: f64,
    put_price: f64,
    spot: f64,
    strike: f64,
    risk_free_rate: f64,
    dividend_yield: f64,
    time_years: f64,
) -> Result<f64, DerivativesError> {
    if !call_price.is_finite()
        || !put_price.is_finite()
        || !finite_positive(spot)
        || !finite_positive(strike)
        || !risk_free_rate.is_finite()
        || !dividend_yield.is_finite()
        || !finite_nonnegative(time_years)
    {
        return Err(DerivativesError::InvalidInput);
    }

    Ok(call_price
        - put_price
        - (spot * (-dividend_yield * time_years).exp()
            - strike * (-risk_free_rate * time_years).exp()))
}

pub fn parity_implied_call_price(
    put_price: f64,
    spot: f64,
    strike: f64,
    risk_free_rate: f64,
    dividend_yield: f64,
    time_years: f64,
) -> Result<f64, DerivativesError> {
    if !put_price.is_finite()
        || !finite_positive(spot)
        || !finite_positive(strike)
        || !risk_free_rate.is_finite()
        || !dividend_yield.is_finite()
        || !finite_nonnegative(time_years)
    {
        return Err(DerivativesError::InvalidInput);
    }
    Ok(put_price + spot * (-dividend_yield * time_years).exp()
        - strike * (-risk_free_rate * time_years).exp())
}

pub fn parity_implied_put_price(
    call_price: f64,
    spot: f64,
    strike: f64,
    risk_free_rate: f64,
    dividend_yield: f64,
    time_years: f64,
) -> Result<f64, DerivativesError> {
    if !call_price.is_finite()
        || !finite_positive(spot)
        || !finite_positive(strike)
        || !risk_free_rate.is_finite()
        || !dividend_yield.is_finite()
        || !finite_nonnegative(time_years)
    {
        return Err(DerivativesError::InvalidInput);
    }
    Ok(call_price - spot * (-dividend_yield * time_years).exp()
        + strike * (-risk_free_rate * time_years).exp())
}

/// Cox-Ross-Rubinstein binomial option price.
///
/// `steps` must be in `1..=MAX_BINOMIAL_STEPS`. The implementation uses a
/// fixed stack buffer and backward induction, so American early exercise is
/// evaluated at every node without allocating.
pub fn binomial_option_price(
    kind: OptionKind,
    style: OptionStyle,
    spot: f64,
    strike: f64,
    risk_free_rate: f64,
    dividend_yield: f64,
    volatility: f64,
    time_years: f64,
    steps: usize,
) -> Result<f64, DerivativesError> {
    validate_option_inputs(
        spot,
        strike,
        risk_free_rate,
        dividend_yield,
        volatility,
        time_years,
    )?;
    if steps == 0 || steps > MAX_BINOMIAL_STEPS {
        return Err(DerivativesError::InvalidSteps);
    }
    if time_years == 0.0 {
        return Ok(payoff(kind, spot, strike));
    }

    let dt = time_years / steps as f64;
    let sqrt_dt = dt.sqrt();
    let up = (volatility * sqrt_dt).exp();
    let down = 1.0 / up;
    let growth = ((risk_free_rate - dividend_yield) * dt).exp();
    let denom = up - down;
    if denom <= 0.0 || !denom.is_finite() {
        return Err(DerivativesError::NonRecombiningTree);
    }
    let p = (growth - down) / denom;
    if !p.is_finite() || !(0.0..=1.0).contains(&p) {
        return Err(DerivativesError::NonRecombiningTree);
    }

    let discount = (-risk_free_rate * dt).exp();
    let ratio = up / down;
    let mut values = [0.0_f64; MAX_BINOMIAL_STEPS + 1];

    let mut underlying = spot * down.powi(steps as i32);
    for slot in values.iter_mut().take(steps + 1) {
        *slot = payoff(kind, underlying, strike);
        underlying *= ratio;
    }

    for step in (0..steps).rev() {
        let mut node_underlying = spot * down.powi(step as i32);
        for node in 0..=step {
            let continuation = discount * (p * values[node + 1] + (1.0 - p) * values[node]);
            values[node] = match style {
                OptionStyle::European => continuation,
                OptionStyle::American => continuation.max(payoff(kind, node_underlying, strike)),
            };
            node_underlying *= ratio;
        }
    }

    if values[0].is_finite() {
        Ok(values[0])
    } else {
        Err(DerivativesError::InvalidInput)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn black_scholes_put_call_parity_residual_is_small() {
        let call =
            black_scholes_price_and_greeks(OptionKind::Call, 100.0, 100.0, 0.05, 0.01, 0.2, 1.0)
                .unwrap();
        let put =
            black_scholes_price_and_greeks(OptionKind::Put, 100.0, 100.0, 0.05, 0.01, 0.2, 1.0)
                .unwrap();

        let residual =
            put_call_parity(call.price, put.price, 100.0, 100.0, 0.05, 0.01, 1.0).unwrap();
        assert!(residual.abs() < 1e-6);
    }

    #[test]
    fn black_scholes_atm_call_is_in_known_bounds() {
        let result =
            black_scholes_price_and_greeks(OptionKind::Call, 100.0, 100.0, 0.05, 0.0, 0.2, 1.0)
                .unwrap();

        assert!(result.price > 10.40 && result.price < 10.47);
        assert!(result.gamma > 0.018 && result.gamma < 0.020);
        assert!(result.vega > 37.0 && result.vega < 38.0);
    }

    #[test]
    fn deltas_have_expected_signs() {
        let call =
            black_scholes_price_and_greeks(OptionKind::Call, 100.0, 105.0, 0.03, 0.0, 0.25, 0.75)
                .unwrap();
        let put =
            black_scholes_price_and_greeks(OptionKind::Put, 100.0, 105.0, 0.03, 0.0, 0.25, 0.75)
                .unwrap();

        assert!(call.delta > 0.0 && call.delta < 1.0);
        assert!(put.delta < 0.0 && put.delta > -1.0);
    }

    #[test]
    fn american_put_is_at_least_european_put_in_binomial_tree() {
        let european = binomial_option_price(
            OptionKind::Put,
            OptionStyle::European,
            100.0,
            105.0,
            0.04,
            0.0,
            0.25,
            1.0,
            256,
        )
        .unwrap();
        let american = binomial_option_price(
            OptionKind::Put,
            OptionStyle::American,
            100.0,
            105.0,
            0.04,
            0.0,
            0.25,
            1.0,
            256,
        )
        .unwrap();

        assert!(american >= european);
    }

    #[test]
    fn binomial_european_call_converges_near_black_scholes() {
        let bs =
            black_scholes_price_and_greeks(OptionKind::Call, 100.0, 100.0, 0.05, 0.0, 0.2, 1.0)
                .unwrap();
        let tree = binomial_option_price(
            OptionKind::Call,
            OptionStyle::European,
            100.0,
            100.0,
            0.05,
            0.0,
            0.2,
            1.0,
            512,
        )
        .unwrap();

        assert!(approx_eq(tree, bs.price, 0.05));
    }

    #[test]
    fn rejects_invalid_volatility_and_steps() {
        let bad_vol =
            black_scholes_price_and_greeks(OptionKind::Call, 100.0, 100.0, 0.05, 0.0, 0.0, 1.0);
        assert_eq!(bad_vol, Err(DerivativesError::InvalidInput));

        let bad_steps = binomial_option_price(
            OptionKind::Call,
            OptionStyle::European,
            100.0,
            100.0,
            0.05,
            0.0,
            0.2,
            1.0,
            0,
        );
        assert_eq!(bad_steps, Err(DerivativesError::InvalidSteps));
    }
}
