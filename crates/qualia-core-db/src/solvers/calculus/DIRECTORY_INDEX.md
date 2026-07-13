---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# calculus Index

## Functionality Overview
Comprehensive index of functionality for `calculus`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `dense.rs`
  - `fn rk4_step`
  - `fn rk4_integrate`
  - `fn rk4_solve`
  - `fn simpson`
  - `fn simpson_vec`
  - `fn even_panels`
  - `fn shooting_bvp`
  - `fn rk4_solves_a_3d_linear_system`
  - `fn rk4_trajectory_has_all_points`
  - `fn simpson_scalar_and_vector`
  - `fn shooting_bvp_recovers_sine`
  - `fn shooting_bvp_rejects_bad_free_index`
- 📄 `grid.rs`
  - `fn resolve_aligned_byte_offset`
  - `fn pack_f32_pair`
  - `fn unpack_f32_pair`
  - `enum CalculusError`
  - `enum AlignmentError`
  - `struct ContinuousGrid`
  - `fn new`
  - `fn resume_from_quin`
  - `fn len`
  - `fn is_empty`
  - `fn as_slice`
  - `enum SimdWidth`
  - `fn detect_simd_width`
  - `fn detect_cache_line_size`
  - `fn integrate_simpsons_kahan`
  - *(...and 19 more)*
- 📄 `mod.rs`
  - `struct RungeKutta4Static`
  - `struct ODEState`
  - `struct ShootingMethodBVP`
  - `struct BVPState`
  - `struct SimpsonsIntegratorChunked`
  - `struct IntegralChunk`
  - `trait ODEFunction`
  - `fn derivatives`
  - `trait BVPFunction`
  - `fn boundary_residuals`
  - `trait IntegrandFunction`
  - `fn evaluate`
  - `impl RungeKutta4Static`
  - `fn new`
  - `fn integrate`
  - *(...and 28 more)*
- 📄 `ode_advanced.rs`
  - `fn verlet_step`
  - `fn ruth3_step`
  - `fn yoshida4_step`
  - `enum SymplecticMethod`
  - `struct SymplecticResult`
  - `fn integrate_symplectic`
  - `fn dfdy_fd`
  - `fn bdf1_step`
  - `fn bdf2_step`
  - `fn integrate_bdf`
  - `fn hermite_dense_output`
  - `struct SensitivityResult`
  - `fn integrate_with_sensitivity`
  - `fn ho_force`
  - `fn ho_kin`
  - *(...and 7 more)*
- 📄 `ode_solver.rs`
  - `struct ShootingMethod`
  - `trait BvpSystem`
  - `fn derivative`
  - `fn boundary_left`
  - `fn boundary_right`
  - `fn new`
  - `fn with_max_iterations`
  - `fn solve`
  - `fn compute_residual`
  - `struct ChaoitonProfile`
  - `impl ChaoitonProfile`
  - `fn with_params`
  - `impl BvpSystem`
  - `struct LinearDecayBvp`
  - `struct StepSizeAnalyzer`
  - *(...and 54 more)*
- 📄 `tensor_integrity.rs`
  - `struct LineageCommitment`
  - `fn commit_state`
  - `fn lineage_commitment`
  - `fn verify_lineage`
  - `fn integrity_root`
  - `fn transformation_commitment`
  - `fn scale_params`
  - `fn commitment_is_deterministic_and_data_sensitive`
  - `fn lineage_commitment_chains_parent_into_child`
  - `fn tampering_with_an_ancestor_is_detected`
  - `fn integrity_root_is_order_independent_and_change_sensitive`
  - `fn transformation_commitment_binds_the_operation`
- 📄 `tensor_provenance.rs`
  - `struct TensorState`
  - `impl TensorState`
  - `fn new`
  - `fn from_scalar`
  - `fn apply_operation`
  - `fn compute_operation`
  - `fn rk4_step`
  - `fn scale`
  - `fn add`
  - `fn multiply`
  - `fn transpose`
  - `fn reduce_sum`
  - `fn infer_shape`
  - `fn get_provenance_chain`
  - `fn to_quin`
  - *(...and 30 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
