/// GGML element-type identifiers used in GGUF tensor-info headers.
pub const GGML_TYPE_F32: u32 = 0;
pub const GGML_TYPE_F16: u32 = 1;
pub const GGML_TYPE_Q4_0: u32 = 2;
pub const GGML_TYPE_Q5_0: u32 = 6;
pub const GGML_TYPE_Q5_1: u32 = 7;
pub const GGML_TYPE_Q8_0: u32 = 8;
pub const GGML_TYPE_Q4_K: u32 = 12;
/// GGML's 256-value K-quant block with a fifth high-bit plane. Qwen3.8
/// Flash Next stores its fused GatedDeltaNet QKV projection in this layout.
pub const GGML_TYPE_Q5_K: u32 = 13;
pub const GGML_TYPE_Q6_K: u32 = 14;
/// GGML's 32-value non-linear 4-bit block. Qwen3.8 Flash Next uses this
/// specifically for its 160-wide PLE rows, even in the IQ4_XS model variant.
pub const GGML_TYPE_IQ4_NL: u32 = 20;
/// Importance-aware, non-linear 4-bit quantization with per-32-value scales.
///
/// `IQ4_XS` is the layout used by the downloaded Qwen3.8 Flash Next IQ4_XS
/// checkpoint, including its externally streamed PLE table.  It is a stock
/// GGML type (enum value 23), not a Qualia container extension.
pub const GGML_TYPE_IQ4_XS: u32 = 23;
/// Brain float16 (1 sign / 8 exp / 7 mantissa) — used by Gemma-4 and other modern GGUFs
/// for norms / residual scales alongside Q4_K weights (`ggml_type` enum value 30).
pub const GGML_TYPE_BF16: u32 = 30;
pub const GGML_TYPE_MXFP4: u32 = 39;
/// FP8 E4M3 (1 sign / 4 exp / 3 mantissa) — 1 byte per element.
pub const GGML_TYPE_FP8_E4M3: u32 = 40;
/// Qualia conversion-time **SoA Q4_K** (not a stock GGML type).
///
/// Per 256-weight superblock (160 bytes, vs 144 AoS):
/// - `[0..128)`: qs nibbles (same layout as Q4_K)
/// - `[128..144)`: 8× f16 `d * sub_scale` (pre-expanded)
/// - `[144..160)`: 8× f16 `dmin * sub_min` (pre-expanded)
///
/// Decode GEMV loads scales directly (no 6-bit scale unpack, no shared-header
/// barriers). Type id 112 is outside the stock ggml enum range.
pub const GGML_TYPE_Q4_K_SOA: u32 = 112;
/// Bytes per SoA superblock (256 weights).
pub const BLOCK_Q4K_SOA_BYTES: usize = 160;
pub const BLOCK_Q4K_SOA_ELEMS: usize = 256;

/// GGML `block_q6_K` — 210 bytes, 256 weights. Mirrors WGSL `BlockQ6K` layout.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlockQ6K {
    pub ql: [u8; 128],
    pub qh: [u8; 64],
    pub scales: [i8; 16],
    pub d: u16,
}

pub const BLOCK_Q6K_BYTES: usize = 210;
pub const BLOCK_Q6K_ELEMS: usize = 256;

/// Elements per quantization block and packed byte size (from ggml).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GgmlBlockLayout {
    pub block_elems: usize,
    pub block_bytes: usize,
}

/// Return block layout for a GGML type, or `None` if unsupported.
pub fn ggml_block_layout(ggml_type: u32) -> Option<GgmlBlockLayout> {
    match ggml_type {
        GGML_TYPE_Q4_0 => Some(GgmlBlockLayout {
            block_elems: 32,
            block_bytes: 18,
        }),
        GGML_TYPE_Q5_0 => Some(GgmlBlockLayout {
            block_elems: 32,
            block_bytes: 22,
        }),
        GGML_TYPE_Q5_1 => Some(GgmlBlockLayout {
            block_elems: 32,
            block_bytes: 24,
        }),
        GGML_TYPE_MXFP4 => Some(GgmlBlockLayout {
            block_elems: 32,
            block_bytes: 17,
        }),
        GGML_TYPE_Q8_0 => Some(GgmlBlockLayout {
            block_elems: 32,
            block_bytes: 34,
        }),
        GGML_TYPE_Q4_K => Some(GgmlBlockLayout {
            block_elems: 256,
            block_bytes: 144,
        }),
        GGML_TYPE_Q5_K => Some(GgmlBlockLayout {
            block_elems: 256,
            block_bytes: 176,
        }),
        GGML_TYPE_Q4_K_SOA => Some(GgmlBlockLayout {
            block_elems: BLOCK_Q4K_SOA_ELEMS,
            block_bytes: BLOCK_Q4K_SOA_BYTES,
        }),
        GGML_TYPE_Q6_K => Some(GgmlBlockLayout {
            block_elems: 256,
            block_bytes: 210,
        }),
        GGML_TYPE_IQ4_NL => Some(GgmlBlockLayout {
            block_elems: 32,
            block_bytes: 18,
        }),
        GGML_TYPE_IQ4_XS => Some(GgmlBlockLayout {
            block_elems: 256,
            block_bytes: 136,
        }),
        _ => None,
    }
}

/// Packed byte length of one logical row (`n_elems` weights) for the given GGML type.
pub fn ggml_row_bytes(ggml_type: u32, n_elems: usize) -> Option<usize> {
    match ggml_type {
        GGML_TYPE_F32 => Some(n_elems.checked_mul(4)?),
        GGML_TYPE_F16 | GGML_TYPE_BF16 => Some(n_elems.checked_mul(2)?),
        GGML_TYPE_FP8_E4M3 => Some(n_elems),
        _ => {
            let layout = ggml_block_layout(ggml_type)?;
            if n_elems == 0 {
                return Some(0);
            }
            Some(n_elems.div_ceil(layout.block_elems) * layout.block_bytes)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GgmlDequantError {
    UnsupportedType,
    BufferTooSmall,
    TruncatedInput,
}

/// P64 BitNet 1.58-bit whole-tensor encoding.
pub const GGML_TYPE_TERNARY_158: u32 = 1158;
