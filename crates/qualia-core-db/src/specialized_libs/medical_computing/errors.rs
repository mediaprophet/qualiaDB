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
