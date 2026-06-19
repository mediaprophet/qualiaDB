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
pub const Q42W_VERSION: u16 = 1;
/// 14 = 16 KB pages (default; minimizes page faults on large FFN blocks). 12 = 4 KB.
pub const Q42W_DEFAULT_PAGE_LOG2: u16 = 14;
pub const Q42_WEIGHT_HEADER_BYTES: usize = 96;
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

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug)]
pub struct Q42WeightHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub page_log2: u16,
    pub n_tensors: u32,
    pub n_layers: u32,
    pub manifest_offset: u64,
    pub blob_offset: u64,
    pub cold_offset: u64,
    pub cold_len: u64,
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

impl Q42WeightHeader {
    fn write_le(&self, b: &mut [u8]) {
        b[0..4].copy_from_slice(&self.magic);
        b[4..6].copy_from_slice(&self.version.to_le_bytes());
        b[6..8].copy_from_slice(&self.page_log2.to_le_bytes());
        b[8..12].copy_from_slice(&self.n_tensors.to_le_bytes());
        b[12..16].copy_from_slice(&self.n_layers.to_le_bytes());
        b[16..24].copy_from_slice(&self.manifest_offset.to_le_bytes());
        b[24..32].copy_from_slice(&self.blob_offset.to_le_bytes());
        b[32..40].copy_from_slice(&self.cold_offset.to_le_bytes());
        b[40..48].copy_from_slice(&self.cold_len.to_le_bytes());
        write_nquin_le(&self.arch_quin, &mut b[48..96]);
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
    let header = Q42WeightHeader {
        magic: Q42W_MAGIC,
        version: Q42W_VERSION,
        page_log2,
        n_tensors: planned.len() as u32,
        n_layers: n_layer,
        manifest_offset: manifest_offset as u64,
        blob_offset: blob_offset as u64,
        cold_offset: 0,
        cold_len: 0,
        arch_quin: NQuin::zeroed(),
    };
    let mut out = vec![0u8; total];
    header.write_le(&mut out[0..Q42_WEIGHT_HEADER_BYTES]);
    for (k, e) in entries.iter().enumerate() {
        let o = manifest_offset + k * Q42_TENSOR_ENTRY_BYTES;
        e.write_le(&mut out[o..o + Q42_TENSOR_ENTRY_BYTES]);
    }
    for (k, (_, _, info)) in planned.iter().enumerate() {
        let src = tds + info.byte_offset as usize;
        let len = entries[k].byte_len as usize;
        let dst = entries[k].blob_offset as usize;
        out[dst..dst + len].copy_from_slice(&input[src..src + len]);
    }
    Ok(out)
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

        // Blob region + the first tensor blob both sit on a 16KB boundary.
        let manifest_offset = le_u64(&q42, 16) as usize;
        let blob_offset = le_u64(&q42, 24) as usize;
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

        eprintln!(
            "[q42] OK: {n_tensors} tensors, {n_layers} layers, blob@{blob_offset}, total {} MB",
            q42.len() / (1024 * 1024)
        );
    }
}
