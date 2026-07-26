use super::*;

/// Rebalancing engine
pub struct RebalancingEngine {
    rebalancing_strategies: HashMap<String, RebalancingStrategy>,
    optimization_engine: OptimizationEngine,
    execution_engine: ExecutionEngine,
}

/// Rebalancing strategies
#[derive(Debug, Clone)]
pub struct RebalancingStrategy {
    pub strategy_id: String,
    pub strategy_name: String,
    pub strategy_type: RebalancingStrategyType,
    pub parameters: RebalancingParameters,
    pub constraints: RebalancingConstraints,
    /// Target portfolio weights keyed by `asset_id`, summing to ~1.0. Used by
    /// `RebalancingEngine::rebalance` to compute drift away from the target
    /// allocation. Assets without an entry are treated as target weight 0.0.
    pub target_weights: HashMap<String, f64>,
}

/// A single rebalance trade produced by `RebalancingEngine::rebalance`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RebalanceTrade {
    /// The asset to trade.
    pub asset_id: String,
    /// Whether to buy or sell.
    pub action: TradeAction,
    /// Number of units to trade (always positive; direction is in `action`).
    pub quantity: f64,
    /// The target weight this trade moves the asset towards.
    pub target_weight: f64,
}

/// Direction of a `RebalanceTrade`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TradeAction {
    Buy,
    Sell,
}

/// Rebalancing strategy types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RebalancingStrategyType {
    TimeBased,
    ThresholdBased,
    OptimizationBased,
    Hybrid,
}

/// Rebalancing parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalancingParameters {
    pub rebalance_frequency: u32,
    pub deviation_threshold: f64,
    pub min_trade_size: f64,
    pub max_trade_size: f64,
    pub transaction_costs: TransactionCosts,
}

/// Transaction costs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionCosts {
    pub commission_rate: f64,
    pub spread_cost: f64,
    pub market_impact: f64,
    pub tax_rate: f64,
}

/// Rebalancing constraints
#[derive(Debug, Clone)]
pub struct RebalancingConstraints {
    pub asset_class_limits: HashMap<String, f64>,
    pub sector_limits: HashMap<String, f64>,
    pub liquidity_constraints: LiquidityConstraints,
    pub regulatory_constraints: RegulatoryConstraints,
}

/// Liquidity constraints
#[derive(Debug, Clone)]
pub struct LiquidityConstraints {
    pub max_daily_volume: f64,
    pub min_liquidity_score: f64,
    pub liquidity_buffer: f64,
}

/// Regulatory constraints
#[derive(Debug, Clone)]
pub struct RegulatoryConstraints {
    pub concentration_limits: HashMap<String, f64>,
    pub reporting_requirements: Vec<String>,
    pub compliance_deadlines: Vec<u64>,
}

/// Optimization engine
pub struct OptimizationEngine {
    optimization_algorithms: HashMap<String, OptimizationAlgorithm>,
    objective_functions: HashMap<String, ObjectiveFunction>,
    constraints: Vec<OptimizationConstraint>,
}

/// Optimization algorithms
#[derive(Debug, Clone)]
pub struct OptimizationAlgorithm {
    pub algorithm_id: String,
    pub algorithm_type: OptimizationAlgorithmType,
    pub parameters: OptimizationParameters,
}

/// Optimization algorithm types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OptimizationAlgorithmType {
    MeanVariance,
    BlackLitterman,
    RiskParity,
    EqualWeight,
    Custom,
}

/// Optimization parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationParameters {
    pub risk_aversion: f64,
    pub expected_returns: Vec<f64>,
    pub covariance_matrix: Vec<Vec<f64>>,
    pub constraints: Vec<OptimizationConstraint>,
}

/// Objective functions
#[derive(Debug, Clone)]
pub struct ObjectiveFunction {
    pub function_id: String,
    pub function_type: ObjectiveFunctionType,
    pub parameters: HashMap<String, f64>,
}

/// Objective function types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectiveFunctionType {
    MaximizeReturn,
    MinimizeRisk,
    MaximizeSharpe,
    MinimizeDrawdown,
    Custom,
}

/// Optimization constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConstraint {
    pub constraint_id: String,
    pub constraint_type: ConstraintType,
    pub bounds: ConstraintBounds,
}

/// Constraint types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintType {
    Equality,
    Inequality,
    Bound,
    Linear,
    Nonlinear,
}

/// Constraint bounds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintBounds {
    pub lower_bound: f64,
    pub upper_bound: f64,
}

impl RebalancingEngine {
    pub fn new() -> Self {
        Self {
            rebalancing_strategies: HashMap::new(),
            optimization_engine: OptimizationEngine::new(),
            execution_engine: ExecutionEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.optimization_engine.initialize()?;
        self.execution_engine.initialize()?;
        Ok(())
    }

    /// Register a rebalancing strategy, keyed by `strategy.strategy_id`.
    pub fn register_strategy(&mut self, strategy: RebalancingStrategy) {
        self.rebalancing_strategies
            .insert(strategy.strategy_id.clone(), strategy);
    }

    /// Look up a registered strategy by id.
    pub fn get_strategy(&self, strategy_id: &str) -> Option<&RebalancingStrategy> {
        self.rebalancing_strategies.get(strategy_id)
    }

    /// Compute the current portfolio weights (`market_value / total`) keyed by
    /// `asset_id`. These are the "drifted" weights that `rebalance` compares
    /// against a strategy's `target_weights`. Returns an empty map when the
    /// portfolio has no positive market value.
    pub fn calculate_drift(portfolio: &Portfolio) -> HashMap<String, f64> {
        let total: f64 = portfolio.assets.iter().map(|a| a.market_value).sum();
        let mut weights = HashMap::new();
        if !(total > 0.0) {
            return weights;
        }
        for asset in &portfolio.assets {
            weights.insert(asset.asset_id.clone(), asset.market_value / total);
        }
        weights
    }

    /// Compute drift against `strategy.target_weights` and, for any asset whose
    /// drift exceeds `strategy.parameters.deviation_threshold`, generate a
    /// `RebalanceTrade` that would move the asset back to its target weight.
    ///
    /// Trades are sized in units: `quantity = |target_value − current_value| /
    /// current_price`, where `target_value = target_weight · total_value`. The
    /// portfolio is **not** mutated by this method — it only proposes trades;
    /// applying them (and their costs) is the execution layer's job.
    pub fn rebalance(
        &self,
        portfolio: &mut Portfolio,
        strategy: &RebalancingStrategy,
    ) -> Result<Vec<RebalanceTrade>, FinancialError> {
        let total_value: f64 = portfolio.assets.iter().map(|a| a.market_value).sum();
        if !(total_value > 0.0) {
            return Err(FinancialError::PortfolioError(
                "cannot rebalance: total portfolio market value is not positive".to_string(),
            ));
        }

        let current_weights = Self::calculate_drift(portfolio);
        let threshold = strategy.parameters.deviation_threshold;
        let mut trades = Vec::new();

        for asset in &portfolio.assets {
            let current_weight = current_weights.get(&asset.asset_id).copied().unwrap_or(0.0);
            let target_weight = strategy
                .target_weights
                .get(&asset.asset_id)
                .copied()
                .unwrap_or(0.0);
            let drift = current_weight - target_weight;

            if drift.abs() > threshold {
                if asset.current_price <= 0.0 {
                    return Err(FinancialError::AssetError(format!(
                        "asset '{}' has non-positive current price; cannot size a trade",
                        asset.asset_id
                    )));
                }
                let target_value = target_weight * total_value;
                let value_diff = target_value - asset.market_value;
                let quantity = value_diff / asset.current_price;
                let action = if quantity >= 0.0 {
                    TradeAction::Buy
                } else {
                    TradeAction::Sell
                };
                trades.push(RebalanceTrade {
                    asset_id: asset.asset_id.clone(),
                    action,
                    quantity: quantity.abs(),
                    target_weight,
                });
            }
        }

        Ok(trades)
    }
}

impl OptimizationEngine {
    pub fn new() -> Self {
        Self {
            optimization_algorithms: HashMap::new(),
            objective_functions: HashMap::new(),
            constraints: Vec::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        Ok(())
    }

    pub fn add_algorithm(&mut self, algorithm: OptimizationAlgorithm) {
        self.optimization_algorithms
            .insert(algorithm.algorithm_id.clone(), algorithm);
    }

    pub fn get_algorithm(&self, algorithm_id: &str) -> Option<&OptimizationAlgorithm> {
        self.optimization_algorithms.get(algorithm_id)
    }

    pub fn list_algorithms(&self) -> Vec<String> {
        self.optimization_algorithms.keys().cloned().collect()
    }

    pub fn add_objective_function(&mut self, function: ObjectiveFunction) {
        self.objective_functions
            .insert(function.function_id.clone(), function);
    }

    pub fn get_objective_function(&self, function_id: &str) -> Option<&ObjectiveFunction> {
        self.objective_functions.get(function_id)
    }

    pub fn list_objective_functions(&self) -> Vec<String> {
        self.objective_functions.keys().cloned().collect()
    }

    pub fn add_constraint(&mut self, constraint: OptimizationConstraint) {
        self.constraints.push(constraint);
    }

    pub fn list_constraints(&self) -> &[OptimizationConstraint] {
        &self.constraints
    }
}

impl RebalancingStrategy {
    pub fn new() -> Self {
        Self {
            strategy_id: "strategy_1".to_string(),
            strategy_name: "Monthly rebalancing".to_string(),
            strategy_type: RebalancingStrategyType::TimeBased,
            parameters: RebalancingParameters::new(),
            constraints: RebalancingConstraints::new(),
            target_weights: HashMap::new(),
        }
    }
}

impl RebalancingParameters {
    pub fn new() -> Self {
        Self {
            rebalance_frequency: 30,   // 30 days
            deviation_threshold: 0.05, // 5%
            min_trade_size: 1000.0,
            max_trade_size: 100000.0,
            transaction_costs: TransactionCosts::new(),
        }
    }
}

impl TransactionCosts {
    pub fn new() -> Self {
        Self {
            commission_rate: 0.001,
            spread_cost: 0.0005,
            market_impact: 0.0002,
            tax_rate: 0.2,
        }
    }
}

impl RebalancingConstraints {
    pub fn new() -> Self {
        Self {
            asset_class_limits: HashMap::new(),
            sector_limits: HashMap::new(),
            liquidity_constraints: LiquidityConstraints::new(),
            regulatory_constraints: RegulatoryConstraints::new(),
        }
    }
}

impl LiquidityConstraints {
    pub fn new() -> Self {
        Self {
            max_daily_volume: 1000000.0,
            min_liquidity_score: 0.7,
            liquidity_buffer: 0.1,
        }
    }
}

impl RegulatoryConstraints {
    pub fn new() -> Self {
        Self {
            concentration_limits: HashMap::new(),
            reporting_requirements: vec!["Daily report".to_string()],
            compliance_deadlines: vec![86400], // 1 day
        }
    }
}

impl OptimizationAlgorithm {
    pub fn new() -> Self {
        Self {
            algorithm_id: "algo_1".to_string(),
            algorithm_type: OptimizationAlgorithmType::MeanVariance,
            parameters: OptimizationParameters::new(),
        }
    }
}

impl OptimizationParameters {
    pub fn new() -> Self {
        Self {
            risk_aversion: 1.0,
            expected_returns: vec![0.1, 0.08, 0.12],
            covariance_matrix: vec![
                vec![0.04, 0.02, 0.01],
                vec![0.02, 0.09, 0.03],
                vec![0.01, 0.03, 0.16],
            ],
            constraints: vec![],
        }
    }
}

impl ObjectiveFunction {
    pub fn new() -> Self {
        Self {
            function_id: "obj_1".to_string(),
            function_type: ObjectiveFunctionType::MaximizeSharpe,
            parameters: HashMap::new(),
        }
    }
}

impl OptimizationConstraint {
    pub fn new() -> Self {
        Self {
            constraint_id: "constraint_1".to_string(),
            constraint_type: ConstraintType::Equality,
            bounds: ConstraintBounds::new(),
        }
    }
}

impl ConstraintBounds {
    pub fn new() -> Self {
        Self {
            lower_bound: 0.0,
            upper_bound: 1.0,
        }
    }
}
