use super::*;

/// Vibration analysis
pub struct VibrationAnalysis {
    free_vibration: FreeVibration,
    forced_vibration: ForcedVibration,
    random_vibration: RandomVibration,
}

/// Free vibration
#[derive(Debug, Clone)]
pub struct FreeVibration {
    pub natural_frequencies: Vec<f64>,
    pub mode_shapes: Vec<ModeShape>,
    pub damping_ratios: Vec<f64>,
}

/// Forced vibration
#[derive(Debug, Clone)]
pub struct ForcedVibration {
    pub excitation_frequencies: Vec<f64>,
    pub response_amplitudes: Vec<f64>,
    pub phase_angles: Vec<f64>,
}

/// Random vibration
#[derive(Debug, Clone)]
pub struct RandomVibration {
    pub power_spectral_density: Vec<f64>,
    pub rms_response: f64,
    pub fatigue_damage: f64,
}
impl VibrationAnalysis {
    pub fn new() -> Self {
        Self {
            free_vibration: FreeVibration::new(),
            forced_vibration: ForcedVibration::new(),
            random_vibration: RandomVibration::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), EngineeringError> {
        Ok(())
    }

    /// Borrow the free-vibration sub-component.
    pub fn free_vibration(&self) -> &FreeVibration {
        &self.free_vibration
    }

    /// Mutably borrow the free-vibration sub-component.
    pub fn free_vibration_mut(&mut self) -> &mut FreeVibration {
        &mut self.free_vibration
    }

    /// Borrow the forced-vibration sub-component.
    pub fn forced_vibration(&self) -> &ForcedVibration {
        &self.forced_vibration
    }

    /// Mutably borrow the forced-vibration sub-component.
    pub fn forced_vibration_mut(&mut self) -> &mut ForcedVibration {
        &mut self.forced_vibration
    }

    /// Borrow the random-vibration sub-component.
    pub fn random_vibration(&self) -> &RandomVibration {
        &self.random_vibration
    }

    /// Mutably borrow the random-vibration sub-component.
    pub fn random_vibration_mut(&mut self) -> &mut RandomVibration {
        &mut self.random_vibration
    }

    /// Undamped free-vibration analysis of an `num_dofs`-DOF lumped-mass system.
    /// Delegates to the same generalized eigenproblem as modal analysis
    /// (`K φ = ω² M φ`, wired to `symmetric_eigen` via [`solve_modal_eigen`]) and
    /// packs the result into [`FreeVibration`]: `natural_frequencies` are the
    /// **natural angular frequencies ω (rad/s), ascending**, with their mass-
    /// normalized mode shapes and zero damping ratios (undamped). Cached into
    /// `self.free_vibration`.
    pub fn analyze_free(
        &mut self,
        stiffness: &[f64],
        mass_diag: &[f64],
        num_dofs: usize,
    ) -> Result<FreeVibration, EngineeringError> {
        let modes = solve_modal_eigen(stiffness, mass_diag, num_dofs)?;
        let mut natural_frequencies = Vec::with_capacity(modes.len());
        let mut mode_shapes = Vec::with_capacity(modes.len());
        for (i, (omega, phi)) in modes.into_iter().enumerate() {
            natural_frequencies.push(omega);
            mode_shapes.push(ModeShape {
                mode_number: (i + 1) as u32,
                natural_frequency: omega,
                damping_ratio: 0.0,
                mode_shape_vector: phi,
            });
        }
        let damping_ratios = vec![0.0; natural_frequencies.len()];
        let fv = FreeVibration {
            natural_frequencies,
            mode_shapes,
            damping_ratios,
        };
        self.free_vibration = fv.clone();
        Ok(fv)
    }

    /// Single-DOF undamped natural angular frequency `ω = √(k/m)` (rad/s).
    pub fn natural_frequency_sdof(
        &self,
        stiffness: f64,
        mass: f64,
    ) -> Result<f64, EngineeringError> {
        if mass <= 0.0 {
            return Err(EngineeringError::ValidationError(
                "mass must be positive".to_string(),
            ));
        }
        if stiffness < 0.0 {
            return Err(EngineeringError::ValidationError(
                "stiffness must be non-negative".to_string(),
            ));
        }
        Ok((stiffness / mass).sqrt())
    }

    /// Steady-state harmonic (forced-vibration) response of a damped single-DOF
    /// oscillator `m·ẍ + c·ẋ + k·x = F₀·sin(ωt)`. For each excitation angular
    /// frequency ω (rad/s) in `excitation_freqs`, returns the response amplitude
    /// `X(ω) = F₀ / √((k − m·ω²)² + (c·ω)²)` and the phase lag
    /// `φ(ω) = atan2(c·ω, k − m·ω²)` (rad). Genuine closed-form frequency-response
    /// function; no fabricated values. Fills and returns [`ForcedVibration`].
    pub fn analyze_harmonic_sdof(
        &mut self,
        mass: f64,
        damping: f64,
        stiffness: f64,
        force_amplitude: f64,
        excitation_freqs: &[f64],
    ) -> Result<ForcedVibration, EngineeringError> {
        if mass <= 0.0 {
            return Err(EngineeringError::ValidationError(
                "mass must be positive".to_string(),
            ));
        }
        if damping < 0.0 || stiffness < 0.0 {
            return Err(EngineeringError::ValidationError(
                "damping and stiffness must be non-negative".to_string(),
            ));
        }
        if excitation_freqs.is_empty() {
            return Err(EngineeringError::InsufficientData(
                "no excitation frequencies supplied".to_string(),
            ));
        }
        let mut response_amplitudes = Vec::with_capacity(excitation_freqs.len());
        let mut phase_angles = Vec::with_capacity(excitation_freqs.len());
        for &w in excitation_freqs {
            let re = stiffness - mass * w * w;
            let im = damping * w;
            let denom = (re * re + im * im).sqrt();
            let amp = if denom > 0.0 {
                force_amplitude / denom
            } else {
                f64::INFINITY
            };
            response_amplitudes.push(amp);
            phase_angles.push(im.atan2(re));
        }
        let fv = ForcedVibration {
            excitation_frequencies: excitation_freqs.to_vec(),
            response_amplitudes,
            phase_angles,
        };
        self.forced_vibration = fv.clone();
        Ok(fv)
    }
}

impl FreeVibration {
    pub fn new() -> Self {
        Self {
            natural_frequencies: Vec::new(),
            mode_shapes: Vec::new(),
            damping_ratios: Vec::new(),
        }
    }
}

impl ForcedVibration {
    pub fn new() -> Self {
        Self {
            excitation_frequencies: Vec::new(),
            response_amplitudes: Vec::new(),
            phase_angles: Vec::new(),
        }
    }
}

impl RandomVibration {
    pub fn new() -> Self {
        Self {
            power_spectral_density: Vec::new(),
            rms_response: 0.0,
            fatigue_damage: 0.0,
        }
    }
}
