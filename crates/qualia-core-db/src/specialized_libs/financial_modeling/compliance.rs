use super::*;

/// Internal verdict from evaluating a single compliance rule.
struct RuleVerdict {
    passed: bool,
    flagged: bool,
    message: String,
    recommendation: String,
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
                violations.push(format!("{}: {}", rule.rule_id, verdict.message));
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
                        recommendation: "Set the 'max_position' parameter to a positive value."
                            .to_string(),
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
                            message: "KYC verification required but owner identity not verified"
                                .to_string(),
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
                        recommendation: "Provide owner identification for AML screening."
                            .to_string(),
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
                        recommendation: "Set the 'margin_pct' parameter to a positive value."
                            .to_string(),
                    };
                }
                let required_margin = portfolio.total_value * margin_pct / 100.0;
                if portfolio.cash_balance < required_margin {
                    return RuleVerdict {
                        passed: false,
                        flagged: false,
                        message: format!(
                            "Cash balance {:.2} below required margin {:.2} ({:.1}% of {:.2})",
                            portfolio.cash_balance,
                            required_margin,
                            margin_pct,
                            portfolio.total_value
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
                    message: format!(
                        "Margin satisfied: {:.2} >= {:.2}",
                        portfolio.cash_balance, required_margin
                    ),
                    recommendation: String::new(),
                }
            }
            ComplianceRuleType::TradingRestriction => {
                let restricted = rule
                    .string_parameters
                    .get("restricted_assets")
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
                    recommendation: "Implement a custom evaluator if enforcement is needed."
                        .to_string(),
                }
            }
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

    pub fn add_surveillance_rule(&mut self, rule: SurveillanceRule) {
        self.surveillance_rules.insert(rule.rule_id.clone(), rule);
    }

    pub fn get_surveillance_rule(&self, rule_id: &str) -> Option<&SurveillanceRule> {
        self.surveillance_rules.get(rule_id)
    }

    pub fn list_surveillance_rules(&self) -> Vec<String> {
        self.surveillance_rules.keys().cloned().collect()
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

    pub fn add_detection_algorithm(&mut self, algorithm: DetectionAlgorithm) {
        self.detection_algorithms
            .insert(algorithm.algorithm_id.clone(), algorithm);
    }

    pub fn get_detection_algorithm(&self, algorithm_id: &str) -> Option<&DetectionAlgorithm> {
        self.detection_algorithms.get(algorithm_id)
    }

    pub fn list_detection_algorithms(&self) -> Vec<String> {
        self.detection_algorithms.keys().cloned().collect()
    }

    pub fn add_anomaly_pattern(&mut self, pattern: AnomalyPattern) {
        self.anomaly_patterns
            .insert(pattern.pattern_id.clone(), pattern);
    }

    pub fn get_anomaly_pattern(&self, pattern_id: &str) -> Option<&AnomalyPattern> {
        self.anomaly_patterns.get(pattern_id)
    }

    pub fn list_anomaly_patterns(&self) -> Vec<String> {
        self.anomaly_patterns.keys().cloned().collect()
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

    pub fn add_alert(&mut self, alert: Alert) {
        self.alerts.insert(alert.alert_id.clone(), alert);
    }

    pub fn get_alert(&self, alert_id: &str) -> Option<&Alert> {
        self.alerts.get(alert_id)
    }

    pub fn list_alerts(&self) -> Vec<String> {
        self.alerts.keys().cloned().collect()
    }

    pub fn alert_escalation(&self) -> &AlertEscalation {
        &self.alert_escalation
    }

    pub fn alert_escalation_mut(&mut self) -> &mut AlertEscalation {
        &mut self.alert_escalation
    }

    pub fn notification_system(&self) -> &NotificationSystem {
        &self.notification_system
    }

    pub fn notification_system_mut(&mut self) -> &mut NotificationSystem {
        &mut self.notification_system
    }
}

impl AlertEscalation {
    pub fn new() -> Self {
        Self {
            escalation_rules: HashMap::new(),
            escalation_history: HashMap::new(),
        }
    }

    pub fn add_escalation_rule(&mut self, rule: EscalationRule) {
        self.escalation_rules.insert(rule.rule_id.clone(), rule);
    }

    pub fn get_escalation_rule(&self, rule_id: &str) -> Option<&EscalationRule> {
        self.escalation_rules.get(rule_id)
    }

    pub fn list_escalation_rules(&self) -> Vec<String> {
        self.escalation_rules.keys().cloned().collect()
    }

    pub fn add_escalation_history(&mut self, history: EscalationHistory) {
        self.escalation_history
            .insert(history.history_id.clone(), history);
    }

    pub fn get_escalation_history(&self, history_id: &str) -> Option<&EscalationHistory> {
        self.escalation_history.get(history_id)
    }

    pub fn list_escalation_history(&self) -> Vec<String> {
        self.escalation_history.keys().cloned().collect()
    }
}

impl NotificationSystem {
    pub fn new() -> Self {
        Self {
            notification_channels: HashMap::new(),
            notification_templates: HashMap::new(),
        }
    }

    pub fn add_channel(&mut self, channel: NotificationChannel) {
        self.notification_channels
            .insert(channel.channel_id.clone(), channel);
    }

    pub fn get_channel(&self, channel_id: &str) -> Option<&NotificationChannel> {
        self.notification_channels.get(channel_id)
    }

    pub fn list_channels(&self) -> Vec<String> {
        self.notification_channels.keys().cloned().collect()
    }

    pub fn add_template(&mut self, template: NotificationTemplate) {
        self.notification_templates
            .insert(template.template_id.clone(), template);
    }

    pub fn get_template(&self, template_id: &str) -> Option<&NotificationTemplate> {
        self.notification_templates.get(template_id)
    }

    pub fn list_templates(&self) -> Vec<String> {
        self.notification_templates.keys().cloned().collect()
    }
}
