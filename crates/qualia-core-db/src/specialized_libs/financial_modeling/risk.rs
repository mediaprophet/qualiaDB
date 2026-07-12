use super::*;


/// Risk assessor
pub struct RiskAssessor {
    risk_models: HashMap<String, RiskModel>,
    risk_metrics: HashMap<String, RiskMetric>,
    scenario_analyzer: ScenarioAnalyzer,
}

/// Risk models
#[derive(Debug, Clone)]
pub struct RiskModel {
    pub model_id: String,
    pub model_type: RiskModelType,
    pub parameters: RiskModelParameters,
    pub validation_results: ValidationResults,
}

/// Risk model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RiskModelType {
    VaR,
    CVaR,
    MonteCarlo,
    Historical,
    Parametric,
    StressTest,
}

/// Risk model parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskModelParameters {
    pub confidence_level: f64,
    pub time_horizon: u32,
    pub lookback_period: u32,
    pub simulation_count: u32,
}

/// Validation results
#[derive(Debug, Clone)]
pub struct ValidationResults {
    pub backtest_results: BacktestResults,
    pub model_accuracy: f64,
    pub calibration_quality: f64,
}

/// Backtest results
#[derive(Debug, Clone)]
pub struct BacktestResults {
    pub period: (u64, u64),
    pub hit_rate: f64,
    pub average_loss: f64,
    pub maximum_loss: f64,
    pub sharpe_ratio: f64,
}

/// Risk metrics
#[derive(Debug, Clone)]
pub struct RiskMetric {
    pub metric_id: String,
    pub metric_name: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub timestamp: u64,
}

/// Metric types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetricType {
    VaR,
    CVaR,
    Volatility,
    Beta,
    Alpha,
    Sharpe,
    Sortino,
}

/// Scenario analyzer
pub struct ScenarioAnalyzer {
    scenarios: HashMap<String, Scenario>,
    stress_tests: HashMap<String, StressTest>,
    sensitivity_analyzer: SensitivityAnalyzer,
    /// Registered `MarketScenario`s used by `run_scenarios` and as the basis
    /// for deterministic scenario-based stress testing.
    market_scenarios: Vec<MarketScenario>,
}

/// Scenarios
#[derive(Debug, Clone)]
pub struct Scenario {
    pub scenario_id: String,
    pub scenario_name: String,
    pub scenario_type: ScenarioType,
    pub parameters: ScenarioParameters,
    pub probability: f64,
}

/// Scenario types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScenarioType {
    Economic,
    Market,
    Geopolitical,
    Environmental,
    Regulatory,
}

/// Scenario parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioParameters {
    pub market_shocks: HashMap<String, f64>,
    pub interest_rate_changes: HashMap<String, f64>,
    pub currency_movements: HashMap<String, f64>,
    pub commodity_price_changes: HashMap<String, f64>,
}

/// Stress tests
#[derive(Debug, Clone)]
pub struct StressTest {
    pub test_id: String,
    pub test_name: String,
    pub test_type: StressTestType,
    pub scenarios: Vec<String>,
    pub results: StressTestResults,
}

/// Stress test types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StressTestType {
    Historical,
    Hypothetical,
    Reverse,
    Custom,
}

/// Stress test results
#[derive(Debug, Clone)]
pub struct StressTestResults {
    pub portfolio_value_change: f64,
    pub worst_case_loss: f64,
    pub recovery_time: u32,
    pub affected_assets: Vec<String>,
}

/// A market scenario used for scenario-based stress testing.
///
/// `shocks` maps `asset_id` → price shock percentage, where e.g. `-0.20`
/// means a 20% drop in that asset's price. Assets without an entry are
/// assumed to be unaffected by the scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketScenario {
    pub name: String,
    pub probability: f64,
    pub shocks: HashMap<String, f64>,
}

impl MarketScenario {
    pub fn new(name: impl Into<String>, probability: f64, shocks: HashMap<String, f64>) -> Self {
        Self {
            name: name.into(),
            probability,
            shocks,
        }
    }
}

/// Aggregated results of a Monte Carlo stress-test simulation run.
///
/// All monetary values are expressed in the portfolio's currency. VaR figures
/// are reported as positive numbers representing the magnitude of loss at the
/// given confidence level (i.e. the loss that is not exceeded with the stated
/// probability). `expected_shortfall` is the average loss in the tail beyond
/// the 95% VaR. `max_drawdown` is the worst single-simulation loss relative to
/// the initial portfolio value.
#[derive(Debug, Clone, PartialEq)]
pub struct StressTestResult {
    /// Value-at-Risk at the 95% confidence level (positive = loss magnitude).
    pub var_95: f64,
    /// Value-at-Risk at the 99% confidence level (positive = loss magnitude).
    pub var_99: f64,
    /// Expected shortfall (average loss beyond the 95% VaR).
    pub expected_shortfall: f64,
    /// Largest single-simulation loss relative to the initial portfolio value.
    pub max_drawdown: f64,
    /// Fraction of simulations that ended below the initial portfolio value.
    pub probability_of_loss: f64,
    /// Mean portfolio value across all simulations.
    pub mean_portfolio_value: f64,
    /// Standard deviation of simulated portfolio values.
    pub std_dev: f64,
    /// Number of simulations run.
    pub num_simulations: usize,
}

/// The impact of a single defined `MarketScenario` on a portfolio.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioResult {
    /// Name of the scenario that was applied.
    pub scenario_name: String,
    /// Change in portfolio value (negative = loss) as an absolute amount.
    pub portfolio_impact: f64,
    /// Portfolio value after applying the scenario's shocks.
    pub final_value: f64,
    /// The probability assigned to this scenario.
    pub probability: f64,
}

/// Sensitivity analyzer
pub struct SensitivityAnalyzer {
    sensitivity_factors: HashMap<String, SensitivityFactor>,
    correlation_matrix: CorrelationMatrix,
}

/// Sensitivity factors
#[derive(Debug, Clone)]
pub struct SensitivityFactor {
    pub factor_id: String,
    pub factor_name: String,
    pub factor_type: FactorType,
    pub sensitivity: f64,
}

/// Factor types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FactorType {
    InterestRate,
    Equity,
    Credit,
    Currency,
    Commodity,
}

/// Correlation matrix
#[derive(Debug, Clone)]
pub struct CorrelationMatrix {
    pub assets: Vec<String>,
    pub correlations: Vec<Vec<f64>>,
    pub last_updated: u64,
}

/// Risk analyzer
pub struct RiskAnalyzer {
    risk_models: HashMap<String, RiskModel>,
    risk_metrics: HashMap<String, RiskMetric>,
    scenario_analyzer: ScenarioAnalyzer,
    /// Registered benchmark return series used to compute real beta/alpha (see
    /// `portfolio_risk::compute_risk_metrics`). Without an active benchmark,
    /// beta/alpha are honestly reported as NaN rather than fabricated.
    benchmark_comparator: BenchmarkComparator,
    /// Name of the benchmark to use in `calculate_risk_metrics`, if any.
    active_benchmark: Option<String>,
}

impl RiskAssessor {
    pub fn new() -> Self {
        Self {
            risk_models: HashMap::new(),
            risk_metrics: HashMap::new(),
            scenario_analyzer: ScenarioAnalyzer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.scenario_analyzer.initialize()?;
        Ok(())
    }

    pub fn add_risk_model(&mut self, model: RiskModel) {
        self.risk_models.insert(model.model_id.clone(), model);
    }

    pub fn get_risk_model(&self, model_id: &str) -> Option<&RiskModel> {
        self.risk_models.get(model_id)
    }

    pub fn list_risk_models(&self) -> Vec<String> {
        self.risk_models.keys().cloned().collect()
    }

    pub fn add_risk_metric(&mut self, metric: RiskMetric) {
        self.risk_metrics.insert(metric.metric_id.clone(), metric);
    }

    pub fn get_risk_metric(&self, metric_id: &str) -> Option<&RiskMetric> {
        self.risk_metrics.get(metric_id)
    }

    pub fn list_risk_metrics(&self) -> Vec<String> {
        self.risk_metrics.keys().cloned().collect()
    }
}

impl ScenarioAnalyzer {
    pub fn new() -> Self {
        Self {
            scenarios: HashMap::new(),
            stress_tests: HashMap::new(),
            sensitivity_analyzer: SensitivityAnalyzer::new(),
            market_scenarios: Vec::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        Ok(())
    }

    /// Register a `MarketScenario` for later use by `run_scenarios`.
    pub fn add_scenario(&mut self, scenario: MarketScenario) {
        self.market_scenarios.push(scenario);
    }

    /// Run a Monte Carlo stress-test simulation over `portfolio`.
    ///
    /// For each of `num_simulations` trials, every asset receives an
    /// independent multiplicative shock drawn from a normal distribution with
    /// mean `0.0` and standard deviation `volatility` (i.e. a simple return
    /// model: `new_price = price * (1 + z * volatility)` where `z ~ N(0,1)`
    /// via an inline Box-Muller transform). The portfolio value after shocks
    /// is recorded and aggregated into a `StressTestResult`.
    ///
    /// A fixed deterministic seed is used so results are reproducible across
    /// runs (important for test stability). Returns
    /// `FinancialError::PortfolioError` if the portfolio has no assets or no
    /// positive value, and `FinancialError::ValidationError` if
    /// `num_simulations` is zero or `volatility` is negative.
    pub fn run_monte_carlo(
        &self,
        portfolio: &Portfolio,
        num_simulations: usize,
        volatility: f64,
    ) -> Result<StressTestResult, FinancialError> {
        if num_simulations == 0 {
            return Err(FinancialError::ValidationError(
                "num_simulations must be greater than zero".to_string(),
            ));
        }
        if volatility < 0.0 {
            return Err(FinancialError::ValidationError(
                "volatility must be non-negative".to_string(),
            ));
        }
        if portfolio.assets.is_empty() {
            return Err(FinancialError::PortfolioError(
                "portfolio has no assets to simulate".to_string(),
            ));
        }

        let initial_value: f64 =
            portfolio.assets.iter().map(|a| a.market_value).sum::<f64>() + portfolio.cash_balance;
        if !(initial_value > 0.0) {
            return Err(FinancialError::PortfolioError(
                "portfolio has no positive value to simulate".to_string(),
            ));
        }

        // Deterministic seed for reproducible results (and stable tests).
        let mut rng = McRng::new(0x9E37_79B9_7F4A_7C15);

        let mut values: Vec<f64> = Vec::with_capacity(num_simulations);
        for _ in 0..num_simulations {
            let mut sim_value = portfolio.cash_balance;
            for asset in &portfolio.assets {
                let z = if volatility > 0.0 {
                    rng.next_normal()
                } else {
                    0.0
                };
                let shock = z * volatility;
                let new_price = asset.current_price * (1.0 + shock);
                sim_value += new_price * asset.quantity;
            }
            values.push(sim_value);
        }

        Ok(aggregate_stress_test_result(
            &values,
            initial_value,
            num_simulations,
        ))
    }

    /// Apply each registered `MarketScenario` to `portfolio` and compute its
    /// impact. Returns one `ScenarioResult` per registered scenario, in
    /// registration order. An empty scenario set yields an empty result list.
    pub fn run_scenarios(
        &self,
        portfolio: &Portfolio,
    ) -> Result<Vec<ScenarioResult>, FinancialError> {
        if portfolio.assets.is_empty() {
            return Err(FinancialError::PortfolioError(
                "portfolio has no assets to stress".to_string(),
            ));
        }

        let initial_value: f64 =
            portfolio.assets.iter().map(|a| a.market_value).sum::<f64>() + portfolio.cash_balance;

        let mut results = Vec::with_capacity(self.market_scenarios.len());
        for scenario in &self.market_scenarios {
            let mut final_value = portfolio.cash_balance;
            for asset in &portfolio.assets {
                let shock = scenario.shocks.get(&asset.asset_id).copied().unwrap_or(0.0);
                let new_price = asset.current_price * (1.0 + shock);
                final_value += new_price * asset.quantity;
            }
            results.push(ScenarioResult {
                scenario_name: scenario.name.clone(),
                portfolio_impact: final_value - initial_value,
                final_value,
                probability: scenario.probability,
            });
        }
        Ok(results)
    }

    pub fn add_named_scenario(&mut self, scenario: Scenario) {
        self.scenarios
            .insert(scenario.scenario_id.clone(), scenario);
    }

    pub fn get_named_scenario(&self, scenario_id: &str) -> Option<&Scenario> {
        self.scenarios.get(scenario_id)
    }

    pub fn list_named_scenarios(&self) -> Vec<String> {
        self.scenarios.keys().cloned().collect()
    }

    pub fn add_stress_test(&mut self, test: StressTest) {
        self.stress_tests.insert(test.test_id.clone(), test);
    }

    pub fn get_stress_test(&self, test_id: &str) -> Option<&StressTest> {
        self.stress_tests.get(test_id)
    }

    pub fn list_stress_tests(&self) -> Vec<String> {
        self.stress_tests.keys().cloned().collect()
    }

    pub fn sensitivity_analyzer(&self) -> &SensitivityAnalyzer {
        &self.sensitivity_analyzer
    }

    pub fn sensitivity_analyzer_mut(&mut self) -> &mut SensitivityAnalyzer {
        &mut self.sensitivity_analyzer
    }
}

/// Aggregate a vector of simulated portfolio values into a `StressTestResult`.
///
/// `initial_value` is the pre-shock portfolio value used as the reference for
/// loss/drawdown calculations. Expects `values.len() == num_simulations`.
fn aggregate_stress_test_result(
    values: &[f64],
    initial_value: f64,
    num_simulations: usize,
) -> StressTestResult {
    let n = values.len();
    let mean: f64 = values.iter().sum::<f64>() / n as f64;
    let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    let std_dev = variance.sqrt();

    // Losses relative to the initial value (positive = loss).
    let mut losses: Vec<f64> = values.iter().map(|v| initial_value - v).collect();
    losses.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Percentile helper: returns the loss at the given percentile (p in [0,1])
    // using nearest-rank interpolation.
    let percentile = |p: f64| -> f64 {
        if n == 1 {
            return losses[0];
        }
        let rank = (p * (n - 1) as f64).round() as usize;
        losses[rank.min(n - 1)]
    };

    let var_95 = percentile(0.95).max(0.0);
    let var_99 = percentile(0.99).max(0.0);

    // Expected shortfall: average of losses at/above the 95% VaR threshold.
    let tail_threshold = percentile(0.95);
    let tail_losses: Vec<f64> = losses
        .iter()
        .filter(|&&l| l >= tail_threshold)
        .copied()
        .collect();
    let expected_shortfall = if tail_losses.is_empty() {
        var_95
    } else {
        tail_losses.iter().sum::<f64>() / tail_losses.len() as f64
    };

    let max_drawdown = losses.last().copied().unwrap_or(0.0).max(0.0);

    let num_losses = values.iter().filter(|v| **v < initial_value).count() as f64;
    let probability_of_loss = num_losses / n as f64;

    StressTestResult {
        var_95,
        var_99,
        expected_shortfall,
        max_drawdown,
        probability_of_loss,
        mean_portfolio_value: mean,
        std_dev,
        num_simulations,
    }
}

/// A small, deterministic, seedable PRNG for Monte Carlo simulation.
///
/// Implements a 64-bit linear congruential generator (Numerical Recipes
/// constants) plus an inline Box-Muller transform for standard normal
/// samples. No external crate required.
struct McRng {
    state: u64,
}

impl McRng {
    fn new(seed: u64) -> Self {
        // LCG requires a non-zero state; fall back to a canonical seed.
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    /// Next raw u64 from the LCG.
    fn next_u64(&mut self) -> u64 {
        // Numerical Recipes 64-bit LCG constants.
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    /// Next uniform f64 in [0, 1).
    fn next_uniform(&mut self) -> f64 {
        // Use the high 53 bits for a full-precision mantissa.
        let x = self.next_u64() >> 11;
        (x as f64) * (1.0 / (1u64 << 53) as f64)
    }

    /// Next standard normal sample via the Box-Muller transform.
    fn next_normal(&mut self) -> f64 {
        // z = sqrt(-2 ln u1) * cos(2π u2)
        let mut u1 = self.next_uniform();
        if u1 < f64::MIN_POSITIVE {
            u1 = f64::MIN_POSITIVE;
        }
        let u2 = self.next_uniform();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * u2;
        r * theta.cos()
    }
}

impl SensitivityAnalyzer {
    pub fn new() -> Self {
        Self {
            sensitivity_factors: HashMap::new(),
            correlation_matrix: CorrelationMatrix::new(),
        }
    }

    pub fn add_factor(&mut self, factor: SensitivityFactor) {
        self.sensitivity_factors
            .insert(factor.factor_id.clone(), factor);
    }

    pub fn get_factor(&self, factor_id: &str) -> Option<&SensitivityFactor> {
        self.sensitivity_factors.get(factor_id)
    }

    pub fn list_factors(&self) -> Vec<String> {
        self.sensitivity_factors.keys().cloned().collect()
    }

    pub fn correlation_matrix(&self) -> &CorrelationMatrix {
        &self.correlation_matrix
    }

    pub fn set_correlation_matrix(&mut self, matrix: CorrelationMatrix) {
        self.correlation_matrix = matrix;
    }
}

impl CorrelationMatrix {
    pub fn new() -> Self {
        Self {
            assets: Vec::new(),
            correlations: Vec::new(),
            last_updated: 0,
        }
    }
}

impl RiskAnalyzer {
    pub fn new() -> Self {
        Self {
            risk_models: HashMap::new(),
            risk_metrics: HashMap::new(),
            scenario_analyzer: ScenarioAnalyzer::new(),
            benchmark_comparator: BenchmarkComparator::new(),
            active_benchmark: None,
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.scenario_analyzer.initialize()?;
        Ok(())
    }

    /// Register a benchmark return series by name. If no benchmark is currently
    /// active, the newly registered one becomes active so that subsequent
    /// `calculate_risk_metrics` calls produce real beta/alpha.
    pub fn add_benchmark(&mut self, name: &str, returns: Vec<f64>) {
        self.benchmark_comparator.add_benchmark(name, returns);
        if self.active_benchmark.is_none() {
            self.active_benchmark = Some(name.to_string());
        }
    }

    /// Select which registered benchmark `calculate_risk_metrics` should use, or
    /// `None` to compute beta/alpha-free (NaN) metrics.
    pub fn set_active_benchmark(&mut self, name: Option<&str>) {
        self.active_benchmark = name.map(|s| s.to_string());
    }

    pub fn calculate_risk_metrics(
        &self,
        portfolio: &Portfolio,
    ) -> Result<RiskMetrics, FinancialError> {
        // REAL: computed from the portfolio's asset return time series (derived
        // from each Asset's price_history, value-weighted into a portfolio return
        // series), in the `portfolio_risk` library submodule. Volatility, 95% VaR
        // and CVaR (historical), Sharpe, Sortino and max-drawdown are genuine
        // sample statistics — never the old fabricated defaults (Sharpe 0.75 etc).
        // When no return history is present, it still REFUSES with InsufficientData
        // (the metrics are undefined without returns); beta/alpha are reported NaN
        // unless an active benchmark is registered, in which case they are
        // Cov(R_p,R_b)/Var(R_b) and mean(R_p)−beta·mean(R_b).
        let benchmark_returns = self
            .active_benchmark
            .as_deref()
            .and_then(|name| self.benchmark_comparator.benchmark_returns(name));
        let mut metrics = portfolio_risk::compute_risk_metrics(portfolio, benchmark_returns)?;

        // Risk-profile validation: compare the computed volatility / 95% VaR
        // against the portfolio's declared RiskTolerance. A mismatch yields a
        // plain-language warning in `risk_profile_assessment` (never a fabricated
        // "all clear" — `None` means within tolerance, not "unchecked").
        metrics.risk_profile_assessment =
            assess_risk_profile(&portfolio.risk_profile.risk_tolerance, &metrics);
        Ok(metrics)
    }

    pub fn add_risk_model(&mut self, model: RiskModel) {
        self.risk_models.insert(model.model_id.clone(), model);
    }

    pub fn get_risk_model(&self, model_id: &str) -> Option<&RiskModel> {
        self.risk_models.get(model_id)
    }

    pub fn list_risk_models(&self) -> Vec<String> {
        self.risk_models.keys().cloned().collect()
    }

    pub fn add_risk_metric(&mut self, metric: RiskMetric) {
        self.risk_metrics.insert(metric.metric_id.clone(), metric);
    }

    pub fn get_risk_metric(&self, metric_id: &str) -> Option<&RiskMetric> {
        self.risk_metrics.get(metric_id)
    }

    pub fn list_risk_metrics(&self) -> Vec<String> {
        self.risk_metrics.keys().cloned().collect()
    }
}

/// Compare computed risk metrics against a declared `RiskTolerance` and return a
/// warning string when the portfolio is riskier than its profile permits. Returns
/// `None` when the metrics fit the declared tolerance.
fn assess_risk_profile(tolerance: &RiskTolerance, metrics: &RiskMetrics) -> Option<String> {
    // Per-period volatility / VaR thresholds for each tolerance band. These are
    // stated, conservative guards — a Conservative portfolio carrying >10%
    // per-period volatility or >5% 95% VaR is flagged, etc.
    let (max_vol, max_var): (f64, f64) = match tolerance {
        RiskTolerance::Conservative => (0.10, 0.05),
        RiskTolerance::Moderate => (0.20, 0.10),
        RiskTolerance::Aggressive => (0.35, 0.18),
        RiskTolerance::VeryAggressive => (f64::INFINITY, f64::INFINITY),
    };
    let over_vol = metrics.volatility > max_vol;
    let over_var = metrics.var_95 > max_var;
    if over_vol || over_var {
        let label = match tolerance {
            RiskTolerance::Conservative => "Conservative",
            RiskTolerance::Moderate => "Moderate",
            RiskTolerance::Aggressive => "Aggressive",
            RiskTolerance::VeryAggressive => "VeryAggressive",
        };
        Some(format!(
            "Portfolio declared as {label} but computed risk exceeds its tolerance band \
             (volatility {:.4} > limit {:.4}, VaR(95%) {:.4} > limit {:.4}).",
            metrics.volatility, max_vol, metrics.var_95, max_var,
        ))
    } else {
        None
    }
}

impl RiskModel {
    pub fn new() -> Self {
        Self {
            model_id: "model_1".to_string(),
            model_type: RiskModelType::VaR,
            parameters: RiskModelParameters::new(),
            validation_results: ValidationResults::new(),
        }
    }
}

impl RiskModelParameters {
    pub fn new() -> Self {
        Self {
            confidence_level: 0.95,
            time_horizon: 1,
            lookback_period: 252,
            simulation_count: 10000,
        }
    }
}

impl ValidationResults {
    pub fn new() -> Self {
        Self {
            backtest_results: BacktestResults::new(),
            // not measured (scaffold defaults; no model validation is performed)
            model_accuracy: 0.0,
            calibration_quality: 0.0,
        }
    }
}

impl BacktestResults {
    pub fn new() -> Self {
        Self {
            period: (0, 86400 * 365), // 1 year
            hit_rate: 0.95,
            average_loss: 1000.0,
            maximum_loss: 5000.0,
            sharpe_ratio: 1.5,
        }
    }
}

impl RiskMetric {
    pub fn new() -> Self {
        Self {
            metric_id: "metric_1".to_string(),
            metric_name: "VaR".to_string(),
            metric_type: MetricType::VaR,
            value: 1000.0,
            timestamp: 0,
        }
    }
}

impl Scenario {
    pub fn new() -> Self {
        Self {
            scenario_id: "scenario_1".to_string(),
            scenario_name: "Market crash".to_string(),
            scenario_type: ScenarioType::Market,
            parameters: ScenarioParameters::new(),
            probability: 0.05,
        }
    }
}

impl ScenarioParameters {
    pub fn new() -> Self {
        Self {
            market_shocks: HashMap::new(),
            interest_rate_changes: HashMap::new(),
            currency_movements: HashMap::new(),
            commodity_price_changes: HashMap::new(),
        }
    }
}

impl StressTest {
    pub fn new() -> Self {
        Self {
            test_id: "test_1".to_string(),
            test_name: "Market stress test".to_string(),
            test_type: StressTestType::Historical,
            scenarios: vec!["scenario_1".to_string()],
            results: StressTestResults::new(),
        }
    }
}

impl StressTestResults {
    pub fn new() -> Self {
        Self {
            portfolio_value_change: -0.2,
            worst_case_loss: 20000.0,
            recovery_time: 30,
            affected_assets: vec!["asset_1".to_string()],
        }
    }
}

impl SensitivityFactor {
    pub fn new() -> Self {
        Self {
            factor_id: "factor_1".to_string(),
            factor_name: "Interest rate".to_string(),
            factor_type: FactorType::InterestRate,
            sensitivity: 0.5,
        }
    }
}
