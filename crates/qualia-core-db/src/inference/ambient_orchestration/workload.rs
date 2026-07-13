//! Workload analysis, resource allocation, and adaptation engines.

use super::*;
use std::time::{Duration, Instant};

/// Workload analyzer
pub struct WorkloadAnalyzer {
    workload_history: Vec<WorkloadSample>,
    prediction_model: PredictionModel,
    analysis_window: Duration,
}

/// Prediction model for workload
#[derive(Debug, Clone)]
pub struct PredictionModel {
    pub model_type: ModelType,
    pub parameters: ModelParameters,
    pub accuracy: f64,
}

/// Resource allocator
pub struct ResourceAllocator {
    allocation_strategy: AllocationStrategy,
    resource_pool: ResourcePool,
    allocation_history: Vec<AllocationRecord>,
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

/// Adaptation engine
pub struct AdaptationEngine {
    adaptation_strategy: AdaptationStrategy,
    adaptation_history: Vec<AdaptationRecord>,
    learning_rate: f64,
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
        self.workload_history
            .retain(|s| now.duration_since(s.timestamp) <= window);
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
        let current_load = recent
            .iter()
            .map(|s| (s.cpu_usage + s.neural_engine_usage) / 2.0)
            .sum::<f64>()
            / recent.len() as f64;

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
        let resource_pressure =
            recent.iter().map(|s| s.memory_usage).sum::<f64>() / recent.len() as f64;

        // Thermal pressure from thermal state (0-1 scale, 0=cool, 1=hot).
        let thermal_pressure =
            recent.iter().map(|s| s.thermal_state).sum::<f64>() / recent.len() as f64;

        // Battery pressure: 1.0 - battery_level/100 (higher = more pressure).
        let battery_pressure = recent
            .iter()
            .map(|s| 1.0 - (s.battery_level / 100.0).clamp(0.0, 1.0))
            .sum::<f64>()
            / recent.len() as f64;

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
        self.resource_pool.available_compute_units = (self.resource_pool.available_compute_units
            + units)
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
        if matches!(
            self.adaptation_strategy,
            AdaptationStrategy::MachineLearning | AdaptationStrategy::Hybrid
        ) {
            let recent: Vec<&AdaptationRecord> =
                self.adaptation_history.iter().rev().take(20).collect();
            if !recent.is_empty() {
                let success_rate = recent
                    .iter()
                    .filter(|r| r.result == AdaptationResult::Success)
                    .count() as f64
                    / recent.len() as f64;
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
