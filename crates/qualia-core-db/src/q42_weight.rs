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
/// v2 added the hyperparameter block (n_embd/n_head/n_kv_head/vocab/rope) so a `.q42` is a
/// self-contained execution container — the runtime builds the KV cache without the GGUF.
pub const Q42W_VERSION: u16 = 2;
/// 14 = 16 KB pages (default; minimizes page faults on large FFN blocks). 12 = 4 KB.
pub const Q42W_DEFAULT_PAGE_LOG2: u16 = 14;
pub const Q42_WEIGHT_HEADER_BYTES: usize = 128;
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
    // `header_crc`/`format_flags` (8B) keep the 16-aligned NQuin at offset 80, zero padding —
    // preserving the 128B size assert and the manual LE offsets.
    pub arch_quin: NQuin,
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
    let total = align_up(cur, 16);

    // 4. Emit: header + manifest + page-aligned blobs (zero-padded gaps).
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
    Ok(out)
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
