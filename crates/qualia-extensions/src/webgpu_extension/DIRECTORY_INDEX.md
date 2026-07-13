---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# webgpu_extension Index

## Functionality Overview
Comprehensive index of functionality for `webgpu_extension`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `electromagnetics.rs`
  - `fn solve`
  - `fn em_params`
  - `fn peak_index`
  - `fn pulse_propagates_at_the_speed_of_light`
  - `fn lossless_energy_is_bounded`
  - `fn lossy_medium_dissipates_energy`
- 📄 `fluid.rs`
  - `fn wrap`
  - `fn solve`
  - `fn rms`
  - `fn tg_params`
  - `fn taylor_green_vortex_decays_at_analytic_rate`
  - `fn projection_keeps_the_field_divergence_free`
  - `fn inviscid_vortex_barely_decays`
- 📄 `heat.rs`
  - `fn wrap`
  - `fn solve`
  - `fn rms`
  - `fn eigenmode_decays_at_analytic_rate`
- 📄 `mod.rs`
  - `struct WebGpuExtension`
  - `struct WebGpuShaderManager`
  - `struct WebGpuShader`
  - `enum ShaderType`
  - `struct WebGpuJobParams`
  - `struct DispatchParams`
  - `impl Default`
  - `fn default`
  - `struct WebGpuExecutionResult`
  - `struct GpuPerformanceMetrics`
  - `struct ConvergenceInfo`
  - `struct SolverReport`
  - `impl WebGpuExtension`
  - `fn new`
  - `fn load_builtin_shaders`
  - *(...and 13 more)*
- 📄 `particles.rs`
  - `struct System`
  - `impl System`
  - `fn accel`
  - `fn energy`
  - `fn solve`
  - `fn symplectic_integrator_conserves_energy`
  - `fn two_body_orbit_stays_bounded`
- 📄 `shaders.rs`
  - `struct SimParams`
  - `fn widx`
  - `fn navier_stokes`
  - `struct EmParams`
  - `fn maxwell_fdtd`
  - `struct HeatParams`
  - `fn hidx`
  - `fn heat`
  - `struct WaveParams`
  - `fn vwidx`
  - `fn wave`
- 📄 `tensor.rs`
  - `fn solve`
  - `fn gemm_params`
  - `fn computes_exact_2x2_product`
  - `fn identity_is_a_left_unit`
  - `fn rectangular_shapes_multiply_correctly`
- 📄 `wave.rs`
  - `fn wrap`
  - `fn solve`
  - `fn analytic_ic`
  - `fn correlation`
  - `fn wave_params`
  - `fn standing_wave_inverts_after_half_period`
  - `fn leapfrog_is_stable_over_a_long_run`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
