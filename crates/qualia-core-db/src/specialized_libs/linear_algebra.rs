//! Linear Algebra Library - High-Performance Mathematical Computing
//! 
//! This module provides high-performance linear algebra operations leveraging Phase 2 enhancements:
//! - Hardware-Sympathetic Storage (ZNS) for zero-copy matrix operations
//! - NVMe Computational Storage (CSD) for hardware-accelerated computations
//! - Zero-Knowledge Semantic Proofs for privacy-preserving linear algebra
//! - Ambient Sub-Threshold Orchestration for mobile optimization
pub mod core_types;
pub mod storage;
pub mod computation;
pub mod optimization;
pub mod privacy;
pub mod performance;

pub use core_types::*;
pub use storage::*;
pub use computation::*;
pub use optimization::*;
pub use privacy::*;
pub use performance::*;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::ops::{Add, Mul, Sub};
use serde::{Deserialize, Serialize};
use crate::solvers::SolversError;



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_algebra_library_creation() {
        let library = LinearAlgebraLibrary::new();
        assert_eq!(library.list_matrices().len(), 0);
    }

    #[test]
    fn test_matrix_creation() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();
        
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let matrix = library.create_matrix("test_matrix".to_string(), 2, 2, DataType::Float64, data).unwrap();
        
        assert_eq!(matrix.rows, 2);
        assert_eq!(matrix.cols, 2);
        assert_eq!(matrix.data.len(), 4);
    }

    #[test]
    fn test_matrix_multiplication() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();
        
        let a_data = vec![1.0, 2.0, 3.0, 4.0];
        let b_data = vec![5.0, 6.0, 7.0, 8.0];
        
        library.create_matrix("A".to_string(), 2, 2, DataType::Float64, a_data).unwrap();
        library.create_matrix("B".to_string(), 2, 2, DataType::Float64, b_data).unwrap();
        
        let result = library.matrix_multiply("A", "B", "C", 1.0, 0.0).unwrap();
        
        assert_eq!(result.result.rows, 2);
        assert_eq!(result.result.cols, 2);
        assert_eq!(result.result.data[0], 19.0); // 1*5 + 2*7
        assert_eq!(result.result.data[1], 22.0); // 1*6 + 2*8
        assert_eq!(result.result.data[2], 43.0); // 3*5 + 4*7
        assert_eq!(result.result.data[3], 50.0); // 3*6 + 4*8
    }

    #[test]
    fn test_matrix_transpose() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();
        
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        library.create_matrix("A".to_string(), 2, 3, DataType::Float64, data).unwrap();
        
        let result = library.matrix_transpose("A", "AT").unwrap();
        
        assert_eq!(result.result.rows, 3);
        assert_eq!(result.result.cols, 2);
        assert_eq!(result.result.data[0], 1.0);
        assert_eq!(result.result.data[1], 4.0);
        assert_eq!(result.result.data[2], 2.0);
        assert_eq!(result.result.data[3], 5.0);
        assert_eq!(result.result.data[4], 3.0);
        assert_eq!(result.result.data[5], 6.0);
    }

    #[test]
    fn test_matrix_inverse() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();
        
        let data = vec![2.0, 1.0, 1.0, 1.0]; // [[2,1],[1,1]]
        library.create_matrix("A".to_string(), 2, 2, DataType::Float64, data).unwrap();
        
        let result = library.matrix_inverse("A", "A_inv").unwrap();
        
        assert_eq!(result.result.rows, 2);
        assert_eq!(result.result.cols, 2);
        // Inverse of [[2,1],[1,1]] is [[1,-1],[-1,2]]
        assert!((result.result.data[0] - 1.0).abs() < 1e-10);
        assert!((result.result.data[1] + 1.0).abs() < 1e-10);
        assert!((result.result.data[2] + 1.0).abs() < 1e-10);
        assert!((result.result.data[3] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_solve_linear_system() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();
        
        let matrix_data = vec![2.0, 1.0, 1.0, 1.0]; // [[2,1],[1,1]]
        let rhs_data = vec![3.0, 2.0]; // [3,2]
        
        library.create_matrix("A".to_string(), 2, 2, DataType::Float64, matrix_data).unwrap();
        library.create_matrix("b".to_string(), 2, 1, DataType::Float64, rhs_data).unwrap();
        
        let result = library.solve_linear_system("A", "b", "x").unwrap();
        
        assert_eq!(result.result.rows, 2);
        assert_eq!(result.result.cols, 1);
        // Solution should be [1,1] for 2x + y = 3, x + y = 2
        assert!((result.result.data[0] - 1.0).abs() < 1e-10);
        assert!((result.result.data[1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_private_matrix_multiplication() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();
        
        let a_data = vec![1.0, 2.0, 3.0, 4.0];
        let b_data = vec![5.0, 6.0, 7.0, 8.0];
        
        library.create_matrix("A".to_string(), 2, 2, DataType::Float64, a_data).unwrap();
        library.create_matrix("B".to_string(), 2, 2, DataType::Float64, b_data).unwrap();
        
        let result = library.private_matrix_multiply("A", "B", "C").unwrap();

        assert!(result.privacy_preserved, "the Groth16 proof of A·B = C must verify");
        assert_eq!(result.result.rows, 2);
        assert_eq!(result.result.cols, 2);
        // The returned matrix is exactly what the ZK circuit attested: A·B.
        // [[1,2],[3,4]] · [[5,6],[7,8]] = [[19,22],[43,50]].
        assert_eq!(result.result.data, vec![19.0, 22.0, 43.0, 50.0]);
    }

    #[test]
    fn test_private_matrix_multiplication_rectangular() {
        // Non-square, with a negative entry, to exercise general dimensions and the
        // signed field encoding. A is 2x3, B is 3x2.
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();
        library.create_matrix("A".to_string(), 2, 3, DataType::Float64,
            vec![1.0, 2.0, 3.0, 4.0, -5.0, 6.0]).unwrap();
        library.create_matrix("B".to_string(), 3, 2, DataType::Float64,
            vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0]).unwrap();

        let result = library.private_matrix_multiply("A", "B", "C").unwrap();

        assert!(result.privacy_preserved);
        // Row0: [1·7+2·9+3·11, 1·8+2·10+3·12] = [58, 64]
        // Row1: [4·7-5·9+6·11, 4·8-5·10+6·12] = [49, 54]
        assert_eq!(result.result.data, vec![58.0, 64.0, 49.0, 54.0]);
    }
}

/// Linear Algebra Library Manager
pub struct LinearAlgebraLibrary {
    pub matrix_storage: MatrixStorage,
    pub computation_engine: ComputationEngine,
    pub optimization_engine: OptimizationEngine,
    pub privacy_engine: PrivacyEngine,
    pub performance_monitor: LAPerformanceMonitor,
}


impl LinearAlgebraLibrary {
    /// Create new linear algebra library
    pub fn new() -> Self {
        Self {
            matrix_storage: MatrixStorage::new(),
            computation_engine: ComputationEngine::new(),
            optimization_engine: OptimizationEngine::new(),
            privacy_engine: PrivacyEngine::new(),
            performance_monitor: LAPerformanceMonitor::new(),
        }
    }

    /// Initialize the library
    pub fn initialize(&mut self) -> Result<(), LinearAlgebraError> {
        // Initialize storage
        self.matrix_storage.initialize()?;

        // Initialize computation engine
        self.computation_engine.initialize()?;

        // Initialize optimization engine
        self.optimization_engine.initialize()?;

        // Initialize privacy engine
        self.privacy_engine.initialize()?;

        Ok(())
    }

    /// Create a new matrix
    pub fn create_matrix(&mut self, matrix_id: String, rows: usize, cols: usize, data_type: DataType, data: Vec<f64>) -> Result<Matrix, LinearAlgebraError> {
        // Validate input
        if data.len() != rows * cols {
            return Err(LinearAlgebraError::InvalidDimensions("Data size doesn't match dimensions".to_string()));
        }

        // Create matrix metadata
        let metadata = MatrixMetadata {
            matrix_id: matrix_id.clone(),
            rows,
            cols,
            data_type: data_type.clone(),
            storage_format: StorageFormat::RowMajor,
            compression: CompressionType::None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_accessed: 0,
            access_count: 0,
        };

        // Store matrix
        let matrix = Matrix {
            matrix_id: matrix_id.clone(),
            rows,
            cols,
            data_type,
            data,
            storage_format: StorageFormat::RowMajor,
            metadata,
        };

        self.matrix_storage.store_matrix(matrix.clone())?;

        Ok(matrix)
    }

    /// Matrix multiplication with hardware acceleration
    pub fn matrix_multiply(&mut self, left_id: &str, right_id: &str, result_id: &str, alpha: f64, beta: f64) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        // Get matrices
        let left = self.matrix_storage.get_matrix(left_id)?;
        let right = self.matrix_storage.get_matrix(right_id)?;

        // Validate dimensions
        if left.cols != right.rows {
            return Err(LinearAlgebraError::InvalidDimensions("Matrix dimensions incompatible for multiplication".to_string()));
        }

        // Optimize operation
        let optimized_operation = self.optimization_engine.optimize_multiplication(&left, &right)?;

        // Execute multiplication
        let result_data = self.computation_engine.execute_multiplication(&optimized_operation, alpha, beta)?;

        // Create result matrix
        let result = self.create_matrix(
            result_id.to_string(),
            left.rows,
            right.cols,
            left.data_type,
            result_data,
        )?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Update performance metrics
        self.performance_monitor.record_operation("matrix_multiply", execution_time, 0);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage: 0,
            operations_used: vec!["matrix_multiply".to_string()],
            privacy_preserved: false,
        })
    }

    /// Matrix addition
    pub fn matrix_add(&mut self, left_id: &str, right_id: &str, result_id: &str, alpha: f64) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        // Get matrices
        let left = self.matrix_storage.get_matrix(left_id)?;
        let right = self.matrix_storage.get_matrix(right_id)?;

        // Validate dimensions
        if left.rows != right.rows || left.cols != right.cols {
            return Err(LinearAlgebraError::InvalidDimensions("Matrix dimensions incompatible for addition".to_string()));
        }

        // Execute addition
        let mut result_data = Vec::with_capacity(left.data.len());
        for i in 0..left.data.len() {
            result_data.push(alpha * (left.data[i] + right.data[i]));
        }

        // Create result matrix
        let result = self.create_matrix(
            result_id.to_string(),
            left.rows,
            left.cols,
            left.data_type,
            result_data,
        )?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Update performance metrics
        self.performance_monitor.record_operation("matrix_add", execution_time, 0);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage: 0,
            operations_used: vec!["matrix_add".to_string()],
            privacy_preserved: false,
        })
    }

    /// Matrix transpose
    pub fn matrix_transpose(&mut self, input_id: &str, result_id: &str) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        // Get matrix
        let input = self.matrix_storage.get_matrix(input_id)?;

        // Execute transpose
        let mut result_data = Vec::with_capacity(input.data.len());
        for j in 0..input.cols {
            for i in 0..input.rows {
                result_data.push(input.data[i * input.cols + j]);
            }
        }

        // Create result matrix
        let result = self.create_matrix(
            result_id.to_string(),
            input.cols,
            input.rows,
            input.data_type,
            result_data,
        )?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Update performance metrics
        self.performance_monitor.record_operation("matrix_transpose", execution_time, 0);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage: 0,
            operations_used: vec!["matrix_transpose".to_string()],
            privacy_preserved: false,
        })
    }

    /// Matrix inverse
    pub fn matrix_inverse(&mut self, input_id: &str, result_id: &str) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        // Get matrix
        let input = self.matrix_storage.get_matrix(input_id)?;

        // Validate square matrix
        if input.rows != input.cols {
            return Err(LinearAlgebraError::InvalidDimensions("Matrix must be square for inversion".to_string()));
        }

        // Execute inverse (simplified Gaussian elimination)
        let n = input.rows;
        let mut augmented = Vec::with_capacity(n * 2 * n);
        
        // Create augmented matrix [A|I]
        for i in 0..n {
            for j in 0..n {
                augmented.push(input.data[i * n + j]);
            }
            for j in 0..n {
                augmented.push(if i == j { 1.0 } else { 0.0 });
            }
        }

        // Gaussian elimination (simplified)
        for i in 0..n {
            // Find pivot
            let mut pivot_row = i;
            for k in (i + 1)..n {
                if (augmented[k * 2 * n + i]).abs() > (augmented[pivot_row * 2 * n + i]).abs() {
                    pivot_row = k;
                }
            }

            // Swap rows
            for j in 0..(2 * n) {
                augmented.swap(i * 2 * n + j, pivot_row * 2 * n + j);
            }

            // Eliminate column
            let pivot = augmented[i * 2 * n + i];
            if pivot.abs() < 1e-10 {
                return Err(LinearAlgebraError::SingularMatrix("Matrix is singular".to_string()));
            }

            for j in 0..(2 * n) {
                augmented[i * 2 * n + j] /= pivot;
            }

            for k in 0..n {
                if k != i {
                    let factor = augmented[k * 2 * n + i];
                    for j in 0..(2 * n) {
                        augmented[k * 2 * n + j] -= factor * augmented[i * 2 * n + j];
                    }
                }
            }
        }

        // Extract inverse
        let mut result_data = Vec::with_capacity(n * n);
        for i in 0..n {
            for j in 0..n {
                result_data.push(augmented[i * 2 * n + n + j]);
            }
        }

        // Create result matrix
        let result = self.create_matrix(
            result_id.to_string(),
            n,
            n,
            input.data_type,
            result_data,
        )?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Update performance metrics
        self.performance_monitor.record_operation("matrix_inverse", execution_time, 0);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage: 0,
            operations_used: vec!["matrix_inverse".to_string()],
            privacy_preserved: false,
        })
    }

    /// Solve linear system Ax = b
    pub fn solve_linear_system(&mut self, matrix_id: &str, rhs_id: &str, solution_id: &str) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        // Get matrices
        let matrix = self.matrix_storage.get_matrix(matrix_id)?;
        let rhs = self.matrix_storage.get_matrix(rhs_id)?;

        // Validate dimensions
        if matrix.rows != matrix.cols {
            return Err(LinearAlgebraError::InvalidDimensions("Matrix must be square".to_string()));
        }
        if matrix.rows != rhs.rows {
            return Err(LinearAlgebraError::InvalidDimensions("Matrix and RHS dimensions incompatible".to_string()));
        }

        // Solve using LU decomposition (simplified)
        let n = matrix.rows;
        let mut solution_data = Vec::with_capacity(n);

        // For demonstration, use simple Gaussian elimination
        let mut augmented = Vec::with_capacity(n * (n + 1));
        for i in 0..n {
            for j in 0..n {
                augmented.push(matrix.data[i * n + j]);
            }
            augmented.push(rhs.data[i]);
        }

        // Gaussian elimination
        for i in 0..n {
            // Find pivot
            let mut pivot_row = i;
            for k in (i + 1)..n {
                if (augmented[k * (n + 1) + i]).abs() > (augmented[pivot_row * (n + 1) + i]).abs() {
                    pivot_row = k;
                }
            }

            // Swap rows
            for j in 0..(n + 1) {
                augmented.swap(i * (n + 1) + j, pivot_row * (n + 1) + j);
            }

            // Eliminate column
            let pivot = augmented[i * (n + 1) + i];
            if pivot.abs() < 1e-10 {
                return Err(LinearAlgebraError::SingularMatrix("System is singular".to_string()));
            }

            for j in i..(n + 1) {
                augmented[i * (n + 1) + j] /= pivot;
            }

            for k in 0..n {
                if k != i {
                    let factor = augmented[k * (n + 1) + i];
                    for j in i..(n + 1) {
                        augmented[k * (n + 1) + j] -= factor * augmented[i * (n + 1) + j];
                    }
                }
            }
        }

        // Extract solution
        for i in 0..n {
            solution_data.push(augmented[i * (n + 1) + n]);
        }

        // Create result matrix
        let result = self.create_matrix(
            solution_id.to_string(),
            n,
            1,
            matrix.data_type,
            solution_data,
        )?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Update performance metrics
        self.performance_monitor.record_operation("solve_linear_system", execution_time, 0);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage: 0,
            operations_used: vec!["solve_linear_system".to_string()],
            privacy_preserved: false,
        })
    }

    /// Privacy-preserving matrix multiplication
    /// Multiply two matrices and produce a zero-knowledge proof that the published
    /// result really is `A·B`, WITHOUT revealing `A` or `B`.
    ///
    /// The proof is over a real R1CS circuit (see `ZkProofSystem::prove_matrix_multiply`):
    /// the `A`/`B` entries are private witnesses, the result entries are public inputs,
    /// and the circuit enforces `Σ_k A[i][k]·B[k][j] = C[i][j]`. `privacy_preserved` is
    /// set only when that Groth16 proof actually verifies — it is now a genuine
    /// cryptographic attestation, not a structural check.
    ///
    /// The ZK circuit operates over integers: entries are rounded to the nearest
    /// integer (exact for integer / fixed-point matrices, which is the intended use).
    /// The returned matrix holds exactly the values the proof attests.
    pub fn private_matrix_multiply(&mut self, left_id: &str, right_id: &str, result_id: &str) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        let left = self.matrix_storage.get_matrix(left_id)?;
        let right = self.matrix_storage.get_matrix(right_id)?;
        if left.cols != right.rows {
            return Err(LinearAlgebraError::InvalidDimensions(
                "Matrix dimensions incompatible for multiplication".to_string(),
            ));
        }
        let (m, k, n) = (left.rows, left.cols, right.cols);

        // Round entries to field integers for the ZK circuit.
        let a_int: Vec<i128> = left.data.iter().map(|v| v.round() as i128).collect();
        let b_int: Vec<i128> = right.data.iter().map(|v| v.round() as i128).collect();

        // Build the real circuit, prove, and verify in zero knowledge.
        let (verified, c_int) = self.privacy_engine.zk_proofs.lock().unwrap()
            .prove_matrix_multiply(m, k, n, &a_int, &b_int)
            .map_err(|e| LinearAlgebraError::PrivacyError(format!("{:?}", e)))?;

        if !verified {
            return Err(LinearAlgebraError::PrivacyError(
                "zero-knowledge proof of A·B = C failed to verify".to_string(),
            ));
        }

        // Return exactly the values the proof attests (the integer product).
        let result_data: Vec<f64> = c_int.iter().map(|&v| v as f64).collect();
        let result = self.create_matrix(result_id.to_string(), m, n, left.data_type, result_data)?;

        let execution_time = start_time.elapsed().as_millis() as u64;
        self.performance_monitor.record_operation("private_matrix_multiply", execution_time, 0);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage: 0,
            operations_used: vec!["private_matrix_multiply".to_string(), "groth16_zk_proof".to_string()],
            privacy_preserved: true,
        })
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> SystemMetrics {
        self.performance_monitor.get_system_metrics()
    }

    /// List all matrices
    pub fn list_matrices(&self) -> Vec<String> {
        self.matrix_storage.list_matrices()
    }

    /// Get matrix information
    pub fn get_matrix_info(&self, matrix_id: &str) -> Option<MatrixMetadata> {
        self.matrix_storage.get_matrix_metadata(matrix_id)
    }
}

