//! P64 container **byte layout**: magic/version/flags, the cache-line (64 B) DOD structs
//! (`P64WeightHeader`, `P64TensorEntry`, `P64HParams`, `P64LayerScheduleEntry`), the tensor-role
//! and header-flag constants, and the little-endian (de)serialization for the header + hparams.
//! Every constant and offset here is format-critical and must not change.

use alloc::string::{String, ToString};

pub const P64_MAGIC: [u8; 4] = *b"p64\0";
/// Container format version written by the canonical compiler.
/// Keep in lock-step with `docs/manuals/standards/p64-weight-container-standard.md`.
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
pub const P64_MIN_PAGE_LOG2: u16 = 8;
pub const P64_MAX_PAGE_LOG2: u16 = 30;
pub const P64_CACHE_LINE_BYTES: usize = 64;
pub const P64_MIN_PAGE_SIZE: usize = 1usize << (P64_MIN_PAGE_LOG2 as usize);
pub const P64_4K_PAGE_SIZE: usize = 4096;
pub const P64_DEFAULT_PAGE_SIZE: usize = 1usize << (P64_DEFAULT_PAGE_LOG2 as usize);
pub const P64_WEIGHT_HEADER_BYTES: usize = 64;
pub const P64_TENSOR_ENTRY_BYTES: usize = 64;
/// Ten little-endian `f32` values plus 24 bytes of zero padding.
///
/// Keeping every coordinate in one cache line makes `manifold_idx` an exact
/// 64-byte stride and prevents neighbouring coordinates from sharing a cache
/// line or a WASM SIMD fetch.
pub const P64_MANIFOLD_ENTRY_BYTES: usize = 64;

/// Round `x` up to the next multiple of power-of-two alignment `a`.
#[inline]
pub const fn align_up(x: usize, a: usize) -> usize {
    debug_assert!(a.is_power_of_two());
    (x + (a - 1)) & !(a - 1)
}

/// Checked variant of [`align_up`] that returns `None` on overflow or non-power-of-two `a`.
#[inline]
pub const fn checked_align_up(x: usize, a: usize) -> Option<usize> {
    if !a.is_power_of_two() {
        return None;
    }
    match x.checked_add(a - 1) {
        Some(v) => Some(v & !(a - 1)),
        None => None,
    }
}

/// True iff `offset` is aligned to a 64-byte P64 cache line.
#[inline]
pub const fn is_cache_line_aligned(offset: usize) -> bool {
    offset & (P64_CACHE_LINE_BYTES - 1) == 0
}

/// True iff `offset` is aligned to a 4096-byte hardware page boundary.
#[inline]
pub const fn is_4k_page_aligned(offset: usize) -> bool {
    offset & (P64_4K_PAGE_SIZE - 1) == 0
}

/// True iff `page_size` is a valid P64 page size (`>= 256` and power-of-two) and `offset` is aligned to it.
#[inline]
pub const fn is_page_aligned(offset: usize, page_size: usize) -> bool {
    page_size >= P64_MIN_PAGE_SIZE && page_size.is_power_of_two() && (offset & (page_size - 1) == 0)
}

/// Normalize `page_log2` (`0` -> [`P64_DEFAULT_PAGE_LOG2`]) and validate it lies in `8..=30`.
#[inline]
pub const fn resolve_page_log2(page_log2: u16) -> Option<u16> {
    let resolved = if page_log2 == 0 {
        P64_DEFAULT_PAGE_LOG2
    } else {
        page_log2
    };
    if resolved >= P64_MIN_PAGE_LOG2 && resolved <= P64_MAX_PAGE_LOG2 {
        Some(resolved)
    } else {
        None
    }
}

/// Resolve the byte page size `1usize << page_log2` after normalizing `0` -> [`P64_DEFAULT_PAGE_LOG2`].
#[inline]
pub const fn resolve_page_size(page_log2: u16) -> Option<usize> {
    match resolve_page_log2(page_log2) {
        Some(shift) => Some(1usize << (shift as usize)),
        None => None,
    }
}

// ── Header flags (bits of `P64WeightHeader::flags`) ─────────────────────────
pub const P64_FLAG_LITTLE_ENDIAN: u16 = 1 << 0;
// Bits 1–2: see `FORMAT_FLAG_RAW_TRANSCODE` / `FORMAT_FLAG_TERNARY` below (aliases kept
// for historical call sites).
/// At least one 2-D weight matrix was converted to `GGML_TYPE_Q4_K_SOA` (112).
pub const P64_FLAG_Q4K_SOA: u16 = 1 << 3;
/// Tensor blob region is **layer-major** (known roles ordered by layer, then role).
/// Decode residency / CUDA slab fill SHOULD walk entries in table order.
pub const P64_FLAG_LAYER_MAJOR: u16 = 1 << 4;
/// Blobs use **layer-pack** alignment: page-align at layer boundaries only; 256 B within layer.
pub const P64_FLAG_LAYER_PACK: u16 = 1 << 5;
/// `role_table_offset` points at a layer schedule table (`P64LayerScheduleEntry` × n_layer).
pub const P64_FLAG_LAYER_SCHEDULE: u16 = 1 << 6;

/// One row of the optional layer schedule table (64 B, cache-line DOD).
/// Written when [`P64_FLAG_LAYER_SCHEDULE`] is set; offset in `role_table_offset`.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct P64LayerScheduleEntry {
    pub layer: u32,
    /// Inclusive start of first blob in this layer (file offset).
    pub blob_begin: u32,
    /// Exclusive end of last blob in this layer.
    pub blob_end: u32,
    pub tensor_count: u16,
    /// Bit i set if role_id `i` (0..14) appears in this layer.
    pub roles_mask: u16,
    pub reserved: [u8; 48],
}
impl Default for P64LayerScheduleEntry {
    fn default() -> Self {
        Self {
            layer: 0,
            blob_begin: 0,
            blob_end: 0,
            tensor_count: 0,
            roles_mask: 0,
            reserved: [0; 48],
        }
    }
}
const _: () = assert!(core::mem::size_of::<P64LayerScheduleEntry>() == 64);

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
// MoE (Mixture of Experts) roles.
pub const P64_ROLE_MOE_ROUTER: u16 = 0x20;
pub const P64_ROLE_MOE_GATE_EXPS: u16 = 0x21;
pub const P64_ROLE_MOE_UP_EXPS: u16 = 0x22;
pub const P64_ROLE_MOE_DOWN_EXPS: u16 = 0x23;
pub const P64_ROLE_MOE_SHARED_GATE: u16 = 0x24;
pub const P64_ROLE_MOE_SHARED_UP: u16 = 0x25;
pub const P64_ROLE_MOE_SHARED_DOWN: u16 = 0x26;
/// A source GGUF tensor preserved byte-for-byte but not consumed by a known
/// engine role. Its source offset and name hash remain in the entry so a
/// validator can still prove complete model preservation.
pub const P64_ROLE_UNKNOWN: u16 = 0xFFFE;
/// `layer` sentinel for non-layer (global) tensors.
pub const P64_LAYER_GLOBAL: u16 = 0xFFFF;

// Metadata bitfields are handled by the q42 layer, no longer embedded in weights.

/// Precision view flags for multi-precision `.p64` containers.
pub const P64_VIEW_FLAG_F32: u16 = 1 << 0;
pub const P64_VIEW_FLAG_F16: u16 = 1 << 1;
pub const P64_VIEW_FLAG_BF16: u16 = 1 << 2;
pub const P64_VIEW_FLAG_Q8_0: u16 = 1 << 3;
pub const P64_VIEW_FLAG_Q4_K: u16 = 1 << 4;
pub const P64_VIEW_FLAG_SOA: u16 = 1 << 5;
pub const P64_VIEW_FLAG_TERNARY_158: u16 = 1 << 6;

#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct P64WeightHeader {
    pub magic: [u8; 4], // b"p64\0"
    pub version: u16,   // 4
    pub flags: u16,     // Endianness & Feature Flags

    // 32-bit Relative Offsets (WASM-native)
    pub role_table_offset: u32,     // Maps tensors to semantic roles
    pub tensor_table_offset: u32,   // Descriptor table (shape, dtype)
    pub tokenizer_offset: u32,      // Embedded tokenizer vocabulary
    pub hparams_offset: u32,        // Hyperparameters
    pub string_table_offset: u32,   // Centralized string pool
    pub checksum_offset: u32,       // Unkeyed metadata and per-tensor CRC-32C table
    pub manifold_table_offset: u32, // Offset to 10D ManifoldCoordinate10D table

    pub tensor_count: u32, // Number of tensors
    pub page_size: u32,    // Hardware alignment (e.g., 4096)

    pub reserved: [u8; 20], // Pad exactly to 64 bytes
}

#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct P64TensorEntry {
    pub name_offset: u32,          // Relative offset to string table
    pub role_id: u16,              // Standardized enum (e.g., P64_ROLE_FFN_UP)
    pub dtype: u16,                // Primary data type (FP32, FP16, etc.)
    pub manifold_idx: u32,         // Index into the 10D Manifold table
    pub rank: u32,                 // Number of dimensions
    pub dimensions: [u32; 4],      // Shape of the tensor
    pub blob_offset: u32,          // Relative offset to primary tensor data
    pub blob_size: u32,            // Size in bytes of primary blob
    pub source_offset: u64,        // Original offset inside source container
    pub source_name_hash: u64,     // Original tensor-name hash
    pub alt_dtype: u16,            // Secondary/quantized view dtype (e.g. Q4_K / Ternary)
    pub precision_views_mask: u16, // Bitmask of available precision loading views
    pub alt_blob_offset: u32,      // Relative offset to secondary quantized blob
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
    /// Explicit head dim (`0` = derive n_embd/n_head). Occupies former reserved[0..4].
    pub head_dim: u32,
    pub head_dim_swa: u32,
    pub sliding_window: u32,
    pub shared_kv_layers: u32,
    pub logit_softcap: f32,
    pub architecture: u32,
    pub arch_flags: u32,
    pub reserved: [u8; 8], // Pad exactly to 64 bytes
}

// Layouts are exact multiples of 64 for Cache-Line DOD perfection.
const _: () = assert!(core::mem::size_of::<P64WeightHeader>() == P64_WEIGHT_HEADER_BYTES);
const _: () = assert!(core::mem::size_of::<P64TensorEntry>() == P64_TENSOR_ENTRY_BYTES);
const _: () = assert!(core::mem::size_of::<P64HParams>() == 64);

impl P64WeightHeader {
    /// Reads `le`.
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
            reserved: {
                let mut r = [0u8; 20];
                r.copy_from_slice(&data[44..64]);
                r
            },
        })
    }

    /// Writes `le`.
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
        out[44..64].copy_from_slice(&self.reserved);
    }

    /// True iff `page_size` is at least 256 bytes and a power of two.
    #[inline]
    pub const fn has_valid_page_size(&self) -> bool {
        let page = self.page_size as usize;
        page >= P64_MIN_PAGE_SIZE && page.is_power_of_two()
    }

    /// True iff `manifold_table_offset` is 64-byte cache-line aligned.
    #[inline]
    pub const fn is_manifold_cache_aligned(&self) -> bool {
        is_cache_line_aligned(self.manifold_table_offset as usize)
    }

    /// Zero-copy slice of the embedded tokenizer section (`tokenizer_offset..checksum_offset`),
    /// returning `&[]` if offsets are out of bounds or inverted.
    #[inline]
    pub fn tokenizer_slice<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        let start = self.tokenizer_offset as usize;
        let end = self.checksum_offset as usize;
        if start <= end && end <= data.len() {
            &data[start..end]
        } else {
            &[]
        }
    }

    /// Validated byte range of manifold record `index` (where `0 <= index <= n_layer`).
    #[inline]
    pub fn manifold_record_range(
        &self,
        n_layer: u32,
        index: u32,
    ) -> Result<core::ops::Range<usize>, &'static str> {
        if index > n_layer {
            return Err("p64: manifold index out of bounds");
        }
        let offset = (index as usize)
            .checked_mul(P64_MANIFOLD_ENTRY_BYTES)
            .ok_or("p64: manifold offset overflow")?;
        let start = (self.manifold_table_offset as usize)
            .checked_add(offset)
            .ok_or("p64: manifold offset overflow")?;
        let end = start
            .checked_add(P64_MANIFOLD_ENTRY_BYTES)
            .ok_or("p64: manifold offset overflow")?;
        Ok(start..end)
    }

    /// Zero-copy 64-byte manifold record slice for `index` within `data`.
    #[inline]
    pub fn manifold_record_slice<'a>(
        &self,
        data: &'a [u8],
        n_layer: u32,
        index: u32,
    ) -> Result<&'a [u8; P64_MANIFOLD_ENTRY_BYTES], &'static str> {
        let range = self.manifold_record_range(n_layer, index)?;
        let slice = data
            .get(range)
            .ok_or("p64: manifold coordinate out of bounds")?;
        slice
            .try_into()
            .map_err(|_| "p64: manifold coordinate out of bounds")
    }
}

impl Default for P64TensorEntry {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl P64TensorEntry {
    /// Zeroed tensor directory entry (`rank = 0`).
    #[inline]
    pub const fn empty() -> Self {
        Self {
            name_offset: 0,
            role_id: 0,
            dtype: 0,
            manifold_idx: 0,
            rank: 0,
            dimensions: [0; 4],
            blob_offset: 0,
            blob_size: 0,
            source_offset: 0,
            source_name_hash: 0,
            alt_dtype: 0,
            precision_views_mask: 0,
            alt_blob_offset: 0,
        }
    }

    /// Little-endian read from a 64-byte entry slice.
    pub fn read_le(data: &[u8]) -> Result<Self, String> {
        if data.len() < P64_TENSOR_ENTRY_BYTES {
            return Err("p64: truncated tensor entry".to_string());
        }
        Ok(Self::read_le_unchecked(data))
    }

    #[inline]
    pub(crate) fn read_le_unchecked(bytes: &[u8]) -> Self {
        let u16a = |o: usize| u16::from_le_bytes(bytes[o..o + 2].try_into().unwrap());
        let u32a = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
        let u64a = |o: usize| u64::from_le_bytes(bytes[o..o + 8].try_into().unwrap());
        Self {
            name_offset: u32a(0),
            role_id: u16a(4),
            dtype: u16a(6),
            manifold_idx: u32a(8),
            rank: u32a(12),
            dimensions: [u32a(16), u32a(20), u32a(24), u32a(28)],
            blob_offset: u32a(32),
            blob_size: u32a(36),
            source_offset: u64a(40),
            source_name_hash: u64a(48),
            alt_dtype: u16a(56),
            precision_views_mask: u16a(58),
            alt_blob_offset: u32a(60),
        }
    }

    /// Little-endian write into a 64-byte slice.
    pub fn write_le(&self, out: &mut [u8]) {
        assert!(out.len() >= P64_TENSOR_ENTRY_BYTES);
        out[..P64_TENSOR_ENTRY_BYTES].fill(0);
        out[0..4].copy_from_slice(&self.name_offset.to_le_bytes());
        out[4..6].copy_from_slice(&self.role_id.to_le_bytes());
        out[6..8].copy_from_slice(&self.dtype.to_le_bytes());
        out[8..12].copy_from_slice(&self.manifold_idx.to_le_bytes());
        out[12..16].copy_from_slice(&self.rank.to_le_bytes());
        for (index, dimension) in self.dimensions.iter().enumerate() {
            let start = 16 + index * 4;
            out[start..start + 4].copy_from_slice(&dimension.to_le_bytes());
        }
        out[32..36].copy_from_slice(&self.blob_offset.to_le_bytes());
        out[36..40].copy_from_slice(&self.blob_size.to_le_bytes());
        out[40..48].copy_from_slice(&entry_u64_le(self.source_offset));
        out[48..56].copy_from_slice(&entry_u64_le(self.source_name_hash));
        out[56..58].copy_from_slice(&self.alt_dtype.to_le_bytes());
        out[58..60].copy_from_slice(&self.precision_views_mask.to_le_bytes());
        out[60..64].copy_from_slice(&self.alt_blob_offset.to_le_bytes());
    }

    /// Checked byte range `blob_offset..blob_offset + blob_size`.
    #[inline]
    pub fn blob_range(&self) -> Option<core::ops::Range<usize>> {
        let start = self.blob_offset as usize;
        let end = start.checked_add(self.blob_size as usize)?;
        Some(start..end)
    }

    /// Checked zero-copy slice of the primary tensor blob within `data`.
    #[inline]
    pub fn blob_slice<'a>(&self, data: &'a [u8]) -> Option<&'a [u8]> {
        data.get(self.blob_range()?)
    }

    /// True iff `blob_offset` is a multiple of power-of-two `alignment`.
    #[inline]
    pub const fn is_blob_aligned(&self, alignment: usize) -> bool {
        alignment.is_power_of_two() && ((self.blob_offset as usize) & (alignment - 1) == 0)
    }
}

#[inline]
const fn entry_u64_le(v: u64) -> [u8; 8] {
    v.to_le_bytes()
}

/// GGUF tensor-name suffix for a per-layer P64 role (`None` for global or unknown roles).
#[inline]
pub const fn p64_role_suffix_str(role_id: u16) -> Option<&'static str> {
    match role_id {
        P64_ROLE_ATTN_K => Some("attn_k.weight"),
        P64_ROLE_ATTN_V => Some("attn_v.weight"),
        P64_ROLE_ATTN_Q => Some("attn_q.weight"),
        P64_ROLE_ATTN_OUTPUT => Some("attn_output.weight"),
        P64_ROLE_FFN_GATE => Some("ffn_gate.weight"),
        P64_ROLE_FFN_UP => Some("ffn_up.weight"),
        P64_ROLE_FFN_DOWN => Some("ffn_down.weight"),
        P64_ROLE_ATTN_NORM => Some("attn_norm.weight"),
        P64_ROLE_FFN_NORM => Some("ffn_norm.weight"),
        P64_ROLE_MOE_ROUTER => Some("ffn_gate_inp.weight"),
        P64_ROLE_MOE_GATE_EXPS => Some("ffn_gate_exps.weight"),
        P64_ROLE_MOE_UP_EXPS => Some("ffn_up_exps.weight"),
        P64_ROLE_MOE_DOWN_EXPS => Some("ffn_down_exps.weight"),
        P64_ROLE_MOE_SHARED_GATE => Some("ffn_gate_shexp.weight"),
        P64_ROLE_MOE_SHARED_UP => Some("ffn_up_shexp.weight"),
        P64_ROLE_MOE_SHARED_DOWN => Some("ffn_down_shexp.weight"),
        _ => None,
    }
}

/// GGUF tensor-name suffix bytes for a per-layer P64 role (`None` for global or unknown roles).
#[inline]
pub const fn p64_role_suffix(role_id: u16) -> Option<&'static [u8]> {
    match p64_role_suffix_str(role_id) {
        Some(s) => Some(s.as_bytes()),
        None => None,
    }
}

/// Format a canonical P64 tensor name (`token_embd.weight`, `blk.{layer}.{suffix}`, or `tensor.{hash:016x}`)
/// into any [`core::fmt::Write`] sink without heap allocation.
pub fn format_p64_tensor_name(
    out: &mut impl core::fmt::Write,
    role: u16,
    layer: u16,
    hash: u64,
) -> core::fmt::Result {
    if layer == P64_LAYER_GLOBAL {
        return match role {
            P64_ROLE_TOKEN_EMBD => out.write_str("token_embd.weight"),
            P64_ROLE_OUTPUT => out.write_str("output.weight"),
            P64_ROLE_OUTPUT_NORM => out.write_str("output_norm.weight"),
            _ => write!(out, "tensor.{hash:016x}"),
        };
    }
    match p64_role_suffix_str(role) {
        Some(suffix) => write!(out, "blk.{layer}.{suffix}"),
        None => write!(out, "tensor.{hash:016x}"),
    }
}

/// Allocate and format a canonical P64 tensor name (`token_embd.weight`, `blk.{layer}.{suffix}`, or `tensor.{hash:016x}`).
pub fn p64_tensor_name(role: u16, layer: u16, hash: u64) -> String {
    let mut name = String::new();
    format_p64_tensor_name(&mut name, role, layer, hash).expect("String formatting is infallible");
    name
}

/// Zero-heap formatting of a canonical P64 tensor name into a caller-supplied byte buffer.
/// Returns `Some(bytes_written)` or `None` if `bytes` is too small.
pub fn p64_tensor_name_into(role: u16, layer: u16, hash: u64, bytes: &mut [u8]) -> Option<usize> {
    struct SliceWriter<'a> {
        bytes: &'a mut [u8],
        used: usize,
    }
    impl core::fmt::Write for SliceWriter<'_> {
        fn write_str(&mut self, value: &str) -> core::fmt::Result {
            let end = self.used.checked_add(value.len()).ok_or(core::fmt::Error)?;
            let target = self.bytes.get_mut(self.used..end).ok_or(core::fmt::Error)?;
            target.copy_from_slice(value.as_bytes());
            self.used = end;
            Ok(())
        }
    }
    let mut writer = SliceWriter { bytes, used: 0 };
    format_p64_tensor_name(&mut writer, role, layer, hash).ok()?;
    Some(writer.used)
}

impl P64HParams {
    /// Little-endian read.
    /// Preserves legacy behavior: the final eight reserved bytes become zero.
    /// Use [`super::P64View::bytes`] to preserve the original container bytes.
    pub fn read_le(data: &[u8]) -> Result<Self, String> {
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
            head_dim: u32a(28),
            head_dim_swa: u32a(32),
            sliding_window: u32a(36),
            shared_kv_layers: u32a(40),
            logit_softcap: f32a(44),
            architecture: u32a(48),
            arch_flags: u32a(52),
            reserved: [0; 8],
        })
    }

    /// Little-endian write.
    /// Preserves legacy behavior: the final eight reserved bytes are written as zero.
    /// Use [`super::P64View::bytes`] for byte-exact container preservation.
    pub fn write_le(&self, out: &mut [u8]) {
        assert!(out.len() >= 64);
        out[..64].fill(0);
        out[0..4].copy_from_slice(&self.n_layer.to_le_bytes());
        out[4..8].copy_from_slice(&self.n_embd.to_le_bytes());
        out[8..12].copy_from_slice(&self.n_head.to_le_bytes());
        out[12..16].copy_from_slice(&self.n_kv_head.to_le_bytes());
        out[16..20].copy_from_slice(&self.vocab_size.to_le_bytes());
        out[20..24].copy_from_slice(&self.rope_freq_base.to_le_bytes());
        out[24..28].copy_from_slice(&self.rope_scale.to_le_bytes());
        out[28..32].copy_from_slice(&self.head_dim.to_le_bytes());
        out[32..36].copy_from_slice(&self.head_dim_swa.to_le_bytes());
        out[36..40].copy_from_slice(&self.sliding_window.to_le_bytes());
        out[40..44].copy_from_slice(&self.shared_kv_layers.to_le_bytes());
        out[44..48].copy_from_slice(&self.logit_softcap.to_le_bytes());
        out[48..52].copy_from_slice(&self.architecture.to_le_bytes());
        out[52..56].copy_from_slice(&self.arch_flags.to_le_bytes());
    }
}

/// `format_flags` bit: container produced by the **raw streaming transcode** (safetensor/MLX →
/// P64) — tensors are verbatim high-fidelity blobs not yet mapped to engine GEMM roles, and the
/// GGUF hyperparameter block is absent. (Distinguishes it from a `compile_gguf_to_p64` container.)
pub const FORMAT_FLAG_RAW_TRANSCODE: u16 = 1 << 1;
/// Alias of [`FORMAT_FLAG_RAW_TRANSCODE`] (header-flag naming).
pub const P64_FLAG_RAW_TRANSCODE: u16 = FORMAT_FLAG_RAW_TRANSCODE;
/// `format_flags` bit: tensors were **ternary-quantized (BitNet 1.58b)** during transcode — each
/// blob is `[scale: f32][packed trits]` (`ggml_type = ternary::GGML_TYPE_TERNARY_158`); decode via
/// `ternary::dequantize_blob`.
pub const FORMAT_FLAG_TERNARY: u16 = 1 << 2;
/// Alias of [`FORMAT_FLAG_TERNARY`] (header-flag naming).
pub const P64_FLAG_TERNARY: u16 = FORMAT_FLAG_TERNARY;
