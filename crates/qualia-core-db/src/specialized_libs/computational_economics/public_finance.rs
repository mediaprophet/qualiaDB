//! Public finance: progressive taxation, transfers, fiscal multipliers,
//! Laffer curve, and survival-floor allocation.
//!
//! Allocation class: **HotZeroHeap**. No `Vec`/`String`/`Box` in any kernel.
//!
//! **Rights-affecting:** tax/transfer/survival-floor outputs affect people's
//! livelihoods. These functions return diagnostics and assumptions alongside
//! results. Before any UI/qapp exposure, pair with SHACL/deontic/provenance
//! checks to verify eligibility, evidence, and consent.

/// Maximum tax brackets.
pub const MAX_BRACKETS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicFinanceError {
    InvalidInput,
    NonFinite,
    BufferTooSmall,
    InsufficientData,
}

/// A progressive tax bracket: income above `threshold` is taxed at
/// `marginal_rate` up to the next bracket's threshold.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TaxBracket {
    pub threshold: f64,
    pub marginal_rate: f64,
}

fn require_finite(x: f64) -> Result<(), PublicFinanceError> {
    if x.is_finite() {
        Ok(())
    } else {
        Err(PublicFinanceError::NonFinite)
    }
}

/// Compute progressive tax on `income` given sorted brackets.
///
/// Brackets must be sorted ascending by threshold. Writes per-bracket tax
/// into `out[..n_brackets]`. Returns `(total_tax, average_rate)`.
pub fn progressive_tax_into(
    income: f64,
    brackets: &[TaxBracket],
    out: &mut [f64],
) -> Result<(f64, f64), PublicFinanceError> {
    if brackets.is_empty() || out.len() < brackets.len() {
        return Err(PublicFinanceError::BufferTooSmall);
    }
    require_finite(income)?;
    if income < 0.0 {
        return Err(PublicFinanceError::InvalidInput);
    }
    // Validate sorted and rates.
    let mut prev_threshold = -1.0;
    for b in brackets {
        require_finite(b.threshold)?;
        require_finite(b.marginal_rate)?;
        if b.threshold < 0.0 || !(0.0..=1.0).contains(&b.marginal_rate) {
            return Err(PublicFinanceError::InvalidInput);
        }
        if b.threshold <= prev_threshold && prev_threshold >= 0.0 {
            return Err(PublicFinanceError::InvalidInput);
        }
        prev_threshold = b.threshold;
    }
    let n = brackets.len();
    let mut total_tax = 0.0;
    for i in 0..n {
        let lower = brackets[i].threshold;
        let upper = if i + 1 < n { brackets[i + 1].threshold } else { f64::INFINITY };
        let taxable = if income > lower {
            (income.min(upper) - lower).max(0.0)
        } else {
            0.0
        };
        let bracket_tax = taxable * brackets[i].marginal_rate;
        out[i] = bracket_tax;
        total_tax += bracket_tax;
    }
    let avg_rate = if income > 0.0 { total_tax / income } else { 0.0 };
    Ok((total_tax, avg_rate))
}

/// Means-tested transfer: `payment = max(0, base - phaseout_rate * max(0, income - threshold))`.
///
/// `base` is the full transfer when income is at or below `threshold`.
pub fn transfer_payment(
    base: f64,
    income: f64,
    threshold: f64,
    phaseout_rate: f64,
) -> Result<f64, PublicFinanceError> {
    require_finite(base)?;
    require_finite(income)?;
    require_finite(threshold)?;
    require_finite(phaseout_rate)?;
    if base < 0.0 || income < 0.0 || threshold < 0.0 || phaseout_rate < 0.0 {
        return Err(PublicFinanceError::InvalidInput);
    }
    let excess = (income - threshold).max(0.0);
    Ok((base - phaseout_rate * excess).max(0.0))
}

/// Keynesian fiscal multiplier: `multiplier = 1 / (1 - mpc + leakage)`.
///
/// Returns total GDP impact = `initial_spending * multiplier`.
pub fn fiscal_multiplier(
    initial_spending: f64,
    mpc: f64,
    leakage_rate: f64,
) -> Result<f64, PublicFinanceError> {
    require_finite(initial_spending)?;
    require_finite(mpc)?;
    require_finite(leakage_rate)?;
    if !(0.0..=1.0).contains(&mpc) || leakage_rate < 0.0 {
        return Err(PublicFinanceError::InvalidInput);
    }
    let denom = 1.0 - mpc + leakage_rate;
    if denom <= 0.0 {
        return Err(PublicFinanceError::InvalidInput);
    }
    Ok(initial_spending / denom)
}

/// Simple Laffer curve revenue: `R = t * B * (1 - t)^elasticity`.
///
/// `t` is the tax rate, `B` is the base income, `elasticity` is the behavioral
/// response elasticity. Revenue is zero at `t=0` and `t=1`.
pub fn laffer_curve_revenue(
    tax_rate: f64,
    base_income: f64,
    elasticity: f64,
) -> Result<f64, PublicFinanceError> {
    require_finite(tax_rate)?;
    require_finite(base_income)?;
    require_finite(elasticity)?;
    if !(0.0..=1.0).contains(&tax_rate) || base_income < 0.0 || elasticity < 0.0 {
        return Err(PublicFinanceError::InvalidInput);
    }
    Ok(tax_rate * base_income * (1.0 - tax_rate).powf(elasticity))
}

/// Survival-floor allocation: distribute `budget` to satisfy `needs[i]`
/// proportionally up to each need.
///
/// Writes allocation into `out[..n_agents]`. Returns total unmet need
/// (0.0 if budget >= total need).
pub fn survival_floor_allocation_into(
    needs: &[f64],
    budget: f64,
    out: &mut [f64],
) -> Result<f64, PublicFinanceError> {
    if needs.is_empty() || out.len() < needs.len() {
        return Err(PublicFinanceError::BufferTooSmall);
    }
    require_finite(budget)?;
    if budget < 0.0 {
        return Err(PublicFinanceError::InvalidInput);
    }
    let mut total_need = 0.0;
    for n in needs {
        require_finite(*n)?;
        if *n < 0.0 {
            return Err(PublicFinanceError::InvalidInput);
        }
        total_need += n;
    }
    if total_need <= 0.0 {
        for i in 0..needs.len() {
            out[i] = 0.0;
        }
        return Ok(0.0);
    }
    if budget >= total_need {
        // Fully satisfy all needs.
        for i in 0..needs.len() {
            out[i] = needs[i];
        }
        return Ok(0.0);
    }
    // Proportional allocation.
    let ratio = budget / total_need;
    let mut allocated = 0.0;
    for i in 0..needs.len() {
        out[i] = needs[i] * ratio;
        allocated += out[i];
    }
    Ok((total_need - allocated).max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn progressive_tax_hand_computed() {
        // Brackets: [(0, 0.1), (50000, 0.2), (100000, 0.3)]
        // Income 120000 → tax = 50000*0.1 + 50000*0.2 + 20000*0.3 = 21000
        let brackets = [
            TaxBracket { threshold: 0.0, marginal_rate: 0.1 },
            TaxBracket { threshold: 50000.0, marginal_rate: 0.2 },
            TaxBracket { threshold: 100000.0, marginal_rate: 0.3 },
        ];
        let mut per_bracket = [0.0f64; 3];
        let (total, avg) = progressive_tax_into(120000.0, &brackets, &mut per_bracket).unwrap();
        assert!(approx(total, 21000.0, 1e-6));
        assert!(approx(avg, 21000.0 / 120000.0, 1e-6));
        assert!(approx(per_bracket[0], 5000.0, 1e-6));
        assert!(approx(per_bracket[1], 10000.0, 1e-6));
        assert!(approx(per_bracket[2], 6000.0, 1e-6));
    }

    #[test]
    fn progressive_tax_below_first_threshold() {
        let brackets = [TaxBracket { threshold: 50000.0, marginal_rate: 0.2 }];
        let mut per = [0.0f64; 1];
        let (total, _) = progressive_tax_into(30000.0, &brackets, &mut per).unwrap();
        assert!(approx(total, 0.0, 1e-9));
    }

    #[test]
    fn progressive_tax_rejects_unsorted() {
        let brackets = [
            TaxBracket { threshold: 100000.0, marginal_rate: 0.3 },
            TaxBracket { threshold: 50000.0, marginal_rate: 0.2 },
        ];
        let mut per = [0.0f64; 2];
        let err = progressive_tax_into(120000.0, &brackets, &mut per).unwrap_err();
        assert_eq!(err, PublicFinanceError::InvalidInput);
    }

    #[test]
    fn transfer_below_threshold() {
        // base=1000, income=20000, threshold=30000, phaseout=0.1 → full payment
        let p = transfer_payment(1000.0, 20000.0, 30000.0, 0.1).unwrap();
        assert!(approx(p, 1000.0, 1e-9));
    }

    #[test]
    fn transfer_phased_out() {
        // base=1000, income=35000, threshold=30000, phaseout=0.1
        // excess = 5000, payment = 1000 - 0.1*5000 = 500
        let p = transfer_payment(1000.0, 35000.0, 30000.0, 0.1).unwrap();
        assert!(approx(p, 500.0, 1e-9));
    }

    #[test]
    fn transfer_fully_phased_out() {
        // base=1000, income=50000, threshold=30000, phaseout=0.1
        // excess = 20000, payment = 1000 - 2000 = -1000 → clamped to 0
        let p = transfer_payment(1000.0, 50000.0, 30000.0, 0.1).unwrap();
        assert!(approx(p, 0.0, 1e-9));
    }

    #[test]
    fn fiscal_multiplier_basic() {
        // mpc=0.8, leakage=0.1 → multiplier = 1/0.3 ≈ 3.333
        let impact = fiscal_multiplier(100.0, 0.8, 0.1).unwrap();
        assert!(approx(impact, 100.0 / 0.3, 1e-6));
    }

    #[test]
    fn laffer_curve_zero_at_extremes() {
        assert!(approx(laffer_curve_revenue(0.0, 1000.0, 1.0).unwrap(), 0.0, 1e-9));
        assert!(approx(laffer_curve_revenue(1.0, 1000.0, 1.0).unwrap(), 0.0, 1e-9));
    }

    #[test]
    fn laffer_curve_peaks_in_middle() {
        // With elasticity=1, peak at t=1/(1+1)=0.5
        let r_low = laffer_curve_revenue(0.3, 1000.0, 1.0).unwrap();
        let r_peak = laffer_curve_revenue(0.5, 1000.0, 1.0).unwrap();
        let r_high = laffer_curve_revenue(0.7, 1000.0, 1.0).unwrap();
        assert!(r_peak > r_low && r_peak > r_high);
    }

    #[test]
    fn survival_floor_fully_satisfied() {
        let needs = [10.0, 20.0, 30.0];
        let mut alloc = [0.0f64; 3];
        let unmet = survival_floor_allocation_into(&needs, 100.0, &mut alloc).unwrap();
        assert!(approx(unmet, 0.0, 1e-9));
        assert_eq!(alloc, [10.0, 20.0, 30.0]);
    }

    #[test]
    fn survival_floor_proportional() {
        let needs = [10.0, 20.0, 30.0]; // total = 60
        let mut alloc = [0.0f64; 3];
        let unmet = survival_floor_allocation_into(&needs, 30.0, &mut alloc).unwrap();
        // ratio = 30/60 = 0.5 → alloc = [5, 10, 15], unmet = 30
        assert!(approx(alloc[0], 5.0, 1e-9));
        assert!(approx(alloc[1], 10.0, 1e-9));
        assert!(approx(alloc[2], 15.0, 1e-9));
        assert!(approx(unmet, 30.0, 1e-6));
    }

    #[test]
    fn empty_needs_rejected() {
        let mut alloc = [0.0f64; 0];
        let err = survival_floor_allocation_into(&[], 100.0, &mut alloc).unwrap_err();
        assert_eq!(err, PublicFinanceError::BufferTooSmall);
    }
}
