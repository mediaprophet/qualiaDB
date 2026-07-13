//! Ambient performance monitoring and global metric aggregation.

use super::*;
use std::collections::HashMap;
use std::time::Duration;

/// Ambient performance monitor
pub struct AmbientPerformanceMonitor {
    device_metrics: HashMap<String, DeviceMetrics>,
    task_metrics: HashMap<String, TaskMetrics>,
    global_metrics: AmbientGlobalMetrics,
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
            let sum: f64 = self
                .task_metrics
                .values()
                .map(|m| m.execution_time.as_secs_f64())
                .sum();
            Duration::from_secs_f64(sum / self.task_metrics.len() as f64)
        } else {
            self.global_metrics.average_execution_time
        };

        let avg_efficiency = if !self.task_metrics.is_empty() {
            self.task_metrics
                .values()
                .map(|m| m.resource_efficiency)
                .sum::<f64>()
                / self.task_metrics.len() as f64
        } else {
            self.global_metrics.overall_efficiency
        };

        let avg_success_rate = if !self.task_metrics.is_empty() {
            self.task_metrics
                .values()
                .map(|m| m.success_rate)
                .sum::<f64>()
                / self.task_metrics.len() as f64
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
