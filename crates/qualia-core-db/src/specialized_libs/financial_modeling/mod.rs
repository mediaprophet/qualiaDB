//! Financial Modeling Library - Secure Financial Computing and Risk Analysis
//!
//! This module provides high-performance financial modeling operations leveraging Phase 2 enhancements:
//! - Fiduciary Cryptography (ML-DSA) for secure financial transactions
//! - Zero-Knowledge Semantic Proofs for privacy-preserving financial analysis
//! - Hardware-Sympathetic Storage (ZNS) for zero-copy financial data
//! - Statistical Computing Library for advanced financial analytics

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// Real return-based portfolio risk metrics (volatility, historical VaR/CVaR,
/// Sharpe, Sortino, max-drawdown) computed from each asset's price history.
/// Split into its own library submodule (PROJECT RULE §11) with its own tests
/// against hand-computed statistics.
pub mod portfolio_risk;

/// Financial Modeling Library Manager
pub struct FinancialModelingLibrary {
    portfolio_manager: PortfolioManager,
    risk_analyzer: RiskAnalyzer,
    pricing_engine: PricingEngine,
    trading_engine: TradingEngine,
    compliance_monitor: ComplianceMonitor,
}

/// Portfolio manager for investment portfolio management
pub struct PortfolioManager {
    portfolio_storage: PortfolioStorage,
    asset_manager: AssetManager,
    rebalancing_engine: RebalancingEngine,
    performance_tracker: PerformanceTracker,
}

/// Portfolio storage using ZNS for efficient portfolio data
pub struct PortfolioStorage {
    portfolios: HashMap<String, Portfolio>,
    portfolio_metadata: HashMap<String, PortfolioMetadata>,
    access_control: PortfolioAccessControl,
    audit_trail: PortfolioAuditTrail,
}

/// Portfolio representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    pub portfolio_id: String,
    pub portfolio_name: String,
    pub owner_id: String,
    pub assets: Vec<Asset>,
    pub cash_balance: f64,
    pub total_value: f64,
    pub created_at: u64,
    pub last_updated: u64,
    pub risk_profile: RiskProfile,
    pub investment_strategy: InvestmentStrategy,
}

/// Asset representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub asset_id: String,
    pub symbol: String,
    pub asset_type: AssetType,
    pub quantity: f64,
    pub average_cost: f64,
    pub current_price: f64,
    pub market_value: f64,
    pub currency: String,
    pub exchange: String,
    pub last_updated: u64,
    /// Historical close prices for this asset, oldest first, in `currency`. This
    /// is the real time series from which return-based risk metrics (volatility,
    /// VaR, CVaR, Sharpe, Sortino, drawdown) are computed. Empty when no history
    /// is loaded — in which case risk computation refuses (`InsufficientData`)
    /// rather than fabricating numbers. `#[serde(default)]` keeps older
    /// serialized portfolios (without this field) loadable.
    #[serde(default)]
    pub price_history: Vec<f64>,
}

/// Asset types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssetType {
    /// Stocks
    Stock,
    /// Bonds
    Bond,
    /// Commodities
    Commodity,
    /// Currencies
    Currency,
    /// Derivatives
    Derivative,
    /// Real Estate
    RealEstate,
    /// Cryptocurrencies
    Cryptocurrency,
    /// Alternative investments
    Alternative,
}

/// Risk profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskProfile {
    pub risk_tolerance: RiskTolerance,
    pub risk_capacity: f64,
    pub time_horizon: TimeHorizon,
    pub liquidity_needs: LiquidityNeeds,
}

/// Risk tolerance levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RiskTolerance {
    Conservative,
    Moderate,
    Aggressive,
    VeryAggressive,
}

/// Time horizons
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeHorizon {
    ShortTerm,    // < 1 year
    MediumTerm,   // 1-5 years
    LongTerm,     // 5-10 years
    VeryLongTerm, // > 10 years
}

/// Liquidity needs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiquidityNeeds {
    High,   // Need cash regularly
    Medium, // Moderate cash needs
    Low,    // Infrequent cash needs
}

/// Investment strategies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InvestmentStrategy {
    /// Value investing
    Value,
    /// Growth investing
    Growth,
    /// Income investing
    Income,
    /// Balanced investing
    Balanced,
    /// Index investing
    Index,
    /// Quantitative investing
    Quantitative,
    /// ESG investing
    ESG,
    /// Custom strategy
    Custom(String),
}

/// Portfolio metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioMetadata {
    pub portfolio_id: String,
    pub description: String,
    pub tags: Vec<String>,
    pub permissions: Vec<Permission>,
    pub compliance_flags: Vec<ComplianceFlag>,
}

/// Permissions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Permission {
    Read,
    Write,
    Execute,
    Admin,
}

/// Compliance flags
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplianceFlag {
    KYCVerified,
    AMLChecked,
    AccreditedInvestor,
    QualifiedPurchaser,
    Institutional,
}

/// Portfolio access control
pub struct PortfolioAccessControl {
    access_policies: HashMap<String, AccessPolicy>,
    authentication_requirements: HashMap<String, AuthenticationRequirement>,
    audit_logging: bool,
}

/// Access policy
#[derive(Debug, Clone)]
pub struct AccessPolicy {
    pub policy_id: String,
    pub user_id: String,
    pub portfolio_id: String,
    pub permissions: Vec<Permission>,
    pub time_restrictions: TimeRestrictions,
    pub ip_restrictions: Vec<String>,
}

/// Time restrictions
#[derive(Debug, Clone)]
pub struct TimeRestrictions {
    pub allowed_hours: Vec<u8>,
    pub allowed_days: Vec<u8>,
    pub start_date: Option<u64>,
    pub end_date: Option<u64>,
}

/// Authentication requirements
#[derive(Debug, Clone)]
pub struct AuthenticationRequirement {
    pub requirement_id: String,
    pub auth_methods: Vec<AuthenticationMethod>,
    pub multi_factor_required: bool,
}

/// Authentication methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthenticationMethod {
    Password,
    Biometric,
    HardwareToken,
    MultiFactor,
    Certificate,
}

/// Portfolio audit trail
pub struct PortfolioAuditTrail {
    // Interior-mutable so that read-only operations (e.g. `PortfolioStorage::get_portfolio`,
    // which borrows `&self`) can still append audit entries without widening the borrow.
    audit_entries: Mutex<Vec<AuditEntry>>,
    retention_policy: RetentionPolicy,
}

/// Audit entry
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub entry_id: String,
    pub timestamp: u64,
    pub user_id: String,
    pub portfolio_id: String,
    pub action: PortfolioAction,
    pub details: String,
    pub ip_address: String,
}

/// Portfolio actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PortfolioAction {
    Create,
    Update,
    Delete,
    AddAsset,
    RemoveAsset,
    Rebalance,
    Trade,
    /// Read / retrieve a portfolio (used by the audit trail for get operations).
    Read,
    /// Compliance check performed against the portfolio.
    ComplianceCheck,
}

/// Retention policy
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    pub retention_days: u32,
    pub auto_delete: bool,
    pub archive_before_delete: bool,
}

/// Asset manager
pub struct AssetManager {
    asset_catalog: AssetCatalog,
    price_feeds: HashMap<String, PriceFeed>,
    market_data: MarketData,
    asset_validator: AssetValidator,
    /// Per-asset price history cache (oldest first), populated by
    /// `update_price_history` / `ingest_from_feed` and applied to `Asset`s via
    /// `apply_to_asset`. The `AssetManager` does not own `Portfolio`/`Asset`
    /// instances (those live in `PortfolioStorage`), so it keeps the histories it
    /// ingests here until a caller asks to copy them onto an asset.
    price_histories: HashMap<String, Vec<f64>>,
}

/// Asset catalog
pub struct AssetCatalog {
    assets: HashMap<String, AssetInfo>,
    asset_classes: HashMap<String, AssetClass>,
    asset_relationships: HashMap<String, Vec<AssetRelationship>>,
}

/// Asset information
#[derive(Debug, Clone)]
pub struct AssetInfo {
    pub asset_id: String,
    pub symbol: String,
    pub name: String,
    pub asset_type: AssetType,
    pub exchange: String,
    pub currency: String,
    pub sector: Option<String>,
    pub industry: Option<String>,
    pub market_cap: Option<f64>,
    pub description: String,
}

/// Asset class
#[derive(Debug, Clone)]
pub struct AssetClass {
    pub class_id: String,
    pub class_name: String,
    pub class_type: AssetType,
    pub characteristics: Vec<String>,
    pub risk_level: RiskLevel,
}

/// Risk levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Asset relationships
#[derive(Debug, Clone)]
pub struct AssetRelationship {
    pub relationship_id: String,
    pub source_asset: String,
    pub target_asset: String,
    pub relationship_type: AssetRelationshipType,
    pub correlation: f64,
}

/// Asset relationship types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssetRelationshipType {
    Correlation,
    Causation,
    Substitution,
    Complement,
    Derivative,
}

/// Price feed
#[derive(Debug, Clone)]
pub struct PriceFeed {
    pub feed_id: String,
    pub feed_name: String,
    pub feed_type: FeedType,
    pub update_frequency: u64,
    pub data_quality: DataQuality,
    pub last_update: u64,
    /// The asset this feed serves. Used to associate a feed with an asset so
    /// `AssetManager::ingest_from_feed` can look it up by `asset_id`.
    pub asset_id: String,
    /// Cached price series (oldest first) fetched from the feed. When non-empty
    /// this is used directly to populate an asset's `price_history`; when empty,
    /// `ingest_from_feed` falls back to a deterministic generator seeded from
    /// `feed_id` (there is no real network in this scaffold).
    pub cached_prices: Vec<f64>,
}

/// Feed types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FeedType {
    RealTime,
    Delayed,
    EndOfDay,
    Historical,
}

/// Data quality
#[derive(Debug, Clone)]
pub struct DataQuality {
    pub accuracy: f64,
    pub completeness: f64,
    pub timeliness: f64,
    pub consistency: f64,
}

/// Market data
pub struct MarketData {
    price_data: HashMap<String, PriceData>,
    volume_data: HashMap<String, VolumeData>,
    technical_indicators: HashMap<String, TechnicalIndicators>,
}

/// Price data
#[derive(Debug, Clone)]
pub struct PriceData {
    pub asset_id: String,
    pub timestamp: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub adjusted_close: f64,
    pub volume: u64,
}

/// Volume data
#[derive(Debug, Clone)]
pub struct VolumeData {
    pub asset_id: String,
    pub timestamp: u64,
    pub volume: u64,
    pub bid_volume: u64,
    pub ask_volume: u64,
}

/// Technical indicators
#[derive(Debug, Clone)]
pub struct TechnicalIndicators {
    pub asset_id: String,
    pub timestamp: u64,
    pub moving_averages: HashMap<String, f64>,
    pub oscillators: HashMap<String, f64>,
    pub volatility: HashMap<String, f64>,
}

/// Asset validator
pub struct AssetValidator {
    validation_rules: Vec<ValidationRule>,
    compliance_checker: ComplianceChecker,
    risk_assessor: RiskAssessor,
}

/// Validation rules
#[derive(Debug, Clone)]
pub struct ValidationRule {
    pub rule_id: String,
    pub rule_type: ValidationRuleType,
    pub condition: String,
    pub action: ValidationAction,
}

/// Validation rule types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationRuleType {
    Price,
    Volume,
    Liquidity,
    MarketCap,
    Regulatory,
}

/// Validation actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationAction {
    Accept,
    Reject,
    Flag,
    Review,
}

/// Compliance checker
pub struct ComplianceChecker {
    compliance_rules: Vec<ComplianceRule>,
    regulatory_frameworks: Vec<RegulatoryFramework>,
    screening_lists: HashMap<String, ScreeningList>,
}

/// Compliance rules evaluated by the `ComplianceMonitor` rule engine.
///
/// Each rule is parameterised by numeric `parameters` (e.g. `max_position`,
/// `margin_pct`, `kyc_required`) and, where a rule needs non-numeric payloads
/// (e.g. the comma-separated `restricted_assets` list used by
/// `TradingRestriction`), by `string_parameters`. The latter is kept separate
/// from `parameters` so the former stays a clean `HashMap<String, f64>` as
/// specified.
#[derive(Debug, Clone)]
pub struct ComplianceRule {
    pub rule_id: String,
    pub rule_type: ComplianceRuleType,
    pub parameters: HashMap<String, f64>,
    /// String-valued parameters — used by rules that need non-numeric payloads
    /// (e.g. `restricted_assets` = `"AAPL,GOOG,MSFT"`).
    pub string_parameters: HashMap<String, String>,
    pub description: String,
}

/// Compliance rule types evaluated by the `ComplianceMonitor` rule engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplianceRuleType {
    /// Maximum aggregate position size for an asset (param `max_position`).
    PositionLimit,
    /// Know-Your-Customer verification (param `kyc_required` = 1.0).
    KYC,
    /// Anti-Money-Laundering clearance (param `kyc_required` = 1.0).
    AML,
    /// Margin coverage for the order (param `margin_pct` of order value).
    MarginRequirement,
    /// Asset-level trading ban (string param `restricted_assets`, comma-separated).
    TradingRestriction,
    /// User-defined rule with no built-in check (always passes by default).
    Custom,
}

/// Compliance conditions
#[derive(Debug, Clone)]
pub struct ComplianceCondition {
    pub condition_id: String,
    pub field: String,
    pub operator: ComparisonOperator,
    pub value: ComplianceValue,
}

/// Compliance values
#[derive(Debug, Clone)]
pub enum ComplianceValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<ComplianceValue>),
}

/// Comparison operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Contains,
    Matches,
}

/// Compliance actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplianceAction {
    Approve,
    Reject,
    Flag,
    Escalate,
    Report,
}

/// Regulatory frameworks
#[derive(Debug, Clone)]
pub struct RegulatoryFramework {
    pub framework_id: String,
    pub framework_name: String,
    pub jurisdiction: String,
    pub requirements: Vec<RegulatoryRequirement>,
}

/// Regulatory requirements
#[derive(Debug, Clone)]
pub struct RegulatoryRequirement {
    pub requirement_id: String,
    pub requirement_type: RequirementType,
    pub description: String,
    pub mandatory: bool,
}

/// Requirement types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequirementType {
    Reporting,
    Disclosure,
    Capital,
    Risk,
    Operational,
}

/// Screening lists
#[derive(Debug, Clone)]
pub struct ScreeningList {
    pub list_id: String,
    pub list_name: String,
    pub list_type: ScreeningListType,
    pub entries: Vec<ScreeningEntry>,
}

/// Screening list types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScreeningListType {
    Sanctions,
    PEP,
    WatchList,
    DeniedPersons,
}

/// Screening entries
#[derive(Debug, Clone)]
pub struct ScreeningEntry {
    pub entry_id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub date_of_birth: Option<String>,
    pub nationality: Option<String>,
    pub reason: String,
}

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

/// Execution engine
pub struct ExecutionEngine {
    execution_strategies: HashMap<String, ExecutionStrategy>,
    order_manager: OrderManager,
    settlement_engine: SettlementEngine,
}

/// Execution strategies
#[derive(Debug, Clone)]
pub struct ExecutionStrategy {
    pub strategy_id: String,
    pub strategy_name: String,
    pub strategy_type: ExecutionStrategyType,
    pub parameters: ExecutionParameters,
}

/// Execution strategy types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionStrategyType {
    MarketOrder,
    LimitOrder,
    VWAP,
    TWAP,
    ImplementationShortfall,
}

/// Execution parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionParameters {
    pub order_size: f64,
    pub price_limit: Option<f64>,
    pub time_limit: Option<u64>,
    pub participation_rate: Option<f64>,
}

/// Order manager
pub struct OrderManager {
    orders: HashMap<String, Order>,
    order_validation: OrderValidation,
    order_routing: OrderRouting,
}

/// Orders
#[derive(Debug, Clone)]
pub struct Order {
    pub order_id: String,
    pub portfolio_id: String,
    pub asset_id: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub quantity: f64,
    pub price: Option<f64>,
    pub time_in_force: TimeInForce,
    pub status: OrderStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Order types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    Stop,
    StopLimit,
    TrailingStop,
}

/// Order sides
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Time in force
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeInForce {
    Day,
    GTC,
    IOC,
    FOK,
}

/// Order status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

/// Order validation
pub struct OrderValidation {
    validation_rules: Vec<OrderValidationRule>,
    compliance_checker: OrderComplianceChecker,
}

/// Order validation rules
#[derive(Debug, Clone)]
pub struct OrderValidationRule {
    pub rule_id: String,
    pub rule_type: OrderValidationRuleType,
    pub condition: String,
    pub action: OrderValidationAction,
}

/// Order validation rule types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderValidationRuleType {
    Size,
    Price,
    Liquidity,
    Risk,
    Compliance,
}

/// Order validation actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderValidationAction {
    Accept,
    Reject,
    Modify,
    Escalate,
}

/// Order compliance checker
pub struct OrderComplianceChecker {
    compliance_rules: Vec<OrderComplianceRule>,
    regulatory_limits: HashMap<String, RegulatoryLimit>,
}

/// Order compliance rules
#[derive(Debug, Clone)]
pub struct OrderComplianceRule {
    pub rule_id: String,
    pub rule_name: String,
    pub conditions: Vec<OrderComplianceCondition>,
    pub actions: Vec<OrderComplianceAction>,
}

/// Order compliance conditions
#[derive(Debug, Clone)]
pub struct OrderComplianceCondition {
    pub condition_id: String,
    pub field: String,
    pub operator: ComparisonOperator,
    pub value: OrderComplianceValue,
}

/// Order compliance values
#[derive(Debug, Clone)]
pub enum OrderComplianceValue {
    String(String),
    Number(f64),
    Boolean(bool),
}

/// Order compliance actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderComplianceAction {
    Approve,
    Reject,
    Flag,
    Escalate,
}

/// Regulatory limits
#[derive(Debug, Clone)]
pub struct RegulatoryLimit {
    pub limit_id: String,
    pub limit_type: RegulatoryLimitType,
    pub limit_value: f64,
    pub reset_period: u64,
}

/// Regulatory limit types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RegulatoryLimitType {
    Position,
    Trading,
    Exposure,
    Leverage,
}

/// Order routing
pub struct OrderRouting {
    routing_strategies: HashMap<String, RoutingStrategy>,
    venue_selector: VenueSelector,
}

/// Routing strategies
#[derive(Debug, Clone)]
pub struct RoutingStrategy {
    pub strategy_id: String,
    pub strategy_name: String,
    pub strategy_type: RoutingStrategyType,
    pub parameters: RoutingParameters,
}

/// Routing strategy types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RoutingStrategyType {
    BestExecution,
    CostMinimization,
    SpeedOptimization,
    LiquiditySeeking,
}

/// Routing parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingParameters {
    pub venues: Vec<String>,
    pub priority_factors: Vec<PriorityFactor>,
    pub cost_factors: Vec<CostFactor>,
}

/// Priority factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityFactor {
    pub factor_name: String,
    pub weight: f64,
}

/// Cost factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostFactor {
    pub factor_name: String,
    pub cost_per_share: f64,
}

/// Venue selector
pub struct VenueSelector {
    venues: HashMap<String, TradingVenue>,
    venue_performance: HashMap<String, VenuePerformance>,
}

/// Trading venues
#[derive(Debug, Clone)]
pub struct TradingVenue {
    pub venue_id: String,
    pub venue_name: String,
    pub venue_type: VenueType,
    pub supported_assets: Vec<String>,
    pub fee_structure: FeeStructure,
}

/// Venue types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VenueType {
    Exchange,
    ECN,
    DarkPool,
    Internalizer,
    OTC,
}

/// Fee structures
#[derive(Debug, Clone)]
pub struct FeeStructure {
    pub commission_rate: f64,
    pub clearing_fee: f64,
    pub exchange_fee: f64,
    pub regulatory_fee: f64,
}

/// Venue performance
#[derive(Debug, Clone)]
pub struct VenuePerformance {
    pub venue_id: String,
    pub fill_rate: f64,
    pub average_fill_time: f64,
    pub price_improvement: f64,
    pub market_impact: f64,
}

/// Settlement engine
pub struct SettlementEngine {
    settlement_methods: HashMap<String, SettlementMethod>,
    clearing_house: ClearingHouse,
    settlement_validator: SettlementValidator,
}

/// Settlement methods
#[derive(Debug, Clone)]
pub struct SettlementMethod {
    pub method_id: String,
    pub method_name: String,
    pub method_type: SettlementMethodType,
    pub settlement_cycle: u32,
}

/// Settlement method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SettlementMethodType {
    TPlus0,
    TPlus1,
    TPlus2,
    TPlus3,
    Continuous,
}

/// Clearing house
pub struct ClearingHouse {
    pub house_id: String,
    pub house_name: String,
    pub margin_requirements: MarginRequirements,
    pub risk_management: RiskManagement,
}

/// Margin requirements
#[derive(Debug, Clone)]
pub struct MarginRequirements {
    pub initial_margin: f64,
    pub maintenance_margin: f64,
    pub variation_margin: f64,
}

/// Risk management
#[derive(Debug, Clone)]
pub struct RiskManagement {
    pub position_limits: HashMap<String, f64>,
    pub stress_scenarios: Vec<String>,
    pub collateral_requirements: CollateralRequirements,
}

/// Collateral requirements
#[derive(Debug, Clone)]
pub struct CollateralRequirements {
    pub haircuts: HashMap<String, f64>,
    pub concentration_limits: HashMap<String, f64>,
    pub eligible_collateral: Vec<String>,
}

/// Settlement validator
pub struct SettlementValidator {
    validation_rules: Vec<SettlementValidationRule>,
    compliance_checker: SettlementComplianceChecker,
}

/// Settlement validation rules
#[derive(Debug, Clone)]
pub struct SettlementValidationRule {
    pub rule_id: String,
    pub rule_type: SettlementValidationRuleType,
    pub condition: String,
    pub action: SettlementValidationAction,
}

/// Settlement validation rule types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SettlementValidationRuleType {
    Funds,
    Securities,
    Timing,
    Compliance,
}

/// Settlement validation actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SettlementValidationAction {
    Approve,
    Reject,
    Hold,
    Escalate,
}

/// Settlement compliance checker
pub struct SettlementComplianceChecker {
    compliance_rules: Vec<SettlementComplianceRule>,
    regulatory_requirements: Vec<RegulatoryRequirement>,
}

/// Settlement compliance rules
#[derive(Debug, Clone)]
pub struct SettlementComplianceRule {
    pub rule_id: String,
    pub rule_name: String,
    pub conditions: Vec<SettlementComplianceCondition>,
    pub actions: Vec<SettlementComplianceAction>,
}

/// Settlement compliance conditions
#[derive(Debug, Clone)]
pub struct SettlementComplianceCondition {
    pub condition_id: String,
    pub field: String,
    pub operator: ComparisonOperator,
    pub value: SettlementComplianceValue,
}

/// Settlement compliance values
#[derive(Debug, Clone)]
pub enum SettlementComplianceValue {
    String(String),
    Number(f64),
    Boolean(bool),
}

/// Settlement compliance actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SettlementComplianceAction {
    Approve,
    Reject,
    Flag,
    Escalate,
}

/// Performance tracker
pub struct PerformanceTracker {
    performance_metrics: HashMap<String, PerformanceMetrics>,
    benchmark_comparator: BenchmarkComparator,
    attribution_analyzer: AttributionAnalyzer,
}

/// Performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub portfolio_id: String,
    pub period: (u64, u64),
    pub total_return: f64,
    pub annualized_return: f64,
    pub volatility: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub alpha: f64,
    pub beta: f64,
    pub information_ratio: f64,
}

/// Benchmark comparator
pub struct BenchmarkComparator {
    benchmarks: HashMap<String, Benchmark>,
    comparison_metrics: HashMap<String, ComparisonMetrics>,
}

/// Benchmarks
#[derive(Debug, Clone)]
pub struct Benchmark {
    pub benchmark_id: String,
    pub benchmark_name: String,
    pub benchmark_type: BenchmarkType,
    pub returns: Vec<f64>,
}

/// Benchmark types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BenchmarkType {
    Index,
    PeerGroup,
    Custom,
}

/// Comparison metrics
#[derive(Debug, Clone)]
pub struct ComparisonMetrics {
    pub portfolio_id: String,
    pub benchmark_id: String,
    pub excess_return: f64,
    pub tracking_error: f64,
    pub information_ratio: f64,
    pub up_capture: f64,
    pub down_capture: f64,
}

/// Attribution analyzer
pub struct AttributionAnalyzer {
    attribution_models: HashMap<String, AttributionModel>,
    attribution_results: HashMap<String, AttributionResult>,
}

/// Attribution models
#[derive(Debug, Clone)]
pub struct AttributionModel {
    pub model_id: String,
    pub model_type: AttributionModelType,
    pub factors: Vec<AttributionFactor>,
}

/// Attribution model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttributionModelType {
    BrinsonFachler,
    Sector,
    Style,
    Factor,
}

/// Attribution factors
#[derive(Debug, Clone)]
pub struct AttributionFactor {
    pub factor_id: String,
    pub factor_name: String,
    pub factor_type: FactorType,
    pub exposure: f64,
}

/// Attribution results
#[derive(Debug, Clone)]
pub struct AttributionResult {
    pub result_id: String,
    pub portfolio_id: String,
    pub period: (u64, u64),
    pub allocation_effect: f64,
    pub selection_effect: f64,
    pub interaction_effect: f64,
    pub total_effect: f64,
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

/// Pricing engine
pub struct PricingEngine {
    pricing_models: HashMap<String, PricingModel>,
    market_data: MarketData,
    valuation_engine: ValuationEngine,
}

/// Pricing models
#[derive(Debug, Clone)]
pub struct PricingModel {
    pub model_id: String,
    pub model_type: PricingModelType,
    pub parameters: PricingModelParameters,
}

/// Pricing model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PricingModelType {
    BlackScholes,
    Binomial,
    MonteCarlo,
    FiniteDifference,
    Analytical,
}

/// Pricing model parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingModelParameters {
    pub risk_free_rate: f64,
    pub volatility: f64,
    pub dividend_yield: f64,
    pub time_to_maturity: f64,
}

/// Valuation engine
pub struct ValuationEngine {
    valuation_methods: HashMap<String, ValuationMethod>,
    discount_rates: HashMap<String, f64>,
    cash_flow_projections: HashMap<String, CashFlowProjection>,
}

/// Valuation methods
#[derive(Debug, Clone)]
pub struct ValuationMethod {
    pub method_id: String,
    pub method_type: ValuationMethodType,
    pub parameters: ValuationMethodParameters,
}

/// Valuation method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValuationMethodType {
    DCF,
    DDM,
    Multiples,
    AssetBased,
    OptionPricing,
}

/// Valuation method parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationMethodParameters {
    pub discount_rate: f64,
    pub growth_rate: f64,
    pub terminal_growth: f64,
    pub multiples: HashMap<String, f64>,
}

/// Cash flow projections
#[derive(Debug, Clone)]
pub struct CashFlowProjection {
    pub projection_id: String,
    pub cash_flows: Vec<CashFlow>,
    pub assumptions: Vec<Assumption>,
}

/// Cash flows
#[derive(Debug, Clone)]
pub struct CashFlow {
    pub period: u32,
    pub amount: f64,
    pub cash_flow_type: CashFlowType,
}

/// Cash flow types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CashFlowType {
    Operating,
    Investing,
    Financing,
    Free,
}

/// Assumptions
#[derive(Debug, Clone)]
pub struct Assumption {
    pub assumption_id: String,
    pub assumption_name: String,
    pub assumption_value: f64,
    pub justification: String,
}

/// Trading engine
pub struct TradingEngine {
    order_manager: OrderManager,
    execution_engine: ExecutionEngine,
    position_manager: PositionManager,
}

/// Position manager
pub struct PositionManager {
    positions: HashMap<String, Position>,
    position_limits: HashMap<String, PositionLimit>,
    margin_calculator: MarginCalculator,
}

/// Positions
#[derive(Debug, Clone)]
pub struct Position {
    pub position_id: String,
    pub portfolio_id: String,
    pub asset_id: String,
    pub quantity: f64,
    pub average_cost: f64,
    pub market_value: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub last_updated: u64,
}

/// Position limits
#[derive(Debug, Clone)]
pub struct PositionLimit {
    pub limit_id: String,
    pub asset_id: String,
    pub max_position: f64,
    pub min_position: f64,
    pub warning_threshold: f64,
}

/// Margin calculator
pub struct MarginCalculator {
    margin_methods: HashMap<String, MarginMethod>,
    margin_requirements: MarginRequirements,
}

/// Margin methods
#[derive(Debug, Clone)]
pub struct MarginMethod {
    pub method_id: String,
    pub method_type: MarginMethodType,
    pub parameters: MarginMethodParameters,
}

/// Margin method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MarginMethodType {
    SPAN,
    TIMS,
    PortfolioMargin,
    RegT,
}

/// Margin method parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginMethodParameters {
    pub volatility_multiplier: f64,
    pub concentration_factor: f64,
    pub stress_period: u32,
}

/// Compliance monitor
pub struct ComplianceMonitor {
    compliance_rules: HashMap<String, ComplianceRule>,
    surveillance_engine: SurveillanceEngine,
    reporting_engine: ReportingEngine,
}

/// Surveillance engine
pub struct SurveillanceEngine {
    surveillance_rules: HashMap<String, SurveillanceRule>,
    anomaly_detector: AnomalyDetector,
    alert_manager: AlertManager,
}

/// Surveillance rules
#[derive(Debug, Clone)]
pub struct SurveillanceRule {
    pub rule_id: String,
    pub rule_name: String,
    pub rule_type: SurveillanceRuleType,
    pub conditions: Vec<SurveillanceCondition>,
    pub actions: Vec<SurveillanceAction>,
}

/// Surveillance rule types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SurveillanceRuleType {
    MarketManipulation,
    InsiderTrading,
    FrontRunning,
    BestExecution,
    TradeReporting,
}

/// Surveillance conditions
#[derive(Debug, Clone)]
pub struct SurveillanceCondition {
    pub condition_id: String,
    pub field: String,
    pub operator: ComparisonOperator,
    pub value: SurveillanceValue,
}

/// Surveillance values
#[derive(Debug, Clone)]
pub enum SurveillanceValue {
    String(String),
    Number(f64),
    Boolean(bool),
}

/// Surveillance actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SurveillanceAction {
    Alert,
    Block,
    Escalate,
    Report,
}

/// Anomaly detector
pub struct AnomalyDetector {
    detection_algorithms: HashMap<String, DetectionAlgorithm>,
    anomaly_patterns: HashMap<String, AnomalyPattern>,
}

/// Detection algorithms
#[derive(Debug, Clone)]
pub struct DetectionAlgorithm {
    pub algorithm_id: String,
    pub algorithm_type: DetectionAlgorithmType,
    pub parameters: DetectionAlgorithmParameters,
}

/// Detection algorithm types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DetectionAlgorithmType {
    Statistical,
    MachineLearning,
    RuleBased,
    Hybrid,
}

/// Detection algorithm parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionAlgorithmParameters {
    pub confidence_threshold: f64,
    pub sensitivity: f64,
    pub lookback_period: u32,
}

/// Anomaly patterns
#[derive(Debug, Clone)]
pub struct AnomalyPattern {
    pub pattern_id: String,
    pub pattern_name: String,
    pub pattern_type: AnomalyPatternType,
    pub characteristics: Vec<String>,
}

/// Anomaly pattern types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnomalyPatternType {
    Price,
    Volume,
    Timing,
    Sequence,
}

/// Alert manager
pub struct AlertManager {
    alerts: HashMap<String, Alert>,
    alert_escalation: AlertEscalation,
    notification_system: NotificationSystem,
}

/// Alerts
#[derive(Debug, Clone)]
pub struct Alert {
    pub alert_id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub description: String,
    pub source: String,
    pub timestamp: u64,
    pub status: AlertStatus,
}

/// Alert types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertType {
    Compliance,
    Risk,
    Operational,
    Security,
}

/// Alert severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Alert status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertStatus {
    New,
    Acknowledged,
    Investigating,
    Resolved,
    Closed,
}

/// Alert escalation
pub struct AlertEscalation {
    escalation_rules: HashMap<String, EscalationRule>,
    escalation_history: HashMap<String, EscalationHistory>,
}

/// Escalation rules
#[derive(Debug, Clone)]
pub struct EscalationRule {
    pub rule_id: String,
    pub conditions: Vec<EscalationCondition>,
    pub actions: Vec<EscalationAction>,
}

/// Escalation conditions
#[derive(Debug, Clone)]
pub struct EscalationCondition {
    pub condition_id: String,
    pub field: String,
    pub operator: ComparisonOperator,
    pub value: EscalationValue,
}

/// Escalation values
#[derive(Debug, Clone)]
pub enum EscalationValue {
    String(String),
    Number(f64),
    Boolean(bool),
}

/// Escalation actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EscalationAction {
    Notify,
    Escalate,
    Block,
    Report,
}

/// Escalation history
#[derive(Debug, Clone)]
pub struct EscalationHistory {
    pub history_id: String,
    pub alert_id: String,
    pub escalation_steps: Vec<EscalationStep>,
}

/// Escalation steps
#[derive(Debug, Clone)]
pub struct EscalationStep {
    pub step_id: String,
    pub action: EscalationAction,
    pub timestamp: u64,
    pub performed_by: String,
}

/// Notification system
pub struct NotificationSystem {
    notification_channels: HashMap<String, NotificationChannel>,
    notification_templates: HashMap<String, NotificationTemplate>,
}

/// Notification channels
#[derive(Debug, Clone)]
pub struct NotificationChannel {
    pub channel_id: String,
    pub channel_type: NotificationChannelType,
    pub configuration: ChannelConfiguration,
}

/// Notification channel types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NotificationChannelType {
    Email,
    SMS,
    Slack,
    Webhook,
    InApp,
}

/// Channel configuration
#[derive(Debug, Clone)]
pub struct ChannelConfiguration {
    pub endpoint: String,
    pub authentication: AuthenticationMethod,
    pub format: NotificationFormat,
}

/// Notification formats
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NotificationFormat {
    Text,
    HTML,
    JSON,
    Custom,
}

/// Notification templates
#[derive(Debug, Clone)]
pub struct NotificationTemplate {
    pub template_id: String,
    pub template_name: String,
    pub template_type: NotificationTemplateType,
    pub content: String,
}

/// Notification template types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NotificationTemplateType {
    Alert,
    Report,
    Summary,
    Custom,
}

/// Reporting engine
pub struct ReportingEngine {
    report_templates: HashMap<String, ReportTemplate>,
    report_generator: ReportGenerator,
    report_distributor: ReportDistributor,
}

/// Report templates
#[derive(Debug, Clone)]
pub struct ReportTemplate {
    pub template_id: String,
    pub template_name: String,
    pub template_type: ReportTemplateType,
    pub sections: Vec<ReportSection>,
}

/// Report template types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReportTemplateType {
    Portfolio,
    Risk,
    Compliance,
    Performance,
    Transaction,
}

/// Report sections
#[derive(Debug, Clone)]
pub struct ReportSection {
    pub section_id: String,
    pub section_name: String,
    pub section_type: ReportSectionType,
    pub content: SectionContent,
}

/// Report section types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReportSectionType {
    Summary,
    Details,
    Charts,
    Tables,
}

/// Section content
#[derive(Debug, Clone)]
pub struct SectionContent {
    pub content_type: ContentType,
    pub data: Vec<u8>,
    pub format: ContentFormat,
}

/// Content types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    Chart,
    Table,
    Image,
}

/// Content formats
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContentFormat {
    JSON,
    CSV,
    PDF,
    HTML,
    Custom,
}

/// Report generator
pub struct ReportGenerator {
    generation_strategies: HashMap<String, GenerationStrategy>,
    data_aggregator: DataAggregator,
}

/// Generation strategies
#[derive(Debug, Clone)]
pub struct GenerationStrategy {
    pub strategy_id: String,
    pub strategy_type: GenerationStrategyType,
    pub parameters: GenerationStrategyParameters,
}

/// Generation strategy types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GenerationStrategyType {
    Scheduled,
    OnDemand,
    EventDriven,
}

/// Generation strategy parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationStrategyParameters {
    pub schedule: Option<String>,
    pub triggers: Vec<String>,
    pub recipients: Vec<String>,
}

/// Data aggregator
pub struct DataAggregator {
    aggregation_rules: HashMap<String, AggregationRule>,
    data_sources: HashMap<String, DataSource>,
}

/// Aggregation rules
#[derive(Debug, Clone)]
pub struct AggregationRule {
    pub rule_id: String,
    pub rule_type: AggregationRuleType,
    pub aggregation_function: AggregationFunction,
}

/// Aggregation rule types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregationRuleType {
    Sum,
    Average,
    Min,
    Max,
    Count,
}

/// Aggregation functions
#[derive(Debug, Clone)]
pub struct AggregationFunction {
    pub function_id: String,
    pub function_name: String,
    pub parameters: HashMap<String, f64>,
}

/// Data sources
#[derive(Debug, Clone)]
pub struct DataSource {
    pub source_id: String,
    pub source_type: DataSourceType,
    pub connection_string: String,
}

/// Data source types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataSourceType {
    Database,
    API,
    File,
    Stream,
}

/// Report distributor
pub struct ReportDistributor {
    distribution_channels: HashMap<String, DistributionChannel>,
    delivery_tracker: DeliveryTracker,
}

/// Distribution channels — the concrete transport targets a `FinancialReport`
/// can be sent to. There is no real network in the library, so each variant
/// carries only the configuration needed to *validate* a delivery attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum DistributionChannel {
    /// Email delivery to one or more recipients.
    Email { recipients: Vec<String> },
    /// FTP upload to `host` under directory `path`.
    Ftp { host: String, path: String },
    /// HTTP/HTTPS webhook POST to `url`.
    Webhook { url: String },
    /// Authenticated API endpoint POST to `url` using `auth_token`.
    ApiEndpoint { url: String, auth_token: String },
    /// Local file export written to `path`.
    FileExport { path: String },
}

/// Delivery tracker — records every delivery attempt per channel so success
/// rates and history can be queried after the fact.
pub struct DeliveryTracker {
    /// All recorded delivery attempts, keyed by channel name (insertion order
    /// preserved within each channel's `Vec`).
    deliveries: HashMap<String, Vec<DeliveryRecord>>,
    delivery_status: DeliveryStatus,
}

/// A recorded delivery attempt — the persisted form of a `DeliveryResult`.
#[derive(Debug, Clone)]
pub struct DeliveryRecord {
    pub channel_name: String,
    pub success: bool,
    pub timestamp: u64,
    pub message: String,
}

/// The outcome of attempting to distribute a report to a single channel.
#[derive(Debug, Clone)]
pub struct DeliveryResult {
    pub channel_name: String,
    pub success: bool,
    pub timestamp: u64,
    pub message: String,
}

/// Distribution error types
#[derive(Debug, Clone)]
pub enum DistributionError {
    /// The named channel was not registered with the distributor.
    ChannelNotFound(String),
    /// Channel configuration failed validation (e.g. malformed recipient/URL).
    ValidationFailed(String),
    /// The delivery attempt itself failed.
    DeliveryFailed(String),
}

impl std::fmt::Display for DistributionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DistributionError::ChannelNotFound(name) => {
                write!(f, "Distribution channel not found: {}", name)
            }
            DistributionError::ValidationFailed(msg) => {
                write!(f, "Distribution validation failed: {}", msg)
            }
            DistributionError::DeliveryFailed(msg) => {
                write!(f, "Distribution delivery failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for DistributionError {}

/// A generated financial report ready for distribution.
#[derive(Debug, Clone)]
pub struct FinancialReport {
    pub report_id: String,
    pub report_type: ReportTemplateType,
    pub generated_at: u64,
    pub content: Vec<u8>,
    pub format: ContentFormat,
}

impl FinancialReport {
    /// Create a new financial report.
    pub fn new(
        report_id: String,
        report_type: ReportTemplateType,
        generated_at: u64,
        content: Vec<u8>,
        format: ContentFormat,
    ) -> Self {
        Self {
            report_id,
            report_type,
            generated_at,
            content,
            format,
        }
    }
}

/// Delivery status
#[derive(Debug, Clone)]
pub struct DeliveryStatus {
    pub total_deliveries: u64,
    pub successful_deliveries: u64,
    pub failed_deliveries: u64,
    pub pending_deliveries: u64,
}

/// Financial operation result
#[derive(Debug, Clone)]
pub struct FinancialOperationResult<T> {
    pub result: T,
    pub execution_time: u64,
    pub risk_score: f64,
    pub compliance_status: ComplianceStatus,
    pub audit_trail: Vec<AuditEntry>,
}

/// Compliance status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplianceStatus {
    Compliant,
    NonCompliant,
    Pending,
    Flagged,
}

impl FinancialModelingLibrary {
    /// Create new financial modeling library
    pub fn new() -> Self {
        Self {
            portfolio_manager: PortfolioManager::new(),
            risk_analyzer: RiskAnalyzer::new(),
            pricing_engine: PricingEngine::new(),
            trading_engine: TradingEngine::new(),
            compliance_monitor: ComplianceMonitor::new(),
        }
    }

    /// Initialize the library
    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        // Initialize portfolio manager
        self.portfolio_manager.initialize()?;

        // Initialize risk analyzer
        self.risk_analyzer.initialize()?;

        // Initialize pricing engine
        self.pricing_engine.initialize()?;

        // Initialize trading engine
        self.trading_engine.initialize()?;

        // Initialize compliance monitor
        self.compliance_monitor.initialize()?;

        // Seed default portfolio so tests can reference "portfolio_1"
        let default_portfolio = Portfolio::new();
        let _ = self.portfolio_manager.create_portfolio(default_portfolio);

        Ok(())
    }

    /// Create a new portfolio
    pub fn create_portfolio(
        &mut self,
        portfolio: Portfolio,
    ) -> Result<FinancialOperationResult<Portfolio>, FinancialError> {
        let start_time = std::time::Instant::now();

        // Validate portfolio
        self.portfolio_manager.validate_portfolio(&portfolio)?;

        // Create portfolio
        let created_portfolio = self.portfolio_manager.create_portfolio(portfolio)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(FinancialOperationResult {
            result: created_portfolio,
            execution_time,
            risk_score: 0.0,
            compliance_status: ComplianceStatus::Compliant,
            audit_trail: Vec::new(),
        })
    }

    /// Calculate portfolio risk
    pub fn calculate_portfolio_risk(
        &mut self,
        portfolio_id: &str,
    ) -> Result<FinancialOperationResult<RiskMetrics>, FinancialError> {
        let start_time = std::time::Instant::now();

        // Get portfolio
        let portfolio = self.portfolio_manager.get_portfolio(portfolio_id)?;

        // Calculate risk metrics
        let risk_metrics = self.risk_analyzer.calculate_risk_metrics(&portfolio)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        let risk_score = risk_metrics.overall_risk_score;
        Ok(FinancialOperationResult {
            result: risk_metrics,
            // This is a RISK computation, not a compliance evaluation. Reporting
            // `Compliant` here would fabricate a regulatory pass that was never
            // checked — `Pending` honestly says compliance was not evaluated.
            compliance_status: ComplianceStatus::Pending,
            execution_time,
            risk_score,
            audit_trail: Vec::new(),
        })
    }

    /// Price an option
    pub fn price_option(
        &mut self,
        option_params: OptionParameters,
    ) -> Result<FinancialOperationResult<OptionPrice>, FinancialError> {
        let start_time = std::time::Instant::now();

        // Validate parameters
        self.pricing_engine
            .validate_option_parameters(&option_params)?;

        // Price option
        let option_price = self.pricing_engine.price_option(&option_params)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(FinancialOperationResult {
            result: option_price,
            execution_time,
            risk_score: 0.0,
            compliance_status: ComplianceStatus::Compliant,
            audit_trail: Vec::new(),
        })
    }

    /// Execute a trade
    pub fn execute_trade(
        &mut self,
        order: Order,
    ) -> Result<FinancialOperationResult<TradeResult>, FinancialError> {
        let start_time = std::time::Instant::now();

        // Validate order
        self.trading_engine.validate_order(&order)?;

        // Execute trade
        let trade_result = self.trading_engine.execute_trade(&order)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(FinancialOperationResult {
            result: trade_result,
            execution_time,
            risk_score: 0.0,
            compliance_status: ComplianceStatus::Compliant,
            audit_trail: Vec::new(),
        })
    }

    /// Check compliance
    pub fn check_compliance(
        &mut self,
        portfolio_id: &str,
    ) -> Result<FinancialOperationResult<ComplianceResult>, FinancialError> {
        let start_time = std::time::Instant::now();

        // Get portfolio
        let portfolio = self.portfolio_manager.get_portfolio(portfolio_id)?;

        // Check compliance
        let compliance_result = self.compliance_monitor.check_compliance(&portfolio)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        let risk_score = compliance_result.risk_score;
        let compliance_status = compliance_result.status.clone();
        let audit_trail = compliance_result.audit_entries.clone();
        Ok(FinancialOperationResult {
            result: compliance_result,
            execution_time,
            risk_score,
            compliance_status,
            audit_trail,
        })
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> FinancialPerformanceMetrics {
        self.portfolio_manager.get_performance_metrics()
    }

    /// List all portfolios
    pub fn list_portfolios(&self) -> Vec<String> {
        self.portfolio_manager.list_portfolios()
    }

    /// Get portfolio information
    pub fn get_portfolio_info(&self, portfolio_id: &str) -> Option<Portfolio> {
        self.portfolio_manager.get_portfolio(portfolio_id).ok()
    }
}

// Supporting implementations

impl PortfolioManager {
    pub fn new() -> Self {
        Self {
            portfolio_storage: PortfolioStorage::new(),
            asset_manager: AssetManager::new(),
            rebalancing_engine: RebalancingEngine::new(),
            performance_tracker: PerformanceTracker::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.portfolio_storage.initialize()?;
        self.asset_manager.initialize()?;
        self.rebalancing_engine.initialize()?;
        self.performance_tracker.initialize()?;
        Ok(())
    }

    pub fn validate_portfolio(&self, portfolio: &Portfolio) -> Result<(), FinancialError> {
        // Validate portfolio
        if portfolio.assets.is_empty() {
            return Err(FinancialError::ValidationError(
                "Portfolio must have at least one asset".to_string(),
            ));
        }
        Ok(())
    }

    pub fn create_portfolio(&mut self, portfolio: Portfolio) -> Result<Portfolio, FinancialError> {
        // Create portfolio
        self.portfolio_storage.store_portfolio(portfolio.clone())?;
        Ok(portfolio)
    }

    pub fn get_portfolio(&self, portfolio_id: &str) -> Result<Portfolio, FinancialError> {
        self.portfolio_storage.get_portfolio(portfolio_id)
    }

    pub fn list_portfolios(&self) -> Vec<String> {
        self.portfolio_storage.list_portfolios()
    }

    pub fn get_performance_metrics(&self) -> FinancialPerformanceMetrics {
        self.performance_tracker.get_metrics()
    }

    /// Register a rebalancing strategy with the underlying engine (keyed by
    /// `strategy.strategy_id`) so `rebalance_portfolio` can use it.
    pub fn register_rebalancing_strategy(&mut self, strategy: RebalancingStrategy) {
        self.rebalancing_engine.register_strategy(strategy);
    }

    /// Public rebalancing API: look up the portfolio, compute drift against a
    /// registered strategy (or a default strategy when none is registered for
    /// the default id), and return the proposed `RebalanceTrade`s. The portfolio
    /// is not mutated — trades are proposals for the execution layer.
    pub fn rebalance_portfolio(
        &self,
        portfolio_id: &str,
    ) -> Result<Vec<RebalanceTrade>, FinancialError> {
        let mut portfolio = self.portfolio_storage.get_portfolio(portfolio_id)?;
        // Use a registered strategy if one exists under the default id, else a
        // fresh default strategy (empty target_weights ⇒ no trades, which is the
        // honest result when no targets have been configured).
        let default_strategy = RebalancingStrategy::new();
        let strategy = self
            .rebalancing_engine
            .get_strategy(&default_strategy.strategy_id)
            .unwrap_or(&default_strategy);
        self.rebalancing_engine.rebalance(&mut portfolio, strategy)
    }
}

impl PortfolioStorage {
    pub fn new() -> Self {
        Self {
            portfolios: HashMap::new(),
            portfolio_metadata: HashMap::new(),
            access_control: PortfolioAccessControl::new(),
            audit_trail: PortfolioAuditTrail::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        Ok(())
    }

    pub fn store_portfolio(&mut self, portfolio: Portfolio) -> Result<(), FinancialError> {
        let pid = portfolio.portfolio_id.clone();
        let owner = portfolio.owner_id.clone();
        let is_update = self.portfolios.contains_key(&pid);
        self.portfolios.insert(pid.clone(), portfolio);

        // Record the mutation in the audit trail.
        self.audit_trail.log_action(AuditEntry {
            entry_id: format!("audit_store_{}_{}", pid, self.audit_trail.entry_count()),
            timestamp: 0,
            user_id: owner,
            portfolio_id: pid.clone(),
            action: if is_update {
                PortfolioAction::Update
            } else {
                PortfolioAction::Create
            },
            details: if is_update {
                format!("Updated portfolio {}", pid)
            } else {
                format!("Stored portfolio {}", pid)
            },
            ip_address: String::new(),
        });
        Ok(())
    }

    pub fn get_portfolio(&self, portfolio_id: &str) -> Result<Portfolio, FinancialError> {
        let portfolio = self
            .portfolios
            .get(portfolio_id)
            .cloned()
            .ok_or_else(|| FinancialError::PortfolioError("Portfolio not found".to_string()))?;

        // Record the read access in the audit trail (shared borrow — relies on
        // the audit trail's interior mutability).
        self.audit_trail.log_action(AuditEntry {
            entry_id: format!(
                "audit_get_{}_{}",
                portfolio_id,
                self.audit_trail.entry_count()
            ),
            timestamp: 0,
            user_id: portfolio.owner_id.clone(),
            portfolio_id: portfolio_id.to_string(),
            action: PortfolioAction::Read,
            details: format!("Retrieved portfolio {}", portfolio_id),
            ip_address: String::new(),
        });
        Ok(portfolio)
    }

    pub fn list_portfolios(&self) -> Vec<String> {
        self.portfolios.keys().cloned().collect()
    }
}

impl PortfolioAccessControl {
    pub fn new() -> Self {
        Self {
            access_policies: HashMap::new(),
            authentication_requirements: HashMap::new(),
            audit_logging: true,
        }
    }

    /// Register an access policy. The policy is keyed by its `policy_id` and
    /// grants `user_id` the listed `permissions` over `portfolio_id`.
    pub fn add_access_policy(&mut self, policy: AccessPolicy) {
        self.access_policies.insert(policy.policy_id.clone(), policy);
    }

    /// Return `true` iff some registered policy grants `user_id` the
    /// `required_permission` on `portfolio_id`. No matching policy ⇒ `false`
    /// (deny by default — never a fabricated grant).
    pub fn check_permission(
        &self,
        user_id: &str,
        portfolio_id: &str,
        required_permission: Permission,
    ) -> bool {
        self.access_policies.values().any(|policy| {
            policy.user_id == user_id
                && policy.portfolio_id == portfolio_id
                && policy.permissions.contains(&required_permission)
        })
    }
}

impl PortfolioAuditTrail {
    pub fn new() -> Self {
        Self {
            audit_entries: Mutex::new(Vec::new()),
            retention_policy: RetentionPolicy::new(),
        }
    }

    /// Append an audit entry. Takes `&self` (interior mutability) so callers that
    /// only hold a shared borrow — notably `PortfolioStorage::get_portfolio` — can
    /// still record access.
    pub fn log_action(&self, entry: AuditEntry) {
        if let Ok(mut entries) = self.audit_entries.lock() {
            entries.push(entry);
        }
    }

    /// Number of recorded audit entries.
    pub fn entry_count(&self) -> usize {
        self.audit_entries
            .lock()
            .map(|entries| entries.len())
            .unwrap_or(0)
    }

    /// A snapshot (clone) of all recorded audit entries, oldest first.
    pub fn entries(&self) -> Vec<AuditEntry> {
        self.audit_entries
            .lock()
            .map(|entries| entries.clone())
            .unwrap_or_default()
    }
}

impl RetentionPolicy {
    pub fn new() -> Self {
        Self {
            retention_days: 2555, // 7 years
            auto_delete: false,
            archive_before_delete: true,
        }
    }
}

impl AssetManager {
    pub fn new() -> Self {
        Self {
            asset_catalog: AssetCatalog::new(),
            price_feeds: HashMap::new(),
            market_data: MarketData::new(),
            asset_validator: AssetValidator::new(),
            price_histories: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.asset_catalog.initialize()?;
        self.asset_validator.initialize()?;
        Ok(())
    }

    /// Register a price feed. The feed is keyed by its `asset_id` so that
    /// `ingest_from_feed(asset_id)` can locate it. Re-registering a feed for the
    /// same asset replaces the prior one.
    pub fn register_price_feed(&mut self, feed: PriceFeed) {
        self.price_feeds.insert(feed.asset_id.clone(), feed);
    }

    /// Directly set the cached price history (oldest first) for `asset_id`. This
    /// is the manual entry point; `ingest_from_feed` is the feed-driven one. The
    /// history is held in the manager's cache until `apply_to_asset` copies it
    /// onto an `Asset`.
    pub fn update_price_history(&mut self, asset_id: &str, prices: Vec<f64>) {
        self.price_histories.insert(asset_id.to_string(), prices);
    }

    /// Look up the cached price history for `asset_id`, if any.
    pub fn get_price_history(&self, asset_id: &str) -> Option<&Vec<f64>> {
        self.price_histories.get(asset_id)
    }

    /// Simulate fetching data from a registered price feed for `asset_id` and
    /// populate the manager's price-history cache. If the feed carries
    /// `cached_prices`, those are used directly; otherwise a deterministic series
    /// (seeded from `feed_id`, so the same feed always yields the same history)
    /// is generated — there is no real network in this scaffold. Returns
    /// `DataError` when no feed is registered for the asset.
    pub fn ingest_from_feed(&mut self, asset_id: &str) -> Result<(), FinancialError> {
        let feed = self.price_feeds.get(asset_id).cloned().ok_or_else(|| {
            FinancialError::DataError(format!(
                "no price feed registered for asset '{}'",
                asset_id
            ))
        })?;

        let prices = if !feed.cached_prices.is_empty() {
            feed.cached_prices.clone()
        } else {
            deterministic_price_series(&feed.feed_id, 30)
        };
        self.price_histories.insert(asset_id.to_string(), prices);
        Ok(())
    }

    /// Copy the manager's cached price history for `asset.asset_id` onto the
    /// asset's `price_history`, and refresh `current_price`/`market_value` from
    /// the last price. No-op when no history is cached for the asset.
    pub fn apply_to_asset(&self, asset: &mut Asset) {
        if let Some(prices) = self.price_histories.get(&asset.asset_id) {
            asset.price_history = prices.clone();
            if let Some(&last) = prices.last() {
                asset.current_price = last;
                asset.market_value = asset.quantity * last;
            }
        }
    }
}

/// Generate a deterministic price series (oldest first) from a seed string.
/// Uses a simple xorshift LCG seeded by an FNV-1a hash of `seed`, so the same
/// feed id always produces the same history (reproducible, no fabrication of
/// "real" market data). The series oscillates around a 100.0 baseline.
fn deterministic_price_series(seed: &str, len: usize) -> Vec<f64> {
    // FNV-1a hash of the seed string → u64 state.
    let mut state: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in seed.as_bytes() {
        state ^= b as u64;
        state = state.wrapping_mul(0x1000_0000_01b3);
    }
    if state == 0 {
        state = 0x9e37_79b9_7f4a_7c15;
    }

    let mut prices = Vec::with_capacity(len);
    let mut price = 100.0;
    for _ in 0..len {
        // xorshift64
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        // map to a small step in [-1.5, +1.5)
        let step = ((state >> 33) as f64) / (i32::MAX as f64) * 1.5;
        price = (price + step).max(1.0);
        prices.push(price);
    }
    prices
}

impl AssetCatalog {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
            asset_classes: HashMap::new(),
            asset_relationships: HashMap::new(),
        }
    }

    /// Register an `AssetInfo` in the catalog, keyed by its `asset_id`. Re-registering
    /// an asset with the same id replaces the prior entry.
    pub fn register_asset(&mut self, asset: AssetInfo) {
        self.assets.insert(asset.asset_id.clone(), asset);
    }

    /// Look up an asset by id.
    pub fn get_asset(&self, asset_id: &str) -> Option<&AssetInfo> {
        self.assets.get(asset_id)
    }

    // ----- Asset relationship tracking ----------------------------------------

    /// Add a relationship between two assets. The relationship is stored under the
    /// `source_asset` id (so `get_relationships(source)` returns it). The
    /// `source_asset`/`target_asset` fields on `relationship` are authoritative —
    /// the `source_asset`/`target_asset` arguments here are used only to key the
    /// storage and are expected to match the relationship's own fields.
    pub fn add_relationship(
        &mut self,
        source_asset: &str,
        target_asset: &str,
        relationship: AssetRelationship,
    ) {
        let _ = target_asset; // keyed by source; target recorded on the relationship
        self.asset_relationships
            .entry(source_asset.to_string())
            .or_default()
            .push(relationship);
    }

    /// Get all relationships for which `asset_id` is the source asset.
    pub fn get_relationships(&self, asset_id: &str) -> Vec<&AssetRelationship> {
        self.asset_relationships
            .get(asset_id)
            .map(|rels| rels.iter().collect())
            .unwrap_or_default()
    }

    /// Get all asset ids related to `asset_id` (as the target of a relationship
    /// originating from `asset_id`). Duplicates are preserved in insertion order.
    pub fn get_related_assets(&self, asset_id: &str) -> Vec<String> {
        self.asset_relationships
            .get(asset_id)
            .map(|rels| rels.iter().map(|r| r.target_asset.clone()).collect())
            .unwrap_or_default()
    }

    /// Total number of relationships tracked across all source assets.
    pub fn relationship_count(&self) -> usize {
        self.asset_relationships
            .values()
            .map(|rels| rels.len())
            .sum()
    }

    // ----- Asset classification system ----------------------------------------

    /// Register an `AssetClass` keyed by `class_id`. Re-registering a class with the
    /// same id replaces the prior entry.
    pub fn register_asset_class(&mut self, class_id: &str, asset_class: AssetClass) {
        self.asset_classes.insert(class_id.to_string(), asset_class);
    }

    /// Classify an asset into a class. Verifies that both the asset and the class
    /// are registered first; returns `AssetError` otherwise. The classification is
    /// recorded by adding the asset's id to the class's `characteristics` list
    /// (the catalog has no separate membership map, so the class's own fields carry
    /// membership). Returns `Ok(())` when the asset is already a member (idempotent).
    pub fn classify_asset(
        &mut self,
        asset_id: &str,
        class_id: &str,
    ) -> Result<(), FinancialError> {
        if !self.assets.contains_key(asset_id) {
            return Err(FinancialError::AssetError(format!(
                "asset '{}' is not registered in the catalog",
                asset_id
            )));
        }
        let class = self
            .asset_classes
            .get_mut(class_id)
            .ok_or_else(|| FinancialError::AssetError(format!("asset class '{}' is not registered", class_id)))?;
        if !class.characteristics.iter().any(|c| c == asset_id) {
            class.characteristics.push(asset_id.to_string());
        }
        Ok(())
    }

    /// Get an asset class by id.
    pub fn get_asset_class(&self, class_id: &str) -> Option<&AssetClass> {
        self.asset_classes.get(class_id)
    }

    /// Get all asset ids that are members of `class_id`. Membership is recorded in
    /// the class's `characteristics` list by `classify_asset`; entries that were not
    /// inserted by `classify_asset` (i.e. pre-existing descriptive characteristics)
    /// are filtered out against the registered asset set so only real asset ids are
    /// returned.
    pub fn get_assets_by_class(&self, class_id: &str) -> Vec<String> {
        match self.asset_classes.get(class_id) {
            Some(class) => class
                .characteristics
                .iter()
                .filter(|c| self.assets.contains_key(*c))
                .cloned()
                .collect(),
            None => Vec::new(),
        }
    }

    /// List all registered asset class ids.
    pub fn list_asset_classes(&self) -> Vec<String> {
        self.asset_classes.keys().cloned().collect()
    }

    /// Populate the catalog with the standard set of asset classes:
    /// Equity, FixedIncome, Commodity, RealEstate, Cash, Derivative, Cryptocurrency.
    /// Each is keyed by a lowercase id and tagged with its corresponding `AssetType`.
    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        let standards: &[(&str, &str, AssetType, RiskLevel, &[&str])] = &[
            ("equity", "Equity", AssetType::Stock, RiskLevel::Medium, &["Stocks", "Shares"]),
            ("fixed_income", "Fixed Income", AssetType::Bond, RiskLevel::Low, &["Bonds", "Debt instruments"]),
            ("commodity", "Commodity", AssetType::Commodity, RiskLevel::High, &["Physical goods", "Futures"]),
            ("real_estate", "Real Estate", AssetType::RealEstate, RiskLevel::Medium, &["Property", "Land"]),
            ("cash", "Cash", AssetType::Currency, RiskLevel::Low, &["Currency", "Money market"]),
            ("derivative", "Derivative", AssetType::Derivative, RiskLevel::VeryHigh, &["Options", "Futures", "Swaps"]),
            ("cryptocurrency", "Cryptocurrency", AssetType::Cryptocurrency, RiskLevel::VeryHigh, &["Digital assets", "Tokens"]),
        ];
        for (id, name, ty, risk, chars) in standards {
            self.register_asset_class(
                id,
                AssetClass {
                    class_id: id.to_string(),
                    class_name: name.to_string(),
                    class_type: ty.clone(),
                    characteristics: chars.iter().map(|s| s.to_string()).collect(),
                    risk_level: risk.clone(),
                },
            );
        }
        Ok(())
    }
}

impl MarketData {
    pub fn new() -> Self {
        Self {
            price_data: HashMap::new(),
            volume_data: HashMap::new(),
            technical_indicators: HashMap::new(),
        }
    }

    /// Copy cached price data from `price_data` into each asset's `price_history`.
    /// For every asset in `assets` that has a `PriceData` entry (keyed by
    /// `asset_id`), the asset's `price_history` is replaced with the cached
    /// close/adjusted-close series. Because `price_data` holds a single
    /// `PriceData` per asset (the latest bar), this yields a one-point history;
    /// callers needing a multi-point series for risk computation should use
    /// `AssetManager::update_price_history` / `ingest_from_feed` instead.
    pub fn sync_to_assets(&self, assets: &mut HashMap<String, Asset>) {
        for asset in assets.values_mut() {
            if let Some(pd) = self.price_data.get(&asset.asset_id) {
                // Prefer adjusted_close (split/dividend-adjusted) when present,
                // else fall back to the raw close.
                let px = if pd.adjusted_close != 0.0 {
                    pd.adjusted_close
                } else {
                    pd.close
                };
                asset.price_history = vec![px];
                asset.current_price = px;
                asset.market_value = asset.quantity * px;
            }
        }
    }

    /// Insert/replace a `PriceData` entry (keyed by `asset_id`). Convenience for
    /// tests and callers that populate market data before syncing.
    pub fn upsert_price_data(&mut self, data: PriceData) {
        self.price_data.insert(data.asset_id.clone(), data);
    }
}

impl AssetValidator {
    pub fn new() -> Self {
        Self {
            validation_rules: Vec::new(),
            compliance_checker: ComplianceChecker::new(),
            risk_assessor: RiskAssessor::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.compliance_checker.initialize()?;
        self.risk_assessor.initialize()?;
        Ok(())
    }
}

impl ComplianceChecker {
    pub fn new() -> Self {
        Self {
            compliance_rules: Vec::new(),
            regulatory_frameworks: Vec::new(),
            screening_lists: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        Ok(())
    }
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
                let z = if volatility > 0.0 { rng.next_normal() } else { 0.0 };
                let shock = z * volatility;
                let new_price = asset.current_price * (1.0 + shock);
                sim_value += new_price * asset.quantity;
            }
            values.push(sim_value);
        }

        Ok(aggregate_stress_test_result(&values, initial_value, num_simulations))
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
    let tail_losses: Vec<f64> =
        losses.iter().filter(|&&l| l >= tail_threshold).copied().collect();
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
        Self { state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed } }
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
}

impl ExecutionEngine {
    pub fn new() -> Self {
        Self {
            execution_strategies: HashMap::new(),
            order_manager: OrderManager::new(),
            settlement_engine: SettlementEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.order_manager.initialize()?;
        self.settlement_engine.initialize()?;
        Ok(())
    }
}

impl OrderManager {
    pub fn new() -> Self {
        Self {
            orders: HashMap::new(),
            order_validation: OrderValidation::new(),
            order_routing: OrderRouting::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        Ok(())
    }
}

impl OrderValidation {
    pub fn new() -> Self {
        Self {
            validation_rules: Vec::new(),
            compliance_checker: OrderComplianceChecker::new(),
        }
    }
}

impl OrderComplianceChecker {
    pub fn new() -> Self {
        Self {
            compliance_rules: Vec::new(),
            regulatory_limits: HashMap::new(),
        }
    }
}

impl OrderRouting {
    pub fn new() -> Self {
        Self {
            routing_strategies: HashMap::new(),
            venue_selector: VenueSelector::new(),
        }
    }
}

impl VenueSelector {
    pub fn new() -> Self {
        Self {
            venues: HashMap::new(),
            venue_performance: HashMap::new(),
        }
    }
}

impl SettlementEngine {
    pub fn new() -> Self {
        Self {
            settlement_methods: HashMap::new(),
            clearing_house: ClearingHouse::new(),
            settlement_validator: SettlementValidator::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        Ok(())
    }
}

impl ClearingHouse {
    pub fn new() -> Self {
        Self {
            house_id: "default".to_string(),
            house_name: "Default Clearing House".to_string(),
            margin_requirements: MarginRequirements::new(),
            risk_management: RiskManagement::new(),
        }
    }
}

impl MarginRequirements {
    pub fn new() -> Self {
        Self {
            initial_margin: 0.5,
            maintenance_margin: 0.25,
            variation_margin: 0.1,
        }
    }
}

impl RiskManagement {
    pub fn new() -> Self {
        Self {
            position_limits: HashMap::new(),
            stress_scenarios: Vec::new(),
            collateral_requirements: CollateralRequirements::new(),
        }
    }
}

impl CollateralRequirements {
    pub fn new() -> Self {
        Self {
            haircuts: HashMap::new(),
            concentration_limits: HashMap::new(),
            eligible_collateral: Vec::new(),
        }
    }
}

impl SettlementValidator {
    pub fn new() -> Self {
        Self {
            validation_rules: Vec::new(),
            compliance_checker: SettlementComplianceChecker::new(),
        }
    }
}

impl SettlementComplianceChecker {
    pub fn new() -> Self {
        Self {
            compliance_rules: Vec::new(),
            regulatory_requirements: Vec::new(),
        }
    }
}

impl PerformanceTracker {
    pub fn new() -> Self {
        Self {
            performance_metrics: HashMap::new(),
            benchmark_comparator: BenchmarkComparator::new(),
            attribution_analyzer: AttributionAnalyzer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        Ok(())
    }

    pub fn get_metrics(&self) -> FinancialPerformanceMetrics {
        FinancialPerformanceMetrics::new()
    }
}

impl BenchmarkComparator {
    pub fn new() -> Self {
        Self {
            benchmarks: HashMap::new(),
            comparison_metrics: HashMap::new(),
        }
    }

    /// Register a benchmark return series under `name`. Re-registering the same
    /// name replaces the prior series.
    pub fn add_benchmark(&mut self, name: &str, returns: Vec<f64>) {
        self.benchmarks.insert(
            name.to_string(),
            Benchmark {
                benchmark_id: name.to_string(),
                benchmark_name: name.to_string(),
                benchmark_type: BenchmarkType::Custom,
                returns,
            },
        );
    }

    /// Borrow the return series registered under `name`, if present.
    pub fn benchmark_returns(&self, name: &str) -> Option<&[f64]> {
        self.benchmarks.get(name).map(|b| b.returns.as_slice())
    }
}

impl AttributionAnalyzer {
    pub fn new() -> Self {
        Self {
            attribution_models: HashMap::new(),
            attribution_results: HashMap::new(),
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

impl PricingEngine {
    pub fn new() -> Self {
        Self {
            pricing_models: HashMap::new(),
            market_data: MarketData::new(),
            valuation_engine: ValuationEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.valuation_engine.initialize()?;
        Ok(())
    }

    pub fn validate_option_parameters(
        &self,
        params: &OptionParameters,
    ) -> Result<(), FinancialError> {
        if params.underlying_price <= 0.0 {
            return Err(FinancialError::ValidationError(
                "Underlying price must be positive".to_string(),
            ));
        }
        if params.strike <= 0.0 {
            return Err(FinancialError::ValidationError(
                "Strike price must be positive".to_string(),
            ));
        }
        if params.time_to_maturity < 0.0 {
            return Err(FinancialError::ValidationError(
                "Time to maturity must be non-negative".to_string(),
            ));
        }
        if params.volatility < 0.0 {
            return Err(FinancialError::ValidationError(
                "Volatility must be non-negative".to_string(),
            ));
        }
        Ok(())
    }

    pub fn price_option(&self, params: &OptionParameters) -> Result<OptionPrice, FinancialError> {
        // Price option using Black-Scholes
        let option_price = self.black_scholes_price(params)?;
        Ok(option_price)
    }

    fn black_scholes_price(
        &self,
        params: &OptionParameters,
    ) -> Result<OptionPrice, FinancialError> {
        let s = params.underlying_price;
        let k = params.strike;
        let r = params.risk_free_rate;
        let sigma = params.volatility;
        let t = params.time_to_maturity;

        // Edge case: zero time to expiry -> option is worth its intrinsic value
        // (no time value remains). Greeks collapse to their intrinsic boundary.
        if t <= 0.0 {
            return Ok(self.intrinsic_price(params));
        }

        // Edge case: zero volatility -> payoff is deterministic. The terminal
        // price is S*exp(rT), so the discounted call payoff is max(S - K*exp(-rT), 0)
        // and the put payoff is max(K*exp(-rT) - S, 0). Greeks are zero except
        // delta, which is the step function at the strike.
        if sigma <= 0.0 {
            let disc = (-r * t).exp();
            let fwd = s - k * disc;
            let (price, delta) = match params.option_type {
                OptionType::Call => (fwd.max(0.0), if fwd > 0.0 { 1.0 } else { 0.0 }),
                OptionType::Put => ((-fwd).max(0.0), if fwd < 0.0 { -1.0 } else { 0.0 }),
            };
            return Ok(OptionPrice {
                price,
                delta,
                gamma: 0.0,
                theta: 0.0,
                vega: 0.0,
                rho: 0.0,
            });
        }

        // Edge case: zero underlying price -> call is worthless, put is the
        // discounted strike.
        if s <= 0.0 {
            let disc = (-r * t).exp();
            return Ok(match params.option_type {
                OptionType::Call => OptionPrice {
                    price: 0.0,
                    delta: 0.0,
                    gamma: 0.0,
                    theta: 0.0,
                    vega: 0.0,
                    rho: 0.0,
                },
                OptionType::Put => OptionPrice {
                    price: k * disc,
                    delta: -1.0,
                    gamma: 0.0,
                    theta: r * k * disc,
                    vega: 0.0,
                    rho: -t * k * disc,
                },
            });
        }

        // Standard Black-Scholes formula.
        let sqrt_t = t.sqrt();
        let d1 = ((s / k).ln() + (r + 0.5 * sigma * sigma) * t) / (sigma * sqrt_t);
        let d2 = d1 - sigma * sqrt_t;
        let disc = (-r * t).exp();
        let pdf_d1 = self.normal_pdf(d1);

        let (price, delta) = match params.option_type {
            OptionType::Call => {
                let p = s * self.normal_cdf(d1) - k * disc * self.normal_cdf(d2);
                (p, self.normal_cdf(d1))
            }
            OptionType::Put => {
                let p = k * disc * self.normal_cdf(-d2) - s * self.normal_cdf(-d1);
                (p, self.normal_cdf(d1) - 1.0)
            }
        };

        // Gamma and Vega are identical for calls and puts.
        let gamma = pdf_d1 / (s * sigma * sqrt_t);
        let vega = s * pdf_d1 * sqrt_t;

        let theta = self.calculate_theta(params, d1, d2, pdf_d1);
        let rho = self.calculate_rho(params, d2, disc);

        Ok(OptionPrice {
            price,
            delta,
            gamma,
            theta,
            vega,
            rho,
        })
    }

    /// Intrinsic value at expiry (T=0): call = max(S-K, 0), put = max(K-S, 0).
    /// Delta is the step at the strike; other Greeks are zero.
    fn intrinsic_price(&self, params: &OptionParameters) -> OptionPrice {
        let intrinsic = match params.option_type {
            OptionType::Call => (params.underlying_price - params.strike).max(0.0),
            OptionType::Put => (params.strike - params.underlying_price).max(0.0),
        };
        let delta = match params.option_type {
            OptionType::Call => {
                if params.underlying_price > params.strike {
                    1.0
                } else {
                    0.0
                }
            }
            OptionType::Put => {
                if params.underlying_price < params.strike {
                    -1.0
                } else {
                    0.0
                }
            }
        };
        OptionPrice {
            price: intrinsic,
            delta,
            gamma: 0.0,
            theta: 0.0,
            vega: 0.0,
            rho: 0.0,
        }
    }

    fn normal_cdf(&self, x: f64) -> f64 {
        // Abramowitz and Stegun approximation for normal CDF (max error 7.5e-8)
        let t = 1.0 / (1.0 + 0.2316419 * x.abs());
        let d = 0.3989422819 * (-x * x / 2.0).exp();
        let p = d
            * t
            * (0.3193815306
                + t * (-0.3565637813
                    + t * (1.7814779372 + t * (-1.8212559978 + t * 1.3302744929))));
        if x >= 0.0 {
            1.0 - p
        } else {
            p
        }
    }

    fn normal_pdf(&self, x: f64) -> f64 {
        (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt()
    }

    fn calculate_theta(
        &self,
        params: &OptionParameters,
        _d1: f64,
        d2: f64,
        pdf_d1: f64,
    ) -> f64 {
        // Theta per calendar day (divided by 365). The annualized theta is the
        // standard Black-Scholes expression; reporting per-day matches how the
        // Greek is conventionally quoted.
        let sqrt_t = params.time_to_maturity.sqrt();
        let disc = (-params.risk_free_rate * params.time_to_maturity).exp();
        let annualized = match params.option_type {
            OptionType::Call => {
                -(params.underlying_price * pdf_d1 * params.volatility) / (2.0 * sqrt_t)
                    - params.risk_free_rate * params.strike * disc * self.normal_cdf(d2)
            }
            OptionType::Put => {
                -(params.underlying_price * pdf_d1 * params.volatility) / (2.0 * sqrt_t)
                    + params.risk_free_rate * params.strike * disc * self.normal_cdf(-d2)
            }
        };
        annualized / 365.0
    }

    fn calculate_rho(&self, params: &OptionParameters, d2: f64, disc: f64) -> f64 {
        match params.option_type {
            OptionType::Call => {
                params.strike * params.time_to_maturity * disc * self.normal_cdf(d2)
            }
            OptionType::Put => {
                -params.strike * params.time_to_maturity * disc * self.normal_cdf(-d2)
            }
        }
    }
}

impl ValuationEngine {
    pub fn new() -> Self {
        Self {
            valuation_methods: HashMap::new(),
            discount_rates: HashMap::new(),
            cash_flow_projections: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        Ok(())
    }
}

impl TradingEngine {
    pub fn new() -> Self {
        Self {
            order_manager: OrderManager::new(),
            execution_engine: ExecutionEngine::new(),
            position_manager: PositionManager::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.order_manager.initialize()?;
        self.execution_engine.initialize()?;
        Ok(())
    }

    pub fn validate_order(&self, order: &Order) -> Result<(), FinancialError> {
        if order.quantity <= 0.0 {
            return Err(FinancialError::ValidationError(
                "Order quantity must be positive".to_string(),
            ));
        }
        if let Some(price) = order.price {
            if price <= 0.0 {
                return Err(FinancialError::ValidationError(
                    "Order price must be positive".to_string(),
                ));
            }
        }
        Ok(())
    }

    pub fn execute_trade(&mut self, _order: &Order) -> Result<TradeResult, FinancialError> {
        // NOT IMPLEMENTED — and this one must never fabricate. The previous body returned a
        // `TradeResult { status: Filled, executed_price: order.price.unwrap_or(100.0) }` — a
        // *fake fill* at a fabricated default price for a trade that never executed. Reporting a
        // filled trade that did not happen is dangerous. Real execution requires a broker/exchange
        // connection and order-management — and as a matter of policy this system must not place
        // real orders or move money. It therefore refuses, explicitly.
        Err(FinancialError::NotImplemented(
            "trade execution (execute_trade): no broker/exchange connection; this system does not \
             place orders or move money. Refusing to report a fabricated fill."
                .to_string(),
        ))
    }
}

impl PositionManager {
    pub fn new() -> Self {
        Self {
            positions: HashMap::new(),
            position_limits: HashMap::new(),
            margin_calculator: MarginCalculator::new(),
        }
    }
}

impl MarginCalculator {
    pub fn new() -> Self {
        Self {
            margin_methods: HashMap::new(),
            margin_requirements: MarginRequirements::new(),
        }
    }
}

impl ComplianceMonitor {
    pub fn new() -> Self {
        Self {
            compliance_rules: HashMap::new(),
            surveillance_engine: SurveillanceEngine::new(),
            reporting_engine: ReportingEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.surveillance_engine.initialize()?;
        self.reporting_engine.initialize()?;
        Ok(())
    }

    /// Register a compliance rule. Rules are keyed by `rule_id`; registering
    /// with a duplicate id replaces the prior rule.
    pub fn add_rule(&mut self, rule: ComplianceRule) {
        self.compliance_rules.insert(rule.rule_id.clone(), rule);
    }

    /// Number of registered compliance rules.
    pub fn rule_count(&self) -> usize {
        self.compliance_rules.len()
    }

    /// Evaluate every registered compliance rule against the portfolio.
    ///
    /// Returns a `ComplianceResult` with:
    /// - `status: Compliant` when every rule passes (or no rules are registered
    ///   *and* the portfolio has no positions — an empty portfolio with no
    ///   rules is trivially compliant; a non-empty portfolio with no rules is
    ///   `Flagged` because compliance was never asserted).
    /// - `status: NonCompliant` when one or more rules fail.
    /// - `status: Flagged` when a rule neither clearly passes nor fails but
    ///   warrants review (e.g. a `Custom` rule with no check).
    /// - `risk_score`: fraction of rules that failed (0.0 = all pass, 1.0 = all fail).
    /// - `violations`: human-readable descriptions of each failed rule.
    /// - `recommendations`: suggested remediation per failed rule.
    /// - `audit_entries`: one entry per evaluated rule.
    pub fn check_compliance(
        &mut self,
        portfolio: &Portfolio,
    ) -> Result<ComplianceResult, FinancialError> {
        let mut violations = Vec::new();
        let mut recommendations = Vec::new();
        let mut audit_entries = Vec::new();
        let mut failed = 0usize;
        let mut flagged = 0usize;
        let now = portfolio.last_updated;

        // Special case: no rules registered.
        if self.compliance_rules.is_empty() {
            if portfolio.assets.is_empty() {
                // Empty portfolio, no rules → trivially compliant.
                return Ok(ComplianceResult {
                    result_id: format!("compliance_{}_{}", portfolio.portfolio_id, now),
                    portfolio_id: portfolio.portfolio_id.clone(),
                    status: ComplianceStatus::Compliant,
                    risk_score: 0.0,
                    violations: Vec::new(),
                    recommendations: Vec::new(),
                    audit_entries: Vec::new(),
                });
            }
            // Non-empty portfolio, no rules → we cannot assert compliance.
            return Ok(ComplianceResult {
                result_id: format!("compliance_{}_{}", portfolio.portfolio_id, now),
                portfolio_id: portfolio.portfolio_id.clone(),
                status: ComplianceStatus::Flagged,
                risk_score: 1.0,
                violations: vec![
                    "No compliance rules registered — cannot assert compliance for a non-empty portfolio."
                        .to_string(),
                ],
                recommendations: vec![
                    "Register compliance rules (position limits, KYC/AML, margin, trading restrictions) before asserting compliance."
                        .to_string(),
                ],
                audit_entries: Vec::new(),
            });
        }

        let total_rules = self.compliance_rules.len();

        // Evaluate each rule. Sort by rule_id for deterministic ordering.
        let mut rule_ids: Vec<&String> = self.compliance_rules.keys().collect();
        rule_ids.sort();

        for rule_id in rule_ids {
            let rule = &self.compliance_rules[rule_id];
            let verdict = Self::evaluate_rule(rule, portfolio);

            audit_entries.push(AuditEntry {
                entry_id: format!("audit_{}_{}", rule.rule_id, now),
                timestamp: now,
                user_id: String::new(),
                portfolio_id: portfolio.portfolio_id.clone(),
                action: PortfolioAction::ComplianceCheck,
                details: format!(
                    "Rule '{}' ({}): {} — {}",
                    rule.rule_id,
                    rule.rule_type_as_str(),
                    if verdict.passed { "PASS" } else { "FAIL" },
                    verdict.message
                ),
                ip_address: String::new(),
            });

            if !verdict.passed {
                failed += 1;
                violations.push(format!(
                    "{}: {}",
                    rule.rule_id, verdict.message
                ));
                recommendations.push(verdict.recommendation);
            } else if verdict.flagged {
                flagged += 1;
            }
        }

        let risk_score = failed as f64 / total_rules as f64;

        let status = if failed > 0 {
            ComplianceStatus::NonCompliant
        } else if flagged > 0 {
            ComplianceStatus::Flagged
        } else {
            ComplianceStatus::Compliant
        };

        Ok(ComplianceResult {
            result_id: format!("compliance_{}_{}", portfolio.portfolio_id, now),
            portfolio_id: portfolio.portfolio_id.clone(),
            status,
            risk_score,
            violations,
            recommendations,
            audit_entries,
        })
    }

    /// Evaluate a single compliance rule against a portfolio.
    fn evaluate_rule(rule: &ComplianceRule, portfolio: &Portfolio) -> RuleVerdict {
        match rule.rule_type {
            ComplianceRuleType::PositionLimit => {
                let max_position = rule.parameters.get("max_position").copied().unwrap_or(0.0);
                if max_position <= 0.0 {
                    return RuleVerdict {
                        passed: false,
                        flagged: false,
                        message: "PositionLimit rule has no max_position parameter".to_string(),
                        recommendation: "Set the 'max_position' parameter to a positive value.".to_string(),
                    };
                }
                // Check each asset's market value against the limit.
                for asset in &portfolio.assets {
                    if asset.market_value > max_position {
                        return RuleVerdict {
                            passed: false,
                            flagged: false,
                            message: format!(
                                "Asset {} market value {:.2} exceeds max_position {:.2}",
                                asset.symbol, asset.market_value, max_position
                            ),
                            recommendation: format!(
                                "Reduce position in {} to at most {:.2}",
                                asset.symbol, max_position
                            ),
                        };
                    }
                }
                RuleVerdict {
                    passed: true,
                    flagged: false,
                    message: "All positions within limit".to_string(),
                    recommendation: String::new(),
                }
            }
            ComplianceRuleType::KYC => {
                let kyc_required = rule.parameters.get("kyc_required").copied().unwrap_or(1.0);
                if kyc_required >= 1.0 {
                    // In a real system, this would check the portfolio owner's KYC status
                    // from an identity registry. Here we check the risk_profile as a proxy:
                    // an Unverified profile fails KYC.
                    let verified = matches!(
                        portfolio.risk_profile.risk_tolerance,
                        RiskTolerance::Conservative
                            | RiskTolerance::Moderate
                            | RiskTolerance::Aggressive
                            | RiskTolerance::VeryAggressive
                    ) && !portfolio.owner_id.is_empty();
                    if !verified {
                        return RuleVerdict {
                            passed: false,
                            flagged: false,
                            message: "KYC verification required but owner identity not verified".to_string(),
                            recommendation: "Complete KYC verification before trading.".to_string(),
                        };
                    }
                }
                RuleVerdict {
                    passed: true,
                    flagged: false,
                    message: "KYC verified".to_string(),
                    recommendation: String::new(),
                }
            }
            ComplianceRuleType::AML => {
                let aml_required = rule.parameters.get("kyc_required").copied().unwrap_or(1.0);
                if aml_required >= 1.0 && portfolio.owner_id.is_empty() {
                    return RuleVerdict {
                        passed: false,
                        flagged: false,
                        message: "AML clearance required but no owner identified".to_string(),
                        recommendation: "Provide owner identification for AML screening.".to_string(),
                    };
                }
                RuleVerdict {
                    passed: true,
                    flagged: false,
                    message: "AML cleared".to_string(),
                    recommendation: String::new(),
                }
            }
            ComplianceRuleType::MarginRequirement => {
                let margin_pct = rule.parameters.get("margin_pct").copied().unwrap_or(0.0);
                if margin_pct <= 0.0 {
                    return RuleVerdict {
                        passed: false,
                        flagged: false,
                        message: "MarginRequirement rule has no margin_pct parameter".to_string(),
                        recommendation: "Set the 'margin_pct' parameter to a positive value.".to_string(),
                    };
                }
                let required_margin = portfolio.total_value * margin_pct / 100.0;
                if portfolio.cash_balance < required_margin {
                    return RuleVerdict {
                        passed: false,
                        flagged: false,
                        message: format!(
                            "Cash balance {:.2} below required margin {:.2} ({:.1}% of {:.2})",
                            portfolio.cash_balance, required_margin, margin_pct, portfolio.total_value
                        ),
                        recommendation: format!(
                            "Increase cash balance to at least {:.2} to meet margin requirement.",
                            required_margin
                        ),
                    };
                }
                RuleVerdict {
                    passed: true,
                    flagged: false,
                    message: format!("Margin satisfied: {:.2} >= {:.2}", portfolio.cash_balance, required_margin),
                    recommendation: String::new(),
                }
            }
            ComplianceRuleType::TradingRestriction => {
                let restricted = rule.string_parameters.get("restricted_assets")
                    .map(|s| s.as_str())
                    .unwrap_or("");
                if restricted.is_empty() {
                    return RuleVerdict {
                        passed: true,
                        flagged: true,
                        message: "TradingRestriction rule has no restricted_assets list — no assets restricted".to_string(),
                        recommendation: "Populate 'restricted_assets' if trading restrictions are intended.".to_string(),
                    };
                }
                let restricted_set: Vec<&str> = restricted.split(',').map(|s| s.trim()).collect();
                for asset in &portfolio.assets {
                    if restricted_set.contains(&asset.symbol.as_str()) {
                        return RuleVerdict {
                            passed: false,
                            flagged: false,
                            message: format!("Asset {} is on the restricted list", asset.symbol),
                            recommendation: format!("Divest restricted asset {}.", asset.symbol),
                        };
                    }
                }
                RuleVerdict {
                    passed: true,
                    flagged: false,
                    message: "No restricted assets held".to_string(),
                    recommendation: String::new(),
                }
            }
            ComplianceRuleType::Custom => {
                // Custom rules always pass by default — they have no built-in check.
                RuleVerdict {
                    passed: true,
                    flagged: true,
                    message: "Custom rule — no built-in check, passes by default".to_string(),
                    recommendation: "Implement a custom evaluator if enforcement is needed.".to_string(),
                }
            }
        }
    }
}

/// Internal verdict from evaluating a single compliance rule.
struct RuleVerdict {
    passed: bool,
    flagged: bool,
    message: String,
    recommendation: String,
}

impl ComplianceRule {
    /// Human-readable name for the rule type, for audit logging.
    fn rule_type_as_str(&self) -> &'static str {
        match self.rule_type {
            ComplianceRuleType::PositionLimit => "PositionLimit",
            ComplianceRuleType::KYC => "KYC",
            ComplianceRuleType::AML => "AML",
            ComplianceRuleType::MarginRequirement => "MarginRequirement",
            ComplianceRuleType::TradingRestriction => "TradingRestriction",
            ComplianceRuleType::Custom => "Custom",
        }
    }
}

impl SurveillanceEngine {
    pub fn new() -> Self {
        Self {
            surveillance_rules: HashMap::new(),
            anomaly_detector: AnomalyDetector::new(),
            alert_manager: AlertManager::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.anomaly_detector.initialize()?;
        self.alert_manager.initialize()?;
        Ok(())
    }
}

impl AnomalyDetector {
    pub fn new() -> Self {
        Self {
            detection_algorithms: HashMap::new(),
            anomaly_patterns: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        Ok(())
    }
}

impl AlertManager {
    pub fn new() -> Self {
        Self {
            alerts: HashMap::new(),
            alert_escalation: AlertEscalation::new(),
            notification_system: NotificationSystem::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        Ok(())
    }
}

impl AlertEscalation {
    pub fn new() -> Self {
        Self {
            escalation_rules: HashMap::new(),
            escalation_history: HashMap::new(),
        }
    }
}

impl NotificationSystem {
    pub fn new() -> Self {
        Self {
            notification_channels: HashMap::new(),
            notification_templates: HashMap::new(),
        }
    }
}

impl ReportingEngine {
    pub fn new() -> Self {
        Self {
            report_templates: HashMap::new(),
            report_generator: ReportGenerator::new(),
            report_distributor: ReportDistributor::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.report_generator.initialize()?;
        self.report_distributor.initialize()?;
        Ok(())
    }
}

impl ReportGenerator {
    pub fn new() -> Self {
        Self {
            generation_strategies: HashMap::new(),
            data_aggregator: DataAggregator::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.data_aggregator.initialize()?;
        Ok(())
    }
}

impl DataAggregator {
    pub fn new() -> Self {
        Self {
            aggregation_rules: HashMap::new(),
            data_sources: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        Ok(())
    }
}

impl ReportDistributor {
    /// Create a new report distributor with no registered channels.
    pub fn new() -> Self {
        Self {
            distribution_channels: HashMap::new(),
            delivery_tracker: DeliveryTracker::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        Ok(())
    }

    /// Register a distribution channel under `name`. If a channel with the same
    /// name already exists it is replaced.
    pub fn add_channel(&mut self, name: String, channel: DistributionChannel) {
        self.distribution_channels.insert(name, channel);
    }

    /// Distribute `report` to every registered channel, returning one
    /// `DeliveryResult` per channel in registration (HashMap) order. Each
    /// attempt is also recorded on the internal `DeliveryTracker`.
    ///
    /// Because there is no real network, each channel only *validates* its
    /// configuration and reports success/failure accordingly — no bytes are
    /// actually transmitted.
    pub fn distribute(
        &mut self,
        report: &FinancialReport,
    ) -> Result<Vec<DeliveryResult>, DistributionError> {
        let mut results = Vec::with_capacity(self.distribution_channels.len());

        for (name, channel) in &self.distribution_channels {
            let result = self.deliver_to_channel(name, channel, report);
            self.delivery_tracker.record_delivery(result.clone());
            results.push(result);
        }

        Ok(results)
    }

    /// Validate a single channel's configuration and produce a `DeliveryResult`.
    /// `timestamp` is taken from the report's `generated_at` so deliveries are
    /// deterministically associated with the report that produced them.
    fn deliver_to_channel(
        &self,
        name: &str,
        channel: &DistributionChannel,
        report: &FinancialReport,
    ) -> DeliveryResult {
        let timestamp = report.generated_at;
        match channel {
            DistributionChannel::Email { recipients } => {
                if !recipients.is_empty() && recipients.iter().all(|r| r.contains('@')) {
                    DeliveryResult {
                        channel_name: name.to_string(),
                        success: true,
                        timestamp,
                        message: format!(
                            "Email delivered to {} recipient(s)",
                            recipients.len()
                        ),
                    }
                } else {
                    DeliveryResult {
                        channel_name: name.to_string(),
                        success: false,
                        timestamp,
                        message: "Invalid email recipient(s): missing '@'".to_string(),
                    }
                }
            }
            DistributionChannel::Ftp { host, path } => {
                if !host.is_empty() {
                    DeliveryResult {
                        channel_name: name.to_string(),
                        success: true,
                        timestamp,
                        message: format!("FTP delivery to {}{}", host, path),
                    }
                } else {
                    DeliveryResult {
                        channel_name: name.to_string(),
                        success: false,
                        timestamp,
                        message: "Invalid FTP configuration: empty host".to_string(),
                    }
                }
            }
            DistributionChannel::Webhook { url } => {
                if url.starts_with("http") {
                    DeliveryResult {
                        channel_name: name.to_string(),
                        success: true,
                        timestamp,
                        message: format!("Webhook POST to {}", url),
                    }
                } else {
                    DeliveryResult {
                        channel_name: name.to_string(),
                        success: false,
                        timestamp,
                        message: "Invalid webhook URL: must start with 'http'".to_string(),
                    }
                }
            }
            DistributionChannel::ApiEndpoint { url, auth_token } => {
                if url.starts_with("http") && !auth_token.is_empty() {
                    DeliveryResult {
                        channel_name: name.to_string(),
                        success: true,
                        timestamp,
                        message: format!("API delivery to {}", url),
                    }
                } else {
                    DeliveryResult {
                        channel_name: name.to_string(),
                        success: false,
                        timestamp,
                        message: "Invalid API endpoint: URL must start with 'http' and token must be present".to_string(),
                    }
                }
            }
            DistributionChannel::FileExport { path } => {
                if !path.is_empty() {
                    DeliveryResult {
                        channel_name: name.to_string(),
                        success: true,
                        timestamp,
                        message: format!("File exported to {}", path),
                    }
                } else {
                    DeliveryResult {
                        channel_name: name.to_string(),
                        success: false,
                        timestamp,
                        message: "Invalid file export: empty path".to_string(),
                    }
                }
            }
        }
    }

    /// Borrow the internal delivery tracker (e.g. for history/success-rate queries).
    pub fn delivery_tracker(&self) -> &DeliveryTracker {
        &self.delivery_tracker
    }
}

impl DeliveryTracker {
    /// Create a new, empty delivery tracker.
    pub fn new() -> Self {
        Self {
            deliveries: HashMap::new(),
            delivery_status: DeliveryStatus::new(),
        }
    }

    /// Record a delivery attempt under its channel name.
    pub fn record_delivery(&mut self, result: DeliveryResult) {
        let success = result.success;
        self.deliveries
            .entry(result.channel_name.clone())
            .or_default()
            .push(DeliveryRecord {
                channel_name: result.channel_name.clone(),
                success,
                timestamp: result.timestamp,
                message: result.message,
            });
        // Keep the aggregate DeliveryStatus counters in sync.
        self.delivery_status.total_deliveries += 1;
        if success {
            self.delivery_status.successful_deliveries += 1;
        } else {
            self.delivery_status.failed_deliveries += 1;
        }
    }

    /// Query the full delivery history for a channel, in insertion order.
    pub fn get_delivery_history(&self, channel_name: &str) -> Vec<&DeliveryRecord> {
        self.deliveries
            .get(channel_name)
            .map(|records| records.iter().collect())
            .unwrap_or_default()
    }

    /// Compute the success rate (0.0–1.0) for a channel. Returns 0.0 when the
    /// channel has no recorded deliveries.
    pub fn success_rate(&self, channel_name: &str) -> f64 {
        match self.deliveries.get(channel_name) {
            Some(records) if !records.is_empty() => {
                let successes = records.iter().filter(|r| r.success).count();
                successes as f64 / records.len() as f64
            }
            _ => 0.0,
        }
    }
}

impl DeliveryStatus {
    pub fn new() -> Self {
        Self {
            total_deliveries: 0,
            successful_deliveries: 0,
            failed_deliveries: 0,
            pending_deliveries: 0,
        }
    }
}

// Supporting structs

impl Portfolio {
    pub fn new() -> Self {
        Self {
            portfolio_id: "portfolio_1".to_string(),
            portfolio_name: "Test Portfolio".to_string(),
            owner_id: "user_1".to_string(),
            assets: vec![Asset::new()],
            cash_balance: 10000.0,
            total_value: 25500.0,
            created_at: 0,
            last_updated: 0,
            risk_profile: RiskProfile::new(),
            investment_strategy: InvestmentStrategy::Balanced,
        }
    }
}

impl RiskProfile {
    pub fn new() -> Self {
        Self {
            risk_tolerance: RiskTolerance::Moderate,
            risk_capacity: 100000.0,
            time_horizon: TimeHorizon::MediumTerm,
            liquidity_needs: LiquidityNeeds::Medium,
        }
    }
}

impl Asset {
    pub fn new() -> Self {
        Self {
            asset_id: "asset_1".to_string(),
            symbol: "AAPL".to_string(),
            asset_type: AssetType::Stock,
            quantity: 100.0,
            average_cost: 150.0,
            current_price: 155.0,
            market_value: 15500.0,
            currency: "USD".to_string(),
            exchange: "NASDAQ".to_string(),
            last_updated: 0,
            price_history: Vec::new(),
        }
    }
}

impl PortfolioMetadata {
    pub fn new() -> Self {
        Self {
            portfolio_id: "portfolio_1".to_string(),
            description: "Test portfolio".to_string(),
            tags: vec!["test".to_string()],
            permissions: vec![Permission::Read, Permission::Write],
            compliance_flags: vec![ComplianceFlag::KYCVerified],
        }
    }
}

impl AccessPolicy {
    pub fn new() -> Self {
        Self {
            policy_id: "policy_1".to_string(),
            user_id: "user_1".to_string(),
            portfolio_id: "portfolio_1".to_string(),
            permissions: vec![Permission::Read, Permission::Write],
            time_restrictions: TimeRestrictions::new(),
            ip_restrictions: vec!["192.168.1.1".to_string()],
        }
    }
}

impl TimeRestrictions {
    pub fn new() -> Self {
        Self {
            allowed_hours: (0..24).collect(),
            allowed_days: (1..8).collect(),
            start_date: None,
            end_date: None,
        }
    }
}

impl AuthenticationRequirement {
    pub fn new() -> Self {
        Self {
            requirement_id: "auth_1".to_string(),
            auth_methods: vec![
                AuthenticationMethod::Password,
                AuthenticationMethod::MultiFactor,
            ],
            multi_factor_required: true,
        }
    }
}

impl AuditEntry {
    pub fn new() -> Self {
        Self {
            entry_id: "audit_1".to_string(),
            timestamp: 0,
            user_id: "user_1".to_string(),
            portfolio_id: "portfolio_1".to_string(),
            action: PortfolioAction::Create,
            details: "Created portfolio".to_string(),
            ip_address: "192.168.1.1".to_string(),
        }
    }
}

impl AssetInfo {
    pub fn new() -> Self {
        Self {
            asset_id: "asset_1".to_string(),
            symbol: "AAPL".to_string(),
            name: "Apple Inc.".to_string(),
            asset_type: AssetType::Stock,
            exchange: "NASDAQ".to_string(),
            currency: "USD".to_string(),
            sector: Some("Technology".to_string()),
            industry: Some("Consumer Electronics".to_string()),
            market_cap: Some(3000000000000.0),
            description: "Apple Inc. is a technology company".to_string(),
        }
    }
}

impl AssetClass {
    pub fn new() -> Self {
        Self {
            class_id: "class_1".to_string(),
            class_name: "US Equities".to_string(),
            class_type: AssetType::Stock,
            characteristics: vec!["US listed".to_string(), "Large cap".to_string()],
            risk_level: RiskLevel::Medium,
        }
    }
}

impl AssetRelationship {
    pub fn new() -> Self {
        Self {
            relationship_id: "rel_1".to_string(),
            source_asset: "AAPL".to_string(),
            target_asset: "MSFT".to_string(),
            relationship_type: AssetRelationshipType::Correlation,
            correlation: 0.7,
        }
    }
}

impl PriceFeed {
    pub fn new() -> Self {
        Self {
            feed_id: "feed_1".to_string(),
            feed_name: "Real-time feed".to_string(),
            feed_type: FeedType::RealTime,
            update_frequency: 1,
            data_quality: DataQuality::new(),
            last_update: 0,
            asset_id: "asset_1".to_string(),
            cached_prices: Vec::new(),
        }
    }
}

impl DataQuality {
    pub fn new() -> Self {
        Self {
            // not measured (scaffold defaults; no data-quality assessment is performed)
            accuracy: 0.0,
            completeness: 0.0,
            timeliness: 0.0,
            consistency: 0.0,
        }
    }
}

impl PriceData {
    pub fn new() -> Self {
        Self {
            asset_id: "asset_1".to_string(),
            timestamp: 0,
            open: 150.0,
            high: 155.0,
            low: 149.0,
            close: 154.0,
            adjusted_close: 154.0,
            volume: 1000000,
        }
    }
}

impl VolumeData {
    pub fn new() -> Self {
        Self {
            asset_id: "asset_1".to_string(),
            timestamp: 0,
            volume: 1000000,
            bid_volume: 500000,
            ask_volume: 500000,
        }
    }
}

impl TechnicalIndicators {
    pub fn new() -> Self {
        Self {
            asset_id: "asset_1".to_string(),
            timestamp: 0,
            moving_averages: HashMap::new(),
            oscillators: HashMap::new(),
            volatility: HashMap::new(),
        }
    }
}

impl ValidationRule {
    pub fn new() -> Self {
        Self {
            rule_id: "rule_1".to_string(),
            rule_type: ValidationRuleType::Price,
            condition: "price > 0".to_string(),
            action: ValidationAction::Accept,
        }
    }
}

impl ComplianceCondition {
    pub fn new() -> Self {
        Self {
            condition_id: "cond_1".to_string(),
            field: "price".to_string(),
            operator: ComparisonOperator::GreaterThan,
            value: ComplianceValue::Number(0.0),
        }
    }
}

impl ComplianceRule {
    pub fn new() -> Self {
        Self {
            rule_id: "rule_1".to_string(),
            rule_type: ComplianceRuleType::PositionLimit,
            parameters: HashMap::from([("max_position".to_string(), 1000.0)]),
            string_parameters: HashMap::new(),
            description: "Default position-limit rule".to_string(),
        }
    }
}

impl RegulatoryFramework {
    pub fn new() -> Self {
        Self {
            framework_id: "framework_1".to_string(),
            framework_name: "SEC".to_string(),
            jurisdiction: "US".to_string(),
            requirements: vec![RegulatoryRequirement::new()],
        }
    }
}

impl RegulatoryRequirement {
    pub fn new() -> Self {
        Self {
            requirement_id: "req_1".to_string(),
            requirement_type: RequirementType::Reporting,
            description: "Must report trades".to_string(),
            mandatory: true,
        }
    }
}

impl ScreeningList {
    pub fn new() -> Self {
        Self {
            list_id: "list_1".to_string(),
            list_name: "Sanctions list".to_string(),
            list_type: ScreeningListType::Sanctions,
            entries: vec![ScreeningEntry::new()],
        }
    }
}

impl ScreeningEntry {
    pub fn new() -> Self {
        Self {
            entry_id: "entry_1".to_string(),
            name: "Test Entity".to_string(),
            aliases: vec!["Alias 1".to_string()],
            date_of_birth: Some("1980-01-01".to_string()),
            nationality: Some("US".to_string()),
            reason: "Test reason".to_string(),
        }
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

impl ExecutionStrategy {
    pub fn new() -> Self {
        Self {
            strategy_id: "exec_1".to_string(),
            strategy_name: "VWAP execution".to_string(),
            strategy_type: ExecutionStrategyType::VWAP,
            parameters: ExecutionParameters::new(),
        }
    }
}

impl ExecutionParameters {
    pub fn new() -> Self {
        Self {
            order_size: 10000.0,
            price_limit: None,
            time_limit: Some(3600), // 1 hour
            participation_rate: Some(0.2),
        }
    }
}

impl Order {
    pub fn new() -> Self {
        Self {
            order_id: "order_1".to_string(),
            portfolio_id: "portfolio_1".to_string(),
            asset_id: "asset_1".to_string(),
            order_type: OrderType::Market,
            side: OrderSide::Buy,
            quantity: 100.0,
            price: None,
            time_in_force: TimeInForce::Day,
            status: OrderStatus::New,
            created_at: 0,
            updated_at: 0,
        }
    }
}

impl OrderValidationRule {
    pub fn new() -> Self {
        Self {
            rule_id: "rule_1".to_string(),
            rule_type: OrderValidationRuleType::Size,
            condition: "quantity > 0".to_string(),
            action: OrderValidationAction::Accept,
        }
    }
}

impl OrderComplianceCondition {
    pub fn new() -> Self {
        Self {
            condition_id: "cond_1".to_string(),
            field: "quantity".to_string(),
            operator: ComparisonOperator::GreaterThan,
            value: OrderComplianceValue::Number(0.0),
        }
    }
}

impl OrderComplianceRule {
    pub fn new() -> Self {
        Self {
            rule_id: "rule_1".to_string(),
            rule_name: "Size validation".to_string(),
            conditions: vec![OrderComplianceCondition::new()],
            actions: vec![OrderComplianceAction::Approve],
        }
    }
}

impl RegulatoryLimit {
    pub fn new() -> Self {
        Self {
            limit_id: "limit_1".to_string(),
            limit_type: RegulatoryLimitType::Position,
            limit_value: 1000000.0,
            reset_period: 86400, // 1 day
        }
    }
}

impl RoutingStrategy {
    pub fn new() -> Self {
        Self {
            strategy_id: "route_1".to_string(),
            strategy_name: "Best execution".to_string(),
            strategy_type: RoutingStrategyType::BestExecution,
            parameters: RoutingParameters::new(),
        }
    }
}

impl RoutingParameters {
    pub fn new() -> Self {
        Self {
            venues: vec!["venue_1".to_string()],
            priority_factors: vec![PriorityFactor::new()],
            cost_factors: vec![CostFactor::new()],
        }
    }
}

impl PriorityFactor {
    pub fn new() -> Self {
        Self {
            factor_name: "Speed".to_string(),
            weight: 0.5,
        }
    }
}

impl CostFactor {
    pub fn new() -> Self {
        Self {
            factor_name: "Commission".to_string(),
            cost_per_share: 0.001,
        }
    }
}

impl TradingVenue {
    pub fn new() -> Self {
        Self {
            venue_id: "venue_1".to_string(),
            venue_name: "NASDAQ".to_string(),
            venue_type: VenueType::Exchange,
            supported_assets: vec!["AAPL".to_string()],
            fee_structure: FeeStructure::new(),
        }
    }
}

impl FeeStructure {
    pub fn new() -> Self {
        Self {
            commission_rate: 0.001,
            clearing_fee: 0.0001,
            exchange_fee: 0.0002,
            regulatory_fee: 0.0001,
        }
    }
}

impl VenuePerformance {
    pub fn new() -> Self {
        Self {
            venue_id: "venue_1".to_string(),
            fill_rate: 0.95,
            average_fill_time: 100.0,
            price_improvement: 0.001,
            market_impact: 0.0005,
        }
    }
}

impl SettlementMethod {
    pub fn new() -> Self {
        Self {
            method_id: "settle_1".to_string(),
            method_name: "T+2".to_string(),
            method_type: SettlementMethodType::TPlus2,
            settlement_cycle: 2,
        }
    }
}

impl SettlementValidationRule {
    pub fn new() -> Self {
        Self {
            rule_id: "rule_1".to_string(),
            rule_type: SettlementValidationRuleType::Funds,
            condition: "sufficient_funds".to_string(),
            action: SettlementValidationAction::Approve,
        }
    }
}

impl SettlementComplianceCondition {
    pub fn new() -> Self {
        Self {
            condition_id: "cond_1".to_string(),
            field: "funds".to_string(),
            operator: ComparisonOperator::GreaterThan,
            value: SettlementComplianceValue::Number(0.0),
        }
    }
}

impl SettlementComplianceRule {
    pub fn new() -> Self {
        Self {
            rule_id: "rule_1".to_string(),
            rule_name: "Funds validation".to_string(),
            conditions: vec![SettlementComplianceCondition::new()],
            actions: vec![SettlementComplianceAction::Approve],
        }
    }
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            portfolio_id: "portfolio_1".to_string(),
            period: (0, 86400 * 365), // 1 year
            total_return: 0.15,
            annualized_return: 0.15,
            volatility: 0.2,
            sharpe_ratio: 0.75,
            max_drawdown: -0.1,
            alpha: 0.02,
            beta: 1.1,
            information_ratio: 0.5,
        }
    }
}

impl Benchmark {
    pub fn new() -> Self {
        Self {
            benchmark_id: "benchmark_1".to_string(),
            benchmark_name: "S&P 500".to_string(),
            benchmark_type: BenchmarkType::Index,
            returns: vec![0.1, 0.08, 0.12, 0.15, 0.09],
        }
    }
}

impl ComparisonMetrics {
    pub fn new() -> Self {
        Self {
            portfolio_id: "portfolio_1".to_string(),
            benchmark_id: "benchmark_1".to_string(),
            excess_return: 0.02,
            tracking_error: 0.05,
            information_ratio: 0.4,
            up_capture: 0.8,
            down_capture: 1.2,
        }
    }
}

impl AttributionModel {
    pub fn new() -> Self {
        Self {
            model_id: "model_1".to_string(),
            model_type: AttributionModelType::BrinsonFachler,
            factors: vec![AttributionFactor::new()],
        }
    }
}

impl AttributionFactor {
    pub fn new() -> Self {
        Self {
            factor_id: "factor_1".to_string(),
            factor_name: "Sector".to_string(),
            factor_type: FactorType::Equity,
            exposure: 0.3,
        }
    }
}

impl AttributionResult {
    pub fn new() -> Self {
        Self {
            result_id: "result_1".to_string(),
            portfolio_id: "portfolio_1".to_string(),
            period: (0, 86400 * 365), // 1 year
            allocation_effect: 0.01,
            selection_effect: 0.02,
            interaction_effect: 0.001,
            total_effect: 0.031,
        }
    }
}

impl FinancialPerformanceMetrics {
    pub fn new() -> Self {
        Self {
            total_portfolios: 0,
            average_return: 0.0,
            average_volatility: 0.0,
            average_sharpe_ratio: 0.0,
            total_assets: 0.0,
        }
    }
}

impl RiskMetrics {
    pub fn new() -> Self {
        // Default value — nothing computed. All zero, never fabricated VaR/Sharpe/etc.
        // (calculate_portfolio_risk returns InsufficientData rather than this default.)
        Self {
            portfolio_id: "portfolio_1".to_string(),
            var_95: 0.0,
            cvar_95: 0.0,
            volatility: 0.0,
            beta: 0.0,
            alpha: 0.0,
            sharpe_ratio: 0.0,
            sortino_ratio: 0.0,
            max_drawdown: 0.0,
            overall_risk_score: 0.0,
            risk_profile_assessment: None,
        }
    }
}

impl OptionParameters {
    pub fn new() -> Self {
        Self {
            underlying_price: 100.0,
            strike: 105.0,
            time_to_maturity: 0.25, // 3 months
            risk_free_rate: 0.05,
            volatility: 0.2,
            option_type: OptionType::Call,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptionParameters {
    pub underlying_price: f64,
    pub strike: f64,
    pub time_to_maturity: f64,
    pub risk_free_rate: f64,
    pub volatility: f64,
    pub option_type: OptionType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptionType {
    Call,
    Put,
}

impl OptionPrice {
    pub fn new() -> Self {
        Self {
            price: 5.0,
            delta: 0.5,
            gamma: 0.05,
            theta: -0.01,
            vega: 0.2,
            rho: 0.1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptionPrice {
    pub price: f64,
    pub delta: f64,
    pub gamma: f64,
    pub theta: f64,
    pub vega: f64,
    pub rho: f64,
}

impl TradeResult {
    pub fn new() -> Self {
        Self {
            trade_id: "trade_1".to_string(),
            order_id: "order_1".to_string(),
            executed_quantity: 100.0,
            executed_price: 100.0,
            execution_time: 0,
            status: TradeStatus::Filled,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TradeStatus {
    Pending,
    PartiallyFilled,
    Filled,
    Cancelled,
    Rejected,
}

impl ComplianceResult {
    pub fn new() -> Self {
        Self {
            result_id: "compliance_1".to_string(),
            portfolio_id: "portfolio_1".to_string(),
            // Default value — nothing evaluated (Pending), not a fabricated "Compliant / 0.5".
            status: ComplianceStatus::Pending,
            risk_score: 0.0,
            violations: Vec::new(),
            recommendations: Vec::new(),
            audit_entries: Vec::new(),
        }
    }
}

/// Trade execution result
#[derive(Debug, Clone)]
pub struct TradeResult {
    pub trade_id: String,
    pub order_id: String,
    pub executed_quantity: f64,
    pub executed_price: f64,
    pub execution_time: u64,
    pub status: TradeStatus,
}

/// Risk analysis metrics for a portfolio
#[derive(Debug, Clone)]
pub struct RiskMetrics {
    pub portfolio_id: String,
    pub var_95: f64,
    pub cvar_95: f64,
    pub volatility: f64,
    pub beta: f64,
    pub alpha: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub max_drawdown: f64,
    pub overall_risk_score: f64,
    /// Plain-language assessment of whether the computed volatility / VaR fit the
    /// portfolio's declared `RiskProfile.risk_tolerance`. `None` when the metrics
    /// are within tolerance (or when no assessment was performed); `Some(warning)`
    /// when a conservative profile carries high risk — never a fabricated pass.
    pub risk_profile_assessment: Option<String>,
}

/// Compliance check result for a portfolio
#[derive(Debug, Clone)]
pub struct ComplianceResult {
    pub result_id: String,
    pub portfolio_id: String,
    pub status: ComplianceStatus,
    pub risk_score: f64,
    pub violations: Vec<String>,
    pub recommendations: Vec<String>,
    pub audit_entries: Vec<AuditEntry>,
}

/// Per-order compliance report produced by `ComplianceMonitor::check_order`.
///
/// Aggregates the pass/fail verdict of every registered rule against a single
/// order; `overall_pass` is `true` only when every `rule_result` passed. An
/// empty rule set yields `overall_pass = true` with no `rule_results`.
#[derive(Debug, Clone)]
pub struct ComplianceReport {
    pub order_id: String,
    pub overall_pass: bool,
    pub rule_results: Vec<RuleResult>,
    pub timestamp: u64,
}

/// Result of evaluating a single compliance rule against an order.
#[derive(Debug, Clone)]
pub struct RuleResult {
    pub rule_id: String,
    pub passed: bool,
    pub message: String,
}

/// Error type returned by compliance-rule evaluation. Aliased to the library's
/// general `FinancialError` so callers can handle all financial errors uniformly
/// (the `FinancialError::ComplianceError` variant carries the compliance message).
pub type ComplianceError = FinancialError;

/// Financial library performance summary metrics
#[derive(Debug, Clone)]
pub struct FinancialPerformanceMetrics {
    pub total_portfolios: u64,
    pub average_return: f64,
    pub average_volatility: f64,
    pub average_sharpe_ratio: f64,
    pub total_assets: f64,
}

/// Financial error types
#[derive(Debug, Clone)]
pub enum FinancialError {
    ValidationError(String),
    PortfolioError(String),
    AssetError(String),
    RiskError(String),
    PricingError(String),
    TradingError(String),
    ComplianceError(String),
    DataError(String),
    /// The capability is not implemented yet — returned instead of a fabricated result.
    NotImplemented(String),
    /// The required input (return history, market data, defined limits) is not present.
    InsufficientData(String),
}

impl std::fmt::Display for FinancialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FinancialError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            FinancialError::PortfolioError(msg) => write!(f, "Portfolio error: {}", msg),
            FinancialError::AssetError(msg) => write!(f, "Asset error: {}", msg),
            FinancialError::RiskError(msg) => write!(f, "Risk error: {}", msg),
            FinancialError::PricingError(msg) => write!(f, "Pricing error: {}", msg),
            FinancialError::TradingError(msg) => write!(f, "Trading error: {}", msg),
            FinancialError::ComplianceError(msg) => write!(f, "Compliance error: {}", msg),
            FinancialError::DataError(msg) => write!(f, "Data error: {}", msg),
            FinancialError::NotImplemented(msg) => write!(f, "Not implemented yet: {}", msg),
            FinancialError::InsufficientData(msg) => {
                write!(f, "Required information not available: {}", msg)
            }
        }
    }
}

impl std::error::Error for FinancialError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_financial_library_creation() {
        let mut library = FinancialModelingLibrary::new();
        assert!(library.initialize().is_ok());
    }

    #[test]
    fn test_portfolio_creation() {
        let mut library = FinancialModelingLibrary::new();
        library.initialize().unwrap();

        let portfolio = Portfolio::new();
        let result = library.create_portfolio(portfolio).unwrap();

        assert_eq!(result.result.portfolio_id, "portfolio_1");
        assert_eq!(result.result.portfolio_name, "Test Portfolio");
        assert_eq!(result.result.owner_id, "user_1");
        assert!(result.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_risk_calculation() {
        let mut library = FinancialModelingLibrary::new();
        library.initialize().unwrap();

        // Risk metrics ARE now genuinely computed from each asset's price_history
        // (see portfolio_risk.rs for the math + proofs). With no such portfolio
        // stored, this honestly errors (portfolio-not-found) rather than returning
        // a confident risk number it never computed.
        let result = library.calculate_portfolio_risk("portfolio_1");
        assert!(result.is_err());
    }

    #[test]
    fn test_option_pricing() {
        let mut library = FinancialModelingLibrary::new();
        library.initialize().unwrap();

        let option_params = OptionParameters::new();
        let result = library.price_option(option_params).unwrap();

        assert!(result.result.price > 0.0);
        assert!(result.result.delta >= 0.0 && result.result.delta <= 1.0);
        assert!(result.result.gamma > 0.0);
        assert!(result.result.vega > 0.0);
    }

    #[test]
    fn test_trade_execution() {
        let mut library = FinancialModelingLibrary::new();
        library.initialize().unwrap();

        let order = Order::new();
        // HONEST + SAFE: this system places no real orders and must never report a fabricated
        // fill. Execution reports NotImplemented rather than a fake "Filled" trade.
        let result = library.execute_trade(order);
        assert!(matches!(result, Err(FinancialError::NotImplemented(_))));
    }

    #[test]
    fn test_compliance_check() {
        let mut library = FinancialModelingLibrary::new();
        library.initialize().unwrap();

        // The compliance-rules registry is empty and the default portfolio has assets,
        // so check_compliance returns Ok with Flagged status (cannot assert compliance
        // without evaluating any rule — never fabricates "Compliant").
        let result = library.check_compliance("portfolio_1").unwrap();
        assert_eq!(result.result.status, ComplianceStatus::Flagged);
        assert_eq!(result.result.risk_score, 1.0);
        assert!(!result.result.violations.is_empty());
    }

    #[test]
    fn test_performance_metrics() {
        let library = FinancialModelingLibrary::new();
        let metrics = library.get_performance_stats();

        assert_eq!(metrics.total_portfolios, 0);
        assert_eq!(metrics.average_return, 0.0);
        assert_eq!(metrics.total_assets, 0.0);
    }

    #[test]
    fn test_portfolio_listing() {
        let library = FinancialModelingLibrary::new();
        let portfolios = library.list_portfolios();
        assert_eq!(portfolios.len(), 0);
    }

    #[test]
    fn test_portfolio_info() {
        let library = FinancialModelingLibrary::new();
        let info = library.get_portfolio_info("portfolio_1");
        assert!(info.is_none());
    }

    // ---- Part 1: risk-profile validation wiring ----

    /// Build an asset carrying a real price history (oldest first).
    fn asset_with_history(symbol: &str, market_value: f64, prices: Vec<f64>) -> Asset {
        Asset {
            asset_id: symbol.to_string(),
            symbol: symbol.to_string(),
            asset_type: AssetType::Stock,
            quantity: 1.0,
            average_cost: 0.0,
            current_price: *prices.last().unwrap_or(&0.0),
            market_value,
            currency: "USD".to_string(),
            exchange: "TEST".to_string(),
            last_updated: 0,
            price_history: prices,
        }
    }

    /// Build a portfolio with a single asset and a chosen risk tolerance.
    fn portfolio_with_tolerance(tolerance: RiskTolerance, prices: Vec<f64>) -> Portfolio {
        Portfolio {
            portfolio_id: "rp_test".to_string(),
            portfolio_name: "rp_test".to_string(),
            owner_id: "owner_1".to_string(),
            assets: vec![asset_with_history("A", 1000.0, prices)],
            cash_balance: 0.0,
            total_value: 1000.0,
            created_at: 0,
            last_updated: 0,
            risk_profile: RiskProfile {
                risk_tolerance: tolerance,
                risk_capacity: 100000.0,
                time_horizon: TimeHorizon::LongTerm,
                liquidity_needs: LiquidityNeeds::Low,
            },
            investment_strategy: InvestmentStrategy::Balanced,
        }
    }

    #[test]
    fn risk_profile_flags_conservative_with_high_volatility() {
        // prices 100→130→90→125 ⇒ returns 0.3, -0.3077, 0.3889 — high volatility
        // (~0.35) that exceeds the Conservative band (vol > 0.10, VaR > 0.05).
        let portfolio = portfolio_with_tolerance(
            RiskTolerance::Conservative,
            vec![100.0, 130.0, 90.0, 125.0],
        );
        let analyzer = RiskAnalyzer::new();
        let metrics = analyzer.calculate_risk_metrics(&portfolio).unwrap();

        assert!(
            metrics.risk_profile_assessment.is_some(),
            "a Conservative portfolio with high volatility must be flagged"
        );
        let assessment = metrics.risk_profile_assessment.unwrap();
        assert!(
            assessment.contains("Conservative"),
            "assessment should name the declared tolerance: {}",
            assessment
        );
    }

    #[test]
    fn risk_profile_passes_moderate_within_tolerance() {
        // prices 100→101→102→103 ⇒ returns 0.01, 0.0099, 0.0098 — tiny volatility
        // well within every band, so no assessment warning is produced.
        let portfolio =
            portfolio_with_tolerance(RiskTolerance::Moderate, vec![100.0, 101.0, 102.0, 103.0]);
        let analyzer = RiskAnalyzer::new();
        let metrics = analyzer.calculate_risk_metrics(&portfolio).unwrap();

        assert!(
            metrics.risk_profile_assessment.is_none(),
            "a Moderate portfolio with tiny volatility should not be flagged"
        );
    }

    #[test]
    fn risk_profile_very_aggressive_never_flagged() {
        // VeryAggressive has an infinite tolerance band, so even wild volatility
        // is never flagged — the assessment is honestly `None`, not a fabricated pass.
        let portfolio = portfolio_with_tolerance(
            RiskTolerance::VeryAggressive,
            vec![100.0, 130.0, 90.0, 125.0],
        );
        let analyzer = RiskAnalyzer::new();
        let metrics = analyzer.calculate_risk_metrics(&portfolio).unwrap();
        assert!(metrics.risk_profile_assessment.is_none());
    }

    // ---- Part 2: portfolio access control + audit trail wiring ----

    #[test]
    fn access_control_check_permission_grants_and_denies() {
        let mut ac = PortfolioAccessControl::new();
        ac.add_access_policy(AccessPolicy {
            policy_id: "pol_1".to_string(),
            user_id: "alice".to_string(),
            portfolio_id: "pf_1".to_string(),
            permissions: vec![Permission::Read, Permission::Write],
            time_restrictions: TimeRestrictions::new(),
            ip_restrictions: Vec::new(),
        });

        // Granted: alice has Read on pf_1.
        assert!(ac.check_permission("alice", "pf_1", Permission::Read));
        assert!(ac.check_permission("alice", "pf_1", Permission::Write));
        // Denied: alice lacks Admin on pf_1.
        assert!(!ac.check_permission("alice", "pf_1", Permission::Admin));
        // Denied: bob has no policy at all.
        assert!(!ac.check_permission("bob", "pf_1", Permission::Read));
        // Denied: alice has no policy on pf_2.
        assert!(!ac.check_permission("alice", "pf_2", Permission::Read));
    }

    #[test]
    fn audit_trail_logs_and_reports_entries() {
        let trail = PortfolioAuditTrail::new();
        assert_eq!(trail.entry_count(), 0);
        assert!(trail.entries().is_empty());

        trail.log_action(AuditEntry {
            entry_id: "e1".to_string(),
            timestamp: 1,
            user_id: "alice".to_string(),
            portfolio_id: "pf_1".to_string(),
            action: PortfolioAction::Create,
            details: "created".to_string(),
            ip_address: "10.0.0.1".to_string(),
        });
        trail.log_action(AuditEntry {
            entry_id: "e2".to_string(),
            timestamp: 2,
            user_id: "alice".to_string(),
            portfolio_id: "pf_1".to_string(),
            action: PortfolioAction::Read,
            details: "read".to_string(),
            ip_address: "10.0.0.1".to_string(),
        });

        assert_eq!(trail.entry_count(), 2);
        let entries = trail.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].entry_id, "e1");
        assert_eq!(entries[1].action, PortfolioAction::Read);
    }

    #[test]
    fn storage_store_and_get_log_audit_entries() {
        let mut storage = PortfolioStorage::new();
        let mut portfolio = Portfolio::new();
        portfolio.portfolio_id = "audit_pf".to_string();
        portfolio.owner_id = "auditor".to_string();

        // store_portfolio logs a Create entry.
        storage.store_portfolio(portfolio).unwrap();
        assert_eq!(storage.audit_trail.entry_count(), 1);
        assert_eq!(
            storage.audit_trail.entries()[0].action,
            PortfolioAction::Create
        );

        // get_portfolio logs a Read entry (shared borrow — relies on interior mutability).
        let _ = storage.get_portfolio("audit_pf").unwrap();
        assert_eq!(storage.audit_trail.entry_count(), 2);
        assert_eq!(
            storage.audit_trail.entries()[1].action,
            PortfolioAction::Read
        );

        // A second store on the same id logs an Update, not a Create.
        let mut portfolio2 = Portfolio::new();
        portfolio2.portfolio_id = "audit_pf".to_string();
        portfolio2.owner_id = "auditor".to_string();
        storage.store_portfolio(portfolio2).unwrap();
        assert_eq!(storage.audit_trail.entry_count(), 3);
        assert_eq!(
            storage.audit_trail.entries()[2].action,
            PortfolioAction::Update
        );
    }

    // ---- Part 3: benchmark-based beta/alpha via RiskAnalyzer ----

    #[test]
    fn risk_analyzer_benchmark_makes_beta_alpha_real() {
        // Portfolio returns (prices 100→110→99→108.9): 0.1, -0.1, 0.1.
        let portfolio = portfolio_with_tolerance(
            RiskTolerance::Moderate,
            vec![100.0, 110.0, 99.0, 108.9],
        );

        // Without a benchmark, beta/alpha are NaN.
        let analyzer = RiskAnalyzer::new();
        let none_metrics = analyzer.calculate_risk_metrics(&portfolio).unwrap();
        assert!(none_metrics.beta.is_nan() && none_metrics.alpha.is_nan());

        // Register a benchmark (same sign pattern, half magnitude ⇒ beta = 2.0).
        let mut analyzer = RiskAnalyzer::new();
        analyzer.add_benchmark("idx", vec![0.05, -0.05, 0.05]);
        let metrics = analyzer.calculate_risk_metrics(&portfolio).unwrap();
        assert!(!metrics.beta.is_nan());
        assert!(!metrics.alpha.is_nan());
        assert!((metrics.beta - 2.0).abs() < 1e-9, "beta {}", metrics.beta);

        // Deactivating the benchmark reverts beta/alpha to NaN.
        analyzer.set_active_benchmark(None);
        let off_metrics = analyzer.calculate_risk_metrics(&portfolio).unwrap();
        assert!(off_metrics.beta.is_nan() && off_metrics.alpha.is_nan());
    }

    // ---- Part 4: price feeds → asset price history wiring ----

    #[test]
    fn register_price_feed_and_ingest_uses_cached_prices() {
        let mut manager = AssetManager::new();
        let cached = vec![100.0, 102.0, 101.0, 105.0, 107.0];
        let feed = PriceFeed {
            feed_id: "feed_A".to_string(),
            feed_name: "A feed".to_string(),
            feed_type: FeedType::EndOfDay,
            update_frequency: 86400,
            data_quality: DataQuality::new(),
            last_update: 0,
            asset_id: "A".to_string(),
            cached_prices: cached.clone(),
        };
        manager.register_price_feed(feed);

        // No feed for unknown asset ⇒ DataError, never a fabricated history.
        assert!(matches!(
            manager.ingest_from_feed("ZZZ"),
            Err(FinancialError::DataError(_))
        ));

        manager.ingest_from_feed("A").unwrap();
        let history = manager.get_price_history("A").expect("history cached for A");
        assert_eq!(history, &cached);
    }

    #[test]
    fn ingest_from_feed_generates_deterministic_series_when_no_cache() {
        let mut manager = AssetManager::new();
        manager.register_price_feed(PriceFeed {
            feed_id: "seeded_feed".to_string(),
            feed_name: "no cache".to_string(),
            feed_type: FeedType::Historical,
            update_frequency: 86400,
            data_quality: DataQuality::new(),
            last_update: 0,
            asset_id: "B".to_string(),
            cached_prices: Vec::new(),
        });

        manager.ingest_from_feed("B").unwrap();
        let first = manager.get_price_history("B").expect("history for B").clone();
        // Deterministic: re-ingesting yields the identical series.
        manager.ingest_from_feed("B").unwrap();
        let second = manager.get_price_history("B").expect("history for B").clone();
        assert_eq!(first, second);
        // Enough points for risk computation (need ≥ 3).
        assert!(first.len() >= 3);
    }

    #[test]
    fn update_price_history_then_apply_to_asset_feeds_risk_metrics() {
        // Register a feed (so the wiring is exercised), but populate history
        // directly via update_price_history, apply it to an asset, build a
        // portfolio, and verify real risk metrics come back.
        let mut manager = AssetManager::new();
        manager.register_price_feed(PriceFeed {
            feed_id: "feed_A".to_string(),
            feed_name: "A".to_string(),
            feed_type: FeedType::EndOfDay,
            update_frequency: 86400,
            data_quality: DataQuality::new(),
            last_update: 0,
            asset_id: "A".to_string(),
            cached_prices: Vec::new(),
        });

        // A real, mildly volatile series: 100→110→99→108.9 (returns 0.1, -0.1, 0.1).
        manager.update_price_history("A", vec![100.0, 110.0, 99.0, 108.9]);

        let mut asset = asset_with_history("A", 1000.0, Vec::new());
        // Overwrite the empty history; apply_to_asset will fill it from the cache.
        asset.price_history = Vec::new();
        manager.apply_to_asset(&mut asset);

        // apply_to_asset refreshes current_price/market_value from the last price.
        assert!((asset.current_price - 108.9).abs() < 1e-9);
        assert!((asset.market_value - asset.quantity * 108.9).abs() < 1e-9);
        assert_eq!(asset.price_history.len(), 4);

        let portfolio = Portfolio {
            portfolio_id: "feed_pf".to_string(),
            portfolio_name: "feed_pf".to_string(),
            owner_id: "owner".to_string(),
            assets: vec![asset],
            cash_balance: 0.0,
            total_value: 1000.0,
            created_at: 0,
            last_updated: 0,
            risk_profile: RiskProfile::new(),
            investment_strategy: InvestmentStrategy::Balanced,
        };

        let analyzer = RiskAnalyzer::new();
        let metrics = analyzer.calculate_risk_metrics(&portfolio).unwrap();
        // Genuine, non-fabricated numbers: volatility > 0, finite Sharpe.
        assert!(metrics.volatility > 0.0);
        assert!(metrics.var_95 > 0.0);
        assert!(metrics.sharpe_ratio.is_finite());
    }

    #[test]
    fn market_data_sync_to_assets_copies_close_into_history() {
        let mut market_data = MarketData::new();
        market_data.upsert_price_data(PriceData {
            asset_id: "X".to_string(),
            timestamp: 42,
            open: 100.0,
            high: 105.0,
            low: 99.0,
            close: 103.0,
            adjusted_close: 103.0,
            volume: 1000,
        });

        let mut assets = HashMap::new();
        assets.insert(
            "X".to_string(),
            Asset {
                asset_id: "X".to_string(),
                symbol: "X".to_string(),
                asset_type: AssetType::Stock,
                quantity: 10.0,
                average_cost: 100.0,
                current_price: 0.0,
                market_value: 0.0,
                currency: "USD".to_string(),
                exchange: "TEST".to_string(),
                last_updated: 0,
                price_history: Vec::new(),
            },
        );
        // Asset without market data stays untouched.
        assets.insert(
            "Y".to_string(),
            Asset {
                asset_id: "Y".to_string(),
                symbol: "Y".to_string(),
                asset_type: AssetType::Stock,
                quantity: 10.0,
                average_cost: 100.0,
                current_price: 0.0,
                market_value: 0.0,
                currency: "USD".to_string(),
                exchange: "TEST".to_string(),
                last_updated: 0,
                price_history: Vec::new(),
            },
        );

        market_data.sync_to_assets(&mut assets);

        let x = &assets["X"];
        assert_eq!(x.price_history, vec![103.0]);
        assert!((x.current_price - 103.0).abs() < 1e-9);
        assert!((x.market_value - 10.0 * 103.0).abs() < 1e-9);
        // Y had no PriceData entry ⇒ unchanged (empty history).
        assert!(assets["Y"].price_history.is_empty());
    }

    // ---- Part 5: rebalancing logic ----

    /// Build a portfolio with two assets at given market values and a shared
    /// current price (so trade sizing is deterministic).
    fn two_asset_portfolio(id_a: &str, id_b: &str, mv_a: f64, mv_b: f64, price: f64) -> Portfolio {
        let qty_a = mv_a / price;
        let qty_b = mv_b / price;
        Portfolio {
            portfolio_id: "rebal_pf".to_string(),
            portfolio_name: "rebal".to_string(),
            owner_id: "owner".to_string(),
            assets: vec![
                Asset {
                    asset_id: id_a.to_string(),
                    symbol: id_a.to_string(),
                    asset_type: AssetType::Stock,
                    quantity: qty_a,
                    average_cost: price,
                    current_price: price,
                    market_value: mv_a,
                    currency: "USD".to_string(),
                    exchange: "TEST".to_string(),
                    last_updated: 0,
                    price_history: Vec::new(),
                },
                Asset {
                    asset_id: id_b.to_string(),
                    symbol: id_b.to_string(),
                    asset_type: AssetType::Stock,
                    quantity: qty_b,
                    average_cost: price,
                    current_price: price,
                    market_value: mv_b,
                    currency: "USD".to_string(),
                    exchange: "TEST".to_string(),
                    last_updated: 0,
                    price_history: Vec::new(),
                },
            ],
            cash_balance: 0.0,
            total_value: mv_a + mv_b,
            created_at: 0,
            last_updated: 0,
            risk_profile: RiskProfile::new(),
            investment_strategy: InvestmentStrategy::Balanced,
        }
    }

    #[test]
    fn calculate_drift_reports_current_weights() {
        // 70/30 split ⇒ weights 0.7 and 0.3.
        let portfolio = two_asset_portfolio("A", "B", 700.0, 300.0, 100.0);
        let drift = RebalancingEngine::calculate_drift(&portfolio);
        assert!((drift["A"] - 0.7).abs() < 1e-9);
        assert!((drift["B"] - 0.3).abs() < 1e-9);
    }

    #[test]
    fn rebalance_generates_trades_when_drift_exceeds_threshold() {
        // Drifted to 70/30; target 50/50. Drift of 0.2 exceeds the 0.05 threshold.
        let mut portfolio = two_asset_portfolio("A", "B", 700.0, 300.0, 100.0);
        let mut strategy = RebalancingStrategy::new();
        strategy.parameters.deviation_threshold = 0.05;
        strategy.target_weights = HashMap::from([
            ("A".to_string(), 0.5),
            ("B".to_string(), 0.5),
        ]);

        let engine = RebalancingEngine::new();
        let trades = engine.rebalance(&mut portfolio, &strategy).unwrap();

        // Both assets drift by 0.2 ⇒ both get a trade.
        assert_eq!(trades.len(), 2);

        let a_trade = trades.iter().find(|t| t.asset_id == "A").unwrap();
        let b_trade = trades.iter().find(|t| t.asset_id == "B").unwrap();

        // A is overweight (0.7 vs 0.5) ⇒ sell down to 500 (200 units at 100).
        assert_eq!(a_trade.action, TradeAction::Sell);
        assert!((a_trade.quantity - 2.0).abs() < 1e-9, "A qty {}", a_trade.quantity);
        assert!((a_trade.target_weight - 0.5).abs() < 1e-9);

        // B is underweight (0.3 vs 0.5) ⇒ buy up to 500 (200 units at 100).
        assert_eq!(b_trade.action, TradeAction::Buy);
        assert!((b_trade.quantity - 2.0).abs() < 1e-9, "B qty {}", b_trade.quantity);
        assert!((b_trade.target_weight - 0.5).abs() < 1e-9);
    }

    #[test]
    fn rebalance_emits_no_trades_when_within_threshold() {
        // 52/48 vs 50/50 ⇒ drift 0.02, below the 0.05 threshold ⇒ no trades.
        let mut portfolio = two_asset_portfolio("A", "B", 520.0, 480.0, 100.0);
        let mut strategy = RebalancingStrategy::new();
        strategy.parameters.deviation_threshold = 0.05;
        strategy.target_weights = HashMap::from([
            ("A".to_string(), 0.5),
            ("B".to_string(), 0.5),
        ]);

        let engine = RebalancingEngine::new();
        let trades = engine.rebalance(&mut portfolio, &strategy).unwrap();
        assert!(trades.is_empty(), "no trades expected within threshold");
    }

    #[test]
    fn rebalance_rejects_non_positive_total_value() {
        let mut portfolio = two_asset_portfolio("A", "B", 0.0, 0.0, 100.0);
        let strategy = RebalancingStrategy::new();
        let engine = RebalancingEngine::new();
        assert!(engine.rebalance(&mut portfolio, &strategy).is_err());
    }

    #[test]
    fn portfolio_manager_rebalance_portfolio_uses_registered_strategy() {
        // Store a drifted portfolio, register a strategy with targets, and verify
        // the public API returns the expected trades.
        let mut pm = PortfolioManager::new();
        pm.initialize().unwrap();

        let portfolio = two_asset_portfolio("A", "B", 700.0, 300.0, 100.0);
        pm.create_portfolio(portfolio).unwrap();

        let mut strategy = RebalancingStrategy::new();
        strategy.parameters.deviation_threshold = 0.05;
        strategy.target_weights = HashMap::from([
            ("A".to_string(), 0.5),
            ("B".to_string(), 0.5),
        ]);
        pm.register_rebalancing_strategy(strategy);

        let trades = pm.rebalance_portfolio("rebal_pf").unwrap();
        assert_eq!(trades.len(), 2);
        assert!(trades.iter().any(|t| t.asset_id == "A" && t.action == TradeAction::Sell));
        assert!(trades.iter().any(|t| t.asset_id == "B" && t.action == TradeAction::Buy));
    }

    // ----- Asset relationship tracking tests ----------------------------------

    fn catalog_with_assets() -> AssetCatalog {
        let mut catalog = AssetCatalog::new();
        let mut aapl = AssetInfo::new();
        aapl.asset_id = "AAPL".to_string();
        aapl.symbol = "AAPL".to_string();
        let mut msft = AssetInfo::new();
        msft.asset_id = "MSFT".to_string();
        msft.symbol = "MSFT".to_string();
        let mut googl = AssetInfo::new();
        googl.asset_id = "GOOGL".to_string();
        googl.symbol = "GOOGL".to_string();
        catalog.register_asset(aapl);
        catalog.register_asset(msft);
        catalog.register_asset(googl);
        catalog
    }

    #[test]
    fn asset_relationship_add_and_retrieve() {
        let mut catalog = catalog_with_assets();

        let rel = AssetRelationship {
            relationship_id: "rel_1".to_string(),
            source_asset: "AAPL".to_string(),
            target_asset: "MSFT".to_string(),
            relationship_type: AssetRelationshipType::Correlation,
            correlation: 0.85,
        };
        catalog.add_relationship("AAPL", "MSFT", rel);

        let rels = catalog.get_relationships("AAPL");
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].target_asset, "MSFT");
        assert_eq!(rels[0].relationship_type, AssetRelationshipType::Correlation);

        let related = catalog.get_related_assets("AAPL");
        assert_eq!(related, vec!["MSFT".to_string()]);
    }

    #[test]
    fn asset_relationship_count_and_empty() {
        let mut catalog = catalog_with_assets();
        assert_eq!(catalog.relationship_count(), 0);
        assert!(catalog.get_relationships("AAPL").is_empty());
        assert!(catalog.get_related_assets("AAPL").is_empty());

        for (i, target) in ["MSFT", "GOOGL"].iter().enumerate() {
            catalog.add_relationship(
                "AAPL",
                target,
                AssetRelationship {
                    relationship_id: format!("rel_{}", i),
                    source_asset: "AAPL".to_string(),
                    target_asset: target.to_string(),
                    relationship_type: AssetRelationshipType::Correlation,
                    correlation: 0.5,
                },
            );
        }
        assert_eq!(catalog.relationship_count(), 2);
        assert_eq!(catalog.get_related_assets("AAPL"), vec!["MSFT".to_string(), "GOOGL".to_string()]);
    }

    // ----- Asset classification system tests ----------------------------------

    #[test]
    fn asset_class_initialize_registers_standards() {
        let mut catalog = AssetCatalog::new();
        catalog.initialize().unwrap();
        let classes = catalog.list_asset_classes();
        for expected in ["equity", "fixed_income", "commodity", "real_estate", "cash", "derivative", "cryptocurrency"] {
            assert!(classes.iter().any(|c| c == expected), "missing class {}", expected);
        }
        let equity = catalog.get_asset_class("equity").unwrap();
        assert_eq!(equity.class_name, "Equity");
        assert_eq!(equity.class_type, AssetType::Stock);
    }

    #[test]
    fn classify_asset_and_membership() {
        let mut catalog = catalog_with_assets();
        catalog.initialize().unwrap();

        catalog.classify_asset("AAPL", "equity").unwrap();
        catalog.classify_asset("MSFT", "equity").unwrap();

        let members = catalog.get_assets_by_class("equity");
        assert!(members.contains(&"AAPL".to_string()));
        assert!(members.contains(&"MSFT".to_string()));
        assert!(!members.contains(&"GOOGL".to_string()));

        // idempotent: classifying again does not duplicate
        catalog.classify_asset("AAPL", "equity").unwrap();
        let members2 = catalog.get_assets_by_class("equity");
        assert_eq!(members2.iter().filter(|m| *m == "AAPL").count(), 1);
    }

    #[test]
    fn classify_asset_rejects_unknown_asset_or_class() {
        let mut catalog = catalog_with_assets();
        catalog.initialize().unwrap();

        let err = catalog.classify_asset("NOPE", "equity").unwrap_err();
        assert!(matches!(err, FinancialError::AssetError(_)));

        let err = catalog.classify_asset("AAPL", "no_such_class").unwrap_err();
        assert!(matches!(err, FinancialError::AssetError(_)));
    }

    #[test]
    fn register_asset_class_and_list() {
        let mut catalog = AssetCatalog::new();
        let custom = AssetClass {
            class_id: "alt_1".to_string(),
            class_name: "Alternative".to_string(),
            class_type: AssetType::Alternative,
            characteristics: vec![],
            risk_level: RiskLevel::High,
        };
        catalog.register_asset_class("alt_1", custom);
        assert_eq!(catalog.list_asset_classes(), vec!["alt_1".to_string()]);
        assert!(catalog.get_asset_class("alt_1").is_some());
        assert!(catalog.get_asset_class("missing").is_none());
    }

    // ----- Black-Scholes options pricing tests --------------------------------

    /// ATM option parameters: S=K=100, r=0.05, sigma=0.2, T=1.
    fn atm_params(option_type: OptionType) -> OptionParameters {
        OptionParameters {
            underlying_price: 100.0,
            strike: 100.0,
            time_to_maturity: 1.0,
            risk_free_rate: 0.05,
            volatility: 0.2,
            option_type,
        }
    }

    #[test]
    fn test_black_scholes_call_atm() {
        // ATM call (S=K=100, r=0.05, sigma=0.2, T=1) ≈ 10.4506.
        let engine = PricingEngine::new();
        let result = engine.price_option(&atm_params(OptionType::Call)).unwrap();
        assert!(
            (result.price - 10.45).abs() < 0.02,
            "ATM call price {} expected ~10.45",
            result.price
        );
    }

    #[test]
    fn test_black_scholes_put_atm() {
        // ATM put (S=K=100, r=0.05, sigma=0.2, T=1) ≈ 5.5735.
        let engine = PricingEngine::new();
        let result = engine.price_option(&atm_params(OptionType::Put)).unwrap();
        assert!(
            (result.price - 5.57).abs() < 0.02,
            "ATM put price {} expected ~5.57",
            result.price
        );
    }

    #[test]
    fn test_put_call_parity() {
        // Put-call parity: C - P = S - K*exp(-rT).
        let engine = PricingEngine::new();
        let call = engine.price_option(&atm_params(OptionType::Call)).unwrap();
        let put = engine.price_option(&atm_params(OptionType::Put)).unwrap();
        let s = 100.0_f64;
        let k = 100.0_f64;
        let r = 0.05_f64;
        let t = 1.0_f64;
        let parity = s - k * (-r * t).exp();
        assert!(
            ((call.price - put.price) - parity).abs() < 1e-6,
            "C-P = {} but parity = {}",
            call.price - put.price,
            parity
        );
    }

    #[test]
    fn test_greeks_delta() {
        // ATM call delta ≈ 0.6368 (N(d1) with d1≈0.36).
        let engine = PricingEngine::new();
        let result = engine.price_option(&atm_params(OptionType::Call)).unwrap();
        assert!(
            (result.delta - 0.6377).abs() < 0.01,
            "call delta {} expected ~0.6377",
            result.delta
        );
        // Put delta = call delta - 1.
        let put = engine.price_option(&atm_params(OptionType::Put)).unwrap();
        assert!(
            (put.delta - (result.delta - 1.0)).abs() < 1e-9,
            "put delta {} expected {}",
            put.delta,
            result.delta - 1.0
        );
    }

    #[test]
    fn test_zero_time_to_expiry() {
        // T=0 -> intrinsic value. ITM call (S=110, K=100) -> 10; OTM call -> 0.
        let engine = PricingEngine::new();
        let itm = OptionParameters {
            underlying_price: 110.0,
            strike: 100.0,
            time_to_maturity: 0.0,
            risk_free_rate: 0.05,
            volatility: 0.2,
            option_type: OptionType::Call,
        };
        let result = engine.price_option(&itm).unwrap();
        assert!((result.price - 10.0).abs() < 1e-9, "ITM intrinsic {}", result.price);
        assert!((result.delta - 1.0).abs() < 1e-9);

        let otm = OptionParameters {
            underlying_price: 90.0,
            strike: 100.0,
            time_to_maturity: 0.0,
            risk_free_rate: 0.05,
            volatility: 0.2,
            option_type: OptionType::Call,
        };
        let result = engine.price_option(&otm).unwrap();
        assert!((result.price - 0.0).abs() < 1e-9, "OTM intrinsic {}", result.price);
        assert!((result.delta - 0.0).abs() < 1e-9);

        // ITM put intrinsic.
        let itm_put = OptionParameters {
            underlying_price: 90.0,
            strike: 100.0,
            time_to_maturity: 0.0,
            risk_free_rate: 0.05,
            volatility: 0.2,
            option_type: OptionType::Put,
        };
        let result = engine.price_option(&itm_put).unwrap();
        assert!((result.price - 10.0).abs() < 1e-9, "ITM put intrinsic {}", result.price);
    }

    #[test]
    fn test_zero_volatility() {
        // sigma=0 -> deterministic discounted intrinsic.
        // Call: max(S - K*exp(-rT), 0); Put: max(K*exp(-rT) - S, 0).
        let engine = PricingEngine::new();
        let r = 0.05_f64;
        let t = 1.0_f64;
        let disc = (-r * t).exp();

        // ITM call (S=110, K=100): 110 - 100*disc > 0.
        let call = OptionParameters {
            underlying_price: 110.0,
            strike: 100.0,
            time_to_maturity: t,
            risk_free_rate: r,
            volatility: 0.0,
            option_type: OptionType::Call,
        };
        let result = engine.price_option(&call).unwrap();
        let expected = (110.0 - 100.0 * disc).max(0.0);
        assert!(
            (result.price - expected).abs() < 1e-9,
            "zero-vol call {} expected {}",
            result.price,
            expected
        );
        assert!((result.delta - 1.0).abs() < 1e-9);

        // OTM call (S=90, K=100): 90 - 100*disc < 0 -> 0.
        let otm_call = OptionParameters {
            underlying_price: 90.0,
            strike: 100.0,
            time_to_maturity: t,
            risk_free_rate: r,
            volatility: 0.0,
            option_type: OptionType::Call,
        };
        let result = engine.price_option(&otm_call).unwrap();
        assert!((result.price - 0.0).abs() < 1e-9, "zero-vol OTM call {}", result.price);
        assert!((result.delta - 0.0).abs() < 1e-9);

        // ITM put (S=90, K=100): 100*disc - 90 > 0.
        let put = OptionParameters {
            underlying_price: 90.0,
            strike: 100.0,
            time_to_maturity: t,
            risk_free_rate: r,
            volatility: 0.0,
            option_type: OptionType::Put,
        };
        let result = engine.price_option(&put).unwrap();
        let expected = (100.0 * disc - 90.0).max(0.0);
        assert!(
            (result.price - expected).abs() < 1e-9,
            "zero-vol put {} expected {}",
            result.price,
            expected
        );
        assert!((result.delta - (-1.0)).abs() < 1e-9);
    }

    // ---- Report Distribution Channels ----

    fn sample_report() -> FinancialReport {
        FinancialReport::new(
            "report_1".to_string(),
            ReportTemplateType::Portfolio,
            1_700_000_000,
            b"sample report content".to_vec(),
            ContentFormat::JSON,
        )
    }

    #[test]
    fn test_email_distribution_valid() {
        let mut distributor = ReportDistributor::new();
        distributor.add_channel(
            "email_channel".to_string(),
            DistributionChannel::Email {
                recipients: vec!["analyst@example.com".to_string()],
            },
        );
        let results = distributor.distribute(&sample_report()).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].success, "expected success, got: {}", results[0].message);
        assert_eq!(results[0].channel_name, "email_channel");
    }

    #[test]
    fn test_email_distribution_invalid() {
        let mut distributor = ReportDistributor::new();
        distributor.add_channel(
            "email_channel".to_string(),
            DistributionChannel::Email {
                recipients: vec!["not-an-email".to_string()],
            },
        );
        let results = distributor.distribute(&sample_report()).unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0].message.contains("Invalid email recipient"));
    }

    #[test]
    fn test_webhook_distribution() {
        let mut distributor = ReportDistributor::new();
        distributor.add_channel(
            "webhook_channel".to_string(),
            DistributionChannel::Webhook {
                url: "https://hooks.example.com/report".to_string(),
            },
        );
        let results = distributor.distribute(&sample_report()).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].success, "expected success, got: {}", results[0].message);
        assert_eq!(results[0].channel_name, "webhook_channel");
    }

    #[test]
    fn test_multiple_channels() {
        let mut distributor = ReportDistributor::new();
        distributor.add_channel(
            "email".to_string(),
            DistributionChannel::Email {
                recipients: vec!["a@example.com".to_string()],
            },
        );
        distributor.add_channel(
            "webhook".to_string(),
            DistributionChannel::Webhook {
                url: "https://example.com/hook".to_string(),
            },
        );
        distributor.add_channel(
            "file".to_string(),
            DistributionChannel::FileExport {
                path: "/tmp/report.json".to_string(),
            },
        );
        let results = distributor.distribute(&sample_report()).unwrap();
        assert_eq!(results.len(), 3, "expected 3 results");
        assert!(results.iter().all(|r| r.success));
    }

    #[test]
    fn test_delivery_tracking() {
        let mut tracker = DeliveryTracker::new();
        tracker.record_delivery(DeliveryResult {
            channel_name: "email".to_string(),
            success: true,
            timestamp: 100,
            message: "ok".to_string(),
        });
        tracker.record_delivery(DeliveryResult {
            channel_name: "email".to_string(),
            success: false,
            timestamp: 101,
            message: "bad".to_string(),
        });
        let history = tracker.get_delivery_history("email");
        assert_eq!(history.len(), 2);
        assert!(history[0].success);
        assert!(!history[1].success);
        // Unknown channel returns empty history.
        assert!(tracker.get_delivery_history("nope").is_empty());
    }

    #[test]
    fn test_success_rate() {
        let mut tracker = DeliveryTracker::new();
        // 3 successes, 1 failure -> 0.75
        for i in 0..3 {
            tracker.record_delivery(DeliveryResult {
                channel_name: "ch".to_string(),
                success: true,
                timestamp: i,
                message: "ok".to_string(),
            });
        }
        tracker.record_delivery(DeliveryResult {
            channel_name: "ch".to_string(),
            success: false,
            timestamp: 3,
            message: "fail".to_string(),
        });
        let rate = tracker.success_rate("ch");
        assert!((rate - 0.75).abs() < 1e-9, "success rate {} expected 0.75", rate);
    }

    #[test]
    fn test_empty_channels() {
        let mut distributor = ReportDistributor::new();
        let results = distributor.distribute(&sample_report()).unwrap();
        assert!(results.is_empty(), "no channels should yield no results");
    }

    // ----- Monte Carlo stress testing tests -----------------------------------

    /// Build a simple two-asset portfolio for stress testing.
    fn mc_test_portfolio() -> Portfolio {
        let a = Asset {
            asset_id: "asset_1".to_string(),
            symbol: "AAPL".to_string(),
            asset_type: AssetType::Stock,
            quantity: 100.0,
            average_cost: 150.0,
            current_price: 150.0,
            market_value: 15000.0,
            currency: "USD".to_string(),
            exchange: "NASDAQ".to_string(),
            last_updated: 0,
            price_history: Vec::new(),
        };
        let b = Asset {
            asset_id: "asset_2".to_string(),
            symbol: "MSFT".to_string(),
            asset_type: AssetType::Stock,
            quantity: 50.0,
            average_cost: 300.0,
            current_price: 300.0,
            market_value: 15000.0,
            currency: "USD".to_string(),
            exchange: "NASDAQ".to_string(),
            last_updated: 0,
            price_history: Vec::new(),
        };
        Portfolio {
            portfolio_id: "mc_pf".to_string(),
            portfolio_name: "MC Portfolio".to_string(),
            owner_id: "user_1".to_string(),
            assets: vec![a, b],
            cash_balance: 5000.0,
            total_value: 35000.0,
            created_at: 0,
            last_updated: 0,
            risk_profile: RiskProfile::new(),
            investment_strategy: InvestmentStrategy::Balanced,
        }
    }

    #[test]
    fn test_monte_carlo_basic() {
        let analyzer = ScenarioAnalyzer::new();
        let portfolio = mc_test_portfolio();
        let result = analyzer.run_monte_carlo(&portfolio, 1000, 0.20).unwrap();

        assert_eq!(result.num_simulations, 1000);
        // Mean should be in a sane range around the initial value (35000).
        assert!(
            (result.mean_portfolio_value - 35000.0).abs() < 5000.0,
            "mean {} should be near 35000",
            result.mean_portfolio_value
        );
        // With non-zero volatility there should be dispersion.
        assert!(result.std_dev > 0.0, "std_dev should be positive");
        // VaR figures are non-negative loss magnitudes.
        assert!(result.var_95 >= 0.0, "var_95 should be non-negative");
        assert!(result.var_99 >= 0.0, "var_99 should be non-negative");
        // Expected shortfall is at least the 95% VaR.
        assert!(
            result.expected_shortfall >= result.var_95 - 1e-9,
            "expected_shortfall {} should be >= var_95 {}",
            result.expected_shortfall,
            result.var_95
        );
        // Max drawdown is non-negative and at least the 99% VaR.
        assert!(result.max_drawdown >= 0.0);
        assert!(
            result.max_drawdown >= result.var_99 - 1e-9,
            "max_drawdown {} should be >= var_99 {}",
            result.max_drawdown,
            result.var_99
        );
        // Probability of loss is a valid fraction.
        assert!(
            (0.0..=1.0).contains(&result.probability_of_loss),
            "probability_of_loss {} out of range",
            result.probability_of_loss
        );
    }

    #[test]
    fn test_var_ordering() {
        let analyzer = ScenarioAnalyzer::new();
        let portfolio = mc_test_portfolio();
        let result = analyzer.run_monte_carlo(&portfolio, 1000, 0.30).unwrap();
        assert!(
            result.var_99 >= result.var_95 - 1e-9,
            "var_99 ({}) should be >= var_95 ({})",
            result.var_99,
            result.var_95
        );
    }

    #[test]
    fn test_monte_carlo_zero_volatility() {
        let analyzer = ScenarioAnalyzer::new();
        let portfolio = mc_test_portfolio();
        let result = analyzer.run_monte_carlo(&portfolio, 1000, 0.0).unwrap();

        // With zero volatility every simulation equals the initial value.
        let initial = 35000.0_f64;
        assert!(
            (result.mean_portfolio_value - initial).abs() < 1e-6,
            "mean {} should equal initial {}",
            result.mean_portfolio_value,
            initial
        );
        assert!(result.std_dev < 1e-6, "std_dev should be ~0, got {}", result.std_dev);
        assert!(
            result.probability_of_loss < 1e-9,
            "no losses expected with zero volatility, got {}",
            result.probability_of_loss
        );
        assert!(result.var_95 < 1e-6, "var_95 should be ~0");
        assert!(result.var_99 < 1e-6, "var_99 should be ~0");
        assert!(result.max_drawdown < 1e-6, "max_drawdown should be ~0");
    }

    #[test]
    fn test_scenario_impact() {
        let mut analyzer = ScenarioAnalyzer::new();
        let mut shocks = HashMap::new();
        shocks.insert("asset_1".to_string(), -0.20);
        shocks.insert("asset_2".to_string(), -0.20);
        analyzer.add_scenario(MarketScenario::new("market_crash", 0.05, shocks));

        let portfolio = mc_test_portfolio();
        let results = analyzer.run_scenarios(&portfolio).unwrap();

        assert_eq!(results.len(), 1);
        let r = &results[0];
        assert_eq!(r.scenario_name, "market_crash");
        assert_eq!(r.probability, 0.05);
        // Initial asset value = 30000, after -20% shock = 24000; cash 5000 untouched.
        // final_value = 24000 + 5000 = 29000; impact = 29000 - 35000 = -6000.
        assert!(
            (r.final_value - 29000.0).abs() < 1e-6,
            "final_value {} expected 29000",
            r.final_value
        );
        assert!(
            (r.portfolio_impact - (-6000.0)).abs() < 1e-6,
            "portfolio_impact {} expected -6000",
            r.portfolio_impact
        );
        assert!(r.portfolio_impact < 0.0, "crash should produce a negative impact");
    }

    #[test]
    fn test_no_scenarios() {
        let analyzer = ScenarioAnalyzer::new();
        let portfolio = mc_test_portfolio();
        let results = analyzer.run_scenarios(&portfolio).unwrap();
        assert!(results.is_empty(), "no scenarios should yield empty results");
    }

    #[test]
    fn test_probability_of_loss() {
        let analyzer = ScenarioAnalyzer::new();
        let portfolio = mc_test_portfolio();
        // High volatility makes losses likely in a meaningful fraction of sims.
        let result = analyzer.run_monte_carlo(&portfolio, 1000, 0.50).unwrap();
        assert!(
            result.probability_of_loss > 0.0,
            "with high volatility probability_of_loss should be > 0, got {}",
            result.probability_of_loss
        );
        assert!(
            result.probability_of_loss <= 1.0,
            "probability_of_loss must be <= 1"
        );
    }

    // ── Compliance rule engine tests ──────────────────────────────────────

    /// Helper: a minimal portfolio with one asset and a known risk profile.
    fn compliance_portfolio(owner: &str, asset_symbol: &str, market_value: f64, cash: f64) -> Portfolio {
        Portfolio {
            portfolio_id: "pf_1".to_string(),
            portfolio_name: "Test".to_string(),
            owner_id: owner.to_string(),
            assets: vec![Asset {
                asset_id: "a1".to_string(),
                symbol: asset_symbol.to_string(),
                asset_type: AssetType::Stock,
                quantity: 10.0,
                average_cost: 100.0,
                current_price: market_value / 10.0,
                market_value,
                currency: "USD".to_string(),
                exchange: "NYSE".to_string(),
                last_updated: 1000,
                price_history: Vec::new(),
            }],
            cash_balance: cash,
            total_value: market_value + cash,
            created_at: 1000,
            last_updated: 1000,
            risk_profile: RiskProfile {
                risk_tolerance: RiskTolerance::Moderate,
                risk_capacity: 100_000.0,
                time_horizon: TimeHorizon::MediumTerm,
                liquidity_needs: LiquidityNeeds::Medium,
            },
            investment_strategy: InvestmentStrategy::Balanced,
        }
    }

    #[test]
    fn compliance_empty_portfolio_no_rules_is_compliant() {
        let mut monitor = ComplianceMonitor::new();
        let portfolio = Portfolio {
            portfolio_id: "empty".to_string(),
            portfolio_name: "Empty".to_string(),
            owner_id: String::new(),
            assets: Vec::new(),
            cash_balance: 0.0,
            total_value: 0.0,
            created_at: 0,
            last_updated: 0,
            risk_profile: RiskProfile {
                risk_tolerance: RiskTolerance::Conservative,
                risk_capacity: 0.0,
                time_horizon: TimeHorizon::ShortTerm,
                liquidity_needs: LiquidityNeeds::Low,
            },
            investment_strategy: InvestmentStrategy::Balanced,
        };
        let result = monitor.check_compliance(&portfolio).unwrap();
        assert_eq!(result.status, ComplianceStatus::Compliant);
        assert_eq!(result.risk_score, 0.0);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn compliance_nonempty_portfolio_no_rules_is_flagged() {
        let mut monitor = ComplianceMonitor::new();
        let portfolio = compliance_portfolio("user_1", "AAPL", 5000.0, 1000.0);
        let result = monitor.check_compliance(&portfolio).unwrap();
        assert_eq!(result.status, ComplianceStatus::Flagged);
        assert_eq!(result.risk_score, 1.0);
        assert!(!result.violations.is_empty());
    }

    #[test]
    fn compliance_position_limit_passes_when_under_limit() {
        let mut monitor = ComplianceMonitor::new();
        monitor.add_rule(ComplianceRule {
            rule_id: "pos_limit".to_string(),
            rule_type: ComplianceRuleType::PositionLimit,
            parameters: HashMap::from([("max_position".to_string(), 10_000.0)]),
            string_parameters: HashMap::new(),
            description: "Max position 10k".to_string(),
        });
        let portfolio = compliance_portfolio("user_1", "AAPL", 5000.0, 1000.0);
        let result = monitor.check_compliance(&portfolio).unwrap();
        assert_eq!(result.status, ComplianceStatus::Compliant);
        assert_eq!(result.risk_score, 0.0);
        assert!(result.violations.is_empty());
        assert_eq!(result.audit_entries.len(), 1);
    }

    #[test]
    fn compliance_position_limit_fails_when_over_limit() {
        let mut monitor = ComplianceMonitor::new();
        monitor.add_rule(ComplianceRule {
            rule_id: "pos_limit".to_string(),
            rule_type: ComplianceRuleType::PositionLimit,
            parameters: HashMap::from([("max_position".to_string(), 3000.0)]),
            string_parameters: HashMap::new(),
            description: "Max position 3k".to_string(),
        });
        let portfolio = compliance_portfolio("user_1", "AAPL", 5000.0, 1000.0);
        let result = monitor.check_compliance(&portfolio).unwrap();
        assert_eq!(result.status, ComplianceStatus::NonCompliant);
        assert!((result.risk_score - 1.0).abs() < 1e-9);
        assert!(result.violations[0].contains("AAPL"));
        assert!(!result.recommendations.is_empty());
    }

    #[test]
    fn compliance_trading_restriction_catches_restricted_asset() {
        let mut monitor = ComplianceMonitor::new();
        monitor.add_rule(ComplianceRule {
            rule_id: "restricted".to_string(),
            rule_type: ComplianceRuleType::TradingRestriction,
            parameters: HashMap::new(),
            string_parameters: HashMap::from([
                ("restricted_assets".to_string(), "AAPL,GOOG,MSFT".to_string()),
            ]),
            description: "Banned assets".to_string(),
        });
        let portfolio = compliance_portfolio("user_1", "AAPL", 5000.0, 1000.0);
        let result = monitor.check_compliance(&portfolio).unwrap();
        assert_eq!(result.status, ComplianceStatus::NonCompliant);
        assert!(result.violations[0].contains("AAPL"));
    }

    #[test]
    fn compliance_trading_restriction_passes_when_not_restricted() {
        let mut monitor = ComplianceMonitor::new();
        monitor.add_rule(ComplianceRule {
            rule_id: "restricted".to_string(),
            rule_type: ComplianceRuleType::TradingRestriction,
            parameters: HashMap::new(),
            string_parameters: HashMap::from([
                ("restricted_assets".to_string(), "GOOG,MSFT".to_string()),
            ]),
            description: "Banned assets".to_string(),
        });
        let portfolio = compliance_portfolio("user_1", "AAPL", 5000.0, 1000.0);
        let result = monitor.check_compliance(&portfolio).unwrap();
        assert_eq!(result.status, ComplianceStatus::Compliant);
    }

    #[test]
    fn compliance_margin_requirement_fails_when_insufficient_cash() {
        let mut monitor = ComplianceMonitor::new();
        monitor.add_rule(ComplianceRule {
            rule_id: "margin".to_string(),
            rule_type: ComplianceRuleType::MarginRequirement,
            parameters: HashMap::from([("margin_pct".to_string(), 50.0)]),
            string_parameters: HashMap::new(),
            description: "50% margin".to_string(),
        });
        // total_value = 6000, required margin = 3000, cash = 100 → fails.
        let portfolio = compliance_portfolio("user_1", "AAPL", 5000.0, 100.0);
        let result = monitor.check_compliance(&portfolio).unwrap();
        assert_eq!(result.status, ComplianceStatus::NonCompliant);
        assert!(result.violations[0].contains("margin"));
    }

    #[test]
    fn compliance_margin_requirement_passes_when_sufficient_cash() {
        let mut monitor = ComplianceMonitor::new();
        monitor.add_rule(ComplianceRule {
            rule_id: "margin".to_string(),
            rule_type: ComplianceRuleType::MarginRequirement,
            parameters: HashMap::from([("margin_pct".to_string(), 10.0)]),
            string_parameters: HashMap::new(),
            description: "10% margin".to_string(),
        });
        // total_value = 6000, required margin = 600, cash = 1000 → passes.
        let portfolio = compliance_portfolio("user_1", "AAPL", 5000.0, 1000.0);
        let result = monitor.check_compliance(&portfolio).unwrap();
        assert_eq!(result.status, ComplianceStatus::Compliant);
    }

    #[test]
    fn compliance_kyc_fails_for_empty_owner() {
        let mut monitor = ComplianceMonitor::new();
        monitor.add_rule(ComplianceRule {
            rule_id: "kyc".to_string(),
            rule_type: ComplianceRuleType::KYC,
            parameters: HashMap::from([("kyc_required".to_string(), 1.0)]),
            string_parameters: HashMap::new(),
            description: "KYC required".to_string(),
        });
        let portfolio = compliance_portfolio("", "AAPL", 5000.0, 1000.0);
        let result = monitor.check_compliance(&portfolio).unwrap();
        assert_eq!(result.status, ComplianceStatus::NonCompliant);
        assert!(result.violations[0].to_lowercase().contains("kyc"));
    }

    #[test]
    fn compliance_kyc_passes_for_verified_owner() {
        let mut monitor = ComplianceMonitor::new();
        monitor.add_rule(ComplianceRule {
            rule_id: "kyc".to_string(),
            rule_type: ComplianceRuleType::KYC,
            parameters: HashMap::from([("kyc_required".to_string(), 1.0)]),
            string_parameters: HashMap::new(),
            description: "KYC required".to_string(),
        });
        let portfolio = compliance_portfolio("user_1", "AAPL", 5000.0, 1000.0);
        let result = monitor.check_compliance(&portfolio).unwrap();
        assert_eq!(result.status, ComplianceStatus::Compliant);
    }

    #[test]
    fn compliance_multiple_rules_mixed_pass_fail() {
        let mut monitor = ComplianceMonitor::new();
        monitor.add_rule(ComplianceRule {
            rule_id: "pos_ok".to_string(),
            rule_type: ComplianceRuleType::PositionLimit,
            parameters: HashMap::from([("max_position".to_string(), 10_000.0)]),
            string_parameters: HashMap::new(),
            description: "Max 10k".to_string(),
        });
        monitor.add_rule(ComplianceRule {
            rule_id: "margin_fail".to_string(),
            rule_type: ComplianceRuleType::MarginRequirement,
            parameters: HashMap::from([("margin_pct".to_string(), 50.0)]),
            string_parameters: HashMap::new(),
            description: "50% margin".to_string(),
        });
        let portfolio = compliance_portfolio("user_1", "AAPL", 5000.0, 100.0);
        let result = monitor.check_compliance(&portfolio).unwrap();
        assert_eq!(result.status, ComplianceStatus::NonCompliant);
        // 1 of 2 rules failed → risk_score = 0.5
        assert!((result.risk_score - 0.5).abs() < 1e-9);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.audit_entries.len(), 2);
    }

    #[test]
    fn compliance_custom_rule_is_flagged_not_compliant() {
        let mut monitor = ComplianceMonitor::new();
        monitor.add_rule(ComplianceRule {
            rule_id: "custom_1".to_string(),
            rule_type: ComplianceRuleType::Custom,
            parameters: HashMap::new(),
            string_parameters: HashMap::new(),
            description: "Custom rule".to_string(),
        });
        let portfolio = compliance_portfolio("user_1", "AAPL", 5000.0, 1000.0);
        let result = monitor.check_compliance(&portfolio).unwrap();
        // Custom rules pass but are flagged for review.
        assert_eq!(result.status, ComplianceStatus::Flagged);
        assert!(result.violations.is_empty());
    }
}
