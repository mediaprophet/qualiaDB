//! Yield-curve primitives.
//!
//! Curves are represented as caller-owned slices of zero-rate points. The
//! functions here do not fetch market data and do not allocate; they transform
//! supplied rates into discount factors, forwards, par yields, and bootstrapped
//! zero curves.

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct CurvePoint {
    pub time_years: f64,
    pub zero_rate: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YieldCurveError {
    InvalidInput,
    InvalidFrequency,
    OutputBufferTooSmall,
}

fn finite_nonnegative(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}

fn validate_curve(points: &[CurvePoint]) -> Result<(), YieldCurveError> {
    if points.is_empty() {
        return Err(YieldCurveError::InvalidInput);
    }

    let mut prev_time = -1.0;
    for point in points {
        if !finite_nonnegative(point.time_years) || !point.zero_rate.is_finite() {
            return Err(YieldCurveError::InvalidInput);
        }
        if point.time_years <= prev_time {
            return Err(YieldCurveError::InvalidInput);
        }
        prev_time = point.time_years;
    }
    Ok(())
}

fn discount_from_zero_rate(
    zero_rate: f64,
    time_years: f64,
    compounding_per_year: u32,
) -> Result<f64, YieldCurveError> {
    if !zero_rate.is_finite() || !finite_nonnegative(time_years) {
        return Err(YieldCurveError::InvalidInput);
    }
    if compounding_per_year == 0 {
        return Err(YieldCurveError::InvalidFrequency);
    }
    let f = compounding_per_year as f64;
    let base = 1.0 + zero_rate / f;
    if base <= 0.0 {
        return Err(YieldCurveError::InvalidInput);
    }
    Ok(base.powf(-f * time_years))
}

fn zero_rate_from_discount(
    discount_factor: f64,
    time_years: f64,
    compounding_per_year: u32,
) -> Result<f64, YieldCurveError> {
    if !discount_factor.is_finite()
        || discount_factor <= 0.0
        || !finite_nonnegative(time_years)
        || time_years == 0.0
    {
        return Err(YieldCurveError::InvalidInput);
    }
    if compounding_per_year == 0 {
        return Err(YieldCurveError::InvalidFrequency);
    }
    let f = compounding_per_year as f64;
    Ok(f * (discount_factor.powf(-1.0 / (f * time_years)) - 1.0))
}

/// Linearly interpolate a zero rate. Endpoints use flat extrapolation.
pub fn interpolate_zero_rate(
    points: &[CurvePoint],
    time_years: f64,
) -> Result<f64, YieldCurveError> {
    validate_curve(points)?;
    if !finite_nonnegative(time_years) {
        return Err(YieldCurveError::InvalidInput);
    }

    if time_years <= points[0].time_years {
        return Ok(points[0].zero_rate);
    }

    for pair in points.windows(2) {
        let a = pair[0];
        let b = pair[1];
        if time_years <= b.time_years {
            let width = b.time_years - a.time_years;
            let weight = (time_years - a.time_years) / width;
            return Ok(a.zero_rate + weight * (b.zero_rate - a.zero_rate));
        }
    }

    Ok(points[points.len() - 1].zero_rate)
}

/// Discount factor at `time_years` from an interpolated zero curve.
pub fn discount_factor_from_curve(
    points: &[CurvePoint],
    time_years: f64,
    compounding_per_year: u32,
) -> Result<f64, YieldCurveError> {
    let zero_rate = interpolate_zero_rate(points, time_years)?;
    discount_from_zero_rate(zero_rate, time_years, compounding_per_year)
}

/// Annualized forward rate implied between two maturities.
pub fn annualized_forward_rate(
    points: &[CurvePoint],
    start_years: f64,
    end_years: f64,
    compounding_per_year: u32,
) -> Result<f64, YieldCurveError> {
    if !finite_nonnegative(start_years) || end_years <= start_years {
        return Err(YieldCurveError::InvalidInput);
    }
    let start_df = discount_factor_from_curve(points, start_years, compounding_per_year)?;
    let end_df = discount_factor_from_curve(points, end_years, compounding_per_year)?;
    if start_df <= 0.0 || end_df <= 0.0 {
        return Err(YieldCurveError::InvalidInput);
    }
    Ok((start_df / end_df).powf(1.0 / (end_years - start_years)) - 1.0)
}

/// Par coupon rate for a regular fixed-rate bond priced from a zero curve.
pub fn par_yield_from_zero_curve(
    points: &[CurvePoint],
    maturity_years: f64,
    coupon_frequency: u32,
) -> Result<f64, YieldCurveError> {
    validate_curve(points)?;
    if !finite_nonnegative(maturity_years) || maturity_years == 0.0 {
        return Err(YieldCurveError::InvalidInput);
    }
    if coupon_frequency == 0 {
        return Err(YieldCurveError::InvalidFrequency);
    }

    let periods_f = maturity_years * coupon_frequency as f64;
    let periods = periods_f.round() as u32;
    if periods == 0 || (periods as f64 - periods_f).abs() > 1e-9 {
        return Err(YieldCurveError::InvalidInput);
    }

    let mut annuity = 0.0;
    for period in 1..=periods {
        let t = period as f64 / coupon_frequency as f64;
        annuity += discount_factor_from_curve(points, t, coupon_frequency)?;
    }
    let maturity_df = discount_factor_from_curve(points, maturity_years, coupon_frequency)?;
    if annuity <= 0.0 {
        return Err(YieldCurveError::InvalidInput);
    }
    Ok(coupon_frequency as f64 * (1.0 - maturity_df) / annuity)
}

/// Bootstrap periodic-compounded zero rates from regular par coupon yields.
///
/// `par_yields[i].time_years` is the instrument maturity and
/// `par_yields[i].zero_rate` carries the par coupon yield for that maturity.
/// Coupon dates before each maturity must be covered by earlier bootstrapped
/// maturities, directly or by interpolation.
pub fn bootstrap_zero_curve_from_par_yields(
    par_yields: &[CurvePoint],
    coupon_frequency: u32,
    out: &mut [CurvePoint],
) -> Result<usize, YieldCurveError> {
    validate_curve(par_yields)?;
    if coupon_frequency == 0 {
        return Err(YieldCurveError::InvalidFrequency);
    }
    if out.len() < par_yields.len() {
        return Err(YieldCurveError::OutputBufferTooSmall);
    }

    let mut written = 0usize;
    for point in par_yields {
        let maturity = point.time_years;
        let par_yield = point.zero_rate;
        let periods_f = maturity * coupon_frequency as f64;
        let periods = periods_f.round() as u32;
        if periods == 0 || (periods as f64 - periods_f).abs() > 1e-9 {
            return Err(YieldCurveError::InvalidInput);
        }

        let coupon = par_yield / coupon_frequency as f64;
        if 1.0 + coupon <= 0.0 {
            return Err(YieldCurveError::InvalidInput);
        }

        let mut previous_coupon_pv = 0.0;
        for period in 1..periods {
            if written == 0 {
                return Err(YieldCurveError::InvalidInput);
            }
            let t = period as f64 / coupon_frequency as f64;
            let df = discount_factor_from_curve(&out[..written], t, coupon_frequency)?;
            previous_coupon_pv += coupon * df;
        }

        let maturity_df = (1.0 - previous_coupon_pv) / (1.0 + coupon);
        let zero_rate = zero_rate_from_discount(maturity_df, maturity, coupon_frequency)?;
        out[written] = CurvePoint {
            time_years: maturity,
            zero_rate,
        };
        written += 1;
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(time_years: f64, zero_rate: f64) -> CurvePoint {
        CurvePoint {
            time_years,
            zero_rate,
        }
    }

    #[test]
    fn interpolation_is_linear_between_points() {
        let curve = [point(1.0, 0.02), point(3.0, 0.06)];
        let rate = interpolate_zero_rate(&curve, 2.0).unwrap();
        assert!((rate - 0.04).abs() < 1e-12);
    }

    #[test]
    fn curve_discount_factor_uses_interpolated_zero_rate() {
        let curve = [point(1.0, 0.04), point(2.0, 0.04)];
        let df = discount_factor_from_curve(&curve, 2.0, 2).unwrap();
        let expected = 1.0 / 1.02f64.powi(4);
        assert!((df - expected).abs() < 1e-12);
    }

    #[test]
    fn forward_rate_is_positive_for_upward_curve() {
        let curve = [point(1.0, 0.03), point(2.0, 0.05)];
        let fwd = annualized_forward_rate(&curve, 1.0, 2.0, 2).unwrap();
        assert!(fwd > 0.05);
    }

    #[test]
    fn par_yield_roundtrips_flat_curve() {
        let curve = [
            point(0.5, 0.05),
            point(1.0, 0.05),
            point(1.5, 0.05),
            point(2.0, 0.05),
        ];
        let par = par_yield_from_zero_curve(&curve, 2.0, 2).unwrap();
        assert!((par - 0.05).abs() < 1e-12);
    }

    #[test]
    fn bootstrap_flat_par_yields_to_flat_zero_curve() {
        let par = [
            point(0.5, 0.05),
            point(1.0, 0.05),
            point(1.5, 0.05),
            point(2.0, 0.05),
        ];
        let mut out = [point(0.0, 0.0); 4];
        let n = bootstrap_zero_curve_from_par_yields(&par, 2, &mut out).unwrap();
        assert_eq!(n, 4);
        for bootstrapped in &out[..n] {
            assert!((bootstrapped.zero_rate - 0.05).abs() < 1e-10);
        }
    }

    #[test]
    fn bootstrap_rejects_missing_first_coupon_date() {
        let par = [point(1.0, 0.05)];
        let mut out = [point(0.0, 0.0); 1];
        assert_eq!(
            bootstrap_zero_curve_from_par_yields(&par, 2, &mut out),
            Err(YieldCurveError::InvalidInput)
        );
    }

    #[test]
    fn non_monotonic_curve_is_rejected() {
        let curve = [point(1.0, 0.02), point(1.0, 0.03)];
        assert_eq!(
            interpolate_zero_rate(&curve, 1.0),
            Err(YieldCurveError::InvalidInput)
        );
    }
}
