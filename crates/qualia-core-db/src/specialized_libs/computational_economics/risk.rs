//! Risk metrics over supplied return and scenario data.
//!
//! These routines do not fetch market data or infer missing histories. Callers
//! provide the return series or shocks and the output buffers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskError {
    InvalidInput,
    OutputBufferTooSmall,
}

fn valid_return(x: f64) -> bool {
    x.is_finite() && x > -1.0
}

#[cfg(not(test))]
fn normal_quantile(probability: f64) -> Result<f64, RiskError> {
    if !(0.0..1.0).contains(&probability) {
        return Err(RiskError::InvalidInput);
    }
    Ok(crate::solvers::statistics::distributions::normal::standard_quantile(probability))
}

#[cfg(test)]
fn normal_quantile(probability: f64) -> Result<f64, RiskError> {
    if !(0.0..1.0).contains(&probability) {
        return Err(RiskError::InvalidInput);
    }

    // Peter John Acklam's inverse-normal approximation coefficients.
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];

    let plow = 0.02425;
    let phigh = 1.0 - plow;
    if probability < plow {
        let q = (-2.0 * probability.ln()).sqrt();
        let numerator = ((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5];
        let denominator = (((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0;
        Ok(numerator / denominator)
    } else if probability <= phigh {
        let q = probability - 0.5;
        let r = q * q;
        let numerator = (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q;
        let denominator = ((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0;
        Ok(numerator / denominator)
    } else {
        let q = (-2.0 * (1.0 - probability).ln()).sqrt();
        let numerator = ((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5];
        let denominator = (((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0;
        Ok(-(numerator / denominator))
    }
}

/// Copy and sort returns ascending into caller-owned scratch.
pub fn sorted_returns_into(returns: &[f64], scratch: &mut [f64]) -> Result<usize, RiskError> {
    if returns.is_empty() || scratch.len() < returns.len() {
        return Err(RiskError::OutputBufferTooSmall);
    }
    for (idx, value) in returns.iter().enumerate() {
        if !valid_return(*value) {
            return Err(RiskError::InvalidInput);
        }
        scratch[idx] = *value;
    }
    scratch[..returns.len()].sort_by(|a, b| a.total_cmp(b));
    Ok(returns.len())
}

/// Historical VaR as a positive loss fraction at `confidence`.
pub fn historical_var(
    returns: &[f64],
    confidence: f64,
    scratch: &mut [f64],
) -> Result<f64, RiskError> {
    if !(0.0..1.0).contains(&confidence) {
        return Err(RiskError::InvalidInput);
    }
    let n = sorted_returns_into(returns, scratch)?;
    let tail_probability = 1.0 - confidence;
    let idx = ((tail_probability * n as f64).ceil() as usize).saturating_sub(1);
    Ok((-scratch[idx]).max(0.0))
}

/// Historical expected shortfall/CVaR as a positive average tail loss.
pub fn historical_cvar(
    returns: &[f64],
    confidence: f64,
    scratch: &mut [f64],
) -> Result<f64, RiskError> {
    if !(0.0..1.0).contains(&confidence) {
        return Err(RiskError::InvalidInput);
    }
    let n = sorted_returns_into(returns, scratch)?;
    let tail_probability = 1.0 - confidence;
    let tail_count = ((tail_probability * n as f64).ceil() as usize).max(1);
    let mut loss = 0.0;
    for value in scratch.iter().take(tail_count) {
        loss += (-*value).max(0.0);
    }
    Ok(loss / tail_count as f64)
}

/// Parametric Gaussian VaR as a positive loss fraction.
pub fn gaussian_var(mean: f64, std_dev: f64, confidence: f64) -> Result<f64, RiskError> {
    if !mean.is_finite()
        || !std_dev.is_finite()
        || std_dev < 0.0
        || !(0.0..1.0).contains(&confidence)
    {
        return Err(RiskError::InvalidInput);
    }
    let z_left_tail = normal_quantile(1.0 - confidence)?;
    Ok((-(mean + z_left_tail * std_dev)).max(0.0))
}

/// Apply asset shocks to portfolio weights and return scenario portfolio loss.
pub fn scenario_loss(weights: &[f64], shocks: &[f64]) -> Result<f64, RiskError> {
    if weights.is_empty() || weights.len() != shocks.len() {
        return Err(RiskError::InvalidInput);
    }
    let mut scenario_return = 0.0;
    for idx in 0..weights.len() {
        if !weights[idx].is_finite() || !valid_return(shocks[idx]) {
            return Err(RiskError::InvalidInput);
        }
        scenario_return += weights[idx] * shocks[idx];
    }
    Ok((-scenario_return).max(0.0))
}

/// Scenario losses for row-major scenario shock matrix.
pub fn scenario_losses_into(
    weights: &[f64],
    scenario_shocks: &[f64],
    scenario_count: usize,
    asset_count: usize,
    out: &mut [f64],
) -> Result<usize, RiskError> {
    if scenario_count == 0
        || asset_count == 0
        || weights.len() != asset_count
        || scenario_shocks.len() != scenario_count * asset_count
    {
        return Err(RiskError::InvalidInput);
    }
    if out.len() < scenario_count {
        return Err(RiskError::OutputBufferTooSmall);
    }

    for scenario in 0..scenario_count {
        let start = scenario * asset_count;
        out[scenario] = scenario_loss(weights, &scenario_shocks[start..start + asset_count])?;
    }
    Ok(scenario_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn historical_var_uses_left_tail_loss() {
        let returns = [-0.10, -0.05, 0.0, 0.02, 0.03];
        let mut scratch = [0.0; 5];
        let var = historical_var(&returns, 0.80, &mut scratch).unwrap();
        assert!((var - 0.10).abs() < 1e-12);
    }

    #[test]
    fn historical_cvar_averages_tail_losses() {
        let returns = [-0.10, -0.05, 0.0, 0.02, 0.03];
        let mut scratch = [0.0; 5];
        let cvar = historical_cvar(&returns, 0.60, &mut scratch).unwrap();
        assert!((cvar - 0.075).abs() < 1e-12);
    }

    #[test]
    fn gaussian_var_is_positive_for_left_tail() {
        let var = gaussian_var(0.0, 0.02, 0.95).unwrap();
        assert!(var > 0.032 && var < 0.034);
    }

    #[test]
    fn scenario_loss_is_weighted_negative_return() {
        let weights = [0.25, 0.75];
        let shocks = [-0.10, -0.20];
        let loss = scenario_loss(&weights, &shocks).unwrap();
        assert!((loss - 0.175).abs() < 1e-12);
    }

    #[test]
    fn scenario_losses_use_row_major_shocks() {
        let weights = [0.5, 0.5];
        let shocks = [-0.10, 0.0, 0.0, -0.20];
        let mut out = [0.0; 2];
        scenario_losses_into(&weights, &shocks, 2, 2, &mut out).unwrap();
        assert!((out[0] - 0.05).abs() < 1e-12);
        assert!((out[1] - 0.10).abs() < 1e-12);
    }

    #[test]
    fn invalid_confidence_is_rejected() {
        let returns = [0.0, 0.1];
        let mut scratch = [0.0; 2];
        assert_eq!(
            historical_var(&returns, 1.0, &mut scratch),
            Err(RiskError::InvalidInput)
        );
    }

    #[test]
    fn scratch_too_small_is_rejected() {
        let returns = [0.0, 0.1];
        let mut scratch = [0.0; 1];
        assert_eq!(
            sorted_returns_into(&returns, &mut scratch),
            Err(RiskError::OutputBufferTooSmall)
        );
    }
}
