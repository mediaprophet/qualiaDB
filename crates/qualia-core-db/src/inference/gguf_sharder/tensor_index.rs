//! `GgufTensorIndex` — the tensor-name-hash → info lookup table, built by walking
//! the GGUF tensor-info section (and the KV hyperparameter parse that precedes it),
//! plus the per-layer tensor-name helpers and token-embedding dequant access.

use super::*;

/// Lookup table from tensor-name hash → `GgufTensorInfo`, built by walking the
/// GGUF tensor-info section that immediately follows the KV metadata section.
#[derive(Clone)]
pub struct GgufTensorIndex {
    pub(crate) entries: Vec<(u64, GgufTensorInfo)>, // (name_hash, info)
    /// Absolute byte offset in the mmap where tensor payload data begins.
    pub tensor_data_start: u64,
    /// Cached metadata for `token_embd.weight` (embedding lookup target).
    pub(crate) token_embd: Option<GgufTensorInfo>,
    /// Cached `output.weight` for final vocabulary projection.
    pub(crate) output_weight: Option<GgufTensorInfo>,
    /// Cached `output_norm.weight` — final RMSNorm before vocab projection (Llama/SmolLM).
    pub(crate) output_norm: Option<GgufTensorInfo>,
    pub hyperparams: GgufHyperparams,
    /// Largest tensor payload in the file (informational).
    pub max_tensor_bytes: usize,
    /// Largest layer matmul tensor (attn/ffn weights) — sizes reusable GPU staging.
    pub max_layer_tensor_bytes: usize,
}

/// True when `name` is a per-layer matmul weight consumed by `dispatch_transformer_layer`.
fn is_layer_matmul_tensor_name(name: &[u8]) -> bool {
    const SUFFIXES: [&[u8]; 7] = [
        b"attn_q.weight",
        b"attn_k.weight",
        b"attn_v.weight",
        b"attn_output.weight",
        b"ffn_gate.weight",
        b"ffn_up.weight",
        b"ffn_down.weight",
    ];
    if !name.starts_with(b"blk.") {
        return false;
    }
    SUFFIXES.iter().any(|s| name.ends_with(s))
}

/// Write `blk.{layer}.{suffix}` into `out`; returns total bytes written.
pub fn write_blk_tensor_name(layer: u32, suffix: &[u8], out: &mut [u8]) -> usize {
    let prefix = b"blk.";
    let mut n = 0usize;
    if out.len() < prefix.len() + 1 + suffix.len() {
        return 0;
    }
    out[..prefix.len()].copy_from_slice(prefix);
    n += prefix.len();
    let mut v = layer;
    let mut digits = [0u8; 10];
    let mut d = 0usize;
    if v == 0 {
        digits[0] = b'0';
        d = 1;
    } else {
        while v > 0 && d < digits.len() {
            digits[d] = b'0' + (v % 10) as u8;
            v /= 10;
            d += 1;
        }
    }
    for i in (0..d).rev() {
        if n >= out.len() {
            return n;
        }
        out[n] = digits[i];
        n += 1;
    }
    if n >= out.len() {
        return n;
    }
    out[n] = b'.';
    n += 1;
    let copy = suffix.len().min(out.len() - n);
    out[n..n + copy].copy_from_slice(&suffix[..copy]);
    n + copy
}

impl GgufTensorIndex {
    pub fn from_gguf(mmap: &[u8]) -> Self {
        Self::try_build(mmap).unwrap_or_else(|| Self {
            entries: vec![],
            tensor_data_start: 0,
            token_embd: None,
            output_weight: None,
            output_norm: None,
            hyperparams: GgufHyperparams::default(),
            max_tensor_bytes: 0,
            max_layer_tensor_bytes: 0,
        })
    }

    fn parse_kv_hyperparams(
        key: &str,
        vtype: u32,
        mmap: &[u8],
        pos: &mut usize,
    ) -> GgufHyperparams {
        let mut patch = GgufHyperparams::default();
        // general.architecture = STRING
        if key == "general.architecture" && vtype == 8 {
            if *pos + 8 <= mmap.len() {
                let n =
                    u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8])) as usize;
                *pos += 8;
                if *pos + n <= mmap.len() {
                    let s = std::str::from_utf8(&mmap[*pos..*pos + n]).unwrap_or("");
                    patch.architecture = parse_architecture_id(s);
                    *pos += n;
                }
            }
            return patch;
        }
        if key.ends_with("rope.freq_base") && !key.contains("swa") {
            match vtype {
                6 if *pos + 4 <= mmap.len() => {
                    let bits =
                        u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
                    *pos += 4;
                    patch.rope_freq_base = f32::from_bits(bits);
                }
                12 if *pos + 8 <= mmap.len() => {
                    let bits =
                        u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
                    *pos += 8;
                    patch.rope_freq_base = f64::from_bits(bits) as f32;
                }
                _ => {
                    let _ = gguf_skip_value(mmap, pos, vtype);
                }
            }
            return patch;
        }
        if key.ends_with("final_logit_softcapping") || key.ends_with("attention.logit_softcapping")
        {
            match vtype {
                6 if *pos + 4 <= mmap.len() => {
                    let bits =
                        u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
                    *pos += 4;
                    patch.logit_softcap = f32::from_bits(bits);
                    if patch.logit_softcap > 0.0 {
                        patch.arch_flags |= ARCH_FLAG_HAS_SOFTCAP;
                    }
                }
                12 if *pos + 8 <= mmap.len() => {
                    let bits =
                        u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
                    *pos += 8;
                    patch.logit_softcap = f64::from_bits(bits) as f32;
                    if patch.logit_softcap > 0.0 {
                        patch.arch_flags |= ARCH_FLAG_HAS_SOFTCAP;
                    }
                }
                _ => {
                    let _ = gguf_skip_value(mmap, pos, vtype);
                }
            }
            return patch;
        }
        if key.ends_with("rope.scale_linear") || key.ends_with("rope.scaling.factor") {
            match vtype {
                6 if *pos + 4 <= mmap.len() => {
                    let bits =
                        u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
                    *pos += 4;
                    patch.rope_scale = f32::from_bits(bits);
                }
                12 if *pos + 8 <= mmap.len() => {
                    let bits =
                        u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().unwrap_or([0; 8]));
                    *pos += 8;
                    patch.rope_scale = f64::from_bits(bits) as f32;
                }
                4 if *pos + 4 <= mmap.len() => {
                    patch.rope_scale =
                        u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]))
                            as f32;
                    *pos += 4;
                }
                _ => {
                    let _ = gguf_skip_value(mmap, pos, vtype);
                }
            }
            return patch;
        }
        if vtype != 4 {
            let _ = gguf_skip_value(mmap, pos, vtype);
            return patch;
        }
        if *pos + 4 > mmap.len() {
            return patch;
        }
        let v = u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().unwrap_or([0; 4]));
        *pos += 4;
        if key.ends_with("block_count") {
            patch.n_layer = v;
        } else if key.ends_with("embedding_length") && !key.contains("per_layer") {
            patch.n_embd = v;
        } else if key.ends_with("attention.head_count") && !key.contains("kv") {
            patch.n_head = v;
        } else if key.contains("head_count_kv") || key.contains("n_kv_head") {
            patch.n_kv_head = v;
        } else if key.ends_with("attention.key_length_swa")
            || key.ends_with("attention.value_length_swa")
        {
            // Prefer key_length_swa; value_length_swa is the same for Gemma 4.
            if patch.head_dim_swa == 0 {
                patch.head_dim_swa = v;
            }
            if v > 0 {
                patch.arch_flags |= ARCH_FLAG_HAS_SWA;
            }
        } else if key.ends_with("attention.key_length") || key.ends_with("attention.value_length") {
            if !key.contains("swa") && patch.head_dim == 0 {
                patch.head_dim = v;
            }
        } else if key.ends_with("attention.sliding_window") {
            patch.sliding_window = v;
            if v > 0 {
                patch.arch_flags |= ARCH_FLAG_HAS_SWA;
            }
        } else if key.ends_with("attention.shared_kv_layers") {
            patch.shared_kv_layers = v;
            if v > 0 {
                patch.arch_flags |= ARCH_FLAG_HAS_SHARED_KV;
            }
        }
        patch
    }

    fn try_build(mmap: &[u8]) -> Option<Self> {
        if mmap.len() < 24 || &mmap[0..4] != b"GGUF" {
            return None;
        }
        let version = u32::from_le_bytes(mmap[4..8].try_into().ok()?);
        if version < 2 {
            return None;
        }
        let tensor_count = u64::from_le_bytes(mmap[8..16].try_into().ok()?);
        let kv_count = u64::from_le_bytes(mmap[16..24].try_into().ok()?);

        let mut hyperparams = GgufHyperparams::default();
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
            if patch.architecture != 0 {
                hyperparams.architecture = patch.architecture;
            }
            hyperparams.arch_flags |= patch.arch_flags;
        }

        let mut entries = Vec::with_capacity(tensor_count.min(4096) as usize);
        let mut max_tensor_bytes = 0usize;
        let mut max_layer_tensor_bytes = 0usize;
        for _ in 0..tensor_count {
            if pos + 8 > mmap.len() {
                break;
            }
            let nlen = u64::from_le_bytes(mmap[pos..pos + 8].try_into().ok()?) as usize;
            pos += 8;
            if pos + nlen > mmap.len() {
                break;
            }
            let name = &mmap[pos..pos + nlen];
            let name_hash = gguf_name_hash(name);
            pos += nlen;

            // n_dims
            if pos + 4 > mmap.len() {
                break;
            }
            let n_dims_raw = u32::from_le_bytes(mmap[pos..pos + 4].try_into().ok()?) as usize;
            pos += 4;

            // Shape (up to 4 dims stored; rest skipped)
            let mut dims = [0u64; 4];
            for d in 0..n_dims_raw {
                if pos + 8 > mmap.len() {
                    break;
                }
                let v = u64::from_le_bytes(mmap[pos..pos + 8].try_into().ok()?);
                pos += 8;
                if d < 4 {
                    dims[d] = v;
                }
            }

            // ggml_type + offset
            if pos + 12 > mmap.len() {
                break;
            }
            let ggml_type = u32::from_le_bytes(mmap[pos..pos + 4].try_into().ok()?);
            pos += 4;
            let byte_offset = u64::from_le_bytes(mmap[pos..pos + 8].try_into().ok()?);
            pos += 8;

            let info = GgufTensorInfo {
                dims,
                n_dims: n_dims_raw.min(4) as u32,
                ggml_type,
                byte_offset,
            };
            if let Some(tb) = crate::ggml_quants::tensor_byte_len(&info) {
                max_tensor_bytes = max_tensor_bytes.max(tb);
                if is_layer_matmul_tensor_name(name) {
                    max_layer_tensor_bytes = max_layer_tensor_bytes.max(tb);
                }
            }
            entries.push((name_hash, info));
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
        // Tensor-feature refinement (PLE / QK-norm) — catches gemma4 even if arch string missed.
        let ple_hash = gguf_name_hash(b"per_layer_token_embd.weight");
        if entries.iter().any(|(h, _)| *h == ple_hash) {
            hyperparams.arch_flags |= ARCH_FLAG_HAS_PLE;
            if hyperparams.architecture == ARCH_UNKNOWN || hyperparams.architecture == ARCH_OTHER {
                hyperparams.architecture = ARCH_GEMMA4;
            }
        }
        let qk_norm_hash = gguf_name_hash(b"blk.0.attn_q_norm.weight");
        if entries.iter().any(|(h, _)| *h == qk_norm_hash) {
            hyperparams.arch_flags |= ARCH_FLAG_HAS_QK_NORM;
        }
        Some(Self {
            entries,
            tensor_data_start,
            token_embd,
            output_weight,
            output_norm,
            hyperparams,
            max_tensor_bytes,
            max_layer_tensor_bytes,
        })
    }

    /// Build a synthetic index from an explicit `(name, info)` list — used to boot from a P64
    /// weight container so the *entire* GGUF-based hot path (get_layer_tensors / fetch_tensor_bytes /
    /// resident upload) works unchanged. The caller passes absolute blob offsets in each
    /// `GgufTensorInfo.byte_offset` and `tensor_data_start = 0`, pointing the byte source at the
    /// P64 bytes. Format-agnostic: the hot path never learns it is reading P64.
    pub fn from_components(
        named_tensors: &[(&[u8], GgufTensorInfo)],
        hyperparams: GgufHyperparams,
        tensor_data_start: u64,
    ) -> Self {
        let mut entries = Vec::with_capacity(named_tensors.len());
        let mut max_tensor_bytes = 0usize;
        let mut max_layer_tensor_bytes = 0usize;
        for (name, info) in named_tensors {
            if let Some(tb) = crate::ggml_quants::tensor_byte_len(info) {
                max_tensor_bytes = max_tensor_bytes.max(tb);
                if is_layer_matmul_tensor_name(name) {
                    max_layer_tensor_bytes = max_layer_tensor_bytes.max(tb);
                }
            }
            entries.push((gguf_name_hash(name), *info));
        }
        let find_h = |h: u64| entries.iter().find(|(eh, _)| *eh == h).map(|(_, i)| *i);
        let token_embd = find_h(gguf_name_hash(b"token_embd.weight"));
        let output_weight = find_h(gguf_name_hash(b"output.weight"));
        let output_norm = find_h(gguf_name_hash(b"output_norm.weight"));
        Self {
            entries,
            tensor_data_start,
            token_embd,
            output_weight,
            output_norm,
            hyperparams,
            max_tensor_bytes,
            max_layer_tensor_bytes,
        }
    }

    fn find(&self, name: &[u8]) -> Option<GgufTensorInfo> {
        let h = gguf_name_hash(name);
        self.entries
            .iter()
            .find(|(eh, _)| *eh == h)
            .map(|(_, i)| *i)
    }

    fn find_layer_tensor(&self, layer: u32, suffix: &[u8]) -> Option<GgufTensorInfo> {
        let mut name = [0u8; 96];
        let n = write_blk_tensor_name(layer, suffix, &mut name);
        if n == 0 {
            return None;
        }
        self.find(&name[..n])
    }

    /// Retrieve attention + FFN tensor metadata for one transformer block.
    pub fn get_layer_tensors(&self, layer_idx: u32) -> LayerTensors {
        LayerTensors {
            attn_norm: self.find_layer_tensor(layer_idx, b"attn_norm.weight"),
            attn_q: self.find_layer_tensor(layer_idx, b"attn_q.weight"),
            attn_k: self.find_layer_tensor(layer_idx, b"attn_k.weight"),
            attn_v: self.find_layer_tensor(layer_idx, b"attn_v.weight"),
            attn_output: self.find_layer_tensor(layer_idx, b"attn_output.weight"),
            ffn_norm: self.find_layer_tensor(layer_idx, b"ffn_norm.weight"),
            ffn_gate: self.find_layer_tensor(layer_idx, b"ffn_gate.weight"),
            ffn_up: self.find_layer_tensor(layer_idx, b"ffn_up.weight"),
            ffn_down: self.find_layer_tensor(layer_idx, b"ffn_down.weight"),
        }
    }

    pub fn output_norm_info(&self) -> Option<&GgufTensorInfo> {
        self.output_norm.as_ref()
    }

    pub fn output_weight_info(&self) -> Option<&GgufTensorInfo> {
        self.output_weight.as_ref()
    }

    /// True when `output.weight` is absent and logits use tied `token_embd.weight`.
    pub fn output_weights_tied(&self) -> bool {
        self.output_weight.is_none() && self.token_embd.is_some()
    }

    /// Output projection weights: `output.weight` when present, else tied `token_embd.weight`.
    pub fn logits_projection_info(&self) -> Option<&GgufTensorInfo> {
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
    pub fn token_embd_info(&self) -> Option<&GgufTensorInfo> {
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
        let raw = match crate::ggml_quants::fetch_token_embedding(
            mmap,
            self.tensor_data_start,
            info,
            token_id,
        ) {
            Ok(s) => s,
            Err(_) => return 0,
        };
        crate::ggml_quants::dequantize_row_into(raw, info.ggml_type, n_embd, out).unwrap_or(0)
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
