//! Inference engine, scheduling, batching, and tuning impls.

use super::*;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::collections::HashMap;

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            inference_backends: HashMap::new(),
            request_scheduler: RequestScheduler::new(),
            batch_processor: BatchProcessor::new(),
            performance_optimizer: InferenceOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.request_scheduler.initialize()?;
        self.batch_processor.initialize()?;
        self.performance_optimizer.initialize()?;
        Ok(())
    }

    /// Register an inference backend under its backend id.
    pub fn register_backend(&mut self, backend: InferenceBackend) {
        self.inference_backends
            .insert(backend.backend_id.clone(), backend);
    }

    /// Get a registered inference backend by id.
    pub fn get_backend(&self, backend_id: &str) -> Option<&InferenceBackend> {
        self.inference_backends.get(backend_id)
    }

    /// List the ids of all registered inference backends.
    pub fn list_backends(&self) -> Vec<String> {
        self.inference_backends.keys().cloned().collect()
    }

    /// Remove a registered inference backend by id. Returns `true` if removed.
    pub fn remove_backend(&mut self, backend_id: &str) -> bool {
        self.inference_backends.remove(backend_id).is_some()
    }

    pub fn execute_inference(
        &mut self,
        request: &InferenceRequest,
        model: &Model,
    ) -> Result<InferenceResult, MLError> {
        // Wired to a real (if basic) forward pass over the model's architecture, using the
        // model's `weights` field as the flattened parameter buffer and the element-wise
        // activation math from `crate::solvers::activation`. This is an MLP inference backend:
        // it supports `Linear` (and pass-through `Activation`/`Dropout`) layers and returns a
        // clear error for layer types it cannot yet evaluate (Convolutional, Attention, …).
        // For production autoregressive LLM inference, route to the native gguf_bridge engine.
        let start = std::time::Instant::now();

        // Decode the request's byte payload as a little-endian f64 input vector.
        let input = decode_f64_le(&request.input_data).ok_or_else(|| {
            MLError::DataError(format!(
                "input_data length ({}) is not a multiple of {} (f64 size)",
                request.input_data.len(),
                std::mem::size_of::<f64>()
            ))
        })?;

        // Run the forward pass over the model's layers.
        let output = Self::forward_pass(model, &input)?;

        // Re-encode the output vector as little-endian f64 bytes.
        let output_data = encode_f64_le(&output);

        let inference_time = start.elapsed().as_millis() as u64;

        Ok(InferenceResult {
            result_id: format!("result_{}", request.request_id),
            output_data,
            inference_time,
            // This backend computes a deterministic forward pass, not a probabilistic model,
            // so there is no calibrated confidence to report. Surface 1.0 to indicate the pass
            // completed successfully (callers needing real confidence should use gguf_bridge).
            confidence: 1.0,
            metadata: ResultMetadata {
                model_id: model.model_id.clone(),
                backend_id: "linear_algebra_mlp".to_string(),
                batch_size: request.parameters.batch_size,
                sequence_length: request.parameters.sequence_length,
                tokens_generated: output.len(),
            },
        })
    }

    /// Run a basic MLP forward pass over the model's architecture.
    ///
    /// For each `Linear` layer the flattened `model.weights` buffer is consumed in order:
    /// first the `output_size × input_size` weight matrix (row-major), then the `output_size`
    /// bias vector. The layer output is `activation(W · x + b)`, with the activation drawn from
    /// `crate::solvers::activation`. `Activation` layers apply their activation in place and
    /// `Dropout` is the identity at inference time. All other layer types return a clear error.
    pub(super) fn forward_pass(model: &Model, input: &[f64]) -> Result<Vec<f64>, MLError> {
        let layers = &model.architecture.layers;
        if layers.is_empty() {
            return Err(MLError::InferenceError(
                "model architecture has no layers".to_string(),
            ));
        }

        let mut activations = input.to_vec();
        let mut weight_offset = 0usize;

        for (idx, layer) in layers.iter().enumerate() {
            match layer.layer_type {
                LayerType::Linear => {
                    let in_size = layer.input_shape.first().copied().ok_or_else(|| {
                        MLError::InferenceError(format!(
                            "layer {} ({}): missing input dimension",
                            idx, layer.layer_id
                        ))
                    })?;
                    let out_size = layer.output_shape.first().copied().ok_or_else(|| {
                        MLError::InferenceError(format!(
                            "layer {} ({}): missing output dimension",
                            idx, layer.layer_id
                        ))
                    })?;

                    if activations.len() != in_size {
                        return Err(MLError::InferenceError(format!(
                            "layer {} ({}): expected input size {}, got {}",
                            idx,
                            layer.layer_id,
                            in_size,
                            activations.len()
                        )));
                    }

                    let weight_count = in_size * out_size;
                    let bias_count = out_size;
                    let needed = weight_count + bias_count;
                    if weight_offset + needed > model.weights.len() {
                        return Err(MLError::InferenceError(format!(
                            "layer {} ({}): not enough weights (need {} at offset {}, have {})",
                            idx,
                            layer.layer_id,
                            needed,
                            weight_offset,
                            model.weights.len()
                        )));
                    }

                    // output[j] = sum_i W[j*in_size + i] * x[i] + bias[j]
                    let mut out = vec![0.0f64; out_size];
                    for j in 0..out_size {
                        let mut acc = 0.0;
                        for i in 0..in_size {
                            acc += model.weights[weight_offset + j * in_size + i] * activations[i];
                        }
                        acc += model.weights[weight_offset + weight_count + j];
                        out[j] = acc;
                    }
                    weight_offset += needed;

                    if let Some(act) = &layer.activation {
                        apply_activation(&mut out, act)?;
                    }
                    activations = out;
                }
                LayerType::Activation => {
                    if let Some(act) = &layer.activation {
                        apply_activation(&mut activations, act)?;
                    } else {
                        return Err(MLError::InferenceError(format!(
                            "layer {} ({}): Activation layer has no activation function set",
                            idx, layer.layer_id
                        )));
                    }
                }
                LayerType::Dropout => {
                    // Dropout is the identity at inference time.
                }
                ref other => {
                    return Err(MLError::InferenceError(format!(
                        "layer {} ({}): {:?} layers are not yet supported by the MLP inference \
                         backend (only Linear/Activation/Dropout); use the native gguf_bridge \
                         engine for transformer/cnn workloads",
                        idx, layer.layer_id, other
                    )));
                }
            }
        }

        Ok(activations)
    }
}

/// Decode a byte slice as a little-endian `f64` vector. Returns `None` if the length is not a
/// multiple of 8 (the size of `f64`).
fn decode_f64_le(bytes: &[u8]) -> Option<Vec<f64>> {
    if bytes.len() % std::mem::size_of::<f64>() != 0 {
        return None;
    }
    Some(
        bytes
            .chunks_exact(std::mem::size_of::<f64>())
            .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
            .collect(),
    )
}

/// Encode an `f64` slice as a little-endian byte vector.
fn encode_f64_le(values: &[f64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * std::mem::size_of::<f64>());
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// Apply an activation function in place, dispatching to `crate::solvers::activation` for the
/// standard element-wise maps.
fn apply_activation(buf: &mut [f64], act: &ActivationFunction) -> Result<(), MLError> {
    match act {
        ActivationFunction::ReLU => crate::solvers::activation::relu(buf),
        ActivationFunction::Sigmoid => crate::solvers::activation::sigmoid(buf),
        ActivationFunction::Tanh => crate::solvers::activation::tanh(buf),
        ActivationFunction::GELU => crate::solvers::activation::gelu(buf),
        ActivationFunction::Softmax => crate::solvers::activation::softmax(buf),
        ActivationFunction::Swish => crate::solvers::activation::silu(buf),
        ActivationFunction::LeakyReLU => {
            // Leaky ReLU: x if x >= 0 else 0.01·x, element-wise.
            const SLOPE: f64 = 0.01;
            for v in buf.iter_mut() {
                if *v < 0.0 {
                    *v *= SLOPE;
                }
            }
        }
        ActivationFunction::ELU => {
            // ELU: x if x >= 0 else e^x − 1, element-wise (α = 1).
            for v in buf.iter_mut() {
                if *v < 0.0 {
                    *v = (*v).exp() - 1.0;
                }
            }
        }
        ActivationFunction::Custom(name) => {
            return Err(MLError::InferenceError(format!(
                "custom activation '{}' is not supported by the MLP inference backend",
                name
            )));
        }
    }
    Ok(())
}

impl InferenceBackend {
    pub fn new() -> Self {
        Self {
            backend_id: "backend_1".to_string(),
            backend_type: InferenceBackendType::GPU,
            capabilities: BackendCapabilities::new(),
            current_load: 0.5,
        }
    }
}

impl BackendCapabilities {
    pub fn new() -> Self {
        Self {
            supported_models: vec!["gpt-3".to_string(), "bert".to_string()],
            max_batch_size: 32,
            max_sequence_length: 2048,
            supported_precisions: vec![Precision::FP16, Precision::FP32],
            memory_limit: 8 * 1024 * 1024 * 1024, // 8GB
            throughput: 100.0,
        }
    }
}

impl RequestScheduler {
    pub fn new() -> Self {
        Self {
            scheduling_policy: SchedulingPolicy::Priority,
            queue_manager: QueueManager::new(),
            load_balancer: LoadBalancer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Return the current scheduling policy.
    pub fn scheduling_policy(&self) -> &SchedulingPolicy {
        &self.scheduling_policy
    }

    /// Set the scheduling policy.
    pub fn set_scheduling_policy(&mut self, policy: SchedulingPolicy) {
        self.scheduling_policy = policy;
    }

    /// Return a reference to the queue manager.
    pub fn queue_manager(&self) -> &QueueManager {
        &self.queue_manager
    }

    /// Return a mutable reference to the queue manager.
    pub fn queue_manager_mut(&mut self) -> &mut QueueManager {
        &mut self.queue_manager
    }

    /// Return a reference to the load balancer.
    pub fn load_balancer(&self) -> &LoadBalancer {
        &self.load_balancer
    }

    /// Return a mutable reference to the load balancer.
    pub fn load_balancer_mut(&mut self) -> &mut LoadBalancer {
        &mut self.load_balancer
    }

    pub fn schedule_request(&mut self, _request: &InferenceRequest) -> Result<String, MLError> {
        // Simplified scheduling - return backend ID
        Ok("backend_1".to_string())
    }
}

impl QueueManager {
    pub fn new() -> Self {
        Self {
            pending_requests: Vec::new(),
            running_requests: HashMap::new(),
            completed_requests: Vec::new(),
        }
    }

    /// Enqueue a pending inference request.
    pub fn enqueue(&mut self, request: InferenceRequest) {
        self.pending_requests.push(request);
    }

    /// Dequeue the next pending request (FIFO order).
    pub fn dequeue(&mut self) -> Option<InferenceRequest> {
        if self.pending_requests.is_empty() {
            None
        } else {
            Some(self.pending_requests.remove(0))
        }
    }

    /// Mark a request as running on a given backend.
    pub fn start_request(&mut self, running: RunningRequest) {
        self.running_requests
            .insert(running.request_id.clone(), running);
    }

    /// Mark a running request as completed, removing it from the running set
    /// and appending it to the completed list.
    pub fn complete_request(&mut self, request_id: &str, completed: CompletedRequest) {
        self.running_requests.remove(request_id);
        self.completed_requests.push(completed);
    }

    /// Return a reference to the pending requests queue.
    pub fn pending_requests(&self) -> &[InferenceRequest] {
        &self.pending_requests
    }

    /// Return a reference to the running requests map.
    pub fn running_requests(&self) -> &HashMap<String, RunningRequest> {
        &self.running_requests
    }

    /// Return a reference to the completed requests list.
    pub fn completed_requests(&self) -> &[CompletedRequest] {
        &self.completed_requests
    }

    /// Number of pending requests.
    pub fn pending_count(&self) -> usize {
        self.pending_requests.len()
    }

    /// Number of currently running requests.
    pub fn running_count(&self) -> usize {
        self.running_requests.len()
    }
}

impl LoadBalancer {
    pub fn new() -> Self {
        Self {
            balancing_strategy: LoadBalancingStrategy::RoundRobin,
            backend_metrics: HashMap::new(),
            health_checker: HealthChecker::new(),
        }
    }

    /// Return the current load-balancing strategy.
    pub fn balancing_strategy(&self) -> &LoadBalancingStrategy {
        &self.balancing_strategy
    }

    /// Set the load-balancing strategy.
    pub fn set_balancing_strategy(&mut self, strategy: LoadBalancingStrategy) {
        self.balancing_strategy = strategy;
    }

    /// Record or update metrics for a backend.
    pub fn record_backend_metrics(&mut self, metrics: BackendMetrics) {
        self.backend_metrics
            .insert(metrics.backend_id.clone(), metrics);
    }

    /// Get metrics for a specific backend.
    pub fn get_backend_metrics(&self, backend_id: &str) -> Option<&BackendMetrics> {
        self.backend_metrics.get(backend_id)
    }

    /// List the ids of all backends with recorded metrics.
    pub fn list_backend_metrics(&self) -> Vec<String> {
        self.backend_metrics.keys().cloned().collect()
    }

    /// Return a reference to the health checker.
    pub fn health_checker(&self) -> &HealthChecker {
        &self.health_checker
    }

    /// Return a mutable reference to the health checker.
    pub fn health_checker_mut(&mut self) -> &mut HealthChecker {
        &mut self.health_checker
    }
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            health_checks: HashMap::new(),
            check_interval: 30, // 30 seconds
            timeout: 5,         // 5 seconds
        }
    }

    /// Register a health check under its check id.
    pub fn register_health_check(&mut self, check: HealthCheck) {
        self.health_checks.insert(check.check_id.clone(), check);
    }

    /// Get a registered health check by id.
    pub fn get_health_check(&self, check_id: &str) -> Option<&HealthCheck> {
        self.health_checks.get(check_id)
    }

    /// List the ids of all registered health checks.
    pub fn list_health_checks(&self) -> Vec<String> {
        self.health_checks.keys().cloned().collect()
    }

    /// Remove a registered health check by id. Returns `true` if removed.
    pub fn remove_health_check(&mut self, check_id: &str) -> bool {
        self.health_checks.remove(check_id).is_some()
    }

    /// Return the check interval (seconds).
    pub fn check_interval(&self) -> u64 {
        self.check_interval
    }

    /// Set the check interval (seconds).
    pub fn set_check_interval(&mut self, interval: u64) {
        self.check_interval = interval;
    }

    /// Return the timeout (seconds).
    pub fn timeout(&self) -> u64 {
        self.timeout
    }

    /// Set the timeout (seconds).
    pub fn set_timeout(&mut self, timeout: u64) {
        self.timeout = timeout;
    }
}

impl HealthCheck {
    pub fn new() -> Self {
        Self {
            check_id: "health_1".to_string(),
            check_type: HealthCheckType::HTTP,
            endpoint: "/health".to_string(),
            expected_response: "OK".to_string(),
        }
    }
}

impl BatchProcessor {
    pub fn new() -> Self {
        Self {
            batching_strategy: BatchingStrategy::FixedSize,
            batch_size: 32,
            batch_timeout: 100, // 100ms
            batch_optimizer: BatchOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.batch_optimizer.initialize()?;
        Ok(())
    }

    /// Return the current batching strategy.
    pub fn batching_strategy(&self) -> &BatchingStrategy {
        &self.batching_strategy
    }

    /// Set the batching strategy.
    pub fn set_batching_strategy(&mut self, strategy: BatchingStrategy) {
        self.batching_strategy = strategy;
    }

    /// Return the configured batch size.
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Set the batch size.
    pub fn set_batch_size(&mut self, size: usize) {
        self.batch_size = size;
    }

    /// Return the configured batch timeout (milliseconds).
    pub fn batch_timeout(&self) -> u64 {
        self.batch_timeout
    }

    /// Set the batch timeout (milliseconds).
    pub fn set_batch_timeout(&mut self, timeout: u64) {
        self.batch_timeout = timeout;
    }
}

impl BatchOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_algorithms: HashMap::new(),
            optimization_metrics: BatchOptimizationMetrics::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Register a batch optimization algorithm under the given name.
    pub fn register_algorithm(&mut self, name: &str, algorithm: BatchOptimizationAlgorithm) {
        self.optimization_algorithms
            .insert(name.to_string(), algorithm);
    }

    /// Get a registered batch optimization algorithm by name.
    pub fn get_algorithm(&self, name: &str) -> Option<&BatchOptimizationAlgorithm> {
        self.optimization_algorithms.get(name)
    }

    /// List the names of all registered batch optimization algorithms.
    pub fn list_algorithms(&self) -> Vec<String> {
        self.optimization_algorithms.keys().cloned().collect()
    }

    /// Return a reference to the optimization metrics.
    pub fn optimization_metrics(&self) -> &BatchOptimizationMetrics {
        &self.optimization_metrics
    }

    /// Return a mutable reference to the optimization metrics.
    pub fn optimization_metrics_mut(&mut self) -> &mut BatchOptimizationMetrics {
        &mut self.optimization_metrics
    }
}

impl BatchOptimizationMetrics {
    pub fn new() -> Self {
        Self {
            average_batch_size: 32.0,
            throughput: 100.0,
            latency: 10.0,
            memory_utilization: 0.5,
        }
    }
}

impl InferenceOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_strategies: vec![InferenceOptimizationStrategy::ModelQuantization],
            performance_analyzer: PerformanceAnalyzer::new(),
            auto_tuner: AutoTuner::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.performance_analyzer.initialize()?;
        self.auto_tuner.initialize()?;
        Ok(())
    }

    /// Return a reference to the configured optimization strategies.
    pub fn optimization_strategies(&self) -> &[InferenceOptimizationStrategy] {
        &self.optimization_strategies
    }

    /// Add an optimization strategy to the configured set.
    pub fn add_optimization_strategy(&mut self, strategy: InferenceOptimizationStrategy) {
        self.optimization_strategies.push(strategy);
    }

    /// Replace the full set of optimization strategies.
    pub fn set_optimization_strategies(&mut self, strategies: Vec<InferenceOptimizationStrategy>) {
        self.optimization_strategies = strategies;
    }
}

impl PerformanceAnalyzer {
    pub fn new() -> Self {
        Self {
            analysis_methods: vec![AnalysisMethod::Profiling],
            performance_profiles: HashMap::new(),
            bottleneck_detector: BottleneckDetector::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.bottleneck_detector.initialize()?;
        Ok(())
    }

    /// Return a reference to the configured analysis methods.
    pub fn analysis_methods(&self) -> &[AnalysisMethod] {
        &self.analysis_methods
    }

    /// Add an analysis method to the configured set.
    pub fn add_analysis_method(&mut self, method: AnalysisMethod) {
        self.analysis_methods.push(method);
    }

    /// Register a performance profile under its profile id.
    pub fn register_profile(&mut self, profile: PerformanceProfile) {
        self.performance_profiles
            .insert(profile.profile_id.clone(), profile);
    }

    /// Get a registered performance profile by id.
    pub fn get_profile(&self, profile_id: &str) -> Option<&PerformanceProfile> {
        self.performance_profiles.get(profile_id)
    }

    /// List the ids of all registered performance profiles.
    pub fn list_profiles(&self) -> Vec<String> {
        self.performance_profiles.keys().cloned().collect()
    }
}

impl PerformanceProfile {
    pub fn new() -> Self {
        Self {
            profile_id: "profile_1".to_string(),
            model_id: "model_1".to_string(),
            backend_id: "backend_1".to_string(),
            metrics: PerformanceMetrics::new(),
            characteristics: PerformanceCharacteristics::new(),
        }
    }
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            latency: 10.0,
            throughput: 100.0,
            accuracy: 0.0, // not measured (scaffold default; no evaluation performed)
            memory_usage: 1024 * 1024, // 1MB
        }
    }
}

impl PerformanceCharacteristics {
    pub fn new() -> Self {
        Self {
            compute_bound: true,
            memory_bound: false,
            io_bound: false,
            network_bound: false,
        }
    }
}

impl BottleneckDetector {
    pub fn new() -> Self {
        Self {
            detection_algorithms: vec![BottleneckDetectionAlgorithm::Statistical],
            detection_thresholds: DetectionThresholds::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Return a reference to the configured detection algorithms.
    pub fn detection_algorithms(&self) -> &[BottleneckDetectionAlgorithm] {
        &self.detection_algorithms
    }

    /// Add a detection algorithm to the configured set.
    pub fn add_detection_algorithm(&mut self, algorithm: BottleneckDetectionAlgorithm) {
        self.detection_algorithms.push(algorithm);
    }

    /// Return a reference to the detection thresholds.
    pub fn detection_thresholds(&self) -> &DetectionThresholds {
        &self.detection_thresholds
    }

    /// Return a mutable reference to the detection thresholds.
    pub fn detection_thresholds_mut(&mut self) -> &mut DetectionThresholds {
        &mut self.detection_thresholds
    }
}

impl DetectionThresholds {
    pub fn new() -> Self {
        Self {
            cpu_threshold: 0.8,
            memory_threshold: 0.8,
            io_threshold: 0.8,
            network_threshold: 0.8,
        }
    }
}

impl AutoTuner {
    pub fn new() -> Self {
        Self {
            tuning_algorithms: HashMap::new(),
            tuning_objectives: Vec::new(),
            tuning_history: TuningHistory::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Register a tuning algorithm under the given name.
    pub fn register_tuning_algorithm(&mut self, name: &str, algorithm: TuningAlgorithm) {
        self.tuning_algorithms.insert(name.to_string(), algorithm);
    }

    /// Get a registered tuning algorithm by name.
    pub fn get_tuning_algorithm(&self, name: &str) -> Option<&TuningAlgorithm> {
        self.tuning_algorithms.get(name)
    }

    /// List the names of all registered tuning algorithms.
    pub fn list_tuning_algorithms(&self) -> Vec<String> {
        self.tuning_algorithms.keys().cloned().collect()
    }

    /// Add a tuning objective to the configured set.
    pub fn add_tuning_objective(&mut self, objective: TuningObjective) {
        self.tuning_objectives.push(objective);
    }

    /// Return a reference to the configured tuning objectives.
    pub fn tuning_objectives(&self) -> &[TuningObjective] {
        &self.tuning_objectives
    }

    /// Return a reference to the tuning history.
    pub fn tuning_history(&self) -> &TuningHistory {
        &self.tuning_history
    }

    /// Return a mutable reference to the tuning history.
    pub fn tuning_history_mut(&mut self) -> &mut TuningHistory {
        &mut self.tuning_history
    }
}

impl TuningHistory {
    pub fn new() -> Self {
        Self {
            tuning_records: Vec::new(),
            best_configurations: HashMap::new(),
        }
    }

    /// Append a tuning record to the history.
    pub fn add_record(&mut self, record: TuningRecord) {
        self.tuning_records.push(record);
    }

    /// Return a reference to all tuning records.
    pub fn records(&self) -> &[TuningRecord] {
        &self.tuning_records
    }

    /// Record a best configuration for a given objective name.
    pub fn record_best_configuration(&mut self, objective: &str, config: TuningConfiguration) {
        self.best_configurations
            .insert(objective.to_string(), config);
    }

    /// Get the best configuration for a given objective.
    pub fn get_best_configuration(&self, objective: &str) -> Option<&TuningConfiguration> {
        self.best_configurations.get(objective)
    }

    /// List the objective names that have a recorded best configuration.
    pub fn list_best_configurations(&self) -> Vec<String> {
        self.best_configurations.keys().cloned().collect()
    }
}

impl TuningRecord {
    pub fn new() -> Self {
        Self {
            record_id: "record_1".to_string(),
            timestamp: 0,
            configuration: TuningConfiguration::new(),
            performance: PerformanceMetrics::new(),
            improvement: 0.0,
        }
    }
}

impl TuningConfiguration {
    pub fn new() -> Self {
        Self {
            configuration_id: "config_1".to_string(),
            parameters: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
}
