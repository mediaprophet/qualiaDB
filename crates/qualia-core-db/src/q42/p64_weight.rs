//! Phase 4: AOT GGUF → `.p64` **LLM-weight container** compiler.
//!
//! A *sibling* of the semantic `.q42` graph format — it carries an **independent section magic** `b"p64\0"`
//! so the two never collide. Per the architectural decision, the weights are stored as **opaque,
//! cache-aligned (64-byte), contiguous quantized blobs**.
//!
//! The `NQuin` epistemic scaffold is removed from this probabilistic container. The 48-byte
//! declarative `q42` system manages truth, while this 64-byte aligned `p64` system manages
//! pure mathematical inference with zero-copy relative WASM pointers.
//!
//! Output is **little-endian** on every host via explicit serialization.
//!
//! Layout:
//! ```text
//! [ P64WeightHeader (64B) ]              magic, version, flags, 32-bit relative offsets
//! [ P64TensorEntry[] (64B each) ]        role, dtype, rank, dims, relative blob offsets
//! [ pad → 1<<page_log2 ]
//! [ tensor blob region ]                 quantized bytes; each tensor START page-aligned
//!                                        (default 16KB) for single-fetch mmap.
//! ```

use crate::gguf_sharder::{GgufHyperparams, GgufTensorIndex, GgufTensorInfo};

pub const P64_MAGIC: [u8; 4] = *b"p64\0";
pub const P64_VERSION: u16 = 4;

/// Return `true` only for the canonical four-byte P64 container magic.
///
/// Keep format sniffing centralized here. Historical code used `.q42` names
/// and, in one WASM path, compared against the non-canonical `b"P64"` literal.
#[inline]
pub fn has_p64_magic(data: &[u8]) -> bool {
    data.starts_with(&P64_MAGIC)
}
/// 14 = 16 KB pages (default; minimizes page faults on large FFN blocks). 12 = 4 KB.
pub const P64_DEFAULT_PAGE_LOG2: u16 = 14;
pub const P64_WEIGHT_HEADER_BYTES: usize = 64;
pub const P64_TENSOR_ENTRY_BYTES: usize = 64;
/// Ten little-endian `f32` values plus 24 bytes of zero padding.
///
/// Keeping every coordinate in one cache line makes `manifold_idx` an exact
/// 64-byte stride and prevents neighbouring coordinates from sharing a cache
/// line or a WASM SIMD fetch.
pub const P64_MANIFOLD_ENTRY_BYTES: usize = 64;
pub const P64_FLAG_LITTLE_ENDIAN: u16 = 1;

// Tensor roles.
pub const P64_ROLE_ATTN_K: u16 = 0;
pub const P64_ROLE_ATTN_V: u16 = 1;
pub const P64_ROLE_ATTN_Q: u16 = 2;
pub const P64_ROLE_ATTN_OUTPUT: u16 = 3;
pub const P64_ROLE_FFN_GATE: u16 = 4;
pub const P64_ROLE_FFN_UP: u16 = 5;
pub const P64_ROLE_FFN_DOWN: u16 = 6;
pub const P64_ROLE_ATTN_NORM: u16 = 7;
pub const P64_ROLE_FFN_NORM: u16 = 8;
pub const P64_ROLE_TOKEN_EMBD: u16 = 9;
pub const P64_ROLE_OUTPUT: u16 = 10;
pub const P64_ROLE_OUTPUT_NORM: u16 = 11;
pub const P64_ROLE_ATTN_SUBLN: u16 = 12;
pub const P64_ROLE_FFN_SUBLN: u16 = 13;
/// A source GGUF tensor preserved byte-for-byte but not consumed by a known
/// engine role. Its source offset and name hash remain in the entry so a
/// validator can still prove complete model preservation.
pub const P64_ROLE_UNKNOWN: u16 = 0xFFFE;
/// `layer` sentinel for non-layer (global) tensors.
pub const P64_LAYER_GLOBAL: u16 = 0xFFFF;

// Metadata bitfields are handled by the q42 layer, no longer embedded in weights.

// CRC-32C (Castagnoli, reflected) — delegated to the shared
// `container_10d::crc32c` module (P0.3 consolidation). The algorithm is
// byte-identical to the previous in-line implementation; the p64 round-trip
// tests verify the checksums are unchanged after delegation.
use crate::container_10d::crc32c::crc32c;

#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct P64WeightHeader {
    pub magic: [u8; 4], // b"p64\0"
    pub version: u16,   // 3
    pub flags: u16,     // Endianness

    // 32-bit Relative Offsets (WASM-native)
    pub role_table_offset: u32,     // Maps tensors to semantic roles
    pub tensor_table_offset: u32,   // Descriptor table (shape, dtype)
    pub tokenizer_offset: u32,      // Embedded tokenizer vocabulary
    pub hparams_offset: u32,        // Hyperparameters
    pub string_table_offset: u32,   // Centralized string pool
    pub checksum_offset: u32,       // Cryptographic hash for tamper-evidence
    pub manifold_table_offset: u32, // Offset to 10D ManifoldCoordinate10D table

    pub tensor_count: u32, // Number of tensors
    pub page_size: u32,    // Hardware alignment (e.g., 4096)

    pub reserved: [u8; 20], // Pad exactly to 64 bytes
}

#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct P64TensorEntry {
    pub name_offset: u32,      // Relative offset to string table
    pub role_id: u16,          // Standardized enum (e.g., P64_ROLE_FFN_UP)
    pub dtype: u16,            // Data type (FP32, Q4_K, etc.)
    pub manifold_idx: u32,     // Index into the 10D Manifold table (replaces flat layers)
    pub rank: u32,             // Number of dimensions
    pub dimensions: [u32; 4],  // Shape of the tensor
    pub blob_offset: u32,      // Relative offset to tensor data
    pub blob_size: u32,        // Size in bytes
    pub source_offset: u64,    // Original offset inside the GGUF tensor-data block
    pub source_name_hash: u64, // Original GGUF tensor-name hash
    pub reserved: [u8; 8],     // Pad exactly to 64 bytes
}

#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct P64HParams {
    pub n_layer: u32,
    pub n_embd: u32,
    pub n_head: u32,
    pub n_kv_head: u32,
    pub vocab_size: u32,
    pub rope_freq_base: f32,
    pub rope_scale: f32,
    pub reserved: [u8; 36], // Pad exactly to 64 bytes
}

// Layouts are exact multiples of 64 for Cache-Line DOD perfection.
const _: () = assert!(core::mem::size_of::<P64WeightHeader>() == P64_WEIGHT_HEADER_BYTES);
const _: () = assert!(core::mem::size_of::<P64TensorEntry>() == P64_TENSOR_ENTRY_BYTES);
const _: () = assert!(core::mem::size_of::<P64HParams>() == 64);

#[inline]
fn align_up(x: usize, a: usize) -> usize {
    debug_assert!(a.is_power_of_two());
    (x + a - 1) & !(a - 1)
}

impl P64WeightHeader {
    pub fn read_le(data: &[u8]) -> Result<Self, String> {
        if data.len() < P64_WEIGHT_HEADER_BYTES {
            return Err("p64: truncated header".to_string());
        }
        let u16a = |o: usize| u16::from_le_bytes(data[o..o + 2].try_into().unwrap());
        let u32a = |o: usize| u32::from_le_bytes(data[o..o + 4].try_into().unwrap());
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&data[0..4]);
        Ok(Self {
            magic,
            version: u16a(4),
            flags: u16a(6),
            role_table_offset: u32a(8),
            tensor_table_offset: u32a(12),
            tokenizer_offset: u32a(16),
            hparams_offset: u32a(20),
            string_table_offset: u32a(24),
            checksum_offset: u32a(28),
            manifold_table_offset: u32a(32),
            tensor_count: u32a(36),
            page_size: u32a(40),
            reserved: [0; 20],
        })
    }

    pub fn write_le(&self, out: &mut [u8]) {
        assert!(out.len() >= P64_WEIGHT_HEADER_BYTES);
        out[..P64_WEIGHT_HEADER_BYTES].fill(0);
        out[0..4].copy_from_slice(&self.magic);
        out[4..6].copy_from_slice(&self.version.to_le_bytes());
        out[6..8].copy_from_slice(&self.flags.to_le_bytes());
        out[8..12].copy_from_slice(&self.role_table_offset.to_le_bytes());
        out[12..16].copy_from_slice(&self.tensor_table_offset.to_le_bytes());
        out[16..20].copy_from_slice(&self.tokenizer_offset.to_le_bytes());
        out[20..24].copy_from_slice(&self.hparams_offset.to_le_bytes());
        out[24..28].copy_from_slice(&self.string_table_offset.to_le_bytes());
        out[28..32].copy_from_slice(&self.checksum_offset.to_le_bytes());
        out[32..36].copy_from_slice(&self.manifold_table_offset.to_le_bytes());
        out[36..40].copy_from_slice(&self.tensor_count.to_le_bytes());
        out[40..44].copy_from_slice(&self.page_size.to_le_bytes());
    }
}

impl P64HParams {
    fn read_le(data: &[u8]) -> Result<Self, String> {
        if data.len() < 64 {
            return Err("p64: truncated hyperparameters".to_string());
        }
        let u32a = |o: usize| u32::from_le_bytes(data[o..o + 4].try_into().unwrap());
        let f32a = |o: usize| f32::from_le_bytes(data[o..o + 4].try_into().unwrap());
        Ok(Self {
            n_layer: u32a(0),
            n_embd: u32a(4),
            n_head: u32a(8),
            n_kv_head: u32a(12),
            vocab_size: u32a(16),
            rope_freq_base: f32a(20),
            rope_scale: f32a(24),
            reserved: [0; 36],
        })
    }

    fn write_le(&self, out: &mut [u8]) {
        assert!(out.len() >= 64);
        out[..64].fill(0);
        out[0..4].copy_from_slice(&self.n_layer.to_le_bytes());
        out[4..8].copy_from_slice(&self.n_embd.to_le_bytes());
        out[8..12].copy_from_slice(&self.n_head.to_le_bytes());
        out[12..16].copy_from_slice(&self.n_kv_head.to_le_bytes());
        out[16..20].copy_from_slice(&self.vocab_size.to_le_bytes());
        out[20..24].copy_from_slice(&self.rope_freq_base.to_le_bytes());
        out[24..28].copy_from_slice(&self.rope_scale.to_le_bytes());
    }
}

fn write_manifold_coordinate(
    coordinate: &crate::modalities::manifold::ManifoldCoordinate10D,
    out: &mut [u8],
) {
    assert!(out.len() >= P64_MANIFOLD_ENTRY_BYTES);
    out[..P64_MANIFOLD_ENTRY_BYTES].fill(0);
    for (index, value) in coordinate.as_f32_array().iter().enumerate() {
        let start = index * 4;
        out[start..start + 4].copy_from_slice(&value.to_le_bytes());
    }
}

fn write_tensor_entry(entry: &P64TensorEntry, out: &mut [u8]) {
    assert!(out.len() >= P64_TENSOR_ENTRY_BYTES);
    out[..P64_TENSOR_ENTRY_BYTES].fill(0);
    out[0..4].copy_from_slice(&entry.name_offset.to_le_bytes());
    out[4..6].copy_from_slice(&entry.role_id.to_le_bytes());
    out[6..8].copy_from_slice(&entry.dtype.to_le_bytes());
    out[8..12].copy_from_slice(&entry.manifold_idx.to_le_bytes());
    out[12..16].copy_from_slice(&entry.rank.to_le_bytes());
    for (index, dimension) in entry.dimensions.iter().enumerate() {
        let start = 16 + index * 4;
        out[start..start + 4].copy_from_slice(&dimension.to_le_bytes());
    }
    out[32..36].copy_from_slice(&entry.blob_offset.to_le_bytes());
    out[36..40].copy_from_slice(&entry.blob_size.to_le_bytes());
    out[40..48].copy_from_slice(&entry.source_offset.to_le_bytes());
    out[48..56].copy_from_slice(&entry.source_name_hash.to_le_bytes());
}

fn p64_tensor_name(role: u16, layer: u16, source_name_hash: u64) -> String {
    if layer == P64_LAYER_GLOBAL {
        return match role {
            P64_ROLE_TOKEN_EMBD => "token_embd.weight".to_string(),
            P64_ROLE_OUTPUT => "output.weight".to_string(),
            P64_ROLE_OUTPUT_NORM => "output_norm.weight".to_string(),
            _ => format!("tensor.{source_name_hash:016x}"),
        };
    }
    match p64_role_suffix(role) {
        Some(suffix) => format!("blk.{layer}.{}", String::from_utf8_lossy(suffix)),
        None => format!("tensor.{source_name_hash:016x}"),
    }
}

/// Conversion-time weight layout policy for [`compile_gguf_to_p64_with_layout`].
///
/// The designed product path is: import GGUF once → store GPU-friendly bytes in `.p64`.
/// [`P64ConvertLayout::Verbatim`] is the historical byte-preserving container swap
/// (same kernels, same speed). [`P64ConvertLayout::F16Expand`] dequantizes 2-D weight
/// matrices to IEEE f16 so decode can use the fast `unpack2x16float` path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum P64ConvertLayout {
    /// Copy GGML quant blocks byte-for-byte (no speed change vs running the GGUF).
    #[default]
    Verbatim,
    /// Expand 2-D matrices (attn/FFN/embd/output) to f16; leave 1-D norms as source.
    /// Rejected if the result would exceed the 4 GiB u32-offset container limit.
    F16Expand,
}

/// Compile a GGUF image into the cache-line-native P64 container (verbatim layout).
///
/// Every GGUF tensor is retained. Known inference tensors receive a semantic
/// role/layer; unknown tensors receive [`P64_ROLE_UNKNOWN`] but retain their
/// source name hash and source byte offset. Each tensor starts on a hardware
/// page boundary and has an in-band CRC-32C. The metadata, tokenizer and 10D
/// manifold table share a separate CRC, validated before any blob is exposed.
pub fn compile_gguf_to_p64(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_with_layout(input, page_log2, P64ConvertLayout::Verbatim)
}

/// Like [`compile_gguf_to_p64`] but selects a conversion-time layout policy.
pub fn compile_gguf_to_p64_with_layout(
    input: &[u8],
    page_log2: u16,
    layout: P64ConvertLayout,
) -> Result<Vec<u8>, String> {
    let index = GgufTensorIndex::from_gguf(input);
    if index.tensor_data_start == 0 || index.entries.is_empty() {
        return Err("p64: GGUF parse yielded no tensors".to_string());
    }
    let page_log2 = if page_log2 == 0 {
        P64_DEFAULT_PAGE_LOG2
    } else {
        page_log2
    };
    if !(8..=30).contains(&page_log2) {
        return Err(format!("p64: page_log2 {page_log2} out of range"));
    }
    let page = 1usize << page_log2;
    let tensor_data_start = index.tensor_data_start as usize;

    // (role, layer, source-name hash, source tensor info)
    let mut planned: Vec<(u16, u16, u64, GgufTensorInfo)> = Vec::with_capacity(index.entries.len());
    let mut push_known = |role: u16, layer: u16, candidate: Option<GgufTensorInfo>| {
        if let Some(info) = candidate {
            if planned
                .iter()
                .any(|(_, _, _, existing)| existing.byte_offset == info.byte_offset)
            {
                return;
            }
            let name_hash = index
                .entries
                .iter()
                .find(|(_, source)| source.byte_offset == info.byte_offset)
                .map(|(hash, _)| *hash)
                .unwrap_or(0);
            planned.push((role, layer, name_hash, info));
        }
    };
    for layer in 0..index.hyperparams.n_layer {
        let tensors = index.get_layer_tensors(layer);
        let layer = u16::try_from(layer).map_err(|_| "p64: layer index exceeds u16")?;
        push_known(P64_ROLE_ATTN_NORM, layer, tensors.attn_norm);
        push_known(P64_ROLE_ATTN_Q, layer, tensors.attn_q);
        push_known(P64_ROLE_ATTN_K, layer, tensors.attn_k);
        push_known(P64_ROLE_ATTN_V, layer, tensors.attn_v);
        push_known(P64_ROLE_ATTN_OUTPUT, layer, tensors.attn_output);
        push_known(P64_ROLE_FFN_NORM, layer, tensors.ffn_norm);
        push_known(P64_ROLE_FFN_GATE, layer, tensors.ffn_gate);
        push_known(P64_ROLE_FFN_UP, layer, tensors.ffn_up);
        push_known(P64_ROLE_FFN_DOWN, layer, tensors.ffn_down);
    }
    push_known(
        P64_ROLE_TOKEN_EMBD,
        P64_LAYER_GLOBAL,
        index.token_embd_info().copied(),
    );
    push_known(
        P64_ROLE_OUTPUT,
        P64_LAYER_GLOBAL,
        index.output_weight_info().copied(),
    );
    push_known(
        P64_ROLE_OUTPUT_NORM,
        P64_LAYER_GLOBAL,
        index.output_norm_info().copied(),
    );
    for (name_hash, info) in &index.entries {
        if !planned
            .iter()
            .any(|(_, _, _, existing)| existing.byte_offset == info.byte_offset)
        {
            planned.push((P64_ROLE_UNKNOWN, P64_LAYER_GLOBAL, *name_hash, *info));
        }
    }
    planned.sort_by_key(|(_, _, _, info)| info.byte_offset);

    let mut string_table = vec![0u8];
    let mut name_offsets = Vec::with_capacity(planned.len());
    for (role, layer, name_hash, _) in &planned {
        let name_offset =
            u32::try_from(string_table.len()).map_err(|_| "p64: string table exceeds 4 GiB")?;
        name_offsets.push(name_offset);
        let name = p64_tensor_name(*role, *layer, *name_hash);
        string_table.extend_from_slice(name.as_bytes());
        string_table.push(0);
    }
    let tokenizer = crate::gguf_sharder::GgufTokenizer::from_gguf(input).to_p64_section();
    let manifold_count = index
        .hyperparams
        .n_layer
        .checked_add(1)
        .ok_or("p64: manifold count overflow")? as usize;

    let hparams_offset = P64_WEIGHT_HEADER_BYTES;
    let tensor_table_offset = align_up(hparams_offset + 64, 64);
    let tensor_table_bytes = planned
        .len()
        .checked_mul(P64_TENSOR_ENTRY_BYTES)
        .ok_or("p64: tensor table overflow")?;
    let string_table_offset = tensor_table_offset + tensor_table_bytes;
    let manifold_table_offset = align_up(string_table_offset + string_table.len(), 64);
    let manifold_bytes = manifold_count
        .checked_mul(P64_MANIFOLD_ENTRY_BYTES)
        .ok_or("p64: manifold table overflow")?;
    let tokenizer_offset = align_up(manifold_table_offset + manifold_bytes, 64);
    let checksum_offset = align_up(tokenizer_offset + tokenizer.len(), 64);
    let checksum_bytes = planned
        .len()
        .checked_add(1)
        .and_then(|count| count.checked_mul(4))
        .ok_or("p64: checksum table overflow")?;
    let blob_region_offset = align_up(checksum_offset + checksum_bytes, page);

    let mut entries = Vec::with_capacity(planned.len());
    // Parallel to `entries`: true when this blob is an f16 expand (not a source copy).
    let mut expand_f16: Vec<bool> = Vec::with_capacity(planned.len());
    let mut cursor = blob_region_offset;
    for (position, (role, layer, name_hash, info)) in planned.iter().enumerate() {
        let source_blob_size = crate::ggml_quants::tensor_byte_len(info)
            .ok_or_else(|| format!("p64: unsupported GGML type {}", info.ggml_type))?;
        let source_start = tensor_data_start
            .checked_add(info.byte_offset as usize)
            .ok_or("p64: source tensor offset overflow")?;
        let source_end = source_start
            .checked_add(source_blob_size)
            .ok_or("p64: source tensor length overflow")?;
        if source_end > input.len() {
            return Err(format!("p64: source tensor {position} is out of bounds"));
        }

        let do_f16 = matches!(layout, P64ConvertLayout::F16Expand)
            && p64_role_is_weight_matrix(*role)
            && info.n_dims >= 2
            && info.ggml_type != crate::ggml_quants::GGML_TYPE_F16
            && info.ggml_type != crate::ggml_quants::GGML_TYPE_F32;
        let (out_dtype, blob_size) = if do_f16 {
            let n0 = info.dims[0] as usize;
            let n1 = info.dims[1] as usize;
            let elems = n0
                .checked_mul(n1)
                .ok_or("p64: f16 expand element count overflow")?;
            let bytes = elems
                .checked_mul(2)
                .ok_or("p64: f16 expand byte count overflow")?;
            (crate::ggml_quants::GGML_TYPE_F16 as u16, bytes)
        } else {
            (
                u16::try_from(info.ggml_type).map_err(|_| "p64: GGML type exceeds u16")?,
                source_blob_size,
            )
        };

        cursor = align_up(cursor, page);
        let mut dimensions = [0u32; 4];
        for (target, source) in dimensions.iter_mut().zip(info.dims) {
            *target = u32::try_from(source).map_err(|_| "p64: tensor dimension exceeds u32")?;
        }
        let manifold_idx = if *layer == P64_LAYER_GLOBAL {
            index.hyperparams.n_layer
        } else {
            *layer as u32
        };
        entries.push(P64TensorEntry {
            name_offset: name_offsets[position],
            role_id: *role,
            dtype: out_dtype,
            manifold_idx,
            rank: info.n_dims,
            dimensions,
            blob_offset: u32::try_from(cursor).map_err(|_| "p64: container exceeds 4 GiB")?,
            blob_size: u32::try_from(blob_size).map_err(|_| "p64: tensor exceeds 4 GiB")?,
            source_offset: info.byte_offset,
            source_name_hash: *name_hash,
            reserved: [0; 8],
        });
        expand_f16.push(do_f16);
        cursor = cursor
            .checked_add(blob_size)
            .ok_or("p64: container size overflow")?;
    }
    let total_size = align_up(cursor, 64);
    if total_size > u32::MAX as usize {
        return Err("p64: 32-bit relative-offset container exceeds 4 GiB".to_string());
    }

    let header = P64WeightHeader {
        magic: P64_MAGIC,
        version: P64_VERSION,
        flags: P64_FLAG_LITTLE_ENDIAN,
        role_table_offset: 0,
        tensor_table_offset: tensor_table_offset as u32,
        tokenizer_offset: tokenizer_offset as u32,
        hparams_offset: hparams_offset as u32,
        string_table_offset: string_table_offset as u32,
        checksum_offset: checksum_offset as u32,
        manifold_table_offset: manifold_table_offset as u32,
        tensor_count: entries.len() as u32,
        page_size: page as u32,
        reserved: [0; 20],
    };
    let hp = P64HParams {
        n_layer: index.hyperparams.n_layer,
        n_embd: index.hyperparams.n_embd,
        n_head: index.hyperparams.n_head,
        n_kv_head: index.hyperparams.effective_n_kv_head(),
        vocab_size: index.vocab_dim() as u32,
        rope_freq_base: index.hyperparams.effective_rope_freq_base(),
        rope_scale: index.hyperparams.effective_rope_scale(),
        reserved: [0; 36],
    };

    let mut output = vec![0u8; total_size];
    header.write_le(&mut output[..P64_WEIGHT_HEADER_BYTES]);
    hp.write_le(&mut output[hparams_offset..hparams_offset + 64]);
    for (position, entry) in entries.iter().enumerate() {
        let start = tensor_table_offset + position * P64_TENSOR_ENTRY_BYTES;
        write_tensor_entry(entry, &mut output[start..start + P64_TENSOR_ENTRY_BYTES]);
    }
    output[string_table_offset..string_table_offset + string_table.len()]
        .copy_from_slice(&string_table);
    for layer in 0..manifold_count {
        let coordinate = crate::modalities::manifold::ManifoldCoordinate10D::from_sequential_layer(
            layer.min(index.hyperparams.n_layer as usize) as u32,
            index.hyperparams.n_layer.max(1),
        );
        let start = manifold_table_offset + layer * P64_MANIFOLD_ENTRY_BYTES;
        write_manifold_coordinate(
            &coordinate,
            &mut output[start..start + P64_MANIFOLD_ENTRY_BYTES],
        );
    }
    output[tokenizer_offset..tokenizer_offset + tokenizer.len()].copy_from_slice(&tokenizer);

    for (position, entry) in entries.iter().enumerate() {
        let target_start = entry.blob_offset as usize;
        let target_end = target_start + entry.blob_size as usize;
        if expand_f16[position] {
            let info = &planned[position].3;
            let source_blob_size = crate::ggml_quants::tensor_byte_len(info)
                .ok_or("p64: f16 expand missing source size")?;
            let source_start = tensor_data_start + entry.source_offset as usize;
            let source_end = source_start + source_blob_size;
            let raw = &input[source_start..source_end];
            expand_tensor_to_f16_blob(raw, info, &mut output[target_start..target_end])?;
        } else {
            let source_start = tensor_data_start + entry.source_offset as usize;
            let source_end = source_start + entry.blob_size as usize;
            output[target_start..target_end].copy_from_slice(&input[source_start..source_end]);
        }
        let crc = crc32c(&output[target_start..target_end]);
        let crc_start = checksum_offset + 4 + position * 4;
        output[crc_start..crc_start + 4].copy_from_slice(&crc.to_le_bytes());
    }
    let metadata_crc = crc32c(&output[..checksum_offset]);
    output[checksum_offset..checksum_offset + 4].copy_from_slice(&metadata_crc.to_le_bytes());
    Ok(output)
}

/// Roles that are 2-D weight matrices (eligible for f16 expand). Norms stay source dtype.
#[inline]
fn p64_role_is_weight_matrix(role: u16) -> bool {
    matches!(
        role,
        P64_ROLE_ATTN_K
            | P64_ROLE_ATTN_V
            | P64_ROLE_ATTN_Q
            | P64_ROLE_ATTN_OUTPUT
            | P64_ROLE_FFN_GATE
            | P64_ROLE_FFN_UP
            | P64_ROLE_FFN_DOWN
            | P64_ROLE_TOKEN_EMBD
            | P64_ROLE_OUTPUT
    )
}

/// Dequantize a full 2-D GGUF tensor into a row-major f16 blob (`out` length = n0*n1*2).
fn expand_tensor_to_f16_blob(
    raw: &[u8],
    info: &GgufTensorInfo,
    out: &mut [u8],
) -> Result<(), String> {
    let n0 = info.dims[0] as usize; // cols (in)
    let n1 = info.dims[1] as usize; // rows (out)
    let need = n0
        .checked_mul(n1)
        .and_then(|e| e.checked_mul(2))
        .ok_or("p64: f16 expand size overflow")?;
    if out.len() < need {
        return Err("p64: f16 expand output buffer too small".into());
    }
    let mut row_f32 = vec![0f32; n0];
    for r in 0..n1 {
        crate::ggml_quants::dequant_matrix_row_into(raw, info, r, &mut row_f32)
            .map_err(|e| format!("p64: f16 expand dequant row {r}: {e:?}"))?;
        let row_off = r * n0 * 2;
        for (c, &v) in row_f32.iter().enumerate() {
            let bits = half::f16::from_f32(v).to_le_bytes();
            let o = row_off + c * 2;
            out[o] = bits[0];
            out[o + 1] = bits[1];
        }
    }
    Ok(())
}

/// Compile a flat GGUF byte image into a P64 LLM-weight container.
/// `page_log2 == 0` selects the default (16 KB). Returns the little-endian container bytes.
#[allow(dead_code)]
fn compile_gguf_to_p64_legacy(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    let idx = crate::gguf_sharder::GgufTensorIndex::from_gguf(input);
    if idx.tensor_data_start == 0 && idx.hyperparams.n_layer == 0 {
        return Err("GGUF parse failed or yielded no tensor metadata".to_string());
    }
    let page_log2 = if page_log2 == 0 {
        12 // 4096 default
    } else {
        page_log2
    };
    if page_log2 < 8 || page_log2 > 30 {
        return Err(format!("page_log2 {page_log2} out of range"));
    }
    let page = 1usize << page_log2;
    let n_layer = idx.hyperparams.n_layer;
    let tds = idx.tensor_data_start as usize;

    let mut planned: Vec<(u16, u16, crate::gguf_sharder::GgufTensorInfo)> = Vec::new();
    let mut push = |role_id: u16, layer: u16, t: Option<crate::gguf_sharder::GgufTensorInfo>| {
        if let Some(info) = t {
            planned.push((role_id, layer, info));
        }
    };
    for layer in 0..n_layer {
        let t = idx.get_layer_tensors(layer);
        let l = layer as u16;
        push(P64_ROLE_ATTN_NORM, l, t.attn_norm);
        push(P64_ROLE_ATTN_Q, l, t.attn_q);
        push(P64_ROLE_ATTN_K, l, t.attn_k);
        push(P64_ROLE_ATTN_V, l, t.attn_v);
        push(P64_ROLE_ATTN_OUTPUT, l, t.attn_output);
        push(P64_ROLE_FFN_NORM, l, t.ffn_norm);
        push(P64_ROLE_FFN_GATE, l, t.ffn_gate);
        push(P64_ROLE_FFN_UP, l, t.ffn_up);
        push(P64_ROLE_FFN_DOWN, l, t.ffn_down);
    }
    push(
        P64_ROLE_TOKEN_EMBD,
        P64_LAYER_GLOBAL,
        idx.token_embd_info().copied(),
    );
    push(
        P64_ROLE_OUTPUT_NORM,
        P64_LAYER_GLOBAL,
        idx.output_norm_info().copied(),
    );
    push(
        P64_ROLE_OUTPUT,
        P64_LAYER_GLOBAL,
        idx.output_weight_info().copied(),
    );

    // Filter missing/invalid tensors
    planned.retain(|(_, _, info)| info.dims[0] > 0);

    // Build the string table
    let mut string_table: Vec<u8> = Vec::new();
    let mut name_offsets = std::collections::HashMap::new();

    // Push a dummy null at start
    string_table.push(0u8);

    for (role, layer, info) in &planned {
        let name = if let Some(suffix) = p64_role_suffix(*role) {
            if *layer == P64_LAYER_GLOBAL {
                String::from_utf8_lossy(suffix).to_string()
            } else {
                format!("blk.{}.{}", layer, String::from_utf8_lossy(suffix))
            }
        } else {
            format!("tensor_{}", info.byte_offset)
        };

        if !name_offsets.contains_key(&name) {
            let offset = string_table.len() as u32;
            name_offsets.insert(name.clone(), offset);
            string_table.extend_from_slice(name.as_bytes());
            string_table.push(0u8);
        }
    }

    // Now extract vocabulary from GGUF and append to string table.
    let tokenizer_offset = string_table.len() as u32;
    let tok = crate::gguf_sharder::GgufTokenizer::from_gguf(input);

    let mut tok_bytes: Vec<u8> = Vec::new();
    // Serialize vocabulary sizes and strings
    tok_bytes.extend_from_slice(&(tok.vocab.len() as u32).to_le_bytes());
    for v in &tok.vocab {
        let v_bytes = v.as_bytes();
        tok_bytes.extend_from_slice(&(v_bytes.len() as u32).to_le_bytes());
        tok_bytes.extend_from_slice(v_bytes);
    }
    let tokenizer_size = tok_bytes.len() as u32;

    string_table.extend_from_slice(&tok_bytes);

    // Padding string table to 64 bytes
    while string_table.len() % 64 != 0 {
        string_table.push(0u8);
    }

    let string_table_size = string_table.len() as u32;
    let tensor_count = planned.len() as u32;

    // Build Manifold Table
    let mut manifold_table: Vec<u8> = Vec::new();
    let total_layers = idx.hyperparams.n_layer;
    for l in 0..total_layers {
        let coord = crate::modalities::manifold::ManifoldCoordinate10D::from_sequential_layer(
            l,
            total_layers,
        );
        // We do unsafe cast because bytemuck might not be derived for ManifoldCoordinate10D yet. Wait, we can just write the f32s.
        manifold_table.extend_from_slice(&coord.scale.to_le_bytes());
        manifold_table.extend_from_slice(&coord.attention_depth.to_le_bytes());
        manifold_table.extend_from_slice(&coord.epistemic_weight.to_le_bytes());
        manifold_table.extend_from_slice(&coord.topological_spin.to_le_bytes());
        manifold_table.extend_from_slice(&coord.temporal_decay.to_le_bytes());
        manifold_table.extend_from_slice(&coord.entropy_bias.to_le_bytes());
        manifold_table.extend_from_slice(&coord.spatial_phase.to_le_bytes());
        manifold_table.extend_from_slice(&coord.recurrence_frequency.to_le_bytes());
        manifold_table.extend_from_slice(&coord.density_threshold.to_le_bytes());
        manifold_table.extend_from_slice(&coord.manifold_curvature.to_le_bytes());
    }
    // Global layer
    let global_coord =
        crate::modalities::manifold::ManifoldCoordinate10D::from_sequential_layer(0, 1);
    manifold_table.extend_from_slice(&global_coord.scale.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.attention_depth.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.epistemic_weight.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.topological_spin.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.temporal_decay.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.entropy_bias.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.spatial_phase.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.recurrence_frequency.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.density_threshold.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.manifold_curvature.to_le_bytes());
    let manifold_table_size = manifold_table.len() as u32;

    // Layout
    let hparams_offset = 64;
    let entries_offset = 128;
    let string_table_offset = entries_offset + (tensor_count * 64) as u32;
    let manifold_table_offset = string_table_offset + string_table_size;
    let end_of_manifold_table = manifold_table_offset + manifold_table_size;
    let page_aligned_tensor_start =
        (end_of_manifold_table + (page as u32) - 1) & !((page as u32) - 1);

    let mut out = vec![0u8; page_aligned_tensor_start as usize];

    // 1. Header
    out[0..4].copy_from_slice(&P64_MAGIC);
    out[4..6].copy_from_slice(&P64_VERSION.to_le_bytes());
    out[6..8].copy_from_slice(&0u16.to_le_bytes()); // format flags

    out[8..12].copy_from_slice(&0u32.to_le_bytes()); // role_table_offset
    out[12..16].copy_from_slice(&(entries_offset as u32).to_le_bytes()); // tensor_table_offset
    out[16..20].copy_from_slice(&(tokenizer_offset as u32).to_le_bytes()); // tokenizer_offset
    out[20..24].copy_from_slice(&(hparams_offset as u32).to_le_bytes()); // hparams_offset
    out[24..28].copy_from_slice(&string_table_offset.to_le_bytes()); // string_table_offset
    out[28..32].copy_from_slice(&0u32.to_le_bytes()); // checksum_offset
    out[32..36].copy_from_slice(&manifold_table_offset.to_le_bytes()); // manifold_table_offset

    out[36..40].copy_from_slice(&tensor_count.to_le_bytes());
    out[40..44].copy_from_slice(&(page as u32).to_le_bytes());
    // reserved[0..4]: embedded tokenizer blob byte length
    out[44..48].copy_from_slice(&tokenizer_size.to_le_bytes());

    // 2. HParams
    // Just mock for now or use the fields if they are pub. The DOD rewrite doesn't require a strict HParams struct unless defined.
    // Actually we need to serialize idx.hyperparams into the 64 byte slot.
    let hparams = &idx.hyperparams;
    let h_off = hparams_offset as usize;
    out[h_off..h_off + 4].copy_from_slice(&hparams.n_layer.to_le_bytes());
    out[h_off + 4..h_off + 8].copy_from_slice(&hparams.n_embd.to_le_bytes());
    out[h_off + 8..h_off + 12].copy_from_slice(&hparams.n_head.to_le_bytes());
    out[h_off + 12..h_off + 16].copy_from_slice(&hparams.n_kv_head.to_le_bytes());
    out[h_off + 16..h_off + 20].copy_from_slice(&0u32.to_le_bytes());
    out[h_off + 20..h_off + 24].copy_from_slice(&hparams.rope_freq_base.to_le_bytes());
    out[h_off + 24..h_off + 28].copy_from_slice(&hparams.rope_scale.to_le_bytes());

    // 2b. Manifold Table
    let mt_off = manifold_table_offset as usize;
    out[mt_off..mt_off + manifold_table.len()].copy_from_slice(&manifold_table);

    // 3. Entries and Tensor Blobs
    let mut cursor_blob = page_aligned_tensor_start as usize;
    for (i, (role, layer, info)) in planned.iter().enumerate() {
        let e_off = entries_offset as usize + i * 64;

        let name = if let Some(suffix) = p64_role_suffix(*role) {
            if *layer == P64_LAYER_GLOBAL {
                String::from_utf8_lossy(suffix).to_string()
            } else {
                format!("blk.{}.{}", layer, String::from_utf8_lossy(suffix))
            }
        } else {
            format!("tensor_{}", info.byte_offset)
        };
        let n_offset = *name_offsets.get(&name).unwrap();

        // Align blob to its internal alignment requirement if needed, but P64 natively page aligns at start
        // and blobs follow one another. Wait, ggml requires 32-byte alignment usually.
        // Let's pad cursor_blob to 32 bytes
        cursor_blob = (cursor_blob + 31) & !31;

        let n_elements =
            info.dims[0] * info.dims[1].max(1) * info.dims[2].max(1) * info.dims[3].max(1);
        let byte_len = crate::ggml_quants::tensor_byte_len(info).unwrap_or(0);

        // Copy the tensor blob
        out.resize(cursor_blob + byte_len, 0);
        let src_start = tds + info.byte_offset as usize;
        if src_start + byte_len <= input.len() {
            out[cursor_blob..cursor_blob + byte_len]
                .copy_from_slice(&input[src_start..src_start + byte_len]);
        }

        // Write Entry
        out[e_off..e_off + 4].copy_from_slice(&n_offset.to_le_bytes());
        out[e_off + 4..e_off + 6].copy_from_slice(&role.to_le_bytes());
        out[e_off + 6..e_off + 8].copy_from_slice(&(info.ggml_type as u16).to_le_bytes());

        let m_idx = if *layer == P64_LAYER_GLOBAL {
            total_layers
        } else {
            *layer as u32
        };
        out[e_off + 8..e_off + 12].copy_from_slice(&m_idx.to_le_bytes()); // manifold_idx

        out[e_off + 12..e_off + 16].copy_from_slice(&info.n_dims.to_le_bytes());
        out[e_off + 16..e_off + 20].copy_from_slice(&(info.dims[0] as u32).to_le_bytes());
        out[e_off + 20..e_off + 24].copy_from_slice(&(info.dims[1] as u32).to_le_bytes());
        out[e_off + 24..e_off + 28].copy_from_slice(&(info.dims[2] as u32).to_le_bytes());
        out[e_off + 28..e_off + 32].copy_from_slice(&(info.dims[3] as u32).to_le_bytes());
        out[e_off + 32..e_off + 36].copy_from_slice(&(cursor_blob as u32).to_le_bytes());
        out[e_off + 36..e_off + 40].copy_from_slice(&(byte_len as u32).to_le_bytes());
        out[e_off + 40..e_off + 44].copy_from_slice(&(n_elements as u32).to_le_bytes());

        cursor_blob += byte_len;
    }

    // String table write
    out[string_table_offset as usize..string_table_offset as usize + string_table.len()]
        .copy_from_slice(&string_table);

    Ok(out)
}

/// Compatibility alias for the historical pre-P64 API name.
pub fn compile_gguf_to_q42(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64(input, page_log2)
}

/// Task #12 / STELLAR §A — like [`compile_gguf_to_p64`] but **ternary-packs the FFN projections**
/// (gate/up/down) during the compile, producing a **complete, runnable** P64: hyperparameters +
/// tokenizer are preserved (so the live loader boots it and builds the KV cache), while the FFN
/// tensors are BitNet-1.58b ternary blobs (`ternary::dequantize_blob` / the 2-bit GPU kernel).
/// Attention / norms / embeddings stay verbatim at their source precision. This is the loadable
/// container the live FFN-ternary dispatch path will run + measure against.
pub fn compile_gguf_to_p64_ternary_ffn(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ffn_quant_awq(input, page_log2, None, 0.0, FfnQuant::Ternary)
}

/// Compatibility alias for the historical pre-P64 API name.
pub fn compile_gguf_to_q42_ternary_ffn(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ternary_ffn(input, page_log2)
}

/// Target quantization for the FFN tensors in an AWQ P64 compile.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FfnQuant {
    /// BitNet 1.58b ternary (`GGML_TYPE_TERNARY_158`, resident 2-bit GPU path).
    Ternary,
    /// ggml Q4_0 4-bit (`GGML_TYPE_Q4_0`, the standard quantized GPU GEMM path) — AWQ's design regime.
    Q4_0,
}

/// AWQ-aware ternary FFN compile.
pub fn compile_gguf_to_p64_ternary_ffn_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ffn_quant_awq(input, page_log2, awq_scales, alpha, FfnQuant::Ternary)
}

/// Compatibility alias for the historical pre-P64 API name.
pub fn compile_gguf_to_q42_ternary_ffn_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ternary_ffn_awq(input, page_log2, awq_scales, alpha)
}

/// AWQ-aware **Q4_0** FFN compile (Path A) — FFN packed to 4-bit Q4_0 (AWQ's design regime); all else
/// verbatim from the source GGUF.
pub fn compile_gguf_to_p64_q4_ffn_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ffn_quant_awq(input, page_log2, awq_scales, alpha, FfnQuant::Q4_0)
}

/// Compatibility alias for the historical pre-P64 API name.
pub fn compile_gguf_to_q42_q4_ffn_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_q4_ffn_awq(input, page_log2, awq_scales, alpha)
}

/// AWQ-aware FFN-quantized P64 compile. `quant` selects the FFN target (ternary or Q4_0). When
/// `awq_scales` is `Some` (per-layer per-input-channel salience from [`crate::llm_awq::snapshot`]) the
/// gate/up input channel `i` is scaled by `s_i^alpha` before packing and `ffn_norm` is divided by
/// `s_i^alpha` — mathematically exact in f32 (`(X·norm/s^a)·(W·s^a)=(X·norm)·W`) — moving salient
/// channels into a range the quant grid represents better. `awq_scales = None` / `alpha == 0.0`
/// reproduces the plain (un-calibrated) compile. The down projection is left un-scaled (no clean fold
/// site — a v2 item). Everything outside the FFN passes through verbatim from the source GGUF.
pub fn compile_gguf_to_p64_ffn_quant_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
    quant: FfnQuant,
) -> Result<Vec<u8>, String> {
    use crate::ggml_quants::GGML_TYPE_Q4_0;
    use crate::llm_kernel_parity::{q4_0_bytes, quantize_q4_0_from_f32};
    use crate::ternary::{ternary_blob, ternary_blob_len, GGML_TYPE_TERNARY_158};

    let base = compile_gguf_to_p64(input, page_log2)?;
    let base_index = P64TensorIndex::from_p64(&base)?;
    let mut header = base_index.header;
    let hparams = base_index.hparams;
    let mut entries = base_index.entries;
    let string_region =
        base[header.string_table_offset as usize..header.manifold_table_offset as usize].to_vec();
    let manifold_region =
        base[header.manifold_table_offset as usize..header.tokenizer_offset as usize].to_vec();
    let tokenizer_region =
        base[header.tokenizer_offset as usize..header.checksum_offset as usize].to_vec();
    drop(base);

    let page = header.page_size as usize;
    let checksum_start = header.checksum_offset as usize;
    let checksum_bytes = (entries.len() + 1)
        .checked_mul(4)
        .ok_or("p64: checksum table overflow")?;
    let mut cursor = align_up(checksum_start + checksum_bytes, page);
    let is_ffn = |role: u16| {
        matches!(
            role,
            P64_ROLE_FFN_GATE | P64_ROLE_FFN_UP | P64_ROLE_FFN_DOWN
        )
    };
    let element_count = |entry: &P64TensorEntry| -> Result<usize, String> {
        entry.dimensions[..entry.rank as usize]
            .iter()
            .try_fold(1usize, |count, dimension| {
                count.checked_mul((*dimension).max(1) as usize)
            })
            .ok_or_else(|| "p64: tensor element count overflow".to_string())
    };

    for entry in &mut entries {
        let output_size = if is_ffn(entry.role_id) {
            let count = element_count(entry)?;
            match quant {
                FfnQuant::Ternary => {
                    entry.dtype = u16::try_from(GGML_TYPE_TERNARY_158)
                        .map_err(|_| "p64: ternary type exceeds u16")?;
                    ternary_blob_len(count)
                }
                FfnQuant::Q4_0 => {
                    entry.dtype =
                        u16::try_from(GGML_TYPE_Q4_0).map_err(|_| "p64: Q4_0 type exceeds u16")?;
                    q4_0_bytes(count)
                }
            }
        } else {
            entry.blob_size as usize
        };
        cursor = align_up(cursor, page);
        entry.blob_offset = u32::try_from(cursor).map_err(|_| "p64: container exceeds 4 GiB")?;
        entry.blob_size = u32::try_from(output_size).map_err(|_| "p64: tensor exceeds 4 GiB")?;
        cursor = cursor
            .checked_add(output_size)
            .ok_or("p64: output size overflow")?;
    }
    let total_size = align_up(cursor, 64);
    if total_size > u32::MAX as usize {
        return Err("p64: 32-bit relative-offset container exceeds 4 GiB".to_string());
    }
    if matches!(quant, FfnQuant::Ternary) {
        header.flags |= FORMAT_FLAG_TERNARY;
    } else {
        header.flags &= !FORMAT_FLAG_TERNARY;
    }

    let mut output = vec![0u8; total_size];
    header.write_le(&mut output[..P64_WEIGHT_HEADER_BYTES]);
    let hparams_start = header.hparams_offset as usize;
    hparams.write_le(&mut output[hparams_start..hparams_start + 64]);
    let tensor_table_start = header.tensor_table_offset as usize;
    for (position, entry) in entries.iter().enumerate() {
        let start = tensor_table_start + position * P64_TENSOR_ENTRY_BYTES;
        write_tensor_entry(entry, &mut output[start..start + P64_TENSOR_ENTRY_BYTES]);
    }
    let string_start = header.string_table_offset as usize;
    output[string_start..string_start + string_region.len()].copy_from_slice(&string_region);
    let manifold_start = header.manifold_table_offset as usize;
    output[manifold_start..manifold_start + manifold_region.len()]
        .copy_from_slice(&manifold_region);
    let tokenizer_start = header.tokenizer_offset as usize;
    output[tokenizer_start..tokenizer_start + tokenizer_region.len()]
        .copy_from_slice(&tokenizer_region);

    let source_index = GgufTensorIndex::from_gguf(input);
    let source_data_start = source_index.tensor_data_start as usize;
    let awq_enabled = awq_scales.is_some() && alpha != 0.0;
    let awq_scale = |layer: u32, channel: usize| -> f32 {
        awq_scales
            .and_then(|layers| layers.get(layer as usize))
            .and_then(|channels| channels.get(channel))
            .copied()
            .unwrap_or(1.0)
            .max(1e-6)
            .powf(alpha)
    };
    let mut scratch = Vec::<f32>::new();
    for (position, entry) in entries.iter().enumerate() {
        let source_entry = source_index
            .entries
            .iter()
            .find(|(_, info)| info.byte_offset == entry.source_offset)
            .map(|(_, info)| *info)
            .ok_or_else(|| format!("p64: source tensor {position} disappeared"))?;
        let source_size = crate::ggml_quants::tensor_byte_len(&source_entry)
            .ok_or_else(|| format!("p64: source tensor {position} type is unsupported"))?;
        let source_start = source_data_start + source_entry.byte_offset as usize;
        let source_end = source_start + source_size;
        if source_end > input.len() {
            return Err(format!("p64: source tensor {position} is out of bounds"));
        }
        let target_start = entry.blob_offset as usize;
        let target_end = target_start + entry.blob_size as usize;

        if is_ffn(entry.role_id) {
            let count = element_count(entry)?;
            scratch.resize(count, 0.0);
            crate::ggml_quants::dequantize_row_into(
                &input[source_start..source_end],
                source_entry.ggml_type,
                count,
                &mut scratch,
            )
            .map_err(|error| format!("p64: FFN dequantization failed: {error:?}"))?;
            if awq_enabled && matches!(entry.role_id, P64_ROLE_FFN_GATE | P64_ROLE_FFN_UP) {
                let n_in = entry.dimensions[0] as usize;
                let n_out = entry.dimensions[1] as usize;
                if n_in > 0 && n_in.saturating_mul(n_out) == count {
                    for output_channel in 0..n_out {
                        let row = output_channel * n_in;
                        for input_channel in 0..n_in {
                            scratch[row + input_channel] *=
                                awq_scale(entry.manifold_idx, input_channel);
                        }
                    }
                }
            }
            match quant {
                FfnQuant::Ternary => {
                    let blob = ternary_blob(&scratch);
                    if blob.len() != entry.blob_size as usize {
                        return Err("p64: ternary output length mismatch".to_string());
                    }
                    output[target_start..target_end].copy_from_slice(&blob);
                }
                FfnQuant::Q4_0 => {
                    if !quantize_q4_0_from_f32(&scratch, &mut output[target_start..target_end]) {
                        return Err(format!(
                            "p64: Q4_0 quantization failed for tensor {position}"
                        ));
                    }
                }
            }
        } else if awq_enabled && entry.role_id == P64_ROLE_FFN_NORM {
            let count = element_count(entry)?;
            match source_entry.ggml_type {
                crate::ggml_quants::GGML_TYPE_F32 if source_size >= count * 4 => {
                    for channel in 0..count {
                        let source = source_start + channel * 4;
                        let value =
                            f32::from_le_bytes(input[source..source + 4].try_into().unwrap())
                                / awq_scale(entry.manifold_idx, channel);
                        let target = target_start + channel * 4;
                        output[target..target + 4].copy_from_slice(&value.to_le_bytes());
                    }
                }
                crate::ggml_quants::GGML_TYPE_F16 if source_size >= count * 2 => {
                    for channel in 0..count {
                        let source = source_start + channel * 2;
                        let value =
                            half::f16::from_le_bytes(input[source..source + 2].try_into().unwrap())
                                .to_f32()
                                / awq_scale(entry.manifold_idx, channel);
                        let target = target_start + channel * 2;
                        output[target..target + 2]
                            .copy_from_slice(&half::f16::from_f32(value).to_le_bytes());
                    }
                }
                _ => {
                    return Err(format!(
                        "p64: AWQ cannot fold FFN norm type {} at manifold {}",
                        source_entry.ggml_type, entry.manifold_idx
                    ));
                }
            }
        } else {
            if source_size != entry.blob_size as usize {
                return Err(format!(
                    "p64: verbatim tensor {position} changed byte length"
                ));
            }
            output[target_start..target_end].copy_from_slice(&input[source_start..source_end]);
        }
        let crc = crc32c(&output[target_start..target_end]);
        let crc_start = checksum_start + 4 + position * 4;
        output[crc_start..crc_start + 4].copy_from_slice(&crc.to_le_bytes());
    }
    let metadata_crc = crc32c(&output[..checksum_start]);
    output[checksum_start..checksum_start + 4].copy_from_slice(&metadata_crc.to_le_bytes());
    P64TensorIndex::from_p64(&output)?;
    Ok(output)
}

/// Compatibility alias for the historical pre-P64 API name.
pub fn compile_gguf_to_q42_ffn_quant_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
    quant: FfnQuant,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ffn_quant_awq(input, page_log2, awq_scales, alpha, quant)
}

/// `format_flags` bit: container produced by the **raw streaming transcode** (safetensor/MLX →
/// P64) — tensors are verbatim high-fidelity blobs not yet mapped to engine GEMM roles, and the
/// GGUF hyperparameter block is absent. (Distinguishes it from a `compile_gguf_to_p64` container.)
pub const FORMAT_FLAG_RAW_TRANSCODE: u16 = 1 << 1;
/// `format_flags` bit: tensors were **ternary-quantized (BitNet 1.58b)** during transcode — each
/// blob is `[scale: f32][packed trits]` (`ggml_type = ternary::GGML_TYPE_TERNARY_158`); decode via
/// `ternary::dequantize_blob`.
pub const FORMAT_FLAG_TERNARY: u16 = 1 << 2;

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
                if o + 4 > raw.len() {
                    break;
                }
                out.push(f32::from_le_bytes([
                    raw[o],
                    raw[o + 1],
                    raw[o + 2],
                    raw[o + 3],
                ]));
            }
        }
        GGML_F16 => {
            for k in 0..count {
                let o = k * 2;
                if o + 2 > raw.len() {
                    break;
                }
                out.push(half::f16::from_le_bytes([raw[o], raw[o + 1]]).to_f32());
            }
        }
        GGML_BF16 => {
            for k in 0..count {
                let o = k * 2;
                if o + 2 > raw.len() {
                    break;
                }
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

/// Phase 6 / task #12 — **streaming, versioned transcode: safetensor (high-fidelity) → P64**.
///
/// Writes a valid P64 container to `out` forward-only (round-trips through [`P64TensorIndex::from_p64`]).
/// The full layout is computed from the safetensor *header* alone (no tensor reads), so each tensor's
/// bytes pass through **one reused scratch buffer** — the transcoder's peak working memory is ≈ the
/// largest single tensor, not the whole file. On the real path `src` is an `mmap` (demand-paged by
/// the OS), so the file is never resident in full.
///
/// Rejects low-precision (`Q4`-class) inputs — high-fidelity (`F32/F16/BF16/Q8`) only. The legacy
/// `compile_gguf_to_q42` path is untouched (GGUF support unchanged).
#[derive(Clone, Copy)]
enum SafetensorQuantization {
    Verbatim,
    AllTernary,
    FfnTernary,
}

fn transcode_safetensor_with_policy<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
    policy: SafetensorQuantization,
) -> Result<TranscodeReport, String> {
    use crate::safetensor::{
        ggml_elem_bytes, is_high_fidelity_ggml, parse_safetensor_header, safetensor_dtype_to_ggml,
    };
    use crate::ternary::{ternary_blob, ternary_blob_len, GGML_TYPE_TERNARY_158};
    let page_log2 = if page_log2 == 0 {
        P64_DEFAULT_PAGE_LOG2
    } else {
        page_log2
    };
    if !(8..=30).contains(&page_log2) {
        return Err(format!("p64: page_log2 {page_log2} out of range"));
    }
    let page = 1usize << page_log2;
    let plan = parse_safetensor_header(src)?;
    if plan.tensors.is_empty() {
        return Err("p64: Safetensors source contains no tensors".to_string());
    }

    let mut source_types = Vec::with_capacity(plan.tensors.len());
    let mut roles = Vec::with_capacity(plan.tensors.len());
    let mut n_layer = 0u32;
    for tensor in &plan.tensors {
        let ggml_type = safetensor_dtype_to_ggml(&tensor.dtype).ok_or_else(|| {
            format!(
                "p64: tensor '{}' dtype {} is not a supported high-fidelity source",
                tensor.name, tensor.dtype
            )
        })?;
        if !is_high_fidelity_ggml(ggml_type) {
            return Err(format!(
                "p64: tensor '{}' is low precision and cannot be transcoded",
                tensor.name
            ));
        }
        let element_count = tensor
            .shape
            .iter()
            .try_fold(1usize, |count, dimension| count.checked_mul(*dimension))
            .ok_or_else(|| format!("p64: tensor '{}' shape overflow", tensor.name))?;
        let expected_bytes = element_count
            .checked_mul(ggml_elem_bytes(ggml_type).ok_or("p64: unsupported element width")?)
            .ok_or_else(|| format!("p64: tensor '{}' byte-size overflow", tensor.name))?;
        if expected_bytes != tensor.byte_len() {
            return Err(format!(
                "p64: tensor '{}' declares {} bytes but shape/dtype requires {}",
                tensor.name,
                tensor.byte_len(),
                expected_bytes
            ));
        }
        let role = crate::tensor_roles::name_to_role(&tensor.name);
        if let Some(mapped) = role {
            if mapped.layer != P64_LAYER_GLOBAL {
                n_layer = n_layer.max(mapped.layer as u32 + 1);
            }
        }
        source_types.push(ggml_type);
        roles.push(role);
    }

    let ternary_for = |position: usize| match policy {
        SafetensorQuantization::Verbatim => false,
        SafetensorQuantization::AllTernary => true,
        SafetensorQuantization::FfnTernary => roles[position]
            .map(|role| crate::tensor_roles::ternary_eligible(role.role))
            .unwrap_or(false),
    };

    let mut string_table = vec![0u8];
    let mut name_offsets = Vec::with_capacity(plan.tensors.len());
    for tensor in &plan.tensors {
        name_offsets.push(
            u32::try_from(string_table.len()).map_err(|_| "p64: string table exceeds 4 GiB")?,
        );
        string_table.extend_from_slice(tensor.name.as_bytes());
        string_table.push(0);
    }

    let hparams_offset = P64_WEIGHT_HEADER_BYTES;
    let tensor_table_offset = align_up(hparams_offset + 64, 64);
    let tensor_table_bytes = plan
        .tensors
        .len()
        .checked_mul(P64_TENSOR_ENTRY_BYTES)
        .ok_or("p64: tensor table overflow")?;
    let string_table_offset = tensor_table_offset + tensor_table_bytes;
    let manifold_table_offset = align_up(string_table_offset + string_table.len(), 64);
    let manifold_count = n_layer as usize + 1;
    let manifold_bytes = manifold_count
        .checked_mul(P64_MANIFOLD_ENTRY_BYTES)
        .ok_or("p64: manifold table overflow")?;
    let tokenizer_offset = manifold_table_offset + manifold_bytes;
    let checksum_offset = align_up(tokenizer_offset, 64);
    let checksum_bytes = (plan.tensors.len() + 1)
        .checked_mul(4)
        .ok_or("p64: checksum table overflow")?;
    let blob_region_offset = align_up(checksum_offset + checksum_bytes, page);

    let mut entries = Vec::with_capacity(plan.tensors.len());
    let mut cursor = blob_region_offset;
    let mut largest_tensor_bytes = 0usize;
    let mut total_tensor_bytes = 0usize;
    let mut peak_working_bytes = 0usize;
    for (position, tensor) in plan.tensors.iter().enumerate() {
        let element_count = tensor.shape.iter().copied().product::<usize>();
        let blob_size = if ternary_for(position) {
            ternary_blob_len(element_count)
        } else {
            tensor.byte_len()
        };
        cursor = align_up(cursor, page);
        let mapped = roles[position];
        let role_id = mapped.map(|role| role.role).unwrap_or(P64_ROLE_UNKNOWN);
        let manifold_idx = mapped
            .filter(|role| role.layer != P64_LAYER_GLOBAL)
            .map(|role| role.layer as u32)
            .unwrap_or(n_layer);
        let mut dimensions = [0u32; 4];
        for (target, source) in dimensions.iter_mut().zip(tensor.shape.iter().take(4)) {
            *target = u32::try_from(*source).map_err(|_| "p64: tensor dimension exceeds u32")?;
        }
        entries.push(P64TensorEntry {
            name_offset: name_offsets[position],
            role_id,
            dtype: if ternary_for(position) {
                GGML_TYPE_TERNARY_158 as u16
            } else {
                source_types[position] as u16
            },
            manifold_idx,
            rank: tensor.shape.len().clamp(1, 4) as u32,
            dimensions,
            blob_offset: u32::try_from(cursor).map_err(|_| "p64: container exceeds 4 GiB")?,
            blob_size: u32::try_from(blob_size).map_err(|_| "p64: tensor exceeds 4 GiB")?,
            source_offset: tensor.begin as u64,
            source_name_hash: crate::q_hash(&tensor.name),
            reserved: [0; 8],
        });
        cursor = cursor
            .checked_add(blob_size)
            .ok_or("p64: container size overflow")?;
        largest_tensor_bytes = largest_tensor_bytes.max(blob_size);
        total_tensor_bytes = total_tensor_bytes
            .checked_add(blob_size)
            .ok_or("p64: tensor byte total overflow")?;
        peak_working_bytes = peak_working_bytes.max(tensor.byte_len());
    }
    if cursor > u32::MAX as usize {
        return Err("p64: 32-bit relative-offset container exceeds 4 GiB".to_string());
    }

    let mut flags = P64_FLAG_LITTLE_ENDIAN | FORMAT_FLAG_RAW_TRANSCODE;
    if !matches!(policy, SafetensorQuantization::Verbatim) {
        flags |= FORMAT_FLAG_TERNARY;
    }
    let header = P64WeightHeader {
        magic: P64_MAGIC,
        version: P64_VERSION,
        flags,
        role_table_offset: 0,
        tensor_table_offset: tensor_table_offset as u32,
        tokenizer_offset: tokenizer_offset as u32,
        hparams_offset: hparams_offset as u32,
        string_table_offset: string_table_offset as u32,
        checksum_offset: checksum_offset as u32,
        manifold_table_offset: manifold_table_offset as u32,
        tensor_count: entries.len() as u32,
        page_size: page as u32,
        reserved: [0; 20],
    };
    let hparams = P64HParams {
        n_layer,
        n_embd: 0,
        n_head: 0,
        n_kv_head: 0,
        vocab_size: 0,
        rope_freq_base: 0.0,
        rope_scale: 0.0,
        reserved: [0; 36],
    };

    let mut metadata = vec![0u8; checksum_offset + checksum_bytes];
    header.write_le(&mut metadata[..P64_WEIGHT_HEADER_BYTES]);
    hparams.write_le(&mut metadata[hparams_offset..hparams_offset + 64]);
    for (position, entry) in entries.iter().enumerate() {
        let start = tensor_table_offset + position * P64_TENSOR_ENTRY_BYTES;
        write_tensor_entry(entry, &mut metadata[start..start + P64_TENSOR_ENTRY_BYTES]);
    }
    metadata[string_table_offset..string_table_offset + string_table.len()]
        .copy_from_slice(&string_table);
    for layer in 0..manifold_count {
        let coordinate = crate::modalities::manifold::ManifoldCoordinate10D::from_sequential_layer(
            layer.min(n_layer as usize) as u32,
            n_layer.max(1),
        );
        let start = manifold_table_offset + layer * P64_MANIFOLD_ENTRY_BYTES;
        write_manifold_coordinate(
            &coordinate,
            &mut metadata[start..start + P64_MANIFOLD_ENTRY_BYTES],
        );
    }

    let mut float_scratch = Vec::new();
    for (position, tensor) in plan.tensors.iter().enumerate() {
        let source_start = plan.data_start + tensor.begin;
        let source_end = plan.data_start + tensor.end;
        let crc = if ternary_for(position) {
            let count = tensor.shape.iter().copied().product::<usize>();
            decode_safetensor_to_f32(
                &src[source_start..source_end],
                source_types[position],
                count,
                &mut float_scratch,
            );
            if float_scratch.len() != count {
                return Err(format!(
                    "p64: tensor '{}' decode was incomplete",
                    tensor.name
                ));
            }
            let blob = ternary_blob(&float_scratch);
            peak_working_bytes = peak_working_bytes.max(blob.len());
            crc32c(&blob)
        } else {
            crc32c(&src[source_start..source_end])
        };
        let start = checksum_offset + 4 + position * 4;
        metadata[start..start + 4].copy_from_slice(&crc.to_le_bytes());
    }
    let metadata_crc = crc32c(&metadata[..checksum_offset]);
    metadata[checksum_offset..checksum_offset + 4].copy_from_slice(&metadata_crc.to_le_bytes());

    out.write_all(&metadata)
        .map_err(|error| error.to_string())?;
    let zeros = [0u8; 4096];
    let mut bytes_written = metadata.len();
    for (position, tensor) in plan.tensors.iter().enumerate() {
        let target = entries[position].blob_offset as usize;
        while bytes_written < target {
            let count = (target - bytes_written).min(zeros.len());
            out.write_all(&zeros[..count])
                .map_err(|error| error.to_string())?;
            bytes_written += count;
        }
        let source_start = plan.data_start + tensor.begin;
        let source_end = plan.data_start + tensor.end;
        if ternary_for(position) {
            let count = tensor.shape.iter().copied().product::<usize>();
            decode_safetensor_to_f32(
                &src[source_start..source_end],
                source_types[position],
                count,
                &mut float_scratch,
            );
            let blob = ternary_blob(&float_scratch);
            out.write_all(&blob).map_err(|error| error.to_string())?;
            bytes_written += blob.len();
        } else {
            out.write_all(&src[source_start..source_end])
                .map_err(|error| error.to_string())?;
            bytes_written += source_end - source_start;
        }
    }

    Ok(TranscodeReport {
        n_tensors: entries.len(),
        bytes_written,
        largest_tensor_bytes,
        total_tensor_bytes,
        peak_working_bytes,
    })
}

pub fn transcode_safetensor_to_p64<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    transcode_safetensor_with_policy(src, page_log2, out, SafetensorQuantization::Verbatim)
}

/// Task #12 / STELLAR §A — **streaming transcode with BitNet 1.58b ternary compression**:
/// safetensor (high-fidelity) → P64, each tensor quantized to `{-1,0,+1}` with a per-tensor
/// absmean scale and packed at ≈ 1.6 bits/weight (`ternary` module) *during* transcode.
///
/// Same streaming discipline as [`transcode_safetensor_to_p64`] (layout from the header; one tensor
/// in flight). Each blob is `[scale: f32][packed trits]` with `ggml_type =
/// ternary::GGML_TYPE_TERNARY_158`; the container carries `FORMAT_FLAG_TERNARY`. Decode with
/// `ternary::dequantize_blob`. Round-trips through [`P64TensorIndex::from_p64`].
pub fn transcode_safetensor_to_p64_ternary<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    transcode_safetensor_with_policy(src, page_log2, out, SafetensorQuantization::AllTernary)
}

/// Task #12 / STELLAR §A — **policy transcode**: ternary the FFN projections, keep everything else
/// (attention, norms, embeddings) verbatim high-fidelity, in ONE P64. This is the real §A policy
/// (`tensor_roles::ternary_eligible`): ternarising attention/norms wrecks coherence, so only
/// `ffn_gate`/`ffn_up`/`ffn_down` are packed to 1.6 bits; the rest pass through unchanged.
///
/// Per tensor the manifest records the engine role (from the name) and `ggml_type =
/// ternary::GGML_TYPE_TERNARY_158` for ternary blobs (decode via `ternary::dequantize_blob`) or the
/// source GGML type for verbatim blobs. Round-trips through [`P64TensorIndex::from_p64`].
pub fn transcode_safetensor_to_p64_policy<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    transcode_safetensor_with_policy(src, page_log2, out, SafetensorQuantization::FfnTernary)
}

pub fn transcode_safetensor_to_p64_ffn_ternary<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    transcode_safetensor_to_p64_policy(src, page_log2, out)
}

/// GGUF tensor-name suffix for a per-layer P64 role (None for global tensors, named directly).
fn p64_role_suffix(role_id: u16) -> Option<&'static [u8]> {
    match role_id {
        P64_ROLE_ATTN_K => Some(b"attn_k.weight"),
        P64_ROLE_ATTN_V => Some(b"attn_v.weight"),
        P64_ROLE_ATTN_Q => Some(b"attn_q.weight"),
        P64_ROLE_ATTN_OUTPUT => Some(b"attn_output.weight"),
        P64_ROLE_FFN_GATE => Some(b"ffn_gate.weight"),
        P64_ROLE_FFN_UP => Some(b"ffn_up.weight"),
        P64_ROLE_FFN_DOWN => Some(b"ffn_down.weight"),
        P64_ROLE_ATTN_NORM => Some(b"attn_norm.weight"),
        P64_ROLE_FFN_NORM => Some(b"ffn_norm.weight"),
        _ => None,
    }
}

/// Runtime reader: parses a P64 container's header + manifest in microseconds. Tensor blobs
/// stay in the caller's byte slice (zero-copy); only the small manifest is materialized. The
/// `role`/`layer`/`blob_offset` fields map directly to the resident WebGPU weight arenas.
#[derive(Clone)]
pub struct P64TensorIndex {
    pub header: P64WeightHeader,
    pub hparams: P64HParams,
    pub entries: Vec<P64TensorEntry>,
}

pub type Q42TensorIndex = P64TensorIndex;

/// How thoroughly [`P64TensorIndex::from_p64`] verifies integrity.
///
/// Full tensor CRCs over a 360M–3B container dominate activate latency (toolkit probe:
/// ~3 s after table CRC optim, previously ~42 s table-less). Convert and audit use
/// [`IntegrityMode::Full`]; hot activate may use [`IntegrityMode::Metadata`] after a
/// trusted convert wrote the container on this machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum IntegrityMode {
    /// Metadata CRC + every tensor blob CRC (safest; slow on large models).
    Full,
    /// Metadata CRC only; still bounds-checks tensor ranges (**default activate**).
    #[default]
    Metadata,
    /// Structure checks only — no CRC. Prefer only for tests / trusted mmap.
    Structure,
}

impl IntegrityMode {
    /// `QUALIA_P64_INTEGRITY=full|metadata|structure`.
    ///
    /// **Default is [`Metadata`]** (bounds + metadata CRC only): convert already sealed
    /// per-tensor CRCs, and full re-scan dominated activate (~2.4 s → ~9 ms on SmolLM2).
    /// Use `full` for audit / untrusted download; `structure` for tests only.
    pub fn from_env() -> Self {
        match std::env::var("QUALIA_P64_INTEGRITY")
            .ok()
            .as_deref()
            .map(str::trim)
            .map(|s| s.to_ascii_lowercase())
            .as_deref()
        {
            Some("full") | Some("strict") | Some("all") => Self::Full,
            Some("structure") | Some("struct") | Some("skip") | Some("none") => Self::Structure,
            Some("metadata") | Some("meta") | Some("fast") | None => Self::Metadata,
            Some(_) => Self::Metadata,
        }
    }
}

/// Recommend convert layout from source size (bytes on disk) and a VRAM headroom budget.
///
/// Heuristic: F16Expand expands quant matrices toward ~2 B/param. Treat `source_bytes * 4` as
/// a conservative upper bound for Q4→f16; if that stays under `vram_budget_bytes * 0.55`
/// (leave room for KV + activations on a 12 GB class card), pick F16Expand — the GPU
/// `unpack2x16float` path is the remarkable decode lever for small/mid models.
pub fn recommend_convert_layout(source_bytes: u64, vram_budget_bytes: u64) -> P64ConvertLayout {
    let est_f16 = source_bytes.saturating_mul(4);
    let room = (vram_budget_bytes as f64 * 0.55) as u64;
    if est_f16 > 0 && est_f16 < room && est_f16 < (4u64 << 30) {
        // Also respect the 4 GiB p64 u32-offset container hard cap.
        P64ConvertLayout::F16Expand
    } else {
        P64ConvertLayout::Verbatim
    }
}

impl P64TensorIndex {
    pub fn from_q42(data: &[u8]) -> Result<Self, String> {
        Self::from_p64(data)
    }
    pub fn from_p64(data: &[u8]) -> Result<Self, String> {
        Self::from_p64_with_integrity(data, IntegrityMode::from_env())
    }

    /// Parse a P64 container with an explicit integrity policy.
    pub fn from_p64_with_integrity(data: &[u8], integrity: IntegrityMode) -> Result<Self, String> {
        let header = P64WeightHeader::read_le(data)?;
        if header.magic != P64_MAGIC {
            return Err("p64: invalid magic".to_string());
        }
        if header.version != P64_VERSION {
            return Err(format!("p64: unsupported version {}", header.version));
        }
        if header.flags & P64_FLAG_LITTLE_ENDIAN == 0 {
            return Err("p64: non-little-endian container is unsupported".to_string());
        }
        let page = header.page_size as usize;
        if page < 256 || !page.is_power_of_two() {
            return Err("p64: invalid page size".to_string());
        }
        let hparams_start = header.hparams_offset as usize;
        let hparams_end = hparams_start
            .checked_add(64)
            .ok_or("p64: hyperparameter offset overflow")?;
        if hparams_end > data.len() {
            return Err("p64: hyperparameters out of bounds".to_string());
        }
        let hparams = P64HParams::read_le(&data[hparams_start..hparams_end])?;

        let tensor_count = header.tensor_count as usize;
        let tensor_table_start = header.tensor_table_offset as usize;
        let tensor_table_end = tensor_table_start
            .checked_add(
                tensor_count
                    .checked_mul(P64_TENSOR_ENTRY_BYTES)
                    .ok_or("p64: tensor table overflow")?,
            )
            .ok_or("p64: tensor table overflow")?;
        let string_table_start = header.string_table_offset as usize;
        let manifold_table_start = header.manifold_table_offset as usize;
        let manifold_count = hparams
            .n_layer
            .checked_add(1)
            .ok_or("p64: manifold count overflow")? as usize;
        let manifold_table_end = manifold_table_start
            .checked_add(
                manifold_count
                    .checked_mul(P64_MANIFOLD_ENTRY_BYTES)
                    .ok_or("p64: manifold table overflow")?,
            )
            .ok_or("p64: manifold table overflow")?;
        let tokenizer_start = header.tokenizer_offset as usize;
        let checksum_start = header.checksum_offset as usize;
        let checksum_end = checksum_start
            .checked_add(
                tensor_count
                    .checked_add(1)
                    .and_then(|count| count.checked_mul(4))
                    .ok_or("p64: checksum table overflow")?,
            )
            .ok_or("p64: checksum table overflow")?;
        if tensor_table_start < hparams_end
            || tensor_table_end > string_table_start
            || string_table_start > manifold_table_start
            || manifold_table_end > tokenizer_start
            || tokenizer_start > checksum_start
            || checksum_end > data.len()
        {
            return Err("p64: metadata sections overlap or are out of bounds".to_string());
        }
        if manifold_table_start % 64 != 0 {
            return Err("p64: manifold table is not cache-line aligned".to_string());
        }
        if !matches!(integrity, IntegrityMode::Structure) {
            let stored_metadata_crc =
                u32::from_le_bytes(data[checksum_start..checksum_start + 4].try_into().unwrap());
            if crc32c(&data[..checksum_start]) != stored_metadata_crc {
                return Err("p64: metadata CRC-32C mismatch".to_string());
            }
        }

        let mut entries = Vec::with_capacity(tensor_count);
        let mut cursor = header.tensor_table_offset as usize;
        let blob_floor = align_up(checksum_end, page);
        let mut previous_blob_end = blob_floor;
        let verify_tensor_crc = matches!(integrity, IntegrityMode::Full);
        for tensor_index in 0..tensor_count {
            if cursor + P64_TENSOR_ENTRY_BYTES > data.len() {
                return Err("p64: truncated tensor table".to_string());
            }
            let bytes = &data[cursor..cursor + P64_TENSOR_ENTRY_BYTES];
            let eu32 = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
            let eu16 = |o: usize| u16::from_le_bytes(bytes[o..o + 2].try_into().unwrap());
            let eu64 = |o: usize| u64::from_le_bytes(bytes[o..o + 8].try_into().unwrap());
            let entry = P64TensorEntry {
                name_offset: eu32(0),
                role_id: eu16(4),
                dtype: eu16(6),
                manifold_idx: eu32(8),
                rank: eu32(12),
                dimensions: [eu32(16), eu32(20), eu32(24), eu32(28)],
                blob_offset: eu32(32),
                blob_size: eu32(36),
                source_offset: eu64(40),
                source_name_hash: eu64(48),
                reserved: [0; 8],
            };
            if !(1..=4).contains(&entry.rank) {
                return Err(format!("p64: tensor {tensor_index} has invalid rank"));
            }
            if entry.manifold_idx as usize >= manifold_count {
                return Err(format!(
                    "p64: tensor {tensor_index} has invalid manifold index"
                ));
            }
            let name_start = string_table_start
                .checked_add(entry.name_offset as usize)
                .ok_or("p64: tensor name offset overflow")?;
            if name_start >= manifold_table_start
                || !data[name_start..manifold_table_start].contains(&0)
            {
                return Err(format!("p64: tensor {tensor_index} has invalid name"));
            }
            let blob_start = entry.blob_offset as usize;
            let blob_end = blob_start
                .checked_add(entry.blob_size as usize)
                .ok_or("p64: tensor blob overflow")?;
            if blob_start % page != 0 || blob_start < previous_blob_end || blob_end > data.len() {
                return Err(format!(
                    "p64: tensor {tensor_index} is unaligned, overlapping, or out of bounds"
                ));
            }
            if verify_tensor_crc {
                let crc_start = checksum_start + 4 + tensor_index * 4;
                let stored_crc =
                    u32::from_le_bytes(data[crc_start..crc_start + 4].try_into().unwrap());
                if crc32c(&data[blob_start..blob_end]) != stored_crc {
                    return Err(format!("p64: tensor {tensor_index} CRC-32C mismatch"));
                }
            }
            previous_blob_end = blob_end;
            entries.push(entry);
            cursor += P64_TENSOR_ENTRY_BYTES;
        }
        Ok(Self {
            header,
            hparams,
            entries,
        })
    }

    pub fn hyperparams(&self) -> GgufHyperparams {
        GgufHyperparams {
            n_layer: self.hparams.n_layer,
            n_embd: self.hparams.n_embd,
            n_head: self.hparams.n_head,
            n_kv_head: self.hparams.n_kv_head,
            rope_freq_base: self.hparams.rope_freq_base,
            rope_scale: self.hparams.rope_scale,
        }
    }

    pub fn blob<'a>(&self, data: &'a [u8], entry: &P64TensorEntry) -> &'a [u8] {
        let start = entry.blob_offset as usize;
        &data[start..start + entry.blob_size as usize]
    }

    pub fn tokenizer_bytes<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        let start = self.header.tokenizer_offset as usize;
        let end = self.header.checksum_offset as usize;
        if start <= data.len() && end <= data.len() && start <= end {
            &data[start..end]
        } else {
            &[]
        }
    }

    pub fn manifold_coordinate(
        &self,
        data: &[u8],
        index: u32,
    ) -> Result<crate::modalities::manifold::ManifoldCoordinate10D, String> {
        if index > self.hparams.n_layer {
            return Err("p64: manifold index out of bounds".to_string());
        }
        let start = (self.header.manifold_table_offset as usize)
            .checked_add(index as usize * P64_MANIFOLD_ENTRY_BYTES)
            .ok_or("p64: manifold offset overflow")?;
        let end = start + P64_MANIFOLD_ENTRY_BYTES;
        if end > data.len() {
            return Err("p64: manifold coordinate out of bounds".to_string());
        }
        crate::modalities::manifold::ManifoldCoordinate10D::from_p64_bytes(&data[start..end])
    }

    /// Build a GGUF-equivalent index for the existing inference hot path.
    pub fn to_gguf_index(&self) -> GgufTensorIndex {
        let mut named: Vec<(Vec<u8>, GgufTensorInfo)> = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            let name = if entry.manifold_idx == self.hparams.n_layer {
                match entry.role_id {
                    P64_ROLE_TOKEN_EMBD => b"token_embd.weight".to_vec(),
                    P64_ROLE_OUTPUT => b"output.weight".to_vec(),
                    P64_ROLE_OUTPUT_NORM => b"output_norm.weight".to_vec(),
                    _ => continue,
                }
            } else if let Some(suffix) = p64_role_suffix(entry.role_id) {
                let mut buffer = [0u8; 96];
                let length = crate::gguf_sharder::write_blk_tensor_name(
                    entry.manifold_idx,
                    suffix,
                    &mut buffer,
                );
                buffer[..length].to_vec()
            } else {
                continue;
            };
            named.push((
                name,
                GgufTensorInfo {
                    dims: entry.dimensions.map(u64::from),
                    n_dims: entry.rank,
                    ggml_type: entry.dtype as u32,
                    byte_offset: entry.blob_offset as u64,
                },
            ));
        }
        let references: Vec<(&[u8], GgufTensorInfo)> = named
            .iter()
            .map(|(name, info)| (name.as_slice(), *info))
            .collect();
        GgufTensorIndex::from_components(&references, self.hyperparams(), 0)
    }

    /// Prove that a parsed P64 contains every GGUF tensor with identical shape,
    /// type and bytes. This is a cold-path validation routine and intentionally
    /// performs a complete byte comparison in addition to the stored CRCs.
    pub fn validate_against_gguf(
        &self,
        p64_data: &[u8],
        gguf_data: &[u8],
    ) -> Result<P64RoundTripReport, String> {
        let source = GgufTensorIndex::from_gguf(gguf_data);
        if source.entries.len() != self.entries.len() {
            return Err(format!(
                "p64: tensor count differs (GGUF {}, P64 {})",
                source.entries.len(),
                self.entries.len()
            ));
        }
        let source_data_start = source.tensor_data_start as usize;
        let mut tensor_bytes = 0u64;
        for (position, entry) in self.entries.iter().enumerate() {
            let (source_name_hash, source_info) = source
                .entries
                .iter()
                .find(|(_, info)| info.byte_offset == entry.source_offset)
                .ok_or_else(|| format!("p64: tensor {position} source offset is missing"))?;
            if *source_name_hash != entry.source_name_hash
                || source_info.ggml_type != entry.dtype as u32
                || source_info.n_dims != entry.rank
                || source_info.dims != entry.dimensions.map(u64::from)
            {
                return Err(format!("p64: tensor {position} metadata differs from GGUF"));
            }
            let source_len = crate::ggml_quants::tensor_byte_len(source_info)
                .ok_or_else(|| format!("p64: tensor {position} has unsupported source type"))?;
            if source_len != entry.blob_size as usize {
                return Err(format!("p64: tensor {position} byte length differs"));
            }
            let source_start = source_data_start + source_info.byte_offset as usize;
            let source_end = source_start + source_len;
            if source_end > gguf_data.len()
                || self.blob(p64_data, entry) != &gguf_data[source_start..source_end]
            {
                return Err(format!("p64: tensor {position} bytes differ from GGUF"));
            }
            tensor_bytes += source_len as u64;
        }
        Ok(P64RoundTripReport {
            tensor_count: self.entries.len(),
            tensor_bytes,
            manifold_count: self.hparams.n_layer as usize + 1,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct P64RoundTripReport {
    pub tensor_count: usize,
    pub tensor_bytes: u64,
    pub manifold_count: usize,
}

pub fn transcode_safetensor_to_q42_ffn_ternary<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    transcode_safetensor_to_p64_policy(src, page_log2, out)
}

// Historical tests for the pre-P64 Q42W layout are retained as migration
// documentation only. They refer to the removed 144/80-byte API.
#[cfg(all(test, any()))]
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

    /// GATE B: streaming safetensor → P64 round-trips, and peak working memory ≈ the largest
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
        let report = transcode_safetensor_to_p64(&src, 12, &mut out).unwrap();

        // peak working memory == largest tensor, and strictly less than the sum (not the whole file).
        assert_eq!(report.n_tensors, 3);
        assert_eq!(report.largest_tensor_bytes, 64);
        assert_eq!(report.total_tensor_bytes, 88);
        assert_eq!(
            report.peak_working_bytes, 64,
            "one tensor in flight = largest, not the file"
        );
        assert!(report.peak_working_bytes < report.total_tensor_bytes);

        // the emitted container is a valid P64 and parses back.
        let idx = P64TensorIndex::from_p64(&out).expect("transcoded container must round-trip");
        assert_eq!(idx.header.n_tensors, 3);
        assert_eq!(
            idx.header.format_flags & FORMAT_FLAG_RAW_TRANSCODE,
            FORMAT_FLAG_RAW_TRANSCODE
        );
        assert_eq!(idx.entries.len(), 3);

        // tensor bytes survived verbatim: each blob's first byte is its stamp; sizes match.
        let plan = crate::safetensor::parse_safetensor_header(&src).unwrap();
        for (i, (e, st)) in idx.entries.iter().zip(plan.tensors.iter()).enumerate() {
            let blob = idx.blob(&out, e);
            assert_eq!(blob.len(), st.blob_size());
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
        let err = transcode_safetensor_to_p64(&src, 12, &mut out).unwrap_err();
        assert!(
            err.contains("high-fidelity") || err.contains("rejected"),
            "got: {err}"
        );
        // and the underlying GGML gate rejects Q4_K directly.
        assert!(!crate::safetensor::is_high_fidelity_ggml(12));
    }

    /// TASK #12 (§A): ternary transcode compresses an F16 tensor to ≈1.6 bits/weight and the
    /// container round-trips + dequantizes correctly.
    #[test]
    fn transcode_ternary_compresses_and_round_trips() {
        // one F16 tensor, 100 weights of alternating ±2.0 (absmean scale = 2.0, exact reconstruction)
        let count = 100usize;
        let weights: Vec<f32> = (0..count)
            .map(|i| if i % 2 == 0 { 2.0 } else { -2.0 })
            .collect();
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
        let report = transcode_safetensor_to_p64_ternary(&src, 12, &mut out).unwrap();
        assert_eq!(report.n_tensors, 1);

        // source F16 tensor = 200 bytes; ternary blob = 4 + ceil(100/5) = 24 bytes (>8x smaller).
        assert_eq!(
            report.total_tensor_bytes,
            crate::ternary::ternary_blob_len(count)
        );
        assert!(
            report.total_tensor_bytes * 5 < data.len(),
            "ternary must be >5x smaller than F16"
        );

        // container round-trips and is flagged ternary.
        let idx = P64TensorIndex::from_p64(&out).expect("ternary container must round-trip");
        assert_eq!(
            idx.header.format_flags & FORMAT_FLAG_TERNARY,
            FORMAT_FLAG_TERNARY
        );
        assert_eq!(
            idx.entries[0].ggml_type,
            crate::ternary::GGML_TYPE_TERNARY_158
        );

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
        // P64_ROLE_* and P64_LAYER_GLOBAL are in scope via `use super::*`.
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
        let report = transcode_safetensor_to_p64_ffn_ternary(&src, 12, &mut out).unwrap();
        assert_eq!(report.n_tensors, 3);

        let idx = P64TensorIndex::from_p64(&out).expect("mixed container must round-trip");
        // entries are ordered by source offset: gate, q_proj, norm.
        let by_role = |role_id: u16| {
            idx.entries
                .iter()
                .find(|e| e.role == role)
                .expect("role present")
        };

        // FFN gate → ternary, FFN_GATE role, much smaller than its F16 source (100 bytes).
        let g = by_role(P64_ROLE_FFN_GATE);
        assert_eq!(g.ggml_type, crate::ternary::GGML_TYPE_TERNARY_158);
        assert_eq!(g.layer, 0);
        assert_eq!(g.byte_len as usize, crate::ternary::ternary_blob_len(count)); // 4 + ceil(50/5) = 14
        assert!((g.byte_len as usize) * 5 < gate.len());

        // attention q_proj → verbatim F16, ATTN_Q role.
        let a = by_role(P64_ROLE_ATTN_Q);
        assert_eq!(a.ggml_type, crate::safetensor::GGML_F16);
        assert_eq!(a.byte_len as usize, ql); // verbatim, unchanged
        assert_eq!(idx.blob(&out, a), &q[..]); // bytes preserved exactly

        // norm → verbatim, OUTPUT_NORM (global).
        let nrm = by_role(P64_ROLE_OUTPUT_NORM);
        assert_eq!(nrm.ggml_type, crate::safetensor::GGML_F16);
        assert_eq!(nrm.layer, P64_LAYER_GLOBAL);

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
    fn compile_smollm2_to_p64_layout() {
        let path = "C:/Projects/qualiaDB/docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf";
        if !std::path::Path::new(path).exists() {
            eprintln!("[p64] model not present — skipping");
            return;
        }
        let gguf = std::fs::read(path).expect("read gguf");
        let p64 = compile_gguf_to_p64(&gguf, 0).expect("compile");

        // Magic + version + default page size.
        assert_eq!(&p64[0..4], &P64_MAGIC, "magic");
        assert_eq!(le_u16(&p64, 4), P64_VERSION, "version");
        assert_eq!(le_u16(&p64, 6), 14, "default page_log2 = 16KB");
        let page = 1usize << 14;

        // Tensor count: SmolLM2-360M has 32 layers × 9 per-layer tensors + globals.
        let n_tensors = le_u32(&p64, 8) as usize;
        let n_layers = le_u32(&p64, 12);
        assert_eq!(n_layers, 32, "n_layers");
        assert!(
            n_tensors >= 32 * 9,
            "expected ≥288 tensors, got {n_tensors}"
        );

        // Hyperparameter block (v2 header) round-trips SmolLM2-360M geometry.
        assert_eq!(le_u32(&p64, 16), 960, "n_embd");
        assert_eq!(le_u32(&p64, 20), 15, "n_head");
        assert_eq!(le_u32(&p64, 24), 5, "n_kv_head");

        // Blob region + the first tensor blob both sit on a 16KB boundary.
        let manifest_offset = le_u64(&p64, 40) as usize;
        let blob_offset = le_u64(&p64, 48) as usize;
        assert_eq!(blob_offset % page, 0, "blob region 16KB-aligned");
        let first_entry = manifest_offset; // entry[0]
        let first_blob = le_u64(&p64, first_entry + 16) as usize; // blob_offset field @ entry+16
        let first_len = le_u64(&p64, first_entry + 24) as usize;
        assert_eq!(first_blob % page, 0, "first tensor blob 16KB-aligned");
        assert_eq!(first_blob, blob_offset, "first blob == blob region start");
        assert!(first_blob + first_len <= p64.len(), "first blob in-bounds");

        // Every tensor blob is 16KB-aligned and in-bounds.
        for k in 0..n_tensors {
            let e = manifest_offset + k * P64_TENSOR_ENTRY_BYTES;
            let bo = le_u64(&p64, e + 16) as usize;
            let bl = le_u64(&p64, e + 24) as usize;
            assert_eq!(bo % page, 0, "tensor {k} blob 16KB-aligned");
            assert!(bo + bl <= p64.len(), "tensor {k} in-bounds");
        }

        // Round-trip through the runtime reader.
        let idx = P64TensorIndex::from_p64(&p64).expect("from_p64");
        assert_eq!(idx.entries.len(), n_tensors, "reader entry count");
        assert_eq!(
            idx.header.blob_offset as usize, blob_offset,
            "reader blob_offset"
        );
        let hp = idx.hyperparams();
        assert_eq!(hp.n_layer, 32);
        assert_eq!(hp.n_embd, 960);
        assert_eq!(hp.n_head, 15);
        assert_eq!(hp.effective_n_kv_head(), 5);
        for (k, e) in idx.entries.iter().enumerate() {
            assert_eq!(e.blob_offset as usize % page, 0, "reader entry {k} aligned");
            assert_eq!(
                idx.blob(&p64, e).len(),
                e.byte_len as usize,
                "reader blob len {k}"
            );
        }
        // Bad magic is rejected.
        let mut bad = p64.clone();
        bad[0] = b'X';
        assert!(
            P64TensorIndex::from_p64(&bad).is_err(),
            "bad magic rejected"
        );

        // Integrity: header CRC populated; a flipped manifest byte (corrupted offset) is rejected.
        assert_ne!(le_u32(&p64, 72), 0, "header_crc populated");
        let mut tampered = p64.clone();
        tampered[manifest_offset + 16] ^= 0xFF; // first entry's blob_offset
        assert!(
            P64TensorIndex::from_p64(&tampered).is_err(),
            "manifest tamper must be caught by CRC before any bind"
        );

        eprintln!(
            "[p64] OK: {n_tensors} tensors, {n_layers} layers, blob@{blob_offset}, total {} MB; reader round-trip + hyperparams verified",
            p64.len() / (1024 * 1024)
        );
    }

    /// Proves inference-from-P64 equivalence WITHOUT a browser: the synthetic GGUF index built
    /// from the P64 manifest returns byte-identical weights + matching metadata vs the original
    /// GGUF index for every tensor. Identical weights → identical logits → identical output. The
    /// P64 carries the tokenizer in its embedded Q42T section.
    #[test]
    fn p64_synthetic_index_matches_gguf() {
        let path = "C:/Projects/qualiaDB/docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf";
        if !std::path::Path::new(path).exists() {
            eprintln!("[p64] model not present — skipping");
            return;
        }
        let gguf = std::fs::read(path).expect("read gguf");
        let p64 = compile_gguf_to_p64(&gguf, 0).expect("compile");
        let orig = GgufTensorIndex::from_gguf(&gguf);
        let q = P64TensorIndex::from_p64(&p64).expect("from_p64");
        let synth = q.to_gguf_index();

        let mut checked = 0usize;
        let mut cmp =
            |label: &str, s: Option<GgufTensorInfo>, o: Option<GgufTensorInfo>| match (s, o) {
                (Some(s), Some(o)) => {
                    assert_eq!(s.ggml_type, o.ggml_type, "{label} ggml_type");
                    assert_eq!(s.dims[0], o.dims[0], "{label} dim0");
                    assert_eq!(s.dims[1], o.dims[1], "{label} dim1");
                    let sb =
                        crate::ggml_quants::fetch_tensor_bytes(&p64, synth.tensor_data_start, &s)
                            .expect("P64 tensor bytes");
                    let ob =
                        crate::ggml_quants::fetch_tensor_bytes(&gguf, orig.tensor_data_start, &o)
                            .expect("gguf tensor bytes");
                    assert_eq!(sb, ob, "{label} weight bytes differ");
                    checked += 1;
                }
                (None, None) => {}
                _ => panic!("{label}: tensor presence mismatch (synthetic vs gguf)"),
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
        cmp(
            "token_embd",
            synth.token_embd_info().copied(),
            orig.token_embd_info().copied(),
        );
        cmp(
            "output",
            synth.output_weight_info().copied(),
            orig.output_weight_info().copied(),
        );
        cmp(
            "output_norm",
            synth.output_norm_info().copied(),
            orig.output_norm_info().copied(),
        );

        assert!(
            checked >= 32 * 9,
            "expected ≥288 tensors byte-checked, got {checked}"
        );
        eprintln!(
            "[p64] synthetic index == GGUF: {checked} tensors byte-identical + metadata match"
        );
    }

    /// Proves the v3 tokenizer section round-trips: a tokenizer rebuilt from the P64 section
    /// encodes/decodes identically to the GGUF tokenizer. With weight byte-parity (above), this
    /// guarantees P64-only inference produces the same tokens as the GGUF path.
    #[test]
    fn p64_tokenizer_roundtrip() {
        use crate::gguf_sharder::GgufTokenizer;
        let path = "C:/Projects/qualiaDB/docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf";
        if !std::path::Path::new(path).exists() {
            eprintln!("[p64] model not present — skipping");
            return;
        }
        let gguf = std::fs::read(path).expect("read gguf");
        let p64 = compile_gguf_to_p64(&gguf, 0).expect("compile");
        let q = P64TensorIndex::from_p64(&p64).expect("from_p64");

        let tok_bytes = q.tokenizer_bytes(&p64);
        assert!(!tok_bytes.is_empty(), "tokenizer section present");
        let tok_p64 = GgufTokenizer::from_p64_section(tok_bytes).expect("from_p64_section");
        let tok_gguf = GgufTokenizer::from_gguf(&gguf);

        assert_eq!(tok_p64.bos_token_id, tok_gguf.bos_token_id, "bos");
        assert_eq!(tok_p64.eos_token_id, tok_gguf.eos_token_id, "eos");
        assert_eq!(tok_p64.add_bos_token, tok_gguf.add_bos_token, "add_bos");
        assert_eq!(tok_p64.vocab.len(), tok_gguf.vocab.len(), "vocab len");
        for prompt in [
            "The capital of France is",
            "<|im_start|>user\nWhat is the capital of France?<|im_end|>\n<|im_start|>assistant\n",
        ] {
            assert_eq!(
                tok_p64.encode_prompt(prompt),
                tok_gguf.encode_prompt(prompt),
                "encode mismatch for {prompt:?}"
            );
        }
        let ids = tok_gguf.encode_prompt("The capital of France is");
        assert_eq!(
            tok_p64.decode(&ids),
            tok_gguf.decode(&ids),
            "decode mismatch"
        );
        eprintln!(
            "[p64] tokenizer round-trip: encode/decode identical to GGUF ({} vocab, section {} KB)",
            tok_p64.vocab.len(),
            tok_bytes.len() / 1024
        );
    }
}

#[cfg(test)]
mod p64_validation_tests {
    use super::*;

    #[test]
    fn p64_magic_sniff_is_exact_and_case_sensitive() {
        assert!(has_p64_magic(b"p64\0payload"));
        assert!(has_p64_magic(&P64_MAGIC));
        assert!(!has_p64_magic(b"P64\0payload"));
        assert!(!has_p64_magic(b"P64"));
        assert!(!has_p64_magic(b"p64"));
        assert!(!has_p64_magic(b"Q42\0payload"));
        assert!(!has_p64_magic(b"GGUFpayload"));
    }

    fn put_kv_u32(out: &mut Vec<u8>, key: &str, value: u32) {
        out.extend_from_slice(&(key.len() as u64).to_le_bytes());
        out.extend_from_slice(key.as_bytes());
        out.extend_from_slice(&4u32.to_le_bytes()); // GGUF_TYPE_UINT32
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn put_tensor(out: &mut Vec<u8>, name: &str, dims: &[u64], offset: u64) {
        out.extend_from_slice(&(name.len() as u64).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&(dims.len() as u32).to_le_bytes());
        for dimension in dims {
            out.extend_from_slice(&dimension.to_le_bytes());
        }
        out.extend_from_slice(&0u32.to_le_bytes()); // GGML F32
        out.extend_from_slice(&offset.to_le_bytes());
    }

    fn synthetic_gguf() -> Vec<u8> {
        let mut output = Vec::new();
        output.extend_from_slice(b"GGUF");
        output.extend_from_slice(&3u32.to_le_bytes());
        output.extend_from_slice(&2u64.to_le_bytes());
        output.extend_from_slice(&3u64.to_le_bytes());
        put_kv_u32(&mut output, "llama.block_count", 1);
        put_kv_u32(&mut output, "llama.embedding_length", 4);
        put_kv_u32(&mut output, "llama.attention.head_count", 1);
        put_tensor(&mut output, "blk.0.attn_q.weight", &[4, 4], 0);
        put_tensor(&mut output, "token_embd.weight", &[4, 2], 64);
        output.resize(align_up(output.len(), 32), 0);
        for byte in 0u8..96 {
            output.push(byte.wrapping_mul(17).wrapping_add(3));
        }
        output
    }

    fn synthetic_ffn_gguf() -> Vec<u8> {
        let mut output = Vec::new();
        output.extend_from_slice(b"GGUF");
        output.extend_from_slice(&3u32.to_le_bytes());
        output.extend_from_slice(&2u64.to_le_bytes());
        output.extend_from_slice(&3u64.to_le_bytes());
        put_kv_u32(&mut output, "llama.block_count", 1);
        put_kv_u32(&mut output, "llama.embedding_length", 32);
        put_kv_u32(&mut output, "llama.attention.head_count", 1);
        put_tensor(&mut output, "blk.0.ffn_gate.weight", &[32, 2], 0);
        put_tensor(&mut output, "token_embd.weight", &[32, 1], 256);
        output.resize(align_up(output.len(), 32), 0);
        for index in 0..64 {
            let value = if index % 2 == 0 { 1.25f32 } else { -0.75f32 };
            output.extend_from_slice(&value.to_le_bytes());
        }
        for index in 0..32 {
            output.extend_from_slice(&(index as f32 / 32.0).to_le_bytes());
        }
        output
    }

    fn synthetic_safetensor() -> Vec<u8> {
        let values = [1.0f32, -1.0, 0.5, -0.5, 2.0, -2.0, 0.25, -0.25];
        let mut ffn = Vec::new();
        let mut attention = Vec::new();
        for value in values {
            ffn.extend_from_slice(&half::f16::from_f32(value).to_le_bytes());
            attention.extend_from_slice(&half::f16::from_f32(value / 2.0).to_le_bytes());
        }
        let header = serde_json::json!({
            "model.layers.0.mlp.gate_proj.weight": {
                "dtype": "F16", "shape": [4, 2], "data_offsets": [0, ffn.len()]
            },
            "model.layers.0.self_attn.q_proj.weight": {
                "dtype": "F16", "shape": [4, 2],
                "data_offsets": [ffn.len(), ffn.len() + attention.len()]
            }
        });
        let header = serde_json::to_vec(&header).unwrap();
        let mut output = Vec::new();
        output.extend_from_slice(&(header.len() as u64).to_le_bytes());
        output.extend_from_slice(&header);
        output.extend_from_slice(&ffn);
        output.extend_from_slice(&attention);
        output
    }

    #[test]
    fn gguf_to_p64_round_trip_is_byte_exact_and_cache_aligned() {
        let gguf = synthetic_gguf();
        let p64 = compile_gguf_to_p64(&gguf, 12).expect("compile synthetic GGUF");
        let index = P64TensorIndex::from_p64(&p64).expect("validate P64");
        let report = index
            .validate_against_gguf(&p64, &gguf)
            .expect("full tensor parity");

        assert_eq!(report.tensor_count, 2);
        assert_eq!(report.tensor_bytes, 96);
        assert_eq!(report.manifold_count, 2);
        assert_eq!(index.header.manifold_table_offset as usize % 64, 0);
        for entry in &index.entries {
            assert_eq!(entry.blob_offset as usize % 4096, 0);
            assert_eq!(
                (index.header.manifold_table_offset as usize
                    + entry.manifold_idx as usize * P64_MANIFOLD_ENTRY_BYTES)
                    % 64,
                0
            );
        }
        let synthetic = index.to_gguf_index();
        assert_eq!(synthetic.hyperparams, index.hyperparams());
        assert!(synthetic.get_layer_tensors(0).attn_q.is_some());
        assert!(synthetic.token_embd_info().is_some());
    }

    #[test]
    fn p64_rejects_metadata_and_tensor_corruption() {
        let gguf = synthetic_gguf();
        let p64 = compile_gguf_to_p64(&gguf, 12).expect("compile");
        let index =
            P64TensorIndex::from_p64_with_integrity(&p64, IntegrityMode::Full).expect("baseline");

        let mut metadata_corrupt = p64.clone();
        metadata_corrupt[index.header.tensor_table_offset as usize + 12] ^= 1;
        assert!(P64TensorIndex::from_p64_with_integrity(
            &metadata_corrupt,
            IntegrityMode::Metadata
        )
        .is_err());

        let mut tensor_corrupt = p64;
        tensor_corrupt[index.entries[0].blob_offset as usize] ^= 1;
        // Tensor CRC is only checked in Full mode.
        assert!(P64TensorIndex::from_p64_with_integrity(
            &tensor_corrupt,
            IntegrityMode::Full
        )
        .is_err());
        // Metadata mode still accepts (bounds ok) — intentional fast-activate tradeoff.
        assert!(P64TensorIndex::from_p64_with_integrity(
            &tensor_corrupt,
            IntegrityMode::Metadata
        )
        .is_ok());
    }

    #[test]
    fn p64_round_trips_after_filesystem_write() {
        let gguf = synthetic_gguf();
        let p64 = compile_gguf_to_p64(&gguf, 12).expect("compile");
        let path = std::env::temp_dir().join(format!(
            "qualia-p64-roundtrip-{}-{}.p64",
            std::process::id(),
            p64.len()
        ));
        std::fs::write(&path, &p64).expect("write P64");
        let persisted = std::fs::read(&path).expect("read P64");
        let _ = std::fs::remove_file(&path);

        assert_eq!(persisted, p64, "filesystem changed P64 bytes");
        let index = P64TensorIndex::from_p64(&persisted).expect("parse persisted P64");
        index
            .validate_against_gguf(&persisted, &gguf)
            .expect("persisted tensor parity");
    }

    #[test]
    fn ffn_quantized_p64_variants_are_loadable_and_preserve_non_ffn_weights() {
        let gguf = synthetic_ffn_gguf();
        let source = GgufTensorIndex::from_gguf(&gguf);

        for (quant, expected_type, expected_size) in [
            (
                FfnQuant::Ternary,
                crate::ternary::GGML_TYPE_TERNARY_158,
                crate::ternary::ternary_blob_len(64),
            ),
            (
                FfnQuant::Q4_0,
                crate::ggml_quants::GGML_TYPE_Q4_0,
                crate::llm_kernel_parity::q4_0_bytes(64),
            ),
        ] {
            let scales = [vec![1.0f32; 32]];
            let p64 = compile_gguf_to_q42_ffn_quant_awq(&gguf, 12, Some(&scales), 0.5, quant)
                .expect("quantized P64");
            let index = P64TensorIndex::from_p64(&p64).expect("load quantized P64");
            let ffn = index
                .entries
                .iter()
                .find(|entry| entry.role_id == P64_ROLE_FFN_GATE)
                .expect("FFN gate");
            assert_eq!(ffn.dtype as u32, expected_type);
            assert_eq!(ffn.blob_size as usize, expected_size);

            let token = index
                .entries
                .iter()
                .find(|entry| entry.role_id == P64_ROLE_TOKEN_EMBD)
                .expect("token embedding");
            let original = source.token_embd_info().unwrap();
            let source_start = source.tensor_data_start as usize + original.byte_offset as usize;
            let source_len = crate::ggml_quants::tensor_byte_len(original).unwrap();
            assert_eq!(
                index.blob(&p64, token),
                &gguf[source_start..source_start + source_len]
            );
        }
    }

    #[test]
    fn safetensor_p64_variants_share_the_validated_container_contract() {
        let source = synthetic_safetensor();
        let source_plan = crate::safetensor::parse_safetensor_header(&source).unwrap();

        let mut verbatim = Vec::new();
        let report = transcode_safetensor_to_p64(&source, 12, &mut verbatim).unwrap();
        assert_eq!(report.n_tensors, 2);
        let index = P64TensorIndex::from_p64(&verbatim).expect("verbatim P64");
        for (entry, tensor) in index.entries.iter().zip(&source_plan.tensors) {
            let start = source_plan.data_start + tensor.begin;
            let end = source_plan.data_start + tensor.end;
            assert_eq!(index.blob(&verbatim, entry), &source[start..end]);
        }

        let mut all_ternary = Vec::new();
        transcode_safetensor_to_p64_ternary(&source, 12, &mut all_ternary).unwrap();
        let all_index = P64TensorIndex::from_p64(&all_ternary).expect("all-ternary P64");
        assert!(all_index
            .entries
            .iter()
            .all(|entry| entry.dtype as u32 == crate::ternary::GGML_TYPE_TERNARY_158));

        let mut policy = Vec::new();
        transcode_safetensor_to_p64_policy(&source, 12, &mut policy).unwrap();
        let policy_index = P64TensorIndex::from_p64(&policy).expect("policy P64");
        let ffn = policy_index
            .entries
            .iter()
            .find(|entry| entry.role_id == P64_ROLE_FFN_GATE)
            .unwrap();
        let attention = policy_index
            .entries
            .iter()
            .find(|entry| entry.role_id == P64_ROLE_ATTN_Q)
            .unwrap();
        assert_eq!(ffn.dtype as u32, crate::ternary::GGML_TYPE_TERNARY_158);
        assert_eq!(attention.dtype as u32, crate::safetensor::GGML_F16);
    }

    #[test]
    #[ignore = "requires the local C:\\LLM_Models SmolLM2 GGUF"]
    fn real_smollm_p64_round_trip_on_disk() {
        let source_path = "C:/LLM_Models/GGUF/lmstudio-community/smollm2-360m-instruct-q8_0.gguf";
        if !std::path::Path::new(source_path).exists() {
            eprintln!("local SmolLM2 model absent; skipping");
            return;
        }
        let gguf = std::fs::read(source_path).expect("read real GGUF");
        let p64 = compile_gguf_to_p64(&gguf, 14).expect("compile real GGUF");
        let output_path =
            std::env::temp_dir().join(format!("qualia-smollm-p64-{}.p64", std::process::id()));
        std::fs::write(&output_path, &p64).expect("persist real P64");
        drop(p64);

        let file = std::fs::File::open(&output_path).expect("reopen P64");
        let persisted = unsafe { memmap2::Mmap::map(&file).expect("mmap P64") };
        let index = P64TensorIndex::from_p64(&persisted).expect("validate persisted P64");
        let report = index
            .validate_against_gguf(&persisted, &gguf)
            .expect("real model tensor parity");
        assert!(report.tensor_count > 100);
        assert!(report.tensor_bytes > 300_000_000);
        assert_eq!(report.manifold_count, index.hparams.n_layer as usize + 1);
        drop(persisted);
        drop(file);
        let _ = std::fs::remove_file(&output_path);
        eprintln!(
            "real P64 parity: {} tensors / {} bytes / {} manifold coordinates",
            report.tensor_count, report.tensor_bytes, report.manifold_count
        );
    }
}
