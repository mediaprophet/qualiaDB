use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::computation::*;
use super::core_types::*;
use super::optimization::*;
use super::privacy::*;
use super::storage::*;

/// Performance monitor for linear algebra operations
pub struct LAPerformanceMonitor {
    pub operation_metrics: HashMap<String, OperationMetrics>,
    pub matrix_metrics: HashMap<String, MatrixMetrics>,
    pub system_metrics: SystemMetrics,
}

/// Operation metrics
#[derive(Debug, Clone)]
pub struct OperationMetrics {
    pub operation_id: String,
    pub operation_type: MatrixOperation,
    pub execution_time: u64,
    pub memory_usage: u64,
    pub cache_hit_rate: f64,
    pub parallel_efficiency: f64,
    pub simd_efficiency: f64,
    /// Timestamp (nanoseconds since UNIX_EPOCH) of the last recorded operation of this type
    pub timestamp: u64,
    /// Number of times this operation type has been recorded
    pub count: u64,
    /// Matrix size (rows, cols) of the last recorded operation
    pub matrix_size: (usize, usize),
}

/// Matrix metrics
#[derive(Debug, Clone)]
pub struct MatrixMetrics {
    pub matrix_id: String,
    pub access_count: u64,
    pub total_access_time: u64,
    pub average_access_time: f64,
    pub cache_hit_rate: f64,
    pub compression_ratio: f64,
    /// Timestamp (nanoseconds since UNIX_EPOCH) of the last access
    pub last_access_time: u64,
    /// Number of cache hits for this matrix
    pub cache_hits: u64,
    /// Number of cache misses for this matrix
    pub cache_misses: u64,
    /// Last operation performed on this matrix
    pub last_operation: String,
}

/// System metrics
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub total_operations: u64,
    pub average_execution_time: f64,
    pub throughput: f64,
    pub memory_utilization: f64,
    pub compute_utilization: f64,
    pub power_efficiency: f64,
    /// Total memory usage across all recorded operations (bytes)
    pub total_memory_usage: u64,
    /// Timestamp (nanoseconds since UNIX_EPOCH) when monitoring started
    pub start_time: u64,
}

impl LAPerformanceMonitor {
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        Self {
            operation_metrics: HashMap::new(),
            matrix_metrics: HashMap::new(),
            system_metrics: SystemMetrics {
                total_operations: 0,
                average_execution_time: 0.0,
                throughput: 0.0,
                memory_utilization: 0.0,
                compute_utilization: 0.0,
                power_efficiency: 0.0,
                total_memory_usage: 0,
                start_time: now,
            },
        }
    }

    /// Get the current time as nanoseconds since UNIX_EPOCH
    fn current_time() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    /// Record an operation with timing and matrix size data.
    /// Creates/updates an OperationMetrics entry keyed by operation name,
    /// and updates total_operations and average_execution_time (running average).
    pub fn record_operation(
        &mut self,
        operation_type: &str,
        execution_time: u64,
        memory_usage: u64,
    ) {
        // Update system-level totals
        self.system_metrics.total_operations += 1;
        self.system_metrics.average_execution_time = (self.system_metrics.average_execution_time
            * (self.system_metrics.total_operations - 1) as f64
            + execution_time as f64)
            / self.system_metrics.total_operations as f64;
        self.system_metrics.total_memory_usage += memory_usage;

        // Update per-operation metrics
        let now = Self::current_time();
        let metrics = self
            .operation_metrics
            .entry(operation_type.to_string())
            .or_insert_with(|| OperationMetrics {
                operation_id: operation_type.to_string(),
                operation_type: MatrixOperation::MatrixMultiply {
                    left: String::new(),
                    right: String::new(),
                    result: String::new(),
                    alpha: 0.0,
                    beta: 0.0,
                },
                execution_time: 0,
                memory_usage: 0,
                cache_hit_rate: 0.0,
                parallel_efficiency: 0.0,
                simd_efficiency: 0.0,
                timestamp: now,
                count: 0,
                matrix_size: (0, 0),
            });

        metrics.count += 1;
        metrics.execution_time = execution_time;
        metrics.memory_usage = memory_usage;
        metrics.timestamp = now;
    }

    /// Record an operation with full detail including matrix size.
    /// Creates/updates an OperationMetrics entry with the operation type,
    /// execution time, estimated memory (rows*cols*8), and timestamp.
    pub fn record_operation_detailed(
        &mut self,
        operation: &str,
        execution_time_ms: f64,
        matrix_size: (usize, usize),
    ) {
        let memory_usage = (matrix_size.0 * matrix_size.1 * 8) as u64;
        self.record_operation(operation, execution_time_ms as u64, memory_usage);

        // Update the matrix_size on the entry
        if let Some(metrics) = self.operation_metrics.get_mut(operation) {
            metrics.matrix_size = matrix_size;
        }
    }

    /// Record a matrix access, tracking access count, last access time,
    /// and cache hit rate for each matrix.
    pub fn record_matrix_access(
        &mut self,
        matrix_id: &str,
        operation: &str,
        cache_hit: bool,
    ) {
        let now = Self::current_time();
        let metrics = self
            .matrix_metrics
            .entry(matrix_id.to_string())
            .or_insert_with(|| MatrixMetrics {
                matrix_id: matrix_id.to_string(),
                access_count: 0,
                total_access_time: 0,
                average_access_time: 0.0,
                cache_hit_rate: 0.0,
                compression_ratio: 1.0,
                last_access_time: now,
                cache_hits: 0,
                cache_misses: 0,
                last_operation: String::new(),
            });

        metrics.access_count += 1;
        metrics.last_access_time = now;
        metrics.last_operation = operation.to_string();
        if cache_hit {
            metrics.cache_hits += 1;
        } else {
            metrics.cache_misses += 1;
        }
        let total = metrics.cache_hits + metrics.cache_misses;
        if total > 0 {
            metrics.cache_hit_rate = metrics.cache_hits as f64 / total as f64;
        }
    }

    /// Get the operation metrics for a given operation name
    pub fn get_operation_metrics(&self, operation: &str) -> Option<&OperationMetrics> {
        self.operation_metrics.get(operation)
    }

    /// Get the matrix metrics for a given matrix ID
    pub fn get_matrix_metrics(&self, matrix_id: &str) -> Option<&MatrixMetrics> {
        self.matrix_metrics.get(matrix_id)
    }

    /// Compute and return the current system metrics, including throughput
    /// (ops/sec from total_operations and elapsed time) and memory utilization.
    pub fn system_metrics(&self) -> &SystemMetrics {
        // Note: we can't mutate self here since we return &SystemMetrics.
        // The throughput and memory_utilization are computed on-demand
        // in get_system_metrics() which returns a clone.
        &self.system_metrics
    }

    /// Get a cloned snapshot of system metrics with computed throughput and memory utilization
    pub fn get_system_metrics(&self) -> SystemMetrics {
        let now = Self::current_time();
        let elapsed_secs = if now > self.system_metrics.start_time {
            (now - self.system_metrics.start_time) as f64 / 1_000_000_000.0
        } else {
            1.0 // avoid division by zero
        };
        let throughput = if elapsed_secs > 0.0 {
            self.system_metrics.total_operations as f64 / elapsed_secs
        } else {
            0.0
        };

        // Memory utilization: sum of all matrix sizes from operation metrics
        let total_memory: u64 = self
            .operation_metrics
            .values()
            .map(|m| m.memory_usage)
            .sum();

        let mut metrics = self.system_metrics.clone();
        metrics.throughput = throughput;
        metrics.memory_utilization = total_memory as f64;
        metrics
    }

    /// Human-readable summary of all metrics
    pub fn summary(&self) -> String {
        let sys = self.get_system_metrics();
        let mut lines = Vec::new();

        lines.push(format!("=== Linear Algebra Performance Summary ==="));
        lines.push(format!("Total operations: {}", sys.total_operations));
        lines.push(format!(
            "Average execution time: {:.3} ms",
            sys.average_execution_time
        ));
        lines.push(format!("Throughput: {:.2} ops/sec", sys.throughput));
        lines.push(format!(
            "Memory utilization: {:.0} bytes",
            sys.memory_utilization
        ));

        lines.push(format!("\n--- Operation Metrics ({} types) ---", self.operation_metrics.len()));
        for (name, m) in &self.operation_metrics {
            lines.push(format!(
                "  {}: count={}, last_time={}ms, memory={}bytes, size={}x{}",
                name, m.count, m.execution_time, m.memory_usage, m.matrix_size.0, m.matrix_size.1
            ));
        }

        lines.push(format!("\n--- Matrix Metrics ({} matrices) ---", self.matrix_metrics.len()));
        for (id, m) in &self.matrix_metrics {
            lines.push(format!(
                "  {}: accesses={}, cache_hit_rate={:.2}%, last_op={}",
                id, m.access_count, m.cache_hit_rate * 100.0, m.last_operation
            ));
        }

        lines.push("=== End Summary ===".to_string());
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_operation_updates_system_metrics() {
        let mut monitor = LAPerformanceMonitor::new();

        monitor.record_operation("matrix_multiply", 10, 100);
        assert_eq!(monitor.system_metrics.total_operations, 1);
        assert!((monitor.system_metrics.average_execution_time - 10.0).abs() < 1e-10);

        monitor.record_operation("matrix_multiply", 20, 200);
        assert_eq!(monitor.system_metrics.total_operations, 2);
        assert!((monitor.system_metrics.average_execution_time - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_record_operation_populates_operation_metrics() {
        let mut monitor = LAPerformanceMonitor::new();

        monitor.record_operation("matrix_multiply", 15, 128);

        let metrics = monitor.get_operation_metrics("matrix_multiply");
        assert!(metrics.is_some());
        let m = metrics.unwrap();
        assert_eq!(m.execution_time, 15);
        assert_eq!(m.memory_usage, 128);
        assert_eq!(m.count, 1);
    }

    #[test]
    fn test_record_operation_detailed() {
        let mut monitor = LAPerformanceMonitor::new();

        monitor.record_operation_detailed("matrix_inverse", 5.0, (3, 3));

        let metrics = monitor.get_operation_metrics("matrix_inverse");
        assert!(metrics.is_some());
        let m = metrics.unwrap();
        assert_eq!(m.matrix_size, (3, 3));
        assert_eq!(m.memory_usage, 72); // 3*3*8 = 72
    }

    #[test]
    fn test_record_matrix_access() {
        let mut monitor = LAPerformanceMonitor::new();

        monitor.record_matrix_access("m1", "matrix_multiply", true);
        monitor.record_matrix_access("m1", "matrix_multiply", true);
        monitor.record_matrix_access("m1", "matrix_multiply", false);

        let metrics = monitor.get_matrix_metrics("m1");
        assert!(metrics.is_some());
        let m = metrics.unwrap();
        assert_eq!(m.access_count, 3);
        assert_eq!(m.cache_hits, 2);
        assert_eq!(m.cache_misses, 1);
        assert!((m.cache_hit_rate - 2.0 / 3.0).abs() < 1e-10);
        assert_eq!(m.last_operation, "matrix_multiply");
    }

    #[test]
    fn test_get_operation_metrics_none() {
        let monitor = LAPerformanceMonitor::new();
        assert!(monitor.get_operation_metrics("nonexistent").is_none());
    }

    #[test]
    fn test_get_matrix_metrics_none() {
        let monitor = LAPerformanceMonitor::new();
        assert!(monitor.get_matrix_metrics("nonexistent").is_none());
    }

    #[test]
    fn test_system_metrics_throughput() {
        let mut monitor = LAPerformanceMonitor::new();

        // Record some operations
        monitor.record_operation("op1", 10, 100);
        monitor.record_operation("op2", 20, 200);

        let metrics = monitor.get_system_metrics();
        assert_eq!(metrics.total_operations, 2);
        assert!(metrics.throughput > 0.0); // Should have positive throughput
    }

    #[test]
    fn test_summary_contains_key_info() {
        let mut monitor = LAPerformanceMonitor::new();
        monitor.record_operation("matrix_multiply", 10, 100);
        monitor.record_matrix_access("m1", "matrix_multiply", true);

        let summary = monitor.summary();
        assert!(summary.contains("Linear Algebra Performance Summary"));
        assert!(summary.contains("Total operations: 1"));
        assert!(summary.contains("matrix_multiply"));
        assert!(summary.contains("m1"));
        assert!(summary.contains("End Summary"));
    }

    #[test]
    fn test_multiple_operation_types() {
        let mut monitor = LAPerformanceMonitor::new();

        monitor.record_operation("matrix_multiply", 10, 100);
        monitor.record_operation("matrix_transpose", 5, 50);
        monitor.record_operation("matrix_inverse", 20, 200);

        assert_eq!(monitor.system_metrics.total_operations, 3);
        assert!(monitor.get_operation_metrics("matrix_multiply").is_some());
        assert!(monitor.get_operation_metrics("matrix_transpose").is_some());
        assert!(monitor.get_operation_metrics("matrix_inverse").is_some());
    }
}
