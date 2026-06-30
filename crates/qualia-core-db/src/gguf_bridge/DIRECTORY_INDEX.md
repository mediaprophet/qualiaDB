---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# gguf_bridge Index

## Functionality Overview
Comprehensive index of functionality for `gguf_bridge`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Subdirectories
- 📁 `[attention](attention/DIRECTORY_INDEX.md)`
- 📁 `[mc8_wasm](mc8_wasm/DIRECTORY_INDEX.md)`

### Files & Exported Functionality
- 📄 `async_dispatch.rs`
  - `impl QTensorEngine`
  - `fn dispatch_output_logits_into`
  - `fn decode_lexicon_bound`
  - `fn dispatch_gemm_into_async`
- 📄 `attention.rs`
  - `impl QTensorEngine`
- 📄 `cpu_ops.rs`
  - `fn silu_kernel_is_the_stem_silu`
  - `fn rms_norm_kernel_is_the_stem_rms_norm`
  - `fn relu_kernel_is_the_stem_relu`
- 📄 `embedding.rs`
  - `impl QTensorEngine`
  - `fn dispatch_quantized_token_embedding`
  - `fn dispatch_fused_transformer_block`
- 📄 `ffn.rs`
  - `impl QTensorEngine`
- 📄 `forward.rs`
  - `impl QTensorEngine`
  - `fn dispatch_prefill_chunk`
  - `fn dispatch_transformer_layer`
  - `fn dispatch_transformer_forward`
  - `fn dispatch_transformer_layer_async`
  - `fn dispatch_transformer_forward_async`
  - `fn verify_topology_draft_batch`
- 📄 `gemm.rs`
  - `impl QTensorEngine`
  - `fn dispatch_gemm_into`
  - `fn run`
  - `fn llm_quant_gemv_is_the_substrate_gemm`
- 📄 `gpu_params.rs`
- 📄 `init.rs`
  - `impl QTensorEngine`
  - `fn device`
  - `fn queue`
  - `fn try_new`
  - `fn new`
  - `fn reset_kv_cache`
- 📄 `load.rs`
  - `impl QTensorEngine`
  - `fn kv_cache_bytes`
  - `fn load_gguf_checked`
  - `fn load_gguf`
  - `fn adopt_resident_q42_mmap`
  - `fn ternary_ffn_resident_len`
  - `fn adopt_resident_mmap`
  - `fn bench_empty_submit_roundtrip`
- 📄 `mod.rs`
  - `fn dequantize_token_embedding_into`
  - `struct QTensor`
  - `impl QTensor`
  - `fn new`
  - `fn map_from_pointer`
  - `impl Mc8WeightRole`
  - `impl Mc8ElemUniformArena`
  - `impl Mc8AttnUniformArena`
  - `impl Mc8UniformArena`
  - `impl Mc8ChunkUniformCursors`
  - `impl WasmGpuPipeline`
  - `struct KvCacheLayout`
  - `impl KvCacheLayout`
  - `fn from_hyperparams`
  - `fn ring_slot`
  - *(...and 21 more)*
- 📄 `output.rs`
  - `impl QTensorEngine`
  - `fn dispatch_output_argmax_chunked`
  - `fn dispatch_output_top1_chunked`
  - `fn dispatch_output_topk_chunked`
  - `fn apply_output_norm_inplace`
- 📄 `pipeline_cache.rs`
  - `impl QTensorEngine`
- 📄 `prefill_async.rs`
  - `impl QTensorEngine`
  - `fn dispatch_prefill_chunk_async`
  - `fn dispatch_fused_transformer_block_async`
  - `fn dispatch_output_argmax_chunked_async`
  - `fn new_async`
- 📄 `quant_support.rs`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
