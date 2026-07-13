//! Calibrated differential-privacy mechanisms and privacy-loss accounting.
//!
//! The caller supplies the query result, output buffer, sensitivity, and entropy source.
//! One budget charge covers the whole vector, so `sensitivity` must be the L1 (Laplace)
//! or L2 (Gaussian) sensitivity of that vector-valued query.

use core::f64::consts::TAU;

/// Noise mechanisms with implemented calibration and sampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NoiseMechanism {
    Laplace = 1,
    Gaussian = 2,
}

/// Privacy-loss composition used by [`PrivacyAccountant`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompositionMethod {
    /// Conservative sequential composition: ε and δ add.
    BasicComposition,
    /// Generalized advanced composition with a caller-selected δ slack.
    AdvancedComposition { delta_slack: f64 },
    /// Rényi-DP accounting for Gaussian mechanisms at a fixed order α.
    RdpComposition { order: f64, target_delta: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyError {
    InvalidEpsilon,
    InvalidDelta,
    InvalidSensitivity,
    NonFiniteInput,
    OutputBufferTooSmall,
    BudgetExceeded,
    EntropyUnavailable,
    UnsupportedComposition,
    CapacityExceeded,
}

impl PrivacyError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidEpsilon => "epsilon must be finite and greater than zero",
            Self::InvalidDelta => "delta is outside the mechanism's valid range",
            Self::InvalidSensitivity => "sensitivity must be finite and non-negative",
            Self::NonFiniteInput => "query result contains a non-finite value",
            Self::OutputBufferTooSmall => "caller output buffer is too small",
            Self::BudgetExceeded => "privacy budget exhausted",
            Self::EntropyUnavailable => "operating-system entropy unavailable",
            Self::UnsupportedComposition => {
                "the selected accountant does not support this mechanism"
            }
            Self::CapacityExceeded => "fixed-capacity privacy registry is full",
        }
    }
}

impl core::fmt::Display for PrivacyError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::error::Error for PrivacyError {}

/// Source of cryptographic noise bytes.
pub trait NoiseSource {
    fn fill_bytes(&mut self, destination: &mut [u8]) -> Result<(), PrivacyError>;
}

/// Operating-system cryptographic entropy.
#[derive(Debug, Default)]
pub struct OsNoise;

impl NoiseSource for OsNoise {
    fn fill_bytes(&mut self, destination: &mut [u8]) -> Result<(), PrivacyError> {
        getrandom::fill(destination).map_err(|_| PrivacyError::EntropyUnavailable)
    }
}

/// A total and remaining (ε,δ) budget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrivacyBudget {
    pub epsilon: f64,
    pub delta: f64,
    pub remaining_epsilon: f64,
    pub remaining_delta: f64,
}

impl PrivacyBudget {
    pub fn try_new(epsilon: f64, delta: f64) -> Result<Self, PrivacyError> {
        let budget = Self::new_unchecked(epsilon, delta);
        budget.validate()?;
        Ok(budget)
    }

    pub const fn new_unchecked(epsilon: f64, delta: f64) -> Self {
        Self {
            epsilon,
            delta,
            remaining_epsilon: epsilon,
            remaining_delta: delta,
        }
    }

    pub fn validate(&self) -> Result<(), PrivacyError> {
        validate_epsilon(self.epsilon)?;
        if !self.delta.is_finite() || self.delta < 0.0 || self.delta >= 1.0 {
            return Err(PrivacyError::InvalidDelta);
        }
        Ok(())
    }

    fn apply_composed_cost(&mut self, epsilon: f64, delta: f64) -> Result<(), PrivacyError> {
        const ROUNDING_TOLERANCE: f64 = 1e-12;
        if epsilon > self.epsilon + ROUNDING_TOLERANCE || delta > self.delta + ROUNDING_TOLERANCE {
            return Err(PrivacyError::BudgetExceeded);
        }
        self.remaining_epsilon = (self.epsilon - epsilon).max(0.0);
        self.remaining_delta = (self.delta - delta).max(0.0);
        Ok(())
    }
}

/// Stateful accountant. Fields remain scalar so charging performs no allocation.
#[derive(Debug, Clone, Copy)]
pub struct PrivacyAccountant {
    pub total_epsilon_spent: f64,
    pub total_delta_spent: f64,
    pub composition_method: CompositionMethod,
    releases: u64,
    sum_epsilon_squared: f64,
    sum_epsilon_expm1: f64,
    raw_delta_sum: f64,
    rdp_epsilon: f64,
}

impl PrivacyAccountant {
    pub const fn new(composition_method: CompositionMethod) -> Self {
        Self {
            total_epsilon_spent: 0.0,
            total_delta_spent: 0.0,
            composition_method,
            releases: 0,
            sum_epsilon_squared: 0.0,
            sum_epsilon_expm1: 0.0,
            raw_delta_sum: 0.0,
            rdp_epsilon: 0.0,
        }
    }

    pub const fn releases(&self) -> u64 {
        self.releases
    }

    fn record(
        &mut self,
        mechanism: NoiseMechanism,
        epsilon: f64,
        delta: f64,
        gaussian_rdp_increment: f64,
    ) -> Result<(), PrivacyError> {
        match self.composition_method {
            CompositionMethod::BasicComposition => {
                self.total_epsilon_spent += epsilon;
                self.total_delta_spent += delta;
            }
            CompositionMethod::AdvancedComposition { delta_slack } => {
                if !delta_slack.is_finite() || delta_slack <= 0.0 || delta_slack >= 1.0 {
                    return Err(PrivacyError::InvalidDelta);
                }
                self.sum_epsilon_squared += epsilon * epsilon;
                self.sum_epsilon_expm1 += epsilon * epsilon.exp_m1();
                self.raw_delta_sum += delta;
                self.total_epsilon_spent =
                    (2.0 * (1.0 / delta_slack).ln() * self.sum_epsilon_squared).sqrt()
                        + self.sum_epsilon_expm1;
                self.total_delta_spent = self.raw_delta_sum + delta_slack;
            }
            CompositionMethod::RdpComposition {
                order,
                target_delta,
            } => {
                if mechanism != NoiseMechanism::Gaussian {
                    return Err(PrivacyError::UnsupportedComposition);
                }
                if !order.is_finite() || order <= 1.0 {
                    return Err(PrivacyError::InvalidEpsilon);
                }
                if !target_delta.is_finite() || target_delta <= 0.0 || target_delta >= 1.0 {
                    return Err(PrivacyError::InvalidDelta);
                }
                self.rdp_epsilon += gaussian_rdp_increment;
                self.total_epsilon_spent =
                    self.rdp_epsilon + (1.0 / target_delta).ln() / (order - 1.0);
                self.total_delta_spent = target_delta;
            }
        }
        self.releases += 1;
        Ok(())
    }
}

/// Differential-privacy release engine with explicit total budget.
pub struct DifferentialPrivacy {
    pub noise_mechanisms: [NoiseMechanism; 2],
    pub privacy_accountant: PrivacyAccountant,
    pub privacy_budget: PrivacyBudget,
}

impl DifferentialPrivacy {
    pub fn new() -> Self {
        Self {
            noise_mechanisms: [NoiseMechanism::Laplace, NoiseMechanism::Gaussian],
            privacy_accountant: PrivacyAccountant::new(CompositionMethod::BasicComposition),
            privacy_budget: PrivacyBudget::new_unchecked(1.0, 1e-6),
        }
    }

    pub fn with_budget(
        epsilon: f64,
        delta: f64,
        composition_method: CompositionMethod,
    ) -> Result<Self, PrivacyError> {
        let result = Self {
            noise_mechanisms: [NoiseMechanism::Laplace, NoiseMechanism::Gaussian],
            privacy_accountant: PrivacyAccountant::new(composition_method),
            privacy_budget: PrivacyBudget::try_new(epsilon, delta)?,
        };
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<(), PrivacyError> {
        self.privacy_budget.validate()?;
        match self.privacy_accountant.composition_method {
            CompositionMethod::BasicComposition => Ok(()),
            CompositionMethod::AdvancedComposition { delta_slack } => {
                if delta_slack.is_finite()
                    && delta_slack > 0.0
                    && delta_slack < self.privacy_budget.delta
                {
                    Ok(())
                } else {
                    Err(PrivacyError::InvalidDelta)
                }
            }
            CompositionMethod::RdpComposition {
                order,
                target_delta,
            } => {
                if order.is_finite()
                    && order > 1.0
                    && target_delta.is_finite()
                    && target_delta > 0.0
                    && target_delta <= self.privacy_budget.delta
                {
                    Ok(())
                } else {
                    Err(PrivacyError::InvalidDelta)
                }
            }
        }
    }

    /// Release with Laplace noise drawn from operating-system cryptographic entropy.
    pub fn release_laplace_into(
        &mut self,
        query_result: &[f64],
        l1_sensitivity: f64,
        epsilon: f64,
        out: &mut [f64],
    ) -> Result<usize, PrivacyError> {
        self.release_laplace_with_noise_into(
            query_result,
            l1_sensitivity,
            epsilon,
            &mut OsNoise,
            out,
        )
    }

    /// Release a vector-valued query with Laplace noise calibrated to L1 sensitivity.
    ///
    /// Custom sources are useful for deterministic tests. A source that is not a
    /// CSPRNG invalidates the differential-privacy guarantee.
    pub fn release_laplace_with_noise_into<R: NoiseSource>(
        &mut self,
        query_result: &[f64],
        l1_sensitivity: f64,
        epsilon: f64,
        noise: &mut R,
        out: &mut [f64],
    ) -> Result<usize, PrivacyError> {
        validate_release(query_result, out, l1_sensitivity)?;
        validate_epsilon(epsilon)?;
        if query_result.is_empty() {
            return Ok(0);
        }
        if l1_sensitivity == 0.0 {
            out[..query_result.len()].copy_from_slice(query_result);
            return Ok(query_result.len());
        }

        let scale = l1_sensitivity / epsilon;
        self.charge(NoiseMechanism::Laplace, epsilon, 0.0, 0.0)?;

        for (destination, &value) in out.iter_mut().zip(query_result) {
            let centered = open_unit_interval(noise)? - 0.5;
            let magnitude = -scale * (1.0 - 2.0 * centered.abs()).ln();
            *destination = value
                + if centered < 0.0 {
                    -magnitude
                } else {
                    magnitude
                };
        }
        Ok(query_result.len())
    }

    /// Release with Gaussian noise drawn from operating-system cryptographic entropy.
    pub fn release_gaussian_into(
        &mut self,
        query_result: &[f64],
        l2_sensitivity: f64,
        epsilon: f64,
        delta: f64,
        out: &mut [f64],
    ) -> Result<usize, PrivacyError> {
        self.release_gaussian_with_noise_into(
            query_result,
            l2_sensitivity,
            epsilon,
            delta,
            &mut OsNoise,
            out,
        )
    }

    /// Release a vector-valued query with the classic calibrated Gaussian mechanism.
    ///
    /// This sufficient calibration is valid for `0 < epsilon <= 1` and `0 < delta < 1`:
    /// `σ = sensitivity * sqrt(2 ln(1.25 / δ)) / ε`.
    pub fn release_gaussian_with_noise_into<R: NoiseSource>(
        &mut self,
        query_result: &[f64],
        l2_sensitivity: f64,
        epsilon: f64,
        delta: f64,
        noise: &mut R,
        out: &mut [f64],
    ) -> Result<usize, PrivacyError> {
        validate_release(query_result, out, l2_sensitivity)?;
        let sigma = gaussian_sigma(l2_sensitivity, epsilon, delta)?;
        if query_result.is_empty() {
            return Ok(0);
        }
        if l2_sensitivity == 0.0 {
            out[..query_result.len()].copy_from_slice(query_result);
            return Ok(query_result.len());
        }

        let order = match self.privacy_accountant.composition_method {
            CompositionMethod::RdpComposition { order, .. } => order,
            _ => 0.0,
        };
        let rdp_increment = if order > 1.0 {
            order * l2_sensitivity * l2_sensitivity / (2.0 * sigma * sigma)
        } else {
            0.0
        };
        self.charge(NoiseMechanism::Gaussian, epsilon, delta, rdp_increment)?;

        let mut index = 0;
        while index < query_result.len() {
            let u1 = open_unit_interval(noise)?;
            let u2 = open_unit_interval(noise)?;
            let radius = (-2.0 * u1.ln()).sqrt();
            let angle = TAU * u2;
            out[index] = query_result[index] + sigma * radius * angle.cos();
            index += 1;
            if index < query_result.len() {
                out[index] = query_result[index] + sigma * radius * angle.sin();
                index += 1;
            }
        }
        Ok(query_result.len())
    }

    fn charge(
        &mut self,
        mechanism: NoiseMechanism,
        epsilon: f64,
        delta: f64,
        gaussian_rdp_increment: f64,
    ) -> Result<(), PrivacyError> {
        let mut prospective = self.privacy_accountant;
        prospective.record(mechanism, epsilon, delta, gaussian_rdp_increment)?;
        self.privacy_budget.apply_composed_cost(
            prospective.total_epsilon_spent,
            prospective.total_delta_spent,
        )?;
        self.privacy_accountant = prospective;
        Ok(())
    }
}

impl Default for DifferentialPrivacy {
    fn default() -> Self {
        Self::new()
    }
}

pub fn gaussian_sigma(sensitivity: f64, epsilon: f64, delta: f64) -> Result<f64, PrivacyError> {
    validate_sensitivity(sensitivity)?;
    validate_epsilon(epsilon)?;
    if epsilon > 1.0 {
        return Err(PrivacyError::InvalidEpsilon);
    }
    if !delta.is_finite() || delta <= 0.0 || delta >= 1.0 {
        return Err(PrivacyError::InvalidDelta);
    }
    Ok(sensitivity * (2.0 * (1.25 / delta).ln()).sqrt() / epsilon)
}

fn validate_release(
    query_result: &[f64],
    out: &[f64],
    sensitivity: f64,
) -> Result<(), PrivacyError> {
    if out.len() < query_result.len() {
        return Err(PrivacyError::OutputBufferTooSmall);
    }
    validate_sensitivity(sensitivity)?;
    if query_result.iter().any(|value| !value.is_finite()) {
        return Err(PrivacyError::NonFiniteInput);
    }
    Ok(())
}

fn validate_epsilon(epsilon: f64) -> Result<(), PrivacyError> {
    if !epsilon.is_finite() || epsilon <= 0.0 {
        Err(PrivacyError::InvalidEpsilon)
    } else {
        Ok(())
    }
}

fn validate_sensitivity(sensitivity: f64) -> Result<(), PrivacyError> {
    if !sensitivity.is_finite() || sensitivity < 0.0 {
        Err(PrivacyError::InvalidSensitivity)
    } else {
        Ok(())
    }
}

fn open_unit_interval<R: NoiseSource>(noise: &mut R) -> Result<f64, PrivacyError> {
    let mut bytes = [0_u8; 8];
    noise.fill_bytes(&mut bytes)?;
    let mantissa = u64::from_le_bytes(bytes) >> 11;
    Ok((mantissa as f64 + 0.5) * (1.0 / ((1_u64 << 53) as f64)))
}

#[cfg(test)]
mod tests;
