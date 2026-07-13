//! Caller-buffered adaptive Dormand-Prince ODE integration.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OdeError {
    InvalidDomain,
    DimensionMismatch,
    WorkspaceTooSmall { required: usize, available: usize },
    NonFiniteDerivative,
    StepUnderflow,
    StepLimitExceeded,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveOdeConfig {
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
    pub initial_step: f64,
    pub minimum_step: f64,
    pub maximum_step: f64,
    pub max_steps: u32,
}

impl Default for AdaptiveOdeConfig {
    fn default() -> Self {
        Self {
            absolute_tolerance: 1e-9,
            relative_tolerance: 1e-7,
            initial_step: 1e-3,
            minimum_step: 1e-14,
            maximum_step: 1.0,
            max_steps: 100_000,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveOdeResult {
    pub final_time: f64,
    pub accepted_steps: u32,
    pub rejected_steps: u32,
    pub derivative_evaluations: u32,
    pub last_error_norm: f64,
    pub last_step: f64,
}

pub const fn dopri5_workspace_len(dimension: usize) -> Option<usize> {
    dimension.checked_mul(8)
}

pub fn integrate_dopri5<F>(
    derivative: F,
    state: &mut [f64],
    t0: f64,
    t_final: f64,
    config: AdaptiveOdeConfig,
    workspace: &mut [f64],
) -> Result<AdaptiveOdeResult, OdeError>
where
    F: Fn(f64, &[f64], &mut [f64]) -> Result<(), OdeError>,
{
    let dimension = state.len();
    let required = dopri5_workspace_len(dimension).ok_or(OdeError::InvalidDomain)?;
    if dimension == 0
        || !t0.is_finite()
        || !t_final.is_finite()
        || t_final < t0
        || state.iter().any(|value| !value.is_finite())
        || !valid_config(config)
    {
        return Err(OdeError::InvalidDomain);
    }
    if workspace.len() < required {
        return Err(OdeError::WorkspaceTooSmall {
            required,
            available: workspace.len(),
        });
    }
    if t_final == t0 {
        return Ok(AdaptiveOdeResult {
            final_time: t0,
            accepted_steps: 0,
            rejected_steps: 0,
            derivative_evaluations: 0,
            last_error_norm: 0.0,
            last_step: 0.0,
        });
    }

    let (k1, rest) = workspace.split_at_mut(dimension);
    let (k2, rest) = rest.split_at_mut(dimension);
    let (k3, rest) = rest.split_at_mut(dimension);
    let (k4, rest) = rest.split_at_mut(dimension);
    let (k5, rest) = rest.split_at_mut(dimension);
    let (k6, rest) = rest.split_at_mut(dimension);
    let (k7, rest) = rest.split_at_mut(dimension);
    let temp = &mut rest[..dimension];

    let mut time = t0;
    let mut step = config.initial_step.min(config.maximum_step);
    let mut accepted = 0;
    let mut rejected = 0;
    let mut evaluations = 0;
    let mut last_error = f64::INFINITY;
    let mut last_step = 0.0;

    for _ in 0..config.max_steps {
        if time >= t_final {
            return Ok(AdaptiveOdeResult {
                final_time: time,
                accepted_steps: accepted,
                rejected_steps: rejected,
                derivative_evaluations: evaluations,
                last_error_norm: last_error,
                last_step,
            });
        }
        step = step.min(t_final - time).min(config.maximum_step);
        if step < config.minimum_step || time + step == time {
            return Err(OdeError::StepUnderflow);
        }

        eval(&derivative, time, state, k1)?;
        stage(state, temp, step, &[(1.0 / 5.0, k1)]);
        eval(&derivative, time + step / 5.0, temp, k2)?;

        stage(state, temp, step, &[(3.0 / 40.0, k1), (9.0 / 40.0, k2)]);
        eval(&derivative, time + 3.0 * step / 10.0, temp, k3)?;

        stage(
            state,
            temp,
            step,
            &[(44.0 / 45.0, k1), (-56.0 / 15.0, k2), (32.0 / 9.0, k3)],
        );
        eval(&derivative, time + 4.0 * step / 5.0, temp, k4)?;

        stage(
            state,
            temp,
            step,
            &[
                (19372.0 / 6561.0, k1),
                (-25360.0 / 2187.0, k2),
                (64448.0 / 6561.0, k3),
                (-212.0 / 729.0, k4),
            ],
        );
        eval(&derivative, time + 8.0 * step / 9.0, temp, k5)?;

        stage(
            state,
            temp,
            step,
            &[
                (9017.0 / 3168.0, k1),
                (-355.0 / 33.0, k2),
                (46732.0 / 5247.0, k3),
                (49.0 / 176.0, k4),
                (-5103.0 / 18656.0, k5),
            ],
        );
        eval(&derivative, time + step, temp, k6)?;

        stage(
            state,
            temp,
            step,
            &[
                (35.0 / 384.0, k1),
                (500.0 / 1113.0, k3),
                (125.0 / 192.0, k4),
                (-2187.0 / 6784.0, k5),
                (11.0 / 84.0, k6),
            ],
        );
        eval(&derivative, time + step, temp, k7)?;
        evaluations += 7;

        let mut error_norm = 0.0_f64;
        for index in 0..dimension {
            let fifth = state[index]
                + step
                    * (35.0 / 384.0 * k1[index]
                        + 500.0 / 1113.0 * k3[index]
                        + 125.0 / 192.0 * k4[index]
                        - 2187.0 / 6784.0 * k5[index]
                        + 11.0 / 84.0 * k6[index]);
            let fourth = state[index]
                + step
                    * (5179.0 / 57600.0 * k1[index]
                        + 7571.0 / 16695.0 * k3[index]
                        + 393.0 / 640.0 * k4[index]
                        - 92097.0 / 339200.0 * k5[index]
                        + 187.0 / 2100.0 * k6[index]
                        + 1.0 / 40.0 * k7[index]);
            let scale = config.absolute_tolerance
                + config.relative_tolerance * state[index].abs().max(fifth.abs());
            error_norm = error_norm.max((fifth - fourth).abs() / scale);
            temp[index] = fifth;
        }
        if !error_norm.is_finite() || temp.iter().any(|value| !value.is_finite()) {
            return Err(OdeError::NonFiniteDerivative);
        }

        last_error = error_norm;
        let factor = if error_norm == 0.0 {
            5.0
        } else {
            (0.9 * error_norm.powf(-0.2)).clamp(0.2, 5.0)
        };
        if error_norm <= 1.0 {
            state.copy_from_slice(temp);
            time += step;
            last_step = step;
            accepted += 1;
        } else {
            rejected += 1;
        }
        step = (step * factor).clamp(config.minimum_step, config.maximum_step);
    }

    Err(OdeError::StepLimitExceeded)
}

fn valid_config(config: AdaptiveOdeConfig) -> bool {
    config.absolute_tolerance.is_finite()
        && config.absolute_tolerance > 0.0
        && config.relative_tolerance.is_finite()
        && config.relative_tolerance >= 0.0
        && config.initial_step.is_finite()
        && config.initial_step > 0.0
        && config.minimum_step.is_finite()
        && config.minimum_step > 0.0
        && config.maximum_step.is_finite()
        && config.maximum_step >= config.minimum_step
        && config.max_steps > 0
}

fn eval<F>(derivative: &F, time: f64, state: &[f64], output: &mut [f64]) -> Result<(), OdeError>
where
    F: Fn(f64, &[f64], &mut [f64]) -> Result<(), OdeError>,
{
    derivative(time, state, output)?;
    if output.len() != state.len() {
        return Err(OdeError::DimensionMismatch);
    }
    if output.iter().any(|value| !value.is_finite()) {
        return Err(OdeError::NonFiniteDerivative);
    }
    Ok(())
}

fn stage(state: &[f64], output: &mut [f64], step: f64, terms: &[(f64, &[f64])]) {
    for index in 0..state.len() {
        let mut value = state[index];
        for (coefficient, derivative) in terms {
            value += step * coefficient * derivative[index];
        }
        output[index] = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decay(_time: f64, state: &[f64], output: &mut [f64]) -> Result<(), OdeError> {
        if state.len() != output.len() {
            return Err(OdeError::DimensionMismatch);
        }
        for (out, value) in output.iter_mut().zip(state) {
            *out = -*value;
        }
        Ok(())
    }

    #[test]
    fn dopri5_lands_exactly_at_final_time_with_tolerance_control() {
        let mut state = [1.0, 2.0, 3.0];
        let mut workspace = [0.0; 24];
        let report = integrate_dopri5(
            decay,
            &mut state,
            0.0,
            2.0,
            AdaptiveOdeConfig::default(),
            &mut workspace,
        )
        .unwrap();
        assert_eq!(report.final_time, 2.0);
        for (index, value) in state.iter().enumerate() {
            let expected = (index + 1) as f64 * (-2.0_f64).exp();
            assert!((value - expected).abs() < 3e-8);
        }
        assert!(report.accepted_steps > 0);
    }

    #[test]
    fn dopri5_rejects_workspace_and_non_finite_derivatives() {
        let mut state = [1.0, 2.0];
        let mut too_small = [0.0; 15];
        assert_eq!(
            integrate_dopri5(
                decay,
                &mut state,
                0.0,
                1.0,
                AdaptiveOdeConfig::default(),
                &mut too_small,
            ),
            Err(OdeError::WorkspaceTooSmall {
                required: 16,
                available: 15
            })
        );

        let mut workspace = [0.0; 16];
        assert_eq!(
            integrate_dopri5(
                |_t, _y, out| {
                    out.fill(f64::NAN);
                    Ok(())
                },
                &mut state,
                0.0,
                1.0,
                AdaptiveOdeConfig::default(),
                &mut workspace,
            ),
            Err(OdeError::NonFiniteDerivative)
        );
    }
}
