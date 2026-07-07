//! GPU uniform-buffer parameter structs + elementwise op codes for the LLM WGSL kernels.
//! Split out of the monolithic `gguf_bridge` module (structural refactor; no behaviour change).
//! All `#[repr(C)]` + `bytemuck::Pod` so they upload directly via `write_buffer`.

/// Uniform block passed to `quantized_embedding.wgsl`.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct EmbeddingGpuParams {
    pub n_embd: u32,
    pub ggml_type: u32,
    pub n_output: u32,
    pub raw_byte_len: u32,
}

/// Uniform block passed to `fused_transformer.wgsl`.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct GemmGpuParams {
    pub n_in: u32,
    pub n_out: u32,
    pub weight_ggml_type: u32,
    pub weight_row_elems: u32,
    pub weight_byte_len: u32,
    pub n_batch: u32,
    pub in_row_stride: u32,
    pub out_row_stride: u32,
}

/// Uniform block passed to `fused_attention.wgsl`.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct AttentionGpuParams {
    pub n_embd: u32,
    pub n_head: u32,
    pub n_kv_head: u32,
    pub head_dim: u32,
    pub q_heads_per_kv: u32,
    pub token_idx: u32,
    pub max_context: u32,
    pub layer_idx: u32,
    pub layer_stride: u32,
    pub slot_kv_elems: u32,
    pub weight_ggml_type: u32,
    pub weight_row_elems: u32,
    pub weight_byte_len: u32,
    pub proj_kind: u32,
    pub rope_theta_base: f32,
    pub rope_scale: f32,
    pub num_tokens_in_batch: u32,
    pub batch_start_token_idx: u32,
    pub mask_active: u32,
    pub mask_word_count: u32,
    pub out_stride_elems: u32,
    /// Phase 5.5: row stride (floats/token) of the PRE-COMPUTED Q/K/V projection bound at binding 0.
    /// Non-zero → the shader reads the projection directly (parallel GEMM did the matmul) instead of
    /// doing the per-element `gemm_row` matmul itself. 0 → legacy in-shader projection.
    pub proj_row_stride: u32,
    /// W5a: `1` ⇒ the KV cache is int8-quantized (packed i8 + per-(slot,kv_head) f32 scale, reusing
    /// the binding-3 buffer reinterpreted); `0` ⇒ the legacy f32 KV cache (byte-identical path).
    pub kv_quant: u32,
    /// W5b Phase 4b: `dict_k (low 16) | n_atoms (high 16)`. Low16 > 0 ⇒ the KV cache stores k-sparse
    /// dictionary codes (reconstructed in the shader from the `kv_atoms` binding). `0` ⇒ f32/int8.
    /// Doubles as the struct's 16-byte-alignment tail.
    pub dict_pack: u32,
}

/// Uniform block for `wasm_elementwise.wgsl` (MC8 GPU norm / SwiGLU / residual).
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ElemGpuParams {
    pub n: u32,
    pub batch: u32,
    pub op: u32,
    pub eps: f32,
    pub a_row_stride: u32,
    pub b_row_stride: u32,
    pub out_row_stride: u32,
    pub a_slot: u32,
    pub b_slot: u32,
    pub out_slot: u32,
    pub _pad: u32,
}

pub(crate) const ELEM_OP_RMS_NORM: u32 = 0;
pub(crate) const ELEM_OP_SILU_MUL: u32 = 1;
pub(crate) const ELEM_OP_ADD_RESIDUAL: u32 = 2;
/// CPU/native ReLU gate fallback (negative logits clamped before SiLU path).
pub(crate) const ELEM_OP_RELU: u32 = 3;

/// Map an elementwise opcode to its human-readable kernel label (logging / diagnostics).
pub(crate) fn elem_op_label(op: u32) -> &'static str {
    match op {
        ELEM_OP_RMS_NORM => "rms_norm",
        ELEM_OP_SILU_MUL => "silu_mul",
        ELEM_OP_ADD_RESIDUAL => "add_residual",
        ELEM_OP_RELU => "relu",
        _ => "unknown",
    }
}
