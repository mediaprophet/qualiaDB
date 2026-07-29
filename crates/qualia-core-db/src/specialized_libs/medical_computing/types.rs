use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
