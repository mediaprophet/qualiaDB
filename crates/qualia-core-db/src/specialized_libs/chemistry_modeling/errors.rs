use super::*;

/// Chemistry error types
#[derive(Debug, Clone)]
pub enum ChemistryError {
    ValidationError(String),
    SimulationError(String),
    QuantumError(String),
    ReactionError(String),
    PropertyError(String),
    DataError(String),
    ConvergenceError(String),
    /// The capability is not implemented yet — returned instead of a fabricated result.
    NotImplemented(String),
    /// The required input (parameters, reference data, a model) is not present.
    InsufficientData(String),
}

impl std::fmt::Display for ChemistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChemistryError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ChemistryError::SimulationError(msg) => write!(f, "Simulation error: {}", msg),
            ChemistryError::QuantumError(msg) => write!(f, "Quantum error: {}", msg),
            ChemistryError::ReactionError(msg) => write!(f, "Reaction error: {}", msg),
            ChemistryError::PropertyError(msg) => write!(f, "Property error: {}", msg),
            ChemistryError::DataError(msg) => write!(f, "Data error: {}", msg),
            ChemistryError::ConvergenceError(msg) => write!(f, "Convergence error: {}", msg),
            ChemistryError::NotImplemented(msg) => write!(f, "Not implemented yet: {}", msg),
            ChemistryError::InsufficientData(msg) => {
                write!(f, "Required information not available: {}", msg)
            }
        }
    }
}

impl std::error::Error for ChemistryError {}
