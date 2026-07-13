use super::*;


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
