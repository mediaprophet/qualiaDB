
/// Physics error types
#[derive(Debug, Clone)]
pub enum PhysicsError {
    InvalidConfiguration(String),
    SolverError(String),
    MeshError(String),
    DataError(String),
    ConvergenceError(String),
    PerformanceError(String),
    NetworkError(String),
    DistributedError(String),
}

impl std::fmt::Display for PhysicsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhysicsError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            PhysicsError::SolverError(msg) => write!(f, "Solver error: {}", msg),
            PhysicsError::MeshError(msg) => write!(f, "Mesh error: {}", msg),
            PhysicsError::DataError(msg) => write!(f, "Data error: {}", msg),
            PhysicsError::ConvergenceError(msg) => write!(f, "Convergence error: {}", msg),
            PhysicsError::PerformanceError(msg) => write!(f, "Performance error: {}", msg),
            PhysicsError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            PhysicsError::DistributedError(msg) => write!(f, "Distributed error: {}", msg),
        }
    }
}

impl std::error::Error for PhysicsError {}
