//! Ambient Sub-Threshold Orchestration Implementation
//!
//! This module provides ambient sub-threshold orchestration for mobile scientific computing
//! using NNAPI/CoreML integration. Designed for edge optimization and power-efficient processing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

/// Ambient Orchestration Manager
pub struct AmbientOrchestrationManager {
    devices: HashMap<String, AmbientDevice>,
    orchestrator: SubThresholdOrchestrator,
    power_manager: PowerManager,
    performance_monitor: AmbientPerformanceMonitor,
    task_scheduler: TaskScheduler,
}

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

/// Sub-threshold orchestrator
pub struct SubThresholdOrchestrator {
    orchestration_policy: OrchestrationPolicy,
    workload_analyzer: WorkloadAnalyzer,
    resource_allocator: ResourceAllocator,
    adaptation_engine: AdaptationEngine,
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

/// Workload analyzer
pub struct WorkloadAnalyzer {
    workload_history: Vec<WorkloadSample>,
    prediction_model: PredictionModel,
    analysis_window: Duration,
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

/// Prediction model for workload
#[derive(Debug, Clone)]
pub struct PredictionModel {
    pub model_type: ModelType,
    pub parameters: ModelParameters,
    pub accuracy: f64,
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

/// Resource allocator
pub struct ResourceAllocator {
    allocation_strategy: AllocationStrategy,
    resource_pool: ResourcePool,
    allocation_history: Vec<AllocationRecord>,
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

/// Resource pool
#[derive(Debug, Clone)]
pub struct ResourcePool {
    pub total_compute_units: u32,
    pub available_compute_units: u32,
    pub total_memory: u64,
    pub available_memory: u64,
    pub total_neural_engines: u32,
    pub available_neural_engines: u32,
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

/// Adaptation engine
pub struct AdaptationEngine {
    adaptation_strategy: AdaptationStrategy,
    adaptation_history: Vec<AdaptationRecord>,
    learning_rate: f64,
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

/// Power manager
pub struct PowerManager {
    power_policy: PowerPolicy,
    battery_monitor: BatteryMonitor,
    thermal_monitor: ThermalMonitor,
    power_optimizer: PowerOptimizer,
    /// Current orchestration state, used to estimate power when no platform
    /// power API is available.
    orchestration_state: AmbientOrchestrationState,
    /// Number of models currently loaded/active on the managed device.
    active_model_count: usize,
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

/// Battery monitor
pub struct BatteryMonitor {
    current_level: f64,
    voltage: f64,
    temperature: f64,
    health: f64,
    charging: bool,
    estimated_time_remaining: Duration,
}

/// Thermal monitor
pub struct ThermalMonitor {
    cpu_temperature: f64,
    gpu_temperature: f64,
    battery_temperature: f64,
    ambient_temperature: f64,
    thermal_state: ThermalState,
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

/// Power optimizer
pub struct PowerOptimizer {
    optimization_algorithm: OptimizationAlgorithm,
    optimization_history: Vec<OptimizationRecord>,
    target_efficiency: f64,
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

/// Task scheduler
pub struct TaskScheduler {
    scheduling_policy: SchedulingPolicy,
    task_queue: TaskQueue,
    execution_history: Vec<TaskExecutionRecord>,
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

/// Task queue
pub struct TaskQueue {
    pending_tasks: Vec<Task>,
    running_tasks: Vec<Task>,
    completed_tasks: Vec<Task>,
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

/// Ambient performance monitor
pub struct AmbientPerformanceMonitor {
    device_metrics: HashMap<String, DeviceMetrics>,
    task_metrics: HashMap<String, TaskMetrics>,
    global_metrics: AmbientGlobalMetrics,
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

impl AmbientOrchestrationManager {
    /// Create new ambient orchestration manager
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            orchestrator: SubThresholdOrchestrator::new(),
            power_manager: PowerManager::new(),
            performance_monitor: AmbientPerformanceMonitor::new(),
            task_scheduler: TaskScheduler::new(),
        }
    }

    /// Register ambient device
    pub fn register_device(&mut self, device: AmbientDevice) -> Result<(), AmbientError> {
        // Validate device
        self.validate_device(&device)?;

        // Store device
        self.devices.insert(device.device_id.clone(), device);

        Ok(())
    }

    /// Discover ambient devices using sysinfo to enumerate real local hardware.
    pub fn discover_devices(&mut self) -> Result<Vec<String>, AmbientError> {
        let mut handles = [AmbientDeviceHandle {
            device_id_hash: 0,
            device_type: DeviceType::Embedded,
            compute_units: 0,
            memory_size: 0,
            state: DeviceState::Offline,
        }; 9];
        let written = self.discover_devices_into(&mut handles)?;
        let mut discovered = Vec::with_capacity(written);
        for handle in handles.into_iter().take(written) {
            if handle.device_id_hash == crate::q_hash("local_host") {
                discovered.push("local_host".to_string());
            } else {
                discovered.push(format!("cpu_core_{}", discovered.len().saturating_sub(1)));
            }
        }
        Ok(discovered)
    }

    /// Zero-heap device discovery snapshots for hot-path schedulers.
    pub fn discover_devices_into(
        &mut self,
        out: &mut [AmbientDeviceHandle],
    ) -> Result<usize, AmbientError> {
        use sysinfo::System;

        // sysinfo 0.39: explicitly refresh after construction before reading
        // CPU/memory (matches the ingest.rs pattern). H0 hardware discovery
        // depends on these values being populated.
        let mut sys = System::new_all();
        sys.refresh_all();
        let mut discovered = 0usize;
        self.devices.clear();

        let cpus = sys.cpus();
        let cpu_count = cpus.len().max(1);
        let cpu_brand = cpus
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());
        let base_freq_mhz = cpus.first().map(|c| c.frequency()).unwrap_or(1000);
        let total_mem = sys.total_memory(); // bytes

        // Register the local machine as one aggregate compute device
        let host_id = "local_host".to_string();
        let host = AmbientDevice {
            device_id: host_id.clone(),
            device_type: DeviceType::Embedded,
            capabilities: DeviceCapabilities {
                neural_engines: vec![NeuralEngine::ONNXRuntime],
                compute_units: cpu_count as u32,
                memory_size: total_mem,
                battery_capacity: 0,
                thermal_limit: 95.0,
                supported_frameworks: vec![Framework::ONNX, Framework::Custom(cpu_brand.clone())],
            },
            current_state: DeviceState::Active,
            performance_profile: PerformanceProfile {
                peak_performance: base_freq_mhz as f64 * cpu_count as f64 / 1000.0,
                sustainable_performance: base_freq_mhz as f64 * cpu_count as f64 * 0.8 / 1000.0,
                thermal_performance: base_freq_mhz as f64 * cpu_count as f64 * 0.6 / 1000.0,
                battery_performance: 1.0,
                efficiency_factor: 0.90,
            },
            power_profile: PowerProfile {
                baseline_power: 5.0 * cpu_count as f64,
                active_power: 15.0 * cpu_count as f64,
                peak_power: 35.0 * cpu_count as f64,
                idle_power: 1.0,
                sleep_power: 0.5,
            },
        };
        if self.register_device(host).is_ok() {
            if discovered >= out.len() {
                return Err(AmbientError::InsufficientResources(
                    "device output buffer full".to_string(),
                ));
            }
            out[discovered] = self.snapshot_device_handle("local_host").unwrap();
            discovered += 1;
        }

        // Register up to 8 individual logical CPU cores for fine-grained scheduling
        for i in 0..cpu_count.min(8) {
            let core_freq = cpus.get(i).map(|c| c.frequency()).unwrap_or(base_freq_mhz);
            let core_id = format!("cpu_core_{}", i);
            let core = AmbientDevice {
                device_id: core_id.clone(),
                device_type: DeviceType::Embedded,
                capabilities: DeviceCapabilities {
                    neural_engines: vec![NeuralEngine::ONNXRuntime],
                    compute_units: 1,
                    memory_size: total_mem / cpu_count as u64,
                    battery_capacity: 0,
                    thermal_limit: 95.0,
                    supported_frameworks: vec![Framework::ONNX],
                },
                current_state: DeviceState::Active,
                performance_profile: PerformanceProfile {
                    peak_performance: core_freq as f64 / 1000.0,
                    sustainable_performance: core_freq as f64 * 0.8 / 1000.0,
                    thermal_performance: core_freq as f64 * 0.6 / 1000.0,
                    battery_performance: 1.0,
                    efficiency_factor: 0.85,
                },
                power_profile: PowerProfile {
                    baseline_power: 5.0,
                    active_power: 15.0,
                    peak_power: 35.0,
                    idle_power: 1.0,
                    sleep_power: 0.5,
                },
            };
            if self.register_device(core).is_ok() {
                if discovered >= out.len() {
                    return Err(AmbientError::InsufficientResources(
                        "device output buffer full".to_string(),
                    ));
                }
                out[discovered] = self.snapshot_device_handle(&core_id).unwrap();
                discovered += 1;
            }
        }

        Ok(discovered)
    }

    /// Submit task for execution
    pub fn submit_task(&mut self, task: Task) -> Result<String, AmbientError> {
        // Validate task
        self.validate_task(&task)?;

        // Add to task queue
        self.task_scheduler.submit_task(task.clone())?;

        Ok(task.task_id.clone())
    }

    /// Execute neural inference task
    pub fn execute_neural_inference(
        &mut self,
        device_id: &str,
        model_data: &[u8],
        input_data: &[u8],
    ) -> Result<Vec<u8>, AmbientError> {
        let mut out = vec![0u8; 1024];
        let written =
            self.execute_neural_inference_into(device_id, model_data, input_data, &mut out)?;
        out.truncate(written);
        Ok(out)
    }

    /// Zero-heap neural inference API using caller-owned output storage.
    pub fn execute_neural_inference_into(
        &mut self,
        device_id: &str,
        model_data: &[u8],
        input_data: &[u8],
        out: &mut [u8],
    ) -> Result<usize, AmbientError> {
        // Clone device to release the borrow on self.devices before calling the helper
        let device = self
            .devices
            .get(device_id)
            .ok_or_else(|| AmbientError::DeviceNotFound(device_id.to_string()))?
            .clone();
        self.execute_inference_on_device(&device, model_data, input_data, out)
    }

    /// Execute sub-threshold computation
    pub fn execute_sub_threshold_computation(
        &mut self,
        device_id: &str,
        computation: SubThresholdComputation,
    ) -> Result<ComputationResult, AmbientError> {
        // Clone device to release the borrow on self.devices before calling the helper
        let device = self
            .devices
            .get(device_id)
            .ok_or_else(|| AmbientError::DeviceNotFound(device_id.to_string()))?
            .clone();
        self.execute_computation_on_device(&device, &computation)
    }

    /// Get device status
    pub fn get_device_status(&self, device_id: &str) -> Option<DeviceStatus> {
        self.devices.get(device_id).map(|device| DeviceStatus {
            device_id: device.device_id.clone(),
            device_type: device.device_type.clone(),
            state: device.current_state.clone(),
            battery_level: self.power_manager.get_battery_level(device_id),
            thermal_state: self.power_manager.get_thermal_state(device_id),
            performance: device.performance_profile.clone(),
            power_consumption: self.power_manager.get_power_consumption(device_id),
        })
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> AmbientGlobalMetrics {
        self.performance_monitor.get_global_stats()
    }

    /// Set the ambient orchestration state used for power estimation.
    ///
    /// Callers (e.g. the `TaskOrchestrator` state machine in `orchestrator.rs`)
    /// should push `ModelLifecycle` transitions here so the power manager can
    /// track real power draw.
    pub fn set_orchestration_state(&mut self, state: AmbientOrchestrationState) {
        self.power_manager.set_orchestration_state(state);
    }

    /// Set the number of models currently active on the managed device.
    pub fn set_active_model_count(&mut self, count: usize) {
        self.power_manager.set_active_model_count(count);
    }

    /// Get the aggregated power/thermal/battery snapshot.
    pub fn get_power_metrics(&self) -> PowerMetrics {
        self.power_manager.get_power_metrics()
    }

    /// Estimate battery life remaining (hours) for the given charge and
    /// battery capacity.
    pub fn estimate_battery_life_remaining(
        &self,
        current_battery_pct: f64,
        battery_capacity_wh: f64,
    ) -> f64 {
        self.power_manager
            .estimate_battery_life_remaining(current_battery_pct, battery_capacity_wh)
    }

    /// Whether inference should be throttled due to thermal or battery pressure.
    pub fn should_throttle_inference(&self) -> bool {
        self.power_manager.should_throttle_inference()
    }

    /// List all devices
    pub fn list_devices(&self) -> Vec<String> {
        self.devices.keys().cloned().collect()
    }

    pub fn list_devices_into(
        &self,
        out: &mut [AmbientDeviceHandle],
    ) -> Result<usize, AmbientError> {
        if out.len() < self.devices.len() {
            return Err(AmbientError::InsufficientResources(
                "device output buffer full".to_string(),
            ));
        }

        let mut written = 0usize;
        for device_id in self.devices.keys() {
            out[written] = self.snapshot_device_handle(device_id).unwrap();
            written += 1;
        }
        Ok(written)
    }

    /// Get pending tasks
    pub fn get_pending_tasks(&self) -> Vec<Task> {
        self.task_scheduler.get_pending_tasks()
    }

    pub fn get_pending_tasks_into(&self, out: &mut [TaskHandle]) -> Result<usize, AmbientError> {
        self.task_scheduler.get_pending_tasks_into(out)
    }

    /// Optimize orchestration policy
    pub fn optimize_orchestration(&mut self) -> Result<(), AmbientError> {
        // Analyze current workload
        let workload_analysis = self.orchestrator.workload_analyzer.analyze_workload();

        // Adapt orchestration policy (mutable: records adaptation history)
        let new_policy = self
            .orchestrator
            .adaptation_engine
            .adapt_policy(workload_analysis);

        // Update orchestration policy
        self.orchestrator.orchestration_policy = new_policy;

        Ok(())
    }

    // Internal methods

    /// Validate device
    fn validate_device(&self, device: &AmbientDevice) -> Result<(), AmbientError> {
        if device.device_id.is_empty() {
            return Err(AmbientError::InvalidDevice(
                "Device ID cannot be empty".to_string(),
            ));
        }

        if device.capabilities.neural_engines.is_empty() {
            return Err(AmbientError::InvalidDevice(
                "Device must have at least one neural engine".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate task
    fn validate_task(&self, task: &Task) -> Result<(), AmbientError> {
        if task.task_id.is_empty() {
            return Err(AmbientError::InvalidTask(
                "Task ID cannot be empty".to_string(),
            ));
        }

        if task.resource_requirements.compute_units == 0 {
            return Err(AmbientError::InvalidTask(
                "Task must require at least one compute unit".to_string(),
            ));
        }

        Ok(())
    }

    /// Execute inference on device
    fn execute_inference_on_device(
        &self,
        device: &AmbientDevice,
        model_data: &[u8],
        input_data: &[u8],
        out: &mut [u8],
    ) -> Result<usize, AmbientError> {
        // In real implementation, would use NNAPI/CoreML for inference
        // For now, simulate inference
        let _ = (device, model_data, input_data);
        thread::sleep(Duration::from_millis(100)); // Simulate 100ms inference

        if out.len() < 1024 {
            return Err(AmbientError::InsufficientResources(
                "inference output buffer too small".to_string(),
            ));
        }
        out[..1024].fill(0);
        Ok(1024)
    }

    /// Execute computation on device
    fn execute_computation_on_device(
        &self,
        _device: &AmbientDevice,
        _computation: &SubThresholdComputation,
    ) -> Result<ComputationResult, AmbientError> {
        // In real implementation, would execute sub-threshold computation
        // For now, simulate computation
        thread::sleep(Duration::from_millis(50)); // Simulate 50ms computation

        Ok(ComputationResult {
            result_data: vec![0u8; 512],
            execution_time: Duration::from_millis(50),
            power_consumed: 0.1,
            thermal_impact: 0.5,
        })
    }

    fn snapshot_device_handle(&self, device_id: &str) -> Option<AmbientDeviceHandle> {
        self.devices
            .get(device_id)
            .map(|device| AmbientDeviceHandle {
                device_id_hash: crate::q_hash(&device.device_id),
                device_type: device.device_type.clone(),
                compute_units: device.capabilities.compute_units,
                memory_size: device.capabilities.memory_size,
                state: device.current_state.clone(),
            })
    }
}

impl SubThresholdOrchestrator {
    /// Create new sub-threshold orchestrator
    pub fn new() -> Self {
        Self {
            orchestration_policy: OrchestrationPolicy::Adaptive,
            workload_analyzer: WorkloadAnalyzer::new(),
            resource_allocator: ResourceAllocator::new(),
            adaptation_engine: AdaptationEngine::new(),
        }
    }

    /// Optimize computation for sub-threshold operation.
    ///
    /// Uses the resource allocator to check available compute units and
    /// scales the computation's resource requirements accordingly. The
    /// orchestration policy influences the reduction factor:
    /// - `PowerEfficiency`: 50% compute, 40% power, 50% thermal
    /// - `ThermalAware`: 60% compute, 50% power, 60% thermal
    /// - `PerformanceFirst`: 90% compute, 80% power, 80% thermal
    /// - `Adaptive`/other: 70% compute, 50% power, 60% thermal
    pub fn optimize_for_sub_threshold(
        &mut self,
        computation: SubThresholdComputation,
    ) -> SubThresholdComputation {
        let mut optimized = computation;

        let (compute_factor, power_factor, thermal_factor) = match self.orchestration_policy {
            OrchestrationPolicy::PowerEfficiency => (0.50, 0.40, 0.50),
            OrchestrationPolicy::ThermalAware => (0.60, 0.50, 0.60),
            OrchestrationPolicy::BatteryAware => (0.40, 0.30, 0.50),
            OrchestrationPolicy::PerformanceFirst => (0.90, 0.80, 0.80),
            OrchestrationPolicy::Adaptive => (0.70, 0.50, 0.60),
        };

        // Check available resources before scaling.
        let available = self.resource_allocator.available_compute_units();
        let requested = optimized.resource_requirements.compute_units;
        let scaled = (requested as f64 * compute_factor) as u32;
        // Don't request more than what's available.
        optimized.resource_requirements.compute_units = scaled.min(available);
        optimized.resource_requirements.power_budget *= power_factor;
        optimized.resource_requirements.thermal_budget *= thermal_factor;

        optimized
    }

    /// Get a mutable reference to the workload analyzer for recording samples.
    pub fn workload_analyzer_mut(&mut self) -> &mut WorkloadAnalyzer {
        &mut self.workload_analyzer
    }

    /// Get a mutable reference to the resource allocator.
    pub fn resource_allocator_mut(&mut self) -> &mut ResourceAllocator {
        &mut self.resource_allocator
    }
}

impl PowerManager {
    /// Create new power manager
    pub fn new() -> Self {
        Self {
            power_policy: PowerPolicy::Balanced,
            battery_monitor: BatteryMonitor::new(),
            thermal_monitor: ThermalMonitor::new(),
            power_optimizer: PowerOptimizer::new(),
            orchestration_state: AmbientOrchestrationState::Idle,
            active_model_count: 0,
        }
    }

    /// Set the current orchestration state so power estimates track the
    /// real state machine in `orchestrator.rs` (`ModelLifecycle`).
    pub fn set_orchestration_state(&mut self, state: AmbientOrchestrationState) {
        self.orchestration_state = state;
    }

    /// Set the number of models currently active on the managed device.
    pub fn set_active_model_count(&mut self, count: usize) {
        self.active_model_count = count;
    }

    /// Check if device can execute task
    pub fn can_execute(&self, _device: &AmbientDevice) -> bool {
        let battery_level = self.battery_monitor.current_level;
        let thermal_state = &self.thermal_monitor.thermal_state;

        battery_level > 20.0 && *thermal_state != ThermalState::Critical
    }

    /// Update power consumption after executing a task on a device.
    ///
    /// Uses the power optimizer to record the power state transition and
    /// the power policy to determine the power budget.
    pub fn update_power_consumption(
        &mut self,
        device: &mut AmbientDevice,
        execution_time: Duration,
    ) {
        let power_consumed = device.power_profile.active_power * execution_time.as_secs_f64();

        // Drain battery based on power consumed.
        self.battery_monitor.drain(power_consumed, execution_time);

        // Apply thermal impact from the power draw.
        self.thermal_monitor.apply_heat(power_consumed, execution_time);

        // Record the optimization state transition.
        let input_state = PowerState {
            power_consumption: power_consumed,
            performance: device.performance_profile.sustainable_performance,
            efficiency: if power_consumed > 0.0 {
                device.performance_profile.sustainable_performance / power_consumed
            } else {
                0.0
            },
            thermal_state: self.thermal_monitor.state(),
            battery_level: self.battery_monitor.level(),
        };
        self.power_optimizer.optimize(&input_state);
    }

    /// Get the current power policy.
    pub fn power_policy(&self) -> &PowerPolicy {
        &self.power_policy
    }

    /// Set the power policy.
    pub fn set_power_policy(&mut self, policy: PowerPolicy) {
        self.power_policy = policy;
    }

    /// Get a mutable reference to the power optimizer for direct optimization.
    pub fn power_optimizer_mut(&mut self) -> &mut PowerOptimizer {
        &mut self.power_optimizer
    }

    /// Get a mutable reference to the battery monitor for updates.
    pub fn battery_monitor_mut(&mut self) -> &mut BatteryMonitor {
        &mut self.battery_monitor
    }

    /// Get a mutable reference to the thermal monitor for updates.
    pub fn thermal_monitor_mut(&mut self) -> &mut ThermalMonitor {
        &mut self.thermal_monitor
    }

    /// Get battery level
    pub fn get_battery_level(&self, _device_id: &str) -> f64 {
        self.battery_monitor.current_level
    }

    /// Get thermal state derived from the current estimated power draw.
    ///
    /// Power-to-thermal mapping (mobile SoC heuristic):
    /// - `< 3W`  → `Normal` (cool)
    /// - `3–7W`  → `Warm`
    /// - `> 7W`  → `Critical`
    pub fn get_thermal_state(&self, device_id: &str) -> ThermalState {
        let power = self.get_power_consumption(device_id);
        if power > 7.0 {
            ThermalState::Critical
        } else if power >= 3.0 {
            ThermalState::Warm
        } else {
            ThermalState::Normal
        }
    }

    /// Get power consumption in watts.
    ///
    /// On platforms exposing a power API (e.g. RAPL on Intel, the Energy
    /// Meter on Android) this would query the hardware. On every other target
    /// we estimate consumption from the current orchestration state and the
    /// number of active models, which is what battery-aware ML scheduling and
    /// thermal management rely on:
    ///
    /// | State            | Base power |
    /// |------------------|------------|
    /// | Idle             | ~0.5 W     |
    /// | Active inference | ~5.0 W + (active_models × 2.0 W) |
    /// | Scrubbing        | ~3.0 W     |
    /// | Streaming        | ~4.0 W     |
    pub fn get_power_consumption(&self, device_id: &str) -> f64 {
        // NOTE: a real implementation would probe `/sys/class/powercap/` (RAPL),
        // `android.os.PowerManager` via JNI, or the CoreML energy log. Until a
        // platform power API is wired in, estimate from the orchestration state.
        let _ = device_id; // hardware query would be keyed on this id
        match self.orchestration_state {
            AmbientOrchestrationState::Idle => 0.5,
            AmbientOrchestrationState::ActiveInference => {
                5.0 + (self.active_model_count as f64) * 2.0
            }
            AmbientOrchestrationState::Scrubbing => 3.0,
            AmbientOrchestrationState::Streaming => 4.0,
        }
    }

    /// Estimate battery life remaining in hours.
    ///
    /// `hours = (battery_capacity_wh * current_battery_pct / 100.0) / power_consumption`
    ///
    /// Returns `0.0` if the estimated power consumption is zero (avoids
    /// division by zero) or if the battery percentage is non-positive.
    pub fn estimate_battery_life_remaining(
        &self,
        current_battery_pct: f64,
        battery_capacity_wh: f64,
    ) -> f64 {
        if current_battery_pct <= 0.0 || battery_capacity_wh <= 0.0 {
            return 0.0;
        }
        let power = self.get_power_consumption("");
        if power <= 0.0 {
            return 0.0;
        }
        (battery_capacity_wh * current_battery_pct / 100.0) / power
    }

    /// Decide whether inference should be throttled.
    ///
    /// Returns `true` when the thermal state is `Critical` or when the
    /// estimated battery life (using the battery monitor's current charge
    /// against a 15 Wh mobile battery as a reasonable default) drops below
    /// 1 hour.
    pub fn should_throttle_inference(&self) -> bool {
        let thermal = self.get_thermal_state("");
        if thermal == ThermalState::Critical {
            return true;
        }
        // Reasonable mobile default: 15 Wh battery. Use the battery monitor's
        // current charge level so real battery drain drives the decision.
        let battery_pct = self.battery_monitor.current_level;
        if battery_pct <= 0.0 {
            return true; // No battery left — must throttle.
        }
        let estimated_hours = self.estimate_battery_life_remaining(battery_pct, 15.0);
        estimated_hours < 1.0
    }

    /// Aggregate the current power/thermal/battery snapshot.
    ///
    /// `estimated_battery_hours` is `Some` when a non-zero battery capacity is
    /// known; here we use the battery monitor's current level against a 15 Wh
    /// mobile battery default. Returns `None` when the device has no battery
    /// (e.g. mains-powered embedded host).
    pub fn get_power_metrics(&self) -> PowerMetrics {
        let current_power_w = self.get_power_consumption("");
        let thermal_state = self.get_thermal_state("");

        // The battery monitor tracks a 0–100 percentage. Use a 15 Wh mobile
        // battery as the default capacity when one is present.
        let battery_pct = self.battery_monitor.current_level;
        let estimated_battery_hours = if battery_pct > 0.0 {
            let hours = self.estimate_battery_life_remaining(battery_pct, 15.0);
            if hours > 0.0 { Some(hours) } else { None }
        } else {
            None
        };

        PowerMetrics {
            current_power_w,
            thermal_state,
            estimated_battery_hours,
            active_model_count: self.active_model_count,
        }
    }
}

impl TaskScheduler {
    /// Create new task scheduler
    pub fn new() -> Self {
        Self {
            scheduling_policy: SchedulingPolicy::Adaptive,
            task_queue: TaskQueue::new(),
            execution_history: Vec::new(),
        }
    }

    /// Submit task
    pub fn submit_task(&mut self, task: Task) -> Result<(), AmbientError> {
        self.task_queue.pending_tasks.push(task);
        // Sort pending tasks according to the scheduling policy.
        self.sort_pending();
        Ok(())
    }

    /// Sort pending tasks according to the current scheduling policy.
    fn sort_pending(&mut self) {
        match self.scheduling_policy {
            SchedulingPolicy::Fifo => {
                // FIFO: keep insertion order (no sort needed).
            }
            SchedulingPolicy::Priority => {
                // Priority: highest priority first.
                self.task_queue.pending_tasks.sort_by(|a, b| {
                    b.priority.cmp(&a.priority)
                });
            }
            SchedulingPolicy::ShortestJobFirst => {
                // SJF: shortest estimated duration first.
                self.task_queue.pending_tasks.sort_by(|a, b| {
                    a.estimated_duration.cmp(&b.estimated_duration)
                });
            }
            SchedulingPolicy::Deadline => {
                // Deadline: earliest deadline first (tasks without deadlines go last).
                self.task_queue.pending_tasks.sort_by(|a, b| {
                    match (a.deadline, b.deadline) {
                        (Some(da), Some(db)) => da.cmp(&db),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                });
            }
            SchedulingPolicy::Adaptive => {
                // Adaptive: priority first, then shortest job as tiebreaker.
                self.task_queue.pending_tasks.sort_by(|a, b| {
                    b.priority.cmp(&a.priority)
                        .then_with(|| a.estimated_duration.cmp(&b.estimated_duration))
                });
            }
        }
    }

    /// Get pending tasks
    pub fn get_pending_tasks(&self) -> Vec<Task> {
        self.task_queue.pending_tasks.clone()
    }

    pub fn get_pending_tasks_into(&self, out: &mut [TaskHandle]) -> Result<usize, AmbientError> {
        if out.len() < self.task_queue.pending_tasks.len() {
            return Err(AmbientError::InsufficientResources(
                "task output buffer full".to_string(),
            ));
        }

        for (index, task) in self.task_queue.pending_tasks.iter().enumerate() {
            out[index] = TaskHandle {
                task_id_hash: crate::q_hash(&task.task_id),
                task_type: task.task_type.clone(),
                priority: task.priority.clone(),
                compute_units: task.resource_requirements.compute_units,
                memory: task.resource_requirements.memory,
            };
        }

        Ok(self.task_queue.pending_tasks.len())
    }

    /// Dispatch the next pending task to a device, moving it to running.
    /// Returns the dispatched task, or `None` if no tasks are pending.
    pub fn dispatch_next(&mut self) -> Option<Task> {
        let task = self.task_queue.pending_tasks.pop()?;
        self.task_queue.running_tasks.push(task.clone());
        Some(task)
    }

    /// Mark a running task as completed, recording it in execution history.
    pub fn complete_task(&mut self, task_id: &str, device_id: &str, success: bool, usage: ResourceUsage) {
        // Remove from running tasks.
        if let Some(pos) = self.task_queue.running_tasks.iter().position(|t| t.task_id == task_id) {
            let task = self.task_queue.running_tasks.remove(pos);
            self.task_queue.completed_tasks.push(task.clone());

            // Record in execution history.
            self.execution_history.push(TaskExecutionRecord {
                task_id: task.task_id.clone(),
                device_id: device_id.to_string(),
                start_time: Instant::now() - task.estimated_duration,
                end_time: Instant::now(),
                actual_duration: task.estimated_duration,
                success,
                resource_usage: usage,
            });

            // Trim history.
            if self.execution_history.len() > 500 {
                let drop = self.execution_history.len() - 500;
                self.execution_history.drain(0..drop);
            }
            // Trim completed tasks.
            if self.task_queue.completed_tasks.len() > 200 {
                let drop = self.task_queue.completed_tasks.len() - 200;
                self.task_queue.completed_tasks.drain(0..drop);
            }
        }
    }

    /// Get the number of currently running tasks.
    pub fn running_count(&self) -> usize {
        self.task_queue.running_tasks.len()
    }

    /// Get the number of completed tasks.
    pub fn completed_count(&self) -> usize {
        self.task_queue.completed_tasks.len()
    }

    /// Get recent execution history records.
    pub fn recent_history(&self, n: usize) -> &[TaskExecutionRecord] {
        let start = self.execution_history.len().saturating_sub(n);
        &self.execution_history[start..]
    }

    /// Set the scheduling policy.
    pub fn set_policy(&mut self, policy: SchedulingPolicy) {
        self.scheduling_policy = policy;
        self.sort_pending();
    }
}

impl AmbientPerformanceMonitor {
    /// Create new performance monitor
    pub fn new() -> Self {
        Self {
            device_metrics: HashMap::new(),
            task_metrics: HashMap::new(),
            global_metrics: AmbientGlobalMetrics {
                total_tasks_processed: 0,
                average_execution_time: Duration::from_millis(100),
                overall_efficiency: 0.85,
                power_savings: 0.30,
                thermal_compliance: 0.95,
                device_utilization: 0.75,
            },
        }
    }

    /// Update device metrics
    pub fn update_device_metrics(
        &mut self,
        device_id: &str,
        execution_time: Duration,
        data_size: usize,
    ) {
        let metrics = self
            .device_metrics
            .entry(device_id.to_string())
            .or_insert(DeviceMetrics {
                device_id: device_id.to_string(),
                utilization: 0.0,
                throughput: 0.0,
                latency: execution_time.as_millis() as f64,
                power_efficiency: 0.85,
                thermal_efficiency: 0.90,
            });

        metrics.latency = execution_time.as_millis() as f64;
        metrics.throughput = data_size as f64 / execution_time.as_secs_f64();
    }

    /// Record task execution metrics after a task completes.
    pub fn record_task_metrics(
        &mut self,
        task_id: &str,
        execution_time: Duration,
        success: bool,
        retry_count: u32,
    ) {
        let metrics = self
            .task_metrics
            .entry(task_id.to_string())
            .or_insert(TaskMetrics {
                task_id: task_id.to_string(),
                execution_time: Duration::ZERO,
                resource_efficiency: 0.0,
                success_rate: 0.0,
                retry_count: 0,
            });

        // Update running average of execution time.
        let prev_time = metrics.execution_time.as_secs_f64();
        let new_time = execution_time.as_secs_f64();
        metrics.execution_time = Duration::from_secs_f64((prev_time + new_time) / 2.0);

        // Update success rate (running average).
        let success_val = if success { 1.0 } else { 0.0 };
        metrics.success_rate = (metrics.success_rate + success_val) / 2.0;
        metrics.retry_count = retry_count;

        // Resource efficiency = 1 / (execution_time_ms * (1 + retries)).
        let time_ms = execution_time.as_millis().max(1) as f64;
        metrics.resource_efficiency = 1000.0 / (time_ms * (1.0 + retry_count as f64));
    }

    /// Get metrics for a specific task.
    pub fn get_task_metrics(&self, task_id: &str) -> Option<&TaskMetrics> {
        self.task_metrics.get(task_id)
    }

    /// Get global statistics, aggregating from device and task metrics.
    pub fn get_global_stats(&self) -> AmbientGlobalMetrics {
        let total_tasks = self.task_metrics.len() as u64;
        let avg_time = if !self.task_metrics.is_empty() {
            let sum: f64 = self.task_metrics.values()
                .map(|m| m.execution_time.as_secs_f64())
                .sum();
            Duration::from_secs_f64(sum / self.task_metrics.len() as f64)
        } else {
            self.global_metrics.average_execution_time
        };

        let avg_efficiency = if !self.task_metrics.is_empty() {
            self.task_metrics.values()
                .map(|m| m.resource_efficiency)
                .sum::<f64>() / self.task_metrics.len() as f64
        } else {
            self.global_metrics.overall_efficiency
        };

        let avg_success_rate = if !self.task_metrics.is_empty() {
            self.task_metrics.values()
                .map(|m| m.success_rate)
                .sum::<f64>() / self.task_metrics.len() as f64
        } else {
            1.0
        };

        AmbientGlobalMetrics {
            total_tasks_processed: total_tasks,
            average_execution_time: avg_time,
            overall_efficiency: avg_efficiency,
            power_savings: self.global_metrics.power_savings,
            thermal_compliance: avg_success_rate * 0.95,
            device_utilization: self.global_metrics.device_utilization,
        }
    }
}

// Supporting implementations

impl BatteryMonitor {
    pub fn new() -> Self {
        Self {
            current_level: 100.0,
            voltage: 3.7,
            temperature: 25.0,
            health: 100.0,
            charging: false,
            estimated_time_remaining: Duration::from_secs(3600 * 10), // 10 hours
        }
    }

    /// Current battery level as a percentage (0–100).
    pub fn level(&self) -> f64 {
        self.current_level
    }

    /// Battery voltage in volts.
    pub fn voltage(&self) -> f64 {
        self.voltage
    }

    /// Battery temperature in degrees Celsius.
    pub fn temperature(&self) -> f64 {
        self.temperature
    }

    /// Battery health as a percentage (0–100, where 100 = new).
    pub fn health(&self) -> f64 {
        self.health
    }

    /// Whether the battery is currently charging.
    pub fn is_charging(&self) -> bool {
        self.charging
    }

    /// Estimated time remaining until the battery is depleted.
    pub fn time_remaining(&self) -> Duration {
        self.estimated_time_remaining
    }

    /// Update the battery state from platform telemetry.
    pub fn update(&mut self, level: f64, voltage: f64, temperature: f64, charging: bool) {
        self.current_level = level.clamp(0.0, 100.0);
        self.voltage = voltage;
        self.temperature = temperature;
        self.charging = charging;
        // Estimate time remaining based on current drain rate.
        // A simple linear model: if not charging, estimate from level and
        // a nominal drain of 10%/hour for active use.
        if !charging && level > 0.0 {
            let hours = level / 10.0;
            self.estimated_time_remaining = Duration::from_secs((hours * 3600.0) as u64);
        } else if charging {
            // While charging, estimate time to full at ~20%/hour charge rate.
            let hours_to_full = (100.0 - level) / 20.0;
            self.estimated_time_remaining = Duration::from_secs((hours_to_full * 3600.0) as u64);
        }
    }

    /// Apply battery drain from a computation that consumed `power_w` watts
    /// for `duration`. Uses a nominal 15 Wh battery capacity.
    pub fn drain(&mut self, power_w: f64, duration: Duration) {
        if self.charging {
            return; // No drain while charging.
        }
        let wh_consumed = power_w * duration.as_secs_f64() / 3600.0;
        let battery_capacity_wh = 15.0;
        let pct_drained = (wh_consumed / battery_capacity_wh) * 100.0;
        self.current_level = (self.current_level - pct_drained).max(0.0);
        // Update temperature estimate from power draw.
        self.temperature += power_w * 0.5 * duration.as_secs_f64();
    }
}

impl ThermalMonitor {
    pub fn new() -> Self {
        Self {
            cpu_temperature: 45.0,
            gpu_temperature: 40.0,
            battery_temperature: 30.0,
            ambient_temperature: 25.0,
            thermal_state: ThermalState::Normal,
        }
    }

    /// CPU temperature in degrees Celsius.
    pub fn cpu_temp(&self) -> f64 {
        self.cpu_temperature
    }

    /// GPU temperature in degrees Celsius.
    pub fn gpu_temp(&self) -> f64 {
        self.gpu_temperature
    }

    /// Battery temperature in degrees Celsius.
    pub fn battery_temp(&self) -> f64 {
        self.battery_temperature
    }

    /// Ambient (environmental) temperature in degrees Celsius.
    pub fn ambient_temp(&self) -> f64 {
        self.ambient_temperature
    }

    /// Current thermal state classification.
    pub fn state(&self) -> ThermalState {
        self.thermal_state
    }

    /// Update thermal readings from platform sensors and reclassify the
    /// thermal state.
    ///
    /// State thresholds (mobile SoC heuristic):
    /// - CPU < 50°C → `Normal`
    /// - CPU 50–70°C → `Warm`
    /// - CPU 70–85°C → `Hot`
    /// - CPU > 85°C → `Critical`
    pub fn update(&mut self, cpu: f64, gpu: f64, battery: f64, ambient: f64) {
        self.cpu_temperature = cpu;
        self.gpu_temperature = gpu;
        self.battery_temperature = battery;
        self.ambient_temperature = ambient;
        self.thermal_state = if cpu > 85.0 {
            ThermalState::Critical
        } else if cpu > 70.0 {
            ThermalState::Hot
        } else if cpu > 50.0 {
            ThermalState::Warm
        } else {
            ThermalState::Normal
        };
    }

    /// Apply thermal impact from a computation that drew `power_w` watts
    /// for `duration`. Increases CPU/GPU temperatures proportionally.
    pub fn apply_heat(&mut self, power_w: f64, duration: Duration) {
        let secs = duration.as_secs_f64();
        // Each watt for 1 second raises CPU temp by ~0.1°C (simplified model).
        self.cpu_temperature += power_w * 0.1 * secs;
        self.gpu_temperature += power_w * 0.08 * secs;
        // Reclassify state after heating.
        self.thermal_state = if self.cpu_temperature > 85.0 {
            ThermalState::Critical
        } else if self.cpu_temperature > 70.0 {
            ThermalState::Hot
        } else if self.cpu_temperature > 50.0 {
            ThermalState::Warm
        } else {
            ThermalState::Normal
        };
    }

    /// Cool down toward ambient temperature. Called during idle periods.
    pub fn cool(&mut self, duration: Duration) {
        let secs = duration.as_secs_f64();
        // Cool at ~0.5°C/s toward ambient.
        let cooling = 0.5 * secs;
        self.cpu_temperature = self.cpu_temperature
            .max(self.ambient_temperature + cooling)
            - cooling;
        self.gpu_temperature = self.gpu_temperature
            .max(self.ambient_temperature + cooling)
            - cooling;
        // Reclassify state after cooling.
        self.thermal_state = if self.cpu_temperature > 85.0 {
            ThermalState::Critical
        } else if self.cpu_temperature > 70.0 {
            ThermalState::Hot
        } else if self.cpu_temperature > 50.0 {
            ThermalState::Warm
        } else {
            ThermalState::Normal
        };
    }
}

impl PowerOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_algorithm: OptimizationAlgorithm::Greedy,
            optimization_history: Vec::new(),
            target_efficiency: 0.85,
        }
    }

    /// Optimize the power state to approach the target efficiency.
    ///
    /// Returns the optimized power state. The optimization algorithm
    /// determines the strategy:
    /// - `Greedy`: picks the lowest-power state that meets the target efficiency.
    /// - `Genetic`/`SimulatedAnnealing`/`ReinforcementLearning`: uses the same
    ///   greedy heuristic but records the decision for future learning.
    pub fn optimize(&mut self, input: &PowerState) -> PowerState {
        let mut output = input.clone();

        // Greedy: if efficiency is below target, reduce power consumption.
        if output.efficiency < self.target_efficiency {
            // Reduce power by 20% and see if efficiency improves.
            output.power_consumption *= 0.8;
            // Recalculate efficiency as performance per watt.
            if output.power_consumption > 0.0 {
                output.efficiency = output.performance / output.power_consumption;
            }
            // Adjust thermal state based on new power level.
            output.thermal_state = if output.power_consumption > 7.0 {
                ThermalState::Critical
            } else if output.power_consumption >= 3.0 {
                ThermalState::Warm
            } else {
                ThermalState::Normal
            };
        }

        // Record the optimization.
        let gain = if input.power_consumption > 0.0 {
            (input.power_consumption - output.power_consumption) / input.power_consumption
        } else {
            0.0
        };
        self.optimization_history.push(OptimizationRecord {
            timestamp: Instant::now(),
            algorithm: self.optimization_algorithm.clone(),
            input_state: input.clone(),
            output_state: output.clone(),
            efficiency_gain: gain,
        });

        // Trim history.
        if self.optimization_history.len() > 200 {
            let drop = self.optimization_history.len() - 200;
            self.optimization_history.drain(0..drop);
        }

        output
    }

    /// Get the average efficiency gain from recent optimizations.
    pub fn average_efficiency_gain(&self) -> f64 {
        if self.optimization_history.is_empty() {
            return 0.0;
        }
        self.optimization_history.iter()
            .map(|r| r.efficiency_gain)
            .sum::<f64>() / self.optimization_history.len() as f64
    }

    /// Get the target efficiency this optimizer is configured for.
    pub fn target_efficiency(&self) -> f64 {
        self.target_efficiency
    }
}

impl WorkloadAnalyzer {
    pub fn new() -> Self {
        Self {
            workload_history: Vec::new(),
            prediction_model: PredictionModel::new(),
            analysis_window: Duration::from_secs(300), // 5 minutes
        }
    }

    /// Record a workload sample for ongoing analysis.
    pub fn record_sample(&mut self, sample: WorkloadSample) {
        self.workload_history.push(sample);
        // Trim history to the analysis window (keep at most 1000 samples).
        let now = Instant::now();
        let window = self.analysis_window;
        self.workload_history.retain(|s| now.duration_since(s.timestamp) <= window);
        if self.workload_history.len() > 1000 {
            let drop = self.workload_history.len() - 1000;
            self.workload_history.drain(0..drop);
        }
    }

    /// Analyze the recorded workload history to produce real metrics.
    ///
    /// When no samples have been recorded, returns a neutral baseline
    /// (all pressures at 0.0, load at 0.0) rather than fabricated values.
    pub fn analyze_workload(&self) -> WorkloadAnalysis {
        if self.workload_history.is_empty() {
            return WorkloadAnalysis {
                current_load: 0.0,
                predicted_load: 0.0,
                resource_pressure: 0.0,
                thermal_pressure: 0.0,
                battery_pressure: 0.0,
            };
        }

        // Current load = average of the most recent samples' CPU + neural engine usage.
        let recent: Vec<&WorkloadSample> = self.workload_history.iter().rev().take(10).collect();
        let current_load = recent.iter()
            .map(|s| (s.cpu_usage + s.neural_engine_usage) / 2.0)
            .sum::<f64>() / recent.len() as f64;

        // Predicted load using the linear regression model:
        // predicted = w0*cpu + w1*memory + w2*neural + bias
        let pm = &self.prediction_model;
        let last = recent[0];
        let predicted_load = if pm.parameters.weights.len() >= 3 {
            let w = &pm.parameters.weights;
            let b = pm.parameters.biases.first().copied().unwrap_or(0.0);
            (w[0] * last.cpu_usage + w[1] * last.memory_usage + w[2] * last.neural_engine_usage + b)
                .clamp(0.0, 1.0)
        } else {
            current_load
        };

        // Resource pressure from memory usage.
        let resource_pressure = recent.iter()
            .map(|s| s.memory_usage)
            .sum::<f64>() / recent.len() as f64;

        // Thermal pressure from thermal state (0-1 scale, 0=cool, 1=hot).
        let thermal_pressure = recent.iter()
            .map(|s| s.thermal_state)
            .sum::<f64>() / recent.len() as f64;

        // Battery pressure: 1.0 - battery_level/100 (higher = more pressure).
        let battery_pressure = recent.iter()
            .map(|s| 1.0 - (s.battery_level / 100.0).clamp(0.0, 1.0))
            .sum::<f64>() / recent.len() as f64;

        WorkloadAnalysis {
            current_load: current_load.clamp(0.0, 1.0),
            predicted_load,
            resource_pressure: resource_pressure.clamp(0.0, 1.0),
            thermal_pressure: thermal_pressure.clamp(0.0, 1.0),
            battery_pressure: battery_pressure.clamp(0.0, 1.0),
        }
    }
}

impl ResourceAllocator {
    pub fn new() -> Self {
        Self {
            allocation_strategy: AllocationStrategy::PowerAware,
            resource_pool: ResourcePool::new(),
            allocation_history: Vec::new(),
        }
    }

    /// Try to allocate compute units for a task, recording the allocation.
    ///
    /// Returns the number of compute units actually granted (may be less than
    /// requested if the pool is exhausted). Returns 0 if no units are available.
    pub fn allocate(&mut self, device_id: &str, units: u32, duration: Duration) -> u32 {
        let granted = match self.allocation_strategy {
            AllocationStrategy::RoundRobin => {
                // Round-robin: grant up to requested, cycling through pool.
                units.min(self.resource_pool.available_compute_units)
            }
            AllocationStrategy::PerformanceBased => {
                // Performance: grant as many as possible for max throughput.
                units.min(self.resource_pool.available_compute_units)
            }
            AllocationStrategy::PowerAware => {
                // Power-aware: grant at most 70% of requested to save power.
                let reduced = (units as f64 * 0.7).ceil() as u32;
                reduced.min(self.resource_pool.available_compute_units)
            }
            AllocationStrategy::ThermalAware => {
                // Thermal-aware: grant at most 60% of requested.
                let reduced = (units as f64 * 0.6).ceil() as u32;
                reduced.min(self.resource_pool.available_compute_units)
            }
            AllocationStrategy::MultiObjective => {
                // Multi-objective: grant at most 80% of requested.
                let reduced = (units as f64 * 0.8).ceil() as u32;
                reduced.min(self.resource_pool.available_compute_units)
            }
        };

        if granted > 0 {
            self.resource_pool.available_compute_units -= granted;
            let efficiency = if units > 0 {
                granted as f64 / units as f64
            } else {
                1.0
            };
            self.allocation_history.push(AllocationRecord {
                timestamp: Instant::now(),
                device_id: device_id.to_string(),
                resource_type: ResourceType::ComputeUnit,
                amount: granted,
                duration,
                efficiency,
            });
        }
        granted
    }

    /// Release previously-allocated compute units back to the pool.
    pub fn release(&mut self, units: u32) {
        self.resource_pool.available_compute_units =
            (self.resource_pool.available_compute_units + units)
                .min(self.resource_pool.total_compute_units);
    }

    /// Get the current available compute units in the pool.
    pub fn available_compute_units(&self) -> u32 {
        self.resource_pool.available_compute_units
    }

    /// Get a summary of allocation history (most recent N records).
    pub fn recent_allocations(&self, n: usize) -> &[AllocationRecord] {
        let start = self.allocation_history.len().saturating_sub(n);
        &self.allocation_history[start..]
    }
}

impl AdaptationEngine {
    pub fn new() -> Self {
        Self {
            adaptation_strategy: AdaptationStrategy::MachineLearning,
            adaptation_history: Vec::new(),
            learning_rate: 0.01,
        }
    }

    /// Adapt the orchestration policy based on workload analysis.
    ///
    /// The adaptation strategy determines how decisions are made:
    /// - `Static`: always returns `Adaptive` (no adaptation).
    /// - `RuleBased`: threshold-based policy selection.
    /// - `MachineLearning`: rule-based with learning-rate-weighted adjustments
    ///   based on past adaptation outcomes.
    /// - `Hybrid`: combines rule-based with ML adjustments.
    pub fn adapt_policy(&mut self, analysis: WorkloadAnalysis) -> OrchestrationPolicy {
        let policy = match self.adaptation_strategy {
            AdaptationStrategy::Static => OrchestrationPolicy::Adaptive,
            AdaptationStrategy::RuleBased
            | AdaptationStrategy::MachineLearning
            | AdaptationStrategy::Hybrid => {
                if analysis.battery_pressure > 0.7 {
                    OrchestrationPolicy::PowerEfficiency
                } else if analysis.thermal_pressure > 0.6 {
                    OrchestrationPolicy::ThermalAware
                } else if analysis.current_load > 0.8 {
                    OrchestrationPolicy::PerformanceFirst
                } else {
                    OrchestrationPolicy::Adaptive
                }
            }
        };

        // For ML/Hybrid strategies, adjust thresholds based on past success rate.
        // If recent adaptations mostly succeeded, we keep the policy; if they
        // failed, we bias toward more conservative (PowerEfficiency) choices.
        if matches!(self.adaptation_strategy, AdaptationStrategy::MachineLearning | AdaptationStrategy::Hybrid) {
            let recent: Vec<&AdaptationRecord> = self.adaptation_history.iter().rev().take(20).collect();
            if !recent.is_empty() {
                let success_rate = recent.iter()
                    .filter(|r| r.result == AdaptationResult::Success)
                    .count() as f64 / recent.len() as f64;
                // Learning rate influences how much we trust past outcomes.
                // Low success rate + high learning rate → more conservative.
                if success_rate < 0.5 && self.learning_rate > 0.0 {
                    // Bias toward power efficiency when past adaptations failed.
                    return OrchestrationPolicy::PowerEfficiency;
                }
            }
        }

        // Record this adaptation decision.
        let trigger = if analysis.battery_pressure > 0.7 {
            AdaptationTrigger::BatteryThreshold
        } else if analysis.thermal_pressure > 0.6 {
            AdaptationTrigger::ThermalThreshold
        } else if analysis.current_load > 0.8 {
            AdaptationTrigger::PerformanceThreshold
        } else {
            AdaptationTrigger::WorkloadChange
        };

        self.adaptation_history.push(AdaptationRecord {
            timestamp: Instant::now(),
            trigger,
            action: match policy {
                OrchestrationPolicy::PerformanceFirst => AdaptationAction::ScaleUp,
                OrchestrationPolicy::PowerEfficiency => AdaptationAction::ScaleDown,
                OrchestrationPolicy::ThermalAware => AdaptationAction::ScaleDown,
                OrchestrationPolicy::BatteryAware => AdaptationAction::Suspend,
                OrchestrationPolicy::Adaptive => AdaptationAction::Resume,
            },
            result: AdaptationResult::Success,
        });

        // Trim history to prevent unbounded growth.
        if self.adaptation_history.len() > 500 {
            let drop = self.adaptation_history.len() - 500;
            self.adaptation_history.drain(0..drop);
        }

        policy
    }

    /// Get the recent adaptation history.
    pub fn recent_adaptations(&self, n: usize) -> &[AdaptationRecord] {
        let start = self.adaptation_history.len().saturating_sub(n);
        &self.adaptation_history[start..]
    }
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            pending_tasks: Vec::new(),
            running_tasks: Vec::new(),
            completed_tasks: Vec::new(),
        }
    }
}

impl PredictionModel {
    pub fn new() -> Self {
        Self {
            model_type: ModelType::LinearRegression,
            parameters: ModelParameters {
                weights: vec![0.5, 0.3, 0.2],
                biases: vec![0.1],
                learning_rate: 0.01,
            },
            accuracy: 0.85,
        }
    }
}

impl ResourcePool {
    pub fn new() -> Self {
        Self {
            total_compute_units: 32,
            available_compute_units: 32,
            total_memory: 16 * 1024 * 1024 * 1024, // 16GB
            available_memory: 16 * 1024 * 1024 * 1024,
            total_neural_engines: 4,
            available_neural_engines: 4,
        }
    }
}

// Supporting types

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ambient_orchestration_creation() {
        let manager = AmbientOrchestrationManager::new();
        assert_eq!(manager.list_devices().len(), 0);
    }

    #[test]
    fn test_device_discovery() {
        let mut manager = AmbientOrchestrationManager::new();

        let devices = manager.discover_devices().unwrap();
        assert!(!devices.is_empty());
        assert!(devices.len() <= 9);
        assert_eq!(devices.len(), manager.list_devices().len());
        assert!(devices.iter().any(|id| id == "local_host"));

        let host = manager.devices.get("local_host").unwrap();
        assert!(host.capabilities.compute_units >= 1);

        let cpu_core_count = devices
            .iter()
            .filter(|id| id.starts_with("cpu_core_"))
            .count();
        assert_eq!(devices.len(), cpu_core_count + 1);

        for device_id in &devices {
            let device_status = manager.get_device_status(device_id);
            assert!(device_status.is_some());
        }
    }

    #[test]
    fn test_task_submission() {
        let mut manager = AmbientOrchestrationManager::new();

        let task = Task {
            task_id: "test_task".to_string(),
            task_type: TaskType::NeuralInference,
            priority: TaskPriority::Normal,
            resource_requirements: ResourceRequirements {
                compute_units: 2,
                memory: 1024 * 1024,
                neural_engines: 1,
                power_budget: 2.0,
                thermal_budget: 1.0,
            },
            deadline: None,
            estimated_duration: Duration::from_millis(100),
            dependencies: vec![],
        };

        let task_id = manager.submit_task(task).unwrap();
        assert_eq!(task_id, "test_task");
    }

    #[test]
    fn test_neural_inference() {
        let mut manager = AmbientOrchestrationManager::new();

        let devices = manager.discover_devices().unwrap();
        let device_id = &devices[0];

        let model_data = vec![1u8; 1024];
        let input_data = vec![2u8; 512];

        let result = manager.execute_neural_inference(device_id, &model_data, &input_data);
        assert!(result.is_ok());
    }

    // ── Ambient power monitoring tests ───────────────────────────────────

    #[test]
    fn test_power_consumption_changes_with_orchestration_state() {
        let mut pm = PowerManager::new();

        // Idle baseline
        pm.set_orchestration_state(AmbientOrchestrationState::Idle);
        let idle_power = pm.get_power_consumption("local_host");
        assert!(
            (idle_power - 0.5).abs() < f64::EPSILON,
            "idle power should be ~0.5W, got {idle_power}"
        );

        // Active inference scales with active model count
        pm.set_active_model_count(2);
        pm.set_orchestration_state(AmbientOrchestrationState::ActiveInference);
        let active_power = pm.get_power_consumption("local_host");
        // 5.0 + 2 * 2.0 = 9.0
        assert!(
            (active_power - 9.0).abs() < f64::EPSILON,
            "active inference power with 2 models should be ~9.0W, got {active_power}"
        );

        // Scrubbing
        pm.set_orchestration_state(AmbientOrchestrationState::Scrubbing);
        let scrub_power = pm.get_power_consumption("local_host");
        assert!(
            (scrub_power - 3.0).abs() < f64::EPSILON,
            "scrubbing power should be ~3.0W, got {scrub_power}"
        );

        // Streaming
        pm.set_orchestration_state(AmbientOrchestrationState::Streaming);
        let stream_power = pm.get_power_consumption("local_host");
        assert!(
            (stream_power - 4.0).abs() < f64::EPSILON,
            "streaming power should be ~4.0W, got {stream_power}"
        );

        // Verify ordering: idle < scrubbing < streaming < active(2 models)
        assert!(idle_power < scrub_power);
        assert!(scrub_power < stream_power);
        assert!(stream_power < active_power);
    }

    #[test]
    fn test_power_consumption_scales_with_active_models() {
        let mut pm = PowerManager::new();
        pm.set_orchestration_state(AmbientOrchestrationState::ActiveInference);

        pm.set_active_model_count(0);
        assert!((pm.get_power_consumption("d") - 5.0).abs() < f64::EPSILON);

        pm.set_active_model_count(1);
        assert!((pm.get_power_consumption("d") - 7.0).abs() < f64::EPSILON);

        pm.set_active_model_count(3);
        assert!((pm.get_power_consumption("d") - 11.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_thermal_state_mapping_cool_warm_critical() {
        let mut pm = PowerManager::new();

        // < 3W → Normal (Cool)
        pm.set_orchestration_state(AmbientOrchestrationState::Idle);
        assert_eq!(pm.get_thermal_state("d"), ThermalState::Normal);

        // 3W (boundary) → Warm
        pm.set_orchestration_state(AmbientOrchestrationState::Scrubbing);
        assert_eq!(pm.get_thermal_state("d"), ThermalState::Warm);

        // 4W → Warm
        pm.set_orchestration_state(AmbientOrchestrationState::Streaming);
        assert_eq!(pm.get_thermal_state("d"), ThermalState::Warm);

        // > 7W → Critical
        pm.set_orchestration_state(AmbientOrchestrationState::ActiveInference);
        pm.set_active_model_count(2); // 9.0W
        assert_eq!(pm.get_thermal_state("d"), ThermalState::Critical);

        // Exactly 7W boundary → Warm (>= 3.0 and <= 7.0)
        pm.set_active_model_count(1); // 7.0W
        assert_eq!(pm.get_thermal_state("d"), ThermalState::Warm);
    }

    #[test]
    fn test_estimate_battery_life_remaining() {
        let mut pm = PowerManager::new();
        pm.set_orchestration_state(AmbientOrchestrationState::ActiveInference);
        pm.set_active_model_count(0); // 5.0W

        // 50% of a 10Wh battery = 5Wh / 5W = 1.0 hour
        let hours = pm.estimate_battery_life_remaining(50.0, 10.0);
        assert!(
            (hours - 1.0).abs() < 1e-9,
            "expected 1.0 hour, got {hours}"
        );

        // 100% of 15Wh / 5W = 3.0 hours
        let hours = pm.estimate_battery_life_remaining(100.0, 15.0);
        assert!(
            (hours - 3.0).abs() < 1e-9,
            "expected 3.0 hours, got {hours}"
        );

        // Zero power → 0.0 (avoid div-by-zero). Idle is 0.5W, so use a
        // contrived zero-battery case instead.
        assert_eq!(pm.estimate_battery_life_remaining(0.0, 10.0), 0.0);
        assert_eq!(pm.estimate_battery_life_remaining(50.0, 0.0), 0.0);
    }

    #[test]
    fn test_should_throttle_inference_thermal_critical() {
        let mut pm = PowerManager::new();
        // Force Critical thermal: > 7W
        pm.set_orchestration_state(AmbientOrchestrationState::ActiveInference);
        pm.set_active_model_count(2); // 9.0W → Critical
        assert!(
            pm.should_throttle_inference(),
            "should throttle when thermal state is Critical"
        );
    }

    #[test]
    fn test_should_throttle_inference_low_battery() {
        let mut pm = PowerManager::new();
        // Idle: 0.5W, thermal Normal. Drain battery so estimated life < 1h.
        // With 0.5W and 15Wh default, full charge = 30h. To get < 1h we need
        // battery_pct such that (15 * pct/100) / 0.5 < 1 → pct < 3.33%.
        pm.set_orchestration_state(AmbientOrchestrationState::Idle);
        pm.battery_monitor.current_level = 2.0; // ~0.6h remaining
        assert!(
            pm.should_throttle_inference(),
            "should throttle when estimated battery life < 1 hour"
        );
    }

    #[test]
    fn test_should_not_throttle_when_healthy() {
        let mut pm = PowerManager::new();
        // Idle, 0.5W, full battery → 30h remaining, Normal thermal.
        pm.set_orchestration_state(AmbientOrchestrationState::Idle);
        pm.battery_monitor.current_level = 100.0;
        assert!(
            !pm.should_throttle_inference(),
            "should not throttle when thermal is cool and battery is healthy"
        );
    }

    #[test]
    fn test_power_metrics_aggregation() {
        let mut pm = PowerManager::new();
        pm.set_orchestration_state(AmbientOrchestrationState::ActiveInference);
        pm.set_active_model_count(1); // 7.0W → Warm
        pm.battery_monitor.current_level = 50.0;

        let metrics = pm.get_power_metrics();
        assert!((metrics.current_power_w - 7.0).abs() < f64::EPSILON);
        assert_eq!(metrics.thermal_state, ThermalState::Warm);
        assert_eq!(metrics.active_model_count, 1);
        // 50% of 15Wh / 7W = 1.0714...h
        let expected = (15.0 * 50.0 / 100.0) / 7.0;
        assert!(metrics.estimated_battery_hours.is_some());
        assert!(
            (metrics.estimated_battery_hours.unwrap() - expected).abs() < 1e-9
        );
    }

    #[test]
    fn test_power_metrics_no_battery() {
        let mut pm = PowerManager::new();
        pm.battery_monitor.current_level = 0.0;
        let metrics = pm.get_power_metrics();
        assert!(metrics.estimated_battery_hours.is_none());
    }

    #[test]
    fn test_manager_power_monitoring_integration() {
        let mut manager = AmbientOrchestrationManager::new();

        // Default idle state
        let metrics = manager.get_power_metrics();
        assert!((metrics.current_power_w - 0.5).abs() < f64::EPSILON);
        assert_eq!(metrics.thermal_state, ThermalState::Normal);

        // Transition to active inference with 3 models
        manager.set_orchestration_state(AmbientOrchestrationState::ActiveInference);
        manager.set_active_model_count(3); // 11.0W → Critical

        let metrics = manager.get_power_metrics();
        assert!((metrics.current_power_w - 11.0).abs() < f64::EPSILON);
        assert_eq!(metrics.thermal_state, ThermalState::Critical);
        assert!(manager.should_throttle_inference());
    }
}
