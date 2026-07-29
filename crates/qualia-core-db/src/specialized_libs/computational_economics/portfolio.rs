//! Portfolio analytics over supplied return matrices.
//!
//! Returns are row-major: period `t`, asset `i` lives at
//! `returns[t * asset_count + i]`. Functions use caller-owned output buffers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortfolioError {
    InvalidInput,
    OutputBufferTooSmall,
}

fn validate_shape(
    flat_returns: &[f64],
    period_count: usize,
    asset_count: usize,
) -> Result<(), PortfolioError> {
    if period_count == 0 || asset_count == 0 || flat_returns.len() != period_count * asset_count {
        return Err(PortfolioError::InvalidInput);
    }
    if !flat_returns.iter().all(|x| x.is_finite()) {
        return Err(PortfolioError::InvalidInput);
    }
    Ok(())
}

fn validate_weights(weights: &[f64], asset_count: usize) -> Result<(), PortfolioError> {
    if weights.len() != asset_count || !weights.iter().all(|x| x.is_finite()) {
        return Err(PortfolioError::InvalidInput);
    }
    Ok(())
}

/// Weighted portfolio return for each period.
pub fn portfolio_returns_into(
    flat_returns: &[f64],
    period_count: usize,
    asset_count: usize,
    weights: &[f64],
    out: &mut [f64],
) -> Result<usize, PortfolioError> {
    validate_shape(flat_returns, period_count, asset_count)?;
    validate_weights(weights, asset_count)?;
    if out.len() < period_count {
        return Err(PortfolioError::OutputBufferTooSmall);
    }

    for period in 0..period_count {
        let mut value = 0.0;
        let offset = period * asset_count;
        for asset in 0..asset_count {
            value += flat_returns[offset + asset] * weights[asset];
        }
        out[period] = value;
    }
    Ok(period_count)
}

/// Arithmetic mean of a supplied return series.
pub fn mean_return(returns: &[f64]) -> Result<f64, PortfolioError> {
    #[cfg(not(test))]
    {
        return crate::solvers::statistics::descriptive::mean(returns)
            .ok_or(PortfolioError::InvalidInput);
    }

    #[cfg(test)]
    {
        if returns.is_empty() || !returns.iter().all(|x| x.is_finite()) {
            return Err(PortfolioError::InvalidInput);
        }
        let mut sum = 0.0;
        for value in returns {
            sum += *value;
        }
        Ok(sum / returns.len() as f64)
    }
}

/// Sample variance of a supplied return series.
pub fn sample_variance(returns: &[f64]) -> Result<f64, PortfolioError> {
    #[cfg(not(test))]
    {
        return crate::solvers::statistics::descriptive::variance(returns, true)
            .ok_or(PortfolioError::InvalidInput);
    }

    #[cfg(test)]
    {
        if returns.len() < 2 || !returns.iter().all(|x| x.is_finite()) {
            return Err(PortfolioError::InvalidInput);
        }
        let mean = mean_return(returns)?;
        let mut sum_sq = 0.0;
        for value in returns {
            let diff = *value - mean;
            sum_sq += diff * diff;
        }
        Ok(sum_sq / (returns.len() - 1) as f64)
    }
}

/// Sample covariance matrix into row-major `asset_count * asset_count` output.
pub fn covariance_matrix_into(
    flat_returns: &[f64],
    period_count: usize,
    asset_count: usize,
    out: &mut [f64],
) -> Result<usize, PortfolioError> {
    validate_shape(flat_returns, period_count, asset_count)?;
    if period_count < 2 {
        return Err(PortfolioError::InvalidInput);
    }
    if out.len() < asset_count * asset_count {
        return Err(PortfolioError::OutputBufferTooSmall);
    }

    let mut means = [0.0f64; 64];
    if asset_count > means.len() {
        return Err(PortfolioError::InvalidInput);
    }

    for period in 0..period_count {
        let offset = period * asset_count;
        for asset in 0..asset_count {
            means[asset] += flat_returns[offset + asset];
        }
    }
    for mean in means.iter_mut().take(asset_count) {
        *mean /= period_count as f64;
    }

    for row in 0..asset_count {
        for col in row..asset_count {
            let mut sum = 0.0;
            for period in 0..period_count {
                let offset = period * asset_count;
                let a = flat_returns[offset + row] - means[row];
                let b = flat_returns[offset + col] - means[col];
                sum += a * b;
            }
            let cov = sum / (period_count - 1) as f64;
            out[row * asset_count + col] = cov;
            out[col * asset_count + row] = cov;
        }
    }
    Ok(asset_count * asset_count)
}

/// Portfolio variance from row-major covariance matrix.
pub fn portfolio_variance_from_covariance(
    weights: &[f64],
    covariance: &[f64],
    asset_count: usize,
) -> Result<f64, PortfolioError> {
    validate_weights(weights, asset_count)?;
    let used_len = asset_count * asset_count;
    if covariance.len() < used_len || !covariance[..used_len].iter().all(|x| x.is_finite()) {
        return Err(PortfolioError::InvalidInput);
    }

    let mut variance = 0.0;
    for row in 0..asset_count {
        for col in 0..asset_count {
            variance += weights[row] * covariance[row * asset_count + col] * weights[col];
        }
    }
    if variance < 0.0 && variance > -1e-15 {
        return Ok(0.0);
    }
    if variance < 0.0 {
        return Err(PortfolioError::InvalidInput);
    }
    Ok(variance)
}

/// Marginal contribution to volatility risk into caller-owned output.
///
/// `out[i] = w_i * (Sigma w)_i / portfolio_volatility`; summing the outputs
/// gives portfolio volatility when the covariance matrix is positive.
pub fn volatility_risk_contributions_into(
    weights: &[f64],
    covariance: &[f64],
    asset_count: usize,
    out: &mut [f64],
) -> Result<usize, PortfolioError> {
    if out.len() < asset_count {
        return Err(PortfolioError::OutputBufferTooSmall);
    }
    let variance = portfolio_variance_from_covariance(weights, covariance, asset_count)?;
    if variance <= 0.0 {
        return Err(PortfolioError::InvalidInput);
    }
    let volatility = variance.sqrt();

    for row in 0..asset_count {
        let mut cov_weight = 0.0;
        for col in 0..asset_count {
            cov_weight += covariance[row * asset_count + col] * weights[col];
        }
        out[row] = weights[row] * cov_weight / volatility;
    }
    Ok(asset_count)
}

/// Maximum drawdown from a supplied return series.
pub fn max_drawdown(returns: &[f64]) -> Result<f64, PortfolioError> {
    if returns.is_empty() || !returns.iter().all(|x| x.is_finite() && *x > -1.0) {
        return Err(PortfolioError::InvalidInput);
    }
    let mut wealth = 1.0;
    let mut peak = 1.0;
    let mut worst = 0.0;
    for r in returns {
        wealth *= 1.0 + *r;
        if wealth > peak {
            peak = wealth;
        }
        let drawdown = wealth / peak - 1.0;
        if drawdown < worst {
            worst = drawdown;
        }
    }
    Ok(worst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portfolio_returns_use_row_major_weights() {
        let returns = [0.10, 0.00, 0.00, 0.20];
        let weights = [0.25, 0.75];
        let mut out = [0.0; 2];
        let n = portfolio_returns_into(&returns, 2, 2, &weights, &mut out).unwrap();
        assert_eq!(n, 2);
        assert!((out[0] - 0.025).abs() < 1e-12);
        assert!((out[1] - 0.15).abs() < 1e-12);
    }

    #[test]
    fn covariance_matrix_matches_hand_values() {
        let returns = [1.0, 2.0, 2.0, 4.0, 3.0, 6.0];
        let mut cov = [0.0; 4];
        covariance_matrix_into(&returns, 3, 2, &mut cov).unwrap();
        assert!((cov[0] - 1.0).abs() < 1e-12);
        assert!((cov[1] - 2.0).abs() < 1e-12);
        assert!((cov[2] - 2.0).abs() < 1e-12);
        assert!((cov[3] - 4.0).abs() < 1e-12);
    }

    #[test]
    fn portfolio_variance_matches_quadratic_form() {
        let weights = [0.5, 0.5];
        let cov = [1.0, 2.0, 2.0, 4.0];
        let variance = portfolio_variance_from_covariance(&weights, &cov, 2).unwrap();
        assert!((variance - 2.25).abs() < 1e-12);
    }

    #[test]
    fn portfolio_variance_ignores_unused_covariance_suffix() {
        let weights = [1.0];
        let cov = [0.04, f64::NAN];
        let variance = portfolio_variance_from_covariance(&weights, &cov, 1).unwrap();
        assert!((variance - 0.04).abs() < 1e-12);
    }

    #[test]
    fn risk_contributions_sum_to_volatility() {
        let weights = [0.5, 0.5];
        let cov = [1.0, 0.0, 0.0, 4.0];
        let mut out = [0.0; 2];
        volatility_risk_contributions_into(&weights, &cov, 2, &mut out).unwrap();
        let volatility = portfolio_variance_from_covariance(&weights, &cov, 2)
            .unwrap()
            .sqrt();
        assert!((out[0] + out[1] - volatility).abs() < 1e-12);
    }

    #[test]
    fn max_drawdown_tracks_peak_to_trough() {
        let returns = [0.10, -0.20, 0.05];
        let dd = max_drawdown(&returns).unwrap();
        assert!((dd + 0.20).abs() < 1e-12);
    }

    #[test]
    fn oversized_asset_count_is_rejected_for_stack_means() {
        let returns = [0.0; 65];
        let mut cov = [0.0; 65 * 65];
        assert_eq!(
            covariance_matrix_into(&returns, 1, 65, &mut cov),
            Err(PortfolioError::InvalidInput)
        );
    }
}
