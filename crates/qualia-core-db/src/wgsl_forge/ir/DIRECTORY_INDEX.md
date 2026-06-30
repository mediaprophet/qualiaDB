---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# ir Index

## Functionality Overview
Comprehensive index of functionality for `ir`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `capabilities.rs`
  - `struct HardwareCapabilityMatrix`
  - `enum IntrinsicSupport`
  - `impl HardwareCapabilityMatrix`
  - `enum LoweringPolicy64Bit`
  - `struct LoweringContext`
  - `impl LoweringContext`
  - `fn new`
  - `fn policy_64bit`
  - `fn ray_query`
  - `fn rt_intrinsic_excluded_without_rt_cores`
  - `fn coopmat_excluded_but_subgroup_lowers`
- 📄 `core.rs`
  - `struct P64GpuWords64`
  - `impl P64GpuWords64`
  - `fn from_u64_fields`
  - `fn u64_field`
  - `enum ScalarType`
  - `impl ScalarType`
  - `enum BufferElement`
  - `impl BufferElement`
  - `enum BufferAccess`
  - `enum SharedLen`
  - `impl SharedLen`
  - `struct SharedMemorySpec`
  - `struct BufferSpec`
  - `enum Op`
  - `struct KernelSpec`
  - *(...and 16 more)*
- 📄 `graph.rs`
  - `struct NodeId`
  - `impl NodeId`
  - `struct TensorId`
  - `struct Shape`
  - `impl Shape`
  - `fn scalar`
  - `fn new`
  - `fn elements`
  - `enum DType`
  - `enum Layout`
  - `struct TensorRef`
  - `impl TensorRef`
  - `fn external`
  - `fn input`
  - `enum EwKind`
  - *(...and 42 more)*
- 📄 `intrinsics.rs`
  - `enum SubgroupReduceOp`
  - `enum IntrinsicClass`
  - `enum Intrinsic`
  - `impl Intrinsic`
- 📄 `mod.rs`
- 📄 `q42_bridge.rs`
  - `fn opcode_of`
  - `fn err`
  - `fn dtype_code`
  - `fn dtype_from`
  - `fn ewkind_code`
  - `fn ewkind_from`
  - `fn redkind_code`
  - `fn redkind_from`
  - `fn axis_code`
  - `fn axis_from`
  - `fn stencil_code`
  - `fn stencil_from`
  - `fn accum_code`
  - `fn accum_from`
  - `fn nb_code`
  - *(...and 21 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
