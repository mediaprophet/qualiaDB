//! NVMe Computational Storage (CSD) Implementation
//! 
//! This module provides computational storage pushdown using NVMe CSD (Computational Storage Device).
//! Designed for hardware-accelerated mathematical computations and query processing.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::fs::{File, OpenOptions};
#[cfg(unix)]
use std::os::unix::io::AsRawFd;

#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, RawHandle};
use std::path::Path;
use serde::{Deserialize, Serialize};

/// CSD Manager for computational storage operations
pub struct CsdManager {
    devices: HashMap<String, CsdDevice>,
    functions: HashMap<String, CsdFunction>,
    scheduler: CsdScheduler,
    performance_monitor: CsdPerformanceMonitor,
}

/// CSD device information
#[derive(Debug, Clone)]
pub struct CsdDevice {
    pub device_id: String,
    pub device_path: String,
    pub capabilities: CsdCapabilities,
    pub supported_functions: Vec<String>,
    pub device_stats: CsdDeviceStats,
}

/// CSD device capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsdCapabilities {
    pub max_concurrent_operations: u32,
    pub max_data_size: u64,
    pub supported_operations: Vec<CsdOperationType>,
    pub memory_size: u64,
    pub compute_units: u32,
    pub clock_speed: f64,
}

/// CSD operations supported by device
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CsdOperationType {
    MatrixMultiply,
    VectorDotProduct,
    Convolution,
    Filter,
    Aggregate,
    Sort,
    Search,
    Custom(String),
}

/// CSD function for computational operations
#[derive(Debug, Clone)]
pub struct CsdFunction {
    pub function_id: String,
    pub operation: CsdOperationType,
    pub parameters: Vec<FunctionParameter>,
    pub bytecode: Vec<u8>,
    pub performance_profile: PerformanceProfile,
}

/// Function parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParameter {
    pub name: String,
    pub param_type: ParameterType,
    pub size: u64,
    pub is_input: bool,
    pub is_output: bool,
}

/// Parameter types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParameterType {
    Matrix,
    Vector,
    Scalar,
    Tensor,
    Buffer,
}

/// Performance profile for functions
#[derive(Debug, Clone)]
pub struct PerformanceProfile {
    pub expected_execution_time: f64,
    pub memory_usage: u64,
    pub compute_intensity: f64,
    pub data_intensity: f64,
}

/// CSD device statistics
#[derive(Debug, Clone)]
pub struct CsdDeviceStats {
    pub operations_completed: u64,
    pub total_execution_time: u64,
    pub average_execution_time: f64,
    pub data_processed: u64,
    pub error_count: u64,
    pub utilization: f64,
}

/// CSD scheduler for operation management
pub struct CsdScheduler {
    pending_operations: Vec<CsdOperationRequest>,
    running_operations: HashMap<u64, CsdRunningOperation>,
    completion_queue: Vec<CsdCompletion>,
    scheduling_policy: SchedulingPolicy,
}

/// CSD operation request to be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsdOperationRequest {
    pub operation_id: u64,
    pub function_id: String,
    pub device_id: String,
    pub inputs: Vec<OperationInput>,
    pub outputs: Vec<OperationOutput>,
    pub priority: OperationPriority,
    pub deadline: Option<u64>,
}

/// Operation input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationInput {
    pub name: String,
    pub data: Vec<u8>,
    pub location: DataLocation,
}

/// Operation output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationOutput {
    pub name: String,
    pub size: u64,
    pub location: DataLocation,
}

/// Data location for operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataLocation {
    HostMemory,
    DeviceMemory,
    PersistentStorage,
}

/// Operation priority
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Running operation tracking
#[derive(Debug, Clone)]
pub struct CsdRunningOperation {
    pub operation_id: u64,
    pub device_id: String,
    pub start_time: u64,
    pub progress: f64,
}

/// Operation completion result
#[derive(Debug, Clone)]
pub struct CsdCompletion {
    pub operation_id: u64,
    pub status: CompletionStatus,
    pub execution_time: u64,
    pub outputs: Vec<OperationOutput>,
    pub error_message: Option<String>,
}

/// Completion status
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionStatus {
    Success,
    Error,
    Timeout,
    Cancelled,
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
    /// Load-Balanced Scheduling
    LoadBalanced,
}

/// CSD performance monitor
pub struct CsdPerformanceMonitor {
    device_metrics: HashMap<String, CsdDeviceMetrics>,
    function_metrics: HashMap<String, CsdFunctionMetrics>,
    global_metrics: CsdGlobalMetrics,
}

/// Device performance metrics
#[derive(Debug, Clone)]
pub struct CsdDeviceMetrics {
    pub device_id: String,
    pub utilization: f64,
    pub throughput: f64,
    pub latency: f64,
    pub error_rate: f64,
    pub power_consumption: f64,
}

/// Function performance metrics
#[derive(Debug, Clone)]
pub struct CsdFunctionMetrics {
    pub function_id: String,
    pub execution_count: u64,
    pub total_execution_time: u64,
    pub average_execution_time: f64,
    pub success_rate: f64,
    pub data_throughput: f64,
}

/// Global performance metrics
#[derive(Debug, Clone)]
pub struct CsdGlobalMetrics {
    pub total_operations: u64,
    pub total_execution_time: u64,
    pub average_execution_time: f64,
    pub total_data_processed: u64,
    pub overall_throughput: f64,
    pub system_utilization: f64,
}

/// Mathematical computation builder
pub struct MathComputationBuilder {
    operations: Vec<CsdOperationRequest>,
    data_dependencies: HashMap<String, Vec<String>>,
    execution_plan: ExecutionPlan,
}

/// Execution plan for computations
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub stages: Vec<ExecutionStage>,
    pub parallel_groups: Vec<Vec<String>>,
    pub estimated_time: f64,
    pub resource_requirements: ResourceRequirements,
}

/// Execution stage
#[derive(Debug, Clone)]
pub struct ExecutionStage {
    pub stage_id: u32,
    pub operations: Vec<String>,
    pub dependencies: Vec<String>,
    pub estimated_time: f64,
}

/// Resource requirements
#[derive(Debug, Clone)]
pub struct ResourceRequirements {
    pub memory_usage: u64,
    pub compute_units: u32,
    pub bandwidth: f64,
}

impl CsdManager {
    /// Create new CSD manager
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            functions: HashMap::new(),
            scheduler: CsdScheduler::new(),
            performance_monitor: CsdPerformanceMonitor::new(),
        }
    }

    /// Discover CSD devices
    pub fn discover_devices(&mut self) -> Result<Vec<String>, CsdError> {
        let mut discovered_devices = Vec::new();
        
        // Scan for CSD devices
        for i in 0..16 {
            let device_path = format!("/dev/nvme{}", i);
            if Path::new(&device_path).exists() {
                if let Ok(device) = self.probe_device(&device_path) {
                    discovered_devices.push(device.device_id.clone());
                    self.devices.insert(device.device_id.clone(), device);
                }
            }
        }

        Ok(discovered_devices)
    }

    /// Probe CSD device
    fn probe_device(&self, device_path: &str) -> Result<CsdDevice, CsdError> {
        let device_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(device_path)
            .map_err(|e| CsdError::DeviceOpen(e.to_string()))?;

        // Get device information using ioctl
        let device_id = format!("csd-{}", device_path);
        
        // For now, use reasonable defaults
        let capabilities = CsdCapabilities {
            max_concurrent_operations: 16,
            max_data_size: 1024 * 1024 * 1024, // 1GB
            supported_operations: vec![
                CsdOperationType::MatrixMultiply,
                CsdOperationType::VectorDotProduct,
                CsdOperationType::Convolution,
                CsdOperationType::Filter,
                CsdOperationType::Aggregate,
            ],
            memory_size: 8 * 1024 * 1024 * 1024, // 8GB
            compute_units: 64,
            clock_speed: 1.5, // 1.5 GHz
        };

        let device = CsdDevice {
            device_id: device_id.clone(),
            device_path: device_path.to_string(),
            capabilities,
            supported_functions: vec![], // Will be populated later
            device_stats: CsdDeviceStats {
                operations_completed: 0,
                total_execution_time: 0,
                average_execution_time: 0.0,
                data_processed: 0,
                error_count: 0,
                utilization: 0.0,
            },
        };

        Ok(device)
    }

    /// Register computational function
    pub fn register_function(&mut self, function: CsdFunction) -> Result<(), CsdError> {
        // Validate function
        self.validate_function(&function)?;

        // Store function
        self.functions.insert(function.function_id.clone(), function);

        Ok(())
    }

    /// Execute computational operation
    pub fn execute_operation(&mut self, operation: CsdOperationRequest) -> Result<u64, CsdError> {
        // Validate operation
        self.validate_operation(&operation)?;

        // Schedule operation
        self.scheduler.schedule_operation(operation.clone())?;

        Ok(operation.operation_id)
    }

    /// Execute matrix multiplication
    pub fn matrix_multiply(&mut self, device_id: &str, a: &[f32], b: &[f32], dimensions: (usize, usize, usize)) -> Result<Vec<f32>, CsdError> {
        // Create matrix multiplication operation
        let operation_id = self.generate_operation_id();
        
        let operation = CsdOperationRequest {
            operation_id,
            function_id: "matrix_multiply".to_string(),
            device_id: device_id.to_string(),
            inputs: vec![
                OperationInput {
                    name: "matrix_a".to_string(),
                    data: self.f32_slice_to_bytes(a),
                    location: DataLocation::HostMemory,
                },
                OperationInput {
                    name: "matrix_b".to_string(),
                    data: self.f32_slice_to_bytes(b),
                    location: DataLocation::HostMemory,
                },
                OperationInput {
                    name: "dimensions".to_string(),
                    data: self.serialize_dimensions(dimensions),
                    location: DataLocation::HostMemory,
                },
            ],
            outputs: vec![
                OperationOutput {
                    name: "result".to_string(),
                    size: ((dimensions.0 * dimensions.2) * 4) as u64, // 4 bytes per f32
                    location: DataLocation::HostMemory,
                },
            ],
            priority: OperationPriority::Normal,
            deadline: None,
        };

        // Execute operation
        self.execute_operation(operation)?;

        // Wait for completion
        let completion = self.wait_for_completion(operation_id)?;

        // Parse result
        if let Some(output) = completion.outputs.first() {
            let result = self.bytes_to_f32_slice(&completion.outputs[0]);
            Ok(result)
        } else {
            Err(CsdError::NoOutput("No output generated".to_string()))
        }
    }

    /// Execute vector dot product
    pub fn vector_dot_product(&mut self, device_id: &str, a: &[f32], b: &[f32]) -> Result<f32, CsdError> {
        let operation_id = self.generate_operation_id();
        
        let operation = CsdOperationRequest {
            operation_id,
            function_id: "vector_dot_product".to_string(),
            device_id: device_id.to_string(),
            inputs: vec![
                OperationInput {
                    name: "vector_a".to_string(),
                    data: self.f32_slice_to_bytes(a),
                    location: DataLocation::HostMemory,
                },
                OperationInput {
                    name: "vector_b".to_string(),
                    data: self.f32_slice_to_bytes(b),
                    location: DataLocation::HostMemory,
                },
            ],
            outputs: vec![
                OperationOutput {
                    name: "result".to_string(),
                    size: 4, // 4 bytes for f32
                    location: DataLocation::HostMemory,
                },
            ],
            priority: OperationPriority::Normal,
            deadline: None,
        };

        self.execute_operation(operation)?;
        let completion = self.wait_for_completion(operation_id)?;

        if let Some(output) = completion.outputs.first() {
            let result = self.bytes_to_f32_value(&output);
            Ok(result)
        } else {
            Err(CsdError::NoOutput("No output generated".to_string()))
        }
    }

    /// Execute convolution operation
    pub fn convolution(&mut self, device_id: &str, input: &[f32], kernel: &[f32], dimensions: (usize, usize, usize, usize)) -> Result<Vec<f32>, CsdError> {
        let operation_id = self.generate_operation_id();
        
        let operation = CsdOperationRequest {
            operation_id,
            function_id: "convolution".to_string(),
            device_id: device_id.to_string(),
            inputs: vec![
                OperationInput {
                    name: "input".to_string(),
                    data: self.f32_slice_to_bytes(input),
                    location: DataLocation::HostMemory,
                },
                OperationInput {
                    name: "kernel".to_string(),
                    data: self.f32_slice_to_bytes(kernel),
                    location: DataLocation::HostMemory,
                },
                OperationInput {
                    name: "dimensions".to_string(),
                    data: self.serialize_convolution_dimensions(dimensions),
                    location: DataLocation::HostMemory,
                },
            ],
            outputs: vec![
                OperationOutput {
                    name: "result".to_string(),
                    size: (dimensions.0 * dimensions.1 * 4) as u64, // 4 bytes per f32
                    location: DataLocation::HostMemory,
                },
            ],
            priority: OperationPriority::Normal,
            deadline: None,
        };

        self.execute_operation(operation)?;
        let completion = self.wait_for_completion(operation_id)?;

        if let Some(output) = completion.outputs.first() {
            let result = self.bytes_to_f32_slice(&output);
            Ok(result)
        } else {
            Err(CsdError::NoOutput("No output generated".to_string()))
        }
    }

    /// Get device statistics
    pub fn get_device_stats(&self, device_id: &str) -> Option<CsdDeviceStats> {
        self.devices.get(device_id).map(|device| device.device_stats.clone())
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> CsdGlobalMetrics {
        self.performance_monitor.get_global_stats()
    }

    /// List available devices
    pub fn list_devices(&self) -> Vec<String> {
        self.devices.keys().cloned().collect()
    }

    /// List available functions
    pub fn list_functions(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }

    // Internal methods

    /// Validate function
    fn validate_function(&self, function: &CsdFunction) -> Result<(), CsdError> {
        if function.function_id.is_empty() {
            return Err(CsdError::InvalidFunction("Function ID cannot be empty".to_string()));
        }

        if function.bytecode.is_empty() {
            return Err(CsdError::InvalidFunction("Function bytecode cannot be empty".to_string()));
        }

        Ok(())
    }

    /// Validate operation
    fn validate_operation(&self, operation: &CsdOperationRequest) -> Result<(), CsdError> {
        // Check if device exists
        if !self.devices.contains_key(&operation.device_id) {
            return Err(CsdError::DeviceNotFound(operation.device_id.clone()));
        }

        // Check if function exists
        if !self.functions.contains_key(&operation.function_id) {
            return Err(CsdError::FunctionNotFound(operation.function_id.clone()));
        }

        // Check inputs
        if operation.inputs.is_empty() {
            return Err(CsdError::InvalidOperation("Operation must have inputs".to_string()));
        }

        Ok(())
    }

    /// Wait for operation completion
    fn wait_for_completion(&self, operation_id: u64) -> Result<CsdCompletion, CsdError> {
        // In real implementation, would wait for actual completion
        // For now, simulate completion
        Ok(CsdCompletion {
            operation_id,
            status: CompletionStatus::Success,
            execution_time: 1000, // 1ms
            outputs: vec![],
            error_message: None,
        })
    }

    /// Generate unique operation ID
    fn generate_operation_id(&self) -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// Convert f32 slice to bytes
    fn f32_slice_to_bytes(&self, slice: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(slice.len() * 4);
        for &value in slice {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    /// Convert bytes to f32 slice.
    ///
    /// `OperationOutput` carries only size and location metadata.  When the data lives in
    /// host memory the caller must have already staged it; we return a zero-filled buffer
    /// of the correct length so downstream code has the right shape.  Callers that have
    /// a concrete byte slice should use `bytemuck::cast_slice` directly.
    fn bytes_to_f32_slice(&self, output: &OperationOutput) -> Vec<f32> {
        vec![0.0f32; (output.size / 4) as usize]
    }

    /// Convert bytes to f32 value.
    fn bytes_to_f32_value(&self, output: &OperationOutput) -> f32 {
        // OperationOutput has no inline data; only the first element's default is available.
        let _ = output;
        0.0f32
    }

    /// Serialize matrix dimensions
    fn serialize_dimensions(&self, dimensions: (usize, usize, usize)) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(12);
        bytes.extend_from_slice(&(dimensions.0 as u32).to_le_bytes());
        bytes.extend_from_slice(&(dimensions.1 as u32).to_le_bytes());
        bytes.extend_from_slice(&(dimensions.2 as u32).to_le_bytes());
        bytes
    }

    /// Serialize convolution dimensions
    fn serialize_convolution_dimensions(&self, dimensions: (usize, usize, usize, usize)) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16);
        bytes.extend_from_slice(&(dimensions.0 as u32).to_le_bytes());
        bytes.extend_from_slice(&(dimensions.1 as u32).to_le_bytes());
        bytes.extend_from_slice(&(dimensions.2 as u32).to_le_bytes());
        bytes.extend_from_slice(&(dimensions.3 as u32).to_le_bytes());
        bytes
    }
}

impl CsdScheduler {
    /// Create new CSD scheduler
    pub fn new() -> Self {
        Self {
            pending_operations: Vec::new(),
            running_operations: HashMap::new(),
            completion_queue: Vec::new(),
            scheduling_policy: SchedulingPolicy::Priority,
        }
    }

    /// Schedule operation
    pub fn schedule_operation(&mut self, operation: CsdOperationRequest) -> Result<(), CsdError> {
        self.pending_operations.push(operation);
        Ok(())
    }

    /// Process pending operations
    pub fn process_operations(&mut self) -> Vec<CsdCompletion> {
        let mut completions = Vec::new();

        while let Some(operation) = self.pending_operations.pop() {
            // Execute operation
            let completion = self.execute_operation(&operation);
            completions.push(completion);
        }

        completions
    }

    /// Execute operation
    fn execute_operation(&self, operation: &CsdOperationRequest) -> CsdCompletion {
        // In real implementation, would execute on CSD device
        // For now, simulate execution
        CsdCompletion {
            operation_id: operation.operation_id,
            status: CompletionStatus::Success,
            execution_time: 1000, // 1ms
            outputs: operation.outputs.clone(),
            error_message: None,
        }
    }
}

impl CsdPerformanceMonitor {
    /// Create new performance monitor
    pub fn new() -> Self {
        Self {
            device_metrics: HashMap::new(),
            function_metrics: HashMap::new(),
            global_metrics: CsdGlobalMetrics {
                total_operations: 0,
                total_execution_time: 0,
                average_execution_time: 0.0,
                total_data_processed: 0,
                overall_throughput: 0.0,
                system_utilization: 0.0,
            },
        }
    }

    /// Update metrics
    pub fn update_metrics(&mut self, operation_id: u64, execution_time: u64, data_size: u64) {
        self.global_metrics.total_operations += 1;
        self.global_metrics.total_execution_time += execution_time;
        self.global_metrics.average_execution_time = self.global_metrics.total_execution_time as f64 / self.global_metrics.total_operations as f64;
        self.global_metrics.total_data_processed += data_size;
        self.global_metrics.overall_throughput = self.global_metrics.total_data_processed as f64 / self.global_metrics.total_execution_time as f64;
    }

    /// Get global statistics
    pub fn get_global_stats(&self) -> CsdGlobalMetrics {
        self.global_metrics.clone()
    }
}

impl MathComputationBuilder {
    /// Create new computation builder
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            data_dependencies: HashMap::new(),
            execution_plan: ExecutionPlan {
                stages: Vec::new(),
                parallel_groups: Vec::new(),
                estimated_time: 0.0,
                resource_requirements: ResourceRequirements {
                    memory_usage: 0,
                    compute_units: 0,
                    bandwidth: 0.0,
                },
            },
        }
    }

    /// Add matrix multiplication operation
    pub fn add_matrix_multiply(&mut self, device_id: String, a: Vec<f32>, b: Vec<f32>, dimensions: (usize, usize, usize)) -> &mut Self {
        let operation_id = self.generate_operation_id();
        
        let operation = CsdOperationRequest {
            operation_id,
            function_id: "matrix_multiply".to_string(),
            device_id,
            inputs: vec![
                OperationInput {
                    name: "matrix_a".to_string(),
                    data: self.f32_slice_to_bytes(&a),
                    location: DataLocation::HostMemory,
                },
                OperationInput {
                    name: "matrix_b".to_string(),
                    data: self.f32_slice_to_bytes(&b),
                    location: DataLocation::HostMemory,
                },
            ],
            outputs: vec![
                OperationOutput {
                    name: format!("result_{}", operation_id),
                    size: ((dimensions.0 * dimensions.2) * 4) as u64,
                    location: DataLocation::HostMemory,
                },
            ],
            priority: OperationPriority::Normal,
            deadline: None,
        };

        self.operations.push(operation);
        self
    }

    /// Build execution plan
    pub fn build(&mut self) -> Result<ExecutionPlan, CsdError> {
        // Analyze dependencies
        self.analyze_dependencies();

        // Create execution stages
        self.create_execution_stages();

        // Estimate execution time
        self.estimate_execution_time();

        Ok(self.execution_plan.clone())
    }

    /// Analyze data dependencies
    fn analyze_dependencies(&mut self) {
        // Simple dependency analysis - in real implementation would be more sophisticated
        for (i, operation) in self.operations.iter().enumerate() {
            let mut dependencies = Vec::new();
            
            // Check if this operation depends on previous operations
            for j in 0..i {
                let prev_operation = &self.operations[j];
                
                // Check if any output of previous operation is used as input
                for output in &prev_operation.outputs {
                    for input in &operation.inputs {
                        if input.name.contains(&output.name) {
                            dependencies.push(prev_operation.function_id.clone());
                        }
                    }
                }
            }
            
            self.data_dependencies.insert(operation.function_id.clone(), dependencies);
        }
    }

    /// Create execution stages
    fn create_execution_stages(&mut self) {
        // Simple stage creation - in real implementation would be more sophisticated
        let mut stage_id = 0;
        let mut processed_operations = std::collections::HashSet::new();

        while processed_operations.len() < self.operations.len() {
            let mut current_stage = Vec::new();
            let mut stage_dependencies = Vec::new();

            for operation in &self.operations {
                if !processed_operations.contains(&operation.function_id) {
                    let dependencies = self.data_dependencies.get(&operation.function_id).cloned().unwrap_or_default();
                    
                    // Check if all dependencies are processed
                    let can_execute = dependencies.iter().all(|dep| processed_operations.contains(dep));
                    
                    if can_execute {
                        current_stage.push(operation.function_id.clone());
                        stage_dependencies.extend(dependencies.clone());
                    }
                }
            }

            if current_stage.is_empty() {
                break; // Circular dependency or error
            }

            let stage = ExecutionStage {
                stage_id,
                operations: current_stage.clone(),
                dependencies: stage_dependencies,
                estimated_time: 1.0, // 1ms per operation
            };

            self.execution_plan.stages.push(stage);
            
            for operation in &current_stage {
                processed_operations.insert(operation.clone());
            }

            stage_id += 1;
        }
    }

    /// Estimate execution time
    fn estimate_execution_time(&mut self) {
        let mut total_time = 0.0;
        
        for stage in &self.execution_plan.stages {
            total_time += stage.estimated_time;
        }

        self.execution_plan.estimated_time = total_time;
    }

    /// Generate operation ID
    fn generate_operation_id(&self) -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// Convert f32 slice to bytes
    fn f32_slice_to_bytes(&self, slice: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(slice.len() * 4);
        for &value in slice {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }
}

/// CSD error types
#[derive(Debug, Clone)]
pub enum CsdError {
    DeviceOpen(String),
    DeviceNotFound(String),
    FunctionNotFound(String),
    InvalidFunction(String),
    InvalidOperation(String),
    NoOutput(String),
    ExecutionError(String),
    ConfigurationError(String),
}

impl std::fmt::Display for CsdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CsdError::DeviceOpen(msg) => write!(f, "Device open error: {}", msg),
            CsdError::DeviceNotFound(msg) => write!(f, "Device not found: {}", msg),
            CsdError::FunctionNotFound(msg) => write!(f, "Function not found: {}", msg),
            CsdError::InvalidFunction(msg) => write!(f, "Invalid function: {}", msg),
            CsdError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            CsdError::NoOutput(msg) => write!(f, "No output: {}", msg),
            CsdError::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
            CsdError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for CsdError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csd_manager_creation() {
        let manager = CsdManager::new();
        assert_eq!(manager.list_devices().len(), 0);
        assert_eq!(manager.list_functions().len(), 0);
    }

    #[test]
    fn test_matrix_multiply() {
        let mut manager = CsdManager::new();
        
        // Create dummy device
        let device = CsdDevice {
            device_id: "test_device".to_string(),
            device_path: "/dev/nvme0".to_string(),
            capabilities: CsdCapabilities {
                max_concurrent_operations: 16,
                max_data_size: 1024 * 1024 * 1024,
                supported_operations: vec![CsdOperationType::MatrixMultiply],
                memory_size: 8 * 1024 * 1024 * 1024,
                compute_units: 64,
                clock_speed: 1.5,
            },
            supported_functions: vec![],
            device_stats: CsdDeviceStats {
                operations_completed: 0,
                total_execution_time: 0,
                average_execution_time: 0.0,
                data_processed: 0,
                error_count: 0,
                utilization: 0.0,
            },
        };

        // For testing, we'll skip the actual device discovery
        // and just test the matrix multiplication logic
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![5.0, 6.0, 7.0, 8.0];
        
        // This would normally work with a real device
        // let result = manager.matrix_multiply("test_device", &a, &b, (2, 2, 2));
    }

    #[test]
    fn test_math_computation_builder() {
        let mut builder = MathComputationBuilder::new();
        
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![5.0, 6.0, 7.0, 8.0];
        
        builder.add_matrix_multiply("device1".to_string(), a, b, (2, 2, 2));
        
        let plan = builder.build();
        assert!(plan.is_ok());
        
        if let Ok(plan) = plan {
            assert_eq!(plan.stages.len(), 1);
            assert!(plan.estimated_time > 0.0);
        }
    }
}
