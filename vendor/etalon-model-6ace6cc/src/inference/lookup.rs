use super::*;

impl<H: From<ModelHyperparams>> TensorIndex<H> {
    fn find(&self, name: &[u8]) -> Option<TensorInfo> {
        let h = gguf_name_hash(name);
        self.entries
            .iter()
            .find(|(eh, _)| *eh == h || synthetic_tensor_hash(name) == Some(*eh))
            .map(|(_, i)| *i)
    }

    /// Resolve a named global tensor from the bounded metadata index.  This
    /// performs no source-payload I/O and is intended for architecture-owned
    /// global paths such as Qwen4Exp's final Hyper-Connection mixer.
    pub fn tensor_info(&self, name: &[u8]) -> Option<TensorInfo> {
        self.find(name)
    }

    fn find_layer_tensor(&self, layer: u32, suffix: &[u8]) -> Option<TensorInfo> {
        let mut name = [0u8; 96];
        let n = write_blk_tensor_name(layer, suffix, &mut name);
        if n > 0 {
            if let Some(info) = self.find(&name[..n]) {
                return Some(info);
            }
        }
        // Architecture-specific aliases (e.g. HuggingFace safetensors or alternative GGUF prefixes)
        let aliases: &[&[u8]] = match suffix {
            b"attn_k.weight" => &[b"self_attn.k_proj.weight", b"k_proj.weight"],
            b"attn_q.weight" => &[b"self_attn.q_proj.weight", b"q_proj.weight"],
            b"attn_v.weight" => &[b"self_attn.v_proj.weight", b"v_proj.weight"],
            b"attn_output.weight" => &[b"self_attn.o_proj.weight", b"o_proj.weight"],
            b"attn_norm.weight" => &[b"input_layernorm.weight"],
            b"ffn_norm.weight" => &[b"post_attention_layernorm.weight"],
            b"ffn_gate.weight" => &[b"mlp.gate_proj.weight", b"gate_proj.weight"],
            b"ffn_up.weight" => &[b"mlp.up_proj.weight", b"up_proj.weight"],
            b"ffn_down.weight" => &[b"mlp.down_proj.weight", b"down_proj.weight"],
            b"ffn_gate_inp.weight" => &[b"mlp.gate.weight", b"gate.weight"],
            b"ssm_conv1d.weight" => &[b"linear_attn.conv1d.weight", b"conv1d.weight"],
            b"ssm_a" => &[b"linear_attn.A_log", b"A_log"],
            b"ssm_dt.bias" => &[b"linear_attn.dt_bias", b"dt_bias"],
            b"ssm_alpha.weight" => &[b"linear_attn.in_proj_ba.weight", b"in_proj_ba.weight"],
            b"ssm_beta.weight" => &[b"linear_attn.in_proj_ba.weight", b"in_proj_ba.weight"],
            b"attn_qkv.weight" => &[
                b"linear_attn.in_proj_qkvz.weight",
                b"self_attn.qkv_proj.weight",
                b"qkv_proj.weight",
            ],
            b"ssm_norm.weight" => &[b"linear_attn.norm.weight"],
            b"ssm_out.weight" => &[b"linear_attn.out_proj.weight"],
            _ => &[],
        };
        for alias in aliases {
            let mut hf_name = [0u8; 96];
            let len = write_hf_layer_tensor_name(layer, alias, &mut hf_name);
            if len > 0 {
                if let Some(info) = self.find(&hf_name[..len]) {
                    return Some(info);
                }
            }
            let blk_len = write_blk_tensor_name(layer, alias, &mut hf_name);
            if blk_len > 0 {
                if let Some(info) = self.find(&hf_name[..blk_len]) {
                    return Some(info);
                }
            }
        }
        None
    }

    /// Retrieve attention + FFN tensor metadata for one transformer block.
    pub fn get_layer_tensors(&self, layer_idx: u32) -> LayerTensors {
        LayerTensors {
            layer_idx,
            attn_norm: self.find_layer_tensor(layer_idx, b"attn_norm.weight"),
            attn_q: self.find_layer_tensor(layer_idx, b"attn_q.weight"),
            attn_k: self.find_layer_tensor(layer_idx, b"attn_k.weight"),
            attn_v: self.find_layer_tensor(layer_idx, b"attn_v.weight"),
            attn_q_norm: self.find_layer_tensor(layer_idx, b"attn_q_norm.weight"),
            attn_k_norm: self.find_layer_tensor(layer_idx, b"attn_k_norm.weight"),
            attn_qkv: self.find_layer_tensor(layer_idx, b"attn_qkv.weight"),
            attn_gate: self.find_layer_tensor(layer_idx, b"attn_gate.weight"),
            attn_output: self.find_layer_tensor(layer_idx, b"attn_output.weight"),
            ffn_norm: self.find_layer_tensor(layer_idx, b"ffn_norm.weight"),
            ffn_gate: self.find_layer_tensor(layer_idx, b"ffn_gate.weight"),
            ffn_up: self.find_layer_tensor(layer_idx, b"ffn_up.weight"),
            ffn_down: self.find_layer_tensor(layer_idx, b"ffn_down.weight"),
            moe_router: self.find_layer_tensor(layer_idx, b"ffn_gate_inp.weight"),
            moe_gate_exps: self.find_layer_tensor(layer_idx, b"ffn_gate_exps.weight"),
            moe_up_exps: self.find_layer_tensor(layer_idx, b"ffn_up_exps.weight"),
            moe_gate_up_exps: self.find_layer_tensor(layer_idx, b"ffn_gate_up_exps.weight"),
            moe_down_exps: self.find_layer_tensor(layer_idx, b"ffn_down_exps.weight"),
            moe_shared_gate: self.find_layer_tensor(layer_idx, b"ffn_gate_shexp.weight"),
            moe_shared_up: self.find_layer_tensor(layer_idx, b"ffn_up_shexp.weight"),
            moe_shared_down: self.find_layer_tensor(layer_idx, b"ffn_down_shexp.weight"),
            moe_shared_gate_input: self.find_layer_tensor(layer_idx, b"ffn_gate_inp_shexp.weight"),
            ssm_alpha: self.find_layer_tensor(layer_idx, b"ssm_alpha.weight"),
            ssm_beta: self.find_layer_tensor(layer_idx, b"ssm_beta.weight"),
            ssm_in: self.find_layer_tensor(layer_idx, b"ssm_in.weight"),
            ssm_conv1d: self.find_layer_tensor(layer_idx, b"ssm_conv1d.weight"),
            ssm_conv1d_bias: self.find_layer_tensor(layer_idx, b"ssm_conv1d.bias"),
            ssm_dt_bias: self.find_layer_tensor(layer_idx, b"ssm_dt.bias"),
            ssm_a: self.find_layer_tensor(layer_idx, b"ssm_a"),
            ssm_norm: self.find_layer_tensor(layer_idx, b"ssm_norm.weight"),
            ssm_out: self.find_layer_tensor(layer_idx, b"ssm_out.weight"),
            hc_attn_norm: self.find_layer_tensor(layer_idx, b"hc_attn_norm.weight"),
            hc_attn_down: self.find_layer_tensor(layer_idx, b"hc_attn_down.weight"),
            hc_attn_up: self.find_layer_tensor(layer_idx, b"hc_attn_up.weight"),
            hc_attn_inject: self.find_layer_tensor(layer_idx, b"hc_attn_inject.weight"),
            hc_ffn_norm: self.find_layer_tensor(layer_idx, b"hc_ffn_norm.weight"),
            hc_ffn_down: self.find_layer_tensor(layer_idx, b"hc_ffn_down.weight"),
            hc_ffn_up: self.find_layer_tensor(layer_idx, b"hc_ffn_up.weight"),
            hc_ffn_inject: self.find_layer_tensor(layer_idx, b"hc_ffn_inject.weight"),
            ple_key: self.find_layer_tensor(layer_idx, b"ple_key.weight"),
            ple_value: self.find_layer_tensor(layer_idx, b"ple_value.weight"),
            ple_norm_key: self.find_layer_tensor(layer_idx, b"ple_norm_key.weight"),
            ple_norm_query: self.find_layer_tensor(layer_idx, b"ple_norm_query.weight"),
            ple_norm_conv: self.find_layer_tensor(layer_idx, b"ple_norm_conv.weight"),
            ple_conv1d: self.find_layer_tensor(layer_idx, b"ple_conv1d.weight"),
            indexer_q: self.find_layer_tensor(layer_idx, b"indexer.q_proj.weight"),
            indexer_k: self.find_layer_tensor(layer_idx, b"indexer.k_proj.weight"),
            indexer_q_norm: self.find_layer_tensor(layer_idx, b"indexer.q_norm.weight"),
            indexer_k_norm: self.find_layer_tensor(layer_idx, b"indexer.k_norm.weight"),
        }
    }

    /// Output-norm tensor info.
    pub fn output_norm_info(&self) -> Option<&TensorInfo> {
        self.output_norm.as_ref()
    }

    /// Required Qwen4Exp hash-gathered PLE n-gram table, if present.
    pub fn ple_ngram_embedding_info(&self) -> Option<&TensorInfo> {
        self.ple_ngram_embedding.as_ref()
    }

    /// `TensorIndex::output_weight_info`.
    pub fn output_weight_info(&self) -> Option<&TensorInfo> {
        self.output_weight.as_ref()
    }

    /// True when `output.weight` is absent and logits use tied `token_embd.weight`.
    pub fn output_weights_tied(&self) -> bool {
        self.output_weight.is_none() && self.token_embd.is_some()
    }

    /// Output projection weights: `output.weight` when present, else tied `token_embd.weight`.
    pub fn logits_projection_info(&self) -> Option<&TensorInfo> {
        self.output_weight
            .as_ref()
            .or_else(|| self.token_embd.as_ref())
    }

    /// Diagnostic: byte offset + dims for tied-weight validation (MC3g).
    pub fn weight_tie_probe(&self) -> (bool, u64, u64, [u64; 4], [u64; 4]) {
        let tied = self.output_weights_tied();
        let emb = self.token_embd.as_ref();
        let out = self.output_weight.as_ref();
        let emb_off = emb.map(|t| t.byte_offset).unwrap_or(0);
        let out_off = out.map(|t| t.byte_offset).unwrap_or(emb_off);
        let emb_dims = emb.map(|t| t.dims).unwrap_or([0; 4]);
        let out_dims = out.map(|t| t.dims).unwrap_or(emb_dims);
        (tied, emb_off, out_off, emb_dims, out_dims)
    }

    /// Cached `token_embd.weight` tensor metadata.
    pub fn token_embd_info(&self) -> Option<&TensorInfo> {
        self.token_embd.as_ref()
    }

    /// Return the embedding dimension (n_embd) from `token_embd.weight`, or 0 if unknown.
    pub fn emb_dim(&self) -> usize {
        self.token_embd_info()
            .map(|i| i.dims[0] as usize)
            .unwrap_or(0)
    }

    /// Vocabulary size from `token_embd.weight` shape `[n_embd, n_vocab]`.
    pub fn vocab_dim(&self) -> usize {
        self.token_embd_info()
            .map(|i| i.dims[1] as usize)
            .unwrap_or(0)
    }

    /// Dequantize one token embedding into caller-supplied `out` (zero heap in hot path).
    pub fn dequantize_token_embedding_into(
        &self,
        mmap: &[u8],
        token_id: u32,
        out: &mut [f32],
    ) -> usize {
        let info = match self.token_embd_info() {
            Some(i) => i,
            None => return 0,
        };
        let n_embd = info.dims[0] as usize;
        let n_vocab = info.dims[1] as usize;
        if n_embd == 0 || token_id as usize >= n_vocab || out.len() < n_embd {
            return 0;
        }
        let raw = match crate::ggml::fetch_token_embedding(
            mmap,
            self.tensor_data_start,
            info,
            token_id,
        ) {
            Ok(s) => s,
            Err(_) => return 0,
        };
        crate::ggml::dequantize_row_into(raw, info.ggml_type, n_embd, out).unwrap_or(0)
    }

    /// Dequantize one Qwen4Exp PLE n-gram table row from an already-mapped
    /// source into caller storage. This is a parity/reference path for the
    /// direct NVMe reader, not a licence to retain the full PLE table resident.
    pub fn dequantize_ple_row_into(&self, mmap: &[u8], row: u64, out: &mut [f32]) -> usize {
        let info = match self.ple_ngram_embedding_info() {
            Some(info) => info,
            None => return 0,
        };
        let row_width = info.dims[0] as usize;
        if row_width == 0 || row > u32::MAX as u64 || row >= info.dims[1] || out.len() < row_width {
            return 0;
        }
        let raw = match crate::ggml::fetch_token_embedding(
            mmap,
            self.tensor_data_start,
            info,
            row as u32,
        ) {
            Ok(raw) => raw,
            Err(_) => return 0,
        };
        crate::ggml::dequantize_row_into(raw, info.ggml_type, row_width, out).unwrap_or(0)
    }

    /// Slice and dequantize a token embedding (test / legacy path; allocates `Vec`).
    pub fn get_token_embedding(&self, mmap: &[u8], token_id: u32) -> Vec<f32> {
        let n_embd = self.emb_dim();
        if n_embd == 0 {
            return vec![];
        }
        let mut out = vec![0.0f32; n_embd];
        let n = self.dequantize_token_embedding_into(mmap, token_id, &mut out);
        if n == 0 {
            vec![]
        } else {
            out.truncate(n);
            out
        }
    }
}
