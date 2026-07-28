# Qwen2 Q/K Bias Fix Plan

## Problem

Qwen2.5 produces garbage output. Root cause identified: Qwen2 uses attention
biases (`blk.{L}.attn_q.bias`, `blk.{L}.attn_k.bias`) added to Q and K
projections **before** RoPE. The engine ignores these tensors entirely.

## Architecture Context

- Qwen2 Q proj: `W_q @ x + b_q` (bias shape = `[n_head * head_dim]`)
- Qwen2 K proj: `W_k @ x + b_k` (bias shape = `[n_kv_head * head_dim]`)
- Qwen2 V proj: `W_v @ x` (no bias)
- Qwen2 O proj: `W_o @ x` (no bias)
- Llama/SmolLM2: no biases on any attention projection

## Data Flow (WASM prefill path)

1. `mc8_stage_prefill_layer_super_arena` stages K/V/Q GEMM params
2. `encode_gemm_bufs_offset` dispatches parallel GEMM → `k_proj`, `v_proj`, `q_proj` buffers
3. `encode_attention_batched_q_prefill` dispatches attention shader reading pre-computed Q
4. `encode_attention_pass_gpu` dispatches attention shader for K/V writes + Q attention

The GEMM shader (`fused_transformer.wgsl`) computes `output[row] = W[row] · x`.
No bias addition exists anywhere in this pipeline.

## Data Flow (WASM decode path)

1. `dispatch_transformer_forward_async` calls `mc8_stage_prefill_layer_super_arena`
2. Then `encode_attention_pass_gpu` for K/V write + Q attention
3. For decode, `proj_row_stride` may be 0 → attention shader does in-shader `gemm_row`

## Data Flow (native path)

1. `dispatch_attention_layer` / `dispatch_attention_q_ffn_token_async`
2. CPU `rope_inplace` is called after Q/K projection
3. CPU GEMM (`stack_gemm_quant`) also has no bias addition

## Implementation Plan

### Approach: Add bias in the attention shader

Adding bias to the GEMM shader would require modifying `GemmGpuParams` (15+
construction sites) and the GEMM shader entry points. Too invasive.

Instead, add bias in the **attention shader** where Q/K are loaded into shared
memory — `gemm_row` already reads the projection there. This is the single
chokepoint for both prefill (proj_row_stride != 0) and decode (proj_row_stride
== 0) paths.

### Step 1: Load bias tensors in `LayerTensors`

File: `inference/gguf_sharder/types.rs`
- Add `attn_q_bias: Option<GgufTensorInfo>` and `attn_k_bias: Option<GgufTensorInfo>`

File: `inference/gguf_sharder/tensor_index.rs`
- Add `attn_q_bias: self.find_layer_tensor(layer_idx, b"attn_q.bias")`
- Add `attn_k_bias: self.find_layer_tensor(layer_idx, b"attn_k.bias")`

### Step 2: Upload bias weights to GPU

The bias is a 1D f32 vector (not quantized in GGUF — it's typically F32).
Size: `n_head * head_dim * 4` bytes for Q bias, `n_kv_head * head_dim * 4` for K bias.

Options:
- A) Upload to a dedicated bias buffer per layer (new binding in attention shader)
- B) Append bias data after weight data in the existing weight buffer
- C) Upload as a separate small staging buffer and bind as a new storage buffer

**Chosen: Option C** — add a new binding slot to the attention bind group for
bias data. This is the least disruptive to existing weight residency logic.

### Step 3: Add bias binding to attention shader

File: `shaders/fused_attention.wgsl`
- Add `@group(0) @binding(6) var<storage, read> q_bias: array<f32>;`
- Add `@group(0) @binding(7) var<storage, read> k_bias: array<f32>;`
- In `attention_parallel` (Q path): after `q_sh[d] = gemm_row(...)`, add bias
  - Only when bias is present (flag in params, e.g. bit 17 of mask_active)
- In `write_kv_head` (K path): after `q_sh[d] = gemm_row(...)`, add bias
  - Only when bias is present

### Step 4: Add bias flag to AttentionGpuParams

Reuse bit 17 of `mask_active` (bit 16 is already rope_neox).
- `has_qk_bias = (mask_active & 0x20000) != 0`

### Step 5: Wire bias buffers in Rust

File: `gguf_bridge/init.rs`
- Add bias buffer slots to bind group layout (bindings 6, 7)
- Create a small bias staging buffer

File: `gguf_bridge/attention.rs`
- In `attention_gpu_params`: set bit 17 when Q/K bias tensors exist
- Pass bias tensor info through to the encode functions

File: `gguf_bridge/prefill_async.rs` / `mc8_wasm/encode.rs`
- Upload bias data and bind in `encode_attention_batched_q_prefill`
- Upload bias data and bind in `encode_attention_pass_gpu`

### Step 6: Native path

File: `gguf_bridge/attention.rs` (CPU attention)
- After Q projection (`stack_gemm_quant`), add bias before `rope_inplace`
- After K projection, add bias before `rope_inplace`

### Step 7: P64 compiler

File: `q42/p64_weight/compiler.rs`
- Add bias tensors to the planned list so they're included in P64 containers
- Or: handle bias as extra tensors with UNKNOWN role (they'd still be in the GGUF mmap)

## Key Questions to Verify

1. [x] Are Qwen2 bias tensors F32 in GGUF? (Expected: yes, biases are small)
2. [x] Does `find_layer_tensor` work with `.bias` suffix? (Should — it just builds `blk.{L}.attn_q.bias`)
3. [x] What's the bind group layout for the attention pipeline? (Bindings 0-5; adding 6,7)
4. [x] Is the bias applied per-head or per-element? (Per-element: shape = [n_head * head_dim])
5. [x] Does the decode path also need bias? (Yes — Q and K projections always have bias in Qwen2)

## Critical Edge Cases (from review)

1. **Dummy buffer for non-bias models**: WebGPU requires all declared bindings to
   be present in bind groups. Models without bias (Llama, SmolLM2) must bind a
   dummy 1-element f32 buffer at slots 6 and 7. The shader checks `has_qk_bias`
   flag (bit 17) before reading, so the dummy content doesn't matter.

2. **Application order**: Bias MUST be added after GEMM, before RoPE.
   Q = W_q @ x + b_q → RoPE(Q). Already planned correctly.

3. **RoPE base frequency**: Qwen2.5 uses theta = 1,000,000 (1e6). Verify GGUF
   reader parses `rope.freq_base` correctly. Already handled via
   `effective_rope_freq_base()`.

4. **RMS norm epsilon**: Qwen2.5 uses 1e-6, not 1e-5. Currently hardcoded as
   `RMS_NORM_EPS = 1e-5`. May need to fix after bias is confirmed working.

5. **Tied embeddings**: Qwen2.5-0.5B/1.5B may use `tie_word_embeddings=true`.
   Verify `logits_projection_info()` falls back to `token_embd.weight`.

6. **WGSL Forge**: We have a shader forge for deterministic generation/validation.
   Use it to verify the modified shader compiles correctly.

## Risk Assessment

- **Non-Qwen models**: No bias tensors → `find_layer_tensor` returns None → bit 17
  not set → shader skips bias addition → zero behavior change
- **Struct size**: No change to `AttentionGpuParams` (reusing mask_active bits)
- **Bind group layout**: Adding bindings 6,7 — need to check if this breaks
  existing bind group creation

## Order of Implementation

1. Verify assumptions (questions above)
2. Add `attn_q_bias`/`attn_k_bias` to `LayerTensors` + `get_layer_tensors`
3. Add bias flag to `attention_gpu_params`
4. Add bias bindings to attention shader + bind group layout
5. Wire bias upload in WASM encode paths
6. Add bias to native CPU attention path
7. Update P64 compiler to include bias tensors
8. Rebuild WASM, test Qwen2.5
