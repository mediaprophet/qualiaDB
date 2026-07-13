---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# graph_ops Index

## Functionality Overview
Comprehensive index of functionality for `graph_ops`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `broadcast.rs`
  - `fn broadcast_wgsl`
  - `fn broadcast_cpu`
  - `fn broadcast_gpu`
  - `fn broadcast_cpu_tiles_the_vector`
  - `fn broadcast_wgsl_validates`
  - `fn broadcast_gpu_matches_oracle`
- 📄 `elementwise.rs`
  - `fn unary_expr`
  - `fn binary_expr`
  - `fn is_fma`
  - `fn elementwise_wgsl`
  - `fn unary_cpu`
  - `fn binary_cpu`
  - `fn fma_cpu`
  - `fn unary_gpu`
  - `fn binary_gpu`
  - `fn unary_cpu_hand_checked`
  - `fn binary_and_fma_cpu_hand_checked`
  - `fn elementwise_wgsl_validates_each_arity`
  - `fn elementwise_gpu_matches_oracle`
- 📄 `executor.rs`
  - `fn elems`
  - `fn execute_graph_cpu`
  - `fn final_output`
  - `fn execute_graph`
  - `struct ForgeGraphExecutor`
  - `impl ForgeGraphExecutor`
  - `fn new`
  - `fn with_capacity`
  - `fn with_context`
  - `fn on_shared_gpu`
  - `fn on_shared_gpu_with_capacity`
  - `fn context`
  - `fn run`
  - `fn run_resident`
  - `fn load_weights`
  - *(...and 48 more)*
- 📄 `gather_dequant.rs`
  - `fn ternary`
  - `fn gather_dequant_ternary_wgsl`
  - `fn gather_dequant_ternary_cpu`
  - `fn pack_ternary_as_words`
  - `fn gather_dequant_ternary_gpu`
  - `fn gather_dequant_wgsl_validates`
  - `fn pack_unpack_roundtrips_cpu`
  - `fn gather_dequant_gpu_matches_oracle`
- 📄 `mod.rs`
- 📄 `neighbor.rs`
  - `enum NeighborPath`
  - `fn legalize`
  - `fn sq_dist`
  - `fn frnn_grid_cpu`
  - `fn knn_grid_cpu`
  - `fn neighbor_grid_cpu`
  - `fn recall_vs_grid`
  - `fn legalize_gates_rt_on_3d`
  - `fn frnn_hand_checked`
  - `fn knn_hand_checked`
  - `fn recall_metric`
  - `fn neighbor_dispatch_routes_kinds`
- 📄 `p64_bridge.rs`
  - `struct P64Tensor`
  - `fn read_role`
  - `fn transpose_2d`
  - `struct ForgeLayerWeights`
  - `fn read_forge_layer_weights`
  - `fn find_smollm_gguf`
  - `fn forge_decode_layer_on_real_p64_weights_matches_oracle`
  - `fn forge_decode_layer_real_weights_ms_per_layer`
- 📄 `reduce.rs`
  - `fn fragments`
  - `fn reduce_wgsl`
  - `fn reduce_cpu`
  - `fn reduce_gpu`
  - `fn reduce_cpu_hand_checked`
  - `fn reduce_wgsl_validates_each_kind`
  - `fn reduce_gpu_matches_oracle`
- 📄 `scatter.rs`
  - `fn accum_op_code`
  - `fn accum_identity`
  - `fn scatter_wgsl`
  - `fn scatter_cpu`
  - `fn scatter_gpu`
  - `fn scatter_wgsl_validates`
  - `fn scatter_cpu_hand_checked`
  - `fn scatter_gpu_matches_oracle`
- 📄 `slice.rs`
  - `fn slice_wgsl`
  - `fn slice_cpu`
  - `fn slice_wgsl_validates`
  - `fn slice_cpu_extracts_range`
- 📄 `stencil.rs`
  - `enum RopeMode`
  - `impl RopeMode`
  - `fn code`
  - `struct RopeConfig`
  - `impl RopeConfig`
  - `fn new`
  - `fn validate`
  - `fn rope_params`
  - `fn stencil_wgsl`
  - `fn rope_wgsl`
  - `fn stencil_cpu`
  - `fn rope_cpu`
  - `fn stencil_gpu`
  - `fn rope_gpu`
  - `fn dot`
  - *(...and 8 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
