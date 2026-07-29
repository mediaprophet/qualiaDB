use super::*;

/// Structural dynamics
pub struct StructuralDynamics {
    modal_analysis: ModalAnalysis,
    transient_analysis: TransientAnalysis,
    harmonic_analysis: HarmonicAnalysis,
}

/// Modal analysis
pub struct ModalAnalysis {
    eigenvalue_solver: EigenvalueSolver,
    mode_shapes: Vec<ModeShape>,
    modal_parameters: ModalParameters,
}

/// Eigenvalue solver
#[derive(Debug, Clone)]
pub struct EigenvalueSolver {
    pub solver_type: EigenvalueSolverType,
    pub num_modes: u32,
    pub frequency_range: (f64, f64),
}

/// Eigenvalue solver types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EigenvalueSolverType {
    Lanczos,
    Subspace,
    Power,
    QR,
}

/// Mode shapes
#[derive(Debug, Clone)]
pub struct ModeShape {
    pub mode_number: u32,
    pub natural_frequency: f64,
    pub damping_ratio: f64,
    pub mode_shape_vector: Vec<f64>,
}

/// Modal parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalParameters {
    pub mass_normalization: bool,
    pub participation_factors: Vec<f64>,
    pub effective_mass: Vec<f64>,
}

/// Transient analysis
pub struct TransientAnalysis {
    time_integration: TimeIntegration,
    loading_history: LoadingHistory,
    response_calculation: ResponseCalculation,
}

/// Time integration
#[derive(Debug, Clone)]
pub struct TimeIntegration {
    pub integration_method: IntegrationMethod,
    pub time_step: f64,
    pub total_time: f64,
}

/// Integration methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IntegrationMethod {
    CentralDifference,
    Newmark,
    WilsonTheta,
    HilberHughesTaylor,
}

/// Loading history
#[derive(Debug, Clone)]
pub struct LoadingHistory {
    pub time_points: Vec<f64>,
    pub load_values: Vec<f64>,
    pub load_type: LoadType,
}

/// Load types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LoadType {
    Force,
    Displacement,
    Acceleration,
    Pressure,
    Point,
}

/// Response calculation
#[derive(Debug, Clone)]
pub struct ResponseCalculation {
    pub response_types: Vec<ResponseType>,
    pub calculation_method: CalculationMethod,
}

/// Response types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResponseType {
    Displacement,
    Velocity,
    Acceleration,
    Stress,
    Strain,
}

/// Calculation methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CalculationMethod {
    Direct,
    Modal,
    FrequencyDomain,
}

/// Harmonic analysis
pub struct HarmonicAnalysis {
    frequency_response: FrequencyResponse,
    resonance_detection: ResonanceDetection,
}

/// Frequency response
#[derive(Debug, Clone)]
pub struct FrequencyResponse {
    pub frequencies: Vec<f64>,
    pub response_amplitudes: Vec<f64>,
    pub response_phases: Vec<f64>,
}

/// Resonance detection
#[derive(Debug, Clone)]
pub struct ResonanceDetection {
    pub resonance_frequencies: Vec<f64>,
    pub resonance_amplitudes: Vec<f64>,
    pub quality_factors: Vec<f64>,
}
impl StructuralDynamics {
    pub fn new() -> Self {
        Self {
            modal_analysis: ModalAnalysis::new(),
            transient_analysis: TransientAnalysis::new(),
            harmonic_analysis: HarmonicAnalysis::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), EngineeringError> {
        Ok(())
    }

    /// Borrow the modal-analysis sub-component.
    pub fn modal_analysis(&self) -> &ModalAnalysis {
        &self.modal_analysis
    }

    /// Mutably borrow the modal-analysis sub-component.
    pub fn modal_analysis_mut(&mut self) -> &mut ModalAnalysis {
        &mut self.modal_analysis
    }

    /// Borrow the transient-analysis sub-component.
    pub fn transient_analysis(&self) -> &TransientAnalysis {
        &self.transient_analysis
    }

    /// Mutably borrow the transient-analysis sub-component.
    pub fn transient_analysis_mut(&mut self) -> &mut TransientAnalysis {
        &mut self.transient_analysis
    }

    /// Borrow the harmonic-analysis sub-component.
    pub fn harmonic_analysis(&self) -> &HarmonicAnalysis {
        &self.harmonic_analysis
    }

    /// Mutably borrow the harmonic-analysis sub-component.
    pub fn harmonic_analysis_mut(&mut self) -> &mut HarmonicAnalysis {
        &mut self.harmonic_analysis
    }

    /// Genuine transient time-history analysis for a 1-DOF system (m, c, k) driven
    /// by the configured `loading_history`. Uses explicit integration with `time_step`
    /// up to `total_time` from the `time_integration` configuration.
    pub fn analyze_transient(
        &self,
        mass: f64,
        stiffness: f64,
        damping: f64,
    ) -> Result<DynamicsResults, EngineeringError> {
        if mass <= 0.0 {
            return Err(EngineeringError::ValidationError(
                "mass must be positive".to_string(),
            ));
        }
        let ti = &self.transient_analysis.time_integration;
        let lh = &self.transient_analysis.loading_history;
        if ti.time_step <= 0.0 || ti.total_time <= 0.0 {
            return Err(EngineeringError::ValidationError(
                "time_step and total_time must be positive".to_string(),
            ));
        }

        let num_steps = (ti.total_time / ti.time_step).ceil() as usize;
        let mut positions = Vec::with_capacity(num_steps + 1);
        let mut velocities = Vec::with_capacity(num_steps + 1);
        let mut accelerations = Vec::with_capacity(num_steps + 1);

        let mut pos = 0.0;
        let mut vel = 0.0;
        let dt = ti.time_step;

        for i in 0..=num_steps {
            let t = i as f64 * dt;

            // Interpolate force from loading history
            let mut force = 0.0;
            if !lh.time_points.is_empty() && lh.time_points.len() == lh.load_values.len() {
                if t <= lh.time_points[0] {
                    force = lh.load_values[0];
                } else if t >= *lh.time_points.last().unwrap() {
                    force = *lh.load_values.last().unwrap();
                } else {
                    for j in 0..lh.time_points.len() - 1 {
                        if t >= lh.time_points[j] && t <= lh.time_points[j + 1] {
                            let dt_int = lh.time_points[j + 1] - lh.time_points[j];
                            let df = lh.load_values[j + 1] - lh.load_values[j];
                            let frac = (t - lh.time_points[j]) / dt_int;
                            force = lh.load_values[j] + df * frac;
                            break;
                        }
                    }
                }
            }

            let acc = (force - damping * vel - stiffness * pos) / mass;

            positions.push(pos);
            velocities.push(vel);
            accelerations.push(acc);

            // Symplectic Euler step
            vel += acc * dt;
            pos += vel * dt;
        }

        let final_pos = positions.last().copied().unwrap_or(0.0);
        let final_vel = velocities.last().copied().unwrap_or(0.0);
        let ke = 0.5 * mass * final_vel * final_vel;
        let pe = 0.5 * stiffness * final_pos * final_pos;
        Ok(DynamicsResults {
            positions,
            velocities,
            accelerations,
            kinetic_energy: ke,
            potential_energy: pe,
            total_energy: ke + pe,
            time_steps: (0..=num_steps).map(|i| i as f64 * dt).collect(),
        })
    }
}

impl ModalAnalysis {
    pub fn new() -> Self {
        Self {
            eigenvalue_solver: EigenvalueSolver::new(),
            mode_shapes: Vec::new(),
            modal_parameters: ModalParameters::new(),
        }
    }

    /// Borrow the eigenvalue solver configuration.
    pub fn eigenvalue_solver(&self) -> &EigenvalueSolver {
        &self.eigenvalue_solver
    }

    /// Mutably borrow the eigenvalue solver configuration.
    pub fn eigenvalue_solver_mut(&mut self) -> &mut EigenvalueSolver {
        &mut self.eigenvalue_solver
    }

    /// Append a computed mode shape to the results.
    pub fn add_mode_shape(&mut self, mode: ModeShape) {
        self.mode_shapes.push(mode);
    }

    /// Borrow the computed mode shapes.
    pub fn mode_shapes(&self) -> &[ModeShape] {
        &self.mode_shapes
    }

    /// Borrow the modal parameters.
    pub fn modal_parameters(&self) -> &ModalParameters {
        &self.modal_parameters
    }

    /// Mutably borrow the modal parameters.
    pub fn modal_parameters_mut(&mut self) -> &mut ModalParameters {
        &mut self.modal_parameters
    }

    /// Undamped modal analysis: solves the generalized eigenproblem
    /// `K φ = ω² M φ` for a symmetric stiffness matrix `stiffness` (row-major
    /// `num_dofs × num_dofs`) and a lumped (diagonal) mass matrix `mass_diag`
    /// (`num_dofs` positive entries), wired to the crate's symmetric Jacobi
    /// eigensolver via [`solve_modal_eigen`]. Returns one [`ModeShape`] per DOF,
    /// ordered by ascending **natural angular frequency ω (rad/s)** (stored in
    /// `ModeShape::natural_frequency`), with zero damping (undamped) and the
    /// mass-normalized mode-shape vector (unit maximum component). The result is
    /// also cached in `self.mode_shapes`.
    pub fn analyze_modal(
        &mut self,
        stiffness: &[f64],
        mass_diag: &[f64],
        num_dofs: usize,
    ) -> Result<Vec<ModeShape>, EngineeringError> {
        let modes = solve_modal_eigen(stiffness, mass_diag, num_dofs)?;
        let shapes: Vec<ModeShape> = modes
            .into_iter()
            .enumerate()
            .map(|(i, (omega, phi))| ModeShape {
                mode_number: (i + 1) as u32,
                natural_frequency: omega,
                damping_ratio: 0.0,
                mode_shape_vector: phi,
            })
            .collect();
        self.mode_shapes = shapes.clone();
        Ok(shapes)
    }
}

impl EigenvalueSolver {
    pub fn new() -> Self {
        Self {
            solver_type: EigenvalueSolverType::Lanczos,
            num_modes: 10,
            frequency_range: (0.0, 1000.0),
        }
    }
}

impl ModalParameters {
    pub fn new() -> Self {
        Self {
            mass_normalization: true,
            participation_factors: Vec::new(),
            effective_mass: Vec::new(),
        }
    }
}

impl TransientAnalysis {
    pub fn new() -> Self {
        Self {
            time_integration: TimeIntegration::new(),
            loading_history: LoadingHistory::new(),
            response_calculation: ResponseCalculation::new(),
        }
    }

    /// Borrow the time-integration configuration.
    pub fn time_integration(&self) -> &TimeIntegration {
        &self.time_integration
    }

    /// Mutably borrow the time-integration configuration.
    pub fn time_integration_mut(&mut self) -> &mut TimeIntegration {
        &mut self.time_integration
    }

    /// Borrow the loading history.
    pub fn loading_history(&self) -> &LoadingHistory {
        &self.loading_history
    }

    /// Mutably borrow the loading history.
    pub fn loading_history_mut(&mut self) -> &mut LoadingHistory {
        &mut self.loading_history
    }

    /// Borrow the response-calculation configuration.
    pub fn response_calculation(&self) -> &ResponseCalculation {
        &self.response_calculation
    }

    /// Mutably borrow the response-calculation configuration.
    pub fn response_calculation_mut(&mut self) -> &mut ResponseCalculation {
        &mut self.response_calculation
    }
}

impl TimeIntegration {
    pub fn new() -> Self {
        Self {
            integration_method: IntegrationMethod::Newmark,
            time_step: 0.01,
            total_time: 10.0,
        }
    }
}

impl LoadingHistory {
    pub fn new() -> Self {
        Self {
            time_points: Vec::new(),
            load_values: Vec::new(),
            load_type: LoadType::Force,
        }
    }
}

impl ResponseCalculation {
    pub fn new() -> Self {
        Self {
            response_types: vec![ResponseType::Displacement, ResponseType::Stress],
            calculation_method: CalculationMethod::Modal,
        }
    }
}

impl HarmonicAnalysis {
    pub fn new() -> Self {
        Self {
            frequency_response: FrequencyResponse::new(),
            resonance_detection: ResonanceDetection::new(),
        }
    }

    /// Borrow the frequency-response data.
    pub fn frequency_response(&self) -> &FrequencyResponse {
        &self.frequency_response
    }

    /// Mutably borrow the frequency-response data.
    pub fn frequency_response_mut(&mut self) -> &mut FrequencyResponse {
        &mut self.frequency_response
    }

    /// Borrow the resonance-detection data.
    pub fn resonance_detection(&self) -> &ResonanceDetection {
        &self.resonance_detection
    }

    /// Mutably borrow the resonance-detection data.
    pub fn resonance_detection_mut(&mut self) -> &mut ResonanceDetection {
        &mut self.resonance_detection
    }
}

impl FrequencyResponse {
    pub fn new() -> Self {
        Self {
            frequencies: Vec::new(),
            response_amplitudes: Vec::new(),
            response_phases: Vec::new(),
        }
    }
}

impl ResonanceDetection {
    pub fn new() -> Self {
        Self {
            resonance_frequencies: Vec::new(),
            resonance_amplitudes: Vec::new(),
            quality_factors: Vec::new(),
        }
    }
}
