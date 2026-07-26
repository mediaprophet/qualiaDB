use super::*;

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
            FinancialError::DataError(format!("no price feed registered for asset '{}'", asset_id))
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

    pub fn market_data(&self) -> &MarketData {
        &self.market_data
    }

    pub fn market_data_mut(&mut self) -> &mut MarketData {
        &mut self.market_data
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
    pub fn classify_asset(&mut self, asset_id: &str, class_id: &str) -> Result<(), FinancialError> {
        if !self.assets.contains_key(asset_id) {
            return Err(FinancialError::AssetError(format!(
                "asset '{}' is not registered in the catalog",
                asset_id
            )));
        }
        let class = self.asset_classes.get_mut(class_id).ok_or_else(|| {
            FinancialError::AssetError(format!("asset class '{}' is not registered", class_id))
        })?;
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
            (
                "equity",
                "Equity",
                AssetType::Stock,
                RiskLevel::Medium,
                &["Stocks", "Shares"],
            ),
            (
                "fixed_income",
                "Fixed Income",
                AssetType::Bond,
                RiskLevel::Low,
                &["Bonds", "Debt instruments"],
            ),
            (
                "commodity",
                "Commodity",
                AssetType::Commodity,
                RiskLevel::High,
                &["Physical goods", "Futures"],
            ),
            (
                "real_estate",
                "Real Estate",
                AssetType::RealEstate,
                RiskLevel::Medium,
                &["Property", "Land"],
            ),
            (
                "cash",
                "Cash",
                AssetType::Currency,
                RiskLevel::Low,
                &["Currency", "Money market"],
            ),
            (
                "derivative",
                "Derivative",
                AssetType::Derivative,
                RiskLevel::VeryHigh,
                &["Options", "Futures", "Swaps"],
            ),
            (
                "cryptocurrency",
                "Cryptocurrency",
                AssetType::Cryptocurrency,
                RiskLevel::VeryHigh,
                &["Digital assets", "Tokens"],
            ),
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

    pub fn upsert_volume_data(&mut self, data: VolumeData) {
        self.volume_data.insert(data.asset_id.clone(), data);
    }

    pub fn get_volume_data(&self, asset_id: &str) -> Option<&VolumeData> {
        self.volume_data.get(asset_id)
    }

    pub fn upsert_technical_indicators(&mut self, indicators: TechnicalIndicators) {
        self.technical_indicators
            .insert(indicators.asset_id.clone(), indicators);
    }

    pub fn get_technical_indicators(&self, asset_id: &str) -> Option<&TechnicalIndicators> {
        self.technical_indicators.get(asset_id)
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

    pub fn add_validation_rule(&mut self, rule: ValidationRule) {
        self.validation_rules.push(rule);
    }

    pub fn list_validation_rules(&self) -> &[ValidationRule] {
        &self.validation_rules
    }

    pub fn validation_rule_count(&self) -> usize {
        self.validation_rules.len()
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

    pub fn add_compliance_rule(&mut self, rule: ComplianceRule) {
        self.compliance_rules.push(rule);
    }

    pub fn list_compliance_rules(&self) -> &[ComplianceRule] {
        &self.compliance_rules
    }

    pub fn add_regulatory_framework(&mut self, framework: RegulatoryFramework) {
        self.regulatory_frameworks.push(framework);
    }

    pub fn list_regulatory_frameworks(&self) -> &[RegulatoryFramework] {
        &self.regulatory_frameworks
    }

    pub fn add_screening_list(&mut self, list: ScreeningList) {
        self.screening_lists.insert(list.list_id.clone(), list);
    }

    pub fn get_screening_list(&self, list_id: &str) -> Option<&ScreeningList> {
        self.screening_lists.get(list_id)
    }

    pub fn list_screening_lists(&self) -> Vec<String> {
        self.screening_lists.keys().cloned().collect()
    }
}

impl ComplianceRule {
    /// Human-readable name for the rule type, for audit logging.
    pub(super) fn rule_type_as_str(&self) -> &'static str {
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
