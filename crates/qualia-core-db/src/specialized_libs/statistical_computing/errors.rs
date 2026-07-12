

// Supporting types

#[derive(Debug, Clone)]
pub struct TTestResult {
    pub t_statistic: f64,
    pub p_value: f64,
    pub degrees_of_freedom: u32,
    pub confidence_interval: (f64, f64),
}

#[derive(Debug, Clone)]
pub struct HistogramResult {
    pub bins: usize,
    pub counts: Vec<u32>,
    pub min_value: f64,
    pub max_value: f64,
    pub bin_width: f64,
}

/// Modal value of a column, with the frequency of that value.
#[derive(Debug, Clone)]
pub struct ModeResult {
    pub value: f64,
    pub count: usize,
    pub sample_size: usize,
}

/// Fitted polynomial-regression model: coefficients (ascending powers, so
/// `coefficients[0]` is the constant term) plus the coefficient of determination.
#[derive(Debug, Clone)]
pub struct PolynomialFit {
    pub degree: usize,
    pub coefficients: Vec<f64>,
    pub r_squared: f64,
    pub n: usize,
}

/// Summary of a fitted soft-margin SVM (the model's internals stay in the solver).
#[derive(Debug, Clone)]
pub struct SvmFitResult {
    pub n_support_vectors: usize,
    pub train_accuracy: f64,
    pub n: usize,
    pub n_features: usize,
}

/// Summary of a fitted random forest, with the in-sample fit metric
/// (R² for a regression forest, classification accuracy for a classifier).
#[derive(Debug, Clone)]
pub struct RandomForestFitResult {
    pub n_trees: usize,
    pub classifier: bool,
    /// R² (regressor) or accuracy in `[0,1]` (classifier), measured in-sample.
    pub train_metric: f64,
    pub n: usize,
    pub n_features: usize,
}

/// Statistical error types
#[derive(Debug, Clone)]
pub enum StatisticalError {
    InvalidData(String),
    InvalidColumn(String),
    InvalidOperation(String),
    StorageError(String),
    ComputationError(String),
    PrivacyError(String),
    AnalysisError(String),
    DataNotFound(String),
}

impl std::fmt::Display for StatisticalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatisticalError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            StatisticalError::InvalidColumn(msg) => write!(f, "Invalid column: {}", msg),
            StatisticalError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            StatisticalError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            StatisticalError::ComputationError(msg) => write!(f, "Computation error: {}", msg),
            StatisticalError::PrivacyError(msg) => write!(f, "Privacy error: {}", msg),
            StatisticalError::AnalysisError(msg) => write!(f, "Analysis error: {}", msg),
            StatisticalError::DataNotFound(msg) => write!(f, "Dataset not found: {}", msg),
        }
    }
}

impl std::error::Error for StatisticalError {}
