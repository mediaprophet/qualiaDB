// Engine field ownership; moved mechanically from the parent module.
use super::*;

pub struct QTensorEngine {
    /// Private prepared backing retains lease exclusion or immutable snapshot
    /// ownership. Public legacy mappings cannot override this execution source.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) prepared_native_bytes: Option<ModelByteHandle>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) prepared_native_sources: Option<NativeModelSources>,
    /// Architecture-owned Qwen3.5 state; generic Llama execution never admits it.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) qwen35_ftw: Option<qwen35::ftw::session::Session>,
    /// Browser WASM keeps a private device; native reuses `gpu_context::shared_gpu()`.
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) device: etalon::Device,
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) queue: etalon::Queue,
    #[cfg(feature = "gpu-runtime")]
    pub pipeline: etalon::ComputePipeline,
    /// WASM multi-row Q8_0 GEMV (llama.cpp-style: 64 thr, 4 rows/WG, u32 packed reads).
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub mmv_q8_0_pipeline: etalon::ComputePipeline,
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) native_pipeline_cache: Option<etalon::PipelineCache>,
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) pipeline_bind_layout: etalon::BindGroupLayout,
    /// 0.0.21: cooperative GEMV (one workgroup per output row, shared-memory reduction). Same shader
    /// MODULE as `pipeline`, entry point `coop_gemv` / `coop_gemv_sg`. Selected per-call when
    /// `llm_bench::coop_gemv_enabled()`. Native only (the wasm decode path is the MC8 arena).
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(crate) coop_gemv_pipeline: etalon::ComputePipeline,
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) coop_gemv_bind_layout: etalon::BindGroupLayout,
    /// Multi-row coop GEMV (8 rows/WG) for Q4_K_SOA large n_out — see `coop_gemv_mr`.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(crate) coop_gemv_mr_pipeline: etalon::ComputePipeline,
    /// GEMV + residual add in one dispatch (O-proj / down-proj in resident mega-pass).
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(crate) coop_gemv_residual_pipeline: etalon::ComputePipeline,
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) coop_gemv_residual_bind_layout: etalon::BindGroupLayout,
    /// Multi-row residual GEMV for Q4_K_SOA.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(crate) coop_gemv_residual_mr_pipeline: etalon::ComputePipeline,
    /// Warp GEMV (32 thr/row) for Q4_K_SOA.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(crate) coop_gemv_warp_pipeline: etalon::ComputePipeline,
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(crate) coop_gemv_residual_warp_pipeline: etalon::ComputePipeline,
    /// Legacy f32×f32 mock block for offset-0 `QTensor` fallback (no mmap).
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) mock_pipeline: etalon::ComputePipeline,
    /// GPU-side Q6_K embedding dequant + matmul (zero CPU dequant).
    #[cfg(feature = "gpu-runtime")]
    pub embedding_pipeline: etalon::ComputePipeline,
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) embedding_bind_layout: etalon::BindGroupLayout,
    pub is_initialized: bool,
    /// DirectML device — Some on Windows when DirectML 1.15 is linked.
    #[cfg(target_os = "windows")]
    pub dml: Option<crate::directml_bridge::DmlDevice>,
    /// Memory-mapped GGUF file (set after `load_gguf`).
    #[cfg(not(target_arch = "wasm32"))]
    pub gguf_mmap: Option<Arc<memmap2::Mmap>>,
    #[cfg(target_arch = "wasm32")]
    pub gguf_mmap: Option<Arc<[u8]>>,
    /// WASM: cached tokenizer extracted from gguf_mmap before dropping it.
    #[cfg(target_arch = "wasm32")]
    pub cached_tokenizer: Option<crate::gguf_sharder::GgufTokenizer>,
    /// WASM: cached tensor index extracted from gguf_mmap before dropping it.
    #[cfg(target_arch = "wasm32")]
    pub cached_tensor_index: Option<crate::gguf_sharder::GgufTensorIndex>,
    /// WASM: raw bytes of the token_embd tensor (for embedding lookup after dropping gguf_mmap).
    #[cfg(target_arch = "wasm32")]
    pub cached_token_embd: Option<Arc<[u8]>>,
    /// Resident P64 container bytes.
    #[cfg(target_arch = "wasm32")]
    pub p64_resident: Option<Arc<[u8]>>,
    /// Cached P64 index after `adopt_resident_p64_*` — decode must not re-CRC the container.
    #[cfg(not(target_arch = "wasm32"))]
    pub p64_index: Option<crate::p64_weight::P64TensorIndex>,
    /// Cached synthetic GGUF index built from `p64_index` (or from GGUF parse).
    #[cfg(not(target_arch = "wasm32"))]
    pub tensor_index_cache: Option<crate::gguf_sharder::GgufTensorIndex>,
    /// FTW multi-shard package for native MoE execution.
    #[cfg(not(target_arch = "wasm32"))]
    pub ftw_package: Option<Arc<crate::inference::moe::ftw_loader::FtwModelPackage>>,

    /// Byte offset into the mmap where tensor data begins.
    pub tensor_data_offset: u64,
    pub hyperparams: crate::gguf_sharder::GgufHyperparams,
    pub max_tensor_bytes: usize,
    /// Reused layer staging buffers (one layer in VRAM at a time).
    #[cfg(feature = "gpu-runtime")]
    pub(super) gemm_input_buf: Option<etalon::Buffer>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) gemm_weight_buf: Option<etalon::Buffer>,
    /// MC8 Part 3t: disjoint per-role weight arena (prefill single-submit).
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) mc8_weight_arena: Option<Mc8WeightArenaBufs>,
    /// MC8 Part 3x: when set, the 7 role buffers hold ALL layers' weights (uploaded once);
    /// hot-path encoders bind a per-layer sub-range instead of re-`write_buffer`ing per forward.
    #[cfg(target_arch = "wasm32")]
    pub(super) mc8_weights_resident: bool,
    /// Per-role 256-byte-aligned per-layer stride (bytes), indexed by `Mc8WeightRole::idx()`.
    #[cfg(target_arch = "wasm32")]
    pub(super) mc8_weight_role_stride: [u64; 7],
    /// Legacy decode-path ping-pong (decode tail not on weight arena yet).
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) gemm_weight_buf_b: Option<etalon::Buffer>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) gemm_output_buf: Option<etalon::Buffer>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) gemm_params_buf: Option<etalon::Buffer>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) gemm_output_staging: Option<etalon::Buffer>,
    // A1a (STELLAR §A): persistent GPU top-k output-projection pipeline + small candidate buffers.
    // Lets the output logits stay on-GPU (top-k over them, read back only K pairs) instead of the
    // 196 KB/token full-logit readback. Created once in `ensure_gemm_buffers`.
    #[cfg(feature = "gpu-runtime")]
    pub(super) output_topk_pipeline: Option<etalon::ComputePipeline>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) output_topk_bind_layout: Option<etalon::BindGroupLayout>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) topk_cand_val_buf: Option<etalon::Buffer>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) topk_cand_idx_buf: Option<etalon::Buffer>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) topk_cand_staging: Option<etalon::Buffer>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) topk_params_buf: Option<etalon::Buffer>,
    /// MC8 FFN / attention scratch (gate, up, o_proj).
    #[cfg(feature = "gpu-runtime")]
    pub(super) gemm_aux_buf: Option<etalon::Buffer>,
    /// MC8 SwiGLU up-projection scratch (cannot alias gemm_output/work — in-place GEMM invalid).
    #[cfg(feature = "gpu-runtime")]
    pub(super) gemm_ffn_buf: Option<etalon::Buffer>,
    /// Batched prefill RMS output (same span as `gemm_input_buf`; avoids in-place on batch_buf).
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) prefill_scratch_buf: Option<etalon::Buffer>,
    /// Strided prefill ping-pong rows (`PREFILL_CHUNK_SIZE × row_stride` floats each).
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) prefill_work_buf_a: Option<etalon::Buffer>,
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) prefill_work_buf_b: Option<etalon::Buffer>,
    /// Phase 5.5: Q/K/V projection scratch (parallel-GEMM output → lightweight attention shader).
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) mc8_q_proj_buf: Option<etalon::Buffer>,
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) mc8_k_proj_buf: Option<etalon::Buffer>,
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) mc8_v_proj_buf: Option<etalon::Buffer>,
    pub(super) gemm_max_out_dim: u32,
    pub(super) gemm_max_input_floats: usize,
    /// Static KV ring-buffer (allocated once at `load_gguf`).
    pub(super) kv_layout: Option<KvCacheLayout>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) kv_cache_gpu: Option<etalon::Buffer>,
    /// CPU mirror for quantized-attention fallback (no growth during decode).
    pub(super) kv_cache_cpu: Option<Box<[f32]>>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) attention_pipeline: etalon::ComputePipeline,
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) attention_bind_layout: etalon::BindGroupLayout,
    #[cfg(feature = "gpu-runtime")]
    pub(super) attention_params_buf: Option<etalon::Buffer>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) attention_mask_buf: Option<etalon::Buffer>,
    /// MC8 elementwise GPU ops (RMSNorm / SiLU×mul / residual).
    #[cfg(feature = "gpu-runtime")]
    pub(super) elem_rms_norm_pipeline: etalon::ComputePipeline,
    #[cfg(feature = "gpu-runtime")]
    pub(super) elem_silu_mul_pipeline: etalon::ComputePipeline,
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) elem_silu_mul_bind_layout: etalon::BindGroupLayout,
    #[cfg(feature = "gpu-runtime")]
    pub(super) elem_add_residual_pipeline: etalon::ComputePipeline,
    #[cfg(feature = "gpu-runtime")]
    pub(super) elem_params_buf: Option<etalon::Buffer>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) norm_weight_buf: Option<etalon::Buffer>,
    /// MC8 Part 3s: dynamic-offset bind group layouts (uniform race elimination).
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) mc8_gemm_bind_layout: etalon::BindGroupLayout,
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) mc8_elem_bind_layout: etalon::BindGroupLayout,
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) mc8_attn_bind_layout: etalon::BindGroupLayout,
    /// Phase 5 dispatch fusion: SwiGLU expansion (gate · SiLU · up) collapsed into one pass.
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) mc8_ffn_fused_bind_layout: etalon::BindGroupLayout,
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) mc8_ffn_fused_pipeline: etalon::ComputePipeline,
    /// Native T-A1: same fused FFN expansion, static uniform (resident mega-pass).
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) ffn_fused_bind_layout: etalon::BindGroupLayout,
    /// Naive 64-thread/row fused expansion (wasm-style; fallback).
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) ffn_fused_pipeline: etalon::ComputePipeline,
    /// T-A1b: coop 256-thread/row fused expansion (preferred when coop GEMV is on).
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) ffn_fused_coop_pipeline: etalon::ComputePipeline,
    /// Multi-row fused FFN (4 rows/WG) for Q4_K_SOA.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) ffn_fused_mr_pipeline: etalon::ComputePipeline,
    /// Warp fused FFN (32 thr/row) for Q4_K_SOA.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) ffn_fused_warp_pipeline: etalon::ComputePipeline,
    /// Dual K+V GEMV (shared act) for resident mega-pass.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) dual_gemv_pipeline: etalon::ComputePipeline,
    /// Dual multi-row (4 rows/WG) — default for SoA K+V.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) dual_gemv_mr_pipeline: etalon::ComputePipeline,
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) dual_gemv_bind_layout: etalon::BindGroupLayout,
    /// Triple Q+K+V GEMV (shared act, GQA-safe) — one dispatch replaces dual+Q.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) triple_gemv_pipeline: etalon::ComputePipeline,
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) triple_gemv_bind_layout: etalon::BindGroupLayout,
    /// Phase 5.3: the output/logits projection (tied `token_embd`, ~50 MB) uploaded to VRAM
    /// once at init so the per-token argmax binds resident sub-ranges instead of re-uploading
    /// the whole matrix every token (the decode throughput killer). A1a step-2 ports this to the
    /// native top-k decode path, so these two fields are available on both targets.
    #[cfg(feature = "gpu-runtime")]
    pub(super) mc8_logits_resident_buf: Option<etalon::Buffer>,
    pub(super) mc8_logits_row_bytes: u32,
    /// A1b (STELLAR §A): resident 2-bit ternary-FFN GEMM dispatcher, built once at P64 boot from
    /// the container's base-3 FFN blobs (rebaked to 2-bit, uploaded once). `None` until a ternary
    /// P64 is adopted; the FFN dispatch branch (`dispatch_ternary_ffn`) uses it when present +
    /// the toggle is on, else the CPU oracle. Native-only; the wasm ternary path is a later step.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) ternary_ffn: Option<crate::ternary_gpu::TernaryFfnResident>,
    /// Bounded cold resident projection cache. Retains immutable source ownership, validates
    /// device and byte-region length, and invalidates model replacement before dropping sources.
    /// Same leased source is reused across requests; no per-request epoch or full-weight hashing.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) projection_cold_byte_budget: usize,
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) projection_readback: etalon_inference::projections::readback::ReadbackCompletion,
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) gemm_resident_weights: std::sync::Mutex<super::gemm::ResidentProjectionCache>,
    /// Phase 3 (FFN fusion): a small uniform buffer holding the gate/up/down GEMM `GemmGpuParams`
    /// at 256-aligned sub-ranges (3 slots), so all three GEMMs of one fused FFN submit can bind
    /// distinct params simultaneously. Lazily created native-only on the first fused FFN.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) ffn_fused_params: Option<etalon::Buffer>,
    /// Native attention preproject fusion: two 256-byte-aligned GEMM uniform slots
    /// (K,V) and two attention uniform slots (K-write,V-write), allowing K/V
    /// projection + KV-cache writes to share one submit without uniform races.
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) attention_kv_gemm_params: Option<etalon::Buffer>,
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) attention_kv_params: Option<etalon::Buffer>,
    /// Phase 5.4: all layers' attn_norm + ffn_norm weights resident (slot 2L = attn, 2L+1 = ffn),
    /// so RMSNorm binds a per-layer sub-range instead of re-`write_buffer`ing a shared single-layer
    /// `norm_weight_buf` every layer (the second per-layer write_buffer race blocking single-submit).
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) mc8_norm_resident_buf: Option<etalon::Buffer>,
    #[cfg(target_arch = "wasm32")]
    pub(super) mc8_norm_stride: u32,
    /// Native GPU-resident single-fence decode plan (see `resident_decode.rs`).
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) resident_decode: resident_decode::ResidentDecodeState,
    /// Cold-built host descriptor for the native CUDA all-layer plan.
    #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
    pub(super) cuda_decode_plan: cuda_decode_plan::CudaDecodePlanState,
    /// W3: native GPU-resident single-fence-per-chunk prefill plan (see `prefill_arena.rs`).
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) prefill_arena: prefill_arena::PrefillArenaState,
    /// W6a: batched speculative-verify forward plan (per-position argmax; see `verify_arena.rs`).
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    pub(super) verify_arena: verify_arena::VerifyArenaState,
    /// Bind group cache: eliminates per-token `create_bind_group` calls by caching
    /// bind groups keyed on (buffer addresses, offsets, weight role, layer). Bind groups
    /// are identical across tokens for the same layer/op since only dynamic uniform
    /// offsets change — those are passed at `set_bind_group` time, not baked into the BG.
    #[cfg(all(target_arch = "wasm32", feature = "gpu-runtime"))]
    pub(super) mc8_bg_cache: std::sync::Mutex<std::collections::HashMap<u64, etalon::BindGroup>>,
    /// Flat recurrent-state arena for all hybrid SSM layers (Granite Hybrid / Qwen GatedDeltaNet).
    ///
    /// Allocated once by `ssm_forward::init_ssm_state` after model load; empty (len=0) for
    /// pure-attention models.  During the hot decode loop the arena is accessed as a
    /// `&mut [f32]` slice — no per-token Vec allocation (Tier-1 zero-heap).
    pub(crate) ssm_recurrent_state: Box<[f32]>,
    pub(crate) ssm_state_byte_budget: usize,
}
