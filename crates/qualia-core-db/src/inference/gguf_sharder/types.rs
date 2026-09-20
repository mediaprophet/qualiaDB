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
    pub ssm_norm: Option<GgufTensorInfo>,
    pub ssm_out: Option<GgufTensorInfo>,
}

impl LayerTensors {
    /// A Qwen hybrid SSM layer is identified by its required recurrent tensors, not merely by a
    /// fused `attn_qkv` name.  The latter must not be routed through scaled-dot-product attention.
    pub fn is_hybrid_ssm_layer(&self) -> bool {
        let qwen = self.attn_qkv.is_some()
            && self.ssm_alpha.is_some()
            && self.ssm_beta.is_some()
            && self.ssm_conv1d.is_some()
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
