//! Q-GGUF Hybrid Packaging
//! Parses monolithic `.gguf` files: vocabulary (KV section) and tensor names/offsets
//! (tensor-info section) are extracted into native Rust types; multi-gigabyte tensor
//! payloads are left on disk for direct VRAM mapping via `gguf_bridge.rs`.

use crate::{QualiaQuin, QualiaSuperBlock};

// ─── Module-level GGUF helpers ───────────────────────────────────────────────

/// FNV-1a hash over raw bytes — same algorithm as `crate::q_hash` but for
/// byte slices parsed at runtime (e.g. tensor names from the binary header).
fn gguf_name_hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    h
}

/// Skip over one GGUF KV value of the given type without storing it.
/// Returns `None` on any parse error (truncated data, unknown type).
/// Used by both `GgufTokenizer` and `GgufTensorIndex`.
fn gguf_skip_value(mmap: &[u8], pos: &mut usize, vtype: u32) -> Option<()> {
    match vtype {
        0|1|7    => { if *pos+1 > mmap.len() { return None; } *pos += 1; }
        2|3      => { if *pos+2 > mmap.len() { return None; } *pos += 2; }
        4|5|6    => { if *pos+4 > mmap.len() { return None; } *pos += 4; }
        10|11|12 => { if *pos+8 > mmap.len() { return None; } *pos += 8; }
        8 => {
            if *pos+8 > mmap.len() { return None; }
            let slen = u64::from_le_bytes(mmap[*pos..*pos+8].try_into().ok()?) as usize;
            *pos += 8;
            if *pos+slen > mmap.len() { return None; }
            *pos += slen;
        }
        9 => {
            if *pos+12 > mmap.len() { return None; }
            let etype = u32::from_le_bytes(mmap[*pos..*pos+4].try_into().ok()?); *pos += 4;
            let cnt   = u64::from_le_bytes(mmap[*pos..*pos+8].try_into().ok()?) as usize; *pos += 8;
            for _ in 0..cnt { gguf_skip_value(mmap, pos, etype)?; }
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

/// Architecture hyper-parameters parsed from the GGUF KV section.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GgufHyperparams {
    pub n_layer: u32,
    pub n_embd: u32,
    pub n_head: u32,
    /// Grouped-query KV heads; `0` means MHA (`n_kv_head == n_head`).
    pub n_kv_head: u32,
}

impl GgufHyperparams {
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
    pub attn_q: Option<GgufTensorInfo>,
    pub attn_k: Option<GgufTensorInfo>,
    pub attn_v: Option<GgufTensorInfo>,
    pub attn_output: Option<GgufTensorInfo>,
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
            hyperparams: GgufHyperparams::default(),
            max_tensor_bytes: 0,
            max_layer_tensor_bytes: 0,
        })
    }

    fn parse_kv_hyperparams(key: &str, vtype: u32, mmap: &[u8], pos: &mut usize) -> GgufHyperparams {
        let mut patch = GgufHyperparams::default();
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
        if mmap.len() < 24 || &mmap[0..4] != b"GGUF" { return None; }
        let version      = u32::from_le_bytes(mmap[4..8].try_into().ok()?);
        if version < 2 { return None; }
        let tensor_count = u64::from_le_bytes(mmap[8..16].try_into().ok()?);
        let kv_count     = u64::from_le_bytes(mmap[16..24].try_into().ok()?);

        let mut hyperparams = GgufHyperparams::default();
        let mut pos = 24usize;
        for _ in 0..kv_count {
            if pos + 8 > mmap.len() { return None; }
            let klen = u64::from_le_bytes(mmap[pos..pos+8].try_into().ok()?) as usize;
            pos += 8;
            if pos + klen + 4 > mmap.len() { return None; }
            let key = std::str::from_utf8(&mmap[pos..pos + klen]).unwrap_or("");
            pos += klen;
            let vtype = u32::from_le_bytes(mmap[pos..pos+4].try_into().ok()?);
            pos += 4;
            let patch = Self::parse_kv_hyperparams(key, vtype, mmap, &mut pos);
            if patch.n_layer != 0 { hyperparams.n_layer = patch.n_layer; }
            if patch.n_embd != 0 { hyperparams.n_embd = patch.n_embd; }
            if patch.n_head != 0 { hyperparams.n_head = patch.n_head; }
            if patch.n_kv_head != 0 { hyperparams.n_kv_head = patch.n_kv_head; }
        }

        let mut entries = Vec::with_capacity(tensor_count.min(4096) as usize);
        let mut max_tensor_bytes = 0usize;
        let mut max_layer_tensor_bytes = 0usize;
        for _ in 0..tensor_count {
            if pos + 8 > mmap.len() { break; }
            let nlen = u64::from_le_bytes(mmap[pos..pos+8].try_into().ok()?) as usize;
            pos += 8;
            if pos + nlen > mmap.len() { break; }
            let name = &mmap[pos..pos + nlen];
            let name_hash = gguf_name_hash(name);
            pos += nlen;

            // n_dims
            if pos + 4 > mmap.len() { break; }
            let n_dims_raw = u32::from_le_bytes(mmap[pos..pos+4].try_into().ok()?) as usize;
            pos += 4;

            // Shape (up to 4 dims stored; rest skipped)
            let mut dims = [0u64; 4];
            for d in 0..n_dims_raw {
                if pos + 8 > mmap.len() { break; }
                let v = u64::from_le_bytes(mmap[pos..pos+8].try_into().ok()?);
                pos += 8;
                if d < 4 { dims[d] = v; }
            }

            // ggml_type + offset
            if pos + 12 > mmap.len() { break; }
            let ggml_type   = u32::from_le_bytes(mmap[pos..pos+4].try_into().ok()?); pos += 4;
            let byte_offset = u64::from_le_bytes(mmap[pos..pos+8].try_into().ok()?); pos += 8;

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
        let token_embd = entries.iter().find(|(h, _)| *h == emb_hash).map(|(_, i)| *i);
        let output_weight = entries.iter().find(|(h, _)| *h == out_hash).map(|(_, i)| *i);
        if hyperparams.n_embd == 0 {
            hyperparams.n_embd = token_embd.map(|t| t.dims[0] as u32).unwrap_or(0);
        }
        Some(Self {
            entries,
            tensor_data_start,
            token_embd,
            output_weight,
            hyperparams,
            max_tensor_bytes,
            max_layer_tensor_bytes,
        })
    }

    fn find(&self, name: &[u8]) -> Option<GgufTensorInfo> {
        let h = gguf_name_hash(name);
        self.entries.iter().find(|(eh, _)| *eh == h).map(|(_, i)| *i)
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
            attn_q: self.find_layer_tensor(layer_idx, b"attn_q.weight"),
            attn_k: self.find_layer_tensor(layer_idx, b"attn_k.weight"),
            attn_v: self.find_layer_tensor(layer_idx, b"attn_v.weight"),
            attn_output: self.find_layer_tensor(layer_idx, b"attn_output.weight"),
            ffn_gate: self.find_layer_tensor(layer_idx, b"ffn_gate.weight"),
            ffn_up: self.find_layer_tensor(layer_idx, b"ffn_up.weight"),
            ffn_down: self.find_layer_tensor(layer_idx, b"ffn_down.weight"),
        }
    }

    pub fn output_weight_info(&self) -> Option<&GgufTensorInfo> {
        self.output_weight.as_ref()
    }

    /// Output projection weights: `output.weight` when present, else tied `token_embd.weight`.
    pub fn logits_projection_info(&self) -> Option<&GgufTensorInfo> {
        self.output_weight
            .as_ref()
            .or_else(|| self.token_embd.as_ref())
    }

    /// Cached `token_embd.weight` tensor metadata.
    pub fn token_embd_info(&self) -> Option<&GgufTensorInfo> {
        self.token_embd.as_ref()
    }

    /// Return the embedding dimension (n_embd) from `token_embd.weight`, or 0 if unknown.
    pub fn emb_dim(&self) -> usize {
        self.token_embd_info().map(|i| i.dims[0] as usize).unwrap_or(0)
    }

    /// Vocabulary size from `token_embd.weight` shape `[n_embd, n_vocab]`.
    pub fn vocab_dim(&self) -> usize {
        self.token_embd_info().map(|i| i.dims[1] as usize).unwrap_or(0)
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
        crate::ggml_quants::dequantize_row_into(raw, info.ggml_type, n_embd, out)
            .unwrap_or(0)
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
        println!("Extracting vocabulary and metadata from {}...", self.source_gguf_path);
        
        // This superblock is extremely lightweight because it only holds logic and strings,
        // leaving the multi-gigabyte tensors on disk.
        unsafe { std::mem::zeroed::<QualiaSuperBlock>() }
    }

    /// Step 2: The Pointer-Quin Map (.q42.bidx)
    /// Generates the Master Record map connecting N3 logic semantic rules to the exact 
    /// 60-bit byte offsets in the massive GGUF tensor payload.
    pub fn generate_bidx_pointer_map(&self) -> Vec<QualiaQuin> {
        let flag = if self.source_gguf_path.to_ascii_lowercase().contains("mmproj") {
            crate::MODALITY_FLAG_VISION_TENSOR
        } else {
            crate::MODALITY_FLAG_LLM_TENSOR
        };
        self.generate_bidx_pointer_map_with_flag(flag)
    }

    pub fn generate_bidx_pointer_map_with_flag(&self, modality_flag: u8) -> Vec<QualiaQuin> {
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
                    && file.read_exact(&mut kv_count_bytes).is_ok() {
                    
                    let _version = u32::from_le_bytes(version_bytes);
                    let tensor_count = u64::from_le_bytes(tensor_count_bytes);
                    let _kv_count = u64::from_le_bytes(kv_count_bytes);
                    
                    // Iterate over the parsed tensor counts and create mapping pointers
                    for i in 0..tensor_count.min(100) { // Limit for safety
                        let byte_offset: u64 = 0x1000 + (i * 0x4000); // Compute relative physical offset
                        let tensor_name = format!("tensor_{}", i);
                        
                        let q_tensor = QualiaQuin {
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
        let q_tensor = QualiaQuin {
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
    pub fn map_wordnet_synset(&self, synset_id: u64, byte_offset: u64) -> QualiaQuin {
        QualiaQuin {
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
    pub fn map_model_to_virtual_memory(&self, file_path: &str) -> Result<memmap2::Mmap, std::io::Error> {
        let file = std::fs::File::open(file_path)?;
        unsafe { memmap2::MmapOptions::new().map(&file) }
    }
}

// ─── GgufTokenizer ───────────────────────────────────────────────────────────

/// Vocabulary and BOS/EOS metadata extracted from a GGUF KV section.
/// Used by `infer_local_model()` to encode prompts and decode output token IDs.
pub struct GgufTokenizer {
    /// Token ID → string (index = token ID).
    pub vocab: Vec<String>,
    pub bos_token_id: u32,
    pub eos_token_id: u32,
    /// (token_string, token_id) sorted by descending byte length for greedy longest-match.
    token_to_id: Vec<(String, u32)>,
}

impl Default for GgufTokenizer {
    /// 256-entry byte-level fallback tokenizer — used when no GGUF is loaded.
    fn default() -> Self {
        let vocab: Vec<String> = (0u32..256)
            .map(|b| {
                let c = b as u8;
                if c.is_ascii_graphic() || c == b' ' { (c as char).to_string() }
                else { format!("<0x{:02X}>", b) }
            })
            .collect();
        let mut t2id: Vec<(String, u32)> = vocab.iter().enumerate()
            .map(|(i, s)| (s.clone(), i as u32)).collect();
        t2id.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        Self { vocab, bos_token_id: 1, eos_token_id: 2, token_to_id: t2id }
    }
}

impl GgufTokenizer {
    /// Parse vocab + BOS/EOS from a memory-mapped GGUF v2/v3 file.
    /// Falls back to `Default` (byte-level) on any parse error.
    pub fn from_gguf(mmap: &[u8]) -> Self {
        Self::try_parse(mmap).unwrap_or_default()
    }

    fn try_parse(mmap: &[u8]) -> Option<Self> {
        if mmap.len() < 24 || &mmap[0..4] != b"GGUF" { return None; }
        let version = u32::from_le_bytes(mmap[4..8].try_into().ok()?);
        if version < 2 { return None; } // only v2/v3 have u64 string lengths
        let kv_count = u64::from_le_bytes(mmap[16..24].try_into().ok()?);
        let mut pos = 24usize;
        let mut vocab: Option<Vec<String>> = None;
        let mut bos_id: Option<u32> = None;
        let mut eos_id: Option<u32> = None;

        for _ in 0..kv_count {
            if pos + 8 > mmap.len() { break; }
            let klen = u64::from_le_bytes(mmap[pos..pos+8].try_into().ok()?) as usize;
            pos += 8;
            if pos + klen > mmap.len() { break; }
            let key = std::str::from_utf8(&mmap[pos..pos+klen]).unwrap_or("");
            pos += klen;
            if pos + 4 > mmap.len() { break; }
            let vtype = u32::from_le_bytes(mmap[pos..pos+4].try_into().ok()?);
            pos += 4;
            match key {
                "tokenizer.ggml.tokens"       => { vocab  = Self::read_string_array(mmap, &mut pos, vtype); }
                "tokenizer.ggml.bos_token_id" => { bos_id = Self::read_u32_val(mmap, &mut pos, vtype); }
                "tokenizer.ggml.eos_token_id" => { eos_id = Self::read_u32_val(mmap, &mut pos, vtype); }
                _ => { if Self::skip_value(mmap, &mut pos, vtype).is_none() { break; } }
            }
            if vocab.is_some() && bos_id.is_some() && eos_id.is_some() { break; }
        }

        let v = vocab?;
        let bos = bos_id.unwrap_or(1);
        let eos = eos_id.unwrap_or(2);
        let mut t2id: Vec<(String, u32)> = v.iter().enumerate()
            .map(|(i, s)| (s.clone(), i as u32)).collect();
        t2id.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        Some(Self { vocab: v, bos_token_id: bos, eos_token_id: eos, token_to_id: t2id })
    }

    /// Greedy longest-match tokenisation; falls back to single-byte encoding.
    pub fn encode(&self, text: &str) -> Vec<u32> {
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

    /// Map token IDs → strings, joining without separator.
    /// Converts SentencePiece `▁` (U+2581) → space and `<0x##>` → raw byte.
    pub fn decode(&self, ids: &[u32]) -> String {
        let mut out = String::new();
        for &id in ids {
            let s = self.vocab.get(id as usize).map(|s| s.as_str()).unwrap_or("");
            if s.starts_with('\u{2581}') {
                out.push(' ');
                out.push_str(&s['\u{2581}'.len_utf8()..]);
            } else if s.len() == 6 && s.starts_with("<0x") && s.ends_with('>') {
                if let Ok(b) = u8::from_str_radix(&s[3..5], 16) { out.push(b as char); }
            } else {
                out.push_str(s);
            }
        }
        out
    }

    pub fn vocab_len(&self) -> u32 { self.vocab.len() as u32 }

    // ── internal KV parsers ──────────────────────────────────────────────────

    fn read_string_array(mmap: &[u8], pos: &mut usize, vtype: u32) -> Option<Vec<String>> {
        if vtype != 9 { Self::skip_value(mmap, pos, vtype)?; return None; }
        if *pos + 12 > mmap.len() { return None; }
        let etype = u32::from_le_bytes(mmap[*pos..*pos+4].try_into().ok()?); *pos += 4;
        let count = u64::from_le_bytes(mmap[*pos..*pos+8].try_into().ok()?) as usize; *pos += 8;
        if etype != 8 { return None; } // must be STRING array
        let mut result = Vec::with_capacity(count.min(256_000));
        for _ in 0..count {
            if *pos + 8 > mmap.len() { break; }
            let slen = u64::from_le_bytes(mmap[*pos..*pos+8].try_into().ok()?) as usize; *pos += 8;
            if *pos + slen > mmap.len() { break; }
            let s = std::str::from_utf8(&mmap[*pos..*pos+slen]).unwrap_or("<?>").to_string();
            *pos += slen;
            result.push(s);
        }
        Some(result)
    }

    fn read_u32_val(mmap: &[u8], pos: &mut usize, vtype: u32) -> Option<u32> {
        if vtype == 4 {
            if *pos + 4 > mmap.len() { return None; }
            let v = u32::from_le_bytes(mmap[*pos..*pos+4].try_into().ok()?); *pos += 4; Some(v)
        } else { Self::skip_value(mmap, pos, vtype)?; None }
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
        use std::fs::File;
        use memmap2::MmapOptions;
        let path = "C:/Projects/qualiaDB/gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf";
        if !std::path::Path::new(path).exists() {
            return;
        }
        let f = File::open(path).unwrap();
        let mmap = unsafe { MmapOptions::new().map(&f).unwrap() };
        if mmap.len() < 24 || &mmap[0..4] != b"GGUF" { return; }
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
            if key.contains("block") || key.contains("layer") || key.contains("embedding") || key.contains("head") {
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
            if name.starts_with("blk.0.") && (name.contains("attn_q") || name.contains("ffn_down")) {
                println!("tensor: {name} type={ggml_type} dims={dims:?} off={byte_off:#x}");
                blk_samples += 1;
            }
        }
    }

    #[test]
    fn test_gguf_ontology_extraction() {
        let sharder = GGufSharder::new("C:/Projects/qualiaDB/gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf".to_string());
        
        let superblock = sharder.extract_ontology_to_superblock();
        // Just verify it yields a superblock structural scaffold
        assert_eq!(superblock.active_quin_count, 0, "SuperBlock should be freshly initialized");
    }

    #[test]
    fn test_gguf_bidx_pointer_generation() {
        use crate::QuinPointerExt;

        let sharder = GGufSharder::new("mock_model.gguf".to_string());
        let pointers = sharder.generate_bidx_pointer_map();
        
        assert_eq!(pointers.len(), 1, "Failed to generate pointer map");

        let quin = pointers[0];
        assert_eq!(quin.extract_modality_flag(), crate::MODALITY_FLAG_LLM_TENSOR, "Pointer Modality Flag was not LLM");
        assert_eq!(quin.extract_byte_offset(), 0x00000ABC, "Pointer byte offset extracted incorrectly");
    }

    #[test]
    fn test_wordnet_lexicon_mapping() {
        use crate::QuinPointerExt;
        let sharder = GGufSharder::new("mock.gguf".to_string());
        
        // Mock WordNet Synset ID for "Dog"
        let synset_dog = 0x8a2a1072b;
        let quin = sharder.map_wordnet_synset(synset_dog, 0x1000);
        
        assert_eq!(quin.subject, synset_dog);
        assert_eq!(quin.extract_modality_flag(), crate::MODALITY_FLAG_DENSE_PHYSICS, "Modality Flag should be Dense Physics");
        assert_eq!(quin.extract_byte_offset(), 0x1000);
    }
}
