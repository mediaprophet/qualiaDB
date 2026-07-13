// Part of the cryptographic_library module (split from the former mod.rs monolith
// per CLAUDE.md §11 — pure code motion, no behaviour change).
use super::*;

/// Security monitor
pub struct SecurityMonitor {
    threat_detector: ThreatDetector,
    anomaly_detector: AnomalyDetector,
    compliance_monitor: ComplianceMonitor,
    security_metrics: SecurityMetrics,
}

/// Threat detector
pub struct ThreatDetector {
    threat_signatures: HashMap<String, ThreatSignature>,
    detection_rules: Vec<DetectionRule>,
    alert_system: SecurityAlertSystem,
}

/// Threat signatures
#[derive(Debug, Clone)]
pub struct ThreatSignature {
    pub signature_id: String,
    pub threat_type: ThreatType,
    pub pattern: Vec<u8>,
    pub severity: ThreatSeverity,
    pub description: String,
}

/// Threat types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThreatType {
    MaliciousKey,
    CompromisedCertificate,
    WeakAlgorithm,
    SideChannelAttack,
    TimingAttack,
    Custom(String),
}

/// Threat severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ThreatSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Detection rules
#[derive(Debug, Clone)]
pub struct DetectionRule {
    pub rule_id: String,
    pub rule_type: DetectionRuleType,
    pub conditions: Vec<DetectionCondition>,
    pub actions: Vec<DetectionAction>,
}

/// Detection rule types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DetectionRuleType {
    Signature,
    Heuristic,
    Behavioral,
    Statistical,
    Custom(String),
}

/// Detection conditions
#[derive(Debug, Clone)]
pub struct DetectionCondition {
    pub condition_id: String,
    pub field: String,
    pub operator: ComparisonOperator,
    pub value: Vec<u8>,
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

/// Detection actions
#[derive(Debug, Clone)]
pub struct DetectionAction {
    pub action_id: String,
    pub action_type: DetectionActionType,
    pub parameters: HashMap<String, Vec<u8>>,
}

/// Detection action types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DetectionActionType {
    Alert,
    Block,
    Quarantine,
    Log,
    Custom(String),
}

/// Escalation policy for security alerts
#[derive(Debug, Clone)]
pub struct EscalationPolicy {
    pub policy_id: String,
    pub trigger_conditions: Vec<String>,
    pub timeout: u64,
}

/// Security alert system
pub struct SecurityAlertSystem {
    alert_types: Vec<SecurityAlertType>,
    notification_channels: Vec<NotificationChannel>,
    escalation_policies: Vec<EscalationPolicy>,
}

/// Security alert types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SecurityAlertType {
    Threat,
    Anomaly,
    Compliance,
    System,
    Custom(String),
}

/// Anomaly detector
pub struct AnomalyDetector {
    detection_algorithms: Vec<AnomalyDetectionAlgorithm>,
    baseline_models: HashMap<String, BaselineModel>,
    alert_thresholds: HashMap<String, f64>,
}

/// Anomaly detection algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnomalyDetectionAlgorithm {
    Statistical,
    MachineLearning,
    DeepLearning,
    Ensemble,
    Custom(String),
}

/// Baseline model
#[derive(Debug, Clone)]
pub struct BaselineModel {
    pub model_id: String,
    pub model_type: ModelType,
    pub parameters: Vec<f64>,
    pub accuracy: f64,
}

/// Model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelType {
    Statistical,
    NeuralNetwork,
    DecisionTree,
    Custom(String),
}

/// Compliance monitor
pub struct ComplianceMonitor {
    compliance_frameworks: HashMap<String, ComplianceFramework>,
    audit_trail: AuditTrail,
    reporting_engine: ComplianceReportingEngine,
}

/// Compliance frameworks
#[derive(Debug, Clone)]
pub struct ComplianceFramework {
    pub framework_id: String,
    pub framework_name: String,
    pub requirements: Vec<ComplianceRequirement>,
    pub controls: Vec<ComplianceControl>,
}

/// Compliance controls
#[derive(Debug, Clone)]
pub struct ComplianceControl {
    pub control_id: String,
    pub control_name: String,
    pub control_type: ControlType,
    pub implementation_status: ImplementationStatus,
}

/// Control types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ControlType {
    Preventive,
    Detective,
    Corrective,
    Compensating,
}

/// Implementation status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImplementationStatus {
    Implemented,
    PartiallyImplemented,
    NotImplemented,
    NotApplicable,
}

/// Audit trail
pub struct AuditTrail {
    entries: Vec<AuditEntry>,
    retention_policy: RetentionPolicy,
}

/// Audit entry
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub entry_id: String,
    pub timestamp: u64,
    pub event_type: EventType,
    pub user_id: String,
    pub resource_id: String,
    pub action: String,
    pub result: AuditResult,
}

/// Event types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventType {
    KeyOperation,
    SignatureOperation,
    EncryptionOperation,
    ProofOperation,
    SecurityEvent,
    ComplianceEvent,
}

/// Audit results
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuditResult {
    Success,
    Failure,
    Warning,
    Error,
}

/// Compliance reporting engine
pub struct ComplianceReportingEngine {
    report_templates: HashMap<String, ReportTemplate>,
    scheduling_engine: ReportSchedulingEngine,
    distribution_engine: ReportDistributionEngine,
}

/// Report templates
#[derive(Debug, Clone)]
pub struct ReportTemplate {
    pub template_id: String,
    pub template_name: String,
    pub sections: Vec<ReportSection>,
    pub format: ReportFormat,
}

/// Report sections
#[derive(Debug, Clone)]
pub struct ReportSection {
    pub section_id: String,
    pub section_name: String,
    pub content_generator: ContentGenerator,
    pub data_sources: Vec<String>,
}

/// Content generators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContentGenerator {
    Static,
    Dynamic,
    Template,
    Custom(String),
}

/// Report formats
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReportFormat {
    PDF,
    HTML,
    JSON,
    XML,
    CSV,
    Custom(String),
}

/// Report scheduling engine
pub struct ReportSchedulingEngine {
    schedules: HashMap<String, ReportSchedule>,
    scheduler: ReportScheduler,
}

/// Report schedules
#[derive(Debug, Clone)]
pub struct ReportSchedule {
    pub schedule_id: String,
    pub template_id: String,
    pub schedule_type: ScheduleType,
    pub parameters: ScheduleParameters,
}

/// Schedule types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScheduleType {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
    OnDemand,
    Custom(String),
}

/// Schedule parameters
#[derive(Debug, Clone)]
pub struct ScheduleParameters {
    pub start_date: u64,
    pub end_date: Option<u64>,
    pub frequency: u32,
    pub recipients: Vec<String>,
}

/// Report scheduler
pub struct ReportScheduler {
    scheduler_type: SchedulerType,
    queue_manager: ReportQueueManager,
}

/// Scheduler types
#[derive(Debug, Clone, PartialEq)]
pub enum SchedulerType {
    Cron,
    Interval,
    EventDriven,
    Custom(String),
}

/// Report queue manager
pub struct ReportQueueManager {
    pending_reports: Vec<QueuedReport>,
    running_reports: Vec<RunningReport>,
    completed_reports: Vec<CompletedReport>,
}

/// Queued report
#[derive(Debug, Clone)]
pub struct QueuedReport {
    pub report_id: String,
    pub template_id: String,
    pub queued_at: u64,
    pub priority: ReportPriority,
}

/// Report priorities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReportPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Running report
#[derive(Debug, Clone)]
pub struct RunningReport {
    pub report_id: String,
    pub started_at: u64,
    pub progress: f64,
}

/// Completed report
#[derive(Debug, Clone)]
pub struct CompletedReport {
    pub report_id: String,
    pub template_id: String,
    pub started_at: u64,
    pub completed_at: u64,
    pub success: bool,
}

/// Report distribution engine
pub struct ReportDistributionEngine {
    distribution_channels: HashMap<String, DistributionChannel>,
    delivery_tracker: DeliveryTracker,
}

/// Distribution channels
#[derive(Debug, Clone)]
pub struct DistributionChannel {
    pub channel_id: String,
    pub channel_type: DistributionChannelType,
    pub configuration: ChannelConfiguration,
}

/// Distribution channel types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DistributionChannelType {
    Email,
    FTP,
    SFTP,
    API,
    Webhook,
    Custom(String),
}

/// Channel configuration
#[derive(Debug, Clone)]
pub struct ChannelConfiguration {
    pub endpoint: String,
    pub authentication: AuthenticationMethod,
    pub encryption: bool,
    pub retry_policy: RetryPolicy,
}

/// Retry policy
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff_strategy: BackoffStrategy,
    pub retry_intervals: Vec<u64>,
}

/// Backoff strategies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed,
    Linear,
    Exponential,
    Custom(String),
}

/// Delivery tracker
pub struct DeliveryTracker {
    deliveries: HashMap<String, DeliveryRecord>,
    status: DeliveryStatus,
}

/// Delivery records
#[derive(Debug, Clone)]
pub struct DeliveryRecord {
    pub record_id: String,
    pub report_id: String,
    pub channel_id: String,
    pub attempts: Vec<DeliveryAttempt>,
    pub final_status: DeliveryFinalStatus,
}

/// Delivery attempts
#[derive(Debug, Clone)]
pub struct DeliveryAttempt {
    pub attempt_number: u32,
    pub timestamp: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Delivery final status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeliveryFinalStatus {
    Delivered,
    Failed,
    Pending,
    Cancelled,
}

/// Delivery status
#[derive(Debug, Clone)]
pub struct DeliveryStatus {
    pub total_deliveries: u64,
    pub successful_deliveries: u64,
    pub failed_deliveries: u64,
    pub pending_deliveries: u64,
}

/// Security metrics
#[derive(Debug, Clone)]
pub struct SecurityMetrics {
    pub threat_metrics: ThreatMetrics,
    pub anomaly_metrics: AnomalyMetrics,
    pub compliance_metrics: ComplianceMetrics,
    pub performance_metrics: SecurityPerformanceMetrics,
}

/// Threat metrics
#[derive(Debug, Clone)]
pub struct ThreatMetrics {
    pub threats_detected: u64,
    pub threats_blocked: u64,
    pub false_positives: u64,
    pub detection_rate: f64,
    pub response_time: f64,
}

/// Anomaly metrics
#[derive(Debug, Clone)]
pub struct AnomalyMetrics {
    pub anomalies_detected: u64,
    pub anomalies_investigated: u64,
    pub confirmed_anomalies: u64,
    pub false_positive_rate: f64,
    pub detection_accuracy: f64,
}

/// Compliance metrics
#[derive(Debug, Clone)]
pub struct ComplianceMetrics {
    pub compliance_score: f64,
    pub controls_implemented: u64,
    pub controls_passed: u64,
    pub audit_findings: u64,
    pub remediation_rate: f64,
}

/// Security performance metrics
#[derive(Debug, Clone)]
pub struct SecurityPerformanceMetrics {
    pub average_response_time: f64,
    pub throughput: f64,
    pub resource_utilization: f64,
    pub error_rate: f64,
}
impl SecurityMonitor {
    pub fn new() -> Self {
        Self {
            threat_detector: ThreatDetector::new(),
            anomaly_detector: AnomalyDetector::new(),
            compliance_monitor: ComplianceMonitor::new(),
            security_metrics: SecurityMetrics::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.threat_detector.initialize()?;
        self.anomaly_detector.initialize()?;
        self.compliance_monitor.initialize()?;
        Ok(())
    }

    pub fn get_metrics(&self) -> SecurityMetrics {
        self.security_metrics.clone()
    }
}

impl ThreatDetector {
    pub fn new() -> Self {
        Self {
            threat_signatures: HashMap::new(),
            detection_rules: Vec::new(),
            alert_system: SecurityAlertSystem::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.alert_system.initialize()?;
        Ok(())
    }

    /// Register a threat signature.
    pub fn add_threat_signature(&mut self, signature: ThreatSignature) {
        self.threat_signatures
            .insert(signature.signature_id.clone(), signature);
    }

    /// Look up a threat signature by id.
    pub fn get_threat_signature(&self, signature_id: &str) -> Option<&ThreatSignature> {
        self.threat_signatures.get(signature_id)
    }

    /// Iterate over all registered threat signatures.
    pub fn list_threat_signatures(&self) -> impl Iterator<Item = &ThreatSignature> {
        self.threat_signatures.values()
    }

    /// Add a detection rule.
    pub fn add_detection_rule(&mut self, rule: DetectionRule) {
        self.detection_rules.push(rule);
    }

    /// Iterate over all registered detection rules.
    pub fn list_detection_rules(&self) -> impl Iterator<Item = &DetectionRule> {
        self.detection_rules.iter()
    }
}

impl SecurityAlertSystem {
    pub fn new() -> Self {
        Self {
            alert_types: vec![SecurityAlertType::Threat, SecurityAlertType::Anomaly],
            notification_channels: vec![NotificationChannel::Email],
            escalation_policies: Vec::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Get the configured alert types.
    pub fn alert_types(&self) -> &[SecurityAlertType] {
        &self.alert_types
    }

    /// Add an alert type if not already present.
    pub fn add_alert_type(&mut self, alert_type: SecurityAlertType) {
        if !self.alert_types.contains(&alert_type) {
            self.alert_types.push(alert_type);
        }
    }

    /// Get the configured notification channels.
    pub fn notification_channels(&self) -> &[NotificationChannel] {
        &self.notification_channels
    }

    /// Add a notification channel if not already present.
    pub fn add_notification_channel(&mut self, channel: NotificationChannel) {
        if !self.notification_channels.contains(&channel) {
            self.notification_channels.push(channel);
        }
    }

    /// Add an escalation policy.
    pub fn add_escalation_policy(&mut self, policy: EscalationPolicy) {
        self.escalation_policies.push(policy);
    }

    /// Iterate over all registered escalation policies.
    pub fn list_escalation_policies(&self) -> impl Iterator<Item = &EscalationPolicy> {
        self.escalation_policies.iter()
    }
}

impl AnomalyDetector {
    pub fn new() -> Self {
        Self {
            detection_algorithms: vec![AnomalyDetectionAlgorithm::Statistical],
            baseline_models: HashMap::new(),
            alert_thresholds: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Get the configured detection algorithms.
    pub fn detection_algorithms(&self) -> &[AnomalyDetectionAlgorithm] {
        &self.detection_algorithms
    }

    /// Add a detection algorithm if not already present.
    pub fn add_detection_algorithm(&mut self, algorithm: AnomalyDetectionAlgorithm) {
        if !self.detection_algorithms.contains(&algorithm) {
            self.detection_algorithms.push(algorithm);
        }
    }

    /// Register a baseline model.
    pub fn add_baseline_model(&mut self, model: BaselineModel) {
        self.baseline_models.insert(model.model_id.clone(), model);
    }

    /// Look up a baseline model by id.
    pub fn get_baseline_model(&self, model_id: &str) -> Option<&BaselineModel> {
        self.baseline_models.get(model_id)
    }

    /// Iterate over all registered baseline models.
    pub fn list_baseline_models(&self) -> impl Iterator<Item = &BaselineModel> {
        self.baseline_models.values()
    }

    /// Set an alert threshold for a named metric.
    pub fn set_alert_threshold(&mut self, metric: String, threshold: f64) {
        self.alert_thresholds.insert(metric, threshold);
    }

    /// Look up an alert threshold by metric name.
    pub fn get_alert_threshold(&self, metric: &str) -> Option<f64> {
        self.alert_thresholds.get(metric).copied()
    }
}

impl ComplianceMonitor {
    pub fn new() -> Self {
        Self {
            compliance_frameworks: HashMap::new(),
            audit_trail: AuditTrail::new(),
            reporting_engine: ComplianceReportingEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.reporting_engine.initialize()?;
        Ok(())
    }

    /// Register a compliance framework.
    pub fn add_compliance_framework(&mut self, framework: ComplianceFramework) {
        self.compliance_frameworks
            .insert(framework.framework_id.clone(), framework);
    }

    /// Look up a compliance framework by id.
    pub fn get_compliance_framework(&self, framework_id: &str) -> Option<&ComplianceFramework> {
        self.compliance_frameworks.get(framework_id)
    }

    /// Iterate over all registered compliance frameworks.
    pub fn list_compliance_frameworks(&self) -> impl Iterator<Item = &ComplianceFramework> {
        self.compliance_frameworks.values()
    }

    /// Get a reference to the audit trail.
    pub fn audit_trail(&self) -> &AuditTrail {
        &self.audit_trail
    }

    /// Get a mutable reference to the audit trail.
    pub fn audit_trail_mut(&mut self) -> &mut AuditTrail {
        &mut self.audit_trail
    }
}

impl AuditTrail {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            retention_policy: RetentionPolicy {
                retention_days: 2555, // 7 years
                auto_delete: false,
                archive_before_delete: true,
            },
        }
    }

    /// Record an audit entry, enforcing retention policy.
    pub fn add_entry(&mut self, entry: AuditEntry) {
        let cutoff = entry
            .timestamp
            .saturating_sub((self.retention_policy.retention_days as u64) * 86400);
        self.entries.retain(|e| e.timestamp >= cutoff);
        self.entries.push(entry);
    }

    /// Number of recorded audit entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Iterate over audit entries.
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Get the retention policy for the audit trail.
    pub fn retention_policy(&self) -> &RetentionPolicy {
        &self.retention_policy
    }
}

impl ComplianceReportingEngine {
    pub fn new() -> Self {
        Self {
            report_templates: HashMap::new(),
            scheduling_engine: ReportSchedulingEngine::new(),
            distribution_engine: ReportDistributionEngine::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        self.scheduling_engine.initialize()?;
        self.distribution_engine.initialize()?;
        Ok(())
    }

    /// Register a report template.
    pub fn add_report_template(&mut self, template: ReportTemplate) {
        self.report_templates
            .insert(template.template_id.clone(), template);
    }

    /// Look up a report template by id.
    pub fn get_report_template(&self, template_id: &str) -> Option<&ReportTemplate> {
        self.report_templates.get(template_id)
    }

    /// Iterate over all registered report templates.
    pub fn list_report_templates(&self) -> impl Iterator<Item = &ReportTemplate> {
        self.report_templates.values()
    }
}

impl ReportSchedulingEngine {
    pub fn new() -> Self {
        Self {
            schedules: HashMap::new(),
            scheduler: ReportScheduler::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Register a report schedule.
    pub fn add_schedule(&mut self, schedule: ReportSchedule) {
        self.schedules
            .insert(schedule.schedule_id.clone(), schedule);
    }

    /// Look up a report schedule by id.
    pub fn get_schedule(&self, schedule_id: &str) -> Option<&ReportSchedule> {
        self.schedules.get(schedule_id)
    }

    /// Iterate over all registered report schedules.
    pub fn list_schedules(&self) -> impl Iterator<Item = &ReportSchedule> {
        self.schedules.values()
    }

    /// Get a reference to the report scheduler.
    pub fn scheduler(&self) -> &ReportScheduler {
        &self.scheduler
    }

    /// Get a mutable reference to the report scheduler.
    pub fn scheduler_mut(&mut self) -> &mut ReportScheduler {
        &mut self.scheduler
    }
}

impl ReportScheduler {
    pub fn new() -> Self {
        Self {
            scheduler_type: SchedulerType::Cron,
            queue_manager: ReportQueueManager::new(),
        }
    }

    /// Get the scheduler type.
    pub fn scheduler_type(&self) -> &SchedulerType {
        &self.scheduler_type
    }

    /// Set the scheduler type.
    pub fn set_scheduler_type(&mut self, scheduler_type: SchedulerType) {
        self.scheduler_type = scheduler_type;
    }

    /// Get a reference to the queue manager.
    pub fn queue_manager(&self) -> &ReportQueueManager {
        &self.queue_manager
    }

    /// Get a mutable reference to the queue manager.
    pub fn queue_manager_mut(&mut self) -> &mut ReportQueueManager {
        &mut self.queue_manager
    }
}

impl ReportQueueManager {
    pub fn new() -> Self {
        Self {
            pending_reports: Vec::new(),
            running_reports: Vec::new(),
            completed_reports: Vec::new(),
        }
    }

    /// Enqueue a pending report.
    pub fn enqueue_report(&mut self, report: QueuedReport) {
        self.pending_reports.push(report);
    }

    /// Dequeue the next pending report and mark it as running.
    pub fn start_next_report(&mut self) -> Option<QueuedReport> {
        if self.pending_reports.is_empty() {
            None
        } else {
            let report = self.pending_reports.remove(0);
            self.running_reports.push(RunningReport {
                report_id: report.report_id.clone(),
                started_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                progress: 0.0,
            });
            Some(report)
        }
    }

    /// Mark a running report as completed.
    pub fn complete_report(&mut self, report: CompletedReport) {
        self.running_reports
            .retain(|r| r.report_id != report.report_id);
        self.completed_reports.push(report);
    }

    /// Number of pending reports.
    pub fn pending_count(&self) -> usize {
        self.pending_reports.len()
    }

    /// Number of running reports.
    pub fn running_count(&self) -> usize {
        self.running_reports.len()
    }

    /// Number of completed reports.
    pub fn completed_count(&self) -> usize {
        self.completed_reports.len()
    }
}

impl ReportDistributionEngine {
    pub fn new() -> Self {
        Self {
            distribution_channels: HashMap::new(),
            delivery_tracker: DeliveryTracker::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), CryptographicError> {
        Ok(())
    }

    /// Register a distribution channel.
    pub fn add_distribution_channel(&mut self, channel: DistributionChannel) {
        self.distribution_channels
            .insert(channel.channel_id.clone(), channel);
    }

    /// Look up a distribution channel by id.
    pub fn get_distribution_channel(&self, channel_id: &str) -> Option<&DistributionChannel> {
        self.distribution_channels.get(channel_id)
    }

    /// Iterate over all registered distribution channels.
    pub fn list_distribution_channels(&self) -> impl Iterator<Item = &DistributionChannel> {
        self.distribution_channels.values()
    }

    /// Get a reference to the delivery tracker.
    pub fn delivery_tracker(&self) -> &DeliveryTracker {
        &self.delivery_tracker
    }

    /// Get a mutable reference to the delivery tracker.
    pub fn delivery_tracker_mut(&mut self) -> &mut DeliveryTracker {
        &mut self.delivery_tracker
    }
}

impl DeliveryTracker {
    pub fn new() -> Self {
        Self {
            deliveries: HashMap::new(),
            status: DeliveryStatus {
                total_deliveries: 0,
                successful_deliveries: 0,
                failed_deliveries: 0,
                pending_deliveries: 0,
            },
        }
    }

    /// Record a delivery and update aggregate status counters.
    pub fn record_delivery(&mut self, record: DeliveryRecord) {
        self.status.total_deliveries += 1;
        match record.final_status {
            DeliveryFinalStatus::Delivered => self.status.successful_deliveries += 1,
            DeliveryFinalStatus::Failed => self.status.failed_deliveries += 1,
            DeliveryFinalStatus::Pending | DeliveryFinalStatus::Cancelled => {
                self.status.pending_deliveries += 1;
            }
        }
        self.deliveries.insert(record.record_id.clone(), record);
    }

    /// Look up a delivery record by id.
    pub fn get_delivery(&self, record_id: &str) -> Option<&DeliveryRecord> {
        self.deliveries.get(record_id)
    }

    /// Iterate over all recorded deliveries.
    pub fn list_deliveries(&self) -> impl Iterator<Item = &DeliveryRecord> {
        self.deliveries.values()
    }

    /// Get a snapshot of the aggregate delivery status.
    pub fn status(&self) -> &DeliveryStatus {
        &self.status
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

impl SecurityMetrics {
    pub fn new() -> Self {
        Self {
            threat_metrics: ThreatMetrics::new(),
            anomaly_metrics: AnomalyMetrics::new(),
            compliance_metrics: ComplianceMetrics::new(),
            performance_metrics: SecurityPerformanceMetrics::new(),
        }
    }

    pub fn get_metrics(&self) -> SecurityMetrics {
        self.clone()
    }
}

impl ThreatMetrics {
    pub fn new() -> Self {
        Self {
            threats_detected: 0,
            threats_blocked: 0,
            false_positives: 0,
            detection_rate: 0.0,
            response_time: 0.0,
        }
    }
}

impl AnomalyMetrics {
    pub fn new() -> Self {
        Self {
            anomalies_detected: 0,
            anomalies_investigated: 0,
            confirmed_anomalies: 0,
            false_positive_rate: 0.0,
            detection_accuracy: 0.0,
        }
    }
}

impl ComplianceMetrics {
    pub fn new() -> Self {
        Self {
            compliance_score: 1.0,
            controls_implemented: 0,
            controls_passed: 0,
            audit_findings: 0,
            remediation_rate: 0.0,
        }
    }
}

impl SecurityPerformanceMetrics {
    pub fn new() -> Self {
        Self {
            average_response_time: 0.0,
            throughput: 0.0,
            resource_utilization: 0.0,
            error_rate: 0.0,
        }
    }
}
