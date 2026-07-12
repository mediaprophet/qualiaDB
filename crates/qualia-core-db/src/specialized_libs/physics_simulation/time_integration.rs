use super::*;

/// Time integrator
pub struct TimeIntegrator {
    integrator_type: TimeIntegratorType,
    time_step_control: TimeStepControl,
    stability_analysis: StabilityAnalysis,
}

/// Time integrator types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeIntegratorType {
    /// Explicit Euler method
    ExplicitEuler,
    /// Implicit Euler method
    ImplicitEuler,
    /// Runge-Kutta methods
    RungeKutta,
    /// Adams-Bashforth methods
    AdamsBashforth,
    /// Crank-Nicolson method
    CrankNicolson,
    /// Leapfrog method
    Leapfrog,
    /// Verlet integration
    Verlet,
    /// Newmark-beta method
    NewmarkBeta,
    /// Generalized alpha method
    GeneralizedAlpha,
}

/// Time step control
pub struct TimeStepControl {
    control_type: TimeStepControlType,
    pub(super) cfl_condition: CflCondition,
    pub(super) adaptive_parameters: AdaptiveParameters,
    pub(super) current_time_step: f64,
}

/// Time step control types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeStepControlType {
    /// Fixed time step
    Fixed,
    /// CFL-based adaptive
    CFLBased,
    /// Error-based adaptive
    ErrorBased,
    /// Multi-scale adaptive
    MultiScale,
}

/// CFL conditions
#[derive(Debug, Clone)]
pub struct CflCondition {
    pub cfl_number: f64,
    pub velocity_field: Option<Vec<f64>>,
    pub sound_speed: Option<f64>,
    pub diffusion_coefficient: Option<f64>,
}

/// Adaptive parameters
#[derive(Debug, Clone)]
pub struct AdaptiveParameters {
    pub min_time_step: f64,
    pub max_time_step: f64,
    pub safety_factor: f64,
    pub max_increase_factor: f64,
    pub max_decrease_factor: f64,
}

/// Stability analysis
pub struct StabilityAnalysis {
    analysis_method: StabilityAnalysisMethod,
    eigenvalue_analysis: EigenvalueAnalysis,
    von_neumann_analysis: VonNeumannAnalysis,
}

/// Stability analysis methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StabilityAnalysisMethod {
    /// Von Neumann analysis
    VonNeumann,
    /// Energy method
    Energy,
    /// Matrix method
    Matrix,
    /// Spectral radius method
    SpectralRadius,
}

/// Eigenvalue analysis
#[derive(Debug, Clone)]
pub struct EigenvalueAnalysis {
    pub eigenvalues: Vec<f64>,
    pub eigenvectors: Vec<Vec<f64>>,
    pub spectral_radius: f64,
}

/// Von Neumann analysis
#[derive(Debug, Clone)]
pub struct VonNeumannAnalysis {
    pub amplification_factor: f64,
    pub phase_speed: f64,
    pub dispersion_relation: String,
}

impl TimeIntegrator {
    pub fn new() -> Self {
        Self {
            integrator_type: TimeIntegratorType::ExplicitEuler,
            time_step_control: TimeStepControl::new(),
            stability_analysis: StabilityAnalysis::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), PhysicsError> {
        self.time_step_control.initialize()?;
        self.stability_analysis.initialize()?;
        Ok(())
    }

    /// Compute an adaptive time step for the given field.
    ///
    /// If the time-step control is CFL-based, the CFL dt is computed from the
    /// field's maximum absolute velocity and the field's spatial resolution
    /// (estimated as `1.0 / n` when no explicit `dx` is available). Otherwise
    /// the fixed `dt` argument is returned unchanged.
    pub fn adaptive_step(&mut self, field: &PhysicsField, dt: f64) -> f64 {
        if self.time_step_control.control_type == TimeStepControlType::CFLBased {
            let max_velocity = field.data.iter().map(|&v| v.abs()).fold(0.0f64, f64::max);

            // Estimate dx from the first dimension length.
            let dx = field
                .dimensions
                .first()
                .map(|&n| if n > 1 { 1.0 / (n as f64 - 1.0) } else { 1.0 })
                .unwrap_or(1.0);

            self.time_step_control.update_dt(max_velocity, dx)
        } else {
            dt
        }
    }

    /// Get the integrator type.
    pub fn get_integrator_type(&self) -> &TimeIntegratorType {
        &self.integrator_type
    }

    /// Set the integrator type.
    pub fn set_integrator_type(&mut self, integrator_type: TimeIntegratorType) {
        self.integrator_type = integrator_type;
    }
}

impl TimeStepControl {
    pub fn new() -> Self {
        Self {
            control_type: TimeStepControlType::CFLBased,
            cfl_condition: CflCondition::new(),
            adaptive_parameters: AdaptiveParameters::new(),
            current_time_step: 0.001,
        }
    }

    pub fn initialize(&mut self) -> Result<(), PhysicsError> {
        Ok(())
    }

    /// Compute the CFL-limited time step: dt = CFL * dx / max_velocity.
    ///
    /// The result is clamped to `[min_time_step, max_time_step]`. If `max_velocity`
    /// is zero or non-finite, `max_time_step` is returned (no advective limit).
    pub fn compute_cfl_dt(&self, max_velocity: f64, dx: f64) -> f64 {
        let min_dt = self.adaptive_parameters.min_time_step;
        let max_dt = self.adaptive_parameters.max_time_step;

        if !max_velocity.is_finite() || max_velocity <= 0.0 || dx <= 0.0 {
            return max_dt;
        }

        let raw_dt = self.cfl_condition.cfl_number * dx / max_velocity;
        raw_dt.clamp(min_dt, max_dt)
    }

    /// Compute the CFL-limited time step using the velocity field (or sound
    /// speed fallback) stored on the `CflCondition`.
    ///
    /// This is the field-driven overload of `compute_cfl_dt`: it derives
    /// `max_velocity` from `CflCondition::max_velocity()` instead of requiring
    /// it as a parameter. When neither a velocity field nor a sound speed is
    /// set, `max_velocity()` returns `0.0` and `max_time_step` is returned.
    pub fn compute_cfl_dt_from_field(&self, dx: f64) -> f64 {
        self.compute_cfl_dt(self.cfl_condition.max_velocity(), dx)
    }

    /// Compute the diffusive CFL-limited time step:
    /// `dt_diff = CFL * dx^2 / (2 * diffusion_coeff)`.
    ///
    /// The result is clamped to `[min_time_step, max_time_step]`. Returns `0.0`
    /// for a zero (or non-finite) diffusion coefficient, clamped to the minimum
    /// bound so callers always receive a usable dt.
    pub fn compute_diffusion_dt(&self, diffusion_coeff: f64, dx: f64) -> f64 {
        let min_dt = self.adaptive_parameters.min_time_step;
        let max_dt = self.adaptive_parameters.max_time_step;

        if !diffusion_coeff.is_finite() || diffusion_coeff <= 0.0 || dx <= 0.0 {
            return max_dt;
        }

        let raw_dt = self.cfl_condition.cfl_number * dx * dx / (2.0 * diffusion_coeff);
        raw_dt.clamp(min_dt, max_dt)
    }

    /// Compute the combined advective + diffusive CFL limit and return the
    /// most restrictive (minimum) of the two, clamped to
    /// `[min_time_step, max_time_step]`.
    pub fn compute_combined_dt(&self, max_velocity: f64, diffusion_coeff: f64, dx: f64) -> f64 {
        let advective = self.compute_cfl_dt(max_velocity, dx);
        let diffusive = self.compute_diffusion_dt(diffusion_coeff, dx);
        let min_dt = self.adaptive_parameters.min_time_step;
        let max_dt = self.adaptive_parameters.max_time_step;
        advective.min(diffusive).clamp(min_dt, max_dt)
    }

    /// Compute a new adaptive time step using the CFL condition, apply the safety
    /// factor and increase/decrease limits, update the internal `current_time_step`,
    /// and return the new dt.
    pub fn update_dt(&mut self, max_velocity: f64, dx: f64) -> f64 {
        let cfl_dt = self.compute_cfl_dt(max_velocity, dx);

        // Apply the safety factor and absolute clamping via AdaptiveParameters.
        let safe_dt = self.adaptive_parameters.apply_safety_factor(cfl_dt);

        // Limit the rate of change relative to the previous time step.
        let new_dt = if self.current_time_step > 0.0 {
            let lower = self.current_time_step * self.adaptive_parameters.max_decrease_factor;
            let upper = self.current_time_step * self.adaptive_parameters.max_increase_factor;
            safe_dt.clamp(lower, upper)
        } else {
            safe_dt
        };

        // Clamp to the absolute bounds once more after the relative limiter.
        let new_dt = self.adaptive_parameters.clamp_dt(new_dt);

        self.current_time_step = new_dt;
        new_dt
    }
}

impl CflCondition {
    pub fn new() -> Self {
        Self {
            cfl_number: 0.5,
            velocity_field: None,
            sound_speed: Some(343.0), // Speed of sound in air at 20°C
            diffusion_coefficient: None,
        }
    }

    /// Set the velocity field used for CFL advective time-step estimation.
    pub fn set_velocity_field(&mut self, velocities: Vec<f64>) {
        self.velocity_field = Some(velocities);
    }

    /// Set the sound speed used as a fallback wave speed when no velocity
    /// field is populated.
    pub fn set_sound_speed(&mut self, speed: f64) {
        self.sound_speed = Some(speed);
    }

    /// Set the diffusion coefficient used for the diffusive CFL limit.
    pub fn set_diffusion_coefficient(&mut self, coeff: f64) {
        self.diffusion_coefficient = Some(coeff);
    }

    /// Return the maximum absolute velocity from the velocity field.
    ///
    /// Falls back to `sound_speed` when no velocity field is present, and to
    /// `0.0` when neither is set. Non-finite entries in the velocity field are
    /// ignored.
    pub fn max_velocity(&self) -> f64 {
        if let Some(field) = &self.velocity_field {
            field
                .iter()
                .copied()
                .filter(|v| v.is_finite())
                .map(|v| v.abs())
                .fold(0.0f64, f64::max)
        } else if let Some(c) = self.sound_speed {
            if c.is_finite() {
                c
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Accessor for the velocity field, if populated.
    pub fn get_velocity_field(&self) -> Option<&Vec<f64>> {
        self.velocity_field.as_ref()
    }
}

impl AdaptiveParameters {
    pub fn new() -> Self {
        Self {
            min_time_step: 1e-6,
            max_time_step: 1.0,
            safety_factor: 0.9,
            max_increase_factor: 2.0,
            max_decrease_factor: 0.5,
        }
    }

    /// Constructor with explicit bounds and safety factor. The relative
    /// increase/decrease limits keep their defaults.
    pub fn with_values(min_ts: f64, max_ts: f64, safety: f64) -> Self {
        Self {
            min_time_step: min_ts,
            max_time_step: max_ts,
            safety_factor: safety,
            max_increase_factor: 2.0,
            max_decrease_factor: 0.5,
        }
    }

    /// Clamp `dt` to `[min_time_step, max_time_step]`.
    pub fn clamp_dt(&self, dt: f64) -> f64 {
        dt.clamp(self.min_time_step, self.max_time_step)
    }

    /// Multiply `dt` by the safety factor, then clamp to the absolute bounds.
    pub fn apply_safety_factor(&self, dt: f64) -> f64 {
        self.clamp_dt(dt * self.safety_factor)
    }
}

impl StabilityAnalysis {
    pub fn new() -> Self {
        Self {
            analysis_method: StabilityAnalysisMethod::VonNeumann,
            eigenvalue_analysis: EigenvalueAnalysis::new(),
            von_neumann_analysis: VonNeumannAnalysis::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), PhysicsError> {
        Ok(())
    }

    /// Get the analysis method.
    pub fn get_analysis_method(&self) -> &StabilityAnalysisMethod {
        &self.analysis_method
    }

    /// Set the analysis method.
    pub fn set_analysis_method(&mut self, method: StabilityAnalysisMethod) {
        self.analysis_method = method;
    }

    /// Get a reference to the eigenvalue analysis.
    pub fn get_eigenvalue_analysis(&self) -> &EigenvalueAnalysis {
        &self.eigenvalue_analysis
    }

    /// Get a mutable reference to the eigenvalue analysis.
    pub fn get_eigenvalue_analysis_mut(&mut self) -> &mut EigenvalueAnalysis {
        &mut self.eigenvalue_analysis
    }

    /// Get a reference to the von Neumann analysis.
    pub fn get_von_neumann_analysis(&self) -> &VonNeumannAnalysis {
        &self.von_neumann_analysis
    }

    /// Get a mutable reference to the von Neumann analysis.
    pub fn get_von_neumann_analysis_mut(&mut self) -> &mut VonNeumannAnalysis {
        &mut self.von_neumann_analysis
    }
}

impl EigenvalueAnalysis {
    pub fn new() -> Self {
        Self {
            eigenvalues: Vec::new(),
            eigenvectors: Vec::new(),
            spectral_radius: 0.0,
        }
    }
}

impl VonNeumannAnalysis {
    pub fn new() -> Self {
        Self {
            amplification_factor: 1.0,
            phase_speed: 0.0,
            dispersion_relation: "k^2 = omega^2 / c^2".to_string(),
        }
    }
}
