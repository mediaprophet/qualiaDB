//! Plain data types shared across the GGUF parsing modules.

use serde::{Deserialize, Serialize};

/// Fixed-capacity PLE n-gram metadata recorded by a Qwen4Exp GGUF.
///
/// The model stores the hash multipliers and each logical embedding head's
/// row range in its metadata. Retaining that contract is essential: deriving
/// a convenient hash at runtime would select different trained rows.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Qwen4ExpPleConfig {
    pub ngram_size: u32,
    pub heads_per_ngram: u32,
    pub eos_token_id: u32,
    pub embedding_row_width: u32,
    pub layer_count: u32,
    pub layers: [u32; 8],
    pub multiplier_count: u32,
    pub layer_multipliers: [i64; 8],
    pub head_count: u32,
    /// Current Qwen4Exp format uses 16 heads; 32 keeps this serializable in
    /// the project serde version while failing closed for larger variants.
    pub head_offsets: [i64; 32],
    pub head_vocab_sizes: [i64; 32],
}

impl Default for Qwen4ExpPleConfig {
    fn default() -> Self {
        Self {
            ngram_size: 0,
            heads_per_ngram: 0,
            eos_token_id: 0,
            embedding_row_width: 0,
            layer_count: 0,
            layers: [0; 8],
            multiplier_count: 0,
            layer_multipliers: [0; 8],
            head_count: 0,
            head_offsets: [0; 32],
            head_vocab_sizes: [0; 32],
        }
    }
}

impl Qwen4ExpPleConfig {
    /// True only when every value required to reproduce trained row IDs is present.
    pub fn is_complete(&self) -> bool {
        let required_heads = self
            .ngram_size
            .saturating_sub(1)
            .saturating_mul(self.heads_per_ngram);
        self.ngram_size >= 2
            && self.multiplier_count >= self.ngram_size
            && required_heads != 0
            && self.head_count >= required_heads
            && required_heads <= self.head_offsets.len() as u32
            && self.embedding_row_width != 0
            && self.head_vocab_sizes[..required_heads as usize]
                .iter()
                .all(|&size| size > 0)
    }
}

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

impl GgufTensorInfo {
    /// Return a zero-copy view over a contiguous row range of a matrix tensor.
    ///
    /// GGUF stores fused QKV as consecutive projection rows.  This view adjusts the relative
    /// byte offset and row count without materialising or dequantising the full fused tensor.
    /// It deliberately rejects tensors with batch dimensions: expert tensors are selected by
    /// their dedicated 3-D evaluator rather than pretending they are ordinary projections.
    pub fn row_range(self, first_row: usize, row_count: usize) -> Option<Self> {
        if self.n_dims != 2 || row_count == 0 {
            return None;
        }
        let rows = self.dims[1] as usize;
        if first_row.checked_add(row_count)? > rows {
            return None;
        }
        let row_bytes = crate::ggml_quants::ggml_row_bytes(self.ggml_type, self.dims[0] as usize)?;
        let byte_offset = self
            .byte_offset
            .checked_add(first_row.checked_mul(row_bytes)? as u64)?;
        let mut dims = self.dims;
        dims[1] = row_count as u64;
        Some(Self {
            dims,
            n_dims: self.n_dims,
            ggml_type: self.ggml_type,
            byte_offset,
        })
    }
}

/// Per-layer transformer weight metadata (all `Option` — absent tensors skipped).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LayerTensors {
    pub layer_idx: u32,
    pub attn_norm: Option<GgufTensorInfo>,
    pub attn_q: Option<GgufTensorInfo>,
    pub attn_k: Option<GgufTensorInfo>,
    pub attn_v: Option<GgufTensorInfo>,
    /// Per-head Qwen4Exp QSA RMSNorm weights (256 values = 2×128 heads).
    pub attn_q_norm: Option<GgufTensorInfo>,
    pub attn_k_norm: Option<GgufTensorInfo>,
    /// Qwen hybrid models store Q/K/V as one row-concatenated matrix.
    pub attn_qkv: Option<GgufTensorInfo>,
    /// Qwen gated-attention scalar/vector gate.
    pub attn_gate: Option<GgufTensorInfo>,
    pub attn_output: Option<GgufTensorInfo>,
    pub ffn_norm: Option<GgufTensorInfo>,
    pub ffn_gate: Option<GgufTensorInfo>,
    pub ffn_up: Option<GgufTensorInfo>,
    pub ffn_down: Option<GgufTensorInfo>,
    // MoE (Mixture of Experts) extensions
    pub moe_router: Option<GgufTensorInfo>,
    pub moe_gate_exps: Option<GgufTensorInfo>,
    pub moe_up_exps: Option<GgufTensorInfo>,
    /// Gemma MoE packs gate and up rows consecutively in one expert tensor.
    pub moe_gate_up_exps: Option<GgufTensorInfo>,
    pub moe_down_exps: Option<GgufTensorInfo>,
    pub moe_shared_gate: Option<GgufTensorInfo>,
    pub moe_shared_up: Option<GgufTensorInfo>,
    pub moe_shared_down: Option<GgufTensorInfo>,
    /// Scalar gate applied to the shared-expert output in Qwen-style MoE.
    pub moe_shared_gate_input: Option<GgufTensorInfo>,
    // Qwen GatedDeltaNet / hybrid SSM state tensors.
    pub ssm_alpha: Option<GgufTensorInfo>,
    pub ssm_beta: Option<GgufTensorInfo>,
    /// Granite hybrid input projection; Qwen uses `attn_qkv` instead.
    pub ssm_in: Option<GgufTensorInfo>,
    pub ssm_conv1d: Option<GgufTensorInfo>,
    pub ssm_conv1d_bias: Option<GgufTensorInfo>,
    pub ssm_dt_bias: Option<GgufTensorInfo>,
    /// Per-head learned negative decay used by Qwen4Exp GatedDeltaNet.
    /// Unlike the other SSM tensors this is a vector named `ssm_a`.
    pub ssm_a: Option<GgufTensorInfo>,
    pub ssm_norm: Option<GgufTensorInfo>,
    pub ssm_out: Option<GgufTensorInfo>,
    // Qwen4Exp Gated Residual / four-stream Hyper-Connection tensors.
    pub hc_attn_norm: Option<GgufTensorInfo>,
    pub hc_attn_down: Option<GgufTensorInfo>,
    pub hc_attn_up: Option<GgufTensorInfo>,
    pub hc_attn_inject: Option<GgufTensorInfo>,
    pub hc_ffn_norm: Option<GgufTensorInfo>,
    pub hc_ffn_down: Option<GgufTensorInfo>,
    pub hc_ffn_up: Option<GgufTensorInfo>,
    pub hc_ffn_inject: Option<GgufTensorInfo>,
    // Per-Layer Embedding projections and depthwise dilated convolution.
    pub ple_key: Option<GgufTensorInfo>,
    pub ple_value: Option<GgufTensorInfo>,
    pub ple_norm_key: Option<GgufTensorInfo>,
    pub ple_norm_query: Option<GgufTensorInfo>,
    pub ple_norm_conv: Option<GgufTensorInfo>,
    pub ple_conv1d: Option<GgufTensorInfo>,
    // Qwen Sparse Attention micro-block indexer on full-attention layers.
    pub indexer_q: Option<GgufTensorInfo>,
    pub indexer_k: Option<GgufTensorInfo>,
    pub indexer_q_norm: Option<GgufTensorInfo>,
    pub indexer_k_norm: Option<GgufTensorInfo>,
}

impl LayerTensors {
    /// A Qwen hybrid SSM layer is identified by its required recurrent tensors, not merely by a
    /// fused `attn_qkv` name.  The latter must not be routed through scaled-dot-product attention.
    pub fn is_hybrid_ssm_layer(&self) -> bool {
        let qwen = self.attn_qkv.is_some()
            && self.ssm_alpha.is_some()
            && self.ssm_beta.is_some()
            && self.ssm_conv1d.is_some()
            && self.ssm_a.is_some()
            && self.ssm_dt_bias.is_some()
            && self.ssm_norm.is_some()
            && self.ssm_out.is_some();
        let granite = self.ssm_in.is_some()
            && self.ssm_conv1d.is_some()
            && self.ssm_dt_bias.is_some()
            && self.ssm_norm.is_some()
            && self.ssm_out.is_some();
        qwen || granite
    }

    /// Full attention requires independently projected Q/K/V matrices.  Qwen's SSM qkv matrix
    /// intentionally does not satisfy this predicate.
    pub fn has_full_attention_qkv(&self) -> bool {
        self.attn_q.is_some() && self.attn_k.is_some() && self.attn_v.is_some()
    }

    /// QSA is executable only when all indexer projections/norms accompany a
    /// full attention layer.  Do not silently replace it with dense attention.
    pub fn has_qwen_sparse_attention(&self) -> bool {
        self.has_full_attention_qkv()
            && self.attn_q_norm.is_some()
            && self.attn_k_norm.is_some()
            && self.indexer_q.is_some()
            && self.indexer_k.is_some()
            && self.indexer_q_norm.is_some()
            && self.indexer_k_norm.is_some()
    }

    /// The Gated Residual block needs all four learned paths for the specified
    /// sub-block. Absence is an unsupported graph, not a harmless residual.
    pub fn has_qwen_hyper_connection(&self, ffn: bool) -> bool {
        if ffn {
            self.hc_ffn_norm.is_some()
                && self.hc_ffn_down.is_some()
                && self.hc_ffn_up.is_some()
                && self.hc_ffn_inject.is_some()
        } else {
            self.hc_attn_norm.is_some()
                && self.hc_attn_down.is_some()
                && self.hc_attn_up.is_some()
                && self.hc_attn_inject.is_some()
        }
    }

    /// A layer that receives PLE must supply every projection/state tensor.
    pub fn has_qwen_ple(&self) -> bool {
        self.ple_key.is_some()
            && self.ple_value.is_some()
            && self.ple_norm_key.is_some()
            && self.ple_norm_query.is_some()
            && self.ple_norm_conv.is_some()
            && self.ple_conv1d.is_some()
    }

    /// Resolve separate Q/K/V projections or split the row-concatenated Qwen fused-QKV tensor.
    ///
    /// The row layout is `[Q; K; V]`, with `q_rows = n_head * head_dim` and each KV projection
    /// occupying `kv_rows = n_kv_head * head_dim`.  Returning copies of metadata keeps the
    /// execution ABI zero-copy: the weight bytes stay in the mapped GGUF file.
    pub fn qkv_views(
        &self,
        q_rows: usize,
        kv_rows: usize,
    ) -> Option<(GgufTensorInfo, GgufTensorInfo, GgufTensorInfo)> {
        if let (Some(q), Some(k), Some(v)) = (self.attn_q, self.attn_k, self.attn_v) {
            return Some((q, k, v));
        }
        let fused = self.attn_qkv?;
        let q = fused.row_range(0, q_rows)?;
        let k = fused.row_range(q_rows, kv_rows)?;
        let v = fused.row_range(q_rows.checked_add(kv_rows)?, kv_rows)?;
        Some((q, k, v))
    }
}
