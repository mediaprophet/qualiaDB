//! Phase 4: AOT GGUF → `.q42` **LLM-weight container** compiler.
//!
//! A *sibling* of the semantic `.q42` graph format (`tensor/q42_integration.rs`,
//! `bake_pipeline.rs`) — it carries an **independent section magic** `b"Q42W"` so the two
//! never collide. Per the §6-Q2 architectural decision the weights are stored as **opaque,
//! page-aligned, contiguous quantized blobs strided for WGPU bind groups**; the 48-byte
//! `NQuin` is the *epistemic/topological scaffold* (manifest binding), never the weights.
//!
//! Output is **little-endian** on every host (Decision 4) via explicit serialization, with a
//! `version` gate; a future big-endian target would byte-swap here at ingest, leaving the
//! real-time inference path endian-blind.
//!
//! Layout:
//! ```text
//! [ Q42WeightHeader (96B) ]              magic, version, page_log2, n_tensors, n_layers,
//!                                        manifest_offset, blob_offset, cold_offset, cold_len,
//!                                        arch scaffold NQuin
//! [ Q42TensorEntry[] (80B each) ]        role, layer, ggml_type, dim0, dim1, blob_offset,
//!                                        byte_len, scaffold NQuin   (zero-parse hot manifest)
//! [ pad → 1<<page_log2 ]
//! [ tensor blob region ]                 quantized bytes; each tensor START page-aligned
//!                                        (default 16KB) for single-fetch mmap.
//! ```
//! The optional CBOR-LD ontological "cold" section (`cold_offset`/`cold_len`) is reserved for a
//! later pass (parsed once at ingest); v1 emits `cold_len = 0`.

use crate::gguf_sharder::{GgufTensorIndex, GgufTensorInfo};
use crate::NQuin;
use bytemuck::Zeroable;

pub const Q42W_MAGIC: [u8; 4] = *b"Q42W";
/// v2 added the hyperparameter block; **v3** adds the tokenizer section (`tokenizer_offset`/`_len`)
/// so a `.q42` is a fully self-contained execution container — weights, hyperparams, AND tokenizer,
/// no GGUF sidecar required.
pub const Q42W_VERSION: u16 = 3;
/// 14 = 16 KB pages (default; minimizes page faults on large FFN blocks). 12 = 4 KB.
pub const Q42W_DEFAULT_PAGE_LOG2: u16 = 14;
pub const Q42_WEIGHT_HEADER_BYTES: usize = 144;
pub const Q42_TENSOR_ENTRY_BYTES: usize = 80;

// Tensor roles. 0..=6 mirror the runtime `Mc8WeightRole` GEMM roles; 7..=11 add the
// remaining tensors so a `.q42` is a *complete* execution container (no GGUF re-parse).
pub const Q42_ROLE_ATTN_K: u16 = 0;
pub const Q42_ROLE_ATTN_V: u16 = 1;
pub const Q42_ROLE_ATTN_Q: u16 = 2;
pub const Q42_ROLE_ATTN_OUTPUT: u16 = 3;
pub const Q42_ROLE_FFN_GATE: u16 = 4;
pub const Q42_ROLE_FFN_UP: u16 = 5;
pub const Q42_ROLE_FFN_DOWN: u16 = 6;
pub const Q42_ROLE_ATTN_NORM: u16 = 7;
pub const Q42_ROLE_FFN_NORM: u16 = 8;
pub const Q42_ROLE_TOKEN_EMBD: u16 = 9;
pub const Q42_ROLE_OUTPUT: u16 = 10;
pub const Q42_ROLE_OUTPUT_NORM: u16 = 11;
/// `layer` sentinel for non-layer (global) tensors: token_embd / output / output_norm.
pub const Q42_LAYER_GLOBAL: u16 = 0xFFFF;

// ── `NQuin.metadata` bitfield (per-tensor execution / governance hints; shader-consumed later) ──
/// Tensor block is mostly zeros → engine may dispatch a sparse WGSL path.
pub const Q42_META_SPARSE_HINT: u64 = 1 << 0;
/// Tensor needs dynamic upcast before GEMM.
pub const Q42_META_REQUIRES_UPCAST: u64 = 1 << 1;
/// Deontic / ODRL / SHACL taint — a violating tensor can be driven to zero in-shader.
pub const Q42_META_DEONTIC_TAINT: u64 = 1 << 2;
// bits 3..=63 reserved.
//
// `NQuin.parity` (per entry) holds CRC-32C of the entry's 32 functional bytes in its low 32 bits
// (offset/len corruption → caught before any GPU bind, avoiding WebGPU OOB traps). Blob bit-rot is
// deferred (lazy/sampled) — hashing the multi-hundred-MB blob at index time would defeat zero-copy.

/// CRC-32C (Castagnoli, reflected) — table-less. Used for in-band `.q42` integrity (header+manifest
/// at boot, per-entry metadata at bind). Small inputs (≤ ~23 KB manifest) → microseconds.
#[inline]
fn crc32c_update(mut crc: u32, data: &[u8]) -> u32 {
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0x82F6_3B78 & 0u32.wrapping_sub(crc & 1));
        }
    }
    crc
}
#[inline]
fn crc32c(data: &[u8]) -> u32 {
    !crc32c_update(0xFFFF_FFFF, data)
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug)]
pub struct Q42WeightHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub page_log2: u16,
    pub n_tensors: u32,
    pub n_layers: u32,
    // ── hyperparameter block (v2): lets the runtime boot the KV cache without the GGUF ──
    pub n_embd: u32,
    pub n_head: u32,
    pub n_kv_head: u32,
    pub vocab_size: u32,
    pub rope_freq_base: f32,
    pub rope_scale: f32,
    // ── layout pointers ──
    pub manifest_offset: u64,
    pub blob_offset: u64,
    pub cold_offset: u64,
    pub cold_len: u64,
    /// CRC-32C over the header (excluding these 4 bytes) + the full manifest. Checked once at
    /// `from_q42` — rejects corrupted pointers before any GPU bind. (offset 72)
    pub header_crc: u32,
    /// Container-level flags (reserved: e.g. cold-section present, big-model). (offset 76)
    pub format_flags: u32,
    // `header_crc`/`format_flags` (8B) keep the 16-aligned NQuin at offset 80, zero padding.
    pub arch_quin: NQuin,
    /// v3: byte offset + length of the contiguous tokenizer section (0 if absent). (offset 128/136)
    pub tokenizer_offset: u64,
    pub tokenizer_len: u64,
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug)]
pub struct Q42TensorEntry {
    pub role: u16,
    pub layer: u16,
    pub ggml_type: u32,
    pub dim0: u32,
    pub dim1: u32,
    pub blob_offset: u64,
    pub byte_len: u64,
    pub scaffold_quin: NQuin,
}

// Layouts are padding-free and multiples of 16; assert so the manual LE (de)serializers and
// any future `mmap` cast stay in lock-step with the struct definitions.
const _: () = assert!(core::mem::size_of::<Q42WeightHeader>() == Q42_WEIGHT_HEADER_BYTES);
const _: () = assert!(core::mem::size_of::<Q42TensorEntry>() == Q42_TENSOR_ENTRY_BYTES);

#[inline]
fn align_up(x: usize, a: usize) -> usize {
    debug_assert!(a.is_power_of_two());
    (x + a - 1) & !(a - 1)
}

fn write_nquin_le(q: &NQuin, b: &mut [u8]) {
    b[0..8].copy_from_slice(&q.subject.to_le_bytes());
    b[8..16].copy_from_slice(&q.predicate.to_le_bytes());
    b[16..24].copy_from_slice(&q.object.to_le_bytes());
    b[24..32].copy_from_slice(&q.context.to_le_bytes());
    b[32..40].copy_from_slice(&q.metadata.to_le_bytes());
    b[40..48].copy_from_slice(&q.parity.to_le_bytes());
}

fn read_nquin_le(b: &[u8]) -> NQuin {
    let u = |o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap());
    NQuin {
        subject: u(0),
        predicate: u(8),
        object: u(16),
        context: u(24),
        metadata: u(32),
        parity: u(40),
    }
}

impl Q42WeightHeader {
    fn write_le(&self, b: &mut [u8]) {
        b[0..4].copy_from_slice(&self.magic);
        b[4..6].copy_from_slice(&self.version.to_le_bytes());
        b[6..8].copy_from_slice(&self.page_log2.to_le_bytes());
        b[8..12].copy_from_slice(&self.n_tensors.to_le_bytes());
        b[12..16].copy_from_slice(&self.n_layers.to_le_bytes());
        b[16..20].copy_from_slice(&self.n_embd.to_le_bytes());
        b[20..24].copy_from_slice(&self.n_head.to_le_bytes());
        b[24..28].copy_from_slice(&self.n_kv_head.to_le_bytes());
        b[28..32].copy_from_slice(&self.vocab_size.to_le_bytes());
        b[32..36].copy_from_slice(&self.rope_freq_base.to_le_bytes());
        b[36..40].copy_from_slice(&self.rope_scale.to_le_bytes());
        b[40..48].copy_from_slice(&self.manifest_offset.to_le_bytes());
        b[48..56].copy_from_slice(&self.blob_offset.to_le_bytes());
        b[56..64].copy_from_slice(&self.cold_offset.to_le_bytes());
        b[64..72].copy_from_slice(&self.cold_len.to_le_bytes());
        b[72..76].copy_from_slice(&self.header_crc.to_le_bytes());
        b[76..80].copy_from_slice(&self.format_flags.to_le_bytes());
        write_nquin_le(&self.arch_quin, &mut b[80..128]);
        b[128..136].copy_from_slice(&self.tokenizer_offset.to_le_bytes());
        b[136..144].copy_from_slice(&self.tokenizer_len.to_le_bytes());
    }

    fn read_le(b: &[u8]) -> Self {
        let u16a = |o: usize| u16::from_le_bytes(b[o..o + 2].try_into().unwrap());
        let u32a = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let u64a = |o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap());
        let f32a = |o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&b[0..4]);
        Q42WeightHeader {
            magic,
            version: u16a(4),
            page_log2: u16a(6),
            n_tensors: u32a(8),
            n_layers: u32a(12),
            n_embd: u32a(16),
            n_head: u32a(20),
            n_kv_head: u32a(24),
            vocab_size: u32a(28),
            rope_freq_base: f32a(32),
            rope_scale: f32a(36),
            manifest_offset: u64a(40),
            blob_offset: u64a(48),
            cold_offset: u64a(56),
            cold_len: u64a(64),
            header_crc: u32a(72),
            format_flags: u32a(76),
            arch_quin: read_nquin_le(&b[80..128]),
            tokenizer_offset: u64a(128),
            tokenizer_len: u64a(136),
        }
    }
}

impl Q42TensorEntry {
    fn write_le(&self, b: &mut [u8]) {
        b[0..2].copy_from_slice(&self.role.to_le_bytes());
        b[2..4].copy_from_slice(&self.layer.to_le_bytes());
        b[4..8].copy_from_slice(&self.ggml_type.to_le_bytes());
        b[8..12].copy_from_slice(&self.dim0.to_le_bytes());
        b[12..16].copy_from_slice(&self.dim1.to_le_bytes());
        b[16..24].copy_from_slice(&self.blob_offset.to_le_bytes());
        b[24..32].copy_from_slice(&self.byte_len.to_le_bytes());
        write_nquin_le(&self.scaffold_quin, &mut b[32..80]);
    }

    fn read_le(b: &[u8]) -> Self {
        let u16a = |o: usize| u16::from_le_bytes(b[o..o + 2].try_into().unwrap());
        let u32a = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let u64a = |o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap());
        Q42TensorEntry {
            role: u16a(0),
            layer: u16a(2),
            ggml_type: u32a(4),
            dim0: u32a(8),
            dim1: u32a(12),
            blob_offset: u64a(16),
            byte_len: u64a(24),
            scaffold_quin: read_nquin_le(&b[32..80]),
        }
    }
}

/// Compile a flat GGUF byte image into a `.q42` LLM-weight container.
/// `page_log2 == 0` selects the default (16 KB). Returns the little-endian container bytes.
pub fn compile_gguf_to_q42(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    let idx = GgufTensorIndex::from_gguf(input);
    if idx.tensor_data_start == 0 && idx.hyperparams.n_layer == 0 {
        return Err("GGUF parse failed or yielded no tensor metadata".to_string());
    }
    let page_log2 = if page_log2 == 0 { Q42W_DEFAULT_PAGE_LOG2 } else { page_log2 };
    if page_log2 < 8 || page_log2 > 30 {
        return Err(format!("page_log2 {page_log2} out of range"));
    }
    let page = 1usize << page_log2;
    let n_layer = idx.hyperparams.n_layer;
    let tds = idx.tensor_data_start as usize;

    // 1. Enumerate all engine tensors (per-layer roles + globals) with role/layer tags.
    let mut planned: Vec<(u16, u16, GgufTensorInfo)> = Vec::new();
    let mut push = |role: u16, layer: u16, t: Option<GgufTensorInfo>| {
        if let Some(info) = t {
            planned.push((role, layer, info));
        }
    };
    for layer in 0..n_layer {
        let t = idx.get_layer_tensors(layer);
        let l = layer as u16;
        push(Q42_ROLE_ATTN_NORM, l, t.attn_norm);
        push(Q42_ROLE_ATTN_Q, l, t.attn_q);
        push(Q42_ROLE_ATTN_K, l, t.attn_k);
        push(Q42_ROLE_ATTN_V, l, t.attn_v);
        push(Q42_ROLE_ATTN_OUTPUT, l, t.attn_output);
        push(Q42_ROLE_FFN_NORM, l, t.ffn_norm);
        push(Q42_ROLE_FFN_GATE, l, t.ffn_gate);
        push(Q42_ROLE_FFN_UP, l, t.ffn_up);
        push(Q42_ROLE_FFN_DOWN, l, t.ffn_down);
    }
    push(Q42_ROLE_TOKEN_EMBD, Q42_LAYER_GLOBAL, idx.token_embd_info().copied());
    push(Q42_ROLE_OUTPUT, Q42_LAYER_GLOBAL, idx.output_weight_info().copied());
    push(Q42_ROLE_OUTPUT_NORM, Q42_LAYER_GLOBAL, idx.output_norm_info().copied());

    if planned.is_empty() {
        return Err("no tensors enumerated from GGUF".to_string());
    }

    // 2. Layout: header → manifest → page-aligned blob region.
    let manifest_offset = align_up(Q42_WEIGHT_HEADER_BYTES, 16);
    let manifest_size = Q42_TENSOR_ENTRY_BYTES * planned.len();
    let blob_offset = align_up(manifest_offset + manifest_size, page);

    // 3. Assign each tensor a page-aligned blob offset (emission order).
    let mut cur = blob_offset;
    let mut entries: Vec<Q42TensorEntry> = Vec::with_capacity(planned.len());
    for (role, layer, info) in &planned {
        let byte_len = crate::ggml_quants::tensor_byte_len(info)
            .ok_or_else(|| format!("unsupported tensor type {} (role {role})", info.ggml_type))?;
        let off = align_up(cur, page);
        let src = tds + info.byte_offset as usize;
        if src + byte_len > input.len() {
            return Err(format!("tensor (role {role}, layer {layer}) out of GGUF bounds"));
        }
        entries.push(Q42TensorEntry {
            role: *role,
            layer: *layer,
            ggml_type: info.ggml_type,
            dim0: info.dims[0] as u32,
            dim1: info.dims[1] as u32,
            blob_offset: off as u64,
            byte_len: byte_len as u64,
            scaffold_quin: NQuin::zeroed(),
        });
        cur = off + byte_len;
    }
    let blob_region_end = cur;

    // Tokenizer section (v3): packed contiguous block (no page alignment) appended after the blobs,
    // so a `.q42` can tokenize prompts with no GGUF sidecar.
    let tok_section = crate::gguf_sharder::GgufTokenizer::from_gguf(input).to_q42_section();
    let tokenizer_offset = align_up(blob_region_end, 16);
    let total = align_up(tokenizer_offset + tok_section.len(), 16);

    // 4. Emit: header + manifest + page-aligned blobs + tokenizer section.
    let hp = &idx.hyperparams;
    let header = Q42WeightHeader {
        magic: Q42W_MAGIC,
        version: Q42W_VERSION,
        page_log2,
        n_tensors: planned.len() as u32,
        n_layers: n_layer,
        n_embd: hp.n_embd,
        n_head: hp.n_head,
        n_kv_head: hp.effective_n_kv_head(),
        vocab_size: idx.vocab_dim() as u32,
        rope_freq_base: hp.effective_rope_freq_base(),
        rope_scale: hp.effective_rope_scale(),
        manifest_offset: manifest_offset as u64,
        blob_offset: blob_offset as u64,
        cold_offset: 0,
        cold_len: 0,
        header_crc: 0,
        format_flags: 0,
        arch_quin: NQuin::zeroed(),
        tokenizer_offset: tokenizer_offset as u64,
        tokenizer_len: tok_section.len() as u64,
    };
    let mut out = vec![0u8; total];
    header.write_le(&mut out[0..Q42_WEIGHT_HEADER_BYTES]);
    for (k, e) in entries.iter().enumerate() {
        let o = manifest_offset + k * Q42_TENSOR_ENTRY_BYTES;
        e.write_le(&mut out[o..o + Q42_TENSOR_ENTRY_BYTES]);
        // Per-entry integrity: CRC-32C over the 32 functional bytes → NQuin.parity (entry +72..80).
        let entry_crc = crc32c(&out[o..o + 32]) as u64;
        out[o + 72..o + 80].copy_from_slice(&entry_crc.to_le_bytes());
    }
    // Header integrity: CRC-32C over header (skipping its own 4 crc bytes at 72..76) + manifest.
    let manifest_end = manifest_offset + Q42_TENSOR_ENTRY_BYTES * entries.len();
    let hc = crc32c_update(crc32c_update(0xFFFF_FFFF, &out[0..72]), &out[76..manifest_end]);
    out[72..76].copy_from_slice(&(!hc).to_le_bytes());
    for (k, (_, _, info)) in planned.iter().enumerate() {
        let src = tds + info.byte_offset as usize;
        let len = entries[k].byte_len as usize;
        let dst = entries[k].blob_offset as usize;
        out[dst..dst + len].copy_from_slice(&input[src..src + len]);
    }
    out[tokenizer_offset..tokenizer_offset + tok_section.len()].copy_from_slice(&tok_section);
    Ok(out)
}

/// Task #12 / STELLAR §A — like [`compile_gguf_to_q42`] but **ternary-packs the FFN projections**
/// (gate/up/down) during the compile, producing a **complete, runnable** Q42W: hyperparameters +
/// tokenizer are preserved (so the live loader boots it and builds the KV cache), while the FFN
/// tensors are BitNet-1.58b ternary blobs (`ternary::dequantize_blob` / the 2-bit GPU kernel).
/// Attention / norms / embeddings stay verbatim at their source precision. This is the loadable
/// container the live FFN-ternary dispatch path will run + measure against.
pub fn compile_gguf_to_q42_ternary_ffn(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    compile_gguf_to_q42_ffn_quant_awq(input, page_log2, None, 0.0, FfnQuant::Ternary)
}

/// Target quantization for the FFN tensors in an AWQ `.q42` compile.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FfnQuant {
    /// BitNet 1.58b ternary (`GGML_TYPE_TERNARY_158`, resident 2-bit GPU path).
    Ternary,
    /// ggml Q4_0 4-bit (`GGML_TYPE_Q4_0`, the standard quantized GPU GEMM path) — AWQ's design regime.
    Q4_0,
}

/// AWQ-aware ternary FFN compile (back-compat wrapper for [`compile_gguf_to_q42_ffn_quant_awq`]).
pub fn compile_gguf_to_q42_ternary_ffn_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_q42_ffn_quant_awq(input, page_log2, awq_scales, alpha, FfnQuant::Ternary)
}

/// AWQ-aware **Q4_0** FFN compile (Path A) — FFN packed to 4-bit Q4_0 (AWQ's design regime); all else
/// verbatim from the source GGUF.
pub fn compile_gguf_to_q42_q4_ffn_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_q42_ffn_quant_awq(input, page_log2, awq_scales, alpha, FfnQuant::Q4_0)
}

/// AWQ-aware FFN-quantized `.q42` compile. `quant` selects the FFN target (ternary or Q4_0). When
/// `awq_scales` is `Some` (per-layer per-input-channel salience from [`crate::llm_awq::snapshot`]) the
/// gate/up input channel `i` is scaled by `s_i^alpha` before packing and `ffn_norm` is divided by
/// `s_i^alpha` — mathematically exact in f32 (`(X·norm/s^a)·(W·s^a)=(X·norm)·W`) — moving salient
/// channels into a range the quant grid represents better. `awq_scales = None` / `alpha == 0.0`
/// reproduces the plain (un-calibrated) compile. The down projection is left un-scaled (no clean fold
/// site — a v2 item). Everything outside the FFN passes through verbatim from the source GGUF.
pub fn compile_gguf_to_q42_ffn_quant_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
    quant: FfnQuant,
) -> Result<Vec<u8>, String> {
    use crate::ggml_quants::GGML_TYPE_Q4_0;
    use crate::llm_kernel_parity::{q4_0_bytes, quantize_q4_0_from_f32};
    use crate::ternary::{ternary_blob, ternary_blob_len, GGML_TYPE_TERNARY_158};

    let idx = GgufTensorIndex::from_gguf(input);
    if idx.tensor_data_start == 0 && idx.hyperparams.n_layer == 0 {
        return Err("GGUF parse failed or yielded no tensor metadata".to_string());
    }
    let page_log2 = if page_log2 == 0 { Q42W_DEFAULT_PAGE_LOG2 } else { page_log2 };
    if !(8..=30).contains(&page_log2) {
        return Err(format!("page_log2 {page_log2} out of range"));
    }
    let page = 1usize << page_log2;
    let n_layer = idx.hyperparams.n_layer;
    let tds = idx.tensor_data_start as usize;

    // 1. Enumerate all engine tensors (same set as compile_gguf_to_q42).
    let mut planned: Vec<(u16, u16, GgufTensorInfo)> = Vec::new();
    let mut push = |role: u16, layer: u16, t: Option<GgufTensorInfo>| {
        if let Some(info) = t {
            planned.push((role, layer, info));
        }
    };
    for layer in 0..n_layer {
        let t = idx.get_layer_tensors(layer);
        let l = layer as u16;
        push(Q42_ROLE_ATTN_NORM, l, t.attn_norm);
        push(Q42_ROLE_ATTN_Q, l, t.attn_q);
        push(Q42_ROLE_ATTN_K, l, t.attn_k);
        push(Q42_ROLE_ATTN_V, l, t.attn_v);
        push(Q42_ROLE_ATTN_OUTPUT, l, t.attn_output);
        push(Q42_ROLE_FFN_NORM, l, t.ffn_norm);
        push(Q42_ROLE_FFN_GATE, l, t.ffn_gate);
        push(Q42_ROLE_FFN_UP, l, t.ffn_up);
        push(Q42_ROLE_FFN_DOWN, l, t.ffn_down);
    }
    push(Q42_ROLE_TOKEN_EMBD, Q42_LAYER_GLOBAL, idx.token_embd_info().copied());
    push(Q42_ROLE_OUTPUT, Q42_LAYER_GLOBAL, idx.output_weight_info().copied());
    push(Q42_ROLE_OUTPUT_NORM, Q42_LAYER_GLOBAL, idx.output_norm_info().copied());
    if planned.is_empty() {
        return Err("no tensors enumerated from GGUF".to_string());
    }

    let is_ffn = |role: u16| matches!(role, Q42_ROLE_FFN_GATE | Q42_ROLE_FFN_UP | Q42_ROLE_FFN_DOWN);
    let n_elems_of = |info: &GgufTensorInfo| -> usize {
        let nd = info.n_dims.max(1) as usize;
        info.dims.iter().take(nd).map(|&d| d.max(1) as usize).product()
    };

    // 2. Layout: FFN → ternary blob (small); everything else → verbatim source bytes.
    let manifest_offset = align_up(Q42_WEIGHT_HEADER_BYTES, 16);
    let manifest_size = Q42_TENSOR_ENTRY_BYTES * planned.len();
    let blob_offset = align_up(manifest_offset + manifest_size, page);
    let mut cur = blob_offset;
    let mut entries: Vec<Q42TensorEntry> = Vec::with_capacity(planned.len());
    for (role, layer, info) in &planned {
        let src_len = crate::ggml_quants::tensor_byte_len(info)
            .ok_or_else(|| format!("unsupported tensor type {} (role {role})", info.ggml_type))?;
        if tds + info.byte_offset as usize + src_len > input.len() {
            return Err(format!("tensor (role {role}, layer {layer}) out of GGUF bounds"));
        }
        let (out_ggml, byte_len) = if is_ffn(*role) {
            match quant {
                FfnQuant::Ternary => (GGML_TYPE_TERNARY_158, ternary_blob_len(n_elems_of(info))),
                FfnQuant::Q4_0 => (GGML_TYPE_Q4_0, q4_0_bytes(n_elems_of(info))),
            }
        } else {
            (info.ggml_type, src_len)
        };
        let off = align_up(cur, page);
        entries.push(Q42TensorEntry {
            role: *role,
            layer: *layer,
            ggml_type: out_ggml,
            dim0: info.dims[0] as u32,
            dim1: info.dims[1] as u32,
            blob_offset: off as u64,
            byte_len: byte_len as u64,
            scaffold_quin: NQuin::zeroed(),
        });
        cur = off + byte_len;
    }
    let blob_region_end = cur;

    let tok_section = crate::gguf_sharder::GgufTokenizer::from_gguf(input).to_q42_section();
    let tokenizer_offset = align_up(blob_region_end, 16);
    let total = align_up(tokenizer_offset + tok_section.len(), 16);

    // 3. Header (complete: hyperparams + tokenizer carried through; flagged ternary).
    let hp = &idx.hyperparams;
    let header = Q42WeightHeader {
        magic: Q42W_MAGIC,
        version: Q42W_VERSION,
        page_log2,
        n_tensors: planned.len() as u32,
        n_layers: n_layer,
        n_embd: hp.n_embd,
        n_head: hp.n_head,
        n_kv_head: hp.effective_n_kv_head(),
        vocab_size: idx.vocab_dim() as u32,
        rope_freq_base: hp.effective_rope_freq_base(),
        rope_scale: hp.effective_rope_scale(),
        manifest_offset: manifest_offset as u64,
        blob_offset: blob_offset as u64,
        cold_offset: 0,
        cold_len: 0,
        header_crc: 0,
        format_flags: match quant {
            FfnQuant::Ternary => FORMAT_FLAG_TERNARY,
            FfnQuant::Q4_0 => 0,
        },
        arch_quin: NQuin::zeroed(),
        tokenizer_offset: tokenizer_offset as u64,
        tokenizer_len: tok_section.len() as u64,
    };
    let mut out = vec![0u8; total];
    header.write_le(&mut out[0..Q42_WEIGHT_HEADER_BYTES]);
    for (k, e) in entries.iter().enumerate() {
        let o = manifest_offset + k * Q42_TENSOR_ENTRY_BYTES;
        e.write_le(&mut out[o..o + Q42_TENSOR_ENTRY_BYTES]);
        let entry_crc = crc32c(&out[o..o + 32]) as u64;
        out[o + 72..o + 80].copy_from_slice(&entry_crc.to_le_bytes());
    }
    let manifest_end = manifest_offset + Q42_TENSOR_ENTRY_BYTES * entries.len();
    let hc = crc32c_update(crc32c_update(0xFFFF_FFFF, &out[0..72]), &out[76..manifest_end]);
    out[72..76].copy_from_slice(&(!hc).to_le_bytes());

    // 4. Blobs: FFN → dequant source → ternary-pack; everything else → verbatim copy.
    // AWQ per-input-channel scale `s_i^alpha` (dead channels floored to avoid div-by-zero in the fold).
    let awq_on = awq_scales.is_some() && alpha != 0.0;
    let awq_scale = |layer: u16, chan: usize| -> f32 {
        match awq_scales {
            Some(per_layer) => {
                let l = layer as usize;
                if l < per_layer.len() && chan < per_layer[l].len() {
                    per_layer[l][chan].max(1e-6).powf(alpha)
                } else {
                    1.0
                }
            }
            None => 1.0,
        }
    };

    let mut scratch: Vec<f32> = Vec::new();
    for (k, (role, layer, info)) in planned.iter().enumerate() {
        let src = tds + info.byte_offset as usize;
        let src_len = crate::ggml_quants::tensor_byte_len(info).unwrap();
        let dst = entries[k].blob_offset as usize;
        let blen = entries[k].byte_len as usize;
        if is_ffn(*role) {
            let n = n_elems_of(info);
            if scratch.len() < n {
                scratch.resize(n, 0.0);
            }
            crate::ggml_quants::dequantize_row_into(&input[src..src + src_len], info.ggml_type, n, &mut scratch)
                .map_err(|e| format!("dequant role {role}: {e:?}"))?;
            // AWQ: scale gate/up input channel i by s_i^alpha (row-major [n_out][n_in]). The down
            // projection's input is the FFN hidden (no captured stats) → left plain in v1.
            if awq_on && matches!(*role, Q42_ROLE_FFN_GATE | Q42_ROLE_FFN_UP) {
                let n_in = info.dims[0] as usize;
                let n_out = info.dims[1] as usize;
                if n_in.saturating_mul(n_out) == n && n_in > 0 {
                    for o in 0..n_out {
                        let row = o * n_in;
                        for i in 0..n_in {
                            scratch[row + i] *= awq_scale(*layer, i);
                        }
                    }
                }
            }
            match quant {
                FfnQuant::Ternary => {
                    let blob = ternary_blob(&scratch[..n]);
                    out[dst..dst + blob.len()].copy_from_slice(&blob);
                }
                FfnQuant::Q4_0 => {
                    if !quantize_q4_0_from_f32(&scratch[..n], &mut out[dst..dst + blen]) {
                        return Err(format!("Q4_0 quantize failed (role {role}, layer {layer})"));
                    }
                }
            }
        } else if awq_on && *role == Q42_ROLE_FFN_NORM {
            // Fold 1/s_i^alpha into ffn_norm so the scaled gate/up stays exact. Norm is a per-channel
            // f32 (or f16) vector of the same length/type — read, divide, write in place.
            let n = n_elems_of(info);
            if info.ggml_type == crate::ggml_quants::GGML_TYPE_F32 && blen >= n * 4 {
                for i in 0..n {
                    let b = src + i * 4;
                    let w = f32::from_le_bytes([input[b], input[b + 1], input[b + 2], input[b + 3]]);
                    let folded = w / awq_scale(*layer, i);
                    out[dst + i * 4..dst + i * 4 + 4].copy_from_slice(&folded.to_le_bytes());
                }
            } else if info.ggml_type == crate::ggml_quants::GGML_TYPE_F16 && blen >= n * 2 {
                for i in 0..n {
                    let b = src + i * 2;
                    let w = half::f16::from_le_bytes([input[b], input[b + 1]]).to_f32();
                    let folded = half::f16::from_f32(w / awq_scale(*layer, i)).to_le_bytes();
                    out[dst + i * 2..dst + i * 2 + 2].copy_from_slice(&folded);
                }
            } else {
                return Err(format!(
                    "AWQ: ffn_norm (layer {layer}) has un-foldable type {} — cannot keep the fold exact",
                    info.ggml_type
                ));
            }
        } else {
            out[dst..dst + blen].copy_from_slice(&input[src..src + blen]);
        }
    }
    out[tokenizer_offset..tokenizer_offset + tok_section.len()].copy_from_slice(&tok_section);
    Ok(out)
}

/// `format_flags` bit: container produced by the **raw streaming transcode** (safetensor/MLX →
/// Q42W) — tensors are verbatim high-fidelity blobs not yet mapped to engine GEMM roles, and the
/// GGUF hyperparameter block is absent. (Distinguishes it from a `compile_gguf_to_q42` container.)
pub const FORMAT_FLAG_RAW_TRANSCODE: u32 = 1 << 0;
/// `format_flags` bit: tensors were **ternary-quantized (BitNet 1.58b)** during transcode — each
/// blob is `[scale: f32][packed trits]` (`ggml_type = ternary::GGML_TYPE_TERNARY_158`); decode via
/// `ternary::dequantize_blob`.
pub const FORMAT_FLAG_TERNARY: u32 = 1 << 1;

/// Decode a high-fidelity source tensor's bytes (`F32`/`F16`/`BF16`) to `f32` into `out` (cleared
/// and refilled). Cold-path (ingest) helper for the ternary transcode.
fn decode_safetensor_to_f32(raw: &[u8], ggml: u32, count: usize, out: &mut Vec<f32>) {
    use crate::safetensor::{GGML_BF16, GGML_F16, GGML_F32};
    out.clear();
    out.reserve(count);
    match ggml {
        GGML_F32 => {
            for k in 0..count {
                let o = k * 4;
                if o + 4 > raw.len() { break; }
                out.push(f32::from_le_bytes([raw[o], raw[o + 1], raw[o + 2], raw[o + 3]]));
            }
        }
        GGML_F16 => {
            for k in 0..count {
                let o = k * 2;
                if o + 2 > raw.len() { break; }
                out.push(half::f16::from_le_bytes([raw[o], raw[o + 1]]).to_f32());
            }
        }
        GGML_BF16 => {
            for k in 0..count {
                let o = k * 2;
                if o + 2 > raw.len() { break; }
                out.push(half::bf16::from_le_bytes([raw[o], raw[o + 1]]).to_f32());
            }
        }
        _ => {}
    }
}

/// Outcome of a streaming transcode — the numbers that make the memory claim falsifiable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranscodeReport {
    pub n_tensors: usize,
    pub bytes_written: usize,
    pub largest_tensor_bytes: usize,
    pub total_tensor_bytes: usize,
    /// High-water mark of the transcoder's working buffer — one tensor in flight, so ≈ the largest
    /// single tensor, **never** the whole file.
    pub peak_working_bytes: usize,
}

/// Phase 6 / task #12 — **streaming, versioned transcode: safetensor (high-fidelity) → Q42W**.
///
/// Writes a valid Q42W container to `out` forward-only (round-trips through [`Q42TensorIndex::from_q42`]).
/// The full layout is computed from the safetensor *header* alone (no tensor reads), so each tensor's
/// bytes pass through **one reused scratch buffer** — the transcoder's peak working memory is ≈ the
/// largest single tensor, not the whole file. On the real path `src` is an `mmap` (demand-paged by
/// the OS), so the file is never resident in full.
///
/// Rejects low-precision (`Q4`-class) inputs — high-fidelity (`F32/F16/BF16/Q8`) only. The legacy
/// `compile_gguf_to_q42` path is untouched (GGUF support unchanged).
pub fn transcode_safetensor_to_q42<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    use crate::safetensor::{is_high_fidelity_ggml, parse_safetensor_header, safetensor_dtype_to_ggml};

    let page_log2 = if page_log2 == 0 { Q42W_DEFAULT_PAGE_LOG2 } else { page_log2 };
    if page_log2 < 8 || page_log2 > 30 {
        return Err(format!("page_log2 {page_log2} out of range"));
    }
    let page = 1usize << page_log2;

    let plan = parse_safetensor_header(src)?;
    if plan.tensors.is_empty() {
        return Err("safetensor: no tensors".to_string());
    }

    // 1) dtype gate (high-fidelity only — reject Q4 / low precision) + GGML mapping.
    let mut ggml_types = Vec::with_capacity(plan.tensors.len());
    for t in &plan.tensors {
        let g = safetensor_dtype_to_ggml(&t.dtype).ok_or_else(|| {
            format!("transcode: tensor '{}' dtype {} is not a high-fidelity source (rejected)", t.name, t.dtype)
        })?;
        if !is_high_fidelity_ggml(g) {
            return Err(format!("transcode: tensor '{}' is low-precision (Q4-class) — rejected", t.name));
        }
        ggml_types.push(g);
    }

    // 2) Layout (from the header alone — no tensor reads): header → manifest → page-aligned blobs.
    let n = plan.tensors.len();
    let manifest_offset = align_up(Q42_WEIGHT_HEADER_BYTES, 16);
    let manifest_size = Q42_TENSOR_ENTRY_BYTES * n;
    let blob_offset = align_up(manifest_offset + manifest_size, page);
    let mut entries: Vec<Q42TensorEntry> = Vec::with_capacity(n);
    let mut cur = blob_offset;
    let mut total_tensor_bytes = 0usize;
    let mut largest = 0usize;
    for (i, t) in plan.tensors.iter().enumerate() {
        let blen = t.byte_len();
        let off = align_up(cur, page);
        let mut e = Q42TensorEntry {
            role: crate::tensor_roles::name_to_role(&t.name).map(|r| r.role).unwrap_or(Q42_LAYER_GLOBAL),
            layer: crate::tensor_roles::name_to_role(&t.name).map(|r| r.layer).unwrap_or(Q42_LAYER_GLOBAL),
            ggml_type: ggml_types[i],
            dim0: *t.shape.first().unwrap_or(&1) as u32,
            dim1: *t.shape.get(1).unwrap_or(&1) as u32,
            blob_offset: off as u64,
            byte_len: blen as u64,
            scaffold_quin: NQuin::zeroed(),
        };
        e.scaffold_quin.subject = crate::q_hash(t.name.as_str()); // tensor identity (name hash)
        total_tensor_bytes += blen;
        largest = largest.max(blen);
        entries.push(e);
        cur = off + blen;
    }

    // 3) Header + manifest in a small bounded buffer (size ∝ tensor count, not file), CRC, then write.
    let manifest_end = manifest_offset + manifest_size;
    let mut head = vec![0u8; manifest_end];
    let header = Q42WeightHeader {
        magic: Q42W_MAGIC,
        version: Q42W_VERSION,
        page_log2,
        n_tensors: n as u32,
        n_layers: 0,
        n_embd: 0,
        n_head: 0,
        n_kv_head: 0,
        vocab_size: 0,
        rope_freq_base: 0.0,
        rope_scale: 0.0,
        manifest_offset: manifest_offset as u64,
        blob_offset: blob_offset as u64,
        cold_offset: 0,
        cold_len: 0,
        header_crc: 0,
        format_flags: FORMAT_FLAG_RAW_TRANSCODE,
        arch_quin: NQuin::zeroed(),
        tokenizer_offset: 0,
        tokenizer_len: 0,
    };
    header.write_le(&mut head[0..Q42_WEIGHT_HEADER_BYTES]);
    for (k, e) in entries.iter().enumerate() {
        let o = manifest_offset + k * Q42_TENSOR_ENTRY_BYTES;
        e.write_le(&mut head[o..o + Q42_TENSOR_ENTRY_BYTES]);
        let entry_crc = crc32c(&head[o..o + 32]) as u64;
        head[o + 72..o + 80].copy_from_slice(&entry_crc.to_le_bytes());
    }
    let hc = crc32c_update(crc32c_update(0xFFFF_FFFF, &head[0..72]), &head[76..manifest_end]);
    head[72..76].copy_from_slice(&(!hc).to_le_bytes());
    out.write_all(&head).map_err(|e| e.to_string())?;
    let mut bytes_written = head.len();

    // 4) Stream each blob through ONE reused scratch (peak ≈ largest tensor), page-aligned.
    let zeros = [0u8; 4096];
    let mut scratch: Vec<u8> = Vec::new();
    let mut peak_working = 0usize;
    for (i, t) in plan.tensors.iter().enumerate() {
        let target = entries[i].blob_offset as usize;
        let mut pad = target.saturating_sub(bytes_written);
        while pad > 0 {
            let chunk = pad.min(zeros.len());
            out.write_all(&zeros[..chunk]).map_err(|e| e.to_string())?;
            bytes_written += chunk;
            pad -= chunk;
        }
        let begin = plan.data_start + t.begin;
        let end = plan.data_start + t.end;
        if end > src.len() {
            return Err(format!("transcode: tensor '{}' out of source bounds", t.name));
        }
        scratch.clear();
        scratch.extend_from_slice(&src[begin..end]); // exactly one tensor in flight
        peak_working = peak_working.max(scratch.len());
        out.write_all(&scratch).map_err(|e| e.to_string())?;
        bytes_written += scratch.len();
    }

    Ok(TranscodeReport {
        n_tensors: n,
        bytes_written,
        largest_tensor_bytes: largest,
        total_tensor_bytes,
        peak_working_bytes: peak_working,
    })
}

/// Task #12 / STELLAR §A — **streaming transcode with BitNet 1.58b ternary compression**:
/// safetensor (high-fidelity) → Q42W, each tensor quantized to `{-1,0,+1}` with a per-tensor
/// absmean scale and packed at ≈ 1.6 bits/weight (`ternary` module) *during* transcode.
///
/// Same streaming discipline as [`transcode_safetensor_to_q42`] (layout from the header; one tensor
/// in flight). Each blob is `[scale: f32][packed trits]` with `ggml_type =
/// ternary::GGML_TYPE_TERNARY_158`; the container carries `FORMAT_FLAG_TERNARY`. Decode with
/// `ternary::dequantize_blob`. Round-trips through [`Q42TensorIndex::from_q42`].
pub fn transcode_safetensor_to_q42_ternary<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    use crate::safetensor::{is_high_fidelity_ggml, parse_safetensor_header, safetensor_dtype_to_ggml};
    use crate::ternary::{ternary_blob, ternary_blob_len, GGML_TYPE_TERNARY_158};

    let page_log2 = if page_log2 == 0 { Q42W_DEFAULT_PAGE_LOG2 } else { page_log2 };
    if !(8..=30).contains(&page_log2) {
        return Err(format!("page_log2 {page_log2} out of range"));
    }
    let page = 1usize << page_log2;

    let plan = parse_safetensor_header(src)?;
    if plan.tensors.is_empty() {
        return Err("safetensor: no tensors".to_string());
    }

    // 1) dtype gate + element counts (from the header alone).
    let n = plan.tensors.len();
    let mut ggml_types = Vec::with_capacity(n);
    let mut counts = Vec::with_capacity(n);
    for t in &plan.tensors {
        let g = safetensor_dtype_to_ggml(&t.dtype).ok_or_else(|| {
            format!("transcode: tensor '{}' dtype {} is not a high-fidelity source (rejected)", t.name, t.dtype)
        })?;
        if !is_high_fidelity_ggml(g) {
            return Err(format!("transcode: tensor '{}' is low-precision (Q4-class) — rejected", t.name));
        }
        let count: usize = if t.shape.is_empty() { 0 } else { t.shape.iter().product() };
        ggml_types.push(g);
        counts.push(count);
    }

    // 2) Layout: each output tensor is a compact ternary blob (size known from the count).
    let manifest_offset = align_up(Q42_WEIGHT_HEADER_BYTES, 16);
    let manifest_size = Q42_TENSOR_ENTRY_BYTES * n;
    let blob_offset = align_up(manifest_offset + manifest_size, page);
    let mut entries: Vec<Q42TensorEntry> = Vec::with_capacity(n);
    let mut cur = blob_offset;
    let mut total_out = 0usize;
    let mut largest_out = 0usize;
    for (i, t) in plan.tensors.iter().enumerate() {
        let blen = ternary_blob_len(counts[i]);
        let off = align_up(cur, page);
        let mut e = Q42TensorEntry {
            role: Q42_LAYER_GLOBAL,
            layer: Q42_LAYER_GLOBAL,
            ggml_type: GGML_TYPE_TERNARY_158,
            dim0: *t.shape.first().unwrap_or(&1) as u32,
            dim1: *t.shape.get(1).unwrap_or(&1) as u32,
            blob_offset: off as u64,
            byte_len: blen as u64,
            scaffold_quin: NQuin::zeroed(),
        };
        e.scaffold_quin.subject = crate::q_hash(t.name.as_str());
        total_out += blen;
        largest_out = largest_out.max(blen);
        entries.push(e);
        cur = off + blen;
    }

    // 3) Header + manifest in a small bounded buffer, CRC, then write.
    let manifest_end = manifest_offset + manifest_size;
    let mut head = vec![0u8; manifest_end];
    let header = Q42WeightHeader {
        magic: Q42W_MAGIC,
        version: Q42W_VERSION,
        page_log2,
        n_tensors: n as u32,
        n_layers: 0,
        n_embd: 0,
        n_head: 0,
        n_kv_head: 0,
        vocab_size: 0,
        rope_freq_base: 0.0,
        rope_scale: 0.0,
        manifest_offset: manifest_offset as u64,
        blob_offset: blob_offset as u64,
        cold_offset: 0,
        cold_len: 0,
        header_crc: 0,
        format_flags: FORMAT_FLAG_RAW_TRANSCODE | FORMAT_FLAG_TERNARY,
        arch_quin: NQuin::zeroed(),
        tokenizer_offset: 0,
        tokenizer_len: 0,
    };
    header.write_le(&mut head[0..Q42_WEIGHT_HEADER_BYTES]);
    for (k, e) in entries.iter().enumerate() {
        let o = manifest_offset + k * Q42_TENSOR_ENTRY_BYTES;
        e.write_le(&mut head[o..o + Q42_TENSOR_ENTRY_BYTES]);
        let entry_crc = crc32c(&head[o..o + 32]) as u64;
        head[o + 72..o + 80].copy_from_slice(&entry_crc.to_le_bytes());
    }
    let hc = crc32c_update(crc32c_update(0xFFFF_FFFF, &head[0..72]), &head[76..manifest_end]);
    head[72..76].copy_from_slice(&(!hc).to_le_bytes());
    out.write_all(&head).map_err(|e| e.to_string())?;
    let mut bytes_written = head.len();

    // 4) Stream: decode each source tensor → f32 (one in flight) → ternary-pack → write, page-aligned.
    let zeros = [0u8; 4096];
    let mut f32_scratch: Vec<f32> = Vec::new();
    let mut peak_working = 0usize;
    for (i, t) in plan.tensors.iter().enumerate() {
        let target = entries[i].blob_offset as usize;
        let mut pad = target.saturating_sub(bytes_written);
        while pad > 0 {
            let chunk = pad.min(zeros.len());
            out.write_all(&zeros[..chunk]).map_err(|e| e.to_string())?;
            bytes_written += chunk;
            pad -= chunk;
        }
        let begin = plan.data_start + t.begin;
        let end = plan.data_start + t.end;
        if end > src.len() {
            return Err(format!("transcode: tensor '{}' out of source bounds", t.name));
        }
        decode_safetensor_to_f32(&src[begin..end], ggml_types[i], counts[i], &mut f32_scratch);
        let blob = ternary_blob(&f32_scratch);
        peak_working = peak_working.max(f32_scratch.len() * 4 + blob.len());
        out.write_all(&blob).map_err(|e| e.to_string())?;
        bytes_written += blob.len();
    }

    Ok(TranscodeReport {
        n_tensors: n,
        bytes_written,
        largest_tensor_bytes: largest_out,
        total_tensor_bytes: total_out,
        peak_working_bytes: peak_working,
    })
}

/// Task #12 / STELLAR §A — **policy transcode**: ternary the FFN projections, keep everything else
/// (attention, norms, embeddings) verbatim high-fidelity, in ONE Q42W. This is the real §A policy
/// (`tensor_roles::ternary_eligible`): ternarising attention/norms wrecks coherence, so only
/// `ffn_gate`/`ffn_up`/`ffn_down` are packed to 1.6 bits; the rest pass through unchanged.
///
/// Per tensor the manifest records the engine role (from the name) and `ggml_type =
/// ternary::GGML_TYPE_TERNARY_158` for ternary blobs (decode via `ternary::dequantize_blob`) or the
/// source GGML type for verbatim blobs. Round-trips through [`Q42TensorIndex::from_q42`].
pub fn transcode_safetensor_to_q42_ffn_ternary<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    use crate::safetensor::{is_high_fidelity_ggml, parse_safetensor_header, safetensor_dtype_to_ggml};
    use crate::tensor_roles::{name_is_ternary_eligible, name_to_role};
    use crate::ternary::{ternary_blob, ternary_blob_len, GGML_TYPE_TERNARY_158};

    let page_log2 = if page_log2 == 0 { Q42W_DEFAULT_PAGE_LOG2 } else { page_log2 };
    if !(8..=30).contains(&page_log2) {
        return Err(format!("page_log2 {page_log2} out of range"));
    }
    let page = 1usize << page_log2;

    let plan = parse_safetensor_header(src)?;
    if plan.tensors.is_empty() {
        return Err("safetensor: no tensors".to_string());
    }

    // 1) dtype gate + per-tensor policy (eligible → ternary; else verbatim).
    let n = plan.tensors.len();
    let mut ggml_types = Vec::with_capacity(n);
    let mut counts = Vec::with_capacity(n);
    let mut eligible = Vec::with_capacity(n);
    for t in &plan.tensors {
        let g = safetensor_dtype_to_ggml(&t.dtype).ok_or_else(|| {
            format!("transcode: tensor '{}' dtype {} is not a high-fidelity source (rejected)", t.name, t.dtype)
        })?;
        if !is_high_fidelity_ggml(g) {
            return Err(format!("transcode: tensor '{}' is low-precision (Q4-class) — rejected", t.name));
        }
        let count: usize = if t.shape.is_empty() { 0 } else { t.shape.iter().product() };
        ggml_types.push(g);
        counts.push(count);
        eligible.push(name_is_ternary_eligible(&t.name));
    }

    // 2) Layout: ternary blob (small) for eligible tensors, verbatim bytes for the rest.
    let manifest_offset = align_up(Q42_WEIGHT_HEADER_BYTES, 16);
    let manifest_size = Q42_TENSOR_ENTRY_BYTES * n;
    let blob_offset = align_up(manifest_offset + manifest_size, page);
    let mut entries: Vec<Q42TensorEntry> = Vec::with_capacity(n);
    let mut cur = blob_offset;
    let mut total_out = 0usize;
    let mut largest_out = 0usize;
    for (i, t) in plan.tensors.iter().enumerate() {
        let blen = if eligible[i] { ternary_blob_len(counts[i]) } else { t.byte_len() };
        let ggml_out = if eligible[i] { GGML_TYPE_TERNARY_158 } else { ggml_types[i] };
        let off = align_up(cur, page);
        let rl = name_to_role(&t.name);
        let mut e = Q42TensorEntry {
            role: rl.map(|r| r.role).unwrap_or(Q42_LAYER_GLOBAL),
            layer: rl.map(|r| r.layer).unwrap_or(Q42_LAYER_GLOBAL),
            ggml_type: ggml_out,
            dim0: *t.shape.first().unwrap_or(&1) as u32,
            dim1: *t.shape.get(1).unwrap_or(&1) as u32,
            blob_offset: off as u64,
            byte_len: blen as u64,
            scaffold_quin: NQuin::zeroed(),
        };
        e.scaffold_quin.subject = crate::q_hash(t.name.as_str());
        total_out += blen;
        largest_out = largest_out.max(blen);
        entries.push(e);
        cur = off + blen;
    }

    // 3) Header + manifest + CRC.
    let manifest_end = manifest_offset + manifest_size;
    let mut head = vec![0u8; manifest_end];
    let header = Q42WeightHeader {
        magic: Q42W_MAGIC,
        version: Q42W_VERSION,
        page_log2,
        n_tensors: n as u32,
        n_layers: 0,
        n_embd: 0,
        n_head: 0,
        n_kv_head: 0,
        vocab_size: 0,
        rope_freq_base: 0.0,
        rope_scale: 0.0,
        manifest_offset: manifest_offset as u64,
        blob_offset: blob_offset as u64,
        cold_offset: 0,
        cold_len: 0,
        header_crc: 0,
        format_flags: FORMAT_FLAG_RAW_TRANSCODE | FORMAT_FLAG_TERNARY,
        arch_quin: NQuin::zeroed(),
        tokenizer_offset: 0,
        tokenizer_len: 0,
    };
    header.write_le(&mut head[0..Q42_WEIGHT_HEADER_BYTES]);
    for (k, e) in entries.iter().enumerate() {
        let o = manifest_offset + k * Q42_TENSOR_ENTRY_BYTES;
        e.write_le(&mut head[o..o + Q42_TENSOR_ENTRY_BYTES]);
        let entry_crc = crc32c(&head[o..o + 32]) as u64;
        head[o + 72..o + 80].copy_from_slice(&entry_crc.to_le_bytes());
    }
    let hc = crc32c_update(crc32c_update(0xFFFF_FFFF, &head[0..72]), &head[76..manifest_end]);
    head[72..76].copy_from_slice(&(!hc).to_le_bytes());
    out.write_all(&head).map_err(|e| e.to_string())?;
    let mut bytes_written = head.len();

    // 4) Stream blobs: ternary-pack eligible tensors, copy the rest verbatim. One in flight.
    let zeros = [0u8; 4096];
    let mut f32_scratch: Vec<f32> = Vec::new();
    let mut byte_scratch: Vec<u8> = Vec::new();
    let mut peak_working = 0usize;
    for (i, t) in plan.tensors.iter().enumerate() {
        let target = entries[i].blob_offset as usize;
        let mut pad = target.saturating_sub(bytes_written);
        while pad > 0 {
            let chunk = pad.min(zeros.len());
            out.write_all(&zeros[..chunk]).map_err(|e| e.to_string())?;
            bytes_written += chunk;
            pad -= chunk;
        }
        let begin = plan.data_start + t.begin;
        let end = plan.data_start + t.end;
        if end > src.len() {
            return Err(format!("transcode: tensor '{}' out of source bounds", t.name));
        }
        if eligible[i] {
            decode_safetensor_to_f32(&src[begin..end], ggml_types[i], counts[i], &mut f32_scratch);
            let blob = ternary_blob(&f32_scratch);
            peak_working = peak_working.max(f32_scratch.len() * 4 + blob.len());
            out.write_all(&blob).map_err(|e| e.to_string())?;
            bytes_written += blob.len();
        } else {
            byte_scratch.clear();
            byte_scratch.extend_from_slice(&src[begin..end]);
            peak_working = peak_working.max(byte_scratch.len());
            out.write_all(&byte_scratch).map_err(|e| e.to_string())?;
            bytes_written += byte_scratch.len();
        }
    }

    Ok(TranscodeReport {
        n_tensors: n,
        bytes_written,
        largest_tensor_bytes: largest_out,
        total_tensor_bytes: total_out,
        peak_working_bytes: peak_working,
    })
}

/// GGUF tensor-name suffix for a per-layer `.q42` role (None for global tensors, named directly).
fn q42_role_suffix(role: u16) -> Option<&'static [u8]> {
    match role {
        Q42_ROLE_ATTN_K => Some(b"attn_k.weight"),
        Q42_ROLE_ATTN_V => Some(b"attn_v.weight"),
        Q42_ROLE_ATTN_Q => Some(b"attn_q.weight"),
        Q42_ROLE_ATTN_OUTPUT => Some(b"attn_output.weight"),
        Q42_ROLE_FFN_GATE => Some(b"ffn_gate.weight"),
        Q42_ROLE_FFN_UP => Some(b"ffn_up.weight"),
        Q42_ROLE_FFN_DOWN => Some(b"ffn_down.weight"),
        Q42_ROLE_ATTN_NORM => Some(b"attn_norm.weight"),
        Q42_ROLE_FFN_NORM => Some(b"ffn_norm.weight"),
        _ => None,
    }
}

/// Runtime reader: parses a `.q42` container's header + manifest in microseconds. Tensor blobs
/// stay in the caller's byte slice (zero-copy); only the small manifest is materialized. The
/// `role`/`layer`/`blob_offset` fields map directly to the resident WebGPU weight arenas.
pub struct Q42TensorIndex {
    pub header: Q42WeightHeader,
    pub entries: Vec<Q42TensorEntry>,
}

impl Q42TensorIndex {
    pub fn from_q42(data: &[u8]) -> Result<Self, String> {
        if data.len() < Q42_WEIGHT_HEADER_BYTES {
            return Err("q42: too small for header".to_string());
        }
        if data[0..4] != Q42W_MAGIC {
            return Err("q42: bad magic (not Q42W)".to_string());
        }
        let header = Q42WeightHeader::read_le(&data[0..Q42_WEIGHT_HEADER_BYTES]);
        if header.version != Q42W_VERSION {
            return Err(format!("q42: unsupported version {}", header.version));
        }
        let n = header.n_tensors as usize;
        let mo = header.manifest_offset as usize;
        let manifest_end = mo
            .checked_add(n.checked_mul(Q42_TENSOR_ENTRY_BYTES).ok_or("q42: manifest overflow")?)
            .ok_or("q42: manifest overflow")?;
        if manifest_end > data.len() {
            return Err("q42: manifest out of bounds".to_string());
        }
        // Header + manifest integrity (microseconds) — rejects corrupted pointers before any bind.
        let hc = crc32c_update(crc32c_update(0xFFFF_FFFF, &data[0..72]), &data[76..manifest_end]);
        if !hc != header.header_crc {
            return Err("q42: header/manifest CRC-32C mismatch (corrupt container)".to_string());
        }
        let mut entries = Vec::with_capacity(n);
        for k in 0..n {
            let o = mo + k * Q42_TENSOR_ENTRY_BYTES;
            let e = Q42TensorEntry::read_le(&data[o..o + Q42_TENSOR_ENTRY_BYTES]);
            // Per-entry metadata CRC (in NQuin.parity) — guards offset/len before GPU bind.
            if crc32c(&data[o..o + 32]) as u64 != e.scaffold_quin.parity {
                return Err(format!("q42: tensor {k} metadata CRC-32C mismatch"));
            }
            let end = (e.blob_offset as usize)
                .checked_add(e.byte_len as usize)
                .ok_or("q42: blob overflow")?;
            if end > data.len() {
                return Err(format!("q42: tensor {k} blob out of bounds"));
            }
            entries.push(e);
        }
        Ok(Self { header, entries })
    }

    /// Reconstruct the model hyperparameters needed to boot the KV cache / attention.
    pub fn hyperparams(&self) -> crate::gguf_sharder::GgufHyperparams {
        crate::gguf_sharder::GgufHyperparams {
            n_layer: self.header.n_layers,
            n_embd: self.header.n_embd,
            n_head: self.header.n_head,
            n_kv_head: self.header.n_kv_head,
            rope_freq_base: self.header.rope_freq_base,
            rope_scale: self.header.rope_scale,
        }
    }

    /// Zero-copy view of a tensor's quantized bytes within the container.
    pub fn blob<'a>(&self, data: &'a [u8], entry: &Q42TensorEntry) -> &'a [u8] {
        let s = entry.blob_offset as usize;
        &data[s..s + entry.byte_len as usize]
    }

    /// The tokenizer section bytes within the container (empty if absent / out of bounds).
    pub fn tokenizer_bytes<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        let o = self.header.tokenizer_offset as usize;
        let l = self.header.tokenizer_len as usize;
        match o.checked_add(l) {
            Some(end) if l > 0 && end <= data.len() => &data[o..end],
            _ => &[],
        }
    }

    /// Build a GGUF-equivalent `GgufTensorIndex` from this manifest so the existing GGUF hot path
    /// (get_layer_tensors / fetch_tensor_bytes / resident upload) runs unchanged when booted from a
    /// `.q42`. The synthetic index uses `tensor_data_start = 0` and absolute blob offsets, so the
    /// byte source must be the `.q42` bytes themselves.
    pub fn to_gguf_index(&self) -> GgufTensorIndex {
        let mut named: Vec<(Vec<u8>, GgufTensorInfo)> = Vec::with_capacity(self.entries.len());
        for e in &self.entries {
            let info = GgufTensorInfo {
                dims: [e.dim0 as u64, e.dim1 as u64, 0, 0],
                n_dims: if e.dim1 > 0 { 2 } else { 1 },
                ggml_type: e.ggml_type,
                byte_offset: e.blob_offset,
            };
            let name: Vec<u8> = if e.layer == Q42_LAYER_GLOBAL {
                match e.role {
                    Q42_ROLE_TOKEN_EMBD => b"token_embd.weight".to_vec(),
                    Q42_ROLE_OUTPUT => b"output.weight".to_vec(),
                    Q42_ROLE_OUTPUT_NORM => b"output_norm.weight".to_vec(),
                    _ => continue,
                }
            } else if let Some(suffix) = q42_role_suffix(e.role) {
                let mut buf = [0u8; 96];
                let n = crate::gguf_sharder::write_blk_tensor_name(e.layer as u32, suffix, &mut buf);
                buf[..n].to_vec()
            } else {
                continue;
            };
            named.push((name, info));
        }
        let refs: Vec<(&[u8], GgufTensorInfo)> =
            named.iter().map(|(n, i)| (n.as_slice(), *i)).collect();
        GgufTensorIndex::from_components(&refs, self.hyperparams(), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal safetensor with the given `(name, dtype, shape, nbytes)` tensors (zeroed
    /// data, but each tensor stamped with a distinct first byte so round-trips are checkable).
    fn synth_safetensor(t: &[(&str, &str, Vec<usize>, usize)]) -> Vec<u8> {
        let mut entries = serde_json::Map::new();
        let mut cursor = 0usize;
        for (name, dtype, shape, nbytes) in t {
            let (begin, end) = (cursor, cursor + nbytes);
            cursor = end;
            entries.insert(
                (*name).to_string(),
                serde_json::json!({ "dtype": dtype, "shape": shape, "data_offsets": [begin, end] }),
            );
        }
        let header_bytes = serde_json::to_vec(&serde_json::Value::Object(entries)).unwrap();
        let mut out = Vec::new();
        out.extend_from_slice(&(header_bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(&header_bytes);
        let data_start = out.len();
        out.resize(out.len() + cursor, 0u8);
        // stamp each tensor's first byte with an index so we can verify the right bytes landed.
        let plan = crate::safetensor::parse_safetensor_header(&out).unwrap();
        for (i, te) in plan.tensors.iter().enumerate() {
            out[data_start + te.begin] = (i as u8) + 1;
        }
        out
    }

    /// GATE B: streaming safetensor → Q42W round-trips, and peak working memory ≈ the largest
    /// single tensor (NOT the whole file).
    #[test]
    fn transcode_safetensor_streams_and_round_trips() {
        // three F16 tensors of 8 / 64 / 16 bytes (largest = 64; total = 88).
        let src = synth_safetensor(&[
            ("a", "F16", vec![4], 8),
            ("big", "F16", vec![32], 64),
            ("c", "F16", vec![8], 16),
        ]);
        let mut out = Vec::new();
        let report = transcode_safetensor_to_q42(&src, 12, &mut out).unwrap();

        // peak working memory == largest tensor, and strictly less than the sum (not the whole file).
        assert_eq!(report.n_tensors, 3);
        assert_eq!(report.largest_tensor_bytes, 64);
        assert_eq!(report.total_tensor_bytes, 88);
        assert_eq!(report.peak_working_bytes, 64, "one tensor in flight = largest, not the file");
        assert!(report.peak_working_bytes < report.total_tensor_bytes);

        // the emitted container is a valid Q42W and parses back.
        let idx = Q42TensorIndex::from_q42(&out).expect("transcoded container must round-trip");
        assert_eq!(idx.header.n_tensors, 3);
        assert_eq!(idx.header.format_flags & FORMAT_FLAG_RAW_TRANSCODE, FORMAT_FLAG_RAW_TRANSCODE);
        assert_eq!(idx.entries.len(), 3);

        // tensor bytes survived verbatim: each blob's first byte is its stamp; sizes match.
        let plan = crate::safetensor::parse_safetensor_header(&src).unwrap();
        for (i, (e, st)) in idx.entries.iter().zip(plan.tensors.iter()).enumerate() {
            let blob = idx.blob(&out, e);
            assert_eq!(blob.len(), st.byte_len());
            assert_eq!(blob[0], (i as u8) + 1, "tensor {i} bytes mismatch");
            // identity preserved as the name hash.
            assert_eq!(e.scaffold_quin.subject, crate::q_hash(st.name.as_str()));
        }
    }

    /// GATE B: a low-precision (Q4-class) dtype is rejected — high-fidelity sources only.
    #[test]
    fn transcode_rejects_low_precision() {
        // "U8" is not a high-fidelity weight dtype → the dtype gate rejects it.
        let src = synth_safetensor(&[("w", "U8", vec![16], 16)]);
        let mut out = Vec::new();
        let err = transcode_safetensor_to_q42(&src, 12, &mut out).unwrap_err();
        assert!(err.contains("high-fidelity") || err.contains("rejected"), "got: {err}");
        // and the underlying GGML gate rejects Q4_K directly.
        assert!(!crate::safetensor::is_high_fidelity_ggml(12));
    }

    /// TASK #12 (§A): ternary transcode compresses an F16 tensor to ≈1.6 bits/weight and the
    /// container round-trips + dequantizes correctly.
    #[test]
    fn transcode_ternary_compresses_and_round_trips() {
        // one F16 tensor, 100 weights of alternating ±2.0 (absmean scale = 2.0, exact reconstruction)
        let count = 100usize;
        let weights: Vec<f32> = (0..count).map(|i| if i % 2 == 0 { 2.0 } else { -2.0 }).collect();
        let mut data = Vec::new();
        for &w in &weights {
            data.extend_from_slice(&half::f16::from_f32(w).to_le_bytes());
        }
        let header = serde_json::json!({ "w": { "dtype": "F16", "shape": [count], "data_offsets": [0, data.len()] } });
        let hb = serde_json::to_vec(&header).unwrap();
        let mut src = Vec::new();
        src.extend_from_slice(&(hb.len() as u64).to_le_bytes());
        src.extend_from_slice(&hb);
        src.extend_from_slice(&data);

        let mut out = Vec::new();
        let report = transcode_safetensor_to_q42_ternary(&src, 12, &mut out).unwrap();
        assert_eq!(report.n_tensors, 1);

        // source F16 tensor = 200 bytes; ternary blob = 4 + ceil(100/5) = 24 bytes (>8x smaller).
        assert_eq!(report.total_tensor_bytes, crate::ternary::ternary_blob_len(count));
        assert!(report.total_tensor_bytes * 5 < data.len(), "ternary must be >5x smaller than F16");

        // container round-trips and is flagged ternary.
        let idx = Q42TensorIndex::from_q42(&out).expect("ternary container must round-trip");
        assert_eq!(idx.header.format_flags & FORMAT_FLAG_TERNARY, FORMAT_FLAG_TERNARY);
        assert_eq!(idx.entries[0].ggml_type, crate::ternary::GGML_TYPE_TERNARY_158);

        // dequantize: uniform ±2.0 → scale (absmean) = 2.0, so reconstruction is exact ±2.0.
        let blob = idx.blob(&out, &idx.entries[0]);
        let mut deq = vec![0.0f32; count];
        crate::ternary::dequantize_blob(blob, &mut deq);
        assert!((deq[0] - 2.0).abs() < 1e-3, "deq[0] {}", deq[0]);
        assert!((deq[1] + 2.0).abs() < 1e-3, "deq[1] {}", deq[1]);
    }

    /// TASK #12 (§A): policy transcode ternaries the FFN, keeps attention/norm verbatim, populates
    /// engine roles, and round-trips — all in one container.
    #[test]
    fn transcode_ffn_ternary_policy_mixed_container() {
        // Q42_ROLE_* and Q42_LAYER_GLOBAL are in scope via `use super::*`.
        // three HF-named F16 tensors: an FFN gate (ternary), an attention q_proj + a norm (verbatim).
        let count = 50usize;
        let f16 = |v: f32| half::f16::from_f32(v).to_le_bytes();
        let mut gate = Vec::new();
        let mut q = Vec::new();
        let mut norm = Vec::new();
        for i in 0..count {
            gate.extend_from_slice(&f16(if i % 2 == 0 { 1.0 } else { -1.0 }));
            q.extend_from_slice(&f16(0.25));
            norm.extend_from_slice(&f16(0.5));
        }
        let (gl, ql) = (gate.len(), q.len());
        let header = serde_json::json!({
            "model.layers.0.mlp.gate_proj.weight": { "dtype": "F16", "shape": [count], "data_offsets": [0, gl] },
            "model.layers.0.self_attn.q_proj.weight": { "dtype": "F16", "shape": [count], "data_offsets": [gl, gl + ql] },
            "model.norm.weight": { "dtype": "F16", "shape": [count], "data_offsets": [gl + ql, gl + ql + norm.len()] },
        });
        let hb = serde_json::to_vec(&header).unwrap();
        let mut src = Vec::new();
        src.extend_from_slice(&(hb.len() as u64).to_le_bytes());
        src.extend_from_slice(&hb);
        src.extend_from_slice(&gate);
        src.extend_from_slice(&q);
        src.extend_from_slice(&norm);

        let mut out = Vec::new();
        let report = transcode_safetensor_to_q42_ffn_ternary(&src, 12, &mut out).unwrap();
        assert_eq!(report.n_tensors, 3);

        let idx = Q42TensorIndex::from_q42(&out).expect("mixed container must round-trip");
        // entries are ordered by source offset: gate, q_proj, norm.
        let by_role = |role: u16| idx.entries.iter().find(|e| e.role == role).expect("role present");

        // FFN gate → ternary, FFN_GATE role, much smaller than its F16 source (100 bytes).
        let g = by_role(Q42_ROLE_FFN_GATE);
        assert_eq!(g.ggml_type, crate::ternary::GGML_TYPE_TERNARY_158);
        assert_eq!(g.layer, 0);
        assert_eq!(g.byte_len as usize, crate::ternary::ternary_blob_len(count)); // 4 + ceil(50/5) = 14
        assert!((g.byte_len as usize) * 5 < gate.len());

        // attention q_proj → verbatim F16, ATTN_Q role.
        let a = by_role(Q42_ROLE_ATTN_Q);
        assert_eq!(a.ggml_type, crate::safetensor::GGML_F16);
        assert_eq!(a.byte_len as usize, ql); // verbatim, unchanged
        assert_eq!(idx.blob(&out, a), &q[..]); // bytes preserved exactly

        // norm → verbatim, OUTPUT_NORM (global).
        let nrm = by_role(Q42_ROLE_OUTPUT_NORM);
        assert_eq!(nrm.ggml_type, crate::safetensor::GGML_F16);
        assert_eq!(nrm.layer, Q42_LAYER_GLOBAL);

        // the FFN blob dequantizes (±1.0 uniform → scale 1.0 → ±1.0).
        let mut deq = vec![0.0f32; count];
        crate::ternary::dequantize_blob(idx.blob(&out, g), &mut deq);
        assert!((deq[0] - 1.0).abs() < 1e-3 && (deq[1] + 1.0).abs() < 1e-3);
    }

    fn le_u16(b: &[u8], o: usize) -> u16 {
        u16::from_le_bytes(b[o..o + 2].try_into().unwrap())
    }
    fn le_u32(b: &[u8], o: usize) -> u32 {
        u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
    }
    fn le_u64(b: &[u8], o: usize) -> u64 {
        u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
    }

    #[test]
    fn compile_smollm2_to_q42_layout() {
        let path = "C:/Projects/qualiaDB/docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf";
        if !std::path::Path::new(path).exists() {
            eprintln!("[q42] model not present — skipping");
            return;
        }
        let gguf = std::fs::read(path).expect("read gguf");
        let q42 = compile_gguf_to_q42(&gguf, 0).expect("compile");

        // Magic + version + default page size.
        assert_eq!(&q42[0..4], b"Q42W", "magic");
        assert_eq!(le_u16(&q42, 4), Q42W_VERSION, "version");
        assert_eq!(le_u16(&q42, 6), 14, "default page_log2 = 16KB");
        let page = 1usize << 14;

        // Tensor count: SmolLM2-360M has 32 layers × 9 per-layer tensors + globals.
        let n_tensors = le_u32(&q42, 8) as usize;
        let n_layers = le_u32(&q42, 12);
        assert_eq!(n_layers, 32, "n_layers");
        assert!(n_tensors >= 32 * 9, "expected ≥288 tensors, got {n_tensors}");

        // Hyperparameter block (v2 header) round-trips SmolLM2-360M geometry.
        assert_eq!(le_u32(&q42, 16), 960, "n_embd");
        assert_eq!(le_u32(&q42, 20), 15, "n_head");
        assert_eq!(le_u32(&q42, 24), 5, "n_kv_head");

        // Blob region + the first tensor blob both sit on a 16KB boundary.
        let manifest_offset = le_u64(&q42, 40) as usize;
        let blob_offset = le_u64(&q42, 48) as usize;
        assert_eq!(blob_offset % page, 0, "blob region 16KB-aligned");
        let first_entry = manifest_offset; // entry[0]
        let first_blob = le_u64(&q42, first_entry + 16) as usize; // blob_offset field @ entry+16
        let first_len = le_u64(&q42, first_entry + 24) as usize;
        assert_eq!(first_blob % page, 0, "first tensor blob 16KB-aligned");
        assert_eq!(first_blob, blob_offset, "first blob == blob region start");
        assert!(first_blob + first_len <= q42.len(), "first blob in-bounds");

        // Every tensor blob is 16KB-aligned and in-bounds.
        for k in 0..n_tensors {
            let e = manifest_offset + k * Q42_TENSOR_ENTRY_BYTES;
            let bo = le_u64(&q42, e + 16) as usize;
            let bl = le_u64(&q42, e + 24) as usize;
            assert_eq!(bo % page, 0, "tensor {k} blob 16KB-aligned");
            assert!(bo + bl <= q42.len(), "tensor {k} in-bounds");
        }

        // Round-trip through the runtime reader.
        let idx = Q42TensorIndex::from_q42(&q42).expect("from_q42");
        assert_eq!(idx.entries.len(), n_tensors, "reader entry count");
        assert_eq!(idx.header.blob_offset as usize, blob_offset, "reader blob_offset");
        let hp = idx.hyperparams();
        assert_eq!(hp.n_layer, 32);
        assert_eq!(hp.n_embd, 960);
        assert_eq!(hp.n_head, 15);
        assert_eq!(hp.effective_n_kv_head(), 5);
        for (k, e) in idx.entries.iter().enumerate() {
            assert_eq!(e.blob_offset as usize % page, 0, "reader entry {k} aligned");
            assert_eq!(idx.blob(&q42, e).len(), e.byte_len as usize, "reader blob len {k}");
        }
        // Bad magic is rejected.
        let mut bad = q42.clone();
        bad[0] = b'X';
        assert!(Q42TensorIndex::from_q42(&bad).is_err(), "bad magic rejected");

        // Integrity: header CRC populated; a flipped manifest byte (corrupted offset) is rejected.
        assert_ne!(le_u32(&q42, 72), 0, "header_crc populated");
        let mut tampered = q42.clone();
        tampered[manifest_offset + 16] ^= 0xFF; // first entry's blob_offset
        assert!(
            Q42TensorIndex::from_q42(&tampered).is_err(),
            "manifest tamper must be caught by CRC before any bind"
        );

        eprintln!(
            "[q42] OK: {n_tensors} tensors, {n_layers} layers, blob@{blob_offset}, total {} MB; reader round-trip + hyperparams verified",
            q42.len() / (1024 * 1024)
        );
    }

    /// Proves inference-from-`.q42` equivalence WITHOUT a browser: the synthetic GGUF index built
    /// from the `.q42` manifest returns byte-identical weights + matching metadata vs the original
    /// GGUF index for every tensor. Identical weights → identical logits → identical output. The
    /// only piece the `.q42` does not yet carry is the tokenizer (a separate, flagged gap).
    #[test]
    fn q42_synthetic_index_matches_gguf() {
        let path = "C:/Projects/qualiaDB/docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf";
        if !std::path::Path::new(path).exists() {
            eprintln!("[q42] model not present — skipping");
            return;
        }
        let gguf = std::fs::read(path).expect("read gguf");
        let q42 = compile_gguf_to_q42(&gguf, 0).expect("compile");
        let orig = GgufTensorIndex::from_gguf(&gguf);
        let q = Q42TensorIndex::from_q42(&q42).expect("from_q42");
        let synth = q.to_gguf_index();

        let mut checked = 0usize;
        let mut cmp = |label: &str, s: Option<GgufTensorInfo>, o: Option<GgufTensorInfo>| {
            match (s, o) {
                (Some(s), Some(o)) => {
                    assert_eq!(s.ggml_type, o.ggml_type, "{label} ggml_type");
                    assert_eq!(s.dims[0], o.dims[0], "{label} dim0");
                    assert_eq!(s.dims[1], o.dims[1], "{label} dim1");
                    let sb = crate::ggml_quants::fetch_tensor_bytes(&q42, synth.tensor_data_start, &s)
                        .expect("q42 tensor bytes");
                    let ob = crate::ggml_quants::fetch_tensor_bytes(&gguf, orig.tensor_data_start, &o)
                        .expect("gguf tensor bytes");
                    assert_eq!(sb, ob, "{label} weight bytes differ");
                    checked += 1;
                }
                (None, None) => {}
                _ => panic!("{label}: tensor presence mismatch (synthetic vs gguf)"),
            }
        };
        for l in 0..orig.hyperparams.n_layer {
            let st = synth.get_layer_tensors(l);
            let ot = orig.get_layer_tensors(l);
            cmp(&format!("L{l}.attn_q"), st.attn_q, ot.attn_q);
            cmp(&format!("L{l}.attn_k"), st.attn_k, ot.attn_k);
            cmp(&format!("L{l}.attn_v"), st.attn_v, ot.attn_v);
            cmp(&format!("L{l}.attn_output"), st.attn_output, ot.attn_output);
            cmp(&format!("L{l}.attn_norm"), st.attn_norm, ot.attn_norm);
            cmp(&format!("L{l}.ffn_gate"), st.ffn_gate, ot.ffn_gate);
            cmp(&format!("L{l}.ffn_up"), st.ffn_up, ot.ffn_up);
            cmp(&format!("L{l}.ffn_down"), st.ffn_down, ot.ffn_down);
            cmp(&format!("L{l}.ffn_norm"), st.ffn_norm, ot.ffn_norm);
        }
        cmp("token_embd", synth.token_embd_info().copied(), orig.token_embd_info().copied());
        cmp("output", synth.output_weight_info().copied(), orig.output_weight_info().copied());
        cmp("output_norm", synth.output_norm_info().copied(), orig.output_norm_info().copied());

        assert!(checked >= 32 * 9, "expected ≥288 tensors byte-checked, got {checked}");
        eprintln!("[q42] synthetic index == GGUF: {checked} tensors byte-identical + metadata match");
    }

    /// Proves the v3 tokenizer section round-trips: a tokenizer rebuilt from the `.q42` section
    /// encodes/decodes identically to the GGUF tokenizer. With weight byte-parity (above), this
    /// guarantees q42-only inference produces the same tokens as the GGUF path.
    #[test]
    fn q42_tokenizer_roundtrip() {
        use crate::gguf_sharder::GgufTokenizer;
        let path = "C:/Projects/qualiaDB/docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf";
        if !std::path::Path::new(path).exists() {
            eprintln!("[q42] model not present — skipping");
            return;
        }
        let gguf = std::fs::read(path).expect("read gguf");
        let q42 = compile_gguf_to_q42(&gguf, 0).expect("compile");
        let q = Q42TensorIndex::from_q42(&q42).expect("from_q42");

        let tok_bytes = q.tokenizer_bytes(&q42);
        assert!(!tok_bytes.is_empty(), "tokenizer section present");
        let tok_q42 = GgufTokenizer::from_q42_section(tok_bytes).expect("from_q42_section");
        let tok_gguf = GgufTokenizer::from_gguf(&gguf);

        assert_eq!(tok_q42.bos_token_id, tok_gguf.bos_token_id, "bos");
        assert_eq!(tok_q42.eos_token_id, tok_gguf.eos_token_id, "eos");
        assert_eq!(tok_q42.add_bos_token, tok_gguf.add_bos_token, "add_bos");
        assert_eq!(tok_q42.vocab.len(), tok_gguf.vocab.len(), "vocab len");
        for prompt in [
            "The capital of France is",
            "<|im_start|>user\nWhat is the capital of France?<|im_end|>\n<|im_start|>assistant\n",
        ] {
            assert_eq!(
                tok_q42.encode_prompt(prompt),
                tok_gguf.encode_prompt(prompt),
                "encode mismatch for {prompt:?}"
            );
        }
        let ids = tok_gguf.encode_prompt("The capital of France is");
        assert_eq!(tok_q42.decode(&ids), tok_gguf.decode(&ids), "decode mismatch");
        eprintln!(
            "[q42] tokenizer round-trip: encode/decode identical to GGUF ({} vocab, section {} KB)",
            tok_q42.vocab.len(),
            tok_bytes.len() / 1024
        );
    }
}
