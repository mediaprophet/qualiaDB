//! Labor and household economics: labor supply, household production, and
//! human capital.
//!
//! Allocation class: **HotZeroHeap**. No `Vec`/`String`/`Box` in any kernel.
//!
//! Assumptions:
//! - Labor supply uses the standard labor-leisure tradeoff with Cobb-Douglas
//!   utility over consumption and leisure.
//! - Household production uses a CES aggregator of time and goods.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaborHouseholdError {
    InvalidInput,
    NonFinite,
    InsufficientData,
}

fn require_finite(x: f64) -> Result<(), LaborHouseholdError> {
    if x.is_finite() {
        Ok(())
    } else {
        Err(LaborHouseholdError::NonFinite)
    }
}

/// Labor supply from Cobb-Douglas utility over consumption and leisure.
///
/// Utility: `U = c^alpha * l^(1-alpha)` where `l = T - h` (leisure = time
/// endowment minus hours worked). Budget: `c = w*h + non_labor_income`.
///
/// Optimal hours: `h* = alpha * T - (1-alpha) * non_labor_income / w`
/// (clamped to [0, T]).
pub fn labor_supply_cobb_douglas(
    wage: f64,
    time_endowment: f64,
    non_labor_income: f64,
    alpha: f64,
) -> Result<(f64, f64), LaborHouseholdError> {
    require_finite(wage)?;
    require_finite(time_endowment)?;
    require_finite(non_labor_income)?;
    require_finite(alpha)?;
    if wage <= 0.0 || time_endowment <= 0.0 || non_labor_income < 0.0 || !(0.0..=1.0).contains(&alpha) {
        return Err(LaborHouseholdError::InvalidInput);
    }
    let h_raw = alpha * time_endowment - (1.0 - alpha) * non_labor_income / wage;
    let h = h_raw.clamp(0.0, time_endowment);
    let consumption = wage * h + non_labor_income;
    Ok((h, consumption))
}

/// Household production via CES aggregator: `Y = (alpha * time^rho + (1-alpha) * goods^rho)^(1/rho)`.
pub fn household_production_ces(
    time: f64,
    goods: f64,
    alpha: f64,
    rho: f64,
) -> Result<f64, LaborHouseholdError> {
    require_finite(time)?;
    require_finite(goods)?;
    require_finite(alpha)?;
    require_finite(rho)?;
    if time < 0.0 || goods < 0.0 || !(0.0..=1.0).contains(&alpha) || rho == 0.0 {
        return Err(LaborHouseholdError::InvalidInput);
    }
    let inner = alpha * time.powf(rho) + (1.0 - alpha) * goods.powf(rho);
    if inner <= 0.0 {
        return Err(LaborHouseholdError::InvalidInput);
    }
    Ok(inner.powf(1.0 / rho))
}

/// Human capital accumulation (Ben-Porath): `h_{t+1} = h_t + s_t - delta * h_t`
/// where `s_t` is investment (time spent learning) and `delta` is depreciation.
///
/// Writes `n_periods` human capital values into `out`.
pub fn human_capital_accumulation_into(
    h0: f64,
    investment: &[f64],
    delta: f64,
    out: &mut [f64],
) -> Result<usize, LaborHouseholdError> {
    if investment.is_empty() || out.len() < investment.len() {
        return Err(LaborHouseholdError::InsufficientData);
    }
    require_finite(h0)?;
    require_finite(delta)?;
    if h0 < 0.0 || !(0.0..=1.0).contains(&delta) {
        return Err(LaborHouseholdError::InvalidInput);
    }
    let mut h = h0;
    for (t, s) in investment.iter().enumerate() {
        require_finite(*s)?;
        if *s < 0.0 {
            return Err(LaborHouseholdError::InvalidInput);
        }
        out[t] = h;
        h = h + s - delta * h;
        if !h.is_finite() {
            return Err(LaborHouseholdError::NonFinite);
        }
    }
    Ok(investment.len())
}

/// Efficiency units of labor: `effective_labor = raw_labor * human_capital`.
pub fn efficiency_units(raw_labor: f64, human_capital: f64) -> Result<f64, LaborHouseholdError> {
    require_finite(raw_labor)?;
    require_finite(human_capital)?;
    if raw_labor < 0.0 || human_capital < 0.0 {
        return Err(LaborHouseholdError::InvalidInput);
    }
    Ok(raw_labor * human_capital)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn labor_supply_no_non_labor_income() {
        // alpha=0.5, T=24, w=10, non_labor=0 → h = 0.5*24 = 12
        let (h, c) = labor_supply_cobb_douglas(10.0, 24.0, 0.0, 0.5).unwrap();
        assert!(approx(h, 12.0, 1e-9));
        assert!(approx(c, 120.0, 1e-9));
    }

    #[test]
    fn labor_supply_with_non_labor_income() {
        // alpha=0.5, T=24, w=10, non_labor=100 → h = 12 - 0.5*100/10 = 12 - 5 = 7
        let (h, _) = labor_supply_cobb_douglas(10.0, 24.0, 100.0, 0.5).unwrap();
        assert!(approx(h, 7.0, 1e-9));
    }

    #[test]
    fn labor_supply_clamped_to_zero() {
        // Very high non-labor income → h = 0
        let (h, _) = labor_supply_cobb_douglas(10.0, 24.0, 10000.0, 0.5).unwrap();
        assert!(approx(h, 0.0, 1e-9));
    }

    #[test]
    fn household_production_ces_linear() {
        // rho=1: Y = alpha*time + (1-alpha)*goods
        let y = household_production_ces(10.0, 20.0, 0.5, 1.0).unwrap();
        assert!(approx(y, 15.0, 1e-9));
    }

    #[test]
    fn household_production_ces_leontief_limit() {
        // Large rho → max(time, goods)
        let y = household_production_ces(10.0, 20.0, 0.5, 100.0).unwrap();
        assert!(y > 19.0 && y < 20.1);
    }

    #[test]
    fn human_capital_accumulation() {
        // h0=10, investment=[1, 1, 1], delta=0.1
        // h1 = 10 + 1 - 1 = 10; h2 = 10 + 1 - 1 = 10; h3 = 10
        let inv = [1.0, 1.0, 1.0];
        let mut out = [0.0f64; 3];
        human_capital_accumulation_into(10.0, &inv, 0.1, &mut out).unwrap();
        // out[0] = h0 = 10, out[1] = h0 + 1 - 0.1*10 = 10, out[2] = 10
        assert!(approx(out[0], 10.0, 1e-9));
        assert!(approx(out[1], 10.0, 1e-9));
    }

    #[test]
    fn efficiency_units_basic() {
        let e = efficiency_units(40.0, 2.0).unwrap();
        assert!(approx(e, 80.0, 1e-9));
    }

    #[test]
    fn invalid_inputs_rejected() {
        assert_eq!(
            labor_supply_cobb_douglas(-1.0, 24.0, 0.0, 0.5).unwrap_err(),
            LaborHouseholdError::InvalidInput
        );
        assert_eq!(
            household_production_ces(-1.0, 10.0, 0.5, 1.0).unwrap_err(),
            LaborHouseholdError::InvalidInput
        );
    }
}
