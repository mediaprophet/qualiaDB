---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# linear_algebra Index

## Functionality Overview
Comprehensive index of functionality for `linear_algebra`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `cholesky.rs`
  - `fn cholesky_factor`
  - `fn cholesky_solve`
  - `fn cholesky_determinant`
  - `fn factor_matches_known_lower_triangle`
  - `fn reconstructs_a`
  - `fn solves_linear_system`
  - `fn determinant_via_factor`
  - `fn rejects_non_positive_definite`
  - `fn rejects_bad_dims`
- 📄 `eigen.rs`
  - `fn symmetric_eigen_3x3`
  - `fn symmetric_eigen`
  - `fn approx`
  - `fn closed_form_diagonal`
  - `fn closed_form_known_symmetric`
  - `fn closed_form_matches_jacobi`
  - `fn jacobi_eigenvectors_reconstruct`
  - `fn jacobi_rejects_asymmetric`
  - `fn jacobi_rejects_bad_dims`
- 📄 `gemm.rs`
  - `enum Transpose`
  - `fn gemm`
  - `fn matmul`
  - `fn matvec`
  - `fn transpose`
  - `fn approx`
  - `fn matmul_rectangular`
  - `fn matmul_identity_is_noop`
  - `fn gemm_alpha_beta_accumulate`
  - `fn gemm_beta_zero_ignores_garbage`
  - `fn gemm_transpose_a_normal_equations`
  - `fn gemm_transpose_b`
  - `fn matvec_basic`
  - `fn matvec_transposed_matches_dense_transpose`
  - `fn transpose_roundtrip`
  - *(...and 6 more)*
- 📄 `lu.rs`
  - `struct Lu`
  - `impl Lu`
  - `fn solve`
  - `fn determinant`
  - `fn lu_decompose`
  - `fn lu_solve`
  - `fn determinant_2x2_and_3x3`
  - `fn singular_has_zero_determinant`
  - `fn reconstructs_permuted_a`
  - `fn rejects_bad_dims`
  - `fn lu_solve_recovers_known_solution`
- 📄 `mod.rs`
  - `struct Matrix4x4`
  - `struct Vector4`
  - `struct Tensor3x3x3`
  - `struct FixedLanczosEigensolver`
  - `struct StaticLuDecomposition`
  - `struct ConstTensorContractor`
  - `impl Matrix4x4`
  - `fn get`
  - `fn set`
  - `fn multiply_vector`
  - `fn multiply_matrix`
  - `fn transpose`
  - `fn determinant`
  - `fn minor`
  - `fn determinant_3x3`
  - *(...and 28 more)*
- 📄 `qr.rs`
  - `fn qr_factor`
  - `fn qr_form_q`
  - `fn qr_solve_least_squares`
  - `fn approx`
  - `fn extract_r`
  - `fn factor_reconstructs_a_square`
  - `fn q_has_orthonormal_columns`
  - `fn square_solve_matches_known`
  - `fn least_squares_overdetermined_line_fit`
  - `fn least_squares_overdetermined_noisy`
  - `fn reconstructs_tall_matrix`
  - `fn rank_deficient_fails_closed`
  - `fn rejects_bad_dims`
- 📄 `spectral.rs`
  - `fn characteristic_polynomial`
  - `fn eigenvalues_general`
  - `fn charpoly_of_2x2`
  - `fn general_eigenvalues_real`
  - `fn general_eigenvalues_complex_rotation`
  - `fn rejects_bad_dims`
- 📄 `svd.rs`
  - `struct Svd`
  - `fn svd`
  - `fn reconstructs_square`
  - `fn reconstructs_tall_rectangular`
  - `fn rejects_bad_dims`
- 📄 `vector.rs`
  - `fn add_into`
  - `fn add_assign`
  - `fn hadamard_into`
  - `fn hadamard_assign`
  - `fn scale`
  - `fn axpy`
  - `fn add_is_vector_addition`
  - `fn hadamard_is_elementwise_product`
  - `fn scale_and_axpy`
  - `fn rejects_length_mismatch`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
