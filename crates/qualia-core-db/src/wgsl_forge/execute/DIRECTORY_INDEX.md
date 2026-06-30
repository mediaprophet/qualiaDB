---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# execute Index

## Functionality Overview
Comprehensive index of functionality for `execute`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `compute.rs`
  - `trait QualiaCompute`
  - `fn dispatch`
- 📄 `cuda.rs`
  - `struct AffineParamsRaw`
  - `struct CudaComputeContext`
  - `impl CudaComputeContext`
  - `fn new`
  - `fn allocate_and_write`
  - `fn allocate_transient`
  - `fn advance_read_head`
  - `fn clear_transient_allocations`
  - `fn read_buffer_f32`
  - `fn read_buffer_f64`
  - `struct CudaPipeline`
  - `fn compile_cuda_c`
  - `fn compile_cuda_c_source`
  - `fn from_source`
  - `fn nvrtc_compile_to_ptx`
  - *(...and 8 more)*
- 📄 `memory.rs`
  - `enum MemoryTopology`
  - `enum BindingUsage`
  - `struct BufferView`
  - `struct QualiaSlabAllocator`
  - `impl QualiaSlabAllocator`
  - `fn new`
  - `fn new_with_alignment`
  - `fn capacity`
  - `fn allocate_transient`
  - `fn advance_read_head`
  - `fn clear`
  - `fn align_up`
  - `fn ring_buffer_allocates_and_wraps`
  - `fn sustained_dispatch_never_laps_read_head`
  - `fn every_view_offset_is_binding_aligned`
- 📄 `mod.rs`
- 📄 `oracle_ctx.rs`
  - `trait OracleContext`
  - `fn allocate_and_write`
  - `fn allocate_transient`
  - `fn read_buffer_f32`
  - `fn clear_transient_allocations`
  - `fn adapter`
  - `fn constraints`
  - `fn timestamp_supported`
  - `fn run_kernel`
- 📄 `wgpu.rs`
  - `struct WgpuComputeContext`
  - `impl WgpuComputeContext`
  - `fn new`
  - `fn from_device`
  - `fn slab_for`
  - `fn allocate_and_write`
  - `fn allocate_weight`
  - `fn clear_weights`
  - `fn resident_weight_bytes`
  - `fn allocate_transient`
  - `fn advance_read_head`
  - `fn clear_transient_allocations`
  - `fn build_triangle_scene`
  - `fn compile_pipeline`
  - `fn compile_pipeline_cached`
  - *(...and 18 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
