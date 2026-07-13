//! Training engine, data pipeline, and hyperparameter tuning impls.

use super::*;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::collections::HashMap;

impl TrainingEngine {
    pub fn new() -> Self {
        Self {
            training_backends: HashMap::new(),
            training_scheduler: TrainingScheduler::new(),
            data_pipeline: DataPipeline::new(),
            training_optimizer: TrainingOptimizer::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.training_scheduler.initialize()?;
        self.data_pipeline.initialize()?;
        self.training_optimizer.initialize()?;
        Ok(())
    }

    /// Register a training backend under its backend id.
    pub fn register_backend(&mut self, backend: TrainingBackend) {
        self.training_backends
            .insert(backend.backend_id.clone(), backend);
    }

    /// Get a registered training backend by id.
    pub fn get_backend(&self, backend_id: &str) -> Option<&TrainingBackend> {
        self.training_backends.get(backend_id)
    }

    /// List the ids of all registered training backends.
    pub fn list_backends(&self) -> Vec<String> {
        self.training_backends.keys().cloned().collect()
    }

    /// Remove a registered training backend by id. Returns `true` if removed.
    pub fn remove_backend(&mut self, backend_id: &str) -> bool {
        self.training_backends.remove(backend_id).is_some()
    }

    /// Start a training *job* (the catalog/scheduler path). This records the job with the
    /// scheduler but performs no weight updates; use [`start_training`] for the real SGD
    /// loop that mutates a model's weights.
    pub fn start_training_job(&mut self, _job: &TrainingJob) -> Result<(), MLError> {
        // Start training job
        Ok(())
    }

    /// Run a real stochastic gradient descent (SGD) training loop on a linear model.
    ///
    /// This implements basic batch SGD for a single `Linear` layer with no activation
    /// (i.e. linear regression). For each epoch the samples are deterministically shuffled
    /// (a fixed-seed Fisher–Yates, so runs are reproducible), then processed in batches:
    /// a forward pass computes predictions, the MSE loss and its gradients are computed,
    /// and the weights are updated as `W -= learning_rate * gradient`.
    ///
    /// `training_data` is a flat buffer of inputs laid out as
    /// `[s0_i0, s0_i1, ..., s1_i0, ...]` with `input_size = architecture.layers[0].input_shape[0]`.
    /// `targets` is laid out as `[s0_o0, s0_o1, ..., s1_o0, ...]` with
    /// `output_size = architecture.layers[0].output_shape[0]`.
    ///
    /// Only `TrainingAlgorithm::SGD` is implemented here; other optimizers return a clear
    /// error rather than silently degrading. Only a single `Linear` layer with no
    /// activation is supported (the explicit scope of this training backend).
    pub fn start_training(
        &mut self,
        model: &mut Model,
        training_data: &[f64],
        targets: &[f64],
        config: &TrainingConfig,
    ) -> Result<TrainingResult, MLError> {
        self.start_training_impl(model, training_data, targets, config, None)
    }

    /// Run SGD recovery while preserving an unstructured pruning mask.
    ///
    /// Masked weights are forced to zero before training and skipped during
    /// every optimizer update, so recovery cannot silently regrow them.
    pub fn start_training_with_pruning_mask(
        &mut self,
        model: &mut Model,
        training_data: &[f64],
        targets: &[f64],
        config: &TrainingConfig,
        pruning_mask: &[u8],
    ) -> Result<TrainingResult, MLError> {
        self.start_training_impl(model, training_data, targets, config, Some(pruning_mask))
    }

    fn start_training_impl(
        &mut self,
        model: &mut Model,
        training_data: &[f64],
        targets: &[f64],
        config: &TrainingConfig,
        pruning_mask: Option<&[u8]>,
    ) -> Result<TrainingResult, MLError> {
        // --- Validate the optimizer. ---
        if config.optimizer != TrainingAlgorithm::SGD {
            return Err(MLError::TrainingError(format!(
                "start_training currently implements SGD only; {:?} is not yet supported \
                 by this training backend",
                config.optimizer
            )));
        }

        // --- Validate the model architecture: exactly one Linear layer, no activation. ---
        let layers = &model.architecture.layers;
        if layers.len() != 1 {
            return Err(MLError::TrainingError(format!(
                "start_training (SGD) supports a single Linear layer; this model has {} \
                 layers",
                layers.len()
            )));
        }
        let layer = &layers[0];
        if layer.layer_type != LayerType::Linear {
            return Err(MLError::TrainingError(format!(
                "start_training (SGD) supports only Linear layers; layer '{}' is {:?}",
                layer.layer_id, layer.layer_type
            )));
        }
        if layer.activation.is_some() {
            return Err(MLError::TrainingError(format!(
                "start_training (SGD) implements linear regression (no activation); layer \
                 '{}' has an activation function set",
                layer.layer_id
            )));
        }
        let in_size = layer
            .input_shape
            .first()
            .copied()
            .ok_or_else(|| MLError::TrainingError("layer missing input dimension".into()))?;
        let out_size = layer
            .output_shape
            .first()
            .copied()
            .ok_or_else(|| MLError::TrainingError("layer missing output dimension".into()))?;

        let weight_count = in_size * out_size;
        let bias_count = out_size;
        let needed = weight_count + bias_count;
        if model.weights.len() < needed {
            return Err(MLError::TrainingError(format!(
                "model has {} weights but the linear layer needs {} ({} weights + {} bias)",
                model.weights.len(),
                needed,
                weight_count,
                bias_count
            )));
        }
        if let Some(mask) = pruning_mask {
            let mask_bytes = ModelCompression::pruning_mask_bytes(needed);
            if mask.len() < mask_bytes {
                return Err(MLError::ResourceError(format!(
                    "training pruning mask needs {} bytes, got {}",
                    mask_bytes,
                    mask.len()
                )));
            }
            for index in 0..needed {
                if !ModelCompression::mask_keeps(mask, index) {
                    model.weights[index] = 0.0;
                }
            }
        }

        // --- Validate the data shapes. ---
        if in_size == 0 {
            return Err(MLError::TrainingError("input dimension is zero".into()));
        }
        if training_data.len() % in_size != 0 {
            return Err(MLError::DataError(format!(
                "training_data length ({}) is not a multiple of input_size ({})",
                training_data.len(),
                in_size
            )));
        }
        let n_samples = training_data.len() / in_size;
        if n_samples == 0 {
            return Err(MLError::DataError("no training samples".into()));
        }
        if targets.len() != n_samples * out_size {
            return Err(MLError::DataError(format!(
                "targets length ({}) does not match n_samples ({}) * output_size ({})",
                targets.len(),
                n_samples,
                out_size
            )));
        }
        if config.batch_size == 0 {
            return Err(MLError::TrainingError("batch_size must be > 0".into()));
        }

        let start = std::time::Instant::now();

        // --- Initial loss (before any weight update). ---
        let initial_loss =
            Self::full_dataset_mse(&model.weights, training_data, targets, in_size, out_size);

        let mut last_loss = initial_loss;
        let mut epochs_completed: usize = 0;
        let mut convergence_achieved = false;

        const CONVERGENCE_THRESHOLD: f64 = 1e-9;

        for epoch in 0..config.epochs {
            // Deterministic shuffle of sample indices (fixed seed → reproducible runs).
            let order = deterministic_shuffle(n_samples, epoch as u64);

            let batch_size = config.batch_size.min(n_samples);
            for chunk in order.chunks(batch_size) {
                // Accumulate batch gradients over the samples in this batch.
                let mut grad = vec![0.0f64; needed];
                let b = chunk.len() as f64;
                for &s in chunk {
                    let x = &training_data[s * in_size..s * in_size + in_size];
                    let t = &targets[s * out_size..s * out_size + out_size];
                    // Forward pass for this sample.
                    let pred = forward_linear(&model.weights, x, in_size, out_size);
                    // Per-sample gradient contribution: dL/dW and dL/db for MSE.
                    // L_s = sum_j (pred_j - t_j)^2 ; averaged over batch and outputs below.
                    for j in 0..out_size {
                        let diff = pred[j] - t[j];
                        // dL_s/dW[j*in+i] = 2 * diff * x[i]
                        for i in 0..in_size {
                            grad[j * in_size + i] += 2.0 * diff * x[i];
                        }
                        // dL_s/db[j] = 2 * diff
                        grad[weight_count + j] += 2.0 * diff;
                    }
                }

                // Average over (batch_size * out_size) and apply the learning rate.
                let scale = config.learning_rate / (b * out_size as f64);
                for k in 0..needed {
                    if pruning_mask
                        .map(|mask| ModelCompression::mask_keeps(mask, k))
                        .unwrap_or(true)
                    {
                        model.weights[k] -= scale * grad[k];
                    }
                }
            }

            epochs_completed = (epoch + 1) as usize;
            let loss =
                Self::full_dataset_mse(&model.weights, training_data, targets, in_size, out_size);
            if (last_loss - loss).abs() < CONVERGENCE_THRESHOLD {
                convergence_achieved = true;
                last_loss = loss;
                break;
            }
            last_loss = loss;
        }

        let training_time_ms = start.elapsed().as_millis() as u64;

        Ok(TrainingResult {
            initial_loss,
            final_loss: last_loss,
            epochs_completed,
            convergence_achieved,
            training_time_ms,
        })
    }

    /// Mean squared error between predictions and targets: `(1/N) * sum (p - t)^2`.
    pub fn compute_mse(predictions: &[f64], targets: &[f64]) -> f64 {
        if predictions.is_empty() || predictions.len() != targets.len() {
            return 0.0;
        }
        let n = predictions.len() as f64;
        let sum: f64 = predictions
            .iter()
            .zip(targets.iter())
            .map(|(p, t)| {
                let d = p - t;
                d * d
            })
            .sum();
        sum / n
    }

    /// Compute the MSE gradient (scaled by `learning_rate`) for a single linear sample.
    ///
    /// For a linear model `y = W·x + b` with `weights = [W (out×in), b (out)]`, the MSE
    /// loss for one sample is `L = sum_j (pred_j - target_j)^2`. The returned vector has
    /// the same layout as `weights` and contains `learning_rate * dL/dweight`, i.e. the
    /// amount to *subtract* from each weight. (For a single-output model `pred` and
    /// `target` are scalars and `inputs` is the input vector.)
    pub fn compute_gradients(
        weights: &[f64],
        inputs: &[f64],
        prediction: f64,
        target: f64,
        learning_rate: f64,
    ) -> Vec<f64> {
        // Infer the layout: weights = [W (out×in), b (out)].
        // weights.len() = out_size * in_size + out_size = out_size * (in_size + 1)
        //  =>  out_size = weights.len() / (in_size + 1)
        let in_size = inputs.len();
        if in_size == 0 {
            return Vec::new();
        }
        let out_size = if weights.len() >= in_size + 1 {
            weights.len() / (in_size + 1)
        } else {
            1
        };
        let weight_count = in_size * out_size;
        let diff = prediction - target;
        let mut grad = vec![0.0f64; weights.len()];
        for j in 0..out_size {
            // For the single-output case the prediction/target are the scalar values.
            let d = if out_size == 1 { diff } else { diff };
            for i in 0..in_size {
                grad[j * in_size + i] = learning_rate * 2.0 * d * inputs[i];
            }
            grad[weight_count + j] = learning_rate * 2.0 * d;
        }
        grad
    }

    /// MSE over the full dataset using the model's current weights (helper for the loop).
    fn full_dataset_mse(
        weights: &[f64],
        training_data: &[f64],
        targets: &[f64],
        in_size: usize,
        out_size: usize,
    ) -> f64 {
        let n_samples = training_data.len() / in_size;
        let mut preds = Vec::with_capacity(n_samples * out_size);
        for s in 0..n_samples {
            let x = &training_data[s * in_size..s * in_size + in_size];
            let pred = forward_linear(weights, x, in_size, out_size);
            preds.extend_from_slice(&pred);
        }
        Self::compute_mse(&preds, targets)
    }
}

/// Forward pass for a single `Linear` layer (no activation): `out = W·x + b`.
/// `weights` = `[W (out×in, row-major), b (out)]`.
fn forward_linear(weights: &[f64], x: &[f64], in_size: usize, out_size: usize) -> Vec<f64> {
    let weight_count = in_size * out_size;
    let mut out = vec![0.0f64; out_size];
    for j in 0..out_size {
        let mut acc = 0.0;
        for i in 0..in_size {
            acc += weights[j * in_size + i] * x[i];
        }
        acc += weights[weight_count + j];
        out[j] = acc;
    }
    out
}

/// Deterministic Fisher–Yates shuffle of `0..n` seeded by `seed`.
///
/// Uses a simple multiplicative LCG (Knuth constants) so that the same `(n, seed)` always
/// produces the same permutation — training runs are reproducible without depending on a
/// crate-level RNG.
fn deterministic_shuffle(n: usize, seed: u64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..n).collect();
    let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
    for i in (1..n).rev() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let j = (state >> 33) as usize % (i + 1);
        order.swap(i, j);
    }
    order
}

impl TrainingBackend {
    pub fn new() -> Self {
        Self {
            backend_id: "training_backend_1".to_string(),
            backend_type: TrainingBackendType::GPU,
            capabilities: TrainingCapabilities::new(),
            current_load: 0.5,
        }
    }
}

impl TrainingCapabilities {
    pub fn new() -> Self {
        Self {
            supported_algorithms: vec![TrainingAlgorithm::Adam, TrainingAlgorithm::SGD],
            max_batch_size: 64,
            max_dataset_size: 100 * 1024 * 1024 * 1024, // 100GB
            parallel_workers: 4,
            memory_limit: 16 * 1024 * 1024 * 1024, // 16GB
        }
    }
}

impl TrainingScheduler {
    pub fn new() -> Self {
        Self {
            scheduling_policy: TrainingSchedulingPolicy::FIFO,
            resource_manager: ResourceManager::new(),
            progress_tracker: ProgressTracker::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.resource_manager.initialize()?;
        self.progress_tracker.initialize()?;
        Ok(())
    }

    /// Return the current training scheduling policy.
    pub fn scheduling_policy(&self) -> &TrainingSchedulingPolicy {
        &self.scheduling_policy
    }

    /// Set the training scheduling policy.
    pub fn set_scheduling_policy(&mut self, policy: TrainingSchedulingPolicy) {
        self.scheduling_policy = policy;
    }
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            allocation_strategy: AllocationStrategy::FirstFit,
            utilization_tracker: UtilizationTracker::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.utilization_tracker.initialize()?;
        Ok(())
    }

    /// Register a resource under its resource id.
    pub fn register_resource(&mut self, resource: Resource) {
        self.resources
            .insert(resource.resource_id.clone(), resource);
    }

    /// Get a registered resource by id.
    pub fn get_resource(&self, resource_id: &str) -> Option<&Resource> {
        self.resources.get(resource_id)
    }

    /// Get a mutable reference to a registered resource by id.
    pub fn get_resource_mut(&mut self, resource_id: &str) -> Option<&mut Resource> {
        self.resources.get_mut(resource_id)
    }

    /// List the ids of all registered resources.
    pub fn list_resources(&self) -> Vec<String> {
        self.resources.keys().cloned().collect()
    }

    /// Return the current allocation strategy.
    pub fn allocation_strategy(&self) -> &AllocationStrategy {
        &self.allocation_strategy
    }

    /// Set the allocation strategy.
    pub fn set_allocation_strategy(&mut self, strategy: AllocationStrategy) {
        self.allocation_strategy = strategy;
    }
}

impl Resource {
    pub fn new() -> Self {
        Self {
            resource_id: "resource_1".to_string(),
            resource_type: ResourceType::GPU,
            capacity: 1.0,
            current_usage: 0.0,
            availability: Availability::Available,
        }
    }
}

impl UtilizationTracker {
    pub fn new() -> Self {
        Self {
            utilization_history: HashMap::new(),
            current_utilization: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Record a utilization sample for a resource, appending it to the history
    /// and updating the current utilization value.
    pub fn record_utilization(&mut self, record: UtilizationRecord) {
        self.utilization_history
            .entry(record.resource_id.clone())
            .or_default()
            .push(record.clone());
        self.current_utilization
            .insert(record.resource_id, record.utilization);
    }

    /// Return the utilization history for a given resource.
    pub fn get_utilization_history(&self, resource_id: &str) -> &[UtilizationRecord] {
        self.utilization_history
            .get(resource_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Return the current utilization for a given resource.
    pub fn current_utilization(&self, resource_id: &str) -> Option<f64> {
        self.current_utilization.get(resource_id).copied()
    }

    /// Set the current utilization for a given resource.
    pub fn set_current_utilization(&mut self, resource_id: &str, utilization: f64) {
        self.current_utilization
            .insert(resource_id.to_string(), utilization);
    }
}

impl UtilizationRecord {
    pub fn new() -> Self {
        Self {
            timestamp: 0,
            resource_id: "resource_1".to_string(),
            utilization: 0.0,
        }
    }
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            training_jobs: HashMap::new(),
            progress_metrics: ProgressMetrics::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Register a training job under its job id.
    pub fn register_job(&mut self, job: TrainingJob) {
        self.training_jobs.insert(job.job_id.clone(), job);
    }

    /// Get a registered training job by id.
    pub fn get_job(&self, job_id: &str) -> Option<&TrainingJob> {
        self.training_jobs.get(job_id)
    }

    /// Get a mutable reference to a registered training job by id.
    pub fn get_job_mut(&mut self, job_id: &str) -> Option<&mut TrainingJob> {
        self.training_jobs.get_mut(job_id)
    }

    /// List the ids of all registered training jobs.
    pub fn list_jobs(&self) -> Vec<String> {
        self.training_jobs.keys().cloned().collect()
    }

    /// Remove a registered training job by id. Returns `true` if removed.
    pub fn remove_job(&mut self, job_id: &str) -> bool {
        self.training_jobs.remove(job_id).is_some()
    }

    /// Return a reference to the progress metrics.
    pub fn progress_metrics(&self) -> &ProgressMetrics {
        &self.progress_metrics
    }

    /// Return a mutable reference to the progress metrics.
    pub fn progress_metrics_mut(&mut self) -> &mut ProgressMetrics {
        &mut self.progress_metrics
    }
}

impl ProgressMetrics {
    pub fn new() -> Self {
        Self {
            total_jobs: 0,
            completed_jobs: 0,
            average_progress: 0.0,
            estimated_completion: 0,
        }
    }
}

impl DataPipeline {
    pub fn new() -> Self {
        Self {
            data_sources: HashMap::new(),
            data_transformers: HashMap::new(),
            data_loaders: HashMap::new(),
            data_augmenters: HashMap::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Register a data source under its source id.
    pub fn register_data_source(&mut self, source: DataSource) {
        self.data_sources.insert(source.source_id.clone(), source);
    }

    /// Get a registered data source by id.
    pub fn get_data_source(&self, source_id: &str) -> Option<&DataSource> {
        self.data_sources.get(source_id)
    }

    /// List the ids of all registered data sources.
    pub fn list_data_sources(&self) -> Vec<String> {
        self.data_sources.keys().cloned().collect()
    }

    /// Register a data transformer under its transformer id.
    pub fn register_transformer(&mut self, transformer: DataTransformer) {
        self.data_transformers
            .insert(transformer.transformer_id.clone(), transformer);
    }

    /// Get a registered data transformer by id.
    pub fn get_transformer(&self, transformer_id: &str) -> Option<&DataTransformer> {
        self.data_transformers.get(transformer_id)
    }

    /// List the ids of all registered data transformers.
    pub fn list_transformers(&self) -> Vec<String> {
        self.data_transformers.keys().cloned().collect()
    }

    /// Register a data loader under its loader id.
    pub fn register_loader(&mut self, loader: DataLoader) {
        self.data_loaders.insert(loader.loader_id.clone(), loader);
    }

    /// Get a registered data loader by id.
    pub fn get_loader(&self, loader_id: &str) -> Option<&DataLoader> {
        self.data_loaders.get(loader_id)
    }

    /// List the ids of all registered data loaders.
    pub fn list_loaders(&self) -> Vec<String> {
        self.data_loaders.keys().cloned().collect()
    }

    /// Register a data augmenter under its augmenter id.
    pub fn register_augmenter(&mut self, augmenter: DataAugmenter) {
        self.data_augmenters
            .insert(augmenter.augmenter_id.clone(), augmenter);
    }

    /// Get a registered data augmenter by id.
    pub fn get_augmenter(&self, augmenter_id: &str) -> Option<&DataAugmenter> {
        self.data_augmenters.get(augmenter_id)
    }

    /// List the ids of all registered data augmenters.
    pub fn list_augmenters(&self) -> Vec<String> {
        self.data_augmenters.keys().cloned().collect()
    }
}

impl DataSource {
    pub fn new() -> Self {
        Self {
            source_id: "source_1".to_string(),
            source_type: DataSourceType::Local,
            location: "/data".to_string(),
            format: DataFormat::CSV,
        }
    }
}

impl DataTransformer {
    pub fn new() -> Self {
        Self {
            transformer_id: "transformer_1".to_string(),
            transformer_type: DataTransformerType::Normalizer,
            transformation_pipeline: Vec::new(),
        }
    }
}

impl TransformationStep {
    pub fn new() -> Self {
        Self {
            step_id: "step_1".to_string(),
            step_type: ConversionStepType::Parsing,
            parameters: HashMap::new(),
        }
    }
}

impl DataLoader {
    pub fn new() -> Self {
        Self {
            loader_id: "loader_1".to_string(),
            loader_type: DataLoaderType::Parallel,
            batch_size: 32,
            shuffle: true,
            num_workers: 4,
        }
    }
}

impl DataAugmenter {
    pub fn new() -> Self {
        Self {
            augmenter_id: "augmenter_1".to_string(),
            augmenter_type: DataAugmenterType::ImageAugmentation,
            augmentation_pipeline: Vec::new(),
        }
    }
}

impl AugmentationStep {
    pub fn new() -> Self {
        Self {
            step_id: "step_1".to_string(),
            step_type: AugmentationStepType::Rotation,
            parameters: HashMap::new(),
        }
    }
}

impl TrainingOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_algorithms: HashMap::new(),
            hyperparameter_tuner: HyperparameterTuner::new(),
            early_stopping: EarlyStopping::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        self.hyperparameter_tuner.initialize()?;
        Ok(())
    }

    /// Register a training optimization algorithm under the given name.
    pub fn register_algorithm(&mut self, name: &str, algorithm: TrainingOptimizationAlgorithm) {
        self.optimization_algorithms
            .insert(name.to_string(), algorithm);
    }

    /// Get a registered training optimization algorithm by name.
    pub fn get_algorithm(&self, name: &str) -> Option<&TrainingOptimizationAlgorithm> {
        self.optimization_algorithms.get(name)
    }

    /// List the names of all registered training optimization algorithms.
    pub fn list_algorithms(&self) -> Vec<String> {
        self.optimization_algorithms.keys().cloned().collect()
    }

    /// Return a reference to the early-stopping configuration.
    pub fn early_stopping(&self) -> &EarlyStopping {
        &self.early_stopping
    }

    /// Return a mutable reference to the early-stopping configuration.
    pub fn early_stopping_mut(&mut self) -> &mut EarlyStopping {
        &mut self.early_stopping
    }
}

impl HyperparameterTuner {
    pub fn new() -> Self {
        Self {
            tuning_space: TuningSpace::new(),
            tuning_algorithm: TuningAlgorithm::BayesianOptimization,
            tuning_history: TuningHistory::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MLError> {
        Ok(())
    }

    /// Return a reference to the tuning space.
    pub fn tuning_space(&self) -> &TuningSpace {
        &self.tuning_space
    }

    /// Return a mutable reference to the tuning space.
    pub fn tuning_space_mut(&mut self) -> &mut TuningSpace {
        &mut self.tuning_space
    }

    /// Return the configured tuning algorithm.
    pub fn tuning_algorithm(&self) -> &TuningAlgorithm {
        &self.tuning_algorithm
    }

    /// Set the tuning algorithm.
    pub fn set_tuning_algorithm(&mut self, algorithm: TuningAlgorithm) {
        self.tuning_algorithm = algorithm;
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

impl TuningSpace {
    pub fn new() -> Self {
        Self {
            hyperparameters: Vec::new(),
            constraints: Vec::new(),
        }
    }
}

impl Hyperparameter {
    pub fn new() -> Self {
        Self {
            name: "learning_rate".to_string(),
            parameter_type: HyperparameterType::Continuous,
            range: HyperparameterRange::new(),
            default_value: 0.001,
        }
    }
}

impl HyperparameterRange {
    pub fn new() -> Self {
        Self {
            min_value: 0.0001,
            max_value: 1.0,
            step: Some(0.0001),
            categories: None,
        }
    }
}

impl HyperparameterConstraint {
    pub fn new() -> Self {
        Self {
            constraint_id: "constraint_1".to_string(),
            constraint_type: ConstraintType::Range,
            parameters: vec!["learning_rate".to_string()],
            condition: "learning_rate > 0".to_string(),
        }
    }
}

impl EarlyStopping {
    pub fn new() -> Self {
        Self {
            stopping_criteria: StoppingCriteria::new(),
            patience: 10,
            min_delta: 0.001,
            restore_best_weights: true,
        }
    }

    /// Return a reference to the stopping criteria.
    pub fn stopping_criteria(&self) -> &StoppingCriteria {
        &self.stopping_criteria
    }

    /// Return a mutable reference to the stopping criteria.
    pub fn stopping_criteria_mut(&mut self) -> &mut StoppingCriteria {
        &mut self.stopping_criteria
    }

    /// Return the configured patience (number of epochs without improvement).
    pub fn patience(&self) -> u32 {
        self.patience
    }

    /// Set the patience.
    pub fn set_patience(&mut self, patience: u32) {
        self.patience = patience;
    }

    /// Return the minimum delta required to count as an improvement.
    pub fn min_delta(&self) -> f64 {
        self.min_delta
    }

    /// Set the minimum delta.
    pub fn set_min_delta(&mut self, min_delta: f64) {
        self.min_delta = min_delta;
    }

    /// Return whether best weights should be restored after early stopping.
    pub fn restore_best_weights(&self) -> bool {
        self.restore_best_weights
    }

    /// Set whether to restore best weights.
    pub fn set_restore_best_weights(&mut self, restore: bool) {
        self.restore_best_weights = restore;
    }
}

impl StoppingCriteria {
    pub fn new() -> Self {
        Self {
            metric: "val_loss".to_string(),
            mode: StoppingMode::Min,
            min_delta: 0.001,
            patience: 10,
        }
    }
}
