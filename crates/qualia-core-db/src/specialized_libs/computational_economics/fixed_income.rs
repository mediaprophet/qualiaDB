//! Fixed-income primitives.
//!
//! These are pure numerical kernels: no market-data lookup, no tax advice, no
//! calendars beyond simple serial-day year fractions. They are intentionally
//! small and caller-buffer friendly so later bond/loan/swap modules can compose
//! them instead of reimplementing discounting math.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayCountConvention {
    Act360,
    Act365,
    Thirty360,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedIncomeError {
    InvalidInput,
    InvalidFrequency,
    NonConvergent,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CashFlow {
    /// Time in years from valuation date.
    pub time_years: f64,
    /// Cash amount paid at `time_years`.
    pub amount: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BondMetrics {
    pub price: f64,
    pub macaulay_duration: f64,
    pub modified_duration: f64,
    pub convexity: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AccruedInterest {
    pub accrued: f64,
    pub accrual_fraction: f64,
    pub coupon_period_fraction: f64,
}

fn finite_nonnegative(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}

/// Year fraction between two serial day numbers. `Thirty360` uses a simple
/// 30/360 convention over serial days by truncating each month to 30 days.
pub fn year_fraction(
    start_day: u32,
    end_day: u32,
    convention: DayCountConvention,
) -> Result<f64, FixedIncomeError> {
    if end_day < start_day {
        return Err(FixedIncomeError::InvalidInput);
    }
    let days = (end_day - start_day) as f64;
    let yf = match convention {
        DayCountConvention::Act360 => days / 360.0,
        DayCountConvention::Act365 => days / 365.0,
        DayCountConvention::Thirty360 => {
            let sy = (start_day / 360) as i64;
            let sm = ((start_day % 360) / 30) as i64;
            let sd = (start_day % 30) as i64;
            let ey = (end_day / 360) as i64;
            let em = ((end_day % 360) / 30) as i64;
            let ed = (end_day % 30) as i64;
            (360 * (ey - sy) + 30 * (em - sm) + (ed - sd)) as f64 / 360.0
        }
    };
    Ok(yf)
}

/// Periodic-compounded discount factor `(1 + r/f)^(-f*t)`.
pub fn discount_factor(
    rate: f64,
    time_years: f64,
    compounding_per_year: u32,
) -> Result<f64, FixedIncomeError> {
    if !rate.is_finite() || !finite_nonnegative(time_years) {
        return Err(FixedIncomeError::InvalidInput);
    }
    if compounding_per_year == 0 {
        return Err(FixedIncomeError::InvalidFrequency);
    }
    let f = compounding_per_year as f64;
    let base = 1.0 + rate / f;
    if base <= 0.0 {
        return Err(FixedIncomeError::InvalidInput);
    }
    Ok(base.powf(-f * time_years))
}

/// Continuous-compounded discount factor `exp(-r*t)`.
pub fn discount_factor_continuous(rate: f64, time_years: f64) -> Result<f64, FixedIncomeError> {
    if !rate.is_finite() || !finite_nonnegative(time_years) {
        return Err(FixedIncomeError::InvalidInput);
    }
    Ok((-rate * time_years).exp())
}

/// Present value of arbitrary cash flows using periodic compounding.
pub fn present_value(
    cash_flows: &[CashFlow],
    yield_rate: f64,
    compounding_per_year: u32,
) -> Result<f64, FixedIncomeError> {
    let mut pv = 0.0;
    for cf in cash_flows {
        if !cf.amount.is_finite() || !finite_nonnegative(cf.time_years) {
            return Err(FixedIncomeError::InvalidInput);
        }
        pv += cf.amount * discount_factor(yield_rate, cf.time_years, compounding_per_year)?;
    }
    Ok(pv)
}

/// Generate a regular fixed-coupon bond cash-flow schedule into `out`.
///
/// Payments occur at `1/f, 2/f, ... maturity`, where `f = payments_per_year`.
/// The final cash flow includes principal. Irregular stubs should be represented
/// by explicit `CashFlow`s.
pub fn coupon_bond_cash_flows_into(
    face_value: f64,
    annual_coupon_rate: f64,
    years_to_maturity: f64,
    payments_per_year: u32,
    out: &mut [CashFlow],
) -> Result<usize, FixedIncomeError> {
    if !finite_nonnegative(face_value)
        || !finite_nonnegative(annual_coupon_rate)
        || !finite_nonnegative(years_to_maturity)
    {
        return Err(FixedIncomeError::InvalidInput);
    }
    if payments_per_year == 0 {
        return Err(FixedIncomeError::InvalidFrequency);
    }
    let periods_f = years_to_maturity * payments_per_year as f64;
    let periods = periods_f.round() as usize;
    if periods == 0 {
        return Ok(0);
    }
    if (periods as f64 - periods_f).abs() > 1e-9 || out.len() < periods {
        return Err(FixedIncomeError::InvalidInput);
    }

    let coupon = face_value * annual_coupon_rate / payments_per_year as f64;
    for (idx, slot) in out.iter_mut().take(periods).enumerate() {
        let period = idx + 1;
        let mut amount = coupon;
        if period == periods {
            amount += face_value;
        }
        *slot = CashFlow {
            time_years: period as f64 / payments_per_year as f64,
            amount,
        };
    }
    Ok(periods)
}

/// Price a plain fixed-coupon bond from annual coupon and yield rates.
pub fn coupon_bond_price(
    face_value: f64,
    annual_coupon_rate: f64,
    annual_yield_rate: f64,
    years_to_maturity: f64,
    payments_per_year: u32,
) -> Result<f64, FixedIncomeError> {
    if !finite_nonnegative(face_value)
        || !finite_nonnegative(annual_coupon_rate)
        || !annual_yield_rate.is_finite()
        || !finite_nonnegative(years_to_maturity)
    {
        return Err(FixedIncomeError::InvalidInput);
    }
    if payments_per_year == 0 {
        return Err(FixedIncomeError::InvalidFrequency);
    }
    let periods_f = years_to_maturity * payments_per_year as f64;
    let periods = periods_f.round() as u32;
    if periods == 0 {
        return Ok(face_value);
    }
    if (periods as f64 - periods_f).abs() > 1e-9 {
        return Err(FixedIncomeError::InvalidInput);
    }

    let coupon = face_value * annual_coupon_rate / payments_per_year as f64;
    let periodic_yield = annual_yield_rate / payments_per_year as f64;
    let base = 1.0 + periodic_yield;
    if base <= 0.0 {
        return Err(FixedIncomeError::InvalidInput);
    }

    let mut price = 0.0;
    for period in 1..=periods {
        let mut amount = coupon;
        if period == periods {
            amount += face_value;
        }
        price += amount / base.powi(period as i32);
    }
    Ok(price)
}

/// Price a plain fixed-coupon bond from generated cash flows.
pub fn coupon_bond_price_from_cash_flows(
    face_value: f64,
    annual_coupon_rate: f64,
    annual_yield_rate: f64,
    years_to_maturity: f64,
    payments_per_year: u32,
    scratch: &mut [CashFlow],
) -> Result<f64, FixedIncomeError> {
    let n = coupon_bond_cash_flows_into(
        face_value,
        annual_coupon_rate,
        years_to_maturity,
        payments_per_year,
        scratch,
    )?;
    present_value(&scratch[..n], annual_yield_rate, payments_per_year)
}

/// Coupon-bond price, duration, modified duration, and convexity.
pub fn coupon_bond_metrics(
    face_value: f64,
    annual_coupon_rate: f64,
    annual_yield_rate: f64,
    years_to_maturity: f64,
    payments_per_year: u32,
) -> Result<BondMetrics, FixedIncomeError> {
    let price = coupon_bond_price(
        face_value,
        annual_coupon_rate,
        annual_yield_rate,
        years_to_maturity,
        payments_per_year,
    )?;
    if price <= 0.0 {
        return Err(FixedIncomeError::InvalidInput);
    }

    let periods = (years_to_maturity * payments_per_year as f64).round() as u32;
    if periods == 0 {
        return Ok(BondMetrics {
            price,
            macaulay_duration: 0.0,
            modified_duration: 0.0,
            convexity: 0.0,
        });
    }
    let coupon = face_value * annual_coupon_rate / payments_per_year as f64;
    let f = payments_per_year as f64;
    let y = annual_yield_rate / f;
    let base = 1.0 + y;
    if base <= 0.0 {
        return Err(FixedIncomeError::InvalidInput);
    }

    let mut weighted_time_pv = 0.0;
    let mut convexity_sum = 0.0;
    for period in 1..=periods {
        let mut amount = coupon;
        if period == periods {
            amount += face_value;
        }
        let pv = amount / base.powi(period as i32);
        let t = period as f64 / f;
        weighted_time_pv += t * pv;
        convexity_sum += period as f64 * (period as f64 + 1.0) * pv;
    }

    let macaulay_duration = weighted_time_pv / price;
    let modified_duration = macaulay_duration / base;
    let convexity = convexity_sum / (price * f * f * base * base);
    Ok(BondMetrics {
        price,
        macaulay_duration,
        modified_duration,
        convexity,
    })
}

/// Solve annual yield from a coupon bond price by bisection.
pub fn coupon_bond_yield_to_maturity(
    target_price: f64,
    face_value: f64,
    annual_coupon_rate: f64,
    years_to_maturity: f64,
    payments_per_year: u32,
    tolerance: f64,
    max_iterations: u32,
) -> Result<f64, FixedIncomeError> {
    if !finite_nonnegative(target_price) || target_price == 0.0 || max_iterations == 0 {
        return Err(FixedIncomeError::InvalidInput);
    }
    if payments_per_year == 0 {
        return Err(FixedIncomeError::InvalidFrequency);
    }

    let mut lo = -0.99 * payments_per_year as f64;
    let mut hi = 1.0;
    for _ in 0..64 {
        let p_hi = coupon_bond_price(
            face_value,
            annual_coupon_rate,
            hi,
            years_to_maturity,
            payments_per_year,
        )?;
        if p_hi < target_price {
            break;
        }
        hi *= 2.0;
    }

    for _ in 0..max_iterations {
        let mid = 0.5 * (lo + hi);
        let price = coupon_bond_price(
            face_value,
            annual_coupon_rate,
            mid,
            years_to_maturity,
            payments_per_year,
        )?;
        let err = price - target_price;
        if err.abs() <= tolerance {
            return Ok(mid);
        }
        if err > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Err(FixedIncomeError::NonConvergent)
}

/// Accrued interest for a regular coupon period.
pub fn accrued_interest(
    face_value: f64,
    annual_coupon_rate: f64,
    payments_per_year: u32,
    previous_coupon_day: u32,
    settlement_day: u32,
    next_coupon_day: u32,
    convention: DayCountConvention,
) -> Result<AccruedInterest, FixedIncomeError> {
    if !finite_nonnegative(face_value)
        || !finite_nonnegative(annual_coupon_rate)
        || payments_per_year == 0
        || settlement_day < previous_coupon_day
        || next_coupon_day <= previous_coupon_day
        || settlement_day > next_coupon_day
    {
        return Err(FixedIncomeError::InvalidInput);
    }
    let elapsed = year_fraction(previous_coupon_day, settlement_day, convention)?;
    let period = year_fraction(previous_coupon_day, next_coupon_day, convention)?;
    if period <= 0.0 {
        return Err(FixedIncomeError::InvalidInput);
    }
    let period_coupon = face_value * annual_coupon_rate / payments_per_year as f64;
    let accrual_fraction = elapsed / period;
    Ok(AccruedInterest {
        accrued: period_coupon * accrual_fraction,
        accrual_fraction,
        coupon_period_fraction: period,
    })
}

/// Dirty price = clean price + accrued interest.
pub fn dirty_price_from_clean(
    clean_price: f64,
    accrued_interest: f64,
) -> Result<f64, FixedIncomeError> {
    if !clean_price.is_finite() || !accrued_interest.is_finite() {
        return Err(FixedIncomeError::InvalidInput);
    }
    Ok(clean_price + accrued_interest)
}

/// Clean price = dirty price - accrued interest.
pub fn clean_price_from_dirty(
    dirty_price: f64,
    accrued_interest: f64,
) -> Result<f64, FixedIncomeError> {
    if !dirty_price.is_finite() || !accrued_interest.is_finite() {
        return Err(FixedIncomeError::InvalidInput);
    }
    Ok(dirty_price - accrued_interest)
}

/// Dollar value of one basis point for a plain fixed-coupon bond.
///
/// This is computed as the price change for a one-basis-point yield decrease,
/// which gives a positive value for ordinary long fixed-income positions.
pub fn coupon_bond_dv01(
    face_value: f64,
    annual_coupon_rate: f64,
    annual_yield_rate: f64,
    years_to_maturity: f64,
    payments_per_year: u32,
) -> Result<f64, FixedIncomeError> {
    let price = coupon_bond_price(
        face_value,
        annual_coupon_rate,
        annual_yield_rate,
        years_to_maturity,
        payments_per_year,
    )?;
    let bumped_down = coupon_bond_price(
        face_value,
        annual_coupon_rate,
        annual_yield_rate - 0.0001,
        years_to_maturity,
        payments_per_year,
    )?;
    Ok(bumped_down - price)
}

/// Key-rate duration by bumping the nearest cash-flow maturity by one bp.
///
/// The input `cash_flows` is explicit so irregular bonds, amortising loans,
/// or future curve-bootstrapped instruments can reuse the same kernel.
pub fn key_rate_duration(
    cash_flows: &[CashFlow],
    base_yield_rate: f64,
    compounding_per_year: u32,
    key_time_years: f64,
) -> Result<f64, FixedIncomeError> {
    if cash_flows.is_empty() || !base_yield_rate.is_finite() || !finite_nonnegative(key_time_years)
    {
        return Err(FixedIncomeError::InvalidInput);
    }
    if compounding_per_year == 0 {
        return Err(FixedIncomeError::InvalidFrequency);
    }

    let price = present_value(cash_flows, base_yield_rate, compounding_per_year)?;
    if price <= 0.0 {
        return Err(FixedIncomeError::InvalidInput);
    }

    let mut nearest = 0usize;
    let mut nearest_distance = f64::INFINITY;
    for (idx, cf) in cash_flows.iter().enumerate() {
        if !cf.amount.is_finite() || !finite_nonnegative(cf.time_years) {
            return Err(FixedIncomeError::InvalidInput);
        }
        let distance = (cf.time_years - key_time_years).abs();
        if distance < nearest_distance {
            nearest = idx;
            nearest_distance = distance;
        }
    }

    let mut bumped_price = 0.0;
    for (idx, cf) in cash_flows.iter().enumerate() {
        let rate = if idx == nearest {
            base_yield_rate + 0.0001
        } else {
            base_yield_rate
        };
        bumped_price += cf.amount * discount_factor(rate, cf.time_years, compounding_per_year)?;
    }

    Ok(-(bumped_price - price) / (price * 0.0001))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discount_factor_matches_hand_value() {
        let df = discount_factor(0.05, 2.0, 1).unwrap();
        assert!((df - 1.0 / 1.05f64.powi(2)).abs() < 1e-12);
    }

    #[test]
    fn par_coupon_bond_prices_at_face_when_coupon_equals_yield() {
        let price = coupon_bond_price(1000.0, 0.05, 0.05, 5.0, 2).unwrap();
        assert!((price - 1000.0).abs() < 1e-9, "price={price}");
    }

    #[test]
    fn premium_bond_yield_roundtrips_from_price() {
        let price = coupon_bond_price(1000.0, 0.06, 0.04, 10.0, 2).unwrap();
        assert!(price > 1000.0);
        let y = coupon_bond_yield_to_maturity(price, 1000.0, 0.06, 10.0, 2, 1e-10, 128).unwrap();
        assert!((y - 0.04).abs() < 1e-8, "ytm={y}");
    }

    #[test]
    fn zero_coupon_duration_is_maturity() {
        let m = coupon_bond_metrics(1000.0, 0.0, 0.03, 7.0, 1).unwrap();
        assert!((m.macaulay_duration - 7.0).abs() < 1e-12);
        assert!(m.modified_duration < m.macaulay_duration);
        assert!(m.convexity > 0.0);
    }

    #[test]
    fn invalid_frequency_is_rejected() {
        assert_eq!(
            coupon_bond_price(1000.0, 0.05, 0.05, 5.0, 0),
            Err(FixedIncomeError::InvalidFrequency)
        );
    }

    #[test]
    fn generated_cash_flows_match_coupon_bond_price() {
        let mut flows = [CashFlow {
            time_years: 0.0,
            amount: 0.0,
        }; 10];
        let n = coupon_bond_cash_flows_into(1000.0, 0.06, 5.0, 2, &mut flows).unwrap();
        assert_eq!(n, 10);
        assert!((flows[0].amount - 30.0).abs() < 1e-12);
        assert!((flows[9].amount - 1030.0).abs() < 1e-12);

        let direct = coupon_bond_price(1000.0, 0.06, 0.04, 5.0, 2).unwrap();
        let via_flows = present_value(&flows[..n], 0.04, 2).unwrap();
        assert!((direct - via_flows).abs() < 1e-9);
    }

    #[test]
    fn accrued_interest_splits_clean_and_dirty_price() {
        let ai = accrued_interest(1000.0, 0.06, 2, 0, 90, 180, DayCountConvention::Act360).unwrap();
        assert!((ai.accrual_fraction - 0.5).abs() < 1e-12);
        assert!((ai.accrued - 15.0).abs() < 1e-12);

        let dirty = dirty_price_from_clean(1010.0, ai.accrued).unwrap();
        assert!((dirty - 1025.0).abs() < 1e-12);
        let clean = clean_price_from_dirty(dirty, ai.accrued).unwrap();
        assert!((clean - 1010.0).abs() < 1e-12);
    }

    #[test]
    fn dv01_is_positive_for_standard_bond() {
        let dv01 = coupon_bond_dv01(1000.0, 0.05, 0.04, 10.0, 2).unwrap();
        assert!(dv01 > 0.0);
    }

    #[test]
    fn key_rate_duration_is_positive_for_nearest_cash_flow() {
        let flows = [
            CashFlow {
                time_years: 1.0,
                amount: 50.0,
            },
            CashFlow {
                time_years: 2.0,
                amount: 1050.0,
            },
        ];
        let krd = key_rate_duration(&flows, 0.04, 2, 2.1).unwrap();
        assert!(krd > 0.0);
    }
}
