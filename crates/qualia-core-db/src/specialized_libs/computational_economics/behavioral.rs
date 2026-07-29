//! Behavioral economics: prospect theory, hyperbolic discounting, and
//! behavioral biases.
//!
//! Allocation class: **HotZeroHeap**. No `Vec`/`String`/`Box` in any kernel.
//!
//! Assumptions:
//! - Prospect theory uses Kahneman-Tversky value function with a reference
//!   point, loss aversion, and diminishing sensitivity.
//! - Hyperbolic discounting uses the quasi-hyperbolic (beta-delta) form.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehavioralError {
    InvalidInput,
    NonFinite,
    InsufficientData,
}

fn require_finite(x: f64) -> Result<(), BehavioralError> {
    if x.is_finite() {
        Ok(())
    } else {
        Err(BehavioralError::NonFinite)
    }
}

/// Prospect theory value function (Kahneman-Tversky).
///
/// For gains (x >= 0): `v = x^alpha`.
/// For losses (x < 0): `v = -lambda * (-x)^beta`.
///
/// Typical parameters: `alpha = beta = 0.88`, `lambda = 2.25`.
/// `x` is measured relative to a reference point.
pub fn prospect_value(x: f64, alpha: f64, beta: f64, lambda: f64) -> Result<f64, BehavioralError> {
    require_finite(x)?;
    require_finite(alpha)?;
    require_finite(beta)?;
    require_finite(lambda)?;
    if !(0.0..=1.0).contains(&alpha) || !(0.0..=1.0).contains(&beta) || lambda <= 0.0 {
        return Err(BehavioralError::InvalidInput);
    }
    if x >= 0.0 {
        Ok(x.powf(alpha))
    } else {
        Ok(-lambda * (-x).powf(beta))
    }
}

/// Probability weighting function (Prelec): `w(p) = exp(-(-ln(p))^gamma)`.
///
/// Overweights small probabilities, underweights moderate probabilities.
pub fn probability_weight(p: f64, gamma: f64) -> Result<f64, BehavioralError> {
    require_finite(p)?;
    require_finite(gamma)?;
    if !(0.0..=1.0).contains(&p) || p == 0.0 || gamma <= 0.0 {
        return Err(BehavioralError::InvalidInput);
    }
    Ok((-(-p.ln()).powf(gamma)).exp())
}

/// Quasi-hyperbolic discount factor: present value of 1 utility at time `t`.
///
/// `d(0) = 1`, `d(t) = beta * delta^t` for `t >= 1`.
///
/// `beta` is the present-bias parameter (0 < beta < 1 for present bias),
/// `delta` is the long-run discount factor.
pub fn hyperbolic_discount(t: u32, beta: f64, delta: f64) -> Result<f64, BehavioralError> {
    require_finite(beta)?;
    require_finite(delta)?;
    if !(0.0..=1.0).contains(&beta) || !(0.0..=1.0).contains(&delta) {
        return Err(BehavioralError::InvalidInput);
    }
    if t == 0 {
        Ok(1.0)
    } else {
        Ok(beta * delta.powi(t as i32))
    }
}

/// Present-biased utility: sum of `hyperbolic_discount(t) * u_t`.
///
/// `utilities` is a slice of per-period utilities. Returns the discounted sum.
pub fn present_biased_utility(
    utilities: &[f64],
    beta: f64,
    delta: f64,
) -> Result<f64, BehavioralError> {
    if utilities.is_empty() {
        return Err(BehavioralError::InsufficientData);
    }
    let mut total = 0.0;
    for (t, u) in utilities.iter().enumerate() {
        require_finite(*u)?;
        let d = hyperbolic_discount(t as u32, beta, delta)?;
        total += d * u;
    }
    Ok(total)
}

/// Endowment effect markup: willingness-to-accept exceeds willingness-to-pay
/// by a factor of `lambda` (loss aversion coefficient).
///
/// `wta = wtp * lambda`.
pub fn endowment_effect_wta(wtp: f64, lambda: f64) -> Result<f64, BehavioralError> {
    require_finite(wtp)?;
    require_finite(lambda)?;
    if wtp < 0.0 || lambda <= 0.0 {
        return Err(BehavioralError::InvalidInput);
    }
    Ok(wtp * lambda)
}

/// S-shaped utility with reference dependence (simplified prospect value).
///
/// `u(x, ref) = prospect_value(x - ref, alpha, beta, lambda)`.
pub fn reference_dependent_utility(
    x: f64,
    reference: f64,
    alpha: f64,
    beta: f64,
    lambda: f64,
) -> Result<f64, BehavioralError> {
    prospect_value(x - reference, alpha, beta, lambda)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn prospect_value_gain() {
        // Gain of 100, alpha=0.88 → 100^0.88
        let v = prospect_value(100.0, 0.88, 0.88, 2.25).unwrap();
        assert!(approx(v, 100.0f64.powf(0.88), 1e-6));
        assert!(v > 0.0);
    }

    #[test]
    fn prospect_value_loss_aversive() {
        // Loss of 100 vs gain of 100: loss should have larger absolute value.
        let v_gain = prospect_value(100.0, 0.88, 0.88, 2.25).unwrap();
        let v_loss = prospect_value(-100.0, 0.88, 0.88, 2.25).unwrap();
        assert!(v_loss.abs() > v_gain.abs());
    }

    #[test]
    fn prospect_value_loss_formula() {
        // Loss of 100: v = -2.25 * 100^0.88
        let v = prospect_value(-100.0, 0.88, 0.88, 2.25).unwrap();
        assert!(approx(v, -2.25 * 100.0f64.powf(0.88), 1e-6));
    }

    #[test]
    fn probability_weight_overweights_small() {
        // Small probability should be overweighted: w(p) > p for small p.
        let w = probability_weight(0.01, 0.65).unwrap();
        assert!(w > 0.01, "w(0.01) = {} should exceed 0.01", w);
    }

    #[test]
    fn probability_weight_w_zero_p() {
        assert_eq!(
            probability_weight(0.0, 0.65).unwrap_err(),
            BehavioralError::InvalidInput
        );
    }

    #[test]
    fn hyperbolic_discount_present() {
        // t=0 → 1.0
        let d = hyperbolic_discount(0, 0.7, 0.95).unwrap();
        assert!(approx(d, 1.0, 1e-9));
    }

    #[test]
    fn hyperbolic_discount_future() {
        // t=1 → beta * delta = 0.7 * 0.95 = 0.665
        let d = hyperbolic_discount(1, 0.7, 0.95).unwrap();
        assert!(approx(d, 0.665, 1e-9));
    }

    #[test]
    fn hyperbolic_discount_declines() {
        let d1 = hyperbolic_discount(1, 0.7, 0.95).unwrap();
        let d2 = hyperbolic_discount(2, 0.7, 0.95).unwrap();
        assert!(d2 < d1);
    }

    #[test]
    fn present_biased_utility_sum() {
        // utilities [10, 10, 10], beta=0.7, delta=0.95
        // PV = 10 + 0.7*0.95*10 + 0.7*0.95^2*10
        let u = present_biased_utility(&[10.0, 10.0, 10.0], 0.7, 0.95).unwrap();
        let expected = 10.0 + 0.7 * 0.95 * 10.0 + 0.7 * 0.95f64.powi(2) * 10.0;
        assert!(approx(u, expected, 1e-6));
    }

    #[test]
    fn endowment_effect_doubles_wta() {
        // lambda=2 → WTA = 2 * WTP
        let wta = endowmenteffect_wta(50.0, 2.0).unwrap();
        assert!(approx(wta, 100.0, 1e-9));
    }

    fn endowmenteffect_wta(wtp: f64, lambda: f64) -> Result<f64, BehavioralError> {
        endowment_effect_wta(wtp, lambda)
    }

    #[test]
    fn reference_dependent_utility_relative() {
        // x=110, ref=100 → gain of 10
        let v = reference_dependent_utility(110.0, 100.0, 0.88, 0.88, 2.25).unwrap();
        let v_direct = prospect_value(10.0, 0.88, 0.88, 2.25).unwrap();
        assert!(approx(v, v_direct, 1e-9));
    }

    #[test]
    fn invalid_inputs_rejected() {
        assert_eq!(
            prospect_value(100.0, 1.5, 0.88, 2.25).unwrap_err(),
            BehavioralError::InvalidInput
        );
        assert_eq!(
            hyperbolic_discount(1, 1.5, 0.95).unwrap_err(),
            BehavioralError::InvalidInput
        );
    }
}
