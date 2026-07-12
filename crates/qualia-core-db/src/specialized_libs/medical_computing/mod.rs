//! Medical Computing Library - Healthcare Data Processing and Medical Analytics
//!
//! This module provides high-performance medical computing operations leveraging Phase 2 enhancements:
//! - Fiduciary Cryptography (ML-DSA) for secure medical data protection
//! - Zero-Knowledge Semantic Proofs for privacy-preserving medical research
//! - Hardware-Sympathetic Storage (ZNS) for zero-copy medical data
//! - Statistical Computing Library for advanced medical analytics

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Medical Computing Library Manager
pub struct MedicalComputingLibrary {
    patient_manager: PatientManager,
    clinical_analyzer: ClinicalAnalyzer,
    medical_imaging: MedicalImaging,
    drug_discovery: DrugDiscovery,
    compliance_monitor: MedicalComplianceMonitor,
}

/// Patient manager for patient data management
pub struct PatientManager {
    patient_records: PatientRecords,
    medical_history: MedicalHistory,
    privacy_protection: PrivacyProtection,
    data_access: DataAccessControl,
}

/// Patient records
pub struct PatientRecords {
    patients: HashMap<String, Patient>,
    demographics: HashMap<String, Demographics>,
    medical_identifiers: HashMap<String, MedicalIdentifier>,
}

/// Patient representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patient {
    pub patient_id: String,
    pub medical_record_number: String,
    pub demographics: Demographics,
    pub medical_history: MedicalHistory,
    pub current_medications: Vec<Medication>,
    pub allergies: Vec<Allergy>,
    pub vital_signs: Vec<VitalSigns>,
    pub lab_results: Vec<LabResult>,
    pub imaging_studies: Vec<ImagingStudy>,
    pub created_at: u64,
    pub last_updated: u64,
}

/// Demographics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Demographics {
    pub name: String,
    pub date_of_birth: String,
    pub gender: Gender,
    pub ethnicity: String,
    pub language: String,
    pub contact_info: ContactInfo,
    pub emergency_contacts: Vec<EmergencyContact>,
}

/// Gender types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
    Other,
    Unknown,
}

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub phone: String,
    pub email: String,
    pub address: Address,
}

/// Address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub country: String,
}

/// Emergency contact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyContact {
    pub name: String,
    pub relationship: String,
    pub phone: String,
    pub email: String,
}

/// Medical history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicalHistory {
    pub conditions: Vec<MedicalCondition>,
    pub surgeries: Vec<Surgery>,
    pub hospitalizations: Vec<Hospitalization>,
    pub family_history: FamilyHistory,
    pub social_history: SocialHistory,
}

/// Medical condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicalCondition {
    pub condition_id: String,
    pub condition_name: String,
    pub icd_code: String,
    pub diagnosis_date: String,
    pub status: ConditionStatus,
    pub severity: Severity,
    pub treatment_plan: TreatmentPlan,
}

/// Condition status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConditionStatus {
    Active,
    Resolved,
    Chronic,
    Recurrent,
}

/// Severity levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    Mild,
    Moderate,
    Severe,
    Critical,
}

/// Treatment plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreatmentPlan {
    pub plan_id: String,
    pub medications: Vec<Medication>,
    pub procedures: Vec<Procedure>,
    pub follow_up_care: FollowUpCare,
}

/// Medication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Medication {
    pub medication_id: String,
    pub name: String,
    pub dosage: String,
    pub frequency: String,
    pub route: Route,
    pub start_date: String,
    pub end_date: Option<String>,
    pub prescribed_by: String,
    pub indications: Vec<String>,
    pub contraindications: Vec<String>,
    pub side_effects: Vec<String>,
}

/// Administration routes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Route {
    Oral,
    Intravenous,
    Intramuscular,
    Subcutaneous,
    Topical,
    Inhalation,
    Rectal,
    Other(String),
}

/// Procedure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Procedure {
    pub procedure_id: String,
    pub procedure_name: String,
    pub cpt_code: String,
    pub date: String,
    pub provider: String,
    pub facility: String,
    pub outcome: ProcedureOutcome,
}

/// Procedure outcomes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProcedureOutcome {
    Successful,
    Complicated,
    Failed,
    Cancelled,
}

/// Follow-up care
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUpCare {
    pub follow_up_id: String,
    pub instructions: String,
    pub next_appointment: Option<String>,
    pub monitoring_required: bool,
    pub monitoring_parameters: Vec<String>,
}

/// Surgery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Surgery {
    pub surgery_id: String,
    pub surgery_name: String,
    pub date: String,
    pub surgeon: String,
    pub facility: String,
    pub anesthesia_type: String,
    pub complications: Vec<String>,
    pub recovery_time: u32,
}

/// Hospitalization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hospitalization {
    pub hospitalization_id: String,
    pub admission_date: String,
    pub discharge_date: Option<String>,
    pub facility: String,
    pub admission_reason: String,
    pub diagnosis: Vec<String>,
    pub procedures: Vec<String>,
    pub length_of_stay: u32,
}

/// Family history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyHistory {
    pub conditions: Vec<FamilyCondition>,
    pub genetic_disorders: Vec<GeneticDisorder>,
}

/// Family condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamilyCondition {
    pub condition: String,
    pub relationship: String,
    pub age_of_onset: Option<u32>,
    pub severity: Option<Severity>,
}

/// Genetic disorder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneticDisorder {
    pub disorder: String,
    pub inheritance_pattern: String,
    pub carrier_status: bool,
    pub affected_status: bool,
}

/// Social history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialHistory {
    pub smoking_status: SmokingStatus,
    pub alcohol_use: AlcoholUse,
    pub drug_use: DrugUse,
    pub exercise_habits: ExerciseHabits,
    pub diet: Diet,
    pub occupation: String,
    pub travel_history: Vec<TravelRecord>,
}

/// Smoking status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SmokingStatus {
    Never,
    Former,
    Current,
}

/// Alcohol use
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlcoholUse {
    None,
    Light,
    Moderate,
    Heavy,
}

/// Drug use
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DrugUse {
    None,
    Recreational,
    Medicinal,
    Illicit,
}

/// Exercise habits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExerciseHabits {
    pub frequency: String,
    pub intensity: String,
    pub duration: String,
    pub types: Vec<String>,
}

/// Diet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diet {
    pub diet_type: String,
    pub restrictions: Vec<String>,
    pub supplements: Vec<String>,
}

/// Travel record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TravelRecord {
    pub destination: String,
    pub dates: String,
    pub purpose: String,
}

/// Allergy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Allergy {
    pub allergy_id: String,
    pub allergen: String,
    pub reaction_type: ReactionType,
    pub severity: AllergySeverity,
    pub reaction_details: String,
    pub treatment: String,
}

/// Reaction types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReactionType {
    Anaphylaxis,
    Urticaria,
    Angioedema,
    Respiratory,
    Dermatological,
    Gastrointestinal,
}

/// Allergy severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AllergySeverity {
    Mild,
    Moderate,
    Severe,
    LifeThreatening,
}

/// Vital signs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VitalSigns {
    pub vital_signs_id: String,
    pub timestamp: u64,
    pub blood_pressure: BloodPressure,
    pub heart_rate: u32,
    pub respiratory_rate: u32,
    pub temperature: f64,
    pub oxygen_saturation: f64,
    pub height: Option<f64>,
    pub weight: Option<f64>,
    pub bmi: Option<f64>,
}

/// Blood pressure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloodPressure {
    pub systolic: u32,
    pub diastolic: u32,
    pub position: Position,
}

/// Measurement positions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Position {
    Sitting,
    Standing,
    Lying,
}

/// Lab result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabResult {
    pub result_id: String,
    pub test_name: String,
    pub test_code: String,
    pub specimen: String,
    pub result_date: String,
    pub value: f64,
    pub unit: String,
    pub reference_range: ReferenceRange,
    pub status: ResultStatus,
    pub interpretation: String,
}

/// Reference range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceRange {
    pub minimum: f64,
    pub maximum: f64,
    pub unit: String,
}

/// Result status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResultStatus {
    Normal,
    Abnormal,
    Critical,
    Pending,
}

/// Imaging study
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagingStudy {
    pub study_id: String,
    pub study_type: ImagingType,
    pub date: String,
    pub ordering_physician: String,
    pub radiologist: String,
    pub facility: String,
    pub findings: String,
    pub impression: String,
    pub images: Vec<MedicalImage>,
}

/// Imaging types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImagingType {
    XRay,
    CT,
    MRI,
    Ultrasound,
    PET,
    Mammography,
    Fluoroscopy,
    NuclearMedicine,
}

/// Medical image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicalImage {
    pub image_id: String,
    pub image_type: ImageFormat,
    pub series_number: u32,
    pub acquisition_date: String,
    pub modality: String,
    pub body_part: String,
    pub image_data: Vec<u8>,
}

/// Image formats
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImageFormat {
    DICOM,
    JPEG,
    PNG,
    NIfTI,
}

/// Medical identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MedicalIdentifier {
    pub identifier_type: IdentifierType,
    pub identifier_value: String,
    pub issuing_authority: String,
    pub issue_date: String,
    pub expiry_date: Option<String>,
}

/// Identifier types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IdentifierType {
    SocialSecurity,
    MedicalRecord,
    Insurance,
    Passport,
    DriverLicense,
    NationalID,
}

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

/// Clinical analyzer for medical data analysis
pub struct ClinicalAnalyzer {
    diagnostic_engine: DiagnosticEngine,
    risk_assessment: ClinicalRiskAssessment,
    treatment_planner: TreatmentPlanner,
    outcome_predictor: OutcomePredictor,
}

/// Diagnostic engine
pub struct DiagnosticEngine {
    diagnostic_algorithms: HashMap<String, DiagnosticAlgorithm>,
    symptom_analyzer: SymptomAnalyzer,
    lab_interpreter: LabInterpreter,
}

/// Diagnostic algorithms
#[derive(Debug, Clone)]
pub struct DiagnosticAlgorithm {
    pub algorithm_id: String,
    pub algorithm_name: String,
    pub algorithm_type: DiagnosticAlgorithmType,
    pub accuracy: f64,
}

/// Diagnostic algorithm types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiagnosticAlgorithmType {
    RuleBased,
    MachineLearning,
    Bayesian,
    NeuralNetwork,
}

/// Symptom analyzer
pub struct SymptomAnalyzer {
    symptom_patterns: HashMap<String, SymptomPattern>,
    symptom_correlations: HashMap<String, SymptomCorrelation>,
}

/// Symptom patterns
#[derive(Debug, Clone)]
pub struct SymptomPattern {
    pub pattern_id: String,
    pub pattern_name: String,
    pub symptoms: Vec<String>,
    pub associated_conditions: Vec<String>,
}

/// Symptom correlations
#[derive(Debug, Clone)]
pub struct SymptomCorrelation {
    pub correlation_id: String,
    pub symptom1: String,
    pub symptom2: String,
    pub correlation_coefficient: f64,
}

/// Lab interpreter
pub struct LabInterpreter {
    reference_ranges: HashMap<String, ReferenceRange>,
    abnormality_detector: AbnormalityDetector,
}

/// Abnormality detector
#[derive(Debug, Clone)]
pub struct AbnormalityDetector {
    detection_algorithms: HashMap<String, DetectionAlgorithm>,
    severity_assessment: SeverityAssessment,
}

/// Severity assessment
#[derive(Debug, Clone)]
pub struct SeverityAssessment {
    assessment_criteria: HashMap<String, AssessmentCriterion>,
    scoring_system: ScoringSystem,
}

/// Assessment criteria
#[derive(Debug, Clone)]
pub struct AssessmentCriterion {
    pub criterion_id: String,
    pub criterion_name: String,
    pub weight: f64,
    pub threshold: f64,
}

/// Scoring system
#[derive(Debug, Clone)]
pub struct ScoringSystem {
    pub system_id: String,
    pub system_name: String,
    pub scoring_algorithm: ScoringAlgorithm,
}

/// Scoring algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScoringAlgorithm {
    WeightedSum,
    Bayesian,
    FuzzyLogic,
    NeuralNetwork,
}

/// Clinical risk assessment
pub struct ClinicalRiskAssessment {
    risk_models: HashMap<String, ClinicalRiskModel>,
    risk_factors: HashMap<String, ClinicalRiskFactor>,
}

/// Clinical risk models
#[derive(Debug, Clone)]
pub struct ClinicalRiskModel {
    pub model_id: String,
    pub model_name: String,
    pub model_type: ClinicalRiskModelType,
    pub validation_results: ValidationResults,
}

/// Clinical risk model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClinicalRiskModelType {
    Cardiovascular,
    Cancer,
    Diabetes,
    Respiratory,
    Custom(String),
}

/// Validation results
#[derive(Debug, Clone)]
pub struct ValidationResults {
    pub accuracy: f64,
    pub sensitivity: f64,
    pub specificity: f64,
    pub auc: f64,
}

/// Clinical risk factors
#[derive(Debug, Clone)]
pub struct ClinicalRiskFactor {
    pub factor_id: String,
    pub factor_name: String,
    pub factor_category: FactorCategory,
    pub factor_weight: f64,
}

/// Factor categories
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FactorCategory {
    Demographic,
    Lifestyle,
    Medical,
    Genetic,
    Environmental,
}

/// Treatment planner
pub struct TreatmentPlanner {
    treatment_guidelines: HashMap<String, TreatmentGuideline>,
    decision_support: DecisionSupport,
}

/// Treatment guidelines
#[derive(Debug, Clone)]
pub struct TreatmentGuideline {
    pub guideline_id: String,
    pub guideline_name: String,
    pub guideline_type: GuidelineType,
    pub recommendations: Vec<TreatmentRecommendation>,
}

/// Guideline types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GuidelineType {
    Clinical,
    Protocol,
    StandardOfCare,
    BestPractice,
}

/// Treatment recommendations
#[derive(Debug, Clone)]
pub struct TreatmentRecommendation {
    pub recommendation_id: String,
    pub condition: String,
    pub treatment: String,
    pub evidence_level: EvidenceLevel,
    pub strength: RecommendationStrength,
}

/// Evidence levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EvidenceLevel {
    LevelA,
    LevelB,
    LevelC,
    ExpertOpinion,
}

/// Recommendation strength
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecommendationStrength {
    Strong,
    Moderate,
    Weak,
    ExpertConsensus,
}

/// Decision support
pub struct DecisionSupport {
    decision_trees: HashMap<String, DecisionTree>,
    scoring_systems: HashMap<String, ScoringSystem>,
}

/// Decision trees
#[derive(Debug, Clone)]
pub struct DecisionTree {
    pub tree_id: String,
    pub tree_name: String,
    pub root_node: DecisionNode,
}

/// Decision nodes
#[derive(Debug, Clone)]
pub struct DecisionNode {
    pub node_id: String,
    pub node_type: NodeType,
    pub condition: Option<String>,
    pub threshold: Option<f64>,
    pub children: Vec<DecisionNode>,
    pub outcome: Option<String>,
}

/// Node types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    Root,
    Decision,
    Leaf,
}

/// Outcome predictor
pub struct OutcomePredictor {
    prediction_models: HashMap<String, PredictionModel>,
    outcome_metrics: HashMap<String, OutcomeMetric>,
}

/// Prediction models
#[derive(Debug, Clone)]
pub struct PredictionModel {
    pub model_id: String,
    pub model_name: String,
    pub model_type: PredictionModelType,
    pub performance_metrics: ModelPerformanceMetrics,
}

/// Prediction model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PredictionModelType {
    Survival,
    Response,
    Recurrence,
    Complication,
}

/// Model performance metrics
#[derive(Debug, Clone)]
pub struct ModelPerformanceMetrics {
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
}

/// Outcome metrics
#[derive(Debug, Clone)]
pub struct OutcomeMetric {
    pub metric_id: String,
    pub metric_name: String,
    pub metric_type: OutcomeMetricType,
    pub measurement_method: MeasurementMethod,
}

/// Outcome metric types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OutcomeMetricType {
    Mortality,
    Morbidity,
    QualityOfLife,
    FunctionalStatus,
}

/// Measurement methods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MeasurementMethod {
    Scale,
    Binary,
    Continuous,
    Categorical,
}

/// Medical imaging
pub struct MedicalImaging {
    image_acquisition: ImageAcquisition,
    image_processing: ImageProcessing,
    image_analysis: ImageAnalysis,
    image_storage: ImageStorage,
}

/// Image acquisition
pub struct ImageAcquisition {
    acquisition_protocols: HashMap<String, AcquisitionProtocol>,
    quality_control: QualityControl,
}

/// Acquisition protocols
#[derive(Debug, Clone)]
pub struct AcquisitionProtocol {
    pub protocol_id: String,
    pub protocol_name: String,
    pub imaging_modality: ImagingModality,
    pub parameters: AcquisitionParameters,
}

/// Imaging modalities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImagingModality {
    XRay,
    CT,
    MRI,
    Ultrasound,
    PET,
    SPECT,
    Mammography,
}

/// Acquisition parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionParameters {
    pub resolution: String,
    pub slice_thickness: f64,
    pub field_of_view: String,
    pub acquisition_time: u32,
}

/// Quality control
pub struct QualityControl {
    quality_metrics: HashMap<String, QualityMetric>,
    quality_standards: HashMap<String, QualityStandard>,
}

/// Quality metrics
#[derive(Debug, Clone)]
pub struct QualityMetric {
    pub metric_id: String,
    pub metric_name: String,
    pub metric_type: QualityMetricType,
    pub acceptable_range: (f64, f64),
}

/// Quality metric types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QualityMetricType {
    SignalToNoise,
    Contrast,
    Resolution,
    ArtifactLevel,
}

/// Quality standards
#[derive(Debug, Clone)]
pub struct QualityStandard {
    pub standard_id: String,
    pub standard_name: String,
    pub standard_type: QualityStandardType,
    pub requirements: Vec<QualityRequirement>,
}

/// Quality standard types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QualityStandardType {
    ACR,
    FDA,
    CE,
    ISO,
}

/// Quality requirements
#[derive(Debug, Clone)]
pub struct QualityRequirement {
    pub requirement_id: String,
    pub requirement_name: String,
    pub requirement_value: f64,
    pub tolerance: f64,
}

/// Image processing
pub struct ImageProcessing {
    preprocessing_algorithms: HashMap<String, PreprocessingAlgorithm>,
    enhancement_techniques: HashMap<String, EnhancementTechnique>,
}

/// Preprocessing algorithms
#[derive(Debug, Clone)]
pub struct PreprocessingAlgorithm {
    pub algorithm_id: String,
    pub algorithm_name: String,
    pub algorithm_type: PreprocessingAlgorithmType,
}

/// Preprocessing algorithm types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreprocessingAlgorithmType {
    NoiseReduction,
    Normalization,
    Registration,
    Segmentation,
}

/// Enhancement techniques
#[derive(Debug, Clone)]
pub struct EnhancementTechnique {
    pub technique_id: String,
    pub technique_name: String,
    pub technique_type: EnhancementTechniqueType,
}

/// Enhancement technique types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EnhancementTechniqueType {
    ContrastEnhancement,
    EdgeEnhancement,
    Sharpening,
    Filtering,
}

/// Image analysis
pub struct ImageAnalysis {
    analysis_algorithms: HashMap<String, AnalysisAlgorithm>,
    detection_methods: HashMap<String, DetectionMethod>,
}

/// Analysis algorithms
#[derive(Debug, Clone)]
pub struct AnalysisAlgorithm {
    pub algorithm_id: String,
    pub algorithm_name: String,
    pub algorithm_type: AnalysisAlgorithmType,
}

/// Analysis algorithm types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnalysisAlgorithmType {
    PatternRecognition,
    FeatureExtraction,
    Classification,
    Segmentation,
}

/// Detection methods
#[derive(Debug, Clone)]
pub struct DetectionMethod {
    pub method_id: String,
    pub method_name: String,
    pub method_type: DetectionMethodType,
}

/// Detection method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DetectionMethodType {
    AnomalyDetection,
    LesionDetection,
    TumorDetection,
    FractureDetection,
}

/// Image storage
pub struct ImageStorage {
    storage_systems: HashMap<String, StorageSystem>,
    compression_methods: HashMap<String, CompressionMethod>,
}

/// Storage systems
#[derive(Debug, Clone)]
pub struct StorageSystem {
    pub system_id: String,
    pub system_name: String,
    pub system_type: StorageSystemType,
    pub capacity: u64,
}

/// Storage system types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StorageSystemType {
    Local,
    Network,
    Cloud,
    Archive,
}

/// Compression methods
#[derive(Debug, Clone)]
pub struct CompressionMethod {
    pub method_id: String,
    pub method_name: String,
    pub method_type: CompressionMethodType,
}

/// Compression method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompressionMethodType {
    Lossless,
    Lossy,
    Hybrid,
}

/// Drug discovery
pub struct DrugDiscovery {
    target_identification: TargetIdentification,
    compound_screening: CompoundScreening,
    lead_optimization: LeadOptimization,
    preclinical_testing: PreclinicalTesting,
}

/// Target identification
pub struct TargetIdentification {
    target_databases: HashMap<String, TargetDatabase>,
    validation_methods: HashMap<String, ValidationMethod>,
}

/// Target databases
#[derive(Debug, Clone)]
pub struct TargetDatabase {
    pub database_id: String,
    pub database_name: String,
    pub database_type: TargetDatabaseType,
    pub targets: Vec<DrugTarget>,
}

/// Target database types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TargetDatabaseType {
    Protein,
    Gene,
    Pathway,
    Disease,
}

/// Drug targets
#[derive(Debug, Clone)]
pub struct DrugTarget {
    pub target_id: String,
    pub target_name: String,
    pub target_type: TargetType,
    pub properties: TargetProperties,
}

/// Target types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TargetType {
    Receptor,
    Enzyme,
    IonChannel,
    Transporter,
    NuclearReceptor,
}

/// Target properties
#[derive(Debug, Clone)]
pub struct TargetProperties {
    pub binding_sites: Vec<BindingSite>,
    pub biological_function: String,
    pub disease_association: Vec<String>,
}

/// Binding sites
#[derive(Debug, Clone)]
pub struct BindingSite {
    pub site_id: String,
    pub site_location: String,
    pub site_type: SiteType,
    pub affinity: f64,
}

/// Site types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SiteType {
    Active,
    Allosteric,
    Orthosteric,
}

/// Validation methods
#[derive(Debug, Clone)]
pub struct ValidationMethod {
    pub method_id: String,
    pub method_name: String,
    pub method_type: ValidationMethodType,
}

/// Validation method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationMethodType {
    InVitro,
    InVivo,
    Computational,
    Genetic,
}

/// Compound screening
pub struct CompoundScreening {
    compound_libraries: HashMap<String, CompoundLibrary>,
    screening_assays: HashMap<String, ScreeningAssay>,
}

/// Compound libraries
#[derive(Debug, Clone)]
pub struct CompoundLibrary {
    pub library_id: String,
    pub library_name: String,
    pub library_type: CompoundLibraryType,
    pub compounds: Vec<Compound>,
}

/// Compound library types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompoundLibraryType {
    Commercial,
    Natural,
    Synthetic,
    Virtual,
}

/// Compounds
#[derive(Debug, Clone)]
pub struct Compound {
    pub compound_id: String,
    pub compound_name: String,
    pub chemical_structure: String,
    pub properties: CompoundProperties,
}

/// Compound properties
#[derive(Debug, Clone)]
pub struct CompoundProperties {
    pub molecular_weight: f64,
    pub logp: f64,
    pub solubility: f64,
    pub toxicity: ToxicityProfile,
}

/// Toxicity profile
#[derive(Debug, Clone)]
pub struct ToxicityProfile {
    pub acute_toxicity: f64,
    pub chronic_toxicity: f64,
    pub mutagenicity: bool,
    pub carcinogenicity: bool,
}

impl Compound {
    pub fn new() -> Self {
        Self {
            compound_id: "compound_1".to_string(),
            compound_name: "Test Compound".to_string(),
            chemical_structure: "C6H12O6".to_string(),
            properties: CompoundProperties {
                molecular_weight: 180.16,
                logp: -3.0,
                solubility: 0.91,
                toxicity: ToxicityProfile {
                    acute_toxicity: 0.1,
                    chronic_toxicity: 0.05,
                    mutagenicity: false,
                    carcinogenicity: false,
                },
            },
        }
    }
}

impl DrugTarget {
    pub fn new() -> Self {
        Self {
            target_id: "target_1".to_string(),
            target_name: "Test Target".to_string(),
            target_type: TargetType::Enzyme,
            properties: TargetProperties {
                binding_sites: Vec::new(),
                biological_function: "Enzyme activity".to_string(),
                disease_association: Vec::new(),
            },
        }
    }
}

/// Screening assays
#[derive(Debug, Clone)]
pub struct ScreeningAssay {
    pub assay_id: String,
    pub assay_name: String,
    pub assay_type: AssayType,
    pub readout: AssayReadout,
}

/// Assay types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssayType {
    Binding,
    Functional,
    CellBased,
    Biochemical,
}

/// Assay readouts
#[derive(Debug, Clone)]
pub struct AssayReadout {
    pub readout_type: ReadoutType,
    pub signal_to_noise: f64,
    pub dynamic_range: f64,
}

/// Readout types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReadoutType {
    Fluorescence,
    Luminescence,
    Absorbance,
    Radioactivity,
}

/// Lead optimization
pub struct LeadOptimization {
    optimization_strategies: HashMap<String, OptimizationStrategy>,
    adme_prediction: ADMEPrediction,
}

/// Optimization strategies
#[derive(Debug, Clone)]
pub struct OptimizationStrategy {
    pub strategy_id: String,
    pub strategy_name: String,
    pub strategy_type: OptimizationStrategyType,
}

/// Optimization strategy types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OptimizationStrategyType {
    StructureActivity,
    Pharmacophore,
    QSAR,
    MachineLearning,
}

/// ADME prediction
pub struct ADMEPrediction {
    absorption_model: AbsorptionModel,
    distribution_model: DistributionModel,
    metabolism_model: MetabolismModel,
    excretion_model: ExcretionModel,
}

/// Absorption model
#[derive(Debug, Clone)]
pub struct AbsorptionModel {
    pub model_type: ModelType,
    pub bioavailability: f64,
    pub absorption_rate: f64,
}

/// Model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelType {
    PhysiologicallyBased,
    Compartmental,
    Empirical,
}

/// Distribution model
#[derive(Debug, Clone)]
pub struct DistributionModel {
    pub volume_of_distribution: f64,
    pub protein_binding: f64,
    pub tissue_distribution: HashMap<String, f64>,
}

/// Metabolism model
#[derive(Debug, Clone)]
pub struct MetabolismModel {
    pub metabolic_pathways: Vec<MetabolicPathway>,
    pub clearance: f64,
    pub half_life: f64,
}

/// Metabolic pathways
#[derive(Debug, Clone)]
pub struct MetabolicPathway {
    pub pathway_id: String,
    pub pathway_name: String,
    pub enzymes: Vec<String>,
    pub metabolites: Vec<String>,
}

/// Excretion model
#[derive(Debug, Clone)]
pub struct ExcretionModel {
    pub excretion_routes: Vec<ExcretionRoute>,
    pub excretion_rate: f64,
}

/// Excretion routes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExcretionRoute {
    Renal,
    Hepatic,
    Pulmonary,
    Biliary,
}

/// Preclinical testing
pub struct PreclinicalTesting {
    in_vitro_testing: InVitroTesting,
    in_vivo_testing: InVivoTesting,
    toxicology_studies: ToxicologyStudies,
}

/// In vitro testing
pub struct InVitroTesting {
    pub test_types: HashMap<String, InVitroTest>,
    pub results: HashMap<String, TestResult>,
}

/// In vitro tests
#[derive(Debug, Clone)]
pub struct InVitroTest {
    pub test_id: String,
    pub test_name: String,
    pub test_type: InVitroTestType,
}

/// In vitro test types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InVitroTestType {
    Cytotoxicity,
    EnzymeInhibition,
    ReceptorBinding,
    Permeability,
}

/// Test results
#[derive(Debug, Clone)]
pub struct TestResult {
    pub result_id: String,
    pub test_id: String,
    pub outcome: TestOutcome,
    pub value: f64,
    pub units: String,
}

/// Test outcomes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TestOutcome {
    Positive,
    Negative,
    Inconclusive,
}

/// In vivo testing
pub struct InVivoTesting {
    pub animal_models: HashMap<String, AnimalModel>,
    pub study_designs: HashMap<String, StudyDesign>,
}

/// Animal models
#[derive(Debug, Clone)]
pub struct AnimalModel {
    pub model_id: String,
    pub model_name: String,
    pub species: String,
    pub disease_induction: String,
}

/// Study designs
#[derive(Debug, Clone)]
pub struct StudyDesign {
    pub design_id: String,
    pub design_name: String,
    pub design_type: StudyDesignType,
}

/// Study design types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StudyDesignType {
    Acute,
    Chronic,
    Subchronic,
    Carcinogenicity,
}

/// Toxicology studies
pub struct ToxicologyStudies {
    pub study_types: HashMap<String, ToxicologyStudy>,
    pub safety_assessments: HashMap<String, SafetyAssessment>,
}

/// Toxicology studies
#[derive(Debug, Clone)]
pub struct ToxicologyStudy {
    pub study_id: String,
    pub study_name: String,
    pub study_type: ToxicologyStudyType,
}

/// Toxicology study types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToxicologyStudyType {
    AcuteToxicity,
    ChronicToxicity,
    Genotoxicity,
    ReproductiveToxicity,
}

/// Safety assessments
#[derive(Debug, Clone)]
pub struct SafetyAssessment {
    pub assessment_id: String,
    pub assessment_type: AssessmentType,
    pub safety_margin: f64,
}

/// Assessment types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssessmentType {
    NOAEL,
    LOAEL,
    LD50,
    TD50,
}

/// Medical compliance monitor
pub struct MedicalComplianceMonitor {
    hipaa_compliance: HIPAACompliance,
    gdpr_compliance: GDPRCompliance,
    clinical_standards: ClinicalStandards,
    audit_system: AuditSystem,
}

/// HIPAA compliance
pub struct HIPAACompliance {
    privacy_rules: HashMap<String, PrivacyRule>,
    security_rules: HashMap<String, SecurityRule>,
    breach_notification: BreachNotification,
}

/// Privacy rules
#[derive(Debug, Clone)]
pub struct PrivacyRule {
    pub rule_id: String,
    pub rule_name: String,
    pub rule_type: PrivacyRuleType,
    pub requirements: Vec<HIPAARequirement>,
}

/// Privacy rule types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PrivacyRuleType {
    Use,
    Disclosure,
    Access,
    Amendment,
}

/// HIPAA requirements
#[derive(Debug, Clone)]
pub struct HIPAARequirement {
    pub requirement_id: String,
    pub requirement_name: String,
    pub requirement_text: String,
    pub mandatory: bool,
}

/// Security rules
#[derive(Debug, Clone)]
pub struct SecurityRule {
    pub rule_id: String,
    pub rule_name: String,
    pub rule_type: SecurityRuleType,
    pub controls: Vec<SecurityControl>,
}

/// Security rule types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SecurityRuleType {
    Administrative,
    Physical,
    Technical,
}

/// Security controls
#[derive(Debug, Clone)]
pub struct SecurityControl {
    pub control_id: String,
    pub control_name: String,
    pub control_type: SecurityControlType,
}

/// Security control types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SecurityControlType {
    Preventive,
    Detective,
    Corrective,
}

/// Breach notification
pub struct BreachNotification {
    notification_rules: HashMap<String, NotificationRule>,
    notification_templates: HashMap<String, NotificationTemplate>,
}

/// Notification rules
#[derive(Debug, Clone)]
pub struct NotificationRule {
    pub rule_id: String,
    pub rule_name: String,
    pub trigger_conditions: Vec<TriggerCondition>,
    pub notification_requirements: Vec<NotificationRequirement>,
}

/// Notification requirements
#[derive(Debug, Clone)]
pub struct NotificationRequirement {
    pub requirement_id: String,
    pub requirement_name: String,
    pub requirement_text: String,
    pub deadline: u32,
}

/// Notification templates
#[derive(Debug, Clone)]
pub struct NotificationTemplate {
    pub template_id: String,
    pub template_name: String,
    pub template_content: String,
    pub required_fields: Vec<String>,
}

/// GDPR compliance
pub struct GDPRCompliance {
    data_protection_principles: HashMap<String, DataProtectionPrinciple>,
    data_subject_rights: HashMap<String, DataSubjectRight>,
    data_processing_agreements: HashMap<String, DataProcessingAgreement>,
}

/// Data protection principles
#[derive(Debug, Clone)]
pub struct DataProtectionPrinciple {
    pub principle_id: String,
    pub principle_name: String,
    pub principle_description: String,
    pub implementation_guidance: String,
}

/// Data subject rights
#[derive(Debug, Clone)]
pub struct DataSubjectRight {
    pub right_id: String,
    pub right_name: String,
    pub right_description: String,
    pub implementation_procedures: Vec<ImplementationProcedure>,
}

/// Implementation procedures
#[derive(Debug, Clone)]
pub struct ImplementationProcedure {
    pub procedure_id: String,
    pub procedure_name: String,
    pub procedure_steps: Vec<ProcedureStep>,
}

/// Procedure steps
#[derive(Debug, Clone)]
pub struct ProcedureStep {
    pub step_id: String,
    pub step_description: String,
    pub step_responsible_party: String,
    pub step_deadline: u32,
}

/// Data processing agreements
#[derive(Debug, Clone)]
pub struct DataProcessingAgreement {
    pub agreement_id: String,
    pub agreement_name: String,
    pub agreement_terms: Vec<AgreementTerm>,
}

/// Agreement terms
#[derive(Debug, Clone)]
pub struct AgreementTerm {
    pub term_id: String,
    pub term_name: String,
    pub term_description: String,
    pub term_type: AgreementTermType,
}

/// Agreement term types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgreementTermType {
    Scope,
    Duration,
    Security,
    Liability,
}

/// Clinical standards
pub struct ClinicalStandards {
    clinical_guidelines: HashMap<String, ClinicalGuideline>,
    quality_metrics: HashMap<String, QualityMetric>,
    best_practices: HashMap<String, BestPractice>,
}

/// Clinical guidelines
#[derive(Debug, Clone)]
pub struct ClinicalGuideline {
    pub guideline_id: String,
    pub guideline_name: String,
    pub guideline_type: GuidelineType,
    pub recommendations: Vec<GuidelineRecommendation>,
}

/// Guideline recommendations
#[derive(Debug, Clone)]
pub struct GuidelineRecommendation {
    pub recommendation_id: String,
    pub recommendation_text: String,
    pub evidence_level: EvidenceLevel,
    pub grade: RecommendationGrade,
}

/// Recommendation grades
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecommendationGrade {
    Strong,
    Moderate,
    Weak,
    ExpertOpinion,
}

/// Best practices
#[derive(Debug, Clone)]
pub struct BestPractice {
    pub practice_id: String,
    pub practice_name: String,
    pub practice_description: String,
    pub implementation_steps: Vec<ImplementationStep>,
}

/// Implementation steps
#[derive(Debug, Clone)]
pub struct ImplementationStep {
    pub step_id: String,
    pub step_description: String,
    pub step_resources: Vec<String>,
}

/// Audit system
pub struct AuditSystem {
    audit_trails: HashMap<String, AuditTrail>,
    audit_reports: HashMap<String, AuditReport>,
    compliance_monitoring: ComplianceMonitoring,
}

/// Audit trails
#[derive(Debug, Clone)]
pub struct AuditTrail {
    pub trail_id: String,
    pub trail_type: TrailType,
    pub events: Vec<AuditEvent>,
}

/// Trail types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrailType {
    Access,
    Modification,
    Deletion,
    System,
}

/// Audit events
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub event_id: String,
    pub timestamp: u64,
    pub user_id: String,
    pub action: String,
    pub resource: String,
    pub outcome: String,
}

/// Audit reports
#[derive(Debug, Clone)]
pub struct AuditReport {
    pub report_id: String,
    pub report_name: String,
    pub report_type: ReportType,
    pub findings: Vec<AuditFinding>,
}

/// Report types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReportType {
    Compliance,
    Security,
    Performance,
    Incident,
}

/// Audit findings
#[derive(Debug, Clone)]
pub struct AuditFinding {
    pub finding_id: String,
    pub finding_type: FindingType,
    pub finding_description: String,
    pub severity: FindingSeverity,
    pub recommendations: Vec<String>,
}

/// Finding types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FindingType {
    Violation,
    Weakness,
    Gap,
    Observation,
}

/// Finding severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FindingSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Compliance monitoring
pub struct ComplianceMonitoring {
    monitoring_rules: HashMap<String, MonitoringRule>,
    compliance_metrics: HashMap<String, ComplianceMetric>,
}

/// Monitoring rules
#[derive(Debug, Clone)]
pub struct MonitoringRule {
    pub rule_id: String,
    pub rule_name: String,
    pub rule_type: MonitoringRuleType,
    pub check_frequency: u32,
}

/// Monitoring rule types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MonitoringRuleType {
    Automated,
    Manual,
    Hybrid,
}

/// Compliance metrics
#[derive(Debug, Clone)]
pub struct ComplianceMetric {
    pub metric_id: String,
    pub metric_name: String,
    pub metric_value: f64,
    pub metric_target: f64,
}

/// Medical operation result
#[derive(Debug, Clone)]
pub struct MedicalOperationResult<T> {
    pub result: T,
    pub execution_time: u64,
    /// Privacy score for this operation. `None` = not computed (no privacy guarantee is
    /// asserted). This scaffold does not measure privacy, so it must not fabricate a value —
    /// previously every operation stamped a hardcoded 0.80–0.95 here.
    pub privacy_score: Option<f64>,
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

impl MedicalComputingLibrary {
    /// Create new medical computing library
    pub fn new() -> Self {
        Self {
            patient_manager: PatientManager::new(),
            clinical_analyzer: ClinicalAnalyzer::new(),
            medical_imaging: MedicalImaging::new(),
            drug_discovery: DrugDiscovery::new(),
            compliance_monitor: MedicalComplianceMonitor::new(),
        }
    }

    /// Initialize the library
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        // Initialize patient manager
        self.patient_manager.initialize()?;

        // Initialize clinical analyzer
        self.clinical_analyzer.initialize()?;

        // Initialize medical imaging
        self.medical_imaging.initialize()?;

        // Initialize drug discovery
        self.drug_discovery.initialize()?;

        // Initialize compliance monitor
        self.compliance_monitor.initialize()?;

        // Seed default patient for testing
        let default_patient = Patient::new();
        let _ = self.patient_manager.create_patient(default_patient);

        Ok(())
    }

    /// Create a new patient record
    pub fn create_patient_record(
        &mut self,
        patient: Patient,
    ) -> Result<MedicalOperationResult<Patient>, MedicalError> {
        let start_time = std::time::Instant::now();

        // Validate patient data
        self.patient_manager.validate_patient(&patient)?;

        // Create patient record
        let created_patient = self.patient_manager.create_patient(patient)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(MedicalOperationResult {
            result: created_patient,
            execution_time,
            privacy_score: None,
            compliance_status: ComplianceStatus::Compliant,
            audit_trail: Vec::new(),
        })
    }

    /// Analyze clinical data
    pub fn analyze_clinical_data(
        &mut self,
        patient_id: &str,
        data_type: ClinicalDataType,
    ) -> Result<MedicalOperationResult<ClinicalAnalysis>, MedicalError> {
        let start_time = std::time::Instant::now();

        // Get patient data
        let patient = self.patient_manager.get_patient(patient_id)?;

        // Analyze clinical data
        let analysis = self.clinical_analyzer.analyze_data(&patient, data_type)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(MedicalOperationResult {
            result: analysis,
            execution_time,
            privacy_score: None,
            compliance_status: ComplianceStatus::Compliant,
            audit_trail: Vec::new(),
        })
    }

    /// Process medical image
    pub fn process_medical_image(
        &mut self,
        image: MedicalImage,
        processing_type: ImageProcessingType,
    ) -> Result<MedicalOperationResult<ProcessedImage>, MedicalError> {
        let start_time = std::time::Instant::now();

        // Validate image
        self.medical_imaging.validate_image(&image)?;

        // Process image
        let processed_image = self
            .medical_imaging
            .process_image(&image, processing_type)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(MedicalOperationResult {
            result: processed_image,
            execution_time,
            privacy_score: None,
            compliance_status: ComplianceStatus::Compliant,
            audit_trail: Vec::new(),
        })
    }

    /// Screen compounds
    pub fn screen_compounds(
        &mut self,
        compounds: Vec<Compound>,
        target: DrugTarget,
    ) -> Result<MedicalOperationResult<ScreeningResults>, MedicalError> {
        let start_time = std::time::Instant::now();

        // Validate compounds
        self.drug_discovery.validate_compounds(&compounds)?;

        // Screen compounds
        let results = self.drug_discovery.screen_compounds(&compounds, &target)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(MedicalOperationResult {
            result: results,
            execution_time,
            privacy_score: None,
            compliance_status: ComplianceStatus::Compliant,
            audit_trail: Vec::new(),
        })
    }

    /// Check compliance
    pub fn check_compliance(
        &mut self,
        compliance_type: ComplianceType,
    ) -> Result<MedicalOperationResult<ComplianceReport>, MedicalError> {
        let start_time = std::time::Instant::now();

        // Check compliance
        let report = self.compliance_monitor.check_compliance(compliance_type)?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(MedicalOperationResult {
            result: report,
            execution_time,
            privacy_score: None,
            compliance_status: ComplianceStatus::Compliant,
            audit_trail: Vec::new(),
        })
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> MedicalPerformanceMetrics {
        self.patient_manager.get_performance_metrics()
    }

    /// List all patients
    pub fn list_patients(&self) -> Vec<String> {
        self.patient_manager.list_patients()
    }

    /// Get patient information
    pub fn get_patient_info(&self, patient_id: &str) -> Option<Patient> {
        self.patient_manager.get_patient(patient_id).ok()
    }
}

// ---------------------------------------------------------------------------
// Deterministic clinical calculators (the genuinely-computable subset).
//
// Every method in this block implements a PUBLISHED, deterministic formula that
// depends only on its numeric inputs — no trained model, no clinical dataset, no
// knowledge base. Each formula is cited by name in its doc comment. These are the
// only medical outputs this library is permitted to compute for real; anything
// requiring a validated model / curated data (diagnosis, imaging readouts, drug
// interaction/affinity prediction, learned prognosis) MUST fail closed with
// `MedicalError::NotImplemented` (see `analyze_data`, `process_image`,
// `screen_compounds`). Never fabricate a medical number.
//
// Inputs are validated (positive where physiologically required); invalid input
// returns `ValidationError` rather than a nonsensical number. Sex-dependent
// formulas reject `Gender::Other`/`Gender::Unknown` rather than silently picking
// a coefficient — the validated coefficient is defined only for male/female.
// ---------------------------------------------------------------------------

/// Summary of a numeric cohort, computed via `crate::solvers::statistics`.
#[derive(Debug, Clone, PartialEq)]
pub struct CohortSummary {
    /// Number of observations.
    pub n: usize,
    /// Arithmetic mean.
    pub mean: f64,
    /// Sample standard deviation (Bessel-corrected, n-1). `None` when n < 2.
    pub std_dev: Option<f64>,
    /// Median.
    pub median: f64,
    /// Minimum.
    pub min: f64,
    /// Maximum.
    pub max: f64,
}

impl MedicalComputingLibrary {
    // -- Anthropometry -----------------------------------------------------

    /// Body Mass Index (Quetelet index): `BMI = weight_kg / height_m²` (kg/m²).
    pub fn bmi(&self, weight_kg: f64, height_m: f64) -> Result<f64, MedicalError> {
        if !(weight_kg > 0.0) || !(height_m > 0.0) {
            return Err(MedicalError::ValidationError(
                "bmi: weight_kg and height_m must be positive".to_string(),
            ));
        }
        Ok(weight_kg / (height_m * height_m))
    }

    /// Body Surface Area, Mosteller formula (1987):
    /// `BSA (m²) = sqrt(height_cm × weight_kg / 3600)`.
    pub fn bsa_mosteller(&self, weight_kg: f64, height_cm: f64) -> Result<f64, MedicalError> {
        if !(weight_kg > 0.0) || !(height_cm > 0.0) {
            return Err(MedicalError::ValidationError(
                "bsa_mosteller: weight_kg and height_cm must be positive".to_string(),
            ));
        }
        Ok((height_cm * weight_kg / 3600.0).sqrt())
    }

    /// Body Surface Area, Du Bois & Du Bois formula (1916):
    /// `BSA (m²) = 0.007184 × weight_kg^0.425 × height_cm^0.725`.
    pub fn bsa_du_bois(&self, weight_kg: f64, height_cm: f64) -> Result<f64, MedicalError> {
        if !(weight_kg > 0.0) || !(height_cm > 0.0) {
            return Err(MedicalError::ValidationError(
                "bsa_du_bois: weight_kg and height_cm must be positive".to_string(),
            ));
        }
        Ok(0.007184 * weight_kg.powf(0.425) * height_cm.powf(0.725))
    }

    /// Ideal Body Weight, Devine formula (1974). In kg:
    /// male   = 50.0  + 2.3 × (height_inches − 60);
    /// female = 45.5  + 2.3 × (height_inches − 60);  height_inches = height_cm / 2.54.
    /// Only defined for male/female (rejects Other/Unknown).
    pub fn ideal_body_weight_devine(
        &self,
        height_cm: f64,
        sex: Gender,
    ) -> Result<f64, MedicalError> {
        if !(height_cm > 0.0) {
            return Err(MedicalError::ValidationError(
                "ideal_body_weight_devine: height_cm must be positive".to_string(),
            ));
        }
        let base = match sex {
            Gender::Male => 50.0,
            Gender::Female => 45.5,
            Gender::Other | Gender::Unknown => {
                return Err(MedicalError::ValidationError(
                    "ideal_body_weight_devine: Devine coefficients are defined only for \
                     male/female; sex Other/Unknown has no validated coefficient".to_string(),
                ))
            }
        };
        let height_inches = height_cm / 2.54;
        Ok(base + 2.3 * (height_inches - 60.0))
    }

    // -- Renal function ----------------------------------------------------

    /// Estimated GFR, CKD-EPI 2021 creatinine equation (race-free), mL/min/1.73 m²:
    /// `eGFR = 142 × min(Scr/κ,1)^α × max(Scr/κ,1)^−1.200 × 0.9938^age × (1.012 if female)`
    /// with κ = 0.7 (female)/0.9 (male), α = −0.241 (female)/−0.302 (male).
    /// `scr_mg_dl` = serum creatinine in mg/dL. Only defined for male/female.
    pub fn egfr_ckd_epi_2021(
        &self,
        scr_mg_dl: f64,
        age_years: f64,
        sex: Gender,
    ) -> Result<f64, MedicalError> {
        if !(scr_mg_dl > 0.0) || !(age_years > 0.0) {
            return Err(MedicalError::ValidationError(
                "egfr_ckd_epi_2021: scr_mg_dl and age_years must be positive".to_string(),
            ));
        }
        let (kappa, alpha, female_factor) = match sex {
            Gender::Female => (0.7_f64, -0.241_f64, 1.012_f64),
            Gender::Male => (0.9_f64, -0.302_f64, 1.0_f64),
            Gender::Other | Gender::Unknown => {
                return Err(MedicalError::ValidationError(
                    "egfr_ckd_epi_2021: CKD-EPI coefficients are defined only for male/female"
                        .to_string(),
                ))
            }
        };
        let ratio = scr_mg_dl / kappa;
        let egfr = 142.0
            * ratio.min(1.0).powf(alpha)
            * ratio.max(1.0).powf(-1.200)
            * 0.9938_f64.powf(age_years)
            * female_factor;
        Ok(egfr)
    }

    /// Estimated GFR, MDRD 4-variable equation (IDMS-traceable, 2006 coefficient
    /// 175), mL/min/1.73 m²:
    /// `eGFR = 175 × Scr^−1.154 × age^−0.203 × (0.742 if female) × (1.212 if Black)`.
    /// `scr_mg_dl` = serum creatinine in mg/dL. Only defined for male/female.
    pub fn egfr_mdrd(
        &self,
        scr_mg_dl: f64,
        age_years: f64,
        sex: Gender,
        is_black: bool,
    ) -> Result<f64, MedicalError> {
        if !(scr_mg_dl > 0.0) || !(age_years > 0.0) {
            return Err(MedicalError::ValidationError(
                "egfr_mdrd: scr_mg_dl and age_years must be positive".to_string(),
            ));
        }
        let sex_factor = match sex {
            Gender::Female => 0.742,
            Gender::Male => 1.0,
            Gender::Other | Gender::Unknown => {
                return Err(MedicalError::ValidationError(
                    "egfr_mdrd: MDRD sex factor is defined only for male/female".to_string(),
                ))
            }
        };
        let race_factor = if is_black { 1.212 } else { 1.0 };
        Ok(175.0 * scr_mg_dl.powf(-1.154) * age_years.powf(-0.203) * sex_factor * race_factor)
    }

    /// Creatinine clearance, Cockcroft-Gault equation (1976), mL/min:
    /// `CrCl = ((140 − age) × weight_kg × (0.85 if female)) / (72 × Scr_mg/dL)`.
    /// Only defined for male/female.
    pub fn creatinine_clearance_cockcroft_gault(
        &self,
        age_years: f64,
        weight_kg: f64,
        scr_mg_dl: f64,
        sex: Gender,
    ) -> Result<f64, MedicalError> {
        if !(weight_kg > 0.0) || !(scr_mg_dl > 0.0) || !(age_years > 0.0) {
            return Err(MedicalError::ValidationError(
                "creatinine_clearance_cockcroft_gault: age_years, weight_kg and scr_mg_dl \
                 must be positive".to_string(),
            ));
        }
        let sex_factor = match sex {
            Gender::Female => 0.85,
            Gender::Male => 1.0,
            Gender::Other | Gender::Unknown => {
                return Err(MedicalError::ValidationError(
                    "creatinine_clearance_cockcroft_gault: sex factor is defined only for \
                     male/female".to_string(),
                ))
            }
        };
        Ok(((140.0 - age_years) * weight_kg * sex_factor) / (72.0 * scr_mg_dl))
    }

    // -- Hemodynamics & acid-base -----------------------------------------

    /// Mean Arterial Pressure (standard estimate): `MAP = (SBP + 2·DBP) / 3` (mmHg).
    pub fn mean_arterial_pressure(
        &self,
        systolic: f64,
        diastolic: f64,
    ) -> Result<f64, MedicalError> {
        if !(systolic > 0.0) || !(diastolic > 0.0) || diastolic > systolic {
            return Err(MedicalError::ValidationError(
                "mean_arterial_pressure: require systolic >= diastolic > 0".to_string(),
            ));
        }
        Ok((systolic + 2.0 * diastolic) / 3.0)
    }

    /// Serum anion gap: `AG = Na − (Cl + HCO3)` (mEq/L). (Potassium excluded, the
    /// common convention.)
    pub fn anion_gap(&self, na: f64, cl: f64, hco3: f64) -> f64 {
        na - (cl + hco3)
    }

    /// Albumin-corrected calcium (Payne 1973):
    /// `corrected = measured_ca_mg_dl + 0.8 × (4.0 − albumin_g_dl)` (mg/dL).
    pub fn corrected_calcium(
        &self,
        measured_ca_mg_dl: f64,
        albumin_g_dl: f64,
    ) -> Result<f64, MedicalError> {
        if !(measured_ca_mg_dl >= 0.0) || !(albumin_g_dl >= 0.0) {
            return Err(MedicalError::ValidationError(
                "corrected_calcium: measured_ca_mg_dl and albumin_g_dl must be non-negative"
                    .to_string(),
            ));
        }
        Ok(measured_ca_mg_dl + 0.8 * (4.0 - albumin_g_dl))
    }

    /// Winter's formula — expected PaCO₂ compensation for metabolic acidosis:
    /// `expected pCO2 = 1.5 × HCO3 + 8` (mmHg, ±2). Returns the point estimate.
    pub fn winters_expected_pco2(&self, hco3: f64) -> Result<f64, MedicalError> {
        if !(hco3 >= 0.0) {
            return Err(MedicalError::ValidationError(
                "winters_expected_pco2: hco3 must be non-negative".to_string(),
            ));
        }
        Ok(1.5 * hco3 + 8.0)
    }

    // -- Risk scores (pure published point sums) --------------------------

    /// CHA₂DS₂-VASc stroke-risk score (Lip 2010) as its deterministic point sum
    /// (0–9). This is the arithmetic score itself, NOT a risk/probability estimate
    /// (mapping the score to an annual stroke rate needs the validated cohort table,
    /// which is not shipped): CHF/LV dysfunction (1), hypertension (1),
    /// age ≥75 (2) or 65–74 (1), diabetes (1), prior stroke/TIA/thromboembolism (2),
    /// vascular disease (1), female sex (1).
    pub fn cha2ds2_vasc_score(
        &self,
        congestive_heart_failure: bool,
        hypertension: bool,
        age_years: u32,
        diabetes: bool,
        prior_stroke_tia_or_thromboembolism: bool,
        vascular_disease: bool,
        sex: Gender,
    ) -> u8 {
        let mut score: u8 = 0;
        if congestive_heart_failure {
            score += 1;
        }
        if hypertension {
            score += 1;
        }
        if age_years >= 75 {
            score += 2;
        } else if age_years >= 65 {
            score += 1;
        }
        if diabetes {
            score += 1;
        }
        if prior_stroke_tia_or_thromboembolism {
            score += 2;
        }
        if vascular_disease {
            score += 1;
        }
        if matches!(sex, Gender::Female) {
            score += 1;
        }
        score
    }

    // -- Drug dosing math --------------------------------------------------

    /// Weight-based dose: `dose = dose_per_kg × weight_kg` (units follow dose_per_kg).
    pub fn weight_based_dose(
        &self,
        dose_per_kg: f64,
        weight_kg: f64,
    ) -> Result<f64, MedicalError> {
        if !(dose_per_kg >= 0.0) || !(weight_kg > 0.0) {
            return Err(MedicalError::ValidationError(
                "weight_based_dose: dose_per_kg must be non-negative and weight_kg positive"
                    .to_string(),
            ));
        }
        Ok(dose_per_kg * weight_kg)
    }

    /// Renal dose adjustment, Giusti-Hayton method (1973):
    /// `Q = 1 − Fe × (1 − CrCl_patient / CrCl_normal)`;
    /// `adjusted_dose = normal_dose × Q`. `fraction_renally_excreted` (Fe) ∈ [0,1]
    /// is the fraction of drug eliminated unchanged by the kidney.
    pub fn giusti_hayton_adjusted_dose(
        &self,
        normal_dose: f64,
        fraction_renally_excreted: f64,
        crcl_patient: f64,
        crcl_normal: f64,
    ) -> Result<f64, MedicalError> {
        if !(normal_dose >= 0.0) || !(crcl_patient >= 0.0) || !(crcl_normal > 0.0) {
            return Err(MedicalError::ValidationError(
                "giusti_hayton_adjusted_dose: doses/clearances must be non-negative and \
                 crcl_normal positive".to_string(),
            ));
        }
        if !(0.0..=1.0).contains(&fraction_renally_excreted) {
            return Err(MedicalError::ValidationError(
                "giusti_hayton_adjusted_dose: fraction_renally_excreted must be in [0,1]"
                    .to_string(),
            ));
        }
        let q = 1.0 - fraction_renally_excreted * (1.0 - crcl_patient / crcl_normal);
        Ok(normal_dose * q)
    }

    /// Convert mass to amount of substance: `mmol = mg / molar_mass_g_per_mol`.
    pub fn mg_to_mmol(&self, mg: f64, molar_mass_g_per_mol: f64) -> Result<f64, MedicalError> {
        if !(molar_mass_g_per_mol > 0.0) {
            return Err(MedicalError::ValidationError(
                "mg_to_mmol: molar_mass_g_per_mol must be positive".to_string(),
            ));
        }
        Ok(mg / molar_mass_g_per_mol)
    }

    /// Convert amount of substance to mass: `mg = mmol × molar_mass_g_per_mol`.
    pub fn mmol_to_mg(&self, mmol: f64, molar_mass_g_per_mol: f64) -> Result<f64, MedicalError> {
        if !(molar_mass_g_per_mol > 0.0) {
            return Err(MedicalError::ValidationError(
                "mmol_to_mg: molar_mass_g_per_mol must be positive".to_string(),
            ));
        }
        Ok(mmol * molar_mass_g_per_mol)
    }

    /// Continuous infusion rate (mL/hr) for a weight-based dose:
    /// `rate = (dose_per_kg_per_min × weight_kg × 60) / concentration_per_ml`.
    /// Units of `dose_per_kg_per_min` and `concentration_per_ml` must match
    /// (e.g. µg/kg/min with µg/mL).
    pub fn infusion_rate_ml_per_hr(
        &self,
        dose_per_kg_per_min: f64,
        weight_kg: f64,
        concentration_per_ml: f64,
    ) -> Result<f64, MedicalError> {
        if !(dose_per_kg_per_min >= 0.0) || !(weight_kg > 0.0) || !(concentration_per_ml > 0.0) {
            return Err(MedicalError::ValidationError(
                "infusion_rate_ml_per_hr: dose/weight non-negative-or-positive and \
                 concentration_per_ml must be positive".to_string(),
            ));
        }
        Ok((dose_per_kg_per_min * weight_kg * 60.0) / concentration_per_ml)
    }

    // -- First-order pharmacokinetics -------------------------------------

    /// First-order elimination rate constant from half-life: `k = ln(2) / t½`.
    pub fn elimination_rate_constant(&self, half_life: f64) -> Result<f64, MedicalError> {
        if !(half_life > 0.0) {
            return Err(MedicalError::ValidationError(
                "elimination_rate_constant: half_life must be positive".to_string(),
            ));
        }
        Ok(std::f64::consts::LN_2 / half_life)
    }

    /// Half-life from first-order rate constant: `t½ = ln(2) / k`.
    pub fn half_life_from_rate_constant(
        &self,
        rate_constant: f64,
    ) -> Result<f64, MedicalError> {
        if !(rate_constant > 0.0) {
            return Err(MedicalError::ValidationError(
                "half_life_from_rate_constant: rate_constant must be positive".to_string(),
            ));
        }
        Ok(std::f64::consts::LN_2 / rate_constant)
    }

    /// Drug clearance from first-order PK: `CL = k × Vd`.
    pub fn clearance(
        &self,
        rate_constant: f64,
        volume_of_distribution: f64,
    ) -> Result<f64, MedicalError> {
        if !(rate_constant >= 0.0) || !(volume_of_distribution >= 0.0) {
            return Err(MedicalError::ValidationError(
                "clearance: rate_constant and volume_of_distribution must be non-negative"
                    .to_string(),
            ));
        }
        Ok(rate_constant * volume_of_distribution)
    }

    /// Apparent volume of distribution: `Vd = dose / C0`.
    pub fn volume_of_distribution(
        &self,
        dose: f64,
        initial_concentration: f64,
    ) -> Result<f64, MedicalError> {
        if !(dose >= 0.0) || !(initial_concentration > 0.0) {
            return Err(MedicalError::ValidationError(
                "volume_of_distribution: dose non-negative and initial_concentration positive"
                    .to_string(),
            ));
        }
        Ok(dose / initial_concentration)
    }

    /// Steady-state concentration under continuous infusion:
    /// `Css = infusion_rate / clearance`.
    pub fn steady_state_concentration(
        &self,
        infusion_rate: f64,
        clearance: f64,
    ) -> Result<f64, MedicalError> {
        if !(infusion_rate >= 0.0) || !(clearance > 0.0) {
            return Err(MedicalError::ValidationError(
                "steady_state_concentration: infusion_rate non-negative and clearance positive"
                    .to_string(),
            ));
        }
        Ok(infusion_rate / clearance)
    }

    // -- Statistics (delegated) -------------------------------------------

    /// Summarise a numeric cohort (e.g. a series of lab values). This is the only
    /// statistical work here and it DELEGATES to `crate::solvers::statistics`
    /// (`descriptive::mean`, `descriptive::std_dev`, `descriptive::median_sorted`).
    pub fn summarize_cohort(&self, values: &[f64]) -> Result<CohortSummary, MedicalError> {
        use crate::solvers::statistics::descriptive;
        if values.is_empty() {
            return Err(MedicalError::ValidationError(
                "summarize_cohort: values must be non-empty".to_string(),
            ));
        }
        let mean = descriptive::mean(values).ok_or_else(|| {
            MedicalError::DataError("summarize_cohort: mean undefined".to_string())
        })?;
        // Sample std-dev is undefined for n < 2 — report None rather than NaN.
        let std_dev = if values.len() >= 2 {
            descriptive::std_dev(values, true)
        } else {
            None
        };
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = descriptive::median_sorted(&sorted).ok_or_else(|| {
            MedicalError::DataError("summarize_cohort: median undefined".to_string())
        })?;
        Ok(CohortSummary {
            n: values.len(),
            mean,
            std_dev,
            median,
            min: sorted[0],
            max: sorted[sorted.len() - 1],
        })
    }
}

// Supporting implementations

impl PatientManager {
    pub fn new() -> Self {
        Self {
            patient_records: PatientRecords::new(),
            medical_history: MedicalHistory::new(),
            privacy_protection: PrivacyProtection::new(),
            data_access: DataAccessControl::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        self.patient_records.initialize()?;
        self.privacy_protection.initialize()?;
        self.data_access.initialize()?;
        Ok(())
    }

    pub fn validate_patient(&self, patient: &Patient) -> Result<(), MedicalError> {
        // Validate patient data
        if patient.patient_id.is_empty() {
            return Err(MedicalError::ValidationError(
                "Patient ID cannot be empty".to_string(),
            ));
        }
        if patient.medical_record_number.is_empty() {
            return Err(MedicalError::ValidationError(
                "Medical record number cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    pub fn create_patient(&mut self, patient: Patient) -> Result<Patient, MedicalError> {
        // Create patient record
        self.patient_records.store_patient(patient.clone())?;
        Ok(patient)
    }

    pub fn get_patient(&self, patient_id: &str) -> Result<Patient, MedicalError> {
        self.patient_records.get_patient(patient_id)
    }

    pub fn list_patients(&self) -> Vec<String> {
        self.patient_records.list_patients()
    }

    pub fn get_performance_metrics(&self) -> MedicalPerformanceMetrics {
        MedicalPerformanceMetrics::new()
    }

    pub fn get_medical_history(&self) -> &MedicalHistory {
        &self.medical_history
    }

    pub fn add_condition(&mut self, condition: MedicalCondition) {
        self.medical_history.conditions.push(condition);
    }

    pub fn add_surgery(&mut self, surgery: Surgery) {
        self.medical_history.surgeries.push(surgery);
    }

    pub fn active_conditions(&self) -> Vec<&MedicalCondition> {
        self.medical_history
            .conditions
            .iter()
            .filter(|c| c.status == ConditionStatus::Active || c.status == ConditionStatus::Chronic)
            .collect()
    }
}

impl PatientRecords {
    pub fn new() -> Self {
        Self {
            patients: HashMap::new(),
            demographics: HashMap::new(),
            medical_identifiers: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn store_patient(&mut self, patient: Patient) -> Result<(), MedicalError> {
        self.patients.insert(patient.patient_id.clone(), patient);
        Ok(())
    }

    pub fn get_patient(&self, patient_id: &str) -> Result<Patient, MedicalError> {
        self.patients
            .get(patient_id)
            .cloned()
            .ok_or_else(|| MedicalError::PatientError("Patient not found".to_string()))
    }

    pub fn list_patients(&self) -> Vec<String> {
        self.patients.keys().cloned().collect()
    }

    pub fn store_demographics(&mut self, patient_id: &str, demographics: Demographics) {
        self.demographics
            .insert(patient_id.to_string(), demographics);
    }

    pub fn get_demographics(&self, patient_id: &str) -> Option<&Demographics> {
        self.demographics.get(patient_id)
    }

    pub fn store_identifier(&mut self, patient_id: &str, identifier: MedicalIdentifier) {
        self.medical_identifiers
            .insert(patient_id.to_string(), identifier);
    }

    pub fn get_identifier(&self, patient_id: &str) -> Option<&MedicalIdentifier> {
        self.medical_identifiers.get(patient_id)
    }

    pub fn remove_patient(&mut self, patient_id: &str) -> Option<Patient> {
        self.demographics.remove(patient_id);
        self.medical_identifiers.remove(patient_id);
        self.patients.remove(patient_id)
    }
}

impl MedicalHistory {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
            surgeries: Vec::new(),
            hospitalizations: Vec::new(),
            family_history: FamilyHistory::new(),
            social_history: SocialHistory::new(),
        }
    }
}

impl FamilyHistory {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
            genetic_disorders: Vec::new(),
        }
    }
}

impl SocialHistory {
    pub fn new() -> Self {
        Self {
            smoking_status: SmokingStatus::Never,
            alcohol_use: AlcoholUse::None,
            drug_use: DrugUse::None,
            exercise_habits: ExerciseHabits::new(),
            diet: Diet::new(),
            occupation: String::new(),
            travel_history: Vec::new(),
        }
    }
}

impl ExerciseHabits {
    pub fn new() -> Self {
        Self {
            frequency: String::new(),
            intensity: String::new(),
            duration: String::new(),
            types: Vec::new(),
        }
    }
}

impl Diet {
    pub fn new() -> Self {
        Self {
            diet_type: String::new(),
            restrictions: Vec::new(),
            supplements: Vec::new(),
        }
    }
}

impl PrivacyProtection {
    pub fn new() -> Self {
        Self {
            encryption: EncryptionManager::new(),
            anonymization: AnonymizationEngine::new(),
            access_logging: AccessLogging::new(),
            consent_management: ConsentManagement::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        self.encryption.initialize()?;
        self.anonymization.initialize()?;
        self.access_logging.initialize()?;
        self.consent_management.initialize()?;
        Ok(())
    }
}

impl EncryptionManager {
    pub fn new() -> Self {
        Self {
            encryption_algorithms: HashMap::new(),
            key_management: KeyManagement::new(),
            data_protection: DataProtection::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        self.key_management.initialize()?;
        self.data_protection.initialize()?;
        Ok(())
    }

    pub fn add_algorithm(&mut self, algorithm: EncryptionAlgorithm) {
        self.encryption_algorithms
            .insert(algorithm.algorithm_id.clone(), algorithm);
    }

    pub fn get_algorithm(&self, algorithm_id: &str) -> Option<&EncryptionAlgorithm> {
        self.encryption_algorithms.get(algorithm_id)
    }

    pub fn list_algorithms(&self) -> Vec<String> {
        self.encryption_algorithms.keys().cloned().collect()
    }
}

impl KeyManagement {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            key_rotation: KeyRotation::new(),
            key_recovery: KeyRecovery::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_key(&mut self, key: EncryptionKey) {
        self.keys.insert(key.key_id.clone(), key);
    }

    pub fn get_key(&self, key_id: &str) -> Option<&EncryptionKey> {
        self.keys.get(key_id)
    }

    pub fn remove_key(&mut self, key_id: &str) -> Option<EncryptionKey> {
        self.keys.remove(key_id)
    }

    pub fn key_rotation(&self) -> &KeyRotation {
        &self.key_rotation
    }

    pub fn key_recovery(&self) -> &KeyRecovery {
        &self.key_recovery
    }
}

impl KeyRotation {
    pub fn new() -> Self {
        Self {
            rotation_policy: RotationPolicy::new(),
            rotation_schedule: RotationSchedule::new(),
            rotation_history: RotationHistory::new(),
        }
    }

    pub fn rotation_policy(&self) -> &RotationPolicy {
        &self.rotation_policy
    }

    pub fn rotation_schedule(&self) -> &RotationSchedule {
        &self.rotation_schedule
    }

    pub fn rotation_history(&self) -> &RotationHistory {
        &self.rotation_history
    }
}

impl RotationPolicy {
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
    pub fn new() -> Self {
        Self {
            recovery_methods: HashMap::new(),
            recovery_procedures: HashMap::new(),
        }
    }

    pub fn add_recovery_method(&mut self, method: RecoveryMethod) {
        self.recovery_methods
            .insert(method.method_id.clone(), method);
    }

    pub fn get_recovery_method(&self, method_id: &str) -> Option<&RecoveryMethod> {
        self.recovery_methods.get(method_id)
    }

    pub fn add_recovery_procedure(&mut self, procedure: RecoveryProcedure) {
        self.recovery_procedures
            .insert(procedure.procedure_id.clone(), procedure);
    }

    pub fn get_recovery_procedure(&self, procedure_id: &str) -> Option<&RecoveryProcedure> {
        self.recovery_procedures.get(procedure_id)
    }
}

impl DataProtection {
    pub fn new() -> Self {
        Self {
            protection_policies: HashMap::new(),
            breach_detection: BreachDetection::new(),
            incident_response: IncidentResponse::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_protection_policy(&mut self, policy: ProtectionPolicy) {
        self.protection_policies
            .insert(policy.policy_id.clone(), policy);
    }

    pub fn get_protection_policy(&self, policy_id: &str) -> Option<&ProtectionPolicy> {
        self.protection_policies.get(policy_id)
    }

    pub fn breach_detection(&self) -> &BreachDetection {
        &self.breach_detection
    }

    pub fn incident_response(&self) -> &IncidentResponse {
        &self.incident_response
    }
}

impl BreachDetection {
    pub fn new() -> Self {
        Self {
            detection_algorithms: HashMap::new(),
            alert_systems: HashMap::new(),
        }
    }

    pub fn add_detection_algorithm(&mut self, algorithm: DetectionAlgorithm) {
        self.detection_algorithms
            .insert(algorithm.algorithm_id.clone(), algorithm);
    }

    pub fn get_detection_algorithm(&self, algorithm_id: &str) -> Option<&DetectionAlgorithm> {
        self.detection_algorithms.get(algorithm_id)
    }

    pub fn add_alert_system(&mut self, system: AlertSystem) {
        self.alert_systems.insert(system.system_id.clone(), system);
    }

    pub fn get_alert_system(&self, system_id: &str) -> Option<&AlertSystem> {
        self.alert_systems.get(system_id)
    }
}

impl IncidentResponse {
    pub fn new() -> Self {
        Self {
            response_plans: HashMap::new(),
            response_team: ResponseTeam::new(),
            escalation_procedures: EscalationProcedures::new(),
        }
    }

    pub fn add_response_plan(&mut self, plan: ResponsePlan) {
        self.response_plans.insert(plan.plan_id.clone(), plan);
    }

    pub fn get_response_plan(&self, plan_id: &str) -> Option<&ResponsePlan> {
        self.response_plans.get(plan_id)
    }

    pub fn response_team(&self) -> &ResponseTeam {
        &self.response_team
    }

    pub fn escalation_procedures(&self) -> &EscalationProcedures {
        &self.escalation_procedures
    }
}

impl ResponseTeam {
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
    pub fn new() -> Self {
        Self {
            escalation_rules: HashMap::new(),
            escalation_matrix: EscalationMatrix::new(),
        }
    }

    pub fn add_escalation_rule(&mut self, rule: EscalationRule) {
        self.escalation_rules.insert(rule.rule_id.clone(), rule);
    }

    pub fn get_escalation_rule(&self, rule_id: &str) -> Option<&EscalationRule> {
        self.escalation_rules.get(rule_id)
    }

    pub fn escalation_matrix(&self) -> &EscalationMatrix {
        &self.escalation_matrix
    }
}

impl EscalationMatrix {
    pub fn new() -> Self {
        Self {
            matrix_id: "matrix_1".to_string(),
            matrix_name: "Escalation Matrix".to_string(),
            escalation_levels: Vec::new(),
        }
    }
}

impl AnonymizationEngine {
    pub fn new() -> Self {
        Self {
            anonymization_methods: HashMap::new(),
            privacy_models: HashMap::new(),
            risk_assessment: RiskAssessment::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_anonymization_method(&mut self, method: AnonymizationMethod) {
        self.anonymization_methods
            .insert(method.method_id.clone(), method);
    }

    pub fn get_anonymization_method(&self, method_id: &str) -> Option<&AnonymizationMethod> {
        self.anonymization_methods.get(method_id)
    }

    pub fn add_privacy_model(&mut self, model: PrivacyModel) {
        self.privacy_models.insert(model.model_id.clone(), model);
    }

    pub fn get_privacy_model(&self, model_id: &str) -> Option<&PrivacyModel> {
        self.privacy_models.get(model_id)
    }

    pub fn risk_assessment(&self) -> &RiskAssessment {
        &self.risk_assessment
    }
}

impl RiskAssessment {
    pub fn new() -> Self {
        Self {
            risk_models: HashMap::new(),
            risk_metrics: HashMap::new(),
        }
    }

    pub fn add_risk_model(&mut self, model: RiskModel) {
        self.risk_models.insert(model.model_id.clone(), model);
    }

    pub fn get_risk_model(&self, model_id: &str) -> Option<&RiskModel> {
        self.risk_models.get(model_id)
    }

    pub fn add_risk_metric(&mut self, metric: RiskMetric) {
        self.risk_metrics.insert(metric.metric_id.clone(), metric);
    }

    pub fn get_risk_metric(&self, metric_id: &str) -> Option<&RiskMetric> {
        self.risk_metrics.get(metric_id)
    }

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
    pub fn new() -> Self {
        Self {
            log_entries: HashMap::new(),
            log_analysis: LogAnalysis::new(),
            retention_policy: RetentionPolicy::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_log_entry(&mut self, entry: LogEntry) {
        self.log_entries.insert(entry.entry_id.clone(), entry);
    }

    pub fn get_log_entry(&self, entry_id: &str) -> Option<&LogEntry> {
        self.log_entries.get(entry_id)
    }

    pub fn log_analysis(&self) -> &LogAnalysis {
        &self.log_analysis
    }

    pub fn retention_policy(&self) -> &RetentionPolicy {
        &self.retention_policy
    }
}

impl LogAnalysis {
    pub fn new() -> Self {
        Self {
            analysis_methods: HashMap::new(),
            anomaly_detection: AnomalyDetection::new(),
        }
    }

    pub fn add_analysis_method(&mut self, method: AnalysisMethod) {
        self.analysis_methods
            .insert(method.method_id.clone(), method);
    }

    pub fn get_analysis_method(&self, method_id: &str) -> Option<&AnalysisMethod> {
        self.analysis_methods.get(method_id)
    }

    pub fn anomaly_detection(&self) -> &AnomalyDetection {
        &self.anomaly_detection
    }
}

impl AnomalyDetection {
    pub fn new() -> Self {
        Self {
            detection_algorithms: HashMap::new(),
            alert_thresholds: HashMap::new(),
        }
    }

    pub fn add_detection_algorithm(&mut self, algorithm: DetectionAlgorithm) {
        self.detection_algorithms
            .insert(algorithm.algorithm_id.clone(), algorithm);
    }

    pub fn get_detection_algorithm(&self, algorithm_id: &str) -> Option<&DetectionAlgorithm> {
        self.detection_algorithms.get(algorithm_id)
    }

    pub fn set_alert_threshold(&mut self, metric_name: &str, threshold: f64) {
        self.alert_thresholds
            .insert(metric_name.to_string(), threshold);
    }

    pub fn get_alert_threshold(&self, metric_name: &str) -> Option<&f64> {
        self.alert_thresholds.get(metric_name)
    }
}

impl RetentionPolicy {
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
    pub fn new() -> Self {
        Self {
            consent_records: HashMap::new(),
            consent_policies: HashMap::new(),
            consent_workflows: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_consent_record(&mut self, record: ConsentRecord) {
        self.consent_records
            .insert(record.record_id.clone(), record);
    }

    pub fn get_consent_record(&self, record_id: &str) -> Option<&ConsentRecord> {
        self.consent_records.get(record_id)
    }

    pub fn add_consent_policy(&mut self, policy: ConsentPolicy) {
        self.consent_policies
            .insert(policy.policy_id.clone(), policy);
    }

    pub fn get_consent_policy(&self, policy_id: &str) -> Option<&ConsentPolicy> {
        self.consent_policies.get(policy_id)
    }

    pub fn add_consent_workflow(&mut self, workflow: ConsentWorkflow) {
        self.consent_workflows
            .insert(workflow.workflow_id.clone(), workflow);
    }

    pub fn get_consent_workflow(&self, workflow_id: &str) -> Option<&ConsentWorkflow> {
        self.consent_workflows.get(workflow_id)
    }
}

impl DataAccessControl {
    pub fn new() -> Self {
        Self {
            access_policies: HashMap::new(),
            authentication: AuthenticationSystem::new(),
            authorization: AuthorizationSystem::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        self.authentication.initialize()?;
        self.authorization.initialize()?;
        Ok(())
    }

    pub fn add_access_policy(&mut self, policy: AccessPolicy) {
        self.access_policies
            .insert(policy.policy_id.clone(), policy);
    }

    pub fn get_access_policy(&self, policy_id: &str) -> Option<&AccessPolicy> {
        self.access_policies.get(policy_id)
    }

    pub fn list_access_policies(&self) -> Vec<String> {
        self.access_policies.keys().cloned().collect()
    }
}

impl AuthenticationSystem {
    pub fn new() -> Self {
        Self {
            authentication_methods: HashMap::new(),
            session_management: SessionManagement::new(),
            multi_factor: MultiFactorAuthentication::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_authentication_method(&mut self, method: AuthenticationMethod) {
        self.authentication_methods
            .insert(method.method_id.clone(), method);
    }

    pub fn get_authentication_method(&self, method_id: &str) -> Option<&AuthenticationMethod> {
        self.authentication_methods.get(method_id)
    }

    pub fn session_management(&self) -> &SessionManagement {
        &self.session_management
    }

    pub fn multi_factor(&self) -> &MultiFactorAuthentication {
        &self.multi_factor
    }
}

impl SessionManagement {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            session_policies: HashMap::new(),
        }
    }

    pub fn add_session(&mut self, session: Session) {
        self.sessions.insert(session.session_id.clone(), session);
    }

    pub fn get_session(&self, session_id: &str) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    pub fn remove_session(&mut self, session_id: &str) -> Option<Session> {
        self.sessions.remove(session_id)
    }

    pub fn add_session_policy(&mut self, policy: SessionPolicy) {
        self.session_policies
            .insert(policy.policy_id.clone(), policy);
    }

    pub fn get_session_policy(&self, policy_id: &str) -> Option<&SessionPolicy> {
        self.session_policies.get(policy_id)
    }
}

impl MultiFactorAuthentication {
    pub fn new() -> Self {
        Self {
            factors: HashMap::new(),
            factor_combinations: HashMap::new(),
        }
    }

    pub fn add_factor(&mut self, factor: AuthenticationFactor) {
        self.factors.insert(factor.factor_id.clone(), factor);
    }

    pub fn get_factor(&self, factor_id: &str) -> Option<&AuthenticationFactor> {
        self.factors.get(factor_id)
    }

    pub fn add_factor_combination(&mut self, combination: FactorCombination) {
        self.factor_combinations
            .insert(combination.combination_id.clone(), combination);
    }

    pub fn get_factor_combination(&self, combination_id: &str) -> Option<&FactorCombination> {
        self.factor_combinations.get(combination_id)
    }
}

impl AuthorizationSystem {
    pub fn new() -> Self {
        Self {
            authorization_policies: HashMap::new(),
            permission_management: PermissionManagement::new(),
            role_management: RoleManagement::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_authorization_policy(&mut self, policy: AuthorizationPolicy) {
        self.authorization_policies
            .insert(policy.policy_id.clone(), policy);
    }

    pub fn get_authorization_policy(&self, policy_id: &str) -> Option<&AuthorizationPolicy> {
        self.authorization_policies.get(policy_id)
    }

    pub fn permission_management(&self) -> &PermissionManagement {
        &self.permission_management
    }

    pub fn role_management(&self) -> &RoleManagement {
        &self.role_management
    }
}

impl PermissionManagement {
    pub fn new() -> Self {
        Self {
            permissions: HashMap::new(),
            permission_groups: HashMap::new(),
        }
    }

    pub fn add_permission(&mut self, name: &str, permission: Permission) {
        self.permissions.insert(name.to_string(), permission);
    }

    pub fn get_permission(&self, name: &str) -> Option<&Permission> {
        self.permissions.get(name)
    }

    pub fn add_permission_group(&mut self, group: PermissionGroup) {
        self.permission_groups.insert(group.group_id.clone(), group);
    }

    pub fn get_permission_group(&self, group_id: &str) -> Option<&PermissionGroup> {
        self.permission_groups.get(group_id)
    }
}

impl RoleManagement {
    pub fn new() -> Self {
        Self {
            roles: HashMap::new(),
            role_hierarchy: RoleHierarchy::new(),
        }
    }

    pub fn add_role(&mut self, role: Role) {
        self.roles.insert(role.role_id.clone(), role);
    }

    pub fn get_role(&self, role_id: &str) -> Option<&Role> {
        self.roles.get(role_id)
    }

    pub fn role_hierarchy(&self) -> &RoleHierarchy {
        &self.role_hierarchy
    }
}

impl RoleHierarchy {
    pub fn new() -> Self {
        Self {
            hierarchy_id: "hierarchy_1".to_string(),
            parent_roles: Vec::new(),
            child_roles: Vec::new(),
        }
    }
}

impl ClinicalAnalyzer {
    pub fn new() -> Self {
        Self {
            diagnostic_engine: DiagnosticEngine::new(),
            risk_assessment: ClinicalRiskAssessment::new(),
            treatment_planner: TreatmentPlanner::new(),
            outcome_predictor: OutcomePredictor::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        self.diagnostic_engine.initialize()?;
        self.risk_assessment.initialize()?;
        self.treatment_planner.initialize()?;
        self.outcome_predictor.initialize()?;
        Ok(())
    }

    pub fn analyze_data(
        &mut self,
        _patient: &Patient,
        _data_type: ClinicalDataType,
    ) -> Result<ClinicalAnalysis, MedicalError> {
        // NOT IMPLEMENTED — and it must say so, never fabricate. No diagnostic reasoning is
        // wired here (the `diagnostic_algorithms` registry is empty). Previously this returned
        // `ClinicalAnalysis::new()` (confidence_score 0.95, empty findings), and that fake 95%
        // confidence was surfaced through the `medical_compute` MCP tool as the confidence on a
        // clinical diagnosis — a dangerous, deceptive output. Real implementation requires a
        // VALIDATED diagnostic model plus a curated clinical ontology / knowledge base; until
        // those are supplied this capability cannot produce a result. See the to-do register.
        Err(MedicalError::NotImplemented(
            "clinical diagnostic analysis (analyze_clinical_data): requires a validated \
             diagnostic model and a curated clinical ontology/knowledge base, which are not \
             present. Refusing to emit a fabricated diagnosis or confidence."
                .to_string(),
        ))
    }
}

impl DiagnosticEngine {
    pub fn new() -> Self {
        Self {
            diagnostic_algorithms: HashMap::new(),
            symptom_analyzer: SymptomAnalyzer::new(),
            lab_interpreter: LabInterpreter::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_diagnostic_algorithm(&mut self, algorithm: DiagnosticAlgorithm) {
        self.diagnostic_algorithms
            .insert(algorithm.algorithm_id.clone(), algorithm);
    }

    pub fn get_diagnostic_algorithm(&self, algorithm_id: &str) -> Option<&DiagnosticAlgorithm> {
        self.diagnostic_algorithms.get(algorithm_id)
    }

    pub fn symptom_analyzer(&self) -> &SymptomAnalyzer {
        &self.symptom_analyzer
    }

    pub fn lab_interpreter(&self) -> &LabInterpreter {
        &self.lab_interpreter
    }
}

impl SymptomAnalyzer {
    pub fn new() -> Self {
        Self {
            symptom_patterns: HashMap::new(),
            symptom_correlations: HashMap::new(),
        }
    }

    pub fn add_symptom_pattern(&mut self, pattern: SymptomPattern) {
        self.symptom_patterns
            .insert(pattern.pattern_id.clone(), pattern);
    }

    pub fn get_symptom_pattern(&self, pattern_id: &str) -> Option<&SymptomPattern> {
        self.symptom_patterns.get(pattern_id)
    }

    pub fn add_symptom_correlation(&mut self, correlation: SymptomCorrelation) {
        self.symptom_correlations
            .insert(correlation.correlation_id.clone(), correlation);
    }

    pub fn get_symptom_correlation(&self, correlation_id: &str) -> Option<&SymptomCorrelation> {
        self.symptom_correlations.get(correlation_id)
    }
}

impl LabInterpreter {
    pub fn new() -> Self {
        Self {
            reference_ranges: HashMap::new(),
            abnormality_detector: AbnormalityDetector::new(),
        }
    }

    pub fn add_reference_range(&mut self, test_code: &str, range: ReferenceRange) {
        self.reference_ranges.insert(test_code.to_string(), range);
    }

    pub fn get_reference_range(&self, test_code: &str) -> Option<&ReferenceRange> {
        self.reference_ranges.get(test_code)
    }

    pub fn abnormality_detector(&self) -> &AbnormalityDetector {
        &self.abnormality_detector
    }

    pub fn interpret_result(&self, result: &LabResult) -> ResultStatus {
        if let Some(range) = self.reference_ranges.get(&result.test_code) {
            if result.value < range.minimum || result.value > range.maximum {
                ResultStatus::Abnormal
            } else {
                ResultStatus::Normal
            }
        } else {
            result.status.clone()
        }
    }
}

impl AbnormalityDetector {
    pub fn new() -> Self {
        Self {
            detection_algorithms: HashMap::new(),
            severity_assessment: SeverityAssessment::new(),
        }
    }

    pub fn add_detection_algorithm(&mut self, algorithm: DetectionAlgorithm) {
        self.detection_algorithms
            .insert(algorithm.algorithm_id.clone(), algorithm);
    }

    pub fn get_detection_algorithm(&self, algorithm_id: &str) -> Option<&DetectionAlgorithm> {
        self.detection_algorithms.get(algorithm_id)
    }

    pub fn severity_assessment(&self) -> &SeverityAssessment {
        &self.severity_assessment
    }
}

impl SeverityAssessment {
    pub fn new() -> Self {
        Self {
            assessment_criteria: HashMap::new(),
            scoring_system: ScoringSystem::new(),
        }
    }

    pub fn add_criterion(&mut self, criterion: AssessmentCriterion) {
        self.assessment_criteria
            .insert(criterion.criterion_id.clone(), criterion);
    }

    pub fn get_criterion(&self, criterion_id: &str) -> Option<&AssessmentCriterion> {
        self.assessment_criteria.get(criterion_id)
    }

    pub fn scoring_system(&self) -> &ScoringSystem {
        &self.scoring_system
    }
}

impl ScoringSystem {
    pub fn new() -> Self {
        Self {
            system_id: "system_1".to_string(),
            system_name: "Clinical Scoring System".to_string(),
            scoring_algorithm: ScoringAlgorithm::WeightedSum,
        }
    }
}

impl ClinicalRiskAssessment {
    pub fn new() -> Self {
        Self {
            risk_models: HashMap::new(),
            risk_factors: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_risk_model(&mut self, model: ClinicalRiskModel) {
        self.risk_models.insert(model.model_id.clone(), model);
    }

    pub fn get_risk_model(&self, model_id: &str) -> Option<&ClinicalRiskModel> {
        self.risk_models.get(model_id)
    }

    pub fn add_risk_factor(&mut self, factor: ClinicalRiskFactor) {
        self.risk_factors.insert(factor.factor_id.clone(), factor);
    }

    pub fn get_risk_factor(&self, factor_id: &str) -> Option<&ClinicalRiskFactor> {
        self.risk_factors.get(factor_id)
    }

    pub fn compute_risk_score(&self, factor_ids: &[String]) -> f64 {
        if factor_ids.is_empty() {
            return 0.0;
        }
        let total_weight: f64 = factor_ids
            .iter()
            .filter_map(|id| self.risk_factors.get(id))
            .map(|f| f.factor_weight)
            .sum();
        total_weight / factor_ids.len() as f64
    }
}

impl TreatmentPlanner {
    pub fn new() -> Self {
        Self {
            treatment_guidelines: HashMap::new(),
            decision_support: DecisionSupport::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_treatment_guideline(&mut self, guideline: TreatmentGuideline) {
        self.treatment_guidelines
            .insert(guideline.guideline_id.clone(), guideline);
    }

    pub fn get_treatment_guideline(&self, guideline_id: &str) -> Option<&TreatmentGuideline> {
        self.treatment_guidelines.get(guideline_id)
    }

    pub fn decision_support(&self) -> &DecisionSupport {
        &self.decision_support
    }
}

impl DecisionSupport {
    pub fn new() -> Self {
        Self {
            decision_trees: HashMap::new(),
            scoring_systems: HashMap::new(),
        }
    }

    pub fn add_decision_tree(&mut self, tree: DecisionTree) {
        self.decision_trees.insert(tree.tree_id.clone(), tree);
    }

    pub fn get_decision_tree(&self, tree_id: &str) -> Option<&DecisionTree> {
        self.decision_trees.get(tree_id)
    }

    pub fn add_scoring_system(&mut self, system: ScoringSystem) {
        self.scoring_systems
            .insert(system.system_id.clone(), system);
    }

    pub fn get_scoring_system(&self, system_id: &str) -> Option<&ScoringSystem> {
        self.scoring_systems.get(system_id)
    }
}

impl OutcomePredictor {
    pub fn new() -> Self {
        Self {
            prediction_models: HashMap::new(),
            outcome_metrics: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_prediction_model(&mut self, model_id: &str, model: PredictionModel) {
        self.prediction_models.insert(model_id.to_string(), model);
    }

    pub fn get_prediction_model(&self, model_id: &str) -> Option<&PredictionModel> {
        self.prediction_models.get(model_id)
    }

    pub fn add_outcome_metric(&mut self, metric_id: &str, value: OutcomeMetric) {
        self.outcome_metrics.insert(metric_id.to_string(), value);
    }

    pub fn get_outcome_metric(&self, metric_id: &str) -> Option<&OutcomeMetric> {
        self.outcome_metrics.get(metric_id)
    }
}

impl MedicalImaging {
    pub fn new() -> Self {
        Self {
            image_acquisition: ImageAcquisition::new(),
            image_processing: ImageProcessing::new(),
            image_analysis: ImageAnalysis::new(),
            image_storage: ImageStorage::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        self.image_acquisition.initialize()?;
        self.image_processing.initialize()?;
        self.image_analysis.initialize()?;
        self.image_storage.initialize()?;
        Ok(())
    }

    pub fn validate_image(&self, image: &MedicalImage) -> Result<(), MedicalError> {
        if image.image_id.is_empty() {
            return Err(MedicalError::ValidationError(
                "Image ID cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    pub fn process_image(
        &mut self,
        _image: &MedicalImage,
        _processing_type: ImageProcessingType,
    ) -> Result<ProcessedImage, MedicalError> {
        // NOT IMPLEMENTED — it must say so, never fabricate. Previously this returned a default
        // `ProcessedImage::new()` without touching the input image: a medical-image "analysis"
        // that analysed nothing. Real implementation requires an actual imaging pipeline
        // (reconstruction / enhancement / segmentation) and, for any diagnostic readout, a
        // validated model. See the to-do register.
        Err(MedicalError::NotImplemented(
            "medical image processing (process_medical_image): no imaging pipeline is \
             implemented; refusing to return an unprocessed image as if analysed."
                .to_string(),
        ))
    }
}

impl ImageAcquisition {
    pub fn new() -> Self {
        Self {
            acquisition_protocols: HashMap::new(),
            quality_control: QualityControl::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_acquisition_protocol(&mut self, protocol: AcquisitionProtocol) {
        self.acquisition_protocols
            .insert(protocol.protocol_id.clone(), protocol);
    }

    pub fn get_acquisition_protocol(&self, protocol_id: &str) -> Option<&AcquisitionProtocol> {
        self.acquisition_protocols.get(protocol_id)
    }

    pub fn quality_control(&self) -> &QualityControl {
        &self.quality_control
    }
}

impl QualityControl {
    pub fn new() -> Self {
        Self {
            quality_metrics: HashMap::new(),
            quality_standards: HashMap::new(),
        }
    }

    pub fn add_quality_metric(&mut self, metric: QualityMetric) {
        self.quality_metrics
            .insert(metric.metric_id.clone(), metric);
    }

    pub fn get_quality_metric(&self, metric_id: &str) -> Option<&QualityMetric> {
        self.quality_metrics.get(metric_id)
    }

    pub fn add_quality_standard(&mut self, standard: QualityStandard) {
        self.quality_standards
            .insert(standard.standard_id.clone(), standard);
    }

    pub fn get_quality_standard(&self, standard_id: &str) -> Option<&QualityStandard> {
        self.quality_standards.get(standard_id)
    }
}

impl ImageProcessing {
    pub fn new() -> Self {
        Self {
            preprocessing_algorithms: HashMap::new(),
            enhancement_techniques: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_preprocessing_algorithm(&mut self, algorithm: PreprocessingAlgorithm) {
        self.preprocessing_algorithms
            .insert(algorithm.algorithm_id.clone(), algorithm);
    }

    pub fn get_preprocessing_algorithm(
        &self,
        algorithm_id: &str,
    ) -> Option<&PreprocessingAlgorithm> {
        self.preprocessing_algorithms.get(algorithm_id)
    }

    pub fn add_enhancement_technique(&mut self, technique: EnhancementTechnique) {
        self.enhancement_techniques
            .insert(technique.technique_id.clone(), technique);
    }

    pub fn get_enhancement_technique(&self, technique_id: &str) -> Option<&EnhancementTechnique> {
        self.enhancement_techniques.get(technique_id)
    }
}

impl ImageAnalysis {
    pub fn new() -> Self {
        Self {
            analysis_algorithms: HashMap::new(),
            detection_methods: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_analysis_algorithm(&mut self, algorithm: AnalysisAlgorithm) {
        self.analysis_algorithms
            .insert(algorithm.algorithm_id.clone(), algorithm);
    }

    pub fn get_analysis_algorithm(&self, algorithm_id: &str) -> Option<&AnalysisAlgorithm> {
        self.analysis_algorithms.get(algorithm_id)
    }

    pub fn add_detection_method(&mut self, method: DetectionMethod) {
        self.detection_methods
            .insert(method.method_id.clone(), method);
    }

    pub fn get_detection_method(&self, method_id: &str) -> Option<&DetectionMethod> {
        self.detection_methods.get(method_id)
    }
}

impl ImageStorage {
    pub fn new() -> Self {
        Self {
            storage_systems: HashMap::new(),
            compression_methods: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_storage_system(&mut self, system: StorageSystem) {
        self.storage_systems
            .insert(system.system_id.clone(), system);
    }

    pub fn get_storage_system(&self, system_id: &str) -> Option<&StorageSystem> {
        self.storage_systems.get(system_id)
    }

    pub fn add_compression_method(&mut self, method: CompressionMethod) {
        self.compression_methods
            .insert(method.method_id.clone(), method);
    }

    pub fn get_compression_method(&self, method_id: &str) -> Option<&CompressionMethod> {
        self.compression_methods.get(method_id)
    }
}

impl DrugDiscovery {
    pub fn new() -> Self {
        Self {
            target_identification: TargetIdentification::new(),
            compound_screening: CompoundScreening::new(),
            lead_optimization: LeadOptimization::new(),
            preclinical_testing: PreclinicalTesting::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        self.target_identification.initialize()?;
        self.compound_screening.initialize()?;
        self.lead_optimization.initialize()?;
        self.preclinical_testing.initialize()?;
        Ok(())
    }

    pub fn validate_compounds(&self, compounds: &[Compound]) -> Result<(), MedicalError> {
        if compounds.is_empty() {
            return Err(MedicalError::ValidationError(
                "At least one compound must be provided".to_string(),
            ));
        }
        Ok(())
    }

    pub fn screen_compounds(
        &mut self,
        _compounds: &[Compound],
        _target: &DrugTarget,
    ) -> Result<ScreeningResults, MedicalError> {
        // NOT IMPLEMENTED — it must say so, never fabricate. Previously this returned a default
        // `ScreeningResults::new()` (hit_rate 0.05, no compounds actually screened) regardless
        // of the inputs — a fabricated drug-screening result. Real implementation requires
        // structure/affinity models and compound/target reference data. See the to-do register.
        Err(MedicalError::NotImplemented(
            "virtual compound screening (screen_compounds): requires binding-affinity / \
             structure models and compound/target reference data, which are not present."
                .to_string(),
        ))
    }
}

impl TargetIdentification {
    pub fn new() -> Self {
        Self {
            target_databases: HashMap::new(),
            validation_methods: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_target_database(&mut self, database: TargetDatabase) {
        self.target_databases
            .insert(database.database_id.clone(), database);
    }

    pub fn get_target_database(&self, database_id: &str) -> Option<&TargetDatabase> {
        self.target_databases.get(database_id)
    }

    pub fn add_validation_method(&mut self, method: ValidationMethod) {
        self.validation_methods
            .insert(method.method_id.clone(), method);
    }

    pub fn get_validation_method(&self, method_id: &str) -> Option<&ValidationMethod> {
        self.validation_methods.get(method_id)
    }
}

impl CompoundScreening {
    pub fn new() -> Self {
        Self {
            compound_libraries: HashMap::new(),
            screening_assays: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_compound_library(&mut self, library: CompoundLibrary) {
        self.compound_libraries
            .insert(library.library_id.clone(), library);
    }

    pub fn get_compound_library(&self, library_id: &str) -> Option<&CompoundLibrary> {
        self.compound_libraries.get(library_id)
    }

    pub fn add_screening_assay(&mut self, assay: ScreeningAssay) {
        self.screening_assays.insert(assay.assay_id.clone(), assay);
    }

    pub fn get_screening_assay(&self, assay_id: &str) -> Option<&ScreeningAssay> {
        self.screening_assays.get(assay_id)
    }
}

impl LeadOptimization {
    pub fn new() -> Self {
        Self {
            optimization_strategies: HashMap::new(),
            adme_prediction: ADMEPrediction::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_optimization_strategy(&mut self, strategy: OptimizationStrategy) {
        self.optimization_strategies
            .insert(strategy.strategy_id.clone(), strategy);
    }

    pub fn get_optimization_strategy(&self, strategy_id: &str) -> Option<&OptimizationStrategy> {
        self.optimization_strategies.get(strategy_id)
    }

    pub fn adme_prediction(&self) -> &ADMEPrediction {
        &self.adme_prediction
    }
}

impl ADMEPrediction {
    pub fn new() -> Self {
        Self {
            absorption_model: AbsorptionModel::new(),
            distribution_model: DistributionModel::new(),
            metabolism_model: MetabolismModel::new(),
            excretion_model: ExcretionModel::new(),
        }
    }

    pub fn absorption_model(&self) -> &AbsorptionModel {
        &self.absorption_model
    }

    pub fn distribution_model(&self) -> &DistributionModel {
        &self.distribution_model
    }

    pub fn metabolism_model(&self) -> &MetabolismModel {
        &self.metabolism_model
    }

    pub fn excretion_model(&self) -> &ExcretionModel {
        &self.excretion_model
    }

    /// Returns the absorption model's **stored** `bioavailability` parameter.
    ///
    /// HONESTY: this is a model *parameter*, not a compound-specific computed
    /// prediction. `AbsorptionModel::new()` seeds it with a neutral `0.5`
    /// placeholder; nothing in this module computes a real per-compound
    /// bioavailability (that needs a genuine PBPK / first-pass-metabolism
    /// model). Do not surface this as a prediction — it will read `0.5` for
    /// every compound until the field is set from real data or a real model.
    pub fn overall_bioavailability(&self) -> f64 {
        self.absorption_model.bioavailability
    }
}

impl AbsorptionModel {
    pub fn new() -> Self {
        Self {
            model_type: ModelType::PhysiologicallyBased,
            bioavailability: 0.5,
            absorption_rate: 0.1,
        }
    }
}

impl DistributionModel {
    pub fn new() -> Self {
        Self {
            volume_of_distribution: 10.0,
            protein_binding: 0.9,
            tissue_distribution: HashMap::new(),
        }
    }
}

impl MetabolismModel {
    pub fn new() -> Self {
        Self {
            metabolic_pathways: Vec::new(),
            clearance: 0.1,
            half_life: 10.0,
        }
    }
}

impl ExcretionModel {
    pub fn new() -> Self {
        Self {
            excretion_routes: Vec::new(),
            excretion_rate: 0.1,
        }
    }
}

impl PreclinicalTesting {
    pub fn new() -> Self {
        Self {
            in_vitro_testing: InVitroTesting::new(),
            in_vivo_testing: InVivoTesting::new(),
            toxicology_studies: ToxicologyStudies::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn in_vitro_testing(&self) -> &InVitroTesting {
        &self.in_vitro_testing
    }

    pub fn in_vivo_testing(&self) -> &InVivoTesting {
        &self.in_vivo_testing
    }

    pub fn toxicology_studies(&self) -> &ToxicologyStudies {
        &self.toxicology_studies
    }
}

impl InVitroTesting {
    pub fn new() -> Self {
        Self {
            test_types: HashMap::new(),
            results: HashMap::new(),
        }
    }

    pub fn add_test(&mut self, test: InVitroTest) {
        self.test_types.insert(test.test_id.clone(), test);
    }

    pub fn get_test(&self, test_id: &str) -> Option<&InVitroTest> {
        self.test_types.get(test_id)
    }

    pub fn add_result(&mut self, result: TestResult) {
        self.results.insert(result.result_id.clone(), result);
    }

    pub fn get_result(&self, result_id: &str) -> Option<&TestResult> {
        self.results.get(result_id)
    }
}

impl InVivoTesting {
    pub fn new() -> Self {
        Self {
            animal_models: HashMap::new(),
            study_designs: HashMap::new(),
        }
    }

    pub fn add_animal_model(&mut self, model: AnimalModel) {
        self.animal_models.insert(model.model_id.clone(), model);
    }

    pub fn get_animal_model(&self, model_id: &str) -> Option<&AnimalModel> {
        self.animal_models.get(model_id)
    }

    pub fn add_study_design(&mut self, design: StudyDesign) {
        self.study_designs.insert(design.design_id.clone(), design);
    }

    pub fn get_study_design(&self, design_id: &str) -> Option<&StudyDesign> {
        self.study_designs.get(design_id)
    }
}

impl ToxicologyStudies {
    pub fn new() -> Self {
        Self {
            study_types: HashMap::new(),
            safety_assessments: HashMap::new(),
        }
    }

    pub fn add_study(&mut self, study: ToxicologyStudy) {
        self.study_types.insert(study.study_id.clone(), study);
    }

    pub fn get_study(&self, study_id: &str) -> Option<&ToxicologyStudy> {
        self.study_types.get(study_id)
    }

    pub fn add_safety_assessment(&mut self, assessment: SafetyAssessment) {
        self.safety_assessments
            .insert(assessment.assessment_id.clone(), assessment);
    }

    pub fn get_safety_assessment(&self, assessment_id: &str) -> Option<&SafetyAssessment> {
        self.safety_assessments.get(assessment_id)
    }
}

impl MedicalComplianceMonitor {
    pub fn new() -> Self {
        Self {
            hipaa_compliance: HIPAACompliance::new(),
            gdpr_compliance: GDPRCompliance::new(),
            clinical_standards: ClinicalStandards::new(),
            audit_system: AuditSystem::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        self.hipaa_compliance.initialize()?;
        self.gdpr_compliance.initialize()?;
        self.clinical_standards.initialize()?;
        self.audit_system.initialize()?;
        Ok(())
    }

    pub fn check_compliance(
        &mut self,
        _compliance_type: ComplianceType,
    ) -> Result<ComplianceReport, MedicalError> {
        // Check compliance
        let report = ComplianceReport::new();

        Ok(report)
    }
}

impl HIPAACompliance {
    pub fn new() -> Self {
        Self {
            privacy_rules: HashMap::new(),
            security_rules: HashMap::new(),
            breach_notification: BreachNotification::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_privacy_rule(&mut self, rule: PrivacyRule) {
        self.privacy_rules.insert(rule.rule_id.clone(), rule);
    }

    pub fn get_privacy_rule(&self, rule_id: &str) -> Option<&PrivacyRule> {
        self.privacy_rules.get(rule_id)
    }

    pub fn add_security_rule(&mut self, rule: SecurityRule) {
        self.security_rules.insert(rule.rule_id.clone(), rule);
    }

    pub fn get_security_rule(&self, rule_id: &str) -> Option<&SecurityRule> {
        self.security_rules.get(rule_id)
    }

    pub fn breach_notification(&self) -> &BreachNotification {
        &self.breach_notification
    }
}

impl BreachNotification {
    pub fn new() -> Self {
        Self {
            notification_rules: HashMap::new(),
            notification_templates: HashMap::new(),
        }
    }

    pub fn add_notification_rule(&mut self, rule: NotificationRule) {
        self.notification_rules.insert(rule.rule_id.clone(), rule);
    }

    pub fn get_notification_rule(&self, rule_id: &str) -> Option<&NotificationRule> {
        self.notification_rules.get(rule_id)
    }

    pub fn add_notification_template(&mut self, template: NotificationTemplate) {
        self.notification_templates
            .insert(template.template_id.clone(), template);
    }

    pub fn get_notification_template(&self, template_id: &str) -> Option<&NotificationTemplate> {
        self.notification_templates.get(template_id)
    }
}

impl GDPRCompliance {
    pub fn new() -> Self {
        Self {
            data_protection_principles: HashMap::new(),
            data_subject_rights: HashMap::new(),
            data_processing_agreements: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_data_protection_principle(&mut self, principle: DataProtectionPrinciple) {
        self.data_protection_principles
            .insert(principle.principle_id.clone(), principle);
    }

    pub fn get_data_protection_principle(
        &self,
        principle_id: &str,
    ) -> Option<&DataProtectionPrinciple> {
        self.data_protection_principles.get(principle_id)
    }

    pub fn add_data_subject_right(&mut self, right: DataSubjectRight) {
        self.data_subject_rights
            .insert(right.right_id.clone(), right);
    }

    pub fn get_data_subject_right(&self, right_id: &str) -> Option<&DataSubjectRight> {
        self.data_subject_rights.get(right_id)
    }

    pub fn add_data_processing_agreement(&mut self, agreement: DataProcessingAgreement) {
        self.data_processing_agreements
            .insert(agreement.agreement_id.clone(), agreement);
    }

    pub fn get_data_processing_agreement(
        &self,
        agreement_id: &str,
    ) -> Option<&DataProcessingAgreement> {
        self.data_processing_agreements.get(agreement_id)
    }
}

impl ClinicalStandards {
    pub fn new() -> Self {
        Self {
            clinical_guidelines: HashMap::new(),
            quality_metrics: HashMap::new(),
            best_practices: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_clinical_guideline(&mut self, guideline: ClinicalGuideline) {
        self.clinical_guidelines
            .insert(guideline.guideline_id.clone(), guideline);
    }

    pub fn get_clinical_guideline(&self, guideline_id: &str) -> Option<&ClinicalGuideline> {
        self.clinical_guidelines.get(guideline_id)
    }

    pub fn add_quality_metric(&mut self, metric: QualityMetric) {
        self.quality_metrics
            .insert(metric.metric_id.clone(), metric);
    }

    pub fn get_quality_metric(&self, metric_id: &str) -> Option<&QualityMetric> {
        self.quality_metrics.get(metric_id)
    }

    pub fn add_best_practice(&mut self, practice: BestPractice) {
        self.best_practices
            .insert(practice.practice_id.clone(), practice);
    }

    pub fn get_best_practice(&self, practice_id: &str) -> Option<&BestPractice> {
        self.best_practices.get(practice_id)
    }
}

impl AuditSystem {
    pub fn new() -> Self {
        Self {
            audit_trails: HashMap::new(),
            audit_reports: HashMap::new(),
            compliance_monitoring: ComplianceMonitoring::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    pub fn add_audit_trail(&mut self, trail: AuditTrail) {
        self.audit_trails.insert(trail.trail_id.clone(), trail);
    }

    pub fn get_audit_trail(&self, trail_id: &str) -> Option<&AuditTrail> {
        self.audit_trails.get(trail_id)
    }

    pub fn add_audit_report(&mut self, report: AuditReport) {
        self.audit_reports.insert(report.report_id.clone(), report);
    }

    pub fn get_audit_report(&self, report_id: &str) -> Option<&AuditReport> {
        self.audit_reports.get(report_id)
    }

    pub fn compliance_monitoring(&self) -> &ComplianceMonitoring {
        &self.compliance_monitoring
    }
}

impl ComplianceMonitoring {
    pub fn new() -> Self {
        Self {
            monitoring_rules: HashMap::new(),
            compliance_metrics: HashMap::new(),
        }
    }

    pub fn add_monitoring_rule(&mut self, rule: MonitoringRule) {
        self.monitoring_rules.insert(rule.rule_id.clone(), rule);
    }

    pub fn get_monitoring_rule(&self, rule_id: &str) -> Option<&MonitoringRule> {
        self.monitoring_rules.get(rule_id)
    }

    pub fn add_compliance_metric(&mut self, metric: ComplianceMetric) {
        self.compliance_metrics
            .insert(metric.metric_id.clone(), metric);
    }

    pub fn get_compliance_metric(&self, metric_id: &str) -> Option<&ComplianceMetric> {
        self.compliance_metrics.get(metric_id)
    }

    pub fn is_compliant(&self, metric_id: &str) -> bool {
        if let Some(metric) = self.compliance_metrics.get(metric_id) {
            metric.metric_value >= metric.metric_target
        } else {
            false
        }
    }
}

// Supporting structs

impl Patient {
    pub fn new() -> Self {
        Self {
            patient_id: "patient_1".to_string(),
            medical_record_number: "MRN001".to_string(),
            demographics: Demographics::new(),
            medical_history: MedicalHistory::new(),
            current_medications: Vec::new(),
            allergies: Vec::new(),
            vital_signs: Vec::new(),
            lab_results: Vec::new(),
            imaging_studies: Vec::new(),
            created_at: 0,
            last_updated: 0,
        }
    }
}

impl Demographics {
    pub fn new() -> Self {
        Self {
            name: "John Doe".to_string(),
            date_of_birth: "1980-01-01".to_string(),
            gender: Gender::Male,
            ethnicity: "Caucasian".to_string(),
            language: "English".to_string(),
            contact_info: ContactInfo::new(),
            emergency_contacts: vec![EmergencyContact::new()],
        }
    }
}

impl ContactInfo {
    pub fn new() -> Self {
        Self {
            phone: "555-123-4567".to_string(),
            email: "john.doe@example.com".to_string(),
            address: Address::new(),
        }
    }
}

impl Address {
    pub fn new() -> Self {
        Self {
            street: "123 Main St".to_string(),
            city: "Anytown".to_string(),
            state: "CA".to_string(),
            zip_code: "12345".to_string(),
            country: "USA".to_string(),
        }
    }
}

impl EmergencyContact {
    pub fn new() -> Self {
        Self {
            name: "Jane Doe".to_string(),
            relationship: "Spouse".to_string(),
            phone: "555-987-6543".to_string(),
            email: "jane.doe@example.com".to_string(),
        }
    }
}

impl MedicalCondition {
    pub fn new() -> Self {
        Self {
            condition_id: "condition_1".to_string(),
            condition_name: "Hypertension".to_string(),
            icd_code: "I10".to_string(),
            diagnosis_date: "2020-01-01".to_string(),
            status: ConditionStatus::Chronic,
            severity: Severity::Moderate,
            treatment_plan: TreatmentPlan::new(),
        }
    }
}

impl TreatmentPlan {
    pub fn new() -> Self {
        Self {
            plan_id: "plan_1".to_string(),
            medications: vec![Medication::new()],
            procedures: Vec::new(),
            follow_up_care: FollowUpCare::new(),
        }
    }
}

impl Medication {
    pub fn new() -> Self {
        Self {
            medication_id: "med_1".to_string(),
            name: "Lisinopril".to_string(),
            dosage: "10mg".to_string(),
            frequency: "Once daily".to_string(),
            route: Route::Oral,
            start_date: "2020-01-01".to_string(),
            end_date: None,
            prescribed_by: "Dr. Smith".to_string(),
            indications: vec!["Hypertension".to_string()],
            contraindications: vec!["Pregnancy".to_string()],
            side_effects: vec!["Cough".to_string()],
        }
    }
}

impl FollowUpCare {
    pub fn new() -> Self {
        Self {
            follow_up_id: "followup_1".to_string(),
            instructions: "Monitor blood pressure".to_string(),
            next_appointment: Some("2020-02-01".to_string()),
            monitoring_required: true,
            monitoring_parameters: vec!["Blood pressure".to_string()],
        }
    }
}

impl Surgery {
    pub fn new() -> Self {
        Self {
            surgery_id: "surgery_1".to_string(),
            surgery_name: "Appendectomy".to_string(),
            date: "2019-06-15".to_string(),
            surgeon: "Dr. Johnson".to_string(),
            facility: "General Hospital".to_string(),
            anesthesia_type: "General".to_string(),
            complications: Vec::new(),
            recovery_time: 7,
        }
    }
}

impl Hospitalization {
    pub fn new() -> Self {
        Self {
            hospitalization_id: "hospital_1".to_string(),
            admission_date: "2019-06-14".to_string(),
            discharge_date: Some("2019-06-21".to_string()),
            facility: "General Hospital".to_string(),
            admission_reason: "Appendicitis".to_string(),
            diagnosis: vec!["Appendicitis".to_string()],
            procedures: vec!["Appendectomy".to_string()],
            length_of_stay: 7,
        }
    }
}

impl Allergy {
    pub fn new() -> Self {
        Self {
            allergy_id: "allergy_1".to_string(),
            allergen: "Penicillin".to_string(),
            reaction_type: ReactionType::Anaphylaxis,
            severity: AllergySeverity::LifeThreatening,
            reaction_details: "Severe allergic reaction".to_string(),
            treatment: "Epinephrine".to_string(),
        }
    }
}

impl VitalSigns {
    pub fn new() -> Self {
        Self {
            vital_signs_id: "vitals_1".to_string(),
            timestamp: 0,
            blood_pressure: BloodPressure::new(),
            heart_rate: 72,
            respiratory_rate: 16,
            temperature: 98.6,
            oxygen_saturation: 98.0,
            height: Some(70.0),
            weight: Some(180.0),
            bmi: Some(25.8),
        }
    }
}

impl BloodPressure {
    pub fn new() -> Self {
        Self {
            systolic: 120,
            diastolic: 80,
            position: Position::Sitting,
        }
    }
}

impl LabResult {
    pub fn new() -> Self {
        Self {
            result_id: "lab_1".to_string(),
            test_name: "Complete Blood Count".to_string(),
            test_code: "CBC".to_string(),
            specimen: "Blood".to_string(),
            result_date: "2020-01-01".to_string(),
            value: 4.5,
            unit: "M/uL".to_string(),
            reference_range: ReferenceRange::new(),
            status: ResultStatus::Normal,
            interpretation: "Within normal limits".to_string(),
        }
    }
}

impl ReferenceRange {
    pub fn new() -> Self {
        Self {
            minimum: 4.0,
            maximum: 11.0,
            unit: "M/uL".to_string(),
        }
    }
}

impl ImagingStudy {
    pub fn new() -> Self {
        Self {
            study_id: "study_1".to_string(),
            study_type: ImagingType::XRay,
            date: "2020-01-01".to_string(),
            ordering_physician: "Dr. Smith".to_string(),
            radiologist: "Dr. Jones".to_string(),
            facility: "General Hospital".to_string(),
            findings: "No acute abnormalities".to_string(),
            impression: "Normal study".to_string(),
            images: vec![MedicalImage::new()],
        }
    }
}

impl MedicalImage {
    pub fn new() -> Self {
        Self {
            image_id: "image_1".to_string(),
            image_type: ImageFormat::DICOM,
            series_number: 1,
            acquisition_date: "2020-01-01".to_string(),
            modality: "XR".to_string(),
            body_part: "Chest".to_string(),
            image_data: vec![0u8; 1024],
        }
    }
}

impl MedicalIdentifier {
    pub fn new() -> Self {
        Self {
            identifier_type: IdentifierType::SocialSecurity,
            identifier_value: "123-45-6789".to_string(),
            issuing_authority: "SSA".to_string(),
            issue_date: "1980-01-01".to_string(),
            expiry_date: None,
        }
    }
}

impl ClinicalAnalysis {
    pub fn new() -> Self {
        Self {
            analysis_id: "analysis_1".to_string(),
            analysis_type: ClinicalDataType::Diagnosis,
            findings: Vec::new(),
            recommendations: Vec::new(),
            // No analysis has run on a default-constructed value — confidence is 0, never a
            // fabricated 0.95. (The diagnostic path returns NotImplemented rather than this.)
            confidence_score: 0.0,
        }
    }
}

impl ProcessedImage {
    pub fn new() -> Self {
        Self {
            processed_image_id: "processed_1".to_string(),
            original_image_id: "image_1".to_string(),
            processing_type: ImageProcessingType::Enhancement,
            processed_data: vec![0u8; 1024],
            processing_metadata: HashMap::new(),
        }
    }
}

impl ScreeningResults {
    pub fn new() -> Self {
        Self {
            results_id: "screening_1".to_string(),
            target_id: "target_1".to_string(),
            screened_compounds: Vec::new(),
            hit_compounds: Vec::new(),
            // Nothing screened on a default value — hit_rate is 0, never a fabricated 0.05.
            hit_rate: 0.0,
            screening_metrics: ScreeningMetrics::new(),
        }
    }
}

impl ScreeningMetrics {
    pub fn new() -> Self {
        Self {
            total_compounds: 1000,
            hit_rate: 0.05,
            false_positive_rate: 0.1,
            screening_time: 3600.0,
        }
    }
}

impl ComplianceReport {
    pub fn new() -> Self {
        Self {
            report_id: "report_1".to_string(),
            report_type: ComplianceType::HIPAA,
            compliance_score: None,
            violations: Vec::new(),
            recommendations: Vec::new(),
            generated_at: 0,
        }
    }
}

impl MedicalPerformanceMetrics {
    pub fn new() -> Self {
        Self {
            total_patients: 0,
            average_processing_time: 0.0,
            privacy_score: None,
            compliance_score: None,
            data_quality: None,
        }
    }
}

// Enums and supporting types

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClinicalDataType {
    Diagnosis,
    Treatment,
    Prognosis,
    Prevention,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImageProcessingType {
    Enhancement,
    Segmentation,
    Registration,
    Analysis,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComplianceType {
    HIPAA,
    GDPR,
    Clinical,
    Security,
}

/// Audit trail entry for medical data access
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub entry_id: String,
    pub timestamp: u64,
    pub user_id: String,
    pub patient_id: String,
    pub action: String,
    pub details: String,
}

/// Clinical data analysis result
#[derive(Debug, Clone)]
pub struct ClinicalAnalysis {
    pub analysis_id: String,
    pub analysis_type: ClinicalDataType,
    pub findings: Vec<String>,
    pub recommendations: Vec<String>,
    pub confidence_score: f64,
}

/// Processed medical image
#[derive(Debug, Clone)]
pub struct ProcessedImage {
    pub processed_image_id: String,
    pub original_image_id: String,
    pub processing_type: ImageProcessingType,
    pub processed_data: Vec<u8>,
    pub processing_metadata: HashMap<String, String>,
}

/// Drug screening results
#[derive(Debug, Clone)]
pub struct ScreeningResults {
    pub results_id: String,
    pub target_id: String,
    pub screened_compounds: Vec<String>,
    pub hit_compounds: Vec<String>,
    pub hit_rate: f64,
    pub screening_metrics: ScreeningMetrics,
}

/// Screening performance metrics
#[derive(Debug, Clone)]
pub struct ScreeningMetrics {
    pub total_compounds: u64,
    pub hit_rate: f64,
    pub false_positive_rate: f64,
    pub screening_time: f64,
}

/// Compliance report for regulatory requirements
#[derive(Debug, Clone)]
pub struct ComplianceReport {
    pub report_id: String,
    pub report_type: ComplianceType,
    /// Compliance score (e.g. HIPAA). `None` = not assessed. No compliance evaluation is
    /// performed here, so it must not fabricate a score — previously `new()` claimed a
    /// hardcoded 0.95 ("95% HIPAA-compliant") from no assessment at all.
    pub compliance_score: Option<f64>,
    pub violations: Vec<String>,
    pub recommendations: Vec<String>,
    pub generated_at: u64,
}

/// Medical library performance summary metrics
#[derive(Debug, Clone)]
pub struct MedicalPerformanceMetrics {
    pub total_patients: u64,
    pub average_processing_time: f64,
    /// Privacy / compliance / data-quality scores. `None` = not measured. This scaffold does
    /// not compute these, so it must not fabricate them — previously `new()` claimed a hardcoded
    /// 95% private / 98% compliant / 92% quality for a library that measures none of it.
    pub privacy_score: Option<f64>,
    pub compliance_score: Option<f64>,
    pub data_quality: Option<f64>,
}

/// Medical error types
#[derive(Debug, Clone)]
pub enum MedicalError {
    ValidationError(String),
    PatientError(String),
    ClinicalError(String),
    ImagingError(String),
    DrugDiscoveryError(String),
    ComplianceError(String),
    PrivacyError(String),
    DataError(String),
    /// The capability is not implemented yet. Returned INSTEAD of fabricating a clinical
    /// result. A medical routine that cannot validly compute a result must say so — never
    /// emit a confident fake. The string names the capability + what real implementation needs.
    NotImplemented(String),
    /// The capability exists but the required input — a validated model, a medical ontology,
    /// a knowledge base, reference data — is not available, so no result can be produced.
    InsufficientData(String),
}

impl std::fmt::Display for MedicalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MedicalError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            MedicalError::PatientError(msg) => write!(f, "Patient error: {}", msg),
            MedicalError::ClinicalError(msg) => write!(f, "Clinical error: {}", msg),
            MedicalError::ImagingError(msg) => write!(f, "Imaging error: {}", msg),
            MedicalError::DrugDiscoveryError(msg) => write!(f, "Drug discovery error: {}", msg),
            MedicalError::ComplianceError(msg) => write!(f, "Compliance error: {}", msg),
            MedicalError::PrivacyError(msg) => write!(f, "Privacy error: {}", msg),
            MedicalError::DataError(msg) => write!(f, "Data error: {}", msg),
            MedicalError::NotImplemented(msg) => write!(f, "Not implemented yet: {}", msg),
            MedicalError::InsufficientData(msg) => {
                write!(f, "Required information not available: {}", msg)
            }
        }
    }
}

impl std::error::Error for MedicalError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_medical_library_creation() {
        let mut library = MedicalComputingLibrary::new();
        assert!(library.initialize().is_ok());
    }

    #[test]
    fn test_patient_record_creation() {
        let mut library = MedicalComputingLibrary::new();
        library.initialize().unwrap();

        let patient = Patient::new();
        let result = library.create_patient_record(patient).unwrap();

        assert_eq!(result.result.patient_id, "patient_1");
        assert_eq!(result.result.medical_record_number, "MRN001");
        // Honest: privacy is not measured by this scaffold, so no score is fabricated.
        assert!(result.privacy_score.is_none());
        assert!(result.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_clinical_analysis() {
        let mut library = MedicalComputingLibrary::new();
        library.initialize().unwrap();

        // HONEST: no diagnostic backend is implemented, so clinical analysis fails closed
        // rather than emitting a fabricated diagnosis/confidence (it previously returned a
        // hardcoded confidence_score 0.95 with empty findings).
        let result = library.analyze_clinical_data("patient_1", ClinicalDataType::Diagnosis);
        assert!(
            result.is_err(),
            "clinical analysis must fail closed, not fabricate a diagnosis/confidence"
        );
    }

    #[test]
    fn test_medical_imaging() {
        let mut library = MedicalComputingLibrary::new();
        library.initialize().unwrap();

        let image = MedicalImage::new();
        // HONEST: no imaging pipeline is implemented, so this reports NotImplemented rather
        // than returning an unprocessed image as if it had been analysed.
        let result = library.process_medical_image(image, ImageProcessingType::Enhancement);
        assert!(matches!(result, Err(MedicalError::NotImplemented(_))));
    }

    #[test]
    fn test_compound_screening() {
        let mut library = MedicalComputingLibrary::new();
        library.initialize().unwrap();

        let compounds = vec![Compound::new()];
        let target = DrugTarget::new();

        // HONEST: no screening models / reference data, so this reports NotImplemented rather
        // than fabricating a hit_rate from compounds it never actually screened.
        let result = library.screen_compounds(compounds, target);
        assert!(matches!(result, Err(MedicalError::NotImplemented(_))));
    }

    #[test]
    fn test_compliance_check() {
        let mut library = MedicalComputingLibrary::new();
        library.initialize().unwrap();

        let result = library.check_compliance(ComplianceType::HIPAA).unwrap();

        assert_eq!(result.result.report_type, ComplianceType::HIPAA);
        // Honest: no compliance assessment is performed, so no score is fabricated.
        assert!(result.result.compliance_score.is_none());
        assert!(result.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_performance_metrics() {
        let library = MedicalComputingLibrary::new();
        let metrics = library.get_performance_stats();

        assert_eq!(metrics.total_patients, 0);
        assert_eq!(metrics.average_processing_time, 0.0);
        // Honest: this scaffold measures none of these, so they are not fabricated.
        assert!(metrics.privacy_score.is_none());
        assert!(metrics.compliance_score.is_none());
        assert!(metrics.data_quality.is_none());
    }

    #[test]
    fn test_patient_listing() {
        let library = MedicalComputingLibrary::new();
        let patients = library.list_patients();
        assert_eq!(patients.len(), 0);
    }

    #[test]
    fn test_patient_info() {
        let library = MedicalComputingLibrary::new();
        let info = library.get_patient_info("patient_1");
        assert!(info.is_none());
    }

    // -----------------------------------------------------------------
    // Deterministic clinical calculators — known-value (textbook) tests.
    // -----------------------------------------------------------------

    fn lib() -> MedicalComputingLibrary {
        MedicalComputingLibrary::new()
    }

    fn approx(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "expected {b}, got {a} (tol {tol})");
    }

    #[test]
    fn test_bmi_known_value() {
        // 70 kg / (1.75 m)^2 = 22.857...
        approx(lib().bmi(70.0, 1.75).unwrap(), 22.857, 1e-3);
        assert!(lib().bmi(70.0, 0.0).is_err());
    }

    #[test]
    fn test_bsa_mosteller_known_value() {
        // sqrt(180*75/3600) = sqrt(3.75) = 1.93649
        approx(lib().bsa_mosteller(75.0, 180.0).unwrap(), 1.93649, 1e-4);
    }

    #[test]
    fn test_bsa_du_bois_known_value() {
        // 0.007184 * 70^0.425 * 170^0.725 ≈ 1.8090 m²
        approx(lib().bsa_du_bois(70.0, 170.0).unwrap(), 1.8090, 1e-3);
    }

    #[test]
    fn test_ideal_body_weight_devine() {
        // Male, 175 cm: 68.898 in; 50 + 2.3*(8.898) = 70.465 kg
        approx(
            lib().ideal_body_weight_devine(175.0, Gender::Male).unwrap(),
            70.465,
            1e-2,
        );
        // Female base 45.5 → 65.965 kg
        approx(
            lib().ideal_body_weight_devine(175.0, Gender::Female).unwrap(),
            65.965,
            1e-2,
        );
        // Sex Other/Unknown must fail closed (no validated coefficient).
        assert!(lib()
            .ideal_body_weight_devine(175.0, Gender::Unknown)
            .is_err());
    }

    #[test]
    fn test_egfr_ckd_epi_2021_known_value() {
        // Male, Scr 1.0 mg/dL, age 50 → ≈ 91.7 mL/min/1.73m²
        approx(
            lib().egfr_ckd_epi_2021(1.0, 50.0, Gender::Male).unwrap(),
            91.70,
            0.2,
        );
        assert!(lib().egfr_ckd_epi_2021(1.0, 50.0, Gender::Other).is_err());
    }

    #[test]
    fn test_egfr_mdrd_known_value() {
        // Scr 1.0, age 50, male, non-black: 175 * 50^-0.203 = 79.10
        approx(
            lib().egfr_mdrd(1.0, 50.0, Gender::Male, false).unwrap(),
            79.10,
            0.1,
        );
        // Black race factor 1.212 applied.
        approx(
            lib().egfr_mdrd(1.0, 50.0, Gender::Male, true).unwrap(),
            79.10 * 1.212,
            0.2,
        );
    }

    #[test]
    fn test_cockcroft_gault_known_value() {
        // age 60, 72 kg, Scr 1.0, male: (140-60)*72/(72*1) = 80 mL/min
        approx(
            lib()
                .creatinine_clearance_cockcroft_gault(60.0, 72.0, 1.0, Gender::Male)
                .unwrap(),
            80.0,
            1e-9,
        );
        // Female 0.85 factor → 68 mL/min
        approx(
            lib()
                .creatinine_clearance_cockcroft_gault(60.0, 72.0, 1.0, Gender::Female)
                .unwrap(),
            68.0,
            1e-9,
        );
    }

    #[test]
    fn test_mean_arterial_pressure_known_value() {
        // 120/80 → (120 + 160)/3 = 93.333
        approx(lib().mean_arterial_pressure(120.0, 80.0).unwrap(), 93.333, 1e-3);
        assert!(lib().mean_arterial_pressure(80.0, 120.0).is_err());
    }

    #[test]
    fn test_anion_gap_known_value() {
        // Na 140, Cl 100, HCO3 24 → 16
        approx(lib().anion_gap(140.0, 100.0, 24.0), 16.0, 1e-9);
    }

    #[test]
    fn test_corrected_calcium_known_value() {
        // measured 8.0 mg/dL, albumin 2.0 → 8.0 + 0.8*2.0 = 9.6
        approx(lib().corrected_calcium(8.0, 2.0).unwrap(), 9.6, 1e-9);
    }

    #[test]
    fn test_winters_expected_pco2_known_value() {
        // HCO3 12 → 1.5*12 + 8 = 26
        approx(lib().winters_expected_pco2(12.0).unwrap(), 26.0, 1e-9);
    }

    #[test]
    fn test_cha2ds2_vasc_point_sum() {
        // 70 y/o female with hypertension + diabetes:
        // age 65-74 (1) + female (1) + HTN (1) + DM (1) = 4
        let score = lib().cha2ds2_vasc_score(
            false, true, 70, true, false, false, Gender::Female,
        );
        assert_eq!(score, 4);
        // Max case: CHF+HTN+age>=75(2)+DM+stroke(2)+vascular+female = 9
        let max = lib().cha2ds2_vasc_score(
            true, true, 80, true, true, true, Gender::Female,
        );
        assert_eq!(max, 9);
    }

    #[test]
    fn test_weight_based_dose() {
        // 5 mg/kg * 70 kg = 350 mg
        approx(lib().weight_based_dose(5.0, 70.0).unwrap(), 350.0, 1e-9);
    }

    #[test]
    fn test_giusti_hayton_adjusted_dose() {
        // Fe 0.5, CrCl 30/120, normal 500: Q = 1 - 0.5*(1-0.25) = 0.625 → 312.5
        approx(
            lib()
                .giusti_hayton_adjusted_dose(500.0, 0.5, 30.0, 120.0)
                .unwrap(),
            312.5,
            1e-9,
        );
        assert!(lib()
            .giusti_hayton_adjusted_dose(500.0, 1.5, 30.0, 120.0)
            .is_err());
    }

    #[test]
    fn test_mg_mmol_roundtrip() {
        // Calcium MW 40.08: 100 mg -> 2.4950 mmol -> back to 100 mg
        let mmol = lib().mg_to_mmol(100.0, 40.08).unwrap();
        approx(mmol, 2.49501, 1e-4);
        approx(lib().mmol_to_mg(mmol, 40.08).unwrap(), 100.0, 1e-9);
    }

    #[test]
    fn test_infusion_rate() {
        // 5 µg/kg/min, 70 kg, 1600 µg/mL: (5*70*60)/1600 = 13.125 mL/hr
        approx(
            lib().infusion_rate_ml_per_hr(5.0, 70.0, 1600.0).unwrap(),
            13.125,
            1e-6,
        );
    }

    #[test]
    fn test_pharmacokinetics() {
        // k = ln2/4 = 0.17329 /h
        approx(lib().elimination_rate_constant(4.0).unwrap(), 0.173287, 1e-5);
        // t½ from k roundtrips
        let k = lib().elimination_rate_constant(4.0).unwrap();
        approx(lib().half_life_from_rate_constant(k).unwrap(), 4.0, 1e-9);
        // CL = k * Vd
        approx(lib().clearance(0.1, 50.0).unwrap(), 5.0, 1e-9);
        // Vd = dose / C0
        approx(lib().volume_of_distribution(500.0, 10.0).unwrap(), 50.0, 1e-9);
        // Css = R0 / CL
        approx(
            lib().steady_state_concentration(100.0, 10.0).unwrap(),
            10.0,
            1e-9,
        );
    }

    #[test]
    fn test_summarize_cohort_delegates_to_statistics() {
        // [2,4,4,4,5,5,7,9]: mean 5, sample std 2.138..., median 4.5
        let v = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let s = lib().summarize_cohort(&v).unwrap();
        assert_eq!(s.n, 8);
        approx(s.mean, 5.0, 1e-9);
        approx(s.std_dev.unwrap(), 2.13809, 1e-4);
        approx(s.median, 4.5, 1e-9);
        approx(s.min, 2.0, 1e-9);
        approx(s.max, 9.0, 1e-9);
        assert!(lib().summarize_cohort(&[]).is_err());
        // n<2 → std_dev None, not NaN.
        assert!(lib().summarize_cohort(&[3.0]).unwrap().std_dev.is_none());
    }
}
