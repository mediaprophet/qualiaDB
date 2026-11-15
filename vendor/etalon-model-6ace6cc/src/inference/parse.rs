use super::*;

impl<H: From<ModelHyperparams>> TensorIndex<H> {
    /// Builds `TensorIndex` from `gguf`.
    pub fn from_gguf(mmap: &[u8]) -> Self {
        Self::try_build(mmap).unwrap_or_else(|| Self {
            entries: vec![],
            tensor_data_start: 0,
            token_embd: None,
            output_weight: None,
            output_norm: None,
            ple_ngram_embedding: None,
            ple_config: None,
            moe_routing: Default::default(),
            hyperparams: ModelHyperparams::default().into(),
            max_tensor_bytes: 0,
            max_layer_tensor_bytes: 0,
        })
    }

    pub(super) fn try_build(mmap: &[u8]) -> Option<Self> {
        if mmap.len() < 24 || &mmap[0..4] != b"GGUF" {
            return None;
        }
        let version = u32::from_le_bytes(mmap[4..8].try_into().ok()?);
        if version < 2 {
            return None;
        }
        let tensor_count = u64::from_le_bytes(mmap[8..16].try_into().ok()?);
        let kv_count = u64::from_le_bytes(mmap[16..24].try_into().ok()?);

        let mut hyperparams = ModelHyperparams::default();
        let mut ple_config = Qwen4ExpPleConfig::default();
        let mut moe_routing = moe_routing::MoeRoutingSettings::default();
        let mut pos = 24usize;
        for _ in 0..kv_count {
            if pos + 8 > mmap.len() {
                return None;
            }
            let klen = u64::from_le_bytes(mmap[pos..pos + 8].try_into().ok()?) as usize;
            pos += 8;
            if pos + klen + 4 > mmap.len() {
                return None;
            }
            let key = std::str::from_utf8(&mmap[pos..pos + klen]).unwrap_or("");
            pos += klen;
            let vtype = u32::from_le_bytes(mmap[pos..pos + 4].try_into().ok()?);
            pos += 4;
            if key.ends_with(".expert_count") || key.ends_with(".expert_used_count") {
                if vtype != 4 {
                    return None;
                }
                let value = u32::from_le_bytes(mmap.get(pos..pos + 4)?.try_into().ok()?);
                pos += 4;
                let field = if key.ends_with(".expert_count") {
                    &mut moe_routing.expert_count
                } else {
                    &mut moe_routing.experts_per_token
                };
                if field.is_some_and(|old| old != value) {
                    return None;
                }
                *field = Some(value);
                continue;
            }
            if Self::parse_ple_config_kv(key, vtype, mmap, &mut pos, &mut ple_config) {
                continue;
            }
            let patch = Self::parse_kv_hyperparams(key, vtype, mmap, &mut pos);
            if patch.n_layer != 0 {
                hyperparams.n_layer = patch.n_layer;
            }
            if patch.n_embd != 0 {
                hyperparams.n_embd = patch.n_embd;
            }
            if patch.n_head != 0 {
                hyperparams.n_head = patch.n_head;
            }
            if patch.n_kv_head != 0 {
                hyperparams.n_kv_head = patch.n_kv_head;
            }
            if patch.rope_freq_base > 0.0 {
                hyperparams.rope_freq_base = patch.rope_freq_base;
            }
            if patch.rope_scale > 0.0 {
                hyperparams.rope_scale = patch.rope_scale;
            }
            if patch.head_dim != 0 {
                hyperparams.head_dim = patch.head_dim;
            }
            if patch.head_dim_swa != 0 {
                hyperparams.head_dim_swa = patch.head_dim_swa;
            }
            if patch.sliding_window != 0 {
                hyperparams.sliding_window = patch.sliding_window;
            }
            if patch.shared_kv_layers != 0 {
                hyperparams.shared_kv_layers = patch.shared_kv_layers;
            }
            if patch.logit_softcap > 0.0 {
                hyperparams.logit_softcap = patch.logit_softcap;
            }
            if patch.ssm_conv_kernel != 0 {
                hyperparams.ssm_conv_kernel = patch.ssm_conv_kernel;
            }
            if patch.ssm_state_size != 0 {
                hyperparams.ssm_state_size = patch.ssm_state_size;
            }
            if patch.ssm_group_count != 0 {
                hyperparams.ssm_group_count = patch.ssm_group_count;
            }
            if patch.ssm_time_step_rank != 0 {
                hyperparams.ssm_time_step_rank = patch.ssm_time_step_rank;
            }
            if patch.ssm_inner_size != 0 {
                hyperparams.ssm_inner_size = patch.ssm_inner_size;
            }
            if patch.full_attention_interval != 0 {
                hyperparams.full_attention_interval = patch.full_attention_interval;
            }
            if patch.architecture != 0 {
                hyperparams.architecture = patch.architecture;
            }
            hyperparams.arch_flags |= patch.arch_flags;
        }

        super::moe_metadata::validate(moe_routing).ok()?;
        let mut entries = Vec::with_capacity(tensor_count.min(4096) as usize);
        let mut max_tensor_bytes = 0usize;
        let mut max_layer_tensor_bytes = 0usize;
        for _ in 0..tensor_count {
            if pos + 8 > mmap.len() {
                return None;
            }
            let nlen = u64::from_le_bytes(mmap[pos..pos + 8].try_into().ok()?) as usize;
            pos += 8;
            if pos + nlen > mmap.len() {
                return None;
            }
            let name = &mmap[pos..pos + nlen];
            let name_hash = gguf_name_hash(name);
            pos += nlen;

            // n_dims
            if pos + 4 > mmap.len() {
                return None;
            }
            let n_dims_raw = u32::from_le_bytes(mmap[pos..pos + 4].try_into().ok()?) as usize;
            pos += 4;

            // Shape (up to 4 dims stored; rest skipped)
            let mut dims = [0u64; 4];
            for d in 0..n_dims_raw {
                if pos + 8 > mmap.len() {
                    return None;
                }
                let v = u64::from_le_bytes(mmap[pos..pos + 8].try_into().ok()?);
                pos += 8;
                if d < 4 {
                    dims[d] = v;
                }
            }

            // ggml_type + offset
            if pos + 12 > mmap.len() {
                return None;
            }
            let ggml_type = u32::from_le_bytes(mmap[pos..pos + 4].try_into().ok()?);
            pos += 4;
            let byte_offset = u64::from_le_bytes(mmap[pos..pos + 8].try_into().ok()?);
            pos += 8;

            let info = TensorInfo {
                dims,
                n_dims: n_dims_raw.min(4) as u32,
                ggml_type,
                byte_offset,
            };
            if let Some(tb) = crate::ggml::tensor_byte_len(&info) {
                max_tensor_bytes = max_tensor_bytes.max(tb);
                if is_layer_matmul_tensor_name(name) {
                    max_layer_tensor_bytes = max_layer_tensor_bytes.max(tb);
                }
            }
            entries.push((name_hash, info));
            let mut canonical = [0u8; 128];
            let c_len = canonical_component_name(name, &mut canonical);
            if c_len > 0 {
                let c_hash = gguf_name_hash(&canonical[..c_len]);
                if c_hash != name_hash {
                    entries.push((c_hash, info));
                }
            }
        }

        let tensor_data_start = ((pos as u64 + 31) & !31) as u64;
        let emb_hash = gguf_name_hash(b"token_embd.weight");
        let out_hash = gguf_name_hash(b"output.weight");
        let out_norm_hash = gguf_name_hash(b"output_norm.weight");
        let token_embd = entries
            .iter()
            .find(|(h, _)| *h == emb_hash)
            .map(|(_, i)| *i);
        let output_weight = entries
            .iter()
            .find(|(h, _)| *h == out_hash)
            .map(|(_, i)| *i);
        let output_norm = entries
            .iter()
            .find(|(h, _)| *h == out_norm_hash)
            .map(|(_, i)| *i);
        if hyperparams.n_embd == 0 {
            hyperparams.n_embd = token_embd.map(|t| t.dims[0] as u32).unwrap_or(0);
        }
        // Tensor-feature refinement. Qwen4Exp's PLE tensor is hash-gathered and must be
        // handled as an NVMe row source, never as an ordinary embedding or GEMM matrix.
        let ple_hash = gguf_name_hash(b"per_layer_token_embd.weight");
        let ple_alt_hash = gguf_name_hash(b"ple.ple_embedding.ngram_embedding.weight");
        let ple_ngram_embedding = entries
            .iter()
            .find(|(h, _)| *h == ple_hash || *h == ple_alt_hash)
            .map(|(_, i)| *i);
        if ple_ngram_embedding.is_some() {
            hyperparams.arch_flags |= ARCH_FLAG_HAS_PLE;
            if hyperparams.architecture == ARCH_UNKNOWN || hyperparams.architecture == ARCH_OTHER {
                hyperparams.architecture = ARCH_QWEN4EXP;
            }
        }
        let qk_norm_hash = gguf_name_hash(b"blk.0.attn_q_norm.weight");
        if entries.iter().any(|(h, _)| *h == qk_norm_hash) {
            hyperparams.arch_flags |= ARCH_FLAG_HAS_QK_NORM;
        }
        // Granite-H uses an SSM input/convolution block rather than normal Q/K/V.  Treat this
        // concrete tensor signature as authoritative when older converters omit or rename the
        // architecture metadata, so the attention-only runtime fails closed instead of skipping
        // recurrent state updates.
        let granite_ssm_in = gguf_name_hash(b"blk.0.ssm_in.weight");
        let granite_ssm_conv = gguf_name_hash(b"blk.0.ssm_conv1d.weight");
        if entries.iter().any(|(h, _)| *h == granite_ssm_in)
            && entries.iter().any(|(h, _)| *h == granite_ssm_conv)
            && (hyperparams.architecture == ARCH_UNKNOWN || hyperparams.architecture == ARCH_OTHER)
        {
            hyperparams.architecture = ARCH_GRANITE_HYBRID;
        }
        Some(Self {
            entries,
            tensor_data_start,
            token_embd,
            output_weight,
            output_norm,
            ple_ngram_embedding,
            ple_config: ple_config.is_complete().then_some(ple_config),
            moe_routing,
            hyperparams: hyperparams.into(),
            max_tensor_bytes,
            max_layer_tensor_bytes,
        })
    }

    /// Build a synthetic index from an explicit `(name, info)` list — used to boot from a P64
    /// weight container so the *entire* GGUF-based hot path (get_layer_tensors / fetch_tensor_bytes /
    /// resident upload) works unchanged. The caller passes absolute blob offsets in each
    /// `TensorInfo.byte_offset` and `tensor_data_start = 0`, pointing the byte source at the
    /// P64 bytes. Format-agnostic: the hot path never learns it is reading P64.
    pub fn from_components(
        named_tensors: &[(&[u8], TensorInfo)],
        hyperparams: H,
        tensor_data_start: u64,
    ) -> Self {
        let mut entries = Vec::with_capacity(named_tensors.len());
        let mut max_tensor_bytes = 0usize;
        let mut max_layer_tensor_bytes = 0usize;
        for (name, info) in named_tensors {
            let mut canonical = [0u8; 128];
            let canonical_len = canonical_component_name(name, &mut canonical);
            let lookup_name = if canonical_len > 0 {
                &canonical[..canonical_len]
            } else {
                name
            };
            if let Some(tb) = crate::ggml::tensor_byte_len(info) {
                max_tensor_bytes = max_tensor_bytes.max(tb);
                if is_layer_matmul_tensor_name(lookup_name) {
                    max_layer_tensor_bytes = max_layer_tensor_bytes.max(tb);
                }
            }
            entries.push((gguf_name_hash(lookup_name), *info));
        }
        Self::from_entries(
            entries,
            hyperparams,
            tensor_data_start,
            max_tensor_bytes,
            max_layer_tensor_bytes,
        )
    }

    pub(super) fn from_entries(
        entries: Vec<(u64, TensorInfo)>,
        hyperparams: H,
        tensor_data_start: u64,
        max_tensor_bytes: usize,
        max_layer_tensor_bytes: usize,
    ) -> Self {
        let find_h = |h: u64| entries.iter().find(|(eh, _)| *eh == h).map(|(_, i)| *i);
        let token_embd = find_h(gguf_name_hash(b"token_embd.weight"));
        let output_weight = find_h(gguf_name_hash(b"output.weight"));
        let output_norm = find_h(gguf_name_hash(b"output_norm.weight"));
        let ple_ngram_embedding = find_h(gguf_name_hash(b"per_layer_token_embd.weight"))
            .or_else(|| find_h(gguf_name_hash(b"ple.ple_embedding.ngram_embedding.weight")));
        Self {
            entries,
            tensor_data_start,
            token_embd,
            output_weight,
            output_norm,
            ple_ngram_embedding,
            ple_config: None,
            moe_routing: Default::default(),
            hyperparams,
            max_tensor_bytes,
            max_layer_tensor_bytes,
        }
    }
}
