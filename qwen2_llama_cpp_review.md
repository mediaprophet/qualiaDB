# Qwen2 llama.cpp Implementation Review

## Source Files Reviewed
- `src/models/qwen2.cpp` — model load + graph build
- `src/llama-graph.cpp` — `build_qkv`, `build_attn`, `build_attn_mha`, `build_norm`, `build_ffn`
- `src/llama-model.cpp` — `create_tensor_qkv`, rope type mapping
- `src/llama-hparams.cpp` — `n_rot()`, rope type constants
- `ggml/src/ggml-cpu/ops.cpp` — `ggml_compute_forward_rope_flt` (CPU RoPE implementation)
- `ggml/include/ggml.h` — RoPE type constants

## Qwen2 Computational Graph (from qwen2.cpp)

```
for each layer il:
  inpSA = inpL

  // RMSNorm (attn)
  cur = rms_norm(inpL, attn_norm_w, eps=f_norm_rms_eps)

  // QKV projection + bias + RoPE
  [Qcur, Kcur, Vcur] = build_qkv(layer, cur, n_embd_head, n_head, n_head_kv)
    // separate Q/K/V path (not fused QKV):
    Qcur = mm(wq, cur) + wq_b    // bias added AFTER projection
    Kcur = mm(wk, cur) + wk_b    // bias added AFTER projection
    Vcur = mm(wv, cur)            // no V bias for Qwen2
    // reshape to [n_embd_head, n_head, n_tokens]

  Qcur = rope_ext(Qcur, pos, n_rot=head_dim, type=NEOX, freq_base, freq_scale)
  Kcur = rope_ext(Kcur, pos, n_rot=head_dim, type=NEOX, freq_base, freq_scale)
  // Vcur: no RoPE

  // Attention
  cur = build_attn(wo, wo_b, Qcur, Kcur, Vcur, scale=1/sqrt(head_dim))
    // kq = mm(k, q)  -- [n_kv_head, n_head_kv, n_tokens, n_tokens]
    // softmax(kq, mask, scale)
    // kqv = mm(v, kq)
    // cur = mm(wo, kqv) + wo_b  (output projection + bias)

  // Residual
  ffn_inp = cur + inpSA

  // RMSNorm (ffn)
  cur = rms_norm(ffn_inp, ffn_norm_w, eps=f_norm_rms_eps)

  // FFN (SiLU + parallel)
  cur = silu(mm(gate, cur)) * mm(up, cur)
  cur = mm(down, cur)

  // Residual
  cur = cur + ffn_inp
  inpL = cur

// Final norm
cur = rms_norm(inpL, output_norm_w, eps=f_norm_rms_eps)

// LM head
logits = mm(output, cur) + output_b  (if output_b exists)
```

## Key Findings

### 1. RMS Norm Epsilon
- **GGUF key**: `attention.layer_norm_rms_eps`
- **Qwen2.5 value**: `1e-6`
- **Our code**: Was hardcoded to `1e-5` everywhere
- **Fix applied**: Added `rms_norm_eps` field to `GgufHyperparams`, parse from GGUF,
  architecture-based fallback (1e-6 for Qwen2, 1e-5 for others)
- **P64 issue**: P64 header doesn't store eps → falls back to arch-based default

### 2. Q/K Bias
- **Tensor names**: `blk.N.attn_q.bias`, `blk.N.attn_k.bias`
- **Application**: `ggml_add(ctx, Qcur, wq_b)` AFTER projection, BEFORE RoPE
- **V bias**: Qwen2 does NOT have V bias
- **Our implementation**: Correct order (bias after GEMM, before RoPE)
- **P64 issue**: P64 format does NOT include bias tensors. Bias data is lost during
  P64 conversion. The `qk_bias_buf` stays zero-filled, which is a no-op (adding zero).
  This is correct for P64-loaded models but means bias is not applied.

### 3. RoPE
- **Type**: `LLAMA_ROPE_TYPE_NEOX` (= 2 = `GGML_ROPE_TYPE_NEOX`)
- **n_rot**: Defaults to `n_embd_head_k` (= head_dim = 64 for Qwen2.5-0.5B)
- **NeoX layout**: Pairs are (i, i+half_dim) for i in 0..half_dim
  - `rotate_pairs(n_dims, n_dims/2, cache, src, dst)` with scale=2
  - `ic = i0/2`, `src[ic]` and `src[ic + n_dims/2]` are the pair
  - `dst[0] = x0*cos - x1*sin`, `dst[n_offset] = x0*sin + x1*cos`
- **Our implementation**: Matches — `(q_sh[i], q_sh[i+half_dim])` pairs
- **freq_base**: Qwen2.5 uses 1,000,000.0 (confirmed in our log output)

### 4. Attention Scale
- **llama.cpp**: `1.0f / sqrtf(float(n_embd_head))` = 1/sqrt(64) = 0.125
- **Our shader**: `1.0 / sqrt(f32(params.head_dim))` — same

### 5. QKV Weight Layout
- **llama.cpp**: Separate Q, K, V weights (not fused QKV) for Qwen2
  - `wq`: [n_embd, n_embd_q] = [896, 896]
  - `wk`: [n_embd, n_embd_kv] = [896, 128]  (GQA: 2 kv heads × 64)
  - `wv`: [n_embd, n_embd_kv] = [896, 128]
  - `wo`: [n_embd, n_embd] = [896, 896]
- **Our code**: Same separate Q/K/V tensors

### 6. GQA (Grouped Query Attention)
- **Qwen2.5-0.5B**: n_head=14, n_kv_head=2, head_dim=64
- **q_heads_per_kv**: 14/2 = 7
- **Our code**: Handles GQA correctly via `kv_head` mapping

### 7. FFN
- **Type**: SiLU parallel (gate * up → down)
- **llama.cpp**: `LLM_FFN_SILU, LLM_FFN_PAR`
- **Our code**: `ELEM_OP_SILU_MUL` then GEMM down — matches

### 8. Output Projection
- **llama.cpp**: `cur = mm(wo, kqv)` then `cur = cur + wo_b` (if wo_b exists)
- **Qwen2**: wo_b does NOT exist (no output bias in Qwen2)
- **Our code**: Handles via `tensors.attn_output` GEMM + residual add

### 9. Token Embedding
- **llama.cpp**: `tok_embd` weight is used directly
- **Qwen2**: `output` may be tied to `tok_embd` (TENSOR_NOT_REQUIRED → TENSOR_DUPLICATED)
- **Our code**: Handles via `output_weight_info()` fallback

## Potential Issues to Investigate

### A. RMS Norm Eps (FIXED)
- Was using 1e-5 instead of 1e-6 for Qwen2.5
- Fixed by adding `effective_rms_norm_eps()` with arch-based fallback
- Need to verify the fix works in WASM build

### B. P64 Bias Tensors (NOT the root cause)
- P64 doesn't include bias tensors, but zero bias is a no-op
- If model was loaded from raw GGUF (not P64), bias would be applied
- For P64 path: bias is correctly skipped (zero-filled buffer)

### C. RoPE Frequency Base
- Confirmed: 1,000,000.0 in our log output
- Matches Qwen2.5 spec

### D. CPU Fallback Path
- The `cpu_attention_pass` function also uses `RMS_NORM_EPS` (hardcoded 1e-5)
- This is used as a reference/fallback — may need updating too

### E. Native Path (not WASM)
- Native path still uses hardcoded `RMS_NORM_EPS` in many places
- Not part of current task but should be updated for consistency

## Summary of Changes Applied

1. **`hyperparams.rs`**: Added `rms_norm_eps` field + `effective_rms_norm_eps()` method
   with architecture-based fallback (1e-6 for Qwen2, 1e-5 for others)

2. **`tensor_index.rs`**: Parse `attention.layer_norm_rms_eps` from GGUF KV metadata
   (float32 vtype=6 or float64 vtype=12)

3. **`p64_weight/reader.rs`**: Added `rms_norm_eps: 0.0` to `hyperparams()` method
   (P64 doesn't store eps; falls back to arch-based default)

4. **`mc8_wasm/encode.rs`**: Use `self.hyperparams.effective_rms_norm_eps()` instead
   of hardcoded `RMS_NORM_EPS` in `encode_elem`

5. **`mc8_wasm/params.rs`**: Added `eps` parameter to `mc8_elem_params()`

6. **`prefill_async.rs`**: Pass per-model eps to all `mc8_elem_params()` calls

## Next Steps

1. Rebuild WASM and test with Qwen2.5-0.5B
2. If still garbage, investigate:
   - Whether the eps fix is actually taking effect in the WASM build
   - Whether the P64 conversion preserves the correct rope_freq_base
   - Whether the attention output projection is working correctly
   - Whether the FFN SiLU path is correct
   - Whether the token embedding lookup is correct for Qwen2
3. Consider adding debug logging for layer-0 single-token output
