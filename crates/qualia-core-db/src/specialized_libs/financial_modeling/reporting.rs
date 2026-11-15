use super::*;

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

impl ReportingEngine {
    /// Creates an engine with registered report generator and distributor.
    pub fn new() -> Self {
        Self {
            report_templates: HashMap::new(),
            report_generator: ReportGenerator::new(),
            report_distributor: ReportDistributor::new(),
        }
    }

    /// No-op returning `Ok(())` — the in-memory template store needs no setup.
    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.report_generator.initialize()?;
        self.report_distributor.initialize()?;
        Ok(())
    }

    /// Registers `template` by `template_id` (overwrites silently).
    pub fn add_report_template(&mut self, template: ReportTemplate) {
        self.report_templates
            .insert(template.template_id.clone(), template);
    }

    /// Registered template for `template_id`, or `None`.
    pub fn get_report_template(&self, template_id: &str) -> Option<&ReportTemplate> {
        self.report_templates.get(template_id)
    }

    /// All registered template IDs.
    pub fn list_report_templates(&self) -> Vec<String> {
        self.report_templates.keys().cloned().collect()
    }

    /// Shared access to report generator.
    pub fn report_generator(&self) -> &ReportGenerator {
        &self.report_generator
    }

    /// Mutable access to report generator.
    pub fn report_generator_mut(&mut self) -> &mut ReportGenerator {
        &mut self.report_generator
    }

    /// Shared access to report distributor.
    pub fn report_distributor(&self) -> &ReportDistributor {
        &self.report_distributor
    }

    /// Mutable access to report distributor.
    pub fn report_distributor_mut(&mut self) -> &mut ReportDistributor {
        &mut self.report_distributor
    }

    /// Generates a concrete `FinancialReport` by looking up the registered template,
    /// validating the generation strategy (if provided), evaluating all template sections,
    /// aggregating dataset inputs, and packaging the result.
    pub fn generate_report(
        &self,
        template_id: &str,
        strategy_id: Option<&str>,
        data_map: &HashMap<String, Vec<f64>>,
    ) -> Result<FinancialReport, FinancialError> {
        let template = self.report_templates.get(template_id).ok_or_else(|| {
            FinancialError::DataError(format!("Report template '{}' not found", template_id))
        })?;

        let content = self
            .report_generator
            .generate_report_content(strategy_id, template, data_map)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(FinancialReport::new(
            format!("rep_{}_{}", template_id, now),
            template.template_type.clone(),
            now,
            content,
            ContentFormat::JSON,
        ))
    }
}

impl ReportGenerator {
    /// Creates a generator with an empty strategy registry.
    pub fn new() -> Self {
        Self {
            generation_strategies: HashMap::new(),
            data_aggregator: DataAggregator::new(),
        }
    }

    /// No-op returning `Ok(())`.
    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        self.data_aggregator.initialize()?;
        Ok(())
    }

    /// Registers `strategy` by `strategy_id`.
    pub fn add_generation_strategy(&mut self, strategy: GenerationStrategy) {
        self.generation_strategies
            .insert(strategy.strategy_id.clone(), strategy);
    }

    /// Registered strategy for `strategy_id`, or `None`.
    pub fn get_generation_strategy(&self, strategy_id: &str) -> Option<&GenerationStrategy> {
        self.generation_strategies.get(strategy_id)
    }

    /// All registered strategy IDs.
    pub fn list_generation_strategies(&self) -> Vec<String> {
        self.generation_strategies.keys().cloned().collect()
    }

    /// Shared access to data aggregator.
    pub fn data_aggregator(&self) -> &DataAggregator {
        &self.data_aggregator
    }

    /// Mutable access to data aggregator.
    pub fn data_aggregator_mut(&mut self) -> &mut DataAggregator {
        &mut self.data_aggregator
    }

    /// Evaluates template sections and generates report bytes using the registered aggregation rules.
    pub fn generate_report_content(
        &self,
        strategy_id: Option<&str>,
        template: &ReportTemplate,
        data_map: &HashMap<String, Vec<f64>>,
    ) -> Result<Vec<u8>, FinancialError> {
        if let Some(strat_id) = strategy_id {
            if !self.generation_strategies.contains_key(strat_id) {
                return Err(FinancialError::DataError(format!(
                    "Generation strategy '{}' not registered",
                    strat_id
                )));
            }
        }

        let mut section_results: Vec<String> = Vec::new();
        for section in &template.sections {
            let section_data = data_map.get(&section.section_id).cloned().unwrap_or_default();
            let mut metric_str = String::new();
            if !section_data.is_empty() {
                if let Ok(val) = self
                    .data_aggregator
                    .aggregate_data(&section.section_id, &section_data)
                {
                    metric_str = format!(",\"aggregate_metric\":{:.4}", val);
                }
            }
            let sec_json = format!(
                "{{\"section_id\":\"{}\",\"section_name\":\"{}\",\"section_type\":\"{:?}\",\"data_points\":{}{}}}",
                section.section_id,
                section.section_name,
                section.section_type,
                section_data.len(),
                metric_str
            );
            section_results.push(sec_json);
        }

        let report_json = format!(
            "{{\"template_id\":\"{}\",\"template_name\":\"{}\",\"template_type\":\"{:?}\",\"sections\":[{}]}}",
            template.template_id,
            template.template_name,
            template.template_type,
            section_results.join(",")
        );

        Ok(report_json.into_bytes())
    }
}

impl DataAggregator {
    /// Creates an aggregator with empty rule/source registries.
    pub fn new() -> Self {
        Self {
            aggregation_rules: HashMap::new(),
            data_sources: HashMap::new(),
        }
    }

    /// No-op returning `Ok(())`.
    pub fn initialize(&mut self) -> Result<(), FinancialError> {
        Ok(())
    }

    /// Registers `rule` by `rule_id`.
    pub fn add_aggregation_rule(&mut self, rule: AggregationRule) {
        self.aggregation_rules.insert(rule.rule_id.clone(), rule);
    }

    /// Registered rule for `rule_id`, or `None`.
    pub fn get_aggregation_rule(&self, rule_id: &str) -> Option<&AggregationRule> {
        self.aggregation_rules.get(rule_id)
    }

    /// All registered rule IDs.
    pub fn list_aggregation_rules(&self) -> Vec<String> {
        self.aggregation_rules.keys().cloned().collect()
    }

    /// Registers `source` by `source_id`.
    pub fn add_data_source(&mut self, source: DataSource) {
        self.data_sources.insert(source.source_id.clone(), source);
    }

    /// Registered source for `source_id`, or `None`.
    pub fn get_data_source(&self, source_id: &str) -> Option<&DataSource> {
        self.data_sources.get(source_id)
    }

    /// All registered source IDs.
    pub fn list_data_sources(&self) -> Vec<String> {
        self.data_sources.keys().cloned().collect()
    }

    /// Aggregates a slice of numeric data according to a registered rule.
    pub fn aggregate_data(&self, rule_id: &str, data: &[f64]) -> Result<f64, FinancialError> {
        let rule = self.aggregation_rules.get(rule_id).ok_or_else(|| {
            FinancialError::DataError(format!("Aggregation rule '{}' not registered", rule_id))
        })?;

        if data.is_empty() {
            return match rule.rule_type {
                AggregationRuleType::Sum | AggregationRuleType::Count => Ok(0.0),
                AggregationRuleType::Average
                | AggregationRuleType::Min
                | AggregationRuleType::Max => Err(FinancialError::InsufficientData(format!(
                    "Cannot compute {:?} on empty data slice",
                    rule.rule_type
                ))),
            };
        }

        let result = match rule.rule_type {
            AggregationRuleType::Sum => data.iter().sum::<f64>(),
            AggregationRuleType::Average => data.iter().sum::<f64>() / (data.len() as f64),
            AggregationRuleType::Min => data.iter().copied().fold(f64::INFINITY, f64::min),
            AggregationRuleType::Max => data.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            AggregationRuleType::Count => data.len() as f64,
        };

        Ok(result)
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

    /// No-op returning `Ok(())` — channels are configured lazily via `add_channel`.
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
                        message: format!("Email delivered to {} recipient(s)", recipients.len()),
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
    /// Creates a default `DeliveryStatus`.
    pub fn new() -> Self {
        Self {
            total_deliveries: 0,
            successful_deliveries: 0,
            failed_deliveries: 0,
            pending_deliveries: 0,
        }
    }
}
