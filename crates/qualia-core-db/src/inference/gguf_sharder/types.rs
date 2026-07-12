//! Plain data types shared across the GGUF parsing modules.

/// Shape + type + offset for one tensor parsed from the GGUF tensor-info section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GgufTensorInfo {
    /// Tensor shape (up to 4 dimensions; extra dims truncated).
    pub dims: [u64; 4],
    pub n_dims: u32,
    /// GGML element type: 0=F32, 1=F16, 8=Q8_0, 12=Q4_K, …
    pub ggml_type: u32,
    /// Byte offset of this tensor's data within the tensor data block.
    pub byte_offset: u64,
}

/// Per-layer transformer weight metadata (all `Option` — absent tensors skipped).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LayerTensors {
    pub attn_norm: Option<GgufTensorInfo>,
    pub attn_q: Option<GgufTensorInfo>,
    pub attn_k: Option<GgufTensorInfo>,
    pub attn_v: Option<GgufTensorInfo>,
    pub attn_output: Option<GgufTensorInfo>,
    pub ffn_norm: Option<GgufTensorInfo>,
    pub ffn_gate: Option<GgufTensorInfo>,
    pub ffn_up: Option<GgufTensorInfo>,
    pub ffn_down: Option<GgufTensorInfo>,
}
