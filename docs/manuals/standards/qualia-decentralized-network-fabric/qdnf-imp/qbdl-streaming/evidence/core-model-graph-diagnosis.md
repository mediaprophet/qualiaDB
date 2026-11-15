# Core model graph failure diagnosis and repair handoff

Source baseline: QualiaDB HEAD afbe431195167fde0fadc233ed79fd2fe85f73f0 (see baseline-revisions.json). Numerical Etalon source remains pinned to 6ace6cc6413a87ca10a1da58397ddc7c0b2fc41e. The newer C:/github/etalon ordinary/ssm_qwen.rs is NOT the compiled source. Root's separate model-io path patch does not upgrade numerical kernels.

## Evidence, not qualification

This agent inspected source, read bounded GGUF headers, wrote focused adapter/tests and ran rustfmt. No build or test was run by this agent. Root's frozen qwen35-debug-decode.log confirms layer30 leaves ran=30; layer31 yields ran=31; expected32 fails in364.79s. The new adapter is unrouted while core94855 builds; its tests are pending.

The original model smoke tests were extracted unchanged into gguf_bridge/gemma4_qwen35/tests.rs. Their full mapped-model to_vec was replaced with the existing retained ModelByteHandle owner. Assertions, models and generated-token counts remain unchanged.

## Dense Qwen3.5 actual cause

Installed Qwen3.5-4B-Q4_K_M.gguf: 32 layers, hidden2560, FFN9216; full attention every4th layer, Q heads16/KV4, head_dim256 and partial RoPE64, epsilon1e-6, base1e7. Linear geometry: conv4, key_heads16, value_heads32, key/value dimension128, QKV8192, value4096.

Pinned ordinary/ssm_layout.rs allocates536576 floats per linear layer from inner4096/state128/head256/groups16. Its legacy ssm_qwen execution instead infers inner4 from conv dims[4,8192], derives10 heads from alpha dims[2560,32], and requires655372 floats. Early layers overlap following storage; last linear layer30 has only536576 remaining. Correct GGUF history+delta geometry is3*8192+128*128*32=548864, with24 independent linear layers. Increasing the old arena would retain the wrong recurrence and head mapping.

Existing gated full attention consumes f32 KV, but default allocation selected int8+scale. Moreover the pinned generic layer returns FFN success even after attention failure. The generic parsed wrapper drops rope_dim and rms_norm_eps. The dense owner therefore validates complete physical roles, preserves exact partial RoPE/epsilon, selects f32 KV under the existing cache budget, and counts a token only if every attention and FFN step succeeds.

New files: gguf_bridge/qwen35/gguf/{mod,metadata,plan,linear,session,bridge,tests}.rs. Each is below500 lines. They reuse existing ModelByteHandle, GgufTensorIndex, bounded Etalon GGUF MetadataView/KvCursor, QTensorEngine quantized GEMM, existing gated full attention, and pinned recurrent::{normalize_qwen_qk,qwen_gates,step_qwen_delta,gated_rms_norm,silu_into,gated_deltanet::step_causal_conv1d}. No dependency revision change and no new numerical primitive.

Physical profile: llama.cpp dense qwen35 GGUF, folded ordinary/QK norms, negative exported A=-exp(A_log), tiled value-head order; text-only positions make all MRoPE axes equal. Unknown graph tensors, duplicate keys/roles, unsupported quantization, scaled RoPE/Hadamard transforms, nonfinite scalar weights and out-of-bounds payloads refuse. Full attention beyond the reserved context refuses instead of silently changing to a truncated ring. Recurrent state/workspace/retained-directory allocation is charged to the existing caller state budget; f32 KV uses the existing separate cache cap and resource admission. The immutable Rust borrow/retained Arc is source identity, not an additional promise that an externally writable legacy mmap file is immutable.

The exporter semantics were checked against the primary [llama.cpp converter](https://github.com/ggml-org/llama.cpp/blob/master/conversion/qwen.py): Qwen3.5 exports V heads in tiled order, folds ordinary norms, and exports negative exponential A. This adapter uses canonical primitives with explicit GGUF packing; it does not relabel the pinned HF/FTW roles as quantized GGUF roles.

Six authored regressions cover complete32-layer execution/reset, last-state bounds and tiled head mapping, failed attention refusing logits, source/directory/position mismatch, missing/duplicate role/budget refusal, and exact metadata/duplicate/scaled-RoPE refusal. Full numerical qualification still needs root's actual-source compilation, regressions and real4B/0.8B smoke selectors; a complete external-logit oracle is not claimed.

## Exact parent routing (not applied during core94855)

1. qwen35/mod.rs: add native-only pub(crate) mod gguf.
2. state.rs: add native-only qwen35_gguf: Option<qwen35::gguf::Session> and qwen35_gguf_required: bool beside qwen35_ftw; initialize None/false in both init.rs constructors. The required marker survives an unwind after taking the session, preventing generic fallback.
3. load_gguf_checked: make index mutable. Before generic decode_supported, call qwen35::gguf::Profile::parse(&mmap,&mut index)?; Some(profile) returns self.load_dense_qwen_gguf(mmap,index,profile). Leave the general guard unchanged. At all five existing model replacement blocks clearing qwen35_ftw, clear the new owner and marker. moe_ffn.rs FTW adoption also clears both immediately before assigning qwen35_ftw.
4. Both ensure_kv_cache implementations: native layout candidate is self.dense_qwen_kv_layout(h); WASM candidate remains KvCacheLayout::from_hyperparams(h). Keep the existing GPU/CPU resource admission and allocation logic.
5. dispatch_transformer_layer native entry: if required, return dense_qwen_layer(index,layer,token_idx,hidden,emb_dim,scratch_a,scratch_b). dispatch_transformer_forward native entry similarly returns dense_qwen_forward(index,hidden,emb_dim,token_idx,max_layers,scratch_a,scratch_b).
6. dispatch_prefill_chunk rejects dense-required with the existing Qwen-MoE Layer{layer:0,stage:DispatchQueryFfn} error; this owner is token-major. Both native try_cuda_mega_pass_decode entry points return None for dense-required, since their prepared CUDA graph has another contract.
7. reset_recurrent_state resets the new session. The legacy dispatch_hybrid_ssm_layer returns false for dense-required; it cannot bypass complete-layer ordering. Legacy snapshot_recurrent_state_into/restore_recurrent_state_from_slice return None/false for dense-required until an architecture snapshot format exists.
8. apply_output_norm_inplace routes dense-required to dense_qwen_output_norm. dispatch_output_argmax_chunked rejects dense-required unless dense_qwen_ready(index), then reuses the existing projection. Native GPU top1 returns None for dense-required so unreviewed execution cannot bypass the owner.

## Gemma: concrete remaining graph integration

The two installed files were read only through32MiB header buffers:

- E4B:42 layers, hidden2560, FFN10240,8 Q/2 KV heads, SWA head_dim256/global512, SWA512,18 shared-KV layers, PLE256. Explicit sliding pattern [1,1,1,1,1,0] repeated. RoPE bases1e4/1e6, dimensions256/512, epsilon1e-6, final softcap30.
- 26B-A4B:30 layers, hidden2816, FFN2112,16 Q heads, KV head_count is an ARRAY [8,8,8,8,8,2] repeated, SWA head_dim256/global512, SWA1024, no shared KV/PLE,128 experts/top8, expert FFN704. Same6-layer sliding pattern and bases, softcap30.

Existing gemma4_qwen35.rs contains a real Gemma graph, but its scalar n_kv loses the26B per-layer array; the metadata wrapper zeros architecture extras; the graph treats compressed KV as floats. E4B's PLE hardcodes token_id=0 and caches in a thread-local64-slot Vec ring rather than the model/session owner. The current hidden-only forward entry has no actual token parameter, so it cannot soundly supply token-aware PLE. These are architecture metadata/state/composition defects, not evidence that Gemma algorithms do not exist.

Proposed next bounded scope (requires parent review before mutation): NEW gguf_bridge/gemma4/{mod,metadata,plan,session,ple,attention,ffn,bridge,tests}.rs; mechanically extract existing Gemma graph/helper implementation from gemma4_qwen35.rs into focused children before behavior changes. Preserve existing Qwen gated helper. Plan owns the explicit per-layer sliding/global mask, dimensions, KV counts, producer/shared-KV map, per-layer RoPE, FFN/MoE role validation and budgeted offsets. Session owns bounded PLE and current token/position; remove thread-local state from the admitted path. Reuse existing dequantize_ple_row_into, token embedding decoder, GELU/RMSNorm/GEMM/expert primitives. New token-aware architecture entry must receive the actual token or perform retained-owner embedding itself; smoke callers pass their already-known token while retaining all current numerical/layer assertions. Legacy hidden-only entry refuses PLE models. Admission occurs only after the exact graph is validated; do not change generic decode_supported into blanket Gemma permission. Per-layer f32 KV and shared-KV producer dependencies must be checked without multiplying max dimensions across every layer or silently reducing context. Numerical oracle tests must exercise distinct nonzero token IDs, two sessions/models interleaved, SWA/global boundaries, shared-KV producers and26B KV8/2 variation before real-model decode is claimed fixed.

The three Gemma failures remain unresolved in this wave. No assertion was weakened and no unsupported architecture was forced.
