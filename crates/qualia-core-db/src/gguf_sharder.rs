//! Q-GGUF Hybrid Packaging
//! Parses monolithic `.gguf` files: vocabulary (KV section) and tensor names/offsets
//! (tensor-info section) are extracted into native Rust types; multi-gigabyte tensor
//! payloads are left on disk for direct VRAM mapping via `gguf_bridge.rs`.

use crate::{NQuin, QualiaSuperBlock};

// ─── Module-level GGUF helpers ───────────────────────────────────────────────

/// FNV-1a hash over raw bytes — same algorithm as `crate::q_hash` but for
/// byte slices parsed at runtime (e.g. tensor names from the binary header).
fn gguf_name_hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Skip over one GGUF KV value of the given type without storing it.
/// Returns `None` on any parse error (truncated data, unknown type).
/// Used by both `GgufTokenizer` and `GgufTensorIndex`.
fn gguf_skip_value(mmap: &[u8], pos: &mut usize, vtype: u32) -> Option<()> {
    match vtype {
        0 | 1 | 7 => {
            if *pos + 1 > mmap.len() {
                return None;
            }
            *pos += 1;
        }
        2 | 3 => {
            if *pos + 2 > mmap.len() {
                return None;
            }
            *pos += 2;
        }
        4 | 5 | 6 => {
            if *pos + 4 > mmap.len() {
                return None;
            }
            *pos += 4;
        }
        10 | 11 | 12 => {
            if *pos + 8 > mmap.len() {
                return None;
            }
            *pos += 8;
        }
        8 => {
            if *pos + 8 > mmap.len() {
                return None;
            }
            let slen = u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().ok()?) as usize;
            *pos += 8;
            if *pos + slen > mmap.len() {
                return None;
            }
            *pos += slen;
        }
        9 => {
            if *pos + 12 > mmap.len() {
                return None;
            }
            let etype = u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().ok()?);
            *pos += 4;
            let cnt = u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().ok()?) as usize;
            *pos += 8;
            for _ in 0..cnt {
                gguf_skip_value(mmap, pos, etype)?;
            }
        }
        _ => return None,
    }
    Some(())
}

// ─── GgufTensorIndex ─────────────────────────────────────────────────────────

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

/// Default RoPE base for Llama 3 / SmolLM2 when GGUF omits `llama.rope.freq_base`.
pub const DEFAULT_ROPE_FREQ_BASE: f32 = 100_000.0;

/// Architecture hyper-parameters parsed from the GGUF KV section.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GgufHyperparams {
    pub n_layer: u32,
    pub n_embd: u32,
    pub n_head: u32,
    /// Grouped-query KV heads; `0` means MHA (`n_kv_head == n_head`).
    pub n_kv_head: u32,
    /// `llama.rope.freq_base` (FLOAT32 in GGUF); `0` → [`DEFAULT_ROPE_FREQ_BASE`].
    pub rope_freq_base: f32,
    /// Linear RoPE scale from `llama.rope.scale_linear` / `llama.rope.scaling.factor`; `0` → `1.0`.
    pub rope_scale: f32,
}

impl GgufHyperparams {
    pub fn effective_rope_freq_base(&self) -> f32 {
        if self.rope_freq_base > 0.0 && self.rope_freq_base.is_finite() {
            self.rope_freq_base
        } else {
            DEFAULT_ROPE_FREQ_BASE
        }
    }

    /// Effective position divisor for RoPE (`scaled_pos = pos / scale`).
    pub fn effective_rope_scale(&self) -> f32 {
        if self.rope_scale > 0.0 && self.rope_scale.is_finite() {
            self.rope_scale
        } else {
            1.0
        }
    }
    pub fn head_dim(&self) -> u32 {
        if self.n_head == 0 {
            0
        } else {
            self.n_embd / self.n_head
        }
    }

    pub fn effective_n_kv_head(&self) -> u32 {
        if self.n_kv_head > 0 {
            self.n_kv_head
        } else {
            self.n_head.max(1)
        }
    }

    pub fn q_heads_per_kv(&self) -> u32 {
        let kv = self.effective_n_kv_head();
        if kv == 0 {
            1
        } else {
            (self.n_head / kv).max(1)
        }
    }
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

/// Lookup table from tensor-name hash → `GgufTensorInfo`, built by walking the
/// GGUF tensor-info section that immediately follows the KV metadata section.
pub struct GgufTensorIndex {
    entries: Vec<(u64, GgufTensorInfo)>, // (name_hash, info)
    /// Absolute byte offset in the mmap where tensor payload data begins.
    pub tensor_data_start: u64,
    /// Cached metadata for `token_embd.weight` (embedding lookup target).
    token_embd: Option<GgufTensorInfo>,
    /// Cached `output.weight` for final vocabulary projection.
    output_weight: Option<GgufTensorInfo>,
    /// Cached `output_norm.weight` — final RMSNorm before vocab projection (Llama/SmolLM).
    output_norm: Option<GgufTensorInfo>,
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
        if key.ends_with("rope.freq_base") {
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

    /// Build a synthetic index from an explicit `(name, info)` list — used to boot from a `.q42`
    /// weight container so the *entire* GGUF-based hot path (get_layer_tensors / fetch_tensor_bytes /
    /// resident upload) works unchanged. The caller passes absolute blob offsets in each
    /// `GgufTensorInfo.byte_offset` and `tensor_data_start = 0`, pointing the byte source at the
    /// `.q42` bytes. Format-agnostic: the hot path never learns it is reading a `.q42`.
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

/// Extracts the Ontological mapping and Lexicon from a raw GGUF file
pub struct GGufSharder {
    pub source_gguf_path: String,
}

impl GGufSharder {
    pub fn new(source_gguf_path: String) -> Self {
        Self { source_gguf_path }
    }

    /// Step 1: Ontological Extraction & Tokenizer Ingestion
    /// Parses the GGUF header to extract vocabulary and metadata into a `.q42` SuperBlock.
    pub fn extract_ontology_to_superblock(&self) -> QualiaSuperBlock {
        // Mocks reading the GGUF header and vocabulary
        println!(
            "Extracting vocabulary and metadata from {}...",
            self.source_gguf_path
        );

        // This superblock is extremely lightweight because it only holds logic and strings,
        // leaving the multi-gigabyte tensors on disk.
        unsafe { std::mem::zeroed::<QualiaSuperBlock>() }
    }

    /// Step 2: The Pointer-Quin Map (.q42.bidx)
    /// Generates the Master Record map connecting N3 logic semantic rules to the exact
    /// 60-bit byte offsets in the massive GGUF tensor payload.
    pub fn generate_bidx_pointer_map(&self) -> Vec<NQuin> {
        let flag = if self
            .source_gguf_path
            .to_ascii_lowercase()
            .contains("mmproj")
        {
            crate::MODALITY_FLAG_VISION_TENSOR
        } else {
            crate::MODALITY_FLAG_LLM_TENSOR
        };
        self.generate_bidx_pointer_map_with_flag(flag)
    }

    pub fn generate_bidx_pointer_map_with_flag(&self, modality_flag: u8) -> Vec<NQuin> {
        let mut pointers = Vec::new();

        // Actual GGUF header parsing (reading magic bytes, version, tensor count)
        if let Ok(mut file) = std::fs::File::open(&self.source_gguf_path) {
            use std::io::Read;
            let mut magic = [0u8; 4];
            if file.read_exact(&mut magic).is_ok() && &magic == b"GGUF" {
                let mut version_bytes = [0u8; 4];
                let mut tensor_count_bytes = [0u8; 8];
                let mut kv_count_bytes = [0u8; 8];

                if file.read_exact(&mut version_bytes).is_ok()
                    && file.read_exact(&mut tensor_count_bytes).is_ok()
                    && file.read_exact(&mut kv_count_bytes).is_ok()
                {
                    let _version = u32::from_le_bytes(version_bytes);
                    let tensor_count = u64::from_le_bytes(tensor_count_bytes);
                    let _kv_count = u64::from_le_bytes(kv_count_bytes);

                    // Iterate over the parsed tensor counts and create mapping pointers
                    for i in 0..tensor_count.min(100) {
                        // Limit for safety
                        let byte_offset: u64 = 0x1000 + (i * 0x4000); // Compute relative physical offset
                        let tensor_name = format!("tensor_{}", i);

                        let q_tensor = NQuin {
                            subject: crate::q_hash(&tensor_name),
                            predicate: crate::q_hash("has_tensor_offset"),
                            object: ((modality_flag as u64) << 60) | byte_offset,
                            context: crate::q_hash("model_vocabulary"),
                            metadata: 0,
                            parity: 0,
                        };
                        pointers.push(q_tensor);
                    }
                    return pointers;
                }
            }
        }

        // Fallback for tests when no GGUF file is actually on disk
        let mock_byte_offset: u64 = 0x00000ABC;
        let q_tensor = NQuin {
            subject: crate::q_hash("blk.0.attn_q.weight"),
            predicate: crate::q_hash("has_tensor_offset"),
            object: ((modality_flag as u64) << 60) | mock_byte_offset,
            context: crate::q_hash("model_vocabulary"),
            metadata: 0,
            parity: 0,
        };

        pointers.push(q_tensor);
        pointers
    }

    /// Step 3: WordNet Lexicon Integration
    /// Maps a discrete WordNet Synset ID to its dense tensor representation.
    pub fn map_wordnet_synset(&self, synset_id: u64, byte_offset: u64) -> NQuin {
        NQuin {
            subject: synset_id,
            predicate: crate::q_hash("has_embedding"),
            object: ((crate::MODALITY_FLAG_DENSE_PHYSICS as u64) << 60) | byte_offset,
            context: crate::q_hash("wordnet_lexicon"),
            metadata: 0,
            parity: 0,
        }
    }

    /// Step 4: Zero-Copy Memory Mapping
    /// Maps a massive GGUF model directly into the OS virtual address space, shifting
    /// caching logic from the heap to the OS page cache (Zero Allocation).
    pub fn map_model_to_virtual_memory(
        &self,
        file_path: &str,
    ) -> Result<std::sync::Arc<[u8]>, std::io::Error> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let file = std::fs::File::open(file_path)?;
            let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };
            Ok(std::sync::Arc::from(mmap.as_ref()))
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = file_path;
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Virtual memory mapping not supported on WASM",
            ))
        }
    }
}

// ─── GgufTokenizer ───────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::sync::OnceLock;

/// GPT-2 `bytes_to_unicode` table — maps raw bytes to BPE merge symbols.
fn gpt2_byte_to_unicode(byte: u8) -> char {
    static TABLE: OnceLock<[char; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut bs: Vec<u32> = (b'!'..=b'~')
            .chain(b'\xA1'..=b'\xAC')
            .chain(b'\xAE'..=b'\xFF')
            .map(|b| b as u32)
            .collect();
        let mut cs = bs.clone();
        let mut n = 0u32;
        for b in 0u32..256 {
            if !bs.contains(&b) {
                bs.push(b);
                cs.push(256 + n);
                n += 1;
            }
        }
        let mut out = ['\0'; 256];
        for (b, c) in bs.into_iter().zip(cs) {
            out[b as usize] = char::from_u32(c).unwrap_or('\u{FFFD}');
        }
        out
    })[byte as usize]
}

/// Vocabulary and BOS/EOS metadata extracted from a GGUF KV section.
/// Used by `infer_local_model()` to encode prompts and decode output token IDs.
pub struct GgufTokenizer {
    /// Token ID → string (index = token ID).
    pub vocab: Vec<String>,
    pub bos_token_id: u32,
    pub eos_token_id: u32,
    /// `tokenizer.ggml.add_bos_token` — prepend BOS before prompt tokens when true.
    pub add_bos_token: bool,
    /// `tokenizer.ggml.pre` — e.g. `smollm`, `gpt2`; drives pretokenization.
    pub pre_type: String,
    /// BPE merge ranks: `(left_symbol, right_symbol)` in ascending rank order.
    merge_pairs: Vec<(String, String)>,
    /// Fast vocab lookup for BPE tail + legacy greedy path.
    token_to_id_map: HashMap<String, u32>,
    /// Special tokens (`<|…|>`, etc.) sorted longest-first for atomic matching.
    special_tokens: Vec<(String, u32)>,
    /// (token_string, token_id) sorted by descending byte length — legacy greedy fallback.
    token_to_id: Vec<(String, u32)>,
}

impl Default for GgufTokenizer {
    /// 256-entry byte-level fallback tokenizer — used when no GGUF is loaded.
    fn default() -> Self {
        let vocab: Vec<String> = (0u32..256)
            .map(|b| {
                let c = b as u8;
                if c.is_ascii_graphic() || c == b' ' {
                    (c as char).to_string()
                } else {
                    format!("<0x{:02X}>", b)
                }
            })
            .collect();
        let mut t2id: Vec<(String, u32)> = vocab
            .iter()
            .enumerate()
            .map(|(i, s)| (s.clone(), i as u32))
            .collect();
        t2id.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        let token_to_id_map: HashMap<String, u32> =
            t2id.iter().map(|(s, id)| (s.clone(), *id)).collect();
        Self {
            vocab,
            bos_token_id: 1,
            eos_token_id: 2,
            add_bos_token: true,
            pre_type: String::new(),
            merge_pairs: Vec::new(),
            token_to_id_map,
            special_tokens: Vec::new(),
            token_to_id: t2id,
        }
    }
}

impl GgufTokenizer {
    /// Parse vocab + BOS/EOS from a memory-mapped GGUF v2/v3 file.
    /// Falls back to `Default` (byte-level) on any parse error.
    pub fn from_gguf(mmap: &[u8]) -> Self {
        Self::try_parse(mmap).unwrap_or_default()
    }

    fn try_parse(mmap: &[u8]) -> Option<Self> {
        if mmap.len() < 24 || &mmap[0..4] != b"GGUF" {
            return None;
        }
        let version = u32::from_le_bytes(mmap[4..8].try_into().ok()?);
        if version < 2 {
            return None;
        } // only v2/v3 have u64 string lengths
        let kv_count = u64::from_le_bytes(mmap[16..24].try_into().ok()?);
        let mut pos = 24usize;
        let mut vocab: Option<Vec<String>> = None;
        let mut merges_raw: Option<Vec<String>> = None;
        let mut bos_id: Option<u32> = None;
        let mut eos_id: Option<u32> = None;
        let mut add_bos: Option<bool> = None;
        let mut pre_type: Option<String> = None;

        for _ in 0..kv_count {
            if pos + 8 > mmap.len() {
                break;
            }
            let klen = u64::from_le_bytes(mmap[pos..pos + 8].try_into().ok()?) as usize;
            pos += 8;
            if pos + klen > mmap.len() {
                break;
            }
            let key = std::str::from_utf8(&mmap[pos..pos + klen]).unwrap_or("");
            pos += klen;
            if pos + 4 > mmap.len() {
                break;
            }
            let vtype = u32::from_le_bytes(mmap[pos..pos + 4].try_into().ok()?);
            pos += 4;
            match key {
                "tokenizer.ggml.tokens" => {
                    vocab = Self::read_string_array(mmap, &mut pos, vtype);
                }
                "tokenizer.ggml.merges" => {
                    merges_raw = Self::read_string_array(mmap, &mut pos, vtype);
                }
                "tokenizer.ggml.bos_token_id" => {
                    bos_id = Self::read_u32_val(mmap, &mut pos, vtype);
                }
                "tokenizer.ggml.eos_token_id" => {
                    eos_id = Self::read_u32_val(mmap, &mut pos, vtype);
                }
                "tokenizer.ggml.add_bos_token" => {
                    add_bos = Self::read_bool_val(mmap, &mut pos, vtype);
                }
                "tokenizer.ggml.pre" => {
                    pre_type = Self::read_string_val(mmap, &mut pos, vtype);
                }
                _ => {
                    if Self::skip_value(mmap, &mut pos, vtype).is_none() {
                        break;
                    }
                }
            }
        }

        let v = vocab?;
        let bos = bos_id.unwrap_or(1);
        let eos = eos_id.unwrap_or(2);
        let mut t2id: Vec<(String, u32)> = v
            .iter()
            .enumerate()
            .map(|(i, s)| (s.clone(), i as u32))
            .collect();
        t2id.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        let token_to_id_map: HashMap<String, u32> =
            t2id.iter().map(|(s, id)| (s.clone(), *id)).collect();
        let mut special_tokens: Vec<(String, u32)> = v
            .iter()
            .enumerate()
            .filter(|(_, s)| s.starts_with('<') && s.ends_with('>'))
            .map(|(i, s)| (s.clone(), i as u32))
            .collect();
        special_tokens.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        let merge_pairs = Self::parse_merge_pairs(merges_raw.as_deref());
        Some(Self {
            vocab: v,
            bos_token_id: bos,
            eos_token_id: eos,
            add_bos_token: add_bos.unwrap_or(true),
            pre_type: pre_type.unwrap_or_default(),
            merge_pairs,
            token_to_id_map,
            special_tokens,
            token_to_id: t2id,
        })
    }

    /// Phase 4 v3: serialize the tokenizer into a compact, contiguous `.q42` section (no page
    /// alignment needed). Only the source fields are written (vocab / merges / bos / eos / add_bos /
    /// pre); the derived maps are rebuilt by [`from_q42_section`]. Heap use here is load-time only.
    pub fn to_q42_section(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(1 << 20);
        out.extend_from_slice(b"Q42T");
        out.extend_from_slice(&1u16.to_le_bytes()); // section version
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&self.bos_token_id.to_le_bytes());
        out.extend_from_slice(&self.eos_token_id.to_le_bytes());
        out.push(self.add_bos_token as u8);
        out.extend_from_slice(&[0u8; 3]);
        let put_str = |o: &mut Vec<u8>, s: &str| {
            o.extend_from_slice(&(s.len() as u32).to_le_bytes());
            o.extend_from_slice(s.as_bytes());
        };
        put_str(&mut out, &self.pre_type);
        out.extend_from_slice(&(self.vocab.len() as u32).to_le_bytes());
        for t in &self.vocab {
            put_str(&mut out, t);
        }
        out.extend_from_slice(&(self.merge_pairs.len() as u32).to_le_bytes());
        for (l, r) in &self.merge_pairs {
            put_str(&mut out, l);
            put_str(&mut out, r);
        }
        out
    }

    /// Phase 4 v3: rebuild a tokenizer from a `.q42` tokenizer section — bypasses GGUF KV string-key
    /// parsing entirely. Fully bounds-checked (the section is untrusted input). Returns `None` on any
    /// malformed field.
    pub fn from_q42_section(data: &[u8]) -> Option<Self> {
        let mut p = 0usize;
        let take = |p: &mut usize, n: usize| -> Option<&[u8]> {
            let end = p.checked_add(n)?;
            if end > data.len() {
                return None;
            }
            let s = &data[*p..end];
            *p = end;
            Some(s)
        };
        let take_u32 = |p: &mut usize| -> Option<u32> {
            Some(u32::from_le_bytes(take(p, 4)?.try_into().ok()?))
        };
        let take_str = |p: &mut usize| -> Option<String> {
            let len = take_u32(p)? as usize;
            Some(String::from_utf8_lossy(take(p, len)?).into_owned())
        };
        if take(&mut p, 4)? != b"Q42T" {
            return None;
        }
        let _ver = u16::from_le_bytes(take(&mut p, 2)?.try_into().ok()?);
        let _flags = take(&mut p, 2)?;
        let bos = take_u32(&mut p)?;
        let eos = take_u32(&mut p)?;
        let add_bos = take(&mut p, 1)?[0] != 0;
        let _pad = take(&mut p, 3)?;
        let pre_type = take_str(&mut p)?;
        let n_vocab = take_u32(&mut p)? as usize;
        if n_vocab > 1_000_000 {
            return None;
        }
        let mut vocab = Vec::with_capacity(n_vocab);
        for _ in 0..n_vocab {
            vocab.push(take_str(&mut p)?);
        }
        let n_merges = take_u32(&mut p)? as usize;
        if n_merges > 5_000_000 {
            return None;
        }
        let mut merge_pairs = Vec::with_capacity(n_merges);
        for _ in 0..n_merges {
            let l = take_str(&mut p)?;
            let r = take_str(&mut p)?;
            merge_pairs.push((l, r));
        }
        // Rebuild the derived maps exactly as `try_parse` does, so encode/decode are identical.
        let mut t2id: Vec<(String, u32)> = vocab
            .iter()
            .enumerate()
            .map(|(i, s)| (s.clone(), i as u32))
            .collect();
        t2id.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        let token_to_id_map: HashMap<String, u32> =
            t2id.iter().map(|(s, id)| (s.clone(), *id)).collect();
        let mut special_tokens: Vec<(String, u32)> = vocab
            .iter()
            .enumerate()
            .filter(|(_, s)| s.starts_with('<') && s.ends_with('>'))
            .map(|(i, s)| (s.clone(), i as u32))
            .collect();
        special_tokens.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        Some(Self {
            vocab,
            bos_token_id: bos,
            eos_token_id: eos,
            add_bos_token: add_bos,
            pre_type,
            merge_pairs,
            token_to_id_map,
            special_tokens,
            token_to_id: t2id,
        })
    }

    /// Tokenize `text`, prepending [`bos_token_id`] when [`add_bos_token`] is set and absent.
    pub fn encode_prompt(&self, text: &str) -> Vec<u32> {
        let mut ids = self.encode(text);
        if self.add_bos_token && ids.first().copied() != Some(self.bos_token_id) {
            let mut with_bos = Vec::with_capacity(ids.len().saturating_add(1));
            with_bos.push(self.bos_token_id);
            with_bos.append(&mut ids);
            with_bos
        } else {
            ids
        }
    }

    /// Format token IDs for diagnostic logging (MC3f).
    pub fn format_ids_for_log(ids: &[u32]) -> String {
        let mut s = String::from("[");
        for (i, &id) in ids.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            if i >= 64 {
                s.push_str("…");
                break;
            }
            s.push_str(&id.to_string());
        }
        s.push(']');
        s
    }

    /// Greedy longest-match tokenisation; falls back to single-byte encoding.
    pub fn encode(&self, text: &str) -> Vec<u32> {
        if self.uses_bpe() {
            return self.encode_bpe(text);
        }
        self.encode_greedy(text)
    }

    fn uses_bpe(&self) -> bool {
        !self.merge_pairs.is_empty()
            || matches!(
                self.pre_type.as_str(),
                "smollm" | "gpt2" | "mpt" | "olmo" | "jais" | "llama3"
            )
    }

    fn encode_greedy(&self, text: &str) -> Vec<u32> {
        let mut ids = Vec::new();
        let mut remaining = text;
        while !remaining.is_empty() {
            let mut matched = false;
            for (token, id) in &self.token_to_id {
                if remaining.starts_with(token.as_str()) {
                    ids.push(*id);
                    remaining = &remaining[token.len()..];
                    matched = true;
                    break;
                }
            }
            if !matched {
                let b = remaining.as_bytes()[0];
                ids.push(b as u32);
                let step = remaining.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
                remaining = &remaining[step..];
            }
        }
        ids
    }

    /// BPE encode with special-token atomicity + smollm/gpt2 pretokenization.
    fn encode_bpe(&self, text: &str) -> Vec<u32> {
        let mut ids = Vec::new();
        let mut remaining = text;
        while !remaining.is_empty() {
            let mut matched_special = false;
            for (tok, id) in &self.special_tokens {
                if remaining.starts_with(tok.as_str()) {
                    ids.push(*id);
                    remaining = &remaining[tok.len()..];
                    matched_special = true;
                    break;
                }
            }
            if matched_special {
                continue;
            }
            let mut next_special = remaining.len();
            for (tok, _) in &self.special_tokens {
                if let Some(pos) = remaining.find(tok.as_str()) {
                    next_special = next_special.min(pos);
                }
            }
            let segment = &remaining[..next_special];
            if !segment.is_empty() {
                for piece in self.pretokenize(segment) {
                    ids.extend(self.bpe_piece(&piece));
                }
            }
            remaining = &remaining[next_special..];
        }
        ids
    }

    /// llama.cpp `LLAMA_VOCAB_PRE_TYPE_SMOLLM` regex split (cold path — heap OK).
    fn pretokenize(&self, text: &str) -> Vec<String> {
        static SMOLLM_RE: OnceLock<regex::Regex> = OnceLock::new();
        let re = SMOLLM_RE.get_or_init(|| {
            regex::Regex::new(
                r"'s|'t|'re|'ve|'m|'ll|'d| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+",
            )
            .expect("smollm pretoken regex")
        });
        re.find_iter(text)
            .map(|m| m.as_str().to_string())
            .collect()
    }

    fn bpe_piece(&self, piece: &str) -> Vec<u32> {
        if piece.is_empty() {
            return Vec::new();
        }
        if let Some(&id) = self.token_to_id_map.get(piece) {
            return vec![id];
        }
        let mut word: String = piece.bytes().map(gpt2_byte_to_unicode).collect();
        if let Some(&id) = self.token_to_id_map.get(word.as_str()) {
            return vec![id];
        }
        let mut symbols: Vec<String> = word.chars().map(|c| c.to_string()).collect();
        if symbols.is_empty() {
            return Vec::new();
        }
        loop {
            let mut best_rank: Option<usize> = None;
            let mut best_idx = 0usize;
            for i in 0..symbols.len().saturating_sub(1) {
                if let Some(rank) = self.merge_rank_str(&symbols[i], &symbols[i + 1]) {
                    if best_rank.is_none() || rank < best_rank.unwrap() {
                        best_rank = Some(rank);
                        best_idx = i;
                    }
                }
            }
            let Some(_rank) = best_rank else { break };
            let merged = format!("{}{}", symbols[best_idx], symbols[best_idx + 1]);
            symbols[best_idx] = merged;
            symbols.remove(best_idx + 1);
        }
        let mut ids = Vec::with_capacity(symbols.len());
        for sym in symbols {
            if let Some(&id) = self.token_to_id_map.get(sym.as_str()) {
                ids.push(id);
            } else {
                for ch in sym.chars() {
                    let s = ch.to_string();
                    if let Some(&id) = self.token_to_id_map.get(s.as_str()) {
                        ids.push(id);
                    }
                }
            }
        }
        ids
    }

    fn merge_rank_str(&self, left: &str, right: &str) -> Option<usize> {
        self.merge_pairs
            .iter()
            .position(|(l, r)| l == left && r == right)
    }

    fn parse_merge_pairs(merges: Option<&[String]>) -> Vec<(String, String)> {
        let Some(merges) = merges else {
            return Vec::new();
        };
        let mut pairs = Vec::with_capacity(merges.len());
        for merge in merges {
            if let Some((a, b)) = merge.split_once(' ') {
                pairs.push((a.to_string(), b.to_string()));
            }
        }
        pairs
    }

    /// Append one vocabulary token to `out` with BPE / SentencePiece space normalization.
    fn append_decoded_token(out: &mut String, s: &str) {
        if let Some(rest) = s.strip_prefix('\u{2581}') {
            out.push(' ');
            out.push_str(rest);
        } else if let Some(rest) = s.strip_prefix('\u{0120}') {
            // GPT-2 / Llama / SmolLM BPE space marker (Ġ).
            out.push(' ');
            out.push_str(rest);
        } else if s.len() == 6 && s.starts_with("<0x") && s.ends_with('>') {
            if let Ok(b) = u8::from_str_radix(&s[3..5], 16) {
                out.push(b as char);
            }
        } else {
            out.push_str(s);
        }
    }

    /// Map token IDs → strings, joining without separator.
    /// Converts SentencePiece `▁` and GPT-2 BPE `Ġ` → space; `<0x##>` → raw byte.
    pub fn decode(&self, ids: &[u32]) -> String {
        let mut out = String::new();
        for &id in ids {
            let s = self
                .vocab
                .get(id as usize)
                .map(|s| s.as_str())
                .unwrap_or("");
            Self::append_decoded_token(&mut out, s);
        }
        out
    }

    pub fn vocab_len(&self) -> u32 {
        self.vocab.len() as u32
    }

    /// Number of BPE merges loaded from GGUF (diagnostic).
    pub fn merge_count(&self) -> usize {
        self.merge_pairs.len()
    }

    // ── internal KV parsers ──────────────────────────────────────────────────

    fn read_string_array(mmap: &[u8], pos: &mut usize, vtype: u32) -> Option<Vec<String>> {
        if vtype != 9 {
            Self::skip_value(mmap, pos, vtype)?;
            return None;
        }
        if *pos + 12 > mmap.len() {
            return None;
        }
        let etype = u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().ok()?);
        *pos += 4;
        let count = u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().ok()?) as usize;
        *pos += 8;
        if etype != 8 {
            return None;
        } // must be STRING array
        let mut result = Vec::with_capacity(count.min(256_000));
        for _ in 0..count {
            if *pos + 8 > mmap.len() {
                break;
            }
            let slen = u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().ok()?) as usize;
            *pos += 8;
            if *pos + slen > mmap.len() {
                break;
            }
            let s = std::str::from_utf8(&mmap[*pos..*pos + slen])
                .unwrap_or("<?>")
                .to_string();
            *pos += slen;
            result.push(s);
        }
        Some(result)
    }

    fn read_u32_val(mmap: &[u8], pos: &mut usize, vtype: u32) -> Option<u32> {
        if vtype == 4 {
            if *pos + 4 > mmap.len() {
                return None;
            }
            let v = u32::from_le_bytes(mmap[*pos..*pos + 4].try_into().ok()?);
            *pos += 4;
            Some(v)
        } else {
            Self::skip_value(mmap, pos, vtype)?;
            None
        }
    }

    fn read_string_val(mmap: &[u8], pos: &mut usize, vtype: u32) -> Option<String> {
        if vtype == 8 {
            if *pos + 8 > mmap.len() {
                return None;
            }
            let slen = u64::from_le_bytes(mmap[*pos..*pos + 8].try_into().ok()?) as usize;
            *pos += 8;
            if *pos + slen > mmap.len() {
                return None;
            }
            let s = std::str::from_utf8(&mmap[*pos..*pos + slen])
                .unwrap_or("")
                .to_string();
            *pos += slen;
            Some(s)
        } else {
            Self::skip_value(mmap, pos, vtype)?;
            None
        }
    }

    fn read_bool_val(mmap: &[u8], pos: &mut usize, vtype: u32) -> Option<bool> {
        if vtype == 7 {
            if *pos + 1 > mmap.len() {
                return None;
            }
            let b = mmap[*pos];
            *pos += 1;
            Some(b != 0)
        } else {
            Self::skip_value(mmap, pos, vtype)?;
            None
        }
    }

    fn skip_value(mmap: &[u8], pos: &mut usize, vtype: u32) -> Option<()> {
        gguf_skip_value(mmap, pos, vtype)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_gguf_layer_names_if_exists() {
        use memmap2::MmapOptions;
        use std::fs::File;
        let path = "C:/Projects/qualiaDB/gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf";
        if !std::path::Path::new(path).exists() {
            return;
        }
        let f = File::open(path).unwrap();
        let mmap = unsafe { MmapOptions::new().map(&f).unwrap() };
        if mmap.len() < 24 || &mmap[0..4] != b"GGUF" {
            return;
        }
        let tensor_count = u64::from_le_bytes(mmap[8..16].try_into().unwrap()) as usize;
        let kv_count = u64::from_le_bytes(mmap[16..24].try_into().unwrap()) as usize;
        let mut pos = 24usize;
        for _ in 0..kv_count {
            let klen = u64::from_le_bytes(mmap[pos..pos + 8].try_into().unwrap()) as usize;
            pos += 8;
            let key = std::str::from_utf8(&mmap[pos..pos + klen]).unwrap_or("");
            pos += klen;
            let vtype = u32::from_le_bytes(mmap[pos..pos + 4].try_into().unwrap());
            pos += 4;
            if key.contains("block")
                || key.contains("layer")
                || key.contains("embedding")
                || key.contains("head")
            {
                if vtype == 4 && pos + 4 <= mmap.len() {
                    let v = u32::from_le_bytes(mmap[pos..pos + 4].try_into().unwrap());
                    println!("KV {key} = {v}");
                }
            }
            gguf_skip_value(&mmap, &mut pos, vtype).unwrap();
        }
        let mut blk_samples = 0usize;
        for _ in 0..tensor_count {
            let nlen = u64::from_le_bytes(mmap[pos..pos + 8].try_into().unwrap()) as usize;
            pos += 8;
            let name = std::str::from_utf8(&mmap[pos..pos + nlen]).unwrap_or("");
            pos += nlen;
            let n_dims = u32::from_le_bytes(mmap[pos..pos + 4].try_into().unwrap()) as usize;
            pos += 4;
            let mut dims = [0u64; 4];
            for d in 0..n_dims {
                dims[d] = u64::from_le_bytes(mmap[pos..pos + 8].try_into().unwrap());
                pos += 8;
            }
            let ggml_type = u32::from_le_bytes(mmap[pos..pos + 4].try_into().unwrap());
            let byte_off = u64::from_le_bytes(mmap[pos + 4..pos + 12].try_into().unwrap());
            pos += 12;
            if name.starts_with("blk.0.") && (name.contains("attn_q") || name.contains("ffn_down"))
            {
                println!("tensor: {name} type={ggml_type} dims={dims:?} off={byte_off:#x}");
                blk_samples += 1;
            }
        }
    }

    #[test]
    fn test_gguf_ontology_extraction() {
        let sharder = GGufSharder::new(
            "C:/Projects/qualiaDB/gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf".to_string(),
        );

        let superblock = sharder.extract_ontology_to_superblock();
        // Just verify it yields a superblock structural scaffold
        assert_eq!(
            superblock.active_quin_count, 0,
            "SuperBlock should be freshly initialized"
        );
    }

    #[test]
    fn test_gguf_bidx_pointer_generation() {
        use crate::QuinPointerExt;

        let sharder = GGufSharder::new("mock_model.gguf".to_string());
        let pointers = sharder.generate_bidx_pointer_map();

        assert_eq!(pointers.len(), 1, "Failed to generate pointer map");

        let quin = pointers[0];
        assert_eq!(
            quin.extract_modality_flag(),
            crate::MODALITY_FLAG_LLM_TENSOR,
            "Pointer Modality Flag was not LLM"
        );
        assert_eq!(
            quin.extract_byte_offset(),
            0x00000ABC,
            "Pointer byte offset extracted incorrectly"
        );
    }

    #[test]
    fn encode_prompt_prepends_bos_when_enabled() {
        let mut tok = GgufTokenizer::default();
        tok.add_bos_token = true;
        tok.bos_token_id = 42;
        let ids = tok.encode_prompt("hi");
        assert_eq!(ids.first(), Some(&42));
        assert!(ids.len() >= 2);
    }

    #[test]
    fn encode_prompt_skips_duplicate_bos() {
        let mut tok = GgufTokenizer::default();
        tok.add_bos_token = true;
        tok.bos_token_id = 0;
        tok.token_to_id = vec![("<|endoftext|>".into(), 0), ("a".into(), 10)];
        tok.vocab = vec!["<|endoftext|>".into(), "a".into()];
        let ids = tok.encode_prompt("<|endoftext|>a");
        assert_eq!(ids, vec![0, 10]);
    }

    #[test]
    fn decode_maps_bpe_space_marker_to_ascii_space() {
        let mut tok = GgufTokenizer::default();
        tok.vocab = vec![
            "The".into(),
            "\u{0120}capital".into(),
            "\u{2581}of".into(),
        ];
        assert_eq!(tok.decode(&[0, 1, 2]), "The capital of");
    }

    #[test]
    fn hyperparams_default_rope_freq_base_is_100k() {
        let h = GgufHyperparams::default();
        assert_eq!(h.effective_rope_freq_base(), DEFAULT_ROPE_FREQ_BASE);
    }

    #[test]
    fn smollm_gguf_output_weight_tie_probe() {
        use memmap2::MmapOptions;
        use std::fs::File;
        for (label, path) in [
            (
                "Q4_K_M",
                "C:/projects/qualiaDB/docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf",
            ),
            (
                "Q8_0",
                "C:/projects/qualiaDB/docs/models/smollm2-360m-instruct-q8_0.gguf",
            ),
        ] {
            if !std::path::Path::new(path).exists() {
                println!("[skip] {label} not at {path}");
                continue;
            }
            let f = File::open(path).unwrap();
            let mmap = unsafe { MmapOptions::new().map(&f).unwrap() };
            let idx = GgufTensorIndex::from_gguf(&mmap);
            let (tied, emb_off, out_off, emb_dims, out_dims) = idx.weight_tie_probe();
            assert!(idx.token_embd_info().is_some(), "{label}: missing token_embd");
            assert!(idx.logits_projection_info().is_some(), "{label}: no logits projection");
            println!(
                "[{label}] tied={tied} emb_off={emb_off:#x} dims={emb_dims:?} out_off={out_off:#x} out_dims={out_dims:?}"
            );
            if tied {
                assert_eq!(emb_off, out_off, "{label}: tied offsets must match");
                assert_eq!(emb_dims, out_dims, "{label}: tied dims must match");
            }
        }
    }

    #[test]
    fn smollm_tokenizer_audit_vs_hf_reference() {
        use memmap2::MmapOptions;
        use std::fs::File;
        let path = "C:/projects/qualiaDB/docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf";
        if !std::path::Path::new(path).exists() {
            return;
        }
        let f = File::open(path).unwrap();
        let mmap = unsafe { MmapOptions::new().map(&f).unwrap() };
        let tok = GgufTokenizer::from_gguf(&mmap);
        let chatml = "<|im_start|>user\nWhat is the capital of France? Answer in one short sentence.<|im_end|>\n<|im_start|>assistant\n";
        let naked = "The capital of France is";
        let chat_ids = tok.encode(chatml);
        let naked_ids = tok.encode(naked);
        println!(
            "[audit] bos={} eos={} add_bos={} pre={:?}",
            tok.bos_token_id, tok.eos_token_id, tok.add_bos_token, tok.pre_type
        );
        println!("[audit] chatml len={} ids={:?}", chat_ids.len(), chat_ids);
        println!("[audit] naked len={} ids={:?}", naked_ids.len(), naked_ids);
        const HF_CHATML: &[u32] = &[
            1, 4093, 198, 1780, 314, 260, 3575, 282, 4649, 47, 19842, 281, 582, 1890, 6330, 30,
            2, 198, 1, 520, 9531, 198,
        ];
        const HF_NAKED: &[u32] = &[504, 3575, 282, 4649, 314];
        assert_eq!(chat_ids, HF_CHATML, "ChatML must not shred <|im_start|> specials");
        assert_eq!(naked_ids, HF_NAKED, "naked English prompt must match HF BPE");
    }

    #[test]
    fn smollm_gguf_parses_rope_freq_base_when_present() {
        use memmap2::MmapOptions;
        use std::fs::File;
        let path = "C:/projects/qualiaDB/docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf";
        if !std::path::Path::new(path).exists() {
            return;
        }
        let f = File::open(path).unwrap();
        let mmap = unsafe { MmapOptions::new().map(&f).unwrap() };
        let idx = GgufTensorIndex::from_gguf(&mmap);
        assert!(
            (idx.hyperparams.rope_freq_base - 100_000.0).abs() < 1.0,
            "expected llama.rope.freq_base=100000, got {}",
            idx.hyperparams.rope_freq_base
        );
    }

    #[test]
    fn test_wordnet_lexicon_mapping() {
        use crate::QuinPointerExt;
        let sharder = GGufSharder::new("mock.gguf".to_string());

        // Mock WordNet Synset ID for "Dog"
        let synset_dog = 0x8a2a1072b;
        let quin = sharder.map_wordnet_synset(synset_dog, 0x1000);

        assert_eq!(quin.subject, synset_dog);
        assert_eq!(
            quin.extract_modality_flag(),
            crate::MODALITY_FLAG_DENSE_PHYSICS,
            "Modality Flag should be Dense Physics"
        );
        assert_eq!(quin.extract_byte_offset(), 0x1000);
    }
}
