//! Pure data types, enums, handles, and error type for ambient orchestration.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Ambient device information
#[derive(Debug, Clone)]
pub struct AmbientDevice {
    pub device_id: String,
    pub device_type: DeviceType,
    pub capabilities: DeviceCapabilities,
    pub current_state: DeviceState,
    pub performance_profile: PerformanceProfile,
    pub power_profile: PowerProfile,
}

/// Device types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DeviceType {
    Mobile,
    Tablet,
    Wearable,
    IoT,
    Embedded,
}

/// Device capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub neural_engines: Vec<NeuralEngine>,
    pub compute_units: u32,
    pub memory_size: u64,
    pub battery_capacity: u64,
    pub thermal_limit: f64,
    pub supported_frameworks: Vec<Framework>,
}

/// Neural engine types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NeuralEngine {
    NNAPI,
    CoreML,
    TensorFlowLite,
    PyTorchMobile,
    ONNXRuntime,
}

/// ML Frameworks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Framework {
    TensorFlow,
    PyTorch,
    CoreML,
    ONNX,
    Custom(String),
}

/// Device state
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DeviceState {
    Active,
    Idle,
    Suspended,
    ThermalThrottled,
    BatteryLow,
    Offline,
}

/// Performance profile
#[derive(Debug, Clone)]
pub struct PerformanceProfile {
    pub peak_performance: f64,
    pub sustainable_performance: f64,
    pub thermal_performance: f64,
    pub battery_performance: f64,
    pub efficiency_factor: f64,
}

/// Power profile
#[derive(Debug, Clone)]
pub struct PowerProfile {
    pub baseline_power: f64,
    pub active_power: f64,
    pub peak_power: f64,
    pub idle_power: f64,
    pub sleep_power: f64,
}

/// Orchestration policies
#[derive(Debug, Clone)]
pub enum OrchestrationPolicy {
    /// Performance-first orchestration
    PerformanceFirst,
    /// Power-efficiency first
    PowerEfficiency,
    /// Thermal-aware orchestration
    ThermalAware,
    /// Battery-aware orchestration
    BatteryAware,
    /// Adaptive orchestration
    Adaptive,
}

/// Workload sample
#[derive(Debug, Clone)]
pub struct WorkloadSample {
    pub timestamp: Instant,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub neural_engine_usage: f64,
    pub power_consumption: f64,
    pub thermal_state: f64,
    pub battery_level: f64,
}

/// Model types
#[derive(Debug, Clone, PartialEq)]
pub enum ModelType {
    LinearRegression,
    NeuralNetwork,
    TimeSeries,
    Ensemble,
}

/// Model parameters
#[derive(Debug, Clone)]
pub struct ModelParameters {
    pub weights: Vec<f64>,
    pub biases: Vec<f64>,
    pub learning_rate: f64,
}

/// Allocation strategies
#[derive(Debug, Clone)]
pub enum AllocationStrategy {
    /// Round-robin allocation
    RoundRobin,
    /// Performance-based allocation
    PerformanceBased,
    /// Power-aware allocation
    PowerAware,
    /// Thermal-aware allocation
    ThermalAware,
    /// Multi-objective allocation
    MultiObjective,
}

/// Allocation record
#[derive(Debug, Clone)]
pub struct AllocationRecord {
    pub timestamp: Instant,
    pub device_id: String,
    pub resource_type: ResourceType,
    pub amount: u32,
    pub duration: Duration,
    pub efficiency: f64,
}

/// Resource types
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceType {
    ComputeUnit,
    Memory,
    NeuralEngine,
    Battery,
    Thermal,
}

/// Adaptation strategies
#[derive(Debug, Clone)]
pub enum AdaptationStrategy {
    /// No adaptation
    Static,
    /// Rule-based adaptation
    RuleBased,
    /// Machine learning adaptation
    MachineLearning,
    /// Hybrid adaptation
    Hybrid,
}

/// Adaptation record
#[derive(Debug, Clone)]
pub struct AdaptationRecord {
    pub timestamp: Instant,
    pub trigger: AdaptationTrigger,
    pub action: AdaptationAction,
    pub result: AdaptationResult,
}

/// Adaptation triggers
#[derive(Debug, Clone, PartialEq)]
pub enum AdaptationTrigger {
    ThermalThreshold,
    BatteryThreshold,
    PerformanceThreshold,
    WorkloadChange,
    UserPreference,
}

/// Adaptation actions
#[derive(Debug, Clone, PartialEq)]
pub enum AdaptationAction {
    ScaleUp,
    ScaleDown,
    Migrate,
    Suspend,
    Resume,
}

/// Adaptation results
#[derive(Debug, Clone, PartialEq)]
pub enum AdaptationResult {
    Success,
    Failure,
    Partial,
    Timeout,
}

/// Power policies
#[derive(Debug, Clone)]
pub enum PowerPolicy {
    /// Maximum performance
    MaxPerformance,
    /// Balanced mode
    Balanced,
    /// Power saving
    PowerSaving,
    /// Ultra power saving
    UltraPowerSaving,
    /// Custom power policy
    Custom(PowerPolicyConfig),
}

/// Power policy configuration
#[derive(Debug, Clone)]
pub struct PowerPolicyConfig {
    pub max_power: f64,
    pub target_battery_life: Duration,
    pub thermal_threshold: f64,
    pub performance_target: f64,
}

/// Thermal states
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThermalState {
    Normal,
    Warm,
    Hot,
    Critical,
}

/// Ambient orchestration state machine used for power estimation.
///
/// Mirrors the `ModelLifecycle` states from `orchestrator.rs` that are relevant
/// to mobile power budgeting. This is NOT a hot-path type (Vec/HashMap/String
/// are acceptable in this module).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbientOrchestrationState {
    /// Device idle — no active inference, model streaming, or scrubbing.
    Idle,
    /// Active inference running on one or more models.
    ActiveInference,
    /// Background scrubbing / memory compaction pass.
    Scrubbing,
    /// Streaming model weights into VRAM/resident memory.
    Streaming,
}

/// Aggregated power/thermal/battery snapshot for battery-aware ML scheduling.
#[derive(Debug, Clone, PartialEq)]
pub struct PowerMetrics {
    /// Current estimated power draw in watts.
    pub current_power_w: f64,
    /// Thermal state derived from the current power draw.
    pub thermal_state: ThermalState,
    /// Estimated battery life remaining in hours, if a battery is present.
    pub estimated_battery_hours: Option<f64>,
    /// Number of models currently active on the device.
    pub active_model_count: usize,
}

/// Optimization algorithms
#[derive(Debug, Clone)]
pub enum OptimizationAlgorithm {
    Greedy,
    Genetic,
    SimulatedAnnealing,
    ReinforcementLearning,
}

/// Optimization record
#[derive(Debug, Clone)]
pub struct OptimizationRecord {
    pub timestamp: Instant,
    pub algorithm: OptimizationAlgorithm,
    pub input_state: PowerState,
    pub output_state: PowerState,
    pub efficiency_gain: f64,
}

/// Power state
#[derive(Debug, Clone)]
pub struct PowerState {
    pub power_consumption: f64,
    pub performance: f64,
    pub efficiency: f64,
    pub thermal_state: ThermalState,
    pub battery_level: f64,
}

/// Scheduling policies
#[derive(Debug, Clone)]
pub enum SchedulingPolicy {
    /// First-In-First-Out
    Fifo,
    /// Priority-based scheduling
    Priority,
    /// Shortest-Job-First
    ShortestJobFirst,
    /// Deadline-Driven Scheduling
    Deadline,
    /// Adaptive scheduling
    Adaptive,
}

/// Task
#[derive(Debug, Clone)]
pub struct Task {
    pub task_id: String,
    pub task_type: TaskType,
    pub priority: TaskPriority,
    pub resource_requirements: ResourceRequirements,
    pub deadline: Option<Instant>,
    pub estimated_duration: Duration,
    pub dependencies: Vec<String>,
}

/// Task types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TaskType {
    NeuralInference,
    ModelTraining,
    DataProcessing,
    MathematicalComputation,
    SensorProcessing,
}

/// Task priorities
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Resource requirements
#[derive(Debug, Clone)]
pub struct ResourceRequirements {
    pub compute_units: u32,
    pub memory: u64,
    pub neural_engines: u32,
    pub power_budget: f64,
    pub thermal_budget: f64,
}

/// Task execution record
#[derive(Debug, Clone)]
pub struct TaskExecutionRecord {
    pub task_id: String,
    pub device_id: String,
    pub start_time: Instant,
    pub end_time: Instant,
    pub actual_duration: Duration,
    pub success: bool,
    pub resource_usage: ResourceUsage,
}

/// Resource usage
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub compute_units_used: u32,
    pub memory_used: u64,
    pub neural_engines_used: u32,
    pub power_consumed: f64,
    pub thermal_impact: f64,
}

/// Device metrics
#[derive(Debug, Clone)]
pub struct DeviceMetrics {
    pub device_id: String,
    pub utilization: f64,
    pub throughput: f64,
    pub latency: f64,
    pub power_efficiency: f64,
    pub thermal_efficiency: f64,
}

/// Task metrics
#[derive(Debug, Clone)]
pub struct TaskMetrics {
    pub task_id: String,
    pub execution_time: Duration,
    pub resource_efficiency: f64,
    pub success_rate: f64,
    pub retry_count: u32,
}

/// Global metrics
#[derive(Debug, Clone)]
pub struct AmbientGlobalMetrics {
    pub total_tasks_processed: u64,
    pub average_execution_time: Duration,
    pub overall_efficiency: f64,
    pub power_savings: f64,
    pub thermal_compliance: f64,
    pub device_utilization: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmbientDeviceHandle {
    pub device_id_hash: u64,
    pub device_type: DeviceType,
    pub compute_units: u32,
    pub memory_size: u64,
    pub state: DeviceState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaskHandle {
    pub task_id_hash: u64,
    pub task_type: TaskType,
    pub priority: TaskPriority,
    pub compute_units: u32,
    pub memory: u64,
}

#[derive(Debug, Clone)]
pub struct SubThresholdComputation {
    pub computation_id: String,
    pub computation_type: ComputationType,
    pub resource_requirements: ResourceRequirements,
    pub optimization_level: OptimizationLevel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComputationType {
    MatrixMultiply,
    Convolution,
    NeuralNetwork,
    DataProcessing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationLevel {
    None,
    Basic,
    Advanced,
    Aggressive,
}

#[derive(Debug, Clone)]
pub struct ComputationResult {
    pub result_data: Vec<u8>,
    pub execution_time: Duration,
    pub power_consumed: f64,
    pub thermal_impact: f64,
}

#[derive(Debug, Clone)]
pub struct DeviceStatus {
    pub device_id: String,
    pub device_type: DeviceType,
    pub state: DeviceState,
    pub battery_level: f64,
    pub thermal_state: ThermalState,
    pub performance: PerformanceProfile,
    pub power_consumption: f64,
}

#[derive(Debug, Clone)]
pub struct WorkloadAnalysis {
    pub current_load: f64,
    pub predicted_load: f64,
    pub resource_pressure: f64,
    pub thermal_pressure: f64,
    pub battery_pressure: f64,
}

/// Ambient error types
#[derive(Debug, Clone)]
pub enum AmbientError {
    DeviceNotFound(String),
    InvalidDevice(String),
    InvalidTask(String),
    UnsupportedOperation(String),
    InsufficientResources(String),
    OrchestrationError(String),
    PowerError(String),
    ThermalError(String),
}

impl std::fmt::Display for AmbientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AmbientError::DeviceNotFound(msg) => write!(f, "Device not found: {}", msg),
            AmbientError::InvalidDevice(msg) => write!(f, "Invalid device: {}", msg),
            AmbientError::InvalidTask(msg) => write!(f, "Invalid task: {}", msg),
            AmbientError::UnsupportedOperation(msg) => write!(f, "Unsupported operation: {}", msg),
            AmbientError::InsufficientResources(msg) => {
                write!(f, "Insufficient resources: {}", msg)
            }
            AmbientError::OrchestrationError(msg) => write!(f, "Orchestration error: {}", msg),
            AmbientError::PowerError(msg) => write!(f, "Power error: {}", msg),
            AmbientError::ThermalError(msg) => write!(f, "Thermal error: {}", msg),
        }
    }
}

impl std::error::Error for AmbientError {}
