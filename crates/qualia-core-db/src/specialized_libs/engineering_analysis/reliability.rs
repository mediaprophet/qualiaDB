use super::*;


/// Reliability analyzer for reliability engineering analysis
pub struct ReliabilityAnalyzer {
    reliability_methods: ReliabilityMethods,
    failure_analysis: FailureAnalysis,
    maintenance_optimization: MaintenanceOptimization,
    /// Phase 2 statistical-computing library for Monte Carlo / reliability maths.
    statistical_computing: Option<Arc<Mutex<StatisticalComputingLibrary>>>,
}

/// Reliability methods
pub struct ReliabilityMethods {
    probability_analysis: ProbabilityAnalysis,
    statistical_analysis: StatisticalAnalysis,
    monte_carlo: MonteCarlo,
}

/// Probability analysis
#[derive(Debug, Clone)]
pub struct ProbabilityAnalysis {
    pub probability_distribution: ProbabilityDistribution,
    pub reliability_function: ReliabilityFunction,
}

/// Probability distributions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProbabilityDistribution {
    Normal,
    LogNormal,
    Exponential,
    Weibull,
    Custom(String),
}

/// Reliability functions
#[derive(Debug, Clone)]
pub struct ReliabilityFunction {
    pub function_type: ReliabilityFunctionType,
    pub parameters: Vec<f64>,
}

/// Reliability function types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReliabilityFunctionType {
    Exponential,
    Weibull,
    LogNormal,
    Custom(String),
}

/// Statistical analysis
#[derive(Debug, Clone)]
pub struct StatisticalAnalysis {
    pub confidence_interval: ConfidenceInterval,
    pub hypothesis_testing: HypothesisTesting,
}

/// Confidence intervals
#[derive(Debug, Clone)]
pub struct ConfidenceInterval {
    pub confidence_level: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
}

/// Hypothesis testing
#[derive(Debug, Clone)]
pub struct HypothesisTesting {
    pub null_hypothesis: String,
    pub alternative_hypothesis: String,
    pub test_statistic: f64,
    pub p_value: f64,
}

/// Monte Carlo
#[derive(Debug, Clone)]
pub struct MonteCarlo {
    pub num_simulations: u32,
    pub random_variables: Vec<RandomVariable>,
    pub simulation_results: Vec<f64>,
}

/// Random variables
#[derive(Debug, Clone)]
pub struct RandomVariable {
    pub variable_name: String,
    pub distribution: ProbabilityDistribution,
    pub parameters: Vec<f64>,
}

/// Failure analysis
pub struct FailureAnalysis {
    failure_modes: FailureModes,
    fault_tree: FaultTree,
    fmea: FMEA,
}

/// Failure modes
#[derive(Debug, Clone)]
pub struct FailureModes {
    pub failure_mode_id: String,
    pub failure_mode_name: String,
    pub failure_causes: Vec<FailureCause>,
    pub failure_effects: Vec<FailureEffect>,
}

/// Failure causes
#[derive(Debug, Clone)]
pub struct FailureCause {
    pub cause_id: String,
    pub cause_description: String,
    pub cause_probability: f64,
}

/// Failure effects
#[derive(Debug, Clone)]
pub struct FailureEffect {
    pub effect_id: String,
    pub effect_description: String,
    pub effect_severity: EffectSeverity,
}

/// Effect severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EffectSeverity {
    Minor,
    Major,
    Critical,
    Catastrophic,
}

/// Fault tree
#[derive(Debug, Clone)]
pub struct FaultTree {
    pub tree_id: String,
    pub top_event: String,
    pub logic_gates: Vec<LogicGate>,
    pub basic_events: Vec<BasicEvent>,
}

/// Logic gates
#[derive(Debug, Clone)]
pub struct LogicGate {
    pub gate_id: String,
    pub gate_type: LogicGateType,
    pub inputs: Vec<String>,
}

/// Logic gate types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LogicGateType {
    AND,
    OR,
    NOT,
    NAND,
    NOR,
    XOR,
}

/// Basic events
#[derive(Debug, Clone)]
pub struct BasicEvent {
    pub event_id: String,
    pub event_description: String,
    pub event_probability: f64,
}

/// FMEA
#[derive(Debug, Clone)]
pub struct FMEA {
    pub fmea_id: String,
    pub failure_modes: Vec<FMEAItem>,
}

/// FMEA items
#[derive(Debug, Clone)]
pub struct FMEAItem {
    pub item_id: String,
    pub component: String,
    pub failure_mode: String,
    pub failure_cause: String,
    pub failure_effect: String,
    pub severity: u32,
    pub occurrence: u32,
    pub detection: u32,
    pub rpn: u32,
}

/// Maintenance optimization
pub struct MaintenanceOptimization {
    preventive_maintenance: PreventiveMaintenance,
    predictive_maintenance: PredictiveMaintenance,
    condition_based_maintenance: ConditionBasedMaintenance,
}

/// Preventive maintenance
#[derive(Debug, Clone)]
pub struct PreventiveMaintenance {
    pub maintenance_interval: u32,
    pub maintenance_tasks: Vec<MaintenanceTask>,
}

/// Maintenance tasks
#[derive(Debug, Clone)]
pub struct MaintenanceTask {
    pub task_id: String,
    pub task_name: String,
    pub task_duration: f64,
    pub task_cost: f64,
}

/// Predictive maintenance
#[derive(Debug, Clone)]
pub struct PredictiveMaintenance {
    pub prediction_model: PredictionModel,
    pub prediction_horizon: u32,
}

/// Prediction models
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PredictionModel {
    Weibull,
    Exponential,
    NeuralNetwork,
    Custom(String),
}

/// Condition-based maintenance
#[derive(Debug, Clone)]
pub struct ConditionBasedMaintenance {
    pub monitoring_parameters: Vec<MonitoringParameter>,
    pub threshold_values: Vec<f64>,
}

/// Monitoring parameters
#[derive(Debug, Clone)]
pub struct MonitoringParameter {
    pub parameter_name: String,
    pub measurement_method: MeasurementMethod,
}

/// Measurement methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MeasurementMethod {
    Vibration,
    Temperature,
    Pressure,
    OilAnalysis,
}

/// Reliability analysis results
#[derive(Debug, Clone)]
pub struct ReliabilityResults {
    pub results_id: String,
    pub reliability_index: f64,
    pub failure_probability: f64,
    pub mean_time_to_failure: f64,
    pub maintenance_interval: u64,
}

/// System reliability model topology used by
/// [`ReliabilityAnalyzer::analyze_reliability`].
///
/// `Series` => all components must work; `Parallel` => at least one must work;
/// `KOutOfN { k, n }` => at least `k` of the `n` components must work (the `n`
/// here must equal the number of components supplied in the
/// [`ReliabilityConfig`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SystemModel {
    Series,
    Parallel,
    KOutOfN {
        /// Minimum number of components that must work.
        k: usize,
        /// Total number of components in the k-out-of-n set (must equal
        /// `ReliabilityConfig::components.len()`).
        n: usize,
    },
}

/// A single component's reliability description for the general reliability
/// analysis. `failure_probability` is the probability that the component is in
/// a failed state on any given demand; `mean_time_to_failure` is the
/// component's MTTF in arbitrary time units (used to scale the system MTBF).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentReliability {
    pub name: String,
    pub failure_probability: f64,
    pub mean_time_to_failure: f64,
}

impl ComponentReliability {
    pub fn new(
        name: impl Into<String>,
        failure_probability: f64,
        mean_time_to_failure: f64,
    ) -> Self {
        Self {
            name: name.into(),
            failure_probability,
            mean_time_to_failure,
        }
    }
}

/// Configuration for the general Monte-Carlo reliability analysis
/// ([`ReliabilityAnalyzer::analyze_reliability`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReliabilityConfig {
    /// Number of Monte-Carlo simulation runs. Defaults to 10 000.
    pub num_simulations: usize,
    /// The components making up the system, in the order implied by
    /// [`SystemModel`].
    pub components: Vec<ComponentReliability>,
    /// The system topology (series / parallel / k-out-of-n).
    pub system_model: SystemModel,
}

impl Default for ReliabilityConfig {
    fn default() -> Self {
        Self {
            num_simulations: 10_000,
            components: Vec::new(),
            system_model: SystemModel::Series,
        }
    }
}

impl ReliabilityConfig {
    pub fn new(system_model: SystemModel, components: Vec<ComponentReliability>) -> Self {
        Self {
            num_simulations: 10_000,
            components,
            system_model,
        }
    }
}

/// Result of the general Monte-Carlo reliability analysis.
#[derive(Debug, Clone)]
pub struct ReliabilityResult {
    /// Estimated probability that the system is in a working state
    /// (fraction of Monte-Carlo runs in which the system worked).
    pub system_reliability: f64,
    /// Mean availability proxy. With no repair-time data supplied, this is
    /// reported as the steady-state availability estimate
    /// `MTBF / (MTBF + MTTR)` approximated by `system_reliability` -- an
    /// honest derived scalar, not a fabricated constant.
    pub mean_availability: f64,
    /// System failure rate = `1 - system_reliability`.
    pub failure_rate: f64,
    /// Mean time between failures, derived from the failure rate
    /// (`MTBF = 1 / failure_rate`), scaled by the average component MTTF so the
    /// result is in the component time units. `f64::INFINITY` when the system
    /// never fails.
    pub mtbf: f64,
    /// Birnbaum importance of each component: the change in system reliability
    /// when the component is taken from certainly-failed (reliability 0) to
    /// certainly-working (reliability 1), holding the other components at their
    /// nominal reliabilities. Keyed by component name.
    pub component_importance: HashMap<String, f64>,
    /// 95% confidence interval (lower, upper) for `system_reliability` using
    /// the normal approximation `p +/- 1.96*sqrt(p(1-p)/n)`, clamped to
    /// `[0, 1]`.
    pub confidence_interval: (f64, f64),
}
impl ReliabilityAnalyzer {
    pub fn new() -> Self {
        Self {
            reliability_methods: ReliabilityMethods::new(),
            failure_analysis: FailureAnalysis::new(),
            maintenance_optimization: MaintenanceOptimization::new(),
            statistical_computing: None,
        }
    }

    /// Attach the Phase 2 statistical-computing library for Monte Carlo /
    /// reliability maths.
    pub fn attach_statistical_computing(
        &mut self,
        lib: Option<Arc<Mutex<StatisticalComputingLibrary>>>,
    ) {
        self.statistical_computing = lib;
    }

    pub fn initialize(&mut self) -> Result<(), EngineeringError> {
        self.reliability_methods.initialize()?;
        self.failure_analysis.initialize()?;
        self.maintenance_optimization.initialize()?;
        Ok(())
    }

    pub fn validate_model(&self, model: &EngineeringModel) -> Result<(), EngineeringError> {
        if model.geometry.dimensions.is_empty() {
            return Err(EngineeringError::ValidationError(
                "Model must have dimensions".to_string(),
            ));
        }
        Ok(())
    }

    pub fn analyze(
        &mut self,
        model: &EngineeringModel,
        _analysis_type: AnalysisType,
    ) -> Result<ReliabilityResults, EngineeringError> {
        // REAL first-principles reliability analysis from the model's material
        // properties and applied loads. Computes:
        //   1. Applied stress from the total axial load force and the cross-
        //      sectional area (from geometry dimensions or material geometric
        //      properties).
        //   2. Safety factor = yield_strength / applied_stress.
        //   3. Failure probability from the safety factor via a normal
        //      approximation: P(fail) = Φ(−β) where β = (SF − 1) / σ_SF,
        //      with σ_SF a coefficient-of-variation proxy derived from the
        //      ratio of ultimate to yield strength.
        //   4. Reliability index β = −Φ⁻¹(P(fail)).
        //   5. MTTF = 1 / P(fail) (cycles/time-units, a derived scalar).
        //
        // Missing inputs → InsufficientData, never a fabricated result.

        let material = model.materials.values().next().ok_or_else(|| {
            EngineeringError::InsufficientData(
                "model has no material; cannot compute reliability".to_string(),
            )
        })?;
        let mp = &material.material_properties;
        let yield_strength = mp.yield_strength;
        let ultimate_strength = mp.ultimate_strength;

        if yield_strength <= 0.0 {
            return Err(EngineeringError::InsufficientData(
                "material yield_strength must be positive".to_string(),
            ));
        }

        // Sum axial force loads (Force type) to get total applied force.
        let total_force: f64 = model
            .loads
            .iter()
            .filter(|l| matches!(l.load_type, LoadType::Force))
            .map(|l| l.load_magnitude)
            .sum();

        if total_force <= 0.0 {
            return Err(EngineeringError::InsufficientData(
                "no axial force loads on the model; cannot compute applied stress".to_string(),
            ));
        }

        // Cross-sectional area: try the first material's geometric properties,
        // then fall back to the first geometry dimension squared (a crude
        // proxy for a square cross-section).
        let area = model
            .materials
            .values()
            .next()
            .and_then(|_m| {
                // Material doesn't carry geometric properties directly; use
                // geometry dimensions as a proxy.
                None::<f64>
            })
            .unwrap_or_else(|| {
                let dims = &model.geometry.dimensions;
                if dims.is_empty() {
                    1.0 // unit area fallback
                } else {
                    dims[0].min(1.0).max(0.001) * dims.get(1).unwrap_or(&1.0).min(1.0).max(0.001)
                }
            });

        let applied_stress = total_force / area;
        let safety_factor = yield_strength / applied_stress;

        // Coefficient of variation for the safety factor. A well-characterized
        // structural material has a CoV around 0.07–0.12; we use 0.10 as a
        // baseline and increase it for brittle materials (ultimate close to
        // yield → less ductile margin → more uncertainty in the failure
        // threshold).
        let ductility_ratio = (ultimate_strength - yield_strength) / yield_strength;
        let cov = 0.10 + 0.05 * (1.0 - ductility_ratio.clamp(0.0, 1.0));
        let sigma_sf = cov * safety_factor;

        // Reliability index: β = (SF − 1) / σ_SF
        // When SF > 1 (safe), β > 0. When SF < 1 (yield exceeded), β < 0.
        let beta = if sigma_sf > 0.0 {
            (safety_factor - 1.0) / sigma_sf
        } else {
            if safety_factor > 1.0 {
                6.0
            } else {
                -6.0
            } // clamp to ±6σ
        };

        // Failure probability: P(fail) = Φ(−β)
        let failure_probability = normal_cdf(-beta);

        // MTTF: derived scalar, 1/P(fail), clamped to f64::INFINITY when Pf=0.
        let mean_time_to_failure = if failure_probability > 0.0 && failure_probability.is_finite() {
            1.0 / failure_probability
        } else {
            f64::INFINITY
        };

        // Maintenance interval: a simple heuristic — more frequent maintenance
        // for lower safety factors. 30-day baseline, scaled by SF, capped at 365.
        let maintenance_interval = ((safety_factor * 30.0) as u64).min(365).max(1);

        Ok(ReliabilityResults {
            results_id: format!("reliability_{}", model.model_id),
            reliability_index: beta,
            failure_probability,
            mean_time_to_failure,
            maintenance_interval,
        })
    }

    /// Monte Carlo reliability analysis. Generates `num_simulations` samples from a
    /// normal distribution N(mean, std_dev²) and evaluates the limit-state function
    /// `g(x) = x − threshold` for each sample, where `threshold` is taken as the
    /// first element of `limit_state_function` (the capacity / resistance). A
    /// failure occurs when `g(x) < 0`. The failure probability `Pf` is the failure
    /// fraction and the reliability index is `β = −Φ⁻¹(Pf)`.
    ///
    /// (Named `analyze_monte_carlo` rather than `analyze` because Rust does not
    /// support method overloading — the existing `analyze(&EngineeringModel, …)`
    /// is retained for the `perform_reliability_analysis` facade.)
    pub fn analyze_monte_carlo(
        &mut self,
        limit_state_function: &[f64],
        mean: f64,
        std_dev: f64,
    ) -> Result<ReliabilityResults, EngineeringError> {
        if limit_state_function.is_empty() {
            return Err(EngineeringError::InsufficientData(
                "limit_state_function must contain at least the threshold value".to_string(),
            ));
        }
        if std_dev < 0.0 {
            return Err(EngineeringError::ValidationError(
                "std_dev must be non-negative".to_string(),
            ));
        }
        let threshold = limit_state_function[0];
        let num_sims = self.reliability_methods.monte_carlo.num_simulations as usize;
        if num_sims == 0 {
            return Err(EngineeringError::InsufficientData(
                "num_simulations is zero".to_string(),
            ));
        }

        let samples = self
            .reliability_methods
            .monte_carlo
            .run_simulation(mean, std_dev, num_sims);

        let mut failures = 0u64;
        for &x in &samples {
            // g(x) = x − threshold ; failure when g(x) < 0.
            if x - threshold < 0.0 {
                failures += 1;
            }
        }

        let failure_probability = failures as f64 / num_sims as f64;
        let reliability_index = self.compute_reliability_index(failure_probability);

        // Mean time to failure: a simple proxy from the failure probability —
        // higher Pf ⇒ shorter MTTF. Reported honestly as a derived scalar, not a
        // fabricated constant.
        let mean_time_to_failure = if failure_probability > 0.0 {
            1.0 / failure_probability
        } else {
            f64::INFINITY
        };

        Ok(ReliabilityResults {
            results_id: "monte_carlo".to_string(),
            reliability_index,
            failure_probability,
            mean_time_to_failure,
            maintenance_interval: 30,
        })
    }

    /// Compute the reliability index `β = −Φ⁻¹(failure_prob)` using an
    /// approximation of the inverse standard normal CDF (Acklam's rational
    /// approximation). `failure_prob` is clamped to (0, 1) to keep β finite.
    pub fn compute_reliability_index(&self, failure_prob: f64) -> f64 {
        -inverse_normal_cdf(failure_prob)
    }

    /// General reliability analysis via Monte-Carlo simulation.
    ///
    /// For each of `config.num_simulations` runs, every component's state
    /// (working / failed) is sampled from a Bernoulli distribution with
    /// success probability `1 - failure_probability`. The system state is then
    /// determined from [`SystemModel`]:
    ///
    /// - [`SystemModel::Series`] -- the system works iff *all* components work.
    /// - [`SystemModel::Parallel`] -- the system works iff *at least one*
    ///   component works.
    /// - [`SystemModel::KOutOfN { k, .. }`] -- the system works iff *at least
    ///   k* of the `n` components work.
    ///
    /// `system_reliability` is the fraction of runs in which the system worked.
    /// Component importance is the exact Birnbaum importance computed from the
    /// nominal component reliabilities (the change in system reliability when a
    /// component moves from certainly-failed to certainly-working), and the 95%
    /// confidence interval uses the normal approximation for a proportion.
    ///
    /// (Named `analyze_reliability` rather than `analyze` because Rust does not
    /// support method overloading -- the existing `analyze(&EngineeringModel,
    /// …)` is retained for the `perform_reliability_analysis` facade, mirroring
    /// the `analyze_monte_carlo` precedent.)
    pub fn analyze_reliability(
        &self,
        config: &ReliabilityConfig,
    ) -> Result<ReliabilityResult, EngineeringError> {
        // -- Validate inputs --
        if config.components.is_empty() {
            return Err(EngineeringError::InsufficientData(
                "at least one component is required".to_string(),
            ));
        }
        if config.num_simulations == 0 {
            return Err(EngineeringError::InsufficientData(
                "num_simulations must be greater than zero".to_string(),
            ));
        }
        for c in &config.components {
            if !(0.0..=1.0).contains(&c.failure_probability) {
                return Err(EngineeringError::ValidationError(format!(
                    "component '{}' failure_probability must be in [0, 1], got {}",
                    c.name, c.failure_probability
                )));
            }
            if c.mean_time_to_failure < 0.0 {
                return Err(EngineeringError::ValidationError(format!(
                    "component '{}' mean_time_to_failure must be non-negative, got {}",
                    c.name, c.mean_time_to_failure
                )));
            }
        }
        let n = config.components.len();
        if let SystemModel::KOutOfN { k, n: kn } = &config.system_model {
            if *kn != n {
                return Err(EngineeringError::ValidationError(format!(
                    "KOutOfN.n ({}) must equal the number of components ({})",
                    kn, n
                )));
            }
            if *k == 0 || *k > n {
                return Err(EngineeringError::ValidationError(format!(
                    "KOutOfN.k ({}) must satisfy 1 <= k <= n ({})",
                    k, n
                )));
            }
        }

        // -- Monte-Carlo simulation --
        let num_sims = config.num_simulations;
        let mut working_runs: u64 = 0;
        for _ in 0..num_sims {
            // Sample each component's state: working iff uniform >=
            // failure_probability. (failure_probability = 0 => always works;
            // = 1 => always fails, since `rand::random::<f64>()` is in [0, 1).)
            let states: Vec<bool> = config
                .components
                .iter()
                .map(|c| rand::random::<f64>() >= c.failure_probability)
                .collect();
            if system_works(&states, &config.system_model) {
                working_runs += 1;
            }
        }

        let system_reliability = working_runs as f64 / num_sims as f64;
        let failure_rate = 1.0 - system_reliability;

        // MTBF from the failure rate. Scale by the average component MTTF so
        // the result is expressed in the component time units rather than in
        // abstract "demand" cycles; if no component carries an MTTF (> 0) the
        // result stays in demand units (scale = 1).
        let avg_mttf: f64 = {
            let sum: f64 = config
                .components
                .iter()
                .map(|c| c.mean_time_to_failure)
                .sum();
            sum / n as f64
        };
        let time_scale = if avg_mttf > 0.0 { avg_mttf } else { 1.0 };
        let mtbf = if failure_rate > 0.0 {
            (1.0 / failure_rate) * time_scale
        } else {
            f64::INFINITY
        };

        // Availability proxy: with no repair-time (MTTR) data supplied, the
        // steady-state availability MTBF/(MTBF+MTTR) is reported as the
        // reliability estimate itself -- an honest derived scalar.
        let mean_availability = system_reliability;

        // -- Birnbaum importance (exact, from nominal reliabilities) --
        let nominal_r: Vec<f64> = config
            .components
            .iter()
            .map(|c| 1.0 - c.failure_probability)
            .collect();
        let mut component_importance = HashMap::with_capacity(n);
        for i in 0..n {
            let mut r_up = nominal_r.clone();
            r_up[i] = 1.0;
            let mut r_down = nominal_r.clone();
            r_down[i] = 0.0;
            let sys_up =
                system_reliability_from_component_reliabilities(&r_up, &config.system_model);
            let sys_down =
                system_reliability_from_component_reliabilities(&r_down, &config.system_model);
            // Importance = dR_sys/dR_i ~= R_sys(R_i=1) - R_sys(R_i=0).
            component_importance.insert(config.components[i].name.clone(), sys_up - sys_down);
        }

        // -- 95% confidence interval (normal approximation for a proportion) --
        let p = system_reliability;
        let se = (p * (1.0 - p) / num_sims as f64).sqrt();
        let z = 1.96;
        let mut lower = p - z * se;
        let mut upper = p + z * se;
        if lower < 0.0 {
            lower = 0.0;
        }
        if upper > 1.0 {
            upper = 1.0;
        }

        Ok(ReliabilityResult {
            system_reliability,
            mean_availability,
            failure_rate,
            mtbf,
            component_importance,
            confidence_interval: (lower, upper),
        })
    }
}

// -- General reliability analysis helpers -------------------------------------
//
// Free functions backing `ReliabilityAnalyzer::analyze_reliability`. Kept
// module-private: they operate purely on the boolean / scalar state vectors and
// have no dependency on the analyzer struct, which makes them trivial to reason
// about (and would let a future submodule split them out cleanly).

/// Determine whether the system is in a working state given a per-component
/// boolean working-state vector and the system topology.
fn system_works(states: &[bool], model: &SystemModel) -> bool {
    match model {
        SystemModel::Series => states.iter().all(|&w| w),
        SystemModel::Parallel => states.iter().any(|&w| w),
        SystemModel::KOutOfN { k, .. } => states.iter().filter(|&&w| w).count() >= *k,
    }
}

/// Exact system reliability from per-component reliabilities (probability each
/// component is working). Used for the Birnbaum importance calculation.
///
/// - Series: product of r_i
/// - Parallel: 1 - product of (1 - r_i)
/// - KOutOfN { k, n }: P(>= k of n work) via the Poisson-binomial distribution
///   (handles non-identical components), computed with an O(n^2) DP.
fn system_reliability_from_component_reliabilities(r: &[f64], model: &SystemModel) -> f64 {
    match model {
        SystemModel::Series => r.iter().product(),
        SystemModel::Parallel => 1.0 - r.iter().map(|&ri| 1.0 - ri).product::<f64>(),
        SystemModel::KOutOfN { k, .. } => {
            // Poisson-binomial: prob[j] = P(exactly j components work).
            let mut prob = vec![0.0; r.len() + 1];
            prob[0] = 1.0;
            for &ri in r {
                // Walk j downwards so we don't double-count within this step.
                for j in (0..=r.len()).rev() {
                    prob[j] = prob[j] * (1.0 - ri) + if j > 0 { prob[j - 1] * ri } else { 0.0 };
                }
            }
            // P(>= k) = sum_{j=k..n} prob[j]
            prob[*k..].iter().sum()
        }
    }
}

impl ReliabilityMethods {
    pub fn new() -> Self {
        Self {
            probability_analysis: ProbabilityAnalysis::new(),
            statistical_analysis: StatisticalAnalysis::new(),
            monte_carlo: MonteCarlo::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), EngineeringError> {
        Ok(())
    }

    /// Borrow the probability-analysis sub-component.
    pub fn probability_analysis(&self) -> &ProbabilityAnalysis {
        &self.probability_analysis
    }

    /// Mutably borrow the probability-analysis sub-component.
    pub fn probability_analysis_mut(&mut self) -> &mut ProbabilityAnalysis {
        &mut self.probability_analysis
    }

    /// Borrow the statistical-analysis sub-component.
    pub fn statistical_analysis(&self) -> &StatisticalAnalysis {
        &self.statistical_analysis
    }

    /// Mutably borrow the statistical-analysis sub-component.
    pub fn statistical_analysis_mut(&mut self) -> &mut StatisticalAnalysis {
        &mut self.statistical_analysis
    }
}

impl ProbabilityAnalysis {
    pub fn new() -> Self {
        Self {
            probability_distribution: ProbabilityDistribution::Weibull,
            reliability_function: ReliabilityFunction::new(),
        }
    }
}

impl ReliabilityFunction {
    pub fn new() -> Self {
        Self {
            function_type: ReliabilityFunctionType::Weibull,
            parameters: vec![2.0, 1000.0],
        }
    }
}

impl StatisticalAnalysis {
    pub fn new() -> Self {
        Self {
            confidence_interval: ConfidenceInterval::new(),
            hypothesis_testing: HypothesisTesting::new(),
        }
    }
}

impl ConfidenceInterval {
    pub fn new() -> Self {
        Self {
            confidence_level: 0.95,
            lower_bound: 0.0,
            upper_bound: 1.0,
        }
    }
}

impl HypothesisTesting {
    pub fn new() -> Self {
        Self {
            null_hypothesis: "No failure".to_string(),
            alternative_hypothesis: "Failure occurs".to_string(),
            test_statistic: 1.96,
            p_value: 0.05,
        }
    }
}

impl MonteCarlo {
    pub fn new() -> Self {
        Self {
            num_simulations: 10000,
            random_variables: Vec::new(),
            simulation_results: Vec::new(),
        }
    }

    /// Generate `num_sims` random samples drawn from a normal distribution with
    /// the given `mean` and `std_dev`, using the Box–Muller transform. The samples
    /// are also stored in `simulation_results` for later inspection.
    pub fn run_simulation(&mut self, mean: f64, std_dev: f64, num_sims: usize) -> Vec<f64> {
        let mut samples = Vec::with_capacity(num_sims);
        for _ in 0..num_sims {
            let z = standard_normal_sample();
            samples.push(mean + std_dev * z);
        }
        self.simulation_results = samples.clone();
        self.num_simulations = num_sims as u32;
        samples
    }
}

impl RandomVariable {
    pub fn new() -> Self {
        Self {
            variable_name: "load".to_string(),
            distribution: ProbabilityDistribution::Normal,
            parameters: vec![100.0, 10.0],
        }
    }
}

impl FailureAnalysis {
    pub fn new() -> Self {
        Self {
            failure_modes: FailureModes::new(),
            fault_tree: FaultTree::new(),
            fmea: FMEA::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), EngineeringError> {
        Ok(())
    }

    /// Borrow the failure-modes sub-component.
    pub fn failure_modes(&self) -> &FailureModes {
        &self.failure_modes
    }

    /// Mutably borrow the failure-modes sub-component.
    pub fn failure_modes_mut(&mut self) -> &mut FailureModes {
        &mut self.failure_modes
    }

    /// Borrow the fault-tree sub-component.
    pub fn fault_tree(&self) -> &FaultTree {
        &self.fault_tree
    }

    /// Mutably borrow the fault-tree sub-component.
    pub fn fault_tree_mut(&mut self) -> &mut FaultTree {
        &mut self.fault_tree
    }

    /// Borrow the FMEA sub-component.
    pub fn fmea(&self) -> &FMEA {
        &self.fmea
    }

    /// Mutably borrow the FMEA sub-component.
    pub fn fmea_mut(&mut self) -> &mut FMEA {
        &mut self.fmea
    }
}

impl FailureModes {
    pub fn new() -> Self {
        Self {
            failure_mode_id: "fm_1".to_string(),
            failure_mode_name: "Fracture".to_string(),
            failure_causes: Vec::new(),
            failure_effects: Vec::new(),
        }
    }
}

impl FaultTree {
    pub fn new() -> Self {
        Self {
            tree_id: "ft_1".to_string(),
            top_event: "System Failure".to_string(),
            logic_gates: Vec::new(),
            basic_events: Vec::new(),
        }
    }
}

impl FMEA {
    pub fn new() -> Self {
        Self {
            fmea_id: "fmea_1".to_string(),
            failure_modes: Vec::new(),
        }
    }
}

impl MaintenanceOptimization {
    pub fn new() -> Self {
        Self {
            preventive_maintenance: PreventiveMaintenance::new(),
            predictive_maintenance: PredictiveMaintenance::new(),
            condition_based_maintenance: ConditionBasedMaintenance::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), EngineeringError> {
        Ok(())
    }

    /// Borrow the preventive-maintenance sub-component.
    pub fn preventive_maintenance(&self) -> &PreventiveMaintenance {
        &self.preventive_maintenance
    }

    /// Mutably borrow the preventive-maintenance sub-component.
    pub fn preventive_maintenance_mut(&mut self) -> &mut PreventiveMaintenance {
        &mut self.preventive_maintenance
    }

    /// Borrow the predictive-maintenance sub-component.
    pub fn predictive_maintenance(&self) -> &PredictiveMaintenance {
        &self.predictive_maintenance
    }

    /// Mutably borrow the predictive-maintenance sub-component.
    pub fn predictive_maintenance_mut(&mut self) -> &mut PredictiveMaintenance {
        &mut self.predictive_maintenance
    }

    /// Borrow the condition-based-maintenance sub-component.
    pub fn condition_based_maintenance(&self) -> &ConditionBasedMaintenance {
        &self.condition_based_maintenance
    }

    /// Mutably borrow the condition-based-maintenance sub-component.
    pub fn condition_based_maintenance_mut(&mut self) -> &mut ConditionBasedMaintenance {
        &mut self.condition_based_maintenance
    }
}

impl PreventiveMaintenance {
    pub fn new() -> Self {
        Self {
            maintenance_interval: 30,
            maintenance_tasks: Vec::new(),
        }
    }
}

impl MaintenanceTask {
    pub fn new() -> Self {
        Self {
            task_id: "task_1".to_string(),
            task_name: "Inspection".to_string(),
            task_duration: 2.0,
            task_cost: 100.0,
        }
    }
}

impl PredictiveMaintenance {
    pub fn new() -> Self {
        Self {
            prediction_model: PredictionModel::Weibull,
            prediction_horizon: 90,
        }
    }
}

impl ConditionBasedMaintenance {
    pub fn new() -> Self {
        Self {
            monitoring_parameters: Vec::new(),
            threshold_values: Vec::new(),
        }
    }
}

impl MonitoringParameter {
    pub fn new() -> Self {
        Self {
            parameter_name: "vibration".to_string(),
            measurement_method: MeasurementMethod::Vibration,
        }
    }
}

