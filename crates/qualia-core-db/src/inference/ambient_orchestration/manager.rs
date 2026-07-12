//! Orchestration core: the ambient orchestration manager and the
//! sub-threshold orchestrator that drives policy adaptation.

use super::*;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

/// Ambient Orchestration Manager
pub struct AmbientOrchestrationManager {
    pub(super) devices: HashMap<String, AmbientDevice>,
    orchestrator: SubThresholdOrchestrator,
    power_manager: PowerManager,
    performance_monitor: AmbientPerformanceMonitor,
    task_scheduler: TaskScheduler,
}

/// Sub-threshold orchestrator
pub struct SubThresholdOrchestrator {
    orchestration_policy: OrchestrationPolicy,
    workload_analyzer: WorkloadAnalyzer,
    resource_allocator: ResourceAllocator,
    adaptation_engine: AdaptationEngine,
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
        // Feed execution history into workload analyzer
        let history = self.task_scheduler.recent_history(10);
        for record in history {
            let sample = WorkloadSample {
                timestamp: record.end_time,
                cpu_usage: record.resource_usage.compute_units_used as f64 / 100.0,
                memory_usage: record.resource_usage.memory_used as f64 / (1024.0 * 1024.0),
                neural_engine_usage: record.resource_usage.neural_engines_used as f64,
                power_consumption: record.resource_usage.power_consumed,
                thermal_state: record.resource_usage.thermal_impact,
                battery_level: self.power_manager.get_battery_level(&record.device_id),
            };
            self.orchestrator.workload_analyzer.record_sample(sample);
        }

        // Analyze current workload
        let workload_analysis = self.orchestrator.workload_analyzer.analyze_workload();

        // Adapt orchestration policy (mutable: records adaptation history)
        let new_policy = self
            .orchestrator
            .adaptation_engine
            .adapt_policy(workload_analysis);

        // Adjust based on active power policy
        let adjusted_policy = match self.power_manager.power_policy() {
            PowerPolicy::PowerSaving | PowerPolicy::UltraPowerSaving => {
                OrchestrationPolicy::BatteryAware
            }
            _ => new_policy,
        };

        // Update orchestration policy
        self.orchestrator.orchestration_policy = adjusted_policy;

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
