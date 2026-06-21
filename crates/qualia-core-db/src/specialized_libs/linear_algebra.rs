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
    fn test_solve_quadratic_two_real() {
        // x² − 5x + 6 = 0 → {2, 3}
        match solve_quadratic(1.0, -5.0, 6.0).unwrap() {
            QuadraticRoots::TwoReal(lo, hi) => {
                assert!((lo - 2.0).abs() < 1e-12);
                assert!((hi - 3.0).abs() < 1e-12);
            }
            other => panic!("expected two real roots, got {:?}", other),
        }
    }

    #[test]
    fn test_solve_quadratic_double_and_complex_and_linear() {
        // x² − 2x + 1 = 0 → double root 1
        assert_eq!(solve_quadratic(1.0, -2.0, 1.0).unwrap(), QuadraticRoots::DoubleReal(1.0));
        // x² + 1 = 0 → ±i
        match solve_quadratic(1.0, 0.0, 1.0).unwrap() {
            QuadraticRoots::ComplexPair { re, im } => {
                assert!(re.abs() < 1e-12 && (im - 1.0).abs() < 1e-12);
            }
            other => panic!("expected complex pair, got {:?}", other),
        }
        // 0·x² + 2x − 4 = 0 → linear root 2
        assert_eq!(solve_quadratic(0.0, 2.0, -4.0).unwrap(), QuadraticRoots::Linear(2.0));
    }

    #[test]
    fn test_polynomial_roots_general() {
        // (x−1)(x−2)(x−3) = x³ − 6x² + 11x − 6 → real roots {1,2,3}
        let mut roots = polynomial_roots(&[1.0, -6.0, 11.0, -6.0]).unwrap();
        assert_eq!(roots.len(), 3);
        assert!(roots.iter().all(|r| r.is_real(1e-7)));
        roots.sort_by(|a, b| a.re.partial_cmp(&b.re).unwrap());
        for (got, want) in roots.iter().zip([1.0, 2.0, 3.0]) {
            assert!((got.re - want).abs() < 1e-6, "root {:?} != {}", got, want);
        }
    }

    #[test]
    fn test_polynomial_roots_complex_quartic() {
        // (x²+1)(x²−1) = x⁴ − 1 → roots {1, −1, i, −i}
        let roots = polynomial_roots(&[1.0, 0.0, 0.0, 0.0, -1.0]).unwrap();
        assert_eq!(roots.len(), 4);
        let real_count = roots.iter().filter(|r| r.is_real(1e-7)).count();
        let imag_count = roots.iter().filter(|r| !r.is_real(1e-7)).count();
        assert_eq!(real_count, 2, "expected ±1 real");
        assert_eq!(imag_count, 2, "expected ±i imaginary");
        // every root satisfies r⁴ = 1 → |r| ≈ 1
        assert!(roots.iter().all(|r| (r.abs() - 1.0).abs() < 1e-6));
    }

    #[test]
    fn test_determinant() {
        // [[1,2],[3,4]] → −2
        assert!((determinant(2, &[1.0, 2.0, 3.0, 4.0]).unwrap() + 2.0).abs() < 1e-12);
        // 3×3 with known det: [[6,1,1],[4,-2,5],[2,8,7]] → −306
        let d = determinant(3, &[6.0, 1.0, 1.0, 4.0, -2.0, 5.0, 2.0, 8.0, 7.0]).unwrap();
        assert!((d + 306.0).abs() < 1e-9, "det = {}", d);
        // Singular matrix → 0
        assert!(determinant(2, &[1.0, 2.0, 2.0, 4.0]).unwrap().abs() < 1e-12);
    }

    #[test]
    fn test_eigen_symmetric() {
        // [[2,1],[1,2]] → eigenvalues {1, 3}; check A·v = λ·v for each.
        let a = [2.0, 1.0, 1.0, 2.0];
        let (mut vals, vecs) = eigen_symmetric(2, &a).unwrap();
        vals.sort_by(|x, y| x.partial_cmp(y).unwrap());
        assert!((vals[0] - 1.0).abs() < 1e-9 && (vals[1] - 3.0).abs() < 1e-9, "vals = {:?}", vals);

        // Re-fetch unsorted to pair eigenvalue j with column j.
        let (vals_u, vecs_u) = eigen_symmetric(2, &a).unwrap();
        for j in 0..2 {
            let (v0, v1) = (vecs_u[0 * 2 + j], vecs_u[1 * 2 + j]);
            // A·v
            let av0 = a[0] * v0 + a[1] * v1;
            let av1 = a[2] * v0 + a[3] * v1;
            // λ·v
            assert!((av0 - vals_u[j] * v0).abs() < 1e-7, "A·v != λ·v (row0, col{j})");
            assert!((av1 - vals_u[j] * v1).abs() < 1e-7, "A·v != λ·v (row1, col{j})");
            // unit eigenvector
            assert!(((v0 * v0 + v1 * v1).sqrt() - 1.0).abs() < 1e-9);
        }
        let _ = vecs;
    }

    #[test]
    fn test_eigen_symmetric_rejects_asymmetric() {
        assert!(eigen_symmetric(2, &[1.0, 2.0, 3.0, 4.0]).is_err());
    }

    #[test]
    fn test_svd_reconstruction() {
        // A is 3×2; verify A ≈ U·Σ·Vᵀ and that singular values are descending ≥ 0.
        let m = 3;
        let n = 2;
        let a = vec![1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let decomp = svd(m, n, &a).unwrap();

        assert_eq!(decomp.singular_values.len(), n);
        assert!(decomp.singular_values[0] >= decomp.singular_values[1] - 1e-12);
        assert!(decomp.singular_values.iter().all(|&s| s >= -1e-12));

        for i in 0..m {
            for j in 0..n {
                let mut recon = 0.0;
                for k in 0..n {
                    recon += decomp.u[i * n + k] * decomp.singular_values[k] * decomp.v[j * n + k];
                }
                assert!(
                    (recon - a[i * n + j]).abs() < 1e-9,
                    "reconstruction[{i}][{j}] = {recon} != {}",
                    a[i * n + j]
                );
            }
        }
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

// ════════════════════════════════════════════════════════════════════════════════
//  Polynomial algebra (ALGEBRA_MANIFOLD_PLAN.md Phase 1)
//  Numeric root finding: quadratics in closed form (stable), general degree via the
//  dependency-free Durand–Kerner (Weierstrass) iteration. f64, dynamic degree.
// ════════════════════════════════════════════════════════════════════════════════

/// A complex number `re + im·i`. Minimal arithmetic for polynomial root finding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const fn new(re: f64, im: f64) -> Self { Self { re, im } }
    pub const fn real(re: f64) -> Self { Self { re, im: 0.0 } }

    #[inline]
    pub fn add(self, o: Complex) -> Complex { Complex::new(self.re + o.re, self.im + o.im) }
    #[inline]
    pub fn sub(self, o: Complex) -> Complex { Complex::new(self.re - o.re, self.im - o.im) }
    #[inline]
    pub fn mul(self, o: Complex) -> Complex {
        Complex::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
    #[inline]
    pub fn div(self, o: Complex) -> Complex {
        let d = o.re * o.re + o.im * o.im;
        Complex::new(
            (self.re * o.re + self.im * o.im) / d,
            (self.im * o.re - self.re * o.im) / d,
        )
    }
    /// Modulus |z|.
    #[inline]
    pub fn abs(self) -> f64 { self.re.hypot(self.im) }
    /// True if within `tol` of the real axis.
    #[inline]
    pub fn is_real(self, tol: f64) -> bool { self.im.abs() <= tol }
}

/// The roots of a real quadratic `a·x² + b·x + c = 0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuadraticRoots {
    /// Two distinct real roots, ascending.
    TwoReal(f64, f64),
    /// One repeated real root (discriminant ≈ 0).
    DoubleReal(f64),
    /// A complex conjugate pair `re ± im·i` (im > 0).
    ComplexPair { re: f64, im: f64 },
    /// Degenerate leading coefficient (a ≈ 0): the single linear root of `b·x + c = 0`.
    Linear(f64),
}

/// Solve `a·x² + b·x + c = 0` over the reals, numerically stably.
///
/// Uses the cancellation-avoiding form `q = -(b + sign(b)·√Δ)/2`, roots `q/a` and `c/q`,
/// for `Δ > 0`; classifies `Δ ≈ 0` as a double root and `Δ < 0` as a complex pair. Falls
/// back to the linear root when `a ≈ 0`.
pub fn solve_quadratic(a: f64, b: f64, c: f64) -> Result<QuadraticRoots, LinearAlgebraError> {
    if !(a.is_finite() && b.is_finite() && c.is_finite()) {
        return Err(LinearAlgebraError::ComputationError("non-finite coefficient".to_string()));
    }
    let scale = a.abs().max(b.abs()).max(c.abs()).max(1.0);

    // Degenerate leading coefficient → linear (or no/everywhere solution).
    if a.abs() <= f64::EPSILON * scale {
        if b.abs() <= f64::EPSILON * scale {
            return Err(LinearAlgebraError::ComputationError(
                "degenerate quadratic: a and b are both ~0".to_string(),
            ));
        }
        return Ok(QuadraticRoots::Linear(-c / b));
    }

    let disc = b * b - 4.0 * a * c;
    let disc_scale = (b * b).max((4.0 * a * c).abs()).max(1.0);
    if disc.abs() <= 1e-12 * disc_scale {
        return Ok(QuadraticRoots::DoubleReal(-b / (2.0 * a)));
    }

    if disc > 0.0 {
        let sqrt_d = disc.sqrt();
        let sign_b = if b >= 0.0 { 1.0 } else { -1.0 };
        let q = -0.5 * (b + sign_b * sqrt_d);
        let r1 = q / a;
        let r2 = c / q;
        let (lo, hi) = if r1 <= r2 { (r1, r2) } else { (r2, r1) };
        Ok(QuadraticRoots::TwoReal(lo, hi))
    } else {
        let re = -b / (2.0 * a);
        let im = (-disc).sqrt() / (2.0 * a.abs());
        Ok(QuadraticRoots::ComplexPair { re, im })
    }
}

/// Evaluate a polynomial at a complex point via Horner's method.
/// `coeffs` are in DESCENDING order: `coeffs[0]·x^n + … + coeffs[n]`.
fn poly_eval_complex(coeffs: &[f64], x: Complex) -> Complex {
    let mut acc = Complex::real(0.0);
    for &c in coeffs {
        acc = acc.mul(x).add(Complex::real(c));
    }
    acc
}

/// Find all complex roots of a real polynomial (DESCENDING coefficients,
/// `coeffs[0]·x^n + … + coeffs[n]`) via the Durand–Kerner iteration.
///
/// Dependency-free and finds all `n` roots simultaneously; suitable for moderate degree.
/// Leading/trailing zeros are trimmed. Returns `n` roots (real roots have `im ≈ 0`).
pub fn polynomial_roots(coeffs: &[f64]) -> Result<Vec<Complex>, LinearAlgebraError> {
    // Trim leading zeros (they do not change the polynomial's degree meaningfully).
    let start = coeffs.iter().position(|c| c.abs() > 0.0)
        .ok_or_else(|| LinearAlgebraError::ComputationError("zero polynomial".to_string()))?;
    let coeffs = &coeffs[start..];
    if coeffs.len() == 1 {
        return Ok(Vec::new()); // a nonzero constant: no roots
    }
    if coeffs.iter().any(|c| !c.is_finite()) {
        return Err(LinearAlgebraError::ComputationError("non-finite coefficient".to_string()));
    }

    // Normalise to monic.
    let lead = coeffs[0];
    let monic: Vec<f64> = coeffs.iter().map(|c| c / lead).collect();
    let degree = monic.len() - 1;

    // Distinct complex initial guesses on a spiral (the classic 0.4 + 0.9i seed).
    let seed = Complex::new(0.4, 0.9);
    let mut roots: Vec<Complex> = (0..degree)
        .map(|k| {
            let mut z = Complex::real(1.0);
            for _ in 0..k { z = z.mul(seed); }
            z
        })
        .collect();

    const MAX_ITERS: usize = 500;
    const TOL: f64 = 1e-14;
    for _ in 0..MAX_ITERS {
        let mut max_delta = 0.0_f64;
        for i in 0..degree {
            let zi = roots[i];
            // denominator = Π_{j≠i} (zi - zj)
            let mut denom = Complex::real(1.0);
            for j in 0..degree {
                if j != i {
                    denom = denom.mul(zi.sub(roots[j]));
                }
            }
            if denom.abs() == 0.0 {
                continue; // coincident guesses; perturb on the next sweep
            }
            let delta = poly_eval_complex(&monic, zi).div(denom);
            roots[i] = zi.sub(delta);
            max_delta = max_delta.max(delta.abs());
        }
        if max_delta < TOL {
            break;
        }
    }

    // Snap near-real roots to the real axis for clean output.
    for r in roots.iter_mut() {
        if r.im.abs() < 1e-9 * (1.0 + r.re.abs()) {
            r.im = 0.0;
        }
    }
    Ok(roots)
}

// ════════════════════════════════════════════════════════════════════════════════
//  Determinant + eigenvalues (ALGEBRA_MANIFOLD_PLAN.md Phase 2)
//  Dependency-free: determinant via LU (partial pivoting); symmetric eigensystem via
//  cyclic Jacobi rotations. Inputs are row-major n×n `f64` slices.
// ════════════════════════════════════════════════════════════════════════════════

/// Determinant of a row-major `n×n` matrix via LU decomposition with partial pivoting.
/// O(n³), numerically robust; returns 0.0 for a singular matrix.
pub fn determinant(n: usize, data: &[f64]) -> Result<f64, LinearAlgebraError> {
    if n == 0 || data.len() != n * n {
        return Err(LinearAlgebraError::InvalidDimensions(
            "determinant expects a non-empty square n×n matrix".to_string(),
        ));
    }
    let mut a = data.to_vec();
    let mut det = 1.0_f64;
    for col in 0..n {
        // Partial pivot: largest magnitude in this column at/below the diagonal.
        let mut pivot = col;
        let mut maxv = a[col * n + col].abs();
        for r in (col + 1)..n {
            let v = a[r * n + col].abs();
            if v > maxv {
                maxv = v;
                pivot = r;
            }
        }
        if maxv == 0.0 {
            return Ok(0.0); // singular column → det = 0
        }
        if pivot != col {
            for k in 0..n {
                a.swap(col * n + k, pivot * n + k);
            }
            det = -det;
        }
        let diag = a[col * n + col];
        det *= diag;
        for r in (col + 1)..n {
            let factor = a[r * n + col] / diag;
            for k in col..n {
                a[r * n + k] -= factor * a[col * n + k];
            }
        }
    }
    Ok(det)
}

/// Eigen-decomposition of a SYMMETRIC row-major `n×n` matrix via cyclic Jacobi
/// rotations. Returns `(eigenvalues, eigenvectors)` where `eigenvectors` is a row-major
/// `n×n` matrix whose COLUMN `j` is the unit eigenvector for `eigenvalues[j]`.
/// Errors if the input is not (within tolerance) symmetric.
pub fn eigen_symmetric(n: usize, data: &[f64]) -> Result<(Vec<f64>, Vec<f64>), LinearAlgebraError> {
    if n == 0 || data.len() != n * n {
        return Err(LinearAlgebraError::InvalidDimensions(
            "eigen_symmetric expects a non-empty square n×n matrix".to_string(),
        ));
    }
    // Symmetry check.
    let scale = data.iter().fold(0.0_f64, |m, &v| m.max(v.abs())).max(1.0);
    for i in 0..n {
        for j in (i + 1)..n {
            if (data[i * n + j] - data[j * n + i]).abs() > 1e-9 * scale {
                return Err(LinearAlgebraError::ComputationError(
                    "eigen_symmetric requires a symmetric matrix".to_string(),
                ));
            }
        }
    }

    let mut a = data.to_vec();
    let mut v = vec![0.0_f64; n * n];
    for i in 0..n {
        v[i * n + i] = 1.0;
    }

    const MAX_SWEEPS: usize = 100;
    for _ in 0..MAX_SWEEPS {
        // Off-diagonal Frobenius norm; stop when negligible.
        let mut off = 0.0_f64;
        for p in 0..n {
            for q in (p + 1)..n {
                off += a[p * n + q] * a[p * n + q];
            }
        }
        if off.sqrt() <= 1e-15 * scale {
            break;
        }
        for p in 0..n {
            for q in (p + 1)..n {
                let apq = a[p * n + q];
                if apq == 0.0 {
                    continue;
                }
                let app = a[p * n + p];
                let aqq = a[q * n + q];
                let theta = (aqq - app) / (2.0 * apq);
                let sign = if theta >= 0.0 { 1.0 } else { -1.0 };
                let t = sign / (theta.abs() + (theta * theta + 1.0).sqrt());
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                // Rotate columns p,q of A.
                for k in 0..n {
                    let akp = a[k * n + p];
                    let akq = a[k * n + q];
                    a[k * n + p] = c * akp - s * akq;
                    a[k * n + q] = s * akp + c * akq;
                }
                // Rotate rows p,q of A.
                for k in 0..n {
                    let apk = a[p * n + k];
                    let aqk = a[q * n + k];
                    a[p * n + k] = c * apk - s * aqk;
                    a[q * n + k] = s * apk + c * aqk;
                }
                // Accumulate the rotation into the eigenvector matrix.
                for k in 0..n {
                    let vkp = v[k * n + p];
                    let vkq = v[k * n + q];
                    v[k * n + p] = c * vkp - s * vkq;
                    v[k * n + q] = s * vkp + c * vkq;
                }
            }
        }
    }

    let eigenvalues: Vec<f64> = (0..n).map(|i| a[i * n + i]).collect();
    Ok((eigenvalues, v))
}

/// Result of a (thin) singular value decomposition `A = U·Σ·Vᵀ` of a row-major `m×n`
/// matrix. `singular_values` (length `n`, descending) is the diagonal of Σ; `u` is
/// row-major `m×n` with left singular vectors as columns; `v` is row-major `n×n` with
/// right singular vectors as columns. Reconstruction: `A[i][j] = Σ_k u[i][k]·σ_k·v[j][k]`.
#[derive(Debug, Clone)]
pub struct Svd {
    pub singular_values: Vec<f64>,
    pub u: Vec<f64>,
    pub v: Vec<f64>,
}

/// Singular value decomposition of a row-major `m×n` matrix via the symmetric
/// eigendecomposition of `AᵀA` (right singular vectors + squared singular values),
/// then `U = A·V·Σ⁻¹`. Singular values are returned in descending order.
pub fn svd(m: usize, n: usize, data: &[f64]) -> Result<Svd, LinearAlgebraError> {
    if m == 0 || n == 0 || data.len() != m * n {
        return Err(LinearAlgebraError::InvalidDimensions(
            "svd expects a non-empty m×n matrix".to_string(),
        ));
    }

    // M = AᵀA  (n×n, symmetric positive semi-definite).
    let mut ata = vec![0.0_f64; n * n];
    for p in 0..n {
        for q in 0..n {
            let mut acc = 0.0;
            for i in 0..m {
                acc += data[i * n + p] * data[i * n + q];
            }
            ata[p * n + q] = acc;
        }
    }

    let (eigvals, eigvecs) = eigen_symmetric(n, &ata)?;

    // Sort columns by descending eigenvalue (= descending σ²).
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&i, &j| eigvals[j].partial_cmp(&eigvals[i]).unwrap());

    let mut singular_values = vec![0.0_f64; n];
    let mut v = vec![0.0_f64; n * n];
    for (new_col, &old_col) in order.iter().enumerate() {
        singular_values[new_col] = eigvals[old_col].max(0.0).sqrt();
        for row in 0..n {
            v[row * n + new_col] = eigvecs[row * n + old_col];
        }
    }

    // U[:,k] = A·V[:,k] / σ_k  (zero column when σ_k ≈ 0).
    let mut u = vec![0.0_f64; m * n];
    let smax = singular_values.first().copied().unwrap_or(0.0).max(1.0);
    for k in 0..n {
        let sigma = singular_values[k];
        if sigma <= 1e-12 * smax {
            continue;
        }
        for i in 0..m {
            let mut acc = 0.0;
            for p in 0..n {
                acc += data[i * n + p] * v[p * n + k];
            }
            u[i * n + k] = acc / sigma;
        }
    }

    Ok(Svd { singular_values, u, v })
}

