use super::*;
use std::collections::HashMap;

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
impl PatientManager {
    /// Creates a `PatientManager` wiring fresh `PatientRecords`, `MedicalHistory`, `PrivacyProtection`, and `DataAccessControl` components.
    pub fn new() -> Self {
        Self {
            patient_records: PatientRecords::new(),
            medical_history: MedicalHistory::new(),
            privacy_protection: PrivacyProtection::new(),
            data_access: DataAccessControl::new(),
        }
    }

    /// Initializes the records store, privacy protection, and access-control components; `MedicalError` propagates from any failing component.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        self.patient_records.initialize()?;
        self.privacy_protection.initialize()?;
        self.data_access.initialize()?;
        Ok(())
    }

    /// Fails closed on an empty `patient_id` or `medical_record_number`; field-level validation is otherwise minimal.
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

    /// Validates nothing itself — stores `patient` into the records store and returns it. Call `validate_patient` first for checks.
    pub fn create_patient(&mut self, patient: Patient) -> Result<Patient, MedicalError> {
        // Create patient record
        self.patient_records.store_patient(patient.clone())?;
        Ok(patient)
    }

    /// Returns the stored `Patient` clone or `PatientError("Patient not found")`.
    pub fn get_patient(&self, patient_id: &str) -> Result<Patient, MedicalError> {
        self.patient_records.get_patient(patient_id)
    }

    /// All patient IDs currently held in the records store.
    pub fn list_patients(&self) -> Vec<String> {
        self.patient_records.list_patients()
    }

    /// Returns a fresh zeroed `MedicalPerformanceMetrics` — scaffold-level reporting, not measured values.
    pub fn get_performance_metrics(&self) -> MedicalPerformanceMetrics {
        MedicalPerformanceMetrics::new()
    }

    /// Shared reference to the in-memory history accumulated via `add_condition`/`add_surgery`.
    pub fn get_medical_history(&self) -> &MedicalHistory {
        &self.medical_history
    }

    /// Appends `condition` to the medical history (no validation or dedup).
    pub fn add_condition(&mut self, condition: MedicalCondition) {
        self.medical_history.conditions.push(condition);
    }

    /// Appends `surgery` to the medical history.
    pub fn add_surgery(&mut self, surgery: Surgery) {
        self.medical_history.surgeries.push(surgery);
    }

    /// Conditions whose status is `Active` or `Chronic`.
    pub fn active_conditions(&self) -> Vec<&MedicalCondition> {
        self.medical_history
            .conditions
            .iter()
            .filter(|c| c.status == ConditionStatus::Active || c.status == ConditionStatus::Chronic)
            .collect()
    }
}

impl PatientRecords {
    /// Creates an empty in-memory store (HashMaps for patients, demographics, identifiers).
    pub fn new() -> Self {
        Self {
            patients: HashMap::new(),
            demographics: HashMap::new(),
            medical_identifiers: HashMap::new(),
        }
    }

    /// No-op — in-memory store needs no setup; returns `Ok(())` honestly.
    pub fn initialize(&mut self) -> Result<(), MedicalError> {
        Ok(())
    }

    /// Inserts/updates `patient` keyed by `patient_id`.
    pub fn store_patient(&mut self, patient: Patient) -> Result<(), MedicalError> {
        self.patients.insert(patient.patient_id.clone(), patient);
        Ok(())
    }

    /// Cloned `Patient` or `PatientError("Patient not found")`.
    pub fn get_patient(&self, patient_id: &str) -> Result<Patient, MedicalError> {
        self.patients
            .get(patient_id)
            .cloned()
            .ok_or_else(|| MedicalError::PatientError("Patient not found".to_string()))
    }

    /// All stored patient IDs.
    pub fn list_patients(&self) -> Vec<String> {
        self.patients.keys().cloned().collect()
    }

    /// Associates `demographics` with `patient_id` (overwrites silently).
    pub fn store_demographics(&mut self, patient_id: &str, demographics: Demographics) {
        self.demographics
            .insert(patient_id.to_string(), demographics);
    }

    /// Borrowed `Demographics` for `patient_id`, or `None`.
    pub fn get_demographics(&self, patient_id: &str) -> Option<&Demographics> {
        self.demographics.get(patient_id)
    }

    /// Associates a `MedicalIdentifier` with `patient_id`.
    pub fn store_identifier(&mut self, patient_id: &str, identifier: MedicalIdentifier) {
        self.medical_identifiers
            .insert(patient_id.to_string(), identifier);
    }

    /// Borrowed `MedicalIdentifier`, or `None`.
    pub fn get_identifier(&self, patient_id: &str) -> Option<&MedicalIdentifier> {
        self.medical_identifiers.get(patient_id)
    }

    /// Removes patient, demographics, and identifier; returns the removed `Patient` if present.
    pub fn remove_patient(&mut self, patient_id: &str) -> Option<Patient> {
        self.demographics.remove(patient_id);
        self.medical_identifiers.remove(patient_id);
        self.patients.remove(patient_id)
    }
}
