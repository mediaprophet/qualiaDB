use super::*;

/// Statistical result
#[derive(Debug, Clone)]
pub struct StatisticalResult {
    pub result_id: String,
    pub result_type: ResultType,
    pub value: ResultValue,
    pub metadata: ResultMetadata,
}

/// Result types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResultType {
    Scalar,
    Vector,
    Matrix,
    Distribution,
    Model,
}

/// Result values
#[derive(Debug, Clone)]
pub enum ResultValue {
    Float(f64),
    Integer(i64),
    Boolean(bool),
    Vector(Vec<f64>),
    Matrix(Vec<Vec<f64>>),
    Distribution(Distribution),
    Model(StatisticalModel),
}

/// Statistical distribution
#[derive(Debug, Clone)]
pub struct Distribution {
    pub distribution_type: DistributionType,
    pub parameters: Vec<f64>,
    pub samples: Option<Vec<f64>>,
}

/// Distribution types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DistributionType {
    Normal,
    Uniform,
    Exponential,
    Poisson,
    Binomial,
    ChiSquare,
    StudentT,
    F,
    Custom(String),
}

/// Statistical model
#[derive(Debug, Clone)]
pub struct StatisticalModel {
    pub model_type: ModelType,
    pub parameters: ModelParameters,
    pub performance_metrics: ModelPerformance,
}

/// Model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelType {
    LinearRegression,
    LogisticRegression,
    PolynomialRegression,
    KMeans,
    SVM,
    RandomForest,
    NeuralNetwork,
    Custom(String),
}

/// Model parameters
#[derive(Debug, Clone)]
pub struct ModelParameters {
    pub coefficients: Vec<f64>,
    pub intercept: f64,
    pub additional_params: HashMap<String, f64>,
}

/// Model performance metrics
#[derive(Debug, Clone)]
pub struct ModelPerformance {
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
    pub mse: f64,
    pub rmse: f64,
    pub r_squared: f64,
}

/// Result metadata
#[derive(Debug, Clone)]
pub struct ResultMetadata {
    pub computation_time: u64,
    pub memory_usage: u64,
    pub sample_size: usize,
    pub confidence_interval: Option<(f64, f64)>,
    pub significance_level: Option<f64>,
    pub privacy_preserved: bool,
}

/// Load balancing strategies
#[derive(Debug, Clone, PartialEq)]
pub enum BalancingStrategy {
    RoundRobin,
    LoadBased,
    CapacityWeighted,
    LeastConnections,
}

