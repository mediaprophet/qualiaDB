use super::*;

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
    // Widened to `pub(super)` during the §11 library-ization split: the
    // `financial_modeling::tests` module (formerly a descendant of the single
    // module) reads this field directly and now lives in a sibling module.
    // Behaviour-preserving; the struct's external API is unchanged.
    pub(super) audit_trail: PortfolioAuditTrail,
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

    pub fn add_metadata(&mut self, metadata: PortfolioMetadata) {
        self.portfolio_metadata
            .insert(metadata.portfolio_id.clone(), metadata);
    }

    pub fn get_metadata(&self, portfolio_id: &str) -> Option<&PortfolioMetadata> {
        self.portfolio_metadata.get(portfolio_id)
    }

    pub fn list_metadata(&self) -> Vec<String> {
        self.portfolio_metadata.keys().cloned().collect()
    }

    pub fn access_control(&self) -> &PortfolioAccessControl {
        &self.access_control
    }

    pub fn access_control_mut(&mut self) -> &mut PortfolioAccessControl {
        &mut self.access_control
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
        self.access_policies
            .insert(policy.policy_id.clone(), policy);
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

    pub fn add_authentication_requirement(&mut self, requirement: AuthenticationRequirement) {
        self.authentication_requirements
            .insert(requirement.requirement_id.clone(), requirement);
    }

    pub fn get_authentication_requirement(
        &self,
        requirement_id: &str,
    ) -> Option<&AuthenticationRequirement> {
        self.authentication_requirements.get(requirement_id)
    }

    pub fn list_authentication_requirements(&self) -> Vec<String> {
        self.authentication_requirements.keys().cloned().collect()
    }

    pub fn is_audit_logging_enabled(&self) -> bool {
        self.audit_logging
    }

    pub fn set_audit_logging(&mut self, enabled: bool) {
        self.audit_logging = enabled;
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

    pub fn retention_policy(&self) -> &RetentionPolicy {
        &self.retention_policy
    }

    pub fn set_retention_policy(&mut self, policy: RetentionPolicy) {
        self.retention_policy = policy;
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
