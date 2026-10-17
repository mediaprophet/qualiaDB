use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Privacy protection
pub struct PrivacyProtection {
    encryption: EncryptionManager,
    anonymization: AnonymizationEngine,
    access_logging: AccessLogging,
    consent_management: ConsentManagement,
}

/// Encryption manager
pub struct EncryptionManager {
    encryption_algorithms: HashMap<String, EncryptionAlgorithm>,
    key_management: KeyManagement,
    data_protection: DataProtection,
}

/// Encryption algorithms
#[derive(Debug, Clone)]
pub struct EncryptionAlgorithm {
    pub algorithm_id: String,
    pub algorithm_name: String,
    pub algorithm_type: EncryptionType,
    pub key_size: u32,
    pub strength: EncryptionStrength,
}

/// Encryption types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EncryptionType {
    AES,
    RSA,
    ECC,
    ChaCha20,
    Custom(String),
}

/// Encryption strength
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EncryptionStrength {
    Weak,
    Moderate,
    Strong,
    Military,
}

/// Key management
pub struct KeyManagement {
    keys: HashMap<String, EncryptionKey>,
    key_rotation: KeyRotation,
    key_recovery: KeyRecovery,
}

/// Encryption keys
#[derive(Debug, Clone)]
pub struct EncryptionKey {
    pub key_id: String,
    pub key_type: KeyType,
    pub key_value: Vec<u8>,
    pub creation_date: u64,
    pub expiry_date: Option<u64>,
    pub usage_count: u64,
}

/// Key types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KeyType {
    Symmetric,
    Asymmetric,
    Public,
    Private,
}

/// Key rotation
pub struct KeyRotation {
    rotation_policy: RotationPolicy,
    rotation_schedule: RotationSchedule,
    rotation_history: RotationHistory,
}

/// Rotation policy
#[derive(Debug, Clone)]
pub struct RotationPolicy {
    pub policy_id: String,
    pub rotation_interval: u32,
    pub rotation_trigger: RotationTrigger,
    pub compliance_requirements: Vec<ComplianceRequirement>,
}

/// Rotation triggers
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RotationTrigger {
    TimeBased,
    UsageBased,
    SecurityEvent,
    Manual,
}

/// Compliance requirements
#[derive(Debug, Clone)]
pub struct ComplianceRequirement {
    pub requirement_id: String,
    pub standard: String,
    pub requirement: String,
    pub mandatory: bool,
}

/// Rotation schedule
#[derive(Debug, Clone)]
pub struct RotationSchedule {
    pub schedule_id: String,
    pub next_rotation: u64,
    pub rotation_frequency: u32,
    pub affected_keys: Vec<String>,
}

/// Rotation history
#[derive(Debug, Clone)]
pub struct RotationHistory {
    pub history_id: String,
    pub rotation_date: u64,
    pub old_key: String,
    pub new_key: String,
    pub reason: String,
}

/// Key recovery
pub struct KeyRecovery {
    recovery_methods: HashMap<String, RecoveryMethod>,
    recovery_procedures: HashMap<String, RecoveryProcedure>,
}

/// Recovery methods
#[derive(Debug, Clone)]
pub struct RecoveryMethod {
    pub method_id: String,
    pub method_type: RecoveryMethodType,
    pub security_level: SecurityLevel,
}

/// Recovery method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecoveryMethodType {
    ShamirSecretSharing,
    HardwareToken,
    Biometric,
    MultiFactor,
}

/// Security levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SecurityLevel {
    Low,
    Medium,
    High,
    Maximum,
}

/// Recovery procedures
#[derive(Debug, Clone)]
pub struct RecoveryProcedure {
    pub procedure_id: String,
    pub steps: Vec<RecoveryStep>,
    pub verification_required: bool,
}

/// Recovery steps
#[derive(Debug, Clone)]
pub struct RecoveryStep {
    pub step_id: String,
    pub step_description: String,
    pub step_type: RecoveryStepType,
}

/// Recovery step types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecoveryStepType {
    Authentication,
    Verification,
    Decryption,
    Validation,
}

/// Data protection
pub struct DataProtection {
    protection_policies: HashMap<String, ProtectionPolicy>,
    breach_detection: BreachDetection,
    incident_response: IncidentResponse,
}

/// Protection policies
#[derive(Debug, Clone)]
pub struct ProtectionPolicy {
    pub policy_id: String,
    pub policy_name: String,
    pub policy_type: PolicyType,
    pub data_classification: DataClassification,
    pub access_controls: Vec<AccessControl>,
}

/// Policy types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyType {
    HIPAA,
    GDPR,
    CCPA,
    HITRUST,
    Custom(String),
}

/// Data classification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataClassification {
    Public,
    Internal,
    Confidential,
    Restricted,
    PHI, // Protected Health Information
}

/// Access controls
#[derive(Debug, Clone)]
pub struct AccessControl {
    pub control_id: String,
    pub control_type: AccessControlType,
    pub permissions: Vec<Permission>,
    pub conditions: Vec<AccessCondition>,
}

/// Access control types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessControlType {
    RoleBased,
    AttributeBased,
    RuleBased,
    Discretionary,
}

/// Permissions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Permission {
    Read,
    Write,
    Delete,
    Share,
    Export,
}

/// Access conditions
#[derive(Debug, Clone)]
pub struct AccessCondition {
    pub condition_id: String,
    pub condition_type: ConditionType,
    pub condition_value: String,
}

/// Condition types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConditionType {
    Time,
    Location,
    Device,
    User,
}

/// Breach detection
pub struct BreachDetection {
    detection_algorithms: HashMap<String, DetectionAlgorithm>,
    alert_systems: HashMap<String, AlertSystem>,
}

/// Detection algorithms
#[derive(Debug, Clone)]
pub struct DetectionAlgorithm {
    pub algorithm_id: String,
    pub algorithm_type: DetectionAlgorithmType,
    pub sensitivity: f64,
    pub false_positive_rate: f64,
}

/// Detection algorithm types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DetectionAlgorithmType {
    AnomalyDetection,
    PatternRecognition,
    MachineLearning,
    RuleBased,
}

/// Alert systems
#[derive(Debug, Clone)]
pub struct AlertSystem {
    pub system_id: String,
    pub system_type: AlertSystemType,
    pub notification_channels: Vec<NotificationChannel>,
}

/// Alert system types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertSystemType {
    Email,
    SMS,
    Slack,
    Pager,
    Custom(String),
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
    Webhook,
    API,
}

/// Channel configuration
#[derive(Debug, Clone)]
pub struct ChannelConfiguration {
    pub endpoint: String,
    pub authentication: AuthenticationMethod,
    pub format: MessageFormat,
}

/// Message formats
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageFormat {
    JSON,
    XML,
    Text,
    Custom(String),
}

/// Incident response
pub struct IncidentResponse {
    response_plans: HashMap<String, ResponsePlan>,
    response_team: ResponseTeam,
    escalation_procedures: EscalationProcedures,
}

/// Response plans
#[derive(Debug, Clone)]
pub struct ResponsePlan {
    pub plan_id: String,
    pub plan_name: String,
    pub plan_type: ResponsePlanType,
    pub steps: Vec<ResponseStep>,
}

/// Response plan types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResponsePlanType {
    DataBreach,
    SecurityIncident,
    PrivacyViolation,
    SystemOutage,
}

/// Response steps
#[derive(Debug, Clone)]
pub struct ResponseStep {
    pub step_id: String,
    pub step_description: String,
    pub step_type: ResponseStepType,
    pub responsible_party: String,
    pub deadline: u32,
}

/// Response step types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResponseStepType {
    Investigation,
    Containment,
    Eradication,
    Recovery,
    Reporting,
}

/// Response team
#[derive(Debug, Clone)]
pub struct ResponseTeam {
    pub team_id: String,
    pub team_name: String,
    pub members: Vec<TeamMember>,
    pub roles: HashMap<String, TeamRole>,
}

/// Team members
#[derive(Debug, Clone)]
pub struct TeamMember {
    pub member_id: String,
    pub name: String,
    pub role: String,
    pub contact_info: ContactInfo,
    pub availability: Availability,
}

/// Availability
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Availability {
    Available,
    Busy,
    OnCall,
    Unavailable,
}

/// Team roles
#[derive(Debug, Clone)]
pub struct TeamRole {
    pub role_id: String,
    pub role_name: String,
    pub responsibilities: Vec<String>,
    pub authority_level: AuthorityLevel,
}

/// Authority levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthorityLevel {
    Observer,
    Operator,
    Manager,
    Director,
}

/// Escalation procedures
pub struct EscalationProcedures {
    escalation_rules: HashMap<String, EscalationRule>,
    escalation_matrix: EscalationMatrix,
}

/// Escalation rules
#[derive(Debug, Clone)]
pub struct EscalationRule {
    pub rule_id: String,
    pub rule_name: String,
    pub trigger_conditions: Vec<TriggerCondition>,
    pub escalation_actions: Vec<EscalationAction>,
}

/// Trigger conditions
#[derive(Debug, Clone)]
pub struct TriggerCondition {
    pub condition_id: String,
    pub condition_type: TriggerConditionType,
    pub condition_value: String,
}

/// Trigger condition types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TriggerConditionType {
    Severity,
    Time,
    Impact,
    Compliance,
}

/// Escalation actions
#[derive(Debug, Clone)]
pub struct EscalationAction {
    pub action_id: String,
    pub action_type: EscalationActionType,
    pub action_details: String,
}

/// Escalation action types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EscalationActionType {
    Notify,
    Escalate,
    Activate,
    Report,
}

/// Escalation matrix
#[derive(Debug, Clone)]
pub struct EscalationMatrix {
    pub matrix_id: String,
    pub matrix_name: String,
    pub escalation_levels: Vec<EscalationLevel>,
}

/// Escalation levels
#[derive(Debug, Clone)]
pub struct EscalationLevel {
    pub level_id: String,
    pub level_name: String,
    pub level_number: u32,
    pub notification_recipients: Vec<String>,
    pub response_time: u32,
}

/// Anonymization engine
pub struct AnonymizationEngine {
    anonymization_methods: HashMap<String, AnonymizationMethod>,
    privacy_models: HashMap<String, PrivacyModel>,
    risk_assessment: RiskAssessment,
}

/// Anonymization methods
#[derive(Debug, Clone)]
pub struct AnonymizationMethod {
    pub method_id: String,
    pub method_name: String,
    pub method_type: AnonymizationMethodType,
    pub parameters: AnonymizationParameters,
}

/// Anonymization method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnonymizationMethodType {
    Suppression,
    Generalization,
    Perturbation,
    Masking,
    Pseudonymization,
}

/// Anonymization parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizationParameters {
    pub privacy_threshold: f64,
    pub information_loss: f64,
    pub utility_preservation: f64,
}

/// Privacy models
#[derive(Debug, Clone)]
pub struct PrivacyModel {
    pub model_id: String,
    pub model_name: String,
    pub model_type: PrivacyModelType,
    pub parameters: PrivacyModelParameters,
}

/// Privacy model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PrivacyModelType {
    KAnonymity,
    LDiversity,
    TCloseness,
    DifferentialPrivacy,
}

/// Privacy model parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyModelParameters {
    pub k_value: Option<u32>,
    pub l_value: Option<u32>,
    pub t_value: Option<f64>,
    pub epsilon: Option<f64>,
}

/// Risk assessment
pub struct RiskAssessment {
    risk_models: HashMap<String, RiskModel>,
    risk_metrics: HashMap<String, RiskMetric>,
}

/// Risk models
#[derive(Debug, Clone)]
pub struct RiskModel {
    pub model_id: String,
    pub model_name: String,
    pub model_type: RiskModelType,
    pub risk_factors: Vec<RiskFactor>,
}

/// Risk model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RiskModelType {
    Statistical,
    MachineLearning,
    ExpertSystem,
    Hybrid,
}

/// Risk factors
#[derive(Debug, Clone)]
pub struct RiskFactor {
    pub factor_id: String,
    pub factor_name: String,
    pub factor_weight: f64,
    pub factor_value: f64,
}

/// Risk metrics
#[derive(Debug, Clone)]
pub struct RiskMetric {
    pub metric_id: String,
    pub metric_name: String,
    pub metric_value: f64,
    pub metric_threshold: f64,
}

/// Access logging
pub struct AccessLogging {
    log_entries: HashMap<String, LogEntry>,
    log_analysis: LogAnalysis,
    retention_policy: RetentionPolicy,
}

/// Log entries
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub entry_id: String,
    pub timestamp: u64,
    pub user_id: String,
    pub action: AccessAction,
    pub resource: String,
    pub outcome: AccessOutcome,
    pub details: String,
}

/// Access actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessAction {
    Read,
    Write,
    Delete,
    Share,
    Export,
    Login,
    Logout,
}

/// Access outcomes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessOutcome {
    Success,
    Failure,
    Blocked,
    Suspicious,
}

/// Log analysis
pub struct LogAnalysis {
    analysis_methods: HashMap<String, AnalysisMethod>,
    anomaly_detection: AnomalyDetection,
}

/// Analysis methods
#[derive(Debug, Clone)]
pub struct AnalysisMethod {
    pub method_id: String,
    pub method_name: String,
    pub method_type: AnalysisMethodType,
}

/// Analysis method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnalysisMethodType {
    Statistical,
    Pattern,
    Behavioral,
    Temporal,
}

/// Anomaly detection
#[derive(Debug, Clone)]
pub struct AnomalyDetection {
    detection_algorithms: HashMap<String, DetectionAlgorithm>,
    alert_thresholds: HashMap<String, f64>,
}

/// Retention policy
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    pub policy_id: String,
    pub policy_name: String,
    pub retention_period: u32,
    pub archival_period: u32,
    pub deletion_method: DeletionMethod,
}

/// Deletion methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeletionMethod {
    SoftDelete,
    HardDelete,
    SecureDelete,
}

/// Consent management
pub struct ConsentManagement {
    consent_records: HashMap<String, ConsentRecord>,
    consent_policies: HashMap<String, ConsentPolicy>,
    consent_workflows: HashMap<String, ConsentWorkflow>,
}

/// Consent records
#[derive(Debug, Clone)]
pub struct ConsentRecord {
    pub record_id: String,
    pub patient_id: String,
    pub consent_type: ConsentType,
    pub consent_status: ConsentStatus,
    pub granted_date: u64,
    pub expiry_date: Option<u64>,
    pub purpose: String,
    pub limitations: Vec<String>,
}

/// Consent types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConsentType {
    Treatment,
    Research,
    DataSharing,
    Marketing,
    Genetic,
}

/// Consent status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConsentStatus {
    Granted,
    Denied,
    Revoked,
    Expired,
}

/// Consent policies
#[derive(Debug, Clone)]
pub struct ConsentPolicy {
    pub policy_id: String,
    pub policy_name: String,
    pub policy_type: ConsentPolicyType,
    pub requirements: Vec<ConsentRequirement>,
}

/// Consent policy types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConsentPolicyType {
    HIPAA,
    GDPR,
    Institutional,
    StudySpecific,
}

/// Consent requirements
#[derive(Debug, Clone)]
pub struct ConsentRequirement {
    pub requirement_id: String,
    pub requirement_name: String,
    pub requirement_type: RequirementType,
    pub mandatory: bool,
}

/// Requirement types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequirementType {
    Informed,
    Written,
    Witnessed,
    Electronic,
}

/// Consent workflows
#[derive(Debug, Clone)]
pub struct ConsentWorkflow {
    pub workflow_id: String,
    pub workflow_name: String,
    pub workflow_steps: Vec<WorkflowStep>,
}

/// Workflow steps
#[derive(Debug, Clone)]
pub struct WorkflowStep {
    pub step_id: String,
    pub step_name: String,
    pub step_type: WorkflowStepType,
    pub step_order: u32,
}

/// Workflow step types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkflowStepType {
    Information,
    Acknowledgment,
    Signature,
    Review,
}

/// Data access control
pub struct DataAccessControl {
    access_policies: HashMap<String, AccessPolicy>,
    authentication: AuthenticationSystem,
    authorization: AuthorizationSystem,
}

/// Access policies
#[derive(Debug, Clone)]
pub struct AccessPolicy {
    pub policy_id: String,
    pub policy_name: String,
    pub policy_type: AccessPolicyType,
    pub rules: Vec<AccessRule>,
}

/// Access policy types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessPolicyType {
    RoleBased,
    AttributeBased,
    RuleBased,
    Hybrid,
}

/// Access rules
#[derive(Debug, Clone)]
pub struct AccessRule {
    pub rule_id: String,
    pub rule_name: String,
    pub conditions: Vec<AccessCondition>,
    pub actions: Vec<AccessAction>,
}

/// Authentication system
pub struct AuthenticationSystem {
    authentication_methods: HashMap<String, AuthenticationMethod>,
    session_management: SessionManagement,
    multi_factor: MultiFactorAuthentication,
}

/// Authentication methods
#[derive(Debug, Clone)]
pub struct AuthenticationMethod {
    pub method_id: String,
    pub method_name: String,
    pub method_type: AuthenticationMethodType,
    pub security_level: SecurityLevel,
}

/// Authentication method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthenticationMethodType {
    Password,
    Biometric,
    Token,
    Certificate,
    SSO,
}

/// Session management
pub struct SessionManagement {
    sessions: HashMap<String, Session>,
    session_policies: HashMap<String, SessionPolicy>,
}

/// Sessions
#[derive(Debug, Clone)]
pub struct Session {
    pub session_id: String,
    pub user_id: String,
    pub creation_time: u64,
    pub expiry_time: u64,
    pub last_activity: u64,
    pub ip_address: String,
    pub user_agent: String,
}

/// Session policies
#[derive(Debug, Clone)]
pub struct SessionPolicy {
    pub policy_id: String,
    pub policy_name: String,
    pub session_timeout: u32,
    pub idle_timeout: u32,
    pub max_concurrent_sessions: u32,
}

/// Multi-factor authentication
pub struct MultiFactorAuthentication {
    factors: HashMap<String, AuthenticationFactor>,
    factor_combinations: HashMap<String, FactorCombination>,
}

/// Authentication factors
#[derive(Debug, Clone)]
pub struct AuthenticationFactor {
    pub factor_id: String,
    pub factor_type: AuthenticationFactorType,
    pub factor_provider: String,
}

/// Authentication factor types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthenticationFactorType {
    Knowledge,
    Possession,
    Inherence,
    Location,
}

/// Factor combinations
#[derive(Debug, Clone)]
pub struct FactorCombination {
    pub combination_id: String,
    pub combination_name: String,
    pub required_factors: Vec<String>,
}

/// Authorization system
pub struct AuthorizationSystem {
    authorization_policies: HashMap<String, AuthorizationPolicy>,
    permission_management: PermissionManagement,
    role_management: RoleManagement,
}

/// Authorization policies
#[derive(Debug, Clone)]
pub struct AuthorizationPolicy {
    pub policy_id: String,
    pub policy_name: String,
    pub policy_type: AuthorizationPolicyType,
    pub policy_rules: Vec<AuthorizationRule>,
}

/// Authorization policy types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthorizationPolicyType {
    Allow,
    Deny,
    Conditional,
}

/// Authorization rules
#[derive(Debug, Clone)]
pub struct AuthorizationRule {
    pub rule_id: String,
    pub rule_name: String,
    pub conditions: Vec<AuthorizationCondition>,
    pub decision: AuthorizationDecision,
}

/// Authorization conditions
#[derive(Debug, Clone)]
pub struct AuthorizationCondition {
    pub condition_id: String,
    pub condition_type: AuthorizationConditionType,
    pub condition_value: String,
}

/// Authorization condition types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthorizationConditionType {
    User,
    Role,
    Resource,
    Time,
    Location,
}

/// Authorization decisions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthorizationDecision {
    Permit,
    Deny,
    NotApplicable,
}

/// Permission management
pub struct PermissionManagement {
    permissions: HashMap<String, Permission>,
    permission_groups: HashMap<String, PermissionGroup>,
}

/// Permission groups
#[derive(Debug, Clone)]
pub struct PermissionGroup {
    pub group_id: String,
    pub group_name: String,
    pub permissions: Vec<String>,
}

/// Role management
pub struct RoleManagement {
    roles: HashMap<String, Role>,
    role_hierarchy: RoleHierarchy,
}

/// Roles
#[derive(Debug, Clone)]
pub struct Role {
    pub role_id: String,
    pub role_name: String,
    pub role_description: String,
    pub permissions: Vec<String>,
}

/// Role hierarchy
#[derive(Debug, Clone)]
pub struct RoleHierarchy {
    pub hierarchy_id: String,
    pub parent_roles: Vec<String>,
    pub child_roles: Vec<String>,
}
impl PrivacyProtection {
    /// Create a `PrivacyProtection` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            encryption: EncryptionManager::new(),
            anonymization: AnonymizationEngine::new(),
            access_logging: AccessLogging::new(),
            consent_management: ConsentManagement::new(),
        }
    }

    /// Initialize owned sub-components in dependency order.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        self.encryption.initialize()?;
        self.anonymization.initialize()?;
        self.access_logging.initialize()?;
        self.consent_management.initialize()?;
        Ok(())
    }
}

impl EncryptionManager {
    /// Create a `EncryptionManager` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            encryption_algorithms: HashMap::new(),
            key_management: KeyManagement::new(),
            data_protection: DataProtection::new(),
        }
    }

    /// Initialize owned sub-components in dependency order.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        self.key_management.initialize()?;
        self.data_protection.initialize()?;
        Ok(())
    }

    /// Register `algorithm` (keyed by its id field).
    pub fn add_algorithm(&mut self, algorithm: EncryptionAlgorithm) {
        self.encryption_algorithms
            .insert(algorithm.algorithm_id.clone(), algorithm);
    }

    /// Fetch the registered `algorithm` by id, or `None`.
    pub fn get_algorithm(&self, algorithm_id: &str) -> Option<&EncryptionAlgorithm> {
        self.encryption_algorithms.get(algorithm_id)
    }

    /// Ids of every registered `algorithms`.
    pub fn list_algorithms(&self) -> Vec<String> {
        self.encryption_algorithms.keys().cloned().collect()
    }
}

impl KeyManagement {
    /// Create a `KeyManagement` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            key_rotation: KeyRotation::new(),
            key_recovery: KeyRecovery::new(),
        }
    }

    /// Initialize owned sub-components in dependency order.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    /// Register `key` (keyed by its id field).
    pub fn add_key(&mut self, key: EncryptionKey) {
        self.keys.insert(key.key_id.clone(), key);
    }

    /// Fetch the registered `key` by id, or `None`.
    pub fn get_key(&self, key_id: &str) -> Option<&EncryptionKey> {
        self.keys.get(key_id)
    }

    /// Remove `key` by id, returning it when present.
    pub fn remove_key(&mut self, key_id: &str) -> Option<EncryptionKey> {
        self.keys.remove(key_id)
    }

    /// Borrow the `KeyRotation` sub-component.
    pub fn key_rotation(&self) -> &KeyRotation {
        &self.key_rotation
    }

    /// Borrow the `KeyRecovery` sub-component.
    pub fn key_recovery(&self) -> &KeyRecovery {
        &self.key_recovery
    }
}

impl KeyRotation {
    /// Create a `KeyRotation` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            rotation_policy: RotationPolicy::new(),
            rotation_schedule: RotationSchedule::new(),
            rotation_history: RotationHistory::new(),
        }
    }

    /// Borrow the `RotationPolicy` sub-component.
    pub fn rotation_policy(&self) -> &RotationPolicy {
        &self.rotation_policy
    }

    /// Borrow the `RotationSchedule` sub-component.
    pub fn rotation_schedule(&self) -> &RotationSchedule {
        &self.rotation_schedule
    }

    /// Borrow the `RotationHistory` sub-component.
    pub fn rotation_history(&self) -> &RotationHistory {
        &self.rotation_history
    }
}

impl RotationPolicy {
    /// Create a `RotationPolicy` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            policy_id: "policy_1".to_string(),
            rotation_interval: 90, // 90 days
            rotation_trigger: RotationTrigger::TimeBased,
            compliance_requirements: Vec::new(),
        }
    }
}

impl RotationSchedule {
    /// Create a `RotationSchedule` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            schedule_id: "schedule_1".to_string(),
            next_rotation: 0,
            rotation_frequency: 90,
            affected_keys: Vec::new(),
        }
    }
}

impl RotationHistory {
    /// Create a `RotationHistory` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            history_id: "history_1".to_string(),
            rotation_date: 0,
            old_key: String::new(),
            new_key: String::new(),
            reason: String::new(),
        }
    }
}

impl KeyRecovery {
    /// Create a `KeyRecovery` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            recovery_methods: HashMap::new(),
            recovery_procedures: HashMap::new(),
        }
    }

    /// Register `recovery_method` (keyed by its id field).
    pub fn add_recovery_method(&mut self, method: RecoveryMethod) {
        self.recovery_methods
            .insert(method.method_id.clone(), method);
    }

    /// Fetch the registered `recovery_method` by id, or `None`.
    pub fn get_recovery_method(&self, method_id: &str) -> Option<&RecoveryMethod> {
        self.recovery_methods.get(method_id)
    }

    /// Register `recovery_procedure` (keyed by its id field).
    pub fn add_recovery_procedure(&mut self, procedure: RecoveryProcedure) {
        self.recovery_procedures
            .insert(procedure.procedure_id.clone(), procedure);
    }

    /// Fetch the registered `recovery_procedure` by id, or `None`.
    pub fn get_recovery_procedure(&self, procedure_id: &str) -> Option<&RecoveryProcedure> {
        self.recovery_procedures.get(procedure_id)
    }
}

impl DataProtection {
    /// Create a `DataProtection` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            protection_policies: HashMap::new(),
            breach_detection: BreachDetection::new(),
            incident_response: IncidentResponse::new(),
        }
    }

    /// Initialize owned sub-components in dependency order.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    /// Register `protection_policy` (keyed by its id field).
    pub fn add_protection_policy(&mut self, policy: ProtectionPolicy) {
        self.protection_policies
            .insert(policy.policy_id.clone(), policy);
    }

    /// Fetch the registered `protection_policy` by id, or `None`.
    pub fn get_protection_policy(&self, policy_id: &str) -> Option<&ProtectionPolicy> {
        self.protection_policies.get(policy_id)
    }

    /// Borrow the `BreachDetection` sub-component.
    pub fn breach_detection(&self) -> &BreachDetection {
        &self.breach_detection
    }

    /// Borrow the `IncidentResponse` sub-component.
    pub fn incident_response(&self) -> &IncidentResponse {
        &self.incident_response
    }
}

impl BreachDetection {
    /// Create a `BreachDetection` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            detection_algorithms: HashMap::new(),
            alert_systems: HashMap::new(),
        }
    }

    /// Register `detection_algorithm` (keyed by its id field).
    pub fn add_detection_algorithm(&mut self, algorithm: DetectionAlgorithm) {
        self.detection_algorithms
            .insert(algorithm.algorithm_id.clone(), algorithm);
    }

    /// Fetch the registered `detection_algorithm` by id, or `None`.
    pub fn get_detection_algorithm(&self, algorithm_id: &str) -> Option<&DetectionAlgorithm> {
        self.detection_algorithms.get(algorithm_id)
    }

    /// Register `alert_system` (keyed by its id field).
    pub fn add_alert_system(&mut self, system: AlertSystem) {
        self.alert_systems.insert(system.system_id.clone(), system);
    }

    /// Fetch the registered `alert_system` by id, or `None`.
    pub fn get_alert_system(&self, system_id: &str) -> Option<&AlertSystem> {
        self.alert_systems.get(system_id)
    }
}

impl IncidentResponse {
    /// Create a `IncidentResponse` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            response_plans: HashMap::new(),
            response_team: ResponseTeam::new(),
            escalation_procedures: EscalationProcedures::new(),
        }
    }

    /// Register `response_plan` (keyed by its id field).
    pub fn add_response_plan(&mut self, plan: ResponsePlan) {
        self.response_plans.insert(plan.plan_id.clone(), plan);
    }

    /// Fetch the registered `response_plan` by id, or `None`.
    pub fn get_response_plan(&self, plan_id: &str) -> Option<&ResponsePlan> {
        self.response_plans.get(plan_id)
    }

    /// Borrow the `ResponseTeam` sub-component.
    pub fn response_team(&self) -> &ResponseTeam {
        &self.response_team
    }

    /// Borrow the `EscalationProcedures` sub-component.
    pub fn escalation_procedures(&self) -> &EscalationProcedures {
        &self.escalation_procedures
    }
}

impl ResponseTeam {
    /// Create a `ResponseTeam` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            team_id: "team_1".to_string(),
            team_name: "Incident Response Team".to_string(),
            members: Vec::new(),
            roles: HashMap::new(),
        }
    }
}

impl EscalationProcedures {
    /// Create a `EscalationProcedures` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            escalation_rules: HashMap::new(),
            escalation_matrix: EscalationMatrix::new(),
        }
    }

    /// Register `escalation_rule` (keyed by its id field).
    pub fn add_escalation_rule(&mut self, rule: EscalationRule) {
        self.escalation_rules.insert(rule.rule_id.clone(), rule);
    }

    /// Fetch the registered `escalation_rule` by id, or `None`.
    pub fn get_escalation_rule(&self, rule_id: &str) -> Option<&EscalationRule> {
        self.escalation_rules.get(rule_id)
    }

    /// Borrow the `EscalationMatrix` sub-component.
    pub fn escalation_matrix(&self) -> &EscalationMatrix {
        &self.escalation_matrix
    }
}

impl EscalationMatrix {
    /// Create a `EscalationMatrix` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            matrix_id: "matrix_1".to_string(),
            matrix_name: "Escalation Matrix".to_string(),
            escalation_levels: Vec::new(),
        }
    }
}

impl AnonymizationEngine {
    /// Create a `AnonymizationEngine` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            anonymization_methods: HashMap::new(),
            privacy_models: HashMap::new(),
            risk_assessment: RiskAssessment::new(),
        }
    }

    /// Initialize owned sub-components in dependency order.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    /// Register `anonymization_method` (keyed by its id field).
    pub fn add_anonymization_method(&mut self, method: AnonymizationMethod) {
        self.anonymization_methods
            .insert(method.method_id.clone(), method);
    }

    /// Fetch the registered `anonymization_method` by id, or `None`.
    pub fn get_anonymization_method(&self, method_id: &str) -> Option<&AnonymizationMethod> {
        self.anonymization_methods.get(method_id)
    }

    /// Register `privacy_model` (keyed by its id field).
    pub fn add_privacy_model(&mut self, model: PrivacyModel) {
        self.privacy_models.insert(model.model_id.clone(), model);
    }

    /// Fetch the registered `privacy_model` by id, or `None`.
    pub fn get_privacy_model(&self, model_id: &str) -> Option<&PrivacyModel> {
        self.privacy_models.get(model_id)
    }

    /// Borrow the `RiskAssessment` sub-component.
    pub fn risk_assessment(&self) -> &RiskAssessment {
        &self.risk_assessment
    }
}

impl RiskAssessment {
    /// Create a `RiskAssessment` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            risk_models: HashMap::new(),
            risk_metrics: HashMap::new(),
        }
    }

    /// Register `risk_model` (keyed by its id field).
    pub fn add_risk_model(&mut self, model: RiskModel) {
        self.risk_models.insert(model.model_id.clone(), model);
    }

    /// Fetch the registered `risk_model` by id, or `None`.
    pub fn get_risk_model(&self, model_id: &str) -> Option<&RiskModel> {
        self.risk_models.get(model_id)
    }

    /// Register `risk_metric` (keyed by its id field).
    pub fn add_risk_metric(&mut self, metric: RiskMetric) {
        self.risk_metrics.insert(metric.metric_id.clone(), metric);
    }

    /// Fetch the registered `risk_metric` by id, or `None`.
    pub fn get_risk_metric(&self, metric_id: &str) -> Option<&RiskMetric> {
        self.risk_metrics.get(metric_id)
    }

    /// Weighted-average risk score over `factors`; 0.0 on empty input or zero total weight.
    pub fn assess_risk(&self, factors: &[RiskFactor]) -> f64 {
        if factors.is_empty() {
            return 0.0;
        }
        let total_weight: f64 = factors.iter().map(|f| f.factor_weight).sum();
        if total_weight == 0.0 {
            return 0.0;
        }
        factors
            .iter()
            .map(|f| f.factor_weight * f.factor_value)
            .sum::<f64>()
            / total_weight
    }
}

impl AccessLogging {
    /// Create a `AccessLogging` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            log_entries: HashMap::new(),
            log_analysis: LogAnalysis::new(),
            retention_policy: RetentionPolicy::new(),
        }
    }

    /// Initialize owned sub-components in dependency order.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    /// Register `log_entry` (keyed by its id field).
    pub fn add_log_entry(&mut self, entry: LogEntry) {
        self.log_entries.insert(entry.entry_id.clone(), entry);
    }

    /// Fetch the registered `log_entry` by id, or `None`.
    pub fn get_log_entry(&self, entry_id: &str) -> Option<&LogEntry> {
        self.log_entries.get(entry_id)
    }

    /// Borrow the `LogAnalysis` sub-component.
    pub fn log_analysis(&self) -> &LogAnalysis {
        &self.log_analysis
    }

    /// Borrow the `RetentionPolicy` sub-component.
    pub fn retention_policy(&self) -> &RetentionPolicy {
        &self.retention_policy
    }
}

impl LogAnalysis {
    /// Create a `LogAnalysis` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            analysis_methods: HashMap::new(),
            anomaly_detection: AnomalyDetection::new(),
        }
    }

    /// Register `analysis_method` (keyed by its id field).
    pub fn add_analysis_method(&mut self, method: AnalysisMethod) {
        self.analysis_methods
            .insert(method.method_id.clone(), method);
    }

    /// Fetch the registered `analysis_method` by id, or `None`.
    pub fn get_analysis_method(&self, method_id: &str) -> Option<&AnalysisMethod> {
        self.analysis_methods.get(method_id)
    }

    /// Borrow the `AnomalyDetection` sub-component.
    pub fn anomaly_detection(&self) -> &AnomalyDetection {
        &self.anomaly_detection
    }
}

impl AnomalyDetection {
    /// Create a `AnomalyDetection` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            detection_algorithms: HashMap::new(),
            alert_thresholds: HashMap::new(),
        }
    }

    /// Register `detection_algorithm` (keyed by its id field).
    pub fn add_detection_algorithm(&mut self, algorithm: DetectionAlgorithm) {
        self.detection_algorithms
            .insert(algorithm.algorithm_id.clone(), algorithm);
    }

    /// Fetch the registered `detection_algorithm` by id, or `None`.
    pub fn get_detection_algorithm(&self, algorithm_id: &str) -> Option<&DetectionAlgorithm> {
        self.detection_algorithms.get(algorithm_id)
    }

    /// Set the `alert_threshold`.
    pub fn set_alert_threshold(&mut self, metric_name: &str, threshold: f64) {
        self.alert_thresholds
            .insert(metric_name.to_string(), threshold);
    }

    /// Fetch the registered `alert_threshold` by id, or `None`.
    pub fn get_alert_threshold(&self, metric_name: &str) -> Option<&f64> {
        self.alert_thresholds.get(metric_name)
    }
}

impl RetentionPolicy {
    /// Create a `RetentionPolicy` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            policy_id: "policy_1".to_string(),
            policy_name: "Log Retention Policy".to_string(),
            retention_period: 2555, // 7 years
            archival_period: 3650,  // 10 years
            deletion_method: DeletionMethod::SecureDelete,
        }
    }
}

impl ConsentManagement {
    /// Create a `ConsentManagement` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            consent_records: HashMap::new(),
            consent_policies: HashMap::new(),
            consent_workflows: HashMap::new(),
        }
    }

    /// Initialize owned sub-components in dependency order.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    /// Register `consent_record` (keyed by its id field).
    pub fn add_consent_record(&mut self, record: ConsentRecord) {
        self.consent_records
            .insert(record.record_id.clone(), record);
    }

    /// Fetch the registered `consent_record` by id, or `None`.
    pub fn get_consent_record(&self, record_id: &str) -> Option<&ConsentRecord> {
        self.consent_records.get(record_id)
    }

    /// Register `consent_policy` (keyed by its id field).
    pub fn add_consent_policy(&mut self, policy: ConsentPolicy) {
        self.consent_policies
            .insert(policy.policy_id.clone(), policy);
    }

    /// Fetch the registered `consent_policy` by id, or `None`.
    pub fn get_consent_policy(&self, policy_id: &str) -> Option<&ConsentPolicy> {
        self.consent_policies.get(policy_id)
    }

    /// Register `consent_workflow` (keyed by its id field).
    pub fn add_consent_workflow(&mut self, workflow: ConsentWorkflow) {
        self.consent_workflows
            .insert(workflow.workflow_id.clone(), workflow);
    }

    /// Fetch the registered `consent_workflow` by id, or `None`.
    pub fn get_consent_workflow(&self, workflow_id: &str) -> Option<&ConsentWorkflow> {
        self.consent_workflows.get(workflow_id)
    }
}

impl DataAccessControl {
    /// Create a `DataAccessControl` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            access_policies: HashMap::new(),
            authentication: AuthenticationSystem::new(),
            authorization: AuthorizationSystem::new(),
        }
    }

    /// Initialize owned sub-components in dependency order.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        self.authentication.initialize()?;
        self.authorization.initialize()?;
        Ok(())
    }

    /// Register `access_policy` (keyed by its id field).
    pub fn add_access_policy(&mut self, policy: AccessPolicy) {
        self.access_policies
            .insert(policy.policy_id.clone(), policy);
    }

    /// Fetch the registered `access_policy` by id, or `None`.
    pub fn get_access_policy(&self, policy_id: &str) -> Option<&AccessPolicy> {
        self.access_policies.get(policy_id)
    }

    /// Ids of every registered `access_policies`.
    pub fn list_access_policies(&self) -> Vec<String> {
        self.access_policies.keys().cloned().collect()
    }
}

impl AuthenticationSystem {
    /// Create a `AuthenticationSystem` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            authentication_methods: HashMap::new(),
            session_management: SessionManagement::new(),
            multi_factor: MultiFactorAuthentication::new(),
        }
    }

    /// Initialize owned sub-components in dependency order.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    /// Register `authentication_method` (keyed by its id field).
    pub fn add_authentication_method(&mut self, method: AuthenticationMethod) {
        self.authentication_methods
            .insert(method.method_id.clone(), method);
    }

    /// Fetch the registered `authentication_method` by id, or `None`.
    pub fn get_authentication_method(&self, method_id: &str) -> Option<&AuthenticationMethod> {
        self.authentication_methods.get(method_id)
    }

    /// Borrow the `SessionManagement` sub-component.
    pub fn session_management(&self) -> &SessionManagement {
        &self.session_management
    }

    /// Borrow the `MultiFactorAuthentication` sub-component.
    pub fn multi_factor(&self) -> &MultiFactorAuthentication {
        &self.multi_factor
    }
}

impl SessionManagement {
    /// Create a `SessionManagement` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            session_policies: HashMap::new(),
        }
    }

    /// Register `session` (keyed by its id field).
    pub fn add_session(&mut self, session: Session) {
        self.sessions.insert(session.session_id.clone(), session);
    }

    /// Fetch the registered `session` by id, or `None`.
    pub fn get_session(&self, session_id: &str) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    /// Remove `session` by id, returning it when present.
    pub fn remove_session(&mut self, session_id: &str) -> Option<Session> {
        self.sessions.remove(session_id)
    }

    /// Register `session_policy` (keyed by its id field).
    pub fn add_session_policy(&mut self, policy: SessionPolicy) {
        self.session_policies
            .insert(policy.policy_id.clone(), policy);
    }

    /// Fetch the registered `session_policy` by id, or `None`.
    pub fn get_session_policy(&self, policy_id: &str) -> Option<&SessionPolicy> {
        self.session_policies.get(policy_id)
    }
}

impl MultiFactorAuthentication {
    /// Create a `MultiFactorAuthentication` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            factors: HashMap::new(),
            factor_combinations: HashMap::new(),
        }
    }

    /// Register `factor` (keyed by its id field).
    pub fn add_factor(&mut self, factor: AuthenticationFactor) {
        self.factors.insert(factor.factor_id.clone(), factor);
    }

    /// Fetch the registered `factor` by id, or `None`.
    pub fn get_factor(&self, factor_id: &str) -> Option<&AuthenticationFactor> {
        self.factors.get(factor_id)
    }

    /// Register `factor_combination` (keyed by its id field).
    pub fn add_factor_combination(&mut self, combination: FactorCombination) {
        self.factor_combinations
            .insert(combination.combination_id.clone(), combination);
    }

    /// Fetch the registered `factor_combination` by id, or `None`.
    pub fn get_factor_combination(&self, combination_id: &str) -> Option<&FactorCombination> {
        self.factor_combinations.get(combination_id)
    }
}

impl AuthorizationSystem {
    /// Create a `AuthorizationSystem` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            authorization_policies: HashMap::new(),
            permission_management: PermissionManagement::new(),
            role_management: RoleManagement::new(),
        }
    }

    /// Initialize owned sub-components in dependency order.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    /// Register `authorization_policy` (keyed by its id field).
    pub fn add_authorization_policy(&mut self, policy: AuthorizationPolicy) {
        self.authorization_policies
            .insert(policy.policy_id.clone(), policy);
    }

    /// Fetch the registered `authorization_policy` by id, or `None`.
    pub fn get_authorization_policy(&self, policy_id: &str) -> Option<&AuthorizationPolicy> {
        self.authorization_policies.get(policy_id)
    }

    /// Borrow the `PermissionManagement` sub-component.
    pub fn permission_management(&self) -> &PermissionManagement {
        &self.permission_management
    }

    /// Borrow the `RoleManagement` sub-component.
    pub fn role_management(&self) -> &RoleManagement {
        &self.role_management
    }
}

impl PermissionManagement {
    /// Create a `PermissionManagement` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            permissions: HashMap::new(),
            permission_groups: HashMap::new(),
        }
    }

    /// Register `permission` (keyed by its id field).
    pub fn add_permission(&mut self, name: &str, permission: Permission) {
        self.permissions.insert(name.to_string(), permission);
    }

    /// Fetch the registered `permission` by id, or `None`.
    pub fn get_permission(&self, name: &str) -> Option<&Permission> {
        self.permissions.get(name)
    }

    /// Register `permission_group` (keyed by its id field).
    pub fn add_permission_group(&mut self, group: PermissionGroup) {
        self.permission_groups.insert(group.group_id.clone(), group);
    }

    /// Fetch the registered `permission_group` by id, or `None`.
    pub fn get_permission_group(&self, group_id: &str) -> Option<&PermissionGroup> {
        self.permission_groups.get(group_id)
    }
}

impl RoleManagement {
    /// Create a `RoleManagement` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            roles: HashMap::new(),
            role_hierarchy: RoleHierarchy::new(),
        }
    }

    /// Register `role` (keyed by its id field).
    pub fn add_role(&mut self, role: Role) {
        self.roles.insert(role.role_id.clone(), role);
    }

    /// Fetch the registered `role` by id, or `None`.
    pub fn get_role(&self, role_id: &str) -> Option<&Role> {
        self.roles.get(role_id)
    }

    /// Borrow the `RoleHierarchy` sub-component.
    pub fn role_hierarchy(&self) -> &RoleHierarchy {
        &self.role_hierarchy
    }
}

impl RoleHierarchy {
    /// Create a `RoleHierarchy` with default sub-components / empty registries.
    pub fn new() -> Self {
        Self {
            hierarchy_id: "hierarchy_1".to_string(),
            parent_roles: Vec::new(),
            child_roles: Vec::new(),
        }
    }
}
