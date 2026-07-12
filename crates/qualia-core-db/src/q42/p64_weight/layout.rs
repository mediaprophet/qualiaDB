//! P64 container **byte layout**: magic/version/flags, the cache-line (64 B) DOD structs
//! (`P64WeightHeader`, `P64TensorEntry`, `P64HParams`, `P64LayerScheduleEntry`), the tensor-role
//! and header-flag constants, and the little-endian (de)serialization for the header + hparams.
//! Every constant and offset here is format-critical and must not change.

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
pub const P64_WEIGHT_HEADER_BYTES: usize = 64;
pub const P64_TENSOR_ENTRY_BYTES: usize = 64;
/// Ten little-endian `f32` values plus 24 bytes of zero padding.
///
/// Keeping every coordinate in one cache line makes `manifold_idx` an exact
/// 64-byte stride and prevents neighbouring coordinates from sharing a cache
/// line or a WASM SIMD fetch.
pub const P64_MANIFOLD_ENTRY_BYTES: usize = 64;

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
/// A source GGUF tensor preserved byte-for-byte but not consumed by a known
/// engine role. Its source offset and name hash remain in the entry so a
/// validator can still prove complete model preservation.
pub const P64_ROLE_UNKNOWN: u16 = 0xFFFE;
/// `layer` sentinel for non-layer (global) tensors.
pub const P64_LAYER_GLOBAL: u16 = 0xFFFF;

// Metadata bitfields are handled by the q42 layer, no longer embedded in weights.


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
}

impl P64HParams {
    pub(super) fn read_le(data: &[u8]) -> Result<Self, String> {
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

    pub(super) fn write_le(&self, out: &mut [u8]) {
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
