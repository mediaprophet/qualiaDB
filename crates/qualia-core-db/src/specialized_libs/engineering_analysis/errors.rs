/// Engineering error types
#[derive(Debug, Clone)]
pub enum EngineeringError {
    ValidationError(String),
    ModelError(String),
    SolverError(String),
    ConvergenceError(String),
    DataError(String),
    AnalysisError(String),
    /// The capability is not implemented yet — returned instead of a fabricated result.
    NotImplemented(String),
    /// The required input (material, geometry, loads, BCs, reference data) is not present.
    InsufficientData(String),
}

impl std::fmt::Display for EngineeringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineeringError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            EngineeringError::ModelError(msg) => write!(f, "Model error: {}", msg),
            EngineeringError::SolverError(msg) => write!(f, "Solver error: {}", msg),
            EngineeringError::ConvergenceError(msg) => write!(f, "Convergence error: {}", msg),
            EngineeringError::DataError(msg) => write!(f, "Data error: {}", msg),
            EngineeringError::AnalysisError(msg) => write!(f, "Analysis error: {}", msg),
            EngineeringError::NotImplemented(msg) => write!(f, "Not implemented yet: {}", msg),
            EngineeringError::InsufficientData(msg) => {
                write!(f, "Required information not available: {}", msg)
            }
        }
    }
}

impl std::error::Error for EngineeringError {}
