//! Linear Algebra Library - High-Performance Mathematical Computing
//!
//! This module provides high-performance linear algebra operations leveraging Phase 2 enhancements:
//! - Hardware-Sympathetic Storage (ZNS) for zero-copy matrix operations
//! - NVMe Computational Storage (CSD) for hardware-accelerated computations
//! - Zero-Knowledge Semantic Proofs for privacy-preserving linear algebra
//! - Ambient Sub-Threshold Orchestration for mobile optimization
pub mod computation;
pub mod core_types;
pub mod optimization;
pub mod performance;
pub mod privacy;
pub mod storage;

pub use computation::*;
pub use core_types::*;
pub use optimization::*;
pub use performance::*;
pub use privacy::*;
pub use storage::*;


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
        let matrix = library
            .create_matrix("test_matrix".to_string(), 2, 2, DataType::Float64, data)
            .unwrap();

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

        library
            .create_matrix("A".to_string(), 2, 2, DataType::Float64, a_data)
            .unwrap();
        library
            .create_matrix("B".to_string(), 2, 2, DataType::Float64, b_data)
            .unwrap();

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
        library
            .create_matrix("A".to_string(), 2, 3, DataType::Float64, data)
            .unwrap();

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
        library
            .create_matrix("A".to_string(), 2, 2, DataType::Float64, data)
            .unwrap();

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

        library
            .create_matrix("A".to_string(), 2, 2, DataType::Float64, matrix_data)
            .unwrap();
        library
            .create_matrix("b".to_string(), 2, 1, DataType::Float64, rhs_data)
            .unwrap();

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
        assert_eq!(
            solve_quadratic(1.0, -2.0, 1.0).unwrap(),
            QuadraticRoots::DoubleReal(1.0)
        );
        // x² + 1 = 0 → ±i
        match solve_quadratic(1.0, 0.0, 1.0).unwrap() {
            QuadraticRoots::ComplexPair { re, im } => {
                assert!(re.abs() < 1e-12 && (im - 1.0).abs() < 1e-12);
            }
            other => panic!("expected complex pair, got {:?}", other),
        }
        // 0·x² + 2x − 4 = 0 → linear root 2
        assert_eq!(
            solve_quadratic(0.0, 2.0, -4.0).unwrap(),
            QuadraticRoots::Linear(2.0)
        );
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
        assert!(
            (vals[0] - 1.0).abs() < 1e-9 && (vals[1] - 3.0).abs() < 1e-9,
            "vals = {:?}",
            vals
        );

        // Re-fetch unsorted to pair eigenvalue j with column j.
        let (vals_u, vecs_u) = eigen_symmetric(2, &a).unwrap();
        for j in 0..2 {
            let (v0, v1) = (vecs_u[0 * 2 + j], vecs_u[1 * 2 + j]);
            // A·v
            let av0 = a[0] * v0 + a[1] * v1;
            let av1 = a[2] * v0 + a[3] * v1;
            // λ·v
            assert!(
                (av0 - vals_u[j] * v0).abs() < 1e-7,
                "A·v != λ·v (row0, col{j})"
            );
            assert!(
                (av1 - vals_u[j] * v1).abs() < 1e-7,
                "A·v != λ·v (row1, col{j})"
            );
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
    fn test_lu_decompose_reconstructs_and_dets() {
        // P·A = L·U: reconstruct A from the factors (applying the pivot permutation).
        let n = 3;
        let a = [4.0, 3.0, 2.0, 2.0, 1.0, 3.0, 3.0, 2.0, 1.0];
        let lu = lu_decompose(n, &a).unwrap();
        assert!(!lu.singular);
        // det agrees with the standalone determinant fn.
        assert!((lu.determinant() - determinant(n, &a).unwrap()).abs() < 1e-9);

        // Rebuild L and U, multiply, and compare to the row-permuted A.
        let mut l = vec![0.0; n * n];
        let mut u = vec![0.0; n * n];
        for i in 0..n {
            l[i * n + i] = 1.0;
            for j in 0..n {
                if j < i {
                    l[i * n + j] = lu.lu[i * n + j];
                } else {
                    u[i * n + j] = lu.lu[i * n + j];
                }
            }
        }
        for i in 0..n {
            for j in 0..n {
                let mut acc = 0.0;
                for k in 0..n {
                    acc += l[i * n + k] * u[k * n + j];
                }
                // (L·U)[i][j] must equal A at the permuted original row.
                let orig_row = lu.pivots[i];
                assert!(
                    (acc - a[orig_row * n + j]).abs() < 1e-9,
                    "LU != P·A at {i},{j}"
                );
            }
        }

        // Singular matrix → flagged + det 0.
        let sing = lu_decompose(2, &[1.0, 2.0, 2.0, 4.0]).unwrap();
        assert!(sing.singular && sing.determinant() == 0.0);
    }

    #[test]
    fn test_eigenvalues_general() {
        // Upper-triangular [[1,2],[0,3]] → eigenvalues {1,3} (real).
        let mut e = eigenvalues_general(2, &[1.0, 2.0, 0.0, 3.0]).unwrap();
        e.sort_by(|a, b| a.re.partial_cmp(&b.re).unwrap());
        assert!(e.iter().all(|z| z.is_real(1e-7)));
        assert!((e[0].re - 1.0).abs() < 1e-6 && (e[1].re - 3.0).abs() < 1e-6);

        // Rotation [[0,-1],[1,0]] → eigenvalues ±i (non-symmetric, complex).
        let r = eigenvalues_general(2, &[0.0, -1.0, 1.0, 0.0]).unwrap();
        assert_eq!(r.len(), 2);
        assert!(r
            .iter()
            .all(|z| z.re.abs() < 1e-6 && (z.im.abs() - 1.0).abs() < 1e-6));
    }

    #[test]
    fn test_characteristic_polynomial_determinant_link() {
        // det(A) = (-1)ⁿ · cₙ for A = [[1,2],[3,4]] (det = −2, n = 2 → cₙ = −2).
        let c = characteristic_polynomial(2, &[1.0, 2.0, 3.0, 4.0]).unwrap();
        assert_eq!(c.len(), 3); // [1, c1, c2]
        assert!((c[1] + 5.0).abs() < 1e-12); // -trace = -(1+4) = -5
        let det_from_poly = c[2]; // (-1)^2 · c2 = c2
        assert!((det_from_poly - determinant(2, &[1.0, 2.0, 3.0, 4.0]).unwrap()).abs() < 1e-9);
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

        library
            .create_matrix("A".to_string(), 2, 2, DataType::Float64, a_data)
            .unwrap();
        library
            .create_matrix("B".to_string(), 2, 2, DataType::Float64, b_data)
            .unwrap();

        let result = library.private_matrix_multiply("A", "B", "C").unwrap();

        assert!(
            result.privacy_preserved,
            "the Groth16 proof of A·B = C must verify"
        );
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
        library
            .create_matrix(
                "A".to_string(),
                2,
                3,
                DataType::Float64,
                vec![1.0, 2.0, 3.0, 4.0, -5.0, 6.0],
            )
            .unwrap();
        library
            .create_matrix(
                "B".to_string(),
                3,
                2,
                DataType::Float64,
                vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
            )
            .unwrap();

        let result = library.private_matrix_multiply("A", "B", "C").unwrap();

        assert!(result.privacy_preserved);
        // Row0: [1·7+2·9+3·11, 1·8+2·10+3·12] = [58, 64]
        // Row1: [4·7-5·9+6·11, 4·8-5·10+6·12] = [49, 54]
        for (got, want) in result.result.data.iter().zip([58.0, 64.0, 49.0, 54.0]) {
            assert!((got - want).abs() < 1e-4);
        }
    }

    #[test]
    fn test_private_matrix_multiplication_fractional() {
        // Real-valued (non-integer) matrices must now work via the fixed-point encoding.
        // [[0.5,1.5],[2.5,0.5]] · [[1.0,0.0],[0.0,2.0]] = [[0.5,3.0],[2.5,1.0]]
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();
        library
            .create_matrix(
                "A".to_string(),
                2,
                2,
                DataType::Float64,
                vec![0.5, 1.5, 2.5, 0.5],
            )
            .unwrap();
        library
            .create_matrix(
                "B".to_string(),
                2,
                2,
                DataType::Float64,
                vec![1.0, 0.0, 0.0, 2.0],
            )
            .unwrap();

        let result = library.private_matrix_multiply("A", "B", "C").unwrap();
        assert!(result.privacy_preserved);
        for (got, want) in result.result.data.iter().zip([0.5, 3.0, 2.5, 1.0]) {
            assert!(
                (got - want).abs() < 1e-4,
                "fixed-point ZK result {got} != {want}"
            );
        }
    }

    // === Integration tests for cache, monitoring, and pattern recognition ===

    #[test]
    fn test_multiply_uses_cache_and_records_metrics() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();

        let a_data = vec![1.0, 2.0, 3.0, 4.0];
        let b_data = vec![5.0, 6.0, 7.0, 8.0];

        library
            .create_matrix("A".to_string(), 2, 2, DataType::Float64, a_data)
            .unwrap();
        library
            .create_matrix("B".to_string(), 2, 2, DataType::Float64, b_data)
            .unwrap();

        // First multiply — should compute and cache the result
        let result1 = library.matrix_multiply("A", "B", "C", 1.0, 0.0).unwrap();
        assert_eq!(result1.result.data, vec![19.0, 22.0, 43.0, 50.0]);

        // Verify performance metrics were recorded
        let stats = library.get_performance_stats();
        assert!(stats.total_operations > 0);

        // Verify operation metrics were recorded
        let op_metrics = library.performance_monitor.get_operation_metrics("matrix_multiply");
        assert!(op_metrics.is_some());
        assert!(op_metrics.unwrap().count > 0);

        // Verify matrix access was recorded
        let m_metrics = library.performance_monitor.get_matrix_metrics("A");
        assert!(m_metrics.is_some());
        assert!(m_metrics.unwrap().access_count > 0);
    }

    #[test]
    fn test_cache_populated_after_operation() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();

        library
            .create_matrix("A".to_string(), 2, 2, DataType::Float64, vec![1.0, 2.0, 3.0, 4.0])
            .unwrap();
        library
            .create_matrix("B".to_string(), 2, 2, DataType::Float64, vec![5.0, 6.0, 7.0, 8.0])
            .unwrap();

        // After multiply, the result "C" should be in the cache
        library.matrix_multiply("A", "B", "C", 1.0, 0.0).unwrap();

        // The cache should have entries (at least the result matrix)
        assert!(library.matrix_storage.cache.cache_size() > 0);
    }

    #[test]
    fn test_analyze_matrix_method() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();

        // Create a symmetric positive definite matrix
        library
            .create_matrix(
                "S".to_string(),
                2,
                2,
                DataType::Float64,
                vec![2.0, 1.0, 1.0, 2.0],
            )
            .unwrap();

        let analysis = library.analyze_matrix("S").unwrap();
        assert_eq!(analysis.matrix_id, "S");
        assert!(analysis.detected_patterns.contains(&MatrixPattern::Symmetric));
        assert!(analysis.detected_patterns.contains(&MatrixPattern::PositiveDefinite));
        assert!(!analysis.recommended_algorithms.is_empty());
    }

    #[test]
    fn test_analyze_diagonal_matrix() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();

        library
            .create_matrix(
                "D".to_string(),
                3,
                3,
                DataType::Float64,
                vec![1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0],
            )
            .unwrap();

        let analysis = library.analyze_matrix("D").unwrap();
        assert!(analysis.detected_patterns.contains(&MatrixPattern::Diagonal));
    }

    #[test]
    fn test_performance_summary() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();

        library
            .create_matrix("A".to_string(), 2, 2, DataType::Float64, vec![1.0, 2.0, 3.0, 4.0])
            .unwrap();
        library
            .create_matrix("B".to_string(), 2, 2, DataType::Float64, vec![5.0, 6.0, 7.0, 8.0])
            .unwrap();
        library.matrix_multiply("A", "B", "C", 1.0, 0.0).unwrap();

        let summary = library.performance_summary();
        assert!(summary.contains("Linear Algebra Performance Summary"));
        assert!(summary.contains("matrix_multiply"));
    }

    #[test]
    fn test_cache_hit_rate_accessor() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();

        // Initially, no cache accesses
        assert_eq!(library.cache_hit_rate(), 0.0);

        library
            .create_matrix("A".to_string(), 2, 2, DataType::Float64, vec![1.0, 2.0, 3.0, 4.0])
            .unwrap();
        library
            .create_matrix("B".to_string(), 2, 2, DataType::Float64, vec![5.0, 6.0, 7.0, 8.0])
            .unwrap();
        library.matrix_multiply("A", "B", "C", 1.0, 0.0).unwrap();

        // After an operation, cache should have entries
        assert!(library.cache_size() > 0);
    }

    #[test]
    fn test_transpose_records_metrics() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();

        library
            .create_matrix("A".to_string(), 2, 3, DataType::Float64, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0])
            .unwrap();

        library.matrix_transpose("A", "AT").unwrap();

        let op_metrics = library.performance_monitor.get_operation_metrics("matrix_transpose");
        assert!(op_metrics.is_some());
        assert!(op_metrics.unwrap().count > 0);
    }

    #[test]
    fn test_inverse_records_metrics() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();

        library
            .create_matrix("A".to_string(), 2, 2, DataType::Float64, vec![2.0, 1.0, 1.0, 1.0])
            .unwrap();

        library.matrix_inverse("A", "A_inv").unwrap();

        let op_metrics = library.performance_monitor.get_operation_metrics("matrix_inverse");
        assert!(op_metrics.is_some());
        assert!(op_metrics.unwrap().count > 0);
    }

    #[test]
    fn test_solve_records_metrics() {
        let mut library = LinearAlgebraLibrary::new();
        library.initialize().unwrap();

        library
            .create_matrix("A".to_string(), 2, 2, DataType::Float64, vec![2.0, 1.0, 1.0, 1.0])
            .unwrap();
        library
            .create_matrix("b".to_string(), 2, 1, DataType::Float64, vec![3.0, 2.0])
            .unwrap();

        library.solve_linear_system("A", "b", "x").unwrap();

        let op_metrics = library.performance_monitor.get_operation_metrics("solve_linear_system");
        assert!(op_metrics.is_some());
        assert!(op_metrics.unwrap().count > 0);
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
    pub fn create_matrix(
        &mut self,
        matrix_id: String,
        rows: usize,
        cols: usize,
        data_type: DataType,
        data: Vec<f64>,
    ) -> Result<Matrix, LinearAlgebraError> {
        // Validate input
        if data.len() != rows * cols {
            return Err(LinearAlgebraError::InvalidDimensions(
                "Data size doesn't match dimensions".to_string(),
            ));
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
    pub fn matrix_multiply(
        &mut self,
        left_id: &str,
        right_id: &str,
        result_id: &str,
        alpha: f64,
        beta: f64,
    ) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        // Check cache for the result matrix
        let cache_key = format!("mul:{}:{}:{}:{}:{}", left_id, right_id, result_id, alpha, beta);
        let cache_hit = self.matrix_storage.cache.get(&cache_key).is_some();
        if cache_hit {
            // Retrieve from cache
            if let Some(cached) = self.matrix_storage.cache.get(&cache_key) {
                let execution_time = start_time.elapsed().as_millis() as u64;
                self.performance_monitor
                    .record_operation("matrix_multiply", execution_time, 0);
                self.performance_monitor
                    .record_matrix_access(result_id, "matrix_multiply", true);
                return Ok(LinearAlgebraResult {
                    result: cached,
                    execution_time,
                    memory_usage: 0,
                    operations_used: vec!["matrix_multiply".to_string(), "cache_hit".to_string()],
                    privacy_preserved: false,
                });
            }
        }

        // Get matrices
        let left = self.matrix_storage.get_matrix(left_id)?;
        let right = self.matrix_storage.get_matrix(right_id)?;

        // Record cache miss for input matrices
        self.performance_monitor
            .record_matrix_access(left_id, "matrix_multiply", false);
        self.performance_monitor
            .record_matrix_access(right_id, "matrix_multiply", false);

        // Validate dimensions
        if left.cols != right.rows {
            return Err(LinearAlgebraError::InvalidDimensions(
                "Matrix dimensions incompatible for multiplication".to_string(),
            ));
        }

        // Optimize operation
        let optimized_operation = self
            .optimization_engine
            .optimize_multiplication(&left, &right)?;

        // Execute multiplication
        let result_data =
            self.computation_engine
                .execute_multiplication(&optimized_operation, alpha, beta)?;

        // Create result matrix
        let result = self.create_matrix(
            result_id.to_string(),
            left.rows,
            right.cols,
            left.data_type.clone(),
            result_data,
        )?;

        // Store result in cache
        self.matrix_storage.cache.put(&result)?;

        let execution_time = start_time.elapsed().as_millis() as u64;
        let memory_usage = (left.rows * right.cols * 8) as u64;

        // Update performance metrics with detailed info
        self.performance_monitor
            .record_operation_detailed("matrix_multiply", execution_time as f64, (left.rows, right.cols));
        self.performance_monitor
            .record_operation("matrix_multiply", execution_time, memory_usage);
        self.performance_monitor
            .record_matrix_access(result_id, "matrix_multiply", false);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage,
            operations_used: vec!["matrix_multiply".to_string()],
            privacy_preserved: false,
        })
    }

    /// Matrix addition
    pub fn matrix_add(
        &mut self,
        left_id: &str,
        right_id: &str,
        result_id: &str,
        alpha: f64,
    ) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        // Get matrices
        let left = self.matrix_storage.get_matrix(left_id)?;
        let right = self.matrix_storage.get_matrix(right_id)?;

        // Record matrix access for monitoring
        self.performance_monitor
            .record_matrix_access(left_id, "matrix_add", false);
        self.performance_monitor
            .record_matrix_access(right_id, "matrix_add", false);

        // Validate dimensions
        if left.rows != right.rows || left.cols != right.cols {
            return Err(LinearAlgebraError::InvalidDimensions(
                "Matrix dimensions incompatible for addition".to_string(),
            ));
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
            left.data_type.clone(),
            result_data,
        )?;

        // Store result in cache
        self.matrix_storage.cache.put(&result)?;

        let execution_time = start_time.elapsed().as_millis() as u64;
        let memory_usage = (left.rows * left.cols * 8) as u64;

        // Update performance metrics
        self.performance_monitor
            .record_operation_detailed("matrix_add", execution_time as f64, (left.rows, left.cols));
        self.performance_monitor
            .record_operation("matrix_add", execution_time, memory_usage);
        self.performance_monitor
            .record_matrix_access(result_id, "matrix_add", false);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage,
            operations_used: vec!["matrix_add".to_string()],
            privacy_preserved: false,
        })
    }

    /// Matrix transpose
    pub fn matrix_transpose(
        &mut self,
        input_id: &str,
        result_id: &str,
    ) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        // Get matrix
        let input = self.matrix_storage.get_matrix(input_id)?;

        // Record matrix access for monitoring
        self.performance_monitor
            .record_matrix_access(input_id, "matrix_transpose", false);

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
            input.data_type.clone(),
            result_data,
        )?;

        // Store result in cache
        self.matrix_storage.cache.put(&result)?;

        let execution_time = start_time.elapsed().as_millis() as u64;
        let memory_usage = (input.rows * input.cols * 8) as u64;

        // Update performance metrics
        self.performance_monitor
            .record_operation_detailed("matrix_transpose", execution_time as f64, (input.rows, input.cols));
        self.performance_monitor
            .record_operation("matrix_transpose", execution_time, memory_usage);
        self.performance_monitor
            .record_matrix_access(result_id, "matrix_transpose", false);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage,
            operations_used: vec!["matrix_transpose".to_string()],
            privacy_preserved: false,
        })
    }

    /// Matrix inverse
    pub fn matrix_inverse(
        &mut self,
        input_id: &str,
        result_id: &str,
    ) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        // Get matrix
        let input = self.matrix_storage.get_matrix(input_id)?;

        // Record matrix access for monitoring
        self.performance_monitor
            .record_matrix_access(input_id, "matrix_inverse", false);

        // Validate square matrix
        if input.rows != input.cols {
            return Err(LinearAlgebraError::InvalidDimensions(
                "Matrix must be square for inversion".to_string(),
            ));
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
                return Err(LinearAlgebraError::SingularMatrix(
                    "Matrix is singular".to_string(),
                ));
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
        let result =
            self.create_matrix(result_id.to_string(), n, n, input.data_type.clone(), result_data)?;

        // Store result in cache
        self.matrix_storage.cache.put(&result)?;

        let execution_time = start_time.elapsed().as_millis() as u64;
        let memory_usage = (n * n * 8) as u64;

        // Update performance metrics
        self.performance_monitor
            .record_operation_detailed("matrix_inverse", execution_time as f64, (n, n));
        self.performance_monitor
            .record_operation("matrix_inverse", execution_time, memory_usage);
        self.performance_monitor
            .record_matrix_access(result_id, "matrix_inverse", false);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage,
            operations_used: vec!["matrix_inverse".to_string()],
            privacy_preserved: false,
        })
    }

    /// Solve linear system Ax = b
    pub fn solve_linear_system(
        &mut self,
        matrix_id: &str,
        rhs_id: &str,
        solution_id: &str,
    ) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        // Get matrices
        let matrix = self.matrix_storage.get_matrix(matrix_id)?;
        let rhs = self.matrix_storage.get_matrix(rhs_id)?;

        // Record matrix access for monitoring
        self.performance_monitor
            .record_matrix_access(matrix_id, "solve_linear_system", false);
        self.performance_monitor
            .record_matrix_access(rhs_id, "solve_linear_system", false);

        // Validate dimensions
        if matrix.rows != matrix.cols {
            return Err(LinearAlgebraError::InvalidDimensions(
                "Matrix must be square".to_string(),
            ));
        }
        if matrix.rows != rhs.rows {
            return Err(LinearAlgebraError::InvalidDimensions(
                "Matrix and RHS dimensions incompatible".to_string(),
            ));
        }

        // Composition boundary: marshal into caller-owned buffers and solve via
        // the engine's Householder QR (replaces an inline Gauss-Jordan duplicate).
        // QR is numerically stable for the square nonsingular case and fails
        // closed on a (near-)singular system.
        use crate::solvers::linear_algebra::qr;
        use crate::solvers::SolversError;
        let n = matrix.rows;
        let mut a = matrix.data.clone(); // QR overwrites with R + reflectors
        let mut tau = vec![0.0; n];
        let mut b = rhs.data.clone(); // overwritten with Qᵀ·b
        let mut solution_data = vec![0.0; n];
        let map_err = |e: SolversError| match e {
            SolversError::SingularMatrix => {
                LinearAlgebraError::SingularMatrix("System is singular".to_string())
            }
            _ => LinearAlgebraError::InvalidDimensions(
                "matrix/RHS dimensions incompatible".to_string(),
            ),
        };
        qr::qr_factor(n, n, &mut a, &mut tau).map_err(map_err)?;
        qr::qr_solve_least_squares(n, n, &a, &tau, &mut b, &mut solution_data).map_err(map_err)?;

        // Create result matrix
        let result = self.create_matrix(
            solution_id.to_string(),
            n,
            1,
            matrix.data_type.clone(),
            solution_data,
        )?;

        // Store result in cache
        self.matrix_storage.cache.put(&result)?;

        let execution_time = start_time.elapsed().as_millis() as u64;
        let memory_usage = (n * n * 8) as u64;

        // Update performance metrics
        self.performance_monitor
            .record_operation_detailed("solve_linear_system", execution_time as f64, (n, n));
        self.performance_monitor
            .record_operation("solve_linear_system", execution_time, memory_usage);
        self.performance_monitor
            .record_matrix_access(solution_id, "solve_linear_system", false);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage,
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
    /// The ZK circuit operates over a FIXED-POINT encoding: each entry is scaled by
    /// 1e6 and rounded to a field integer, so real-valued matrices are supported to
    /// ~1e-6 precision (integer matrices are encoded exactly). The proof attests the
    /// exact scaled-integer identity; the returned matrix is that result rescaled.
    pub fn private_matrix_multiply(
        &mut self,
        left_id: &str,
        right_id: &str,
        result_id: &str,
    ) -> Result<LinearAlgebraResult<Matrix>, LinearAlgebraError> {
        let start_time = std::time::Instant::now();

        let left = self.matrix_storage.get_matrix(left_id)?;
        let right = self.matrix_storage.get_matrix(right_id)?;
        if left.cols != right.rows {
            return Err(LinearAlgebraError::InvalidDimensions(
                "Matrix dimensions incompatible for multiplication".to_string(),
            ));
        }
        let (m, k, n) = (left.rows, left.cols, right.cols);

        // Fixed-point encoding so REAL-valued (not just integer) matrices get a ZK proof.
        // Each entry is scaled by FIXED_POINT_SCALE and rounded to a field integer, so the
        // circuit proves the exact integer identity Σ a'·b' = C' where a' = round(a·S),
        // b' = round(b·S). The product is then scaled by S², so the real result is C'/S².
        // Precision is ~1/S; for integer matrices the encoding is exact (S·int is integer).
        const FIXED_POINT_SCALE: f64 = 1_000_000.0;
        let a_int: Vec<i128> = left
            .data
            .iter()
            .map(|v| (v * FIXED_POINT_SCALE).round() as i128)
            .collect();
        let b_int: Vec<i128> = right
            .data
            .iter()
            .map(|v| (v * FIXED_POINT_SCALE).round() as i128)
            .collect();

        // Build the real circuit, prove, and verify in zero knowledge.
        let (verified, c_int) = self
            .privacy_engine
            .zk_proofs
            .lock()
            .unwrap()
            .prove_matrix_multiply(m, k, n, &a_int, &b_int)
            .map_err(|e| LinearAlgebraError::PrivacyError(format!("{:?}", e)))?;

        if !verified {
            return Err(LinearAlgebraError::PrivacyError(
                "zero-knowledge proof of A·B = C failed to verify".to_string(),
            ));
        }

        // Recover the real-valued product from the attested fixed-point integers (÷ S²).
        let scale_sq = FIXED_POINT_SCALE * FIXED_POINT_SCALE;
        let result_data: Vec<f64> = c_int.iter().map(|&v| v as f64 / scale_sq).collect();
        let result =
            self.create_matrix(result_id.to_string(), m, n, left.data_type, result_data)?;

        let execution_time = start_time.elapsed().as_millis() as u64;
        self.performance_monitor
            .record_operation("private_matrix_multiply", execution_time, 0);

        Ok(LinearAlgebraResult {
            result,
            execution_time,
            memory_usage: 0,
            operations_used: vec![
                "private_matrix_multiply".to_string(),
                "groth16_zk_proof".to_string(),
            ],
            privacy_preserved: true,
        })
    }

    /// Analyze a matrix: detect structural patterns and return optimization hints.
    /// Uses the MatrixAnalyzer to detect diagonal, triangular, symmetric, sparse,
    /// banded, block-diagonal, Toeplitz, orthogonal, circulant, Hankel, and
    /// positive-definite patterns.
    pub fn analyze_matrix(
        &mut self,
        matrix_id: &str,
    ) -> Result<MatrixAnalysis, LinearAlgebraError> {
        let matrix = self.matrix_storage.get_matrix(matrix_id)?;
        self.optimization_engine.analyzer.analyze_matrix(&matrix)
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> SystemMetrics {
        self.performance_monitor.get_system_metrics()
    }

    /// Get a human-readable performance summary
    pub fn performance_summary(&self) -> String {
        self.performance_monitor.summary()
    }

    /// Get the cache hit rate (0.0 to 1.0)
    pub fn cache_hit_rate(&self) -> f64 {
        self.matrix_storage.cache.hit_rate()
    }

    /// Get the current cache size in bytes
    pub fn cache_size(&self) -> usize {
        self.matrix_storage.cache.cache_size()
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

/// Complex / quadratic / polynomial-root algebra now lives in the engine
/// (`solvers::polynomial`); re-exported here so the silo's API is unchanged.
pub use crate::solvers::polynomial::{Complex, QuadraticRoots};

/// Solve `a·x² + b·x + c = 0` over the reals — thin facade over the engine
/// `solvers::polynomial::solve_quadratic` (maps the engine error to this lib's type).
pub fn solve_quadratic(a: f64, b: f64, c: f64) -> Result<QuadraticRoots, LinearAlgebraError> {
    crate::solvers::polynomial::solve_quadratic(a, b, c).map_err(|_| {
        LinearAlgebraError::ComputationError("invalid quadratic coefficients".to_string())
    })
}

/// Find all complex roots of a real polynomial — thin facade over the engine
/// `solvers::polynomial::polynomial_roots` (Durand–Kerner).
pub fn polynomial_roots(coeffs: &[f64]) -> Result<Vec<Complex>, LinearAlgebraError> {
    crate::solvers::polynomial::polynomial_roots(coeffs)
        .map_err(|_| LinearAlgebraError::ComputationError("invalid polynomial".to_string()))
}

// ════════════════════════════════════════════════════════════════════════════════
//  Determinant + eigenvalues (ALGEBRA_MANIFOLD_PLAN.md Phase 2)
//  Dependency-free: determinant via LU (partial pivoting); symmetric eigensystem via
//  cyclic Jacobi rotations. Inputs are row-major n×n `f64` slices.
// ════════════════════════════════════════════════════════════════════════════════

/// LU decomposition with partial pivoting (`P·A = L·U`). The canonical dynamic LU now
/// lives in the engine (`solvers::linear_algebra::lu`); re-exported here so the silo's
/// existing API surface (and `Lu::determinant`) is unchanged.
pub use crate::solvers::linear_algebra::lu::Lu;

/// LU-decompose a row-major `n×n` matrix with partial pivoting — thin facade over the
/// engine `lu_decompose` (maps the engine error to this lib's error type).
pub fn lu_decompose(n: usize, data: &[f64]) -> Result<Lu, LinearAlgebraError> {
    crate::solvers::linear_algebra::lu::lu_decompose(n, data).map_err(|_| {
        LinearAlgebraError::InvalidDimensions(
            "lu_decompose expects a non-empty square n×n matrix".to_string(),
        )
    })
}

/// Determinant of a row-major `n×n` matrix via LU decomposition — thin facade over the
/// engine `determinant`. O(n³), numerically robust; returns 0.0 for a singular matrix.
pub fn determinant(n: usize, data: &[f64]) -> Result<f64, LinearAlgebraError> {
    crate::solvers::linear_algebra::lu::determinant(n, data).map_err(|_| {
        LinearAlgebraError::InvalidDimensions(
            "determinant expects a non-empty square n×n matrix".to_string(),
        )
    })
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
    // Composition boundary: marshal into caller-owned buffers and call the engine's
    // canonical symmetric eigensolver (replaces an inline cyclic-Jacobi duplicate).
    let mut a = data.to_vec();
    let mut v = vec![0.0_f64; n * n];
    crate::solvers::linear_algebra::eigen::symmetric_eigen(n, &mut a, &mut v).map_err(
        |e| match e {
            crate::solvers::SolversError::InvalidParameters => {
                LinearAlgebraError::ComputationError(
                    "eigen_symmetric requires a symmetric matrix".to_string(),
                )
            }
            _ => LinearAlgebraError::InvalidDimensions(
                "eigen_symmetric expects a non-empty square n×n matrix".to_string(),
            ),
        },
    )?;
    // Eigenvalues are the diagonal of the rotated matrix; v's column j is its eigenvector.
    let eigenvalues: Vec<f64> = (0..n).map(|i| a[i * n + i]).collect();
    Ok((eigenvalues, v))
}

/// Characteristic polynomial — thin facade over the engine
/// `solvers::linear_algebra::spectral::characteristic_polynomial` (Faddeev–LeVerrier).
pub fn characteristic_polynomial(n: usize, data: &[f64]) -> Result<Vec<f64>, LinearAlgebraError> {
    crate::solvers::linear_algebra::spectral::characteristic_polynomial(n, data).map_err(|_| {
        LinearAlgebraError::InvalidDimensions(
            "characteristic_polynomial expects a non-empty square n×n matrix".to_string(),
        )
    })
}

/// General (non-symmetric) eigenvalues — thin facade over the engine
/// `solvers::linear_algebra::spectral::eigenvalues_general`.
pub fn eigenvalues_general(n: usize, data: &[f64]) -> Result<Vec<Complex>, LinearAlgebraError> {
    crate::solvers::linear_algebra::spectral::eigenvalues_general(n, data)
        .map_err(|_| LinearAlgebraError::ComputationError("eigenvalues_general failed".to_string()))
}

/// SVD `A = U·Σ·Vᵀ` — the canonical implementation now lives in the engine
/// (`solvers::linear_algebra::svd`); re-exported here so the silo's API is unchanged.
pub use crate::solvers::linear_algebra::svd::Svd;

/// Singular value decomposition of a row-major `m×n` matrix — thin facade over the
/// engine `svd` (maps the engine error to this lib's error type). Singular values are
/// returned in descending order.
pub fn svd(m: usize, n: usize, data: &[f64]) -> Result<Svd, LinearAlgebraError> {
    crate::solvers::linear_algebra::svd::svd(m, n, data).map_err(|_| {
        LinearAlgebraError::InvalidDimensions("svd expects a non-empty m×n matrix".to_string())
    })
}
