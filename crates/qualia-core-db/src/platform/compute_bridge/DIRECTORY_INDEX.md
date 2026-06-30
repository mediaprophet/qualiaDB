---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# compute_bridge Index

## Functionality Overview
Comprehensive index of functionality for `compute_bridge`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `backend.rs`
  - `struct BackendId`
  - `impl BackendId`
  - `fn as_str`
  - `impl core`
  - `fn fmt`
  - `enum DispatchError`
  - `impl std`
  - `struct KernelPanel`
  - `impl Default`
  - `fn default`
  - `impl KernelPanel`
  - `fn quick`
  - `trait ProbeableBackend`
  - `fn id`
  - `fn available`
  - *(...and 12 more)*
- 📄 `execute.rs`
  - `enum RanOn`
  - `fn shared_policy`
  - `fn cpu_gemm_f32`
  - `fn accelerated_gemm_f32`
  - `fn ref_gemm`
  - `fn small_job_runs_on_cpu_and_is_correct`
  - `fn large_job_is_correct_on_whichever_backend_the_machine_chose`
- 📄 `gpu_gemm.rs`
  - `struct WgpuGemm`
  - `fn backend_rank`
  - `impl WgpuGemm`
  - `fn create`
  - `fn fits`
  - `fn gemm`
  - `fn shared`
  - `fn cpu_gemm`
  - `fn substrate_gemm_matches_cpu_reference_when_gpu_present`
- 📄 `kernel_class.rs`
  - `enum KernelClass`
  - `impl KernelClass`
  - `fn label`
  - `fn is_typically_gpu_amenable`
  - `fn all_is_complete_and_unique`
  - `fn labels_are_distinct`
  - `fn divergent_is_flagged_cpu_biased`
- 📄 `matrix.rs`
  - `struct ClassMatrix`
  - `impl ClassMatrix`
  - `fn from_per_class`
  - `fn rows`
  - `fn best_for`
  - `fn summary`
  - `fn time_ms`
  - `struct CpuBackend`
  - `impl ProbeableBackend`
  - `fn id`
  - `fn available`
  - `fn probe_class`
  - `struct WgpuBackend`
  - `fn probe_class_matrix`
  - `fn cpu_backend_probes_every_class_with_real_rows`
  - *(...and 1 more)*
- 📄 `mod.rs`
  - `fn default_registry`
  - `fn default_registry_has_cpu_and_wgpu`
- 📄 `policy.rs`
  - `struct Plan`
  - `impl Plan`
  - `fn is_cpu`
  - `struct ComputePolicy`
  - `impl ComputePolicy`
  - `fn from_class_matrix`
  - `fn probe`
  - `fn matrix`
  - `fn select`
  - `fn backend_id_for`
  - `fn roomy_host`
  - `fn row`
  - `fn synth_matrix`
  - `fn selects_measured_gpu_winner_for_dense_linear`
  - `fn falls_back_to_cpu_when_only_cpu_probed`
  - *(...and 2 more)*
- 📄 `reference.rs`
  - `fn gemv`
  - `fn axpb`
  - `fn reduce_sum`
  - `fn stencil3`
  - `fn allpairs_potential`
  - `fn fft_radix2`
  - `fn prefix_sum`
  - `fn monte_carlo_pi`
  - `fn gemv_matches_hand`
  - `fn axpb_is_affine`
  - `fn reduce_and_scan_agree_on_total`
  - `fn stencil_of_linear_is_zero_interior`
  - `fn fft_matches_naive_dft`
  - `fn fft_inverse_recovers_signal`
  - `fn monte_carlo_pi_is_in_range_and_deterministic`
  - *(...and 1 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
