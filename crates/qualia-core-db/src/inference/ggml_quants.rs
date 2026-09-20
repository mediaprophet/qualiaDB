//! GGML quantization block layout and zero-heap row dequantization.
//!
//! Byte strides match `ggml_row_size()` in llama.cpp / ggml. Embedding lookup slices
//! raw mmap bytes via `fetch_token_embedding`; this module dequantizes into
//! caller-supplied `&mut [f32]` buffers (no `Vec` in the hot path).

use crate::gguf_sharder::GgufTensorInfo;

/// GGML element-type identifiers used in GGUF tensor-info headers.
pub const GGML_TYPE_F32: u32 = 0;
pub const GGML_TYPE_F16: u32 = 1;
pub const GGML_TYPE_Q4_0: u32 = 2;
pub const GGML_TYPE_Q5_0: u32 = 6;
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
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
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

/// Errors from zero-copy mmap tensor slicing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionError {
    TensorNotFound,
    TokenOutOfRange,
    UnsupportedType,
    MmapBounds,
}

/// Return a zero-copy `&[u8]` slice of the packed embedding row for `token_id`.
pub fn fetch_token_embedding<'a>(
    mmap: &'a [u8],
    tensor_data_start: u64,
    tensor: &GgufTensorInfo,
    token_id: u32,
) -> Result<&'a [u8], ExecutionError> {
    let n_embd = tensor.dims[0] as usize;
    let n_vocab = tensor.dims[1] as usize;
    if n_embd == 0 {
        return Err(ExecutionError::TensorNotFound);
    }
    if token_id as usize >= n_vocab {
        return Err(ExecutionError::TokenOutOfRange);
    }
    let bytes_per_token =
        ggml_row_bytes(tensor.ggml_type, n_embd).ok_or(ExecutionError::UnsupportedType)?;
    let start =
        (tensor_data_start + tensor.byte_offset) as usize + token_id as usize * bytes_per_token;
    let end = start + bytes_per_token;
    if end > mmap.len() {
        return Err(ExecutionError::MmapBounds);
    }
    Ok(&mmap[start..end])
}

/// Total packed byte length of a GGUF tensor from its shape and `ggml_type`.
pub fn tensor_byte_len(tensor: &GgufTensorInfo) -> Option<usize> {
    let n0 = tensor.dims[0] as usize;
    if n0 == 0 {
        return None;
    }
    // BitNet-1.58b ternary blob (STELLAR §A, P64 FFN): a SINGLE per-tensor `[scale f32][packed
    // trits]` payload over ALL elements — NOT a per-row block format, so `ggml_row_bytes * dims[1]`
    // does not apply. Compute the whole-tensor packed length directly from the element count.
    if tensor.ggml_type == crate::ternary::GGML_TYPE_TERNARY_158 {
        let mut n_elems = n0;
        for dim in tensor.dims.iter().take(tensor.n_dims as usize).skip(1) {
            if *dim == 0 {
                return None;
            }
            n_elems = n_elems.checked_mul(*dim as usize)?;
        }
        return Some(crate::ternary::ternary_blob_len(n_elems));
    }
    let row = ggml_row_bytes(tensor.ggml_type, n0)?;
    let mut rows = 1usize;
    // GGUF stores matrices with the logical row width in dims[0].  Higher
    // dimensions are contiguous batches of those rows (for example
    // [input, intermediate, expert] MoE tensors).  Treating only dims[1] as
    // rows silently mapped one expert from a 3-D tensor and let live MoE
    // dispatch omit the rest.
    for dim in tensor.dims.iter().take(tensor.n_dims as usize).skip(1) {
        if *dim == 0 {
            return None;
        }
        rows = rows.checked_mul(*dim as usize)?;
    }
    row.checked_mul(rows)
}

/// Zero-copy slice of an entire tensor payload from the mmap.
pub fn fetch_tensor_bytes<'a>(
    mmap: &'a [u8],
    tensor_data_start: u64,
    tensor: &GgufTensorInfo,
) -> Result<&'a [u8], ExecutionError> {
    let len = tensor_byte_len(tensor).ok_or(ExecutionError::UnsupportedType)?;
    let start = (tensor_data_start + tensor.byte_offset) as usize;
    let end = start + len;
    if end > mmap.len() {
        return Err(ExecutionError::MmapBounds);
    }
    Ok(&mmap[start..end])
}

/// Packed byte width of one logical matrix row (`dims[0]` elements).
pub fn tensor_row_byte_len(tensor: &GgufTensorInfo) -> Result<usize, ExecutionError> {
    let n0 = tensor.dims[0] as usize;
    if n0 == 0 {
        return Err(ExecutionError::TensorNotFound);
    }
    ggml_row_bytes(tensor.ggml_type, n0).ok_or(ExecutionError::UnsupportedType)
}

/// Zero-copy slice covering vocabulary rows `[row_start, row_start + row_count)`.
pub fn fetch_tensor_row_range_bytes<'a>(
    mmap: &'a [u8],
    tensor_data_start: u64,
    tensor: &GgufTensorInfo,
    row_start: usize,
    row_count: usize,
) -> Result<&'a [u8], ExecutionError> {
    let row_bytes = tensor_row_byte_len(tensor)?;
    let n_rows = if tensor.n_dims > 1 && tensor.dims[1] > 0 {
        tensor.dims[1] as usize
    } else {
        1
    };
    if row_start >= n_rows || row_count == 0 {
        return Err(ExecutionError::TokenOutOfRange);
    }
    let rows = row_count.min(n_rows - row_start);
    let start = (tensor_data_start + tensor.byte_offset) as usize + row_start * row_bytes;
    let end = start + rows * row_bytes;
    if end > mmap.len() {
        return Err(ExecutionError::MmapBounds);
    }
    Ok(&mmap[start..end])
}

/// Dequantize one matrix row (`row` index along `dims[1]`) into `out`.
pub fn dequant_matrix_row_into(
    raw: &[u8],
    info: &GgufTensorInfo,
    row: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    let n0 = info.dims[0] as usize;
    let row_bytes = ggml_row_bytes(info.ggml_type, n0).ok_or(GgmlDequantError::UnsupportedType)?;
    let start = row
        .checked_mul(row_bytes)
        .ok_or(GgmlDequantError::TruncatedInput)?;
    if start + row_bytes > raw.len() {
        return Err(GgmlDequantError::TruncatedInput);
    }
    dequantize_row_into(&raw[start..start + row_bytes], info.ggml_type, n0, out)
}

/// Dequantize one embedding row from raw mmap bytes into `out`.
/// Returns the number of `f32` elements written (≤ `out.len()`).
pub fn dequantize_row_into(
    raw: &[u8],
    ggml_type: u32,
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    if out.len() < n_elems {
        return Err(GgmlDequantError::BufferTooSmall);
    }
    match ggml_type {
        GGML_TYPE_F32 => dequant_f32(raw, n_elems, out),
        GGML_TYPE_F16 => dequant_f16(raw, n_elems, out),
        GGML_TYPE_BF16 => dequant_bf16(raw, n_elems, out),
        GGML_TYPE_Q4_0 => dequant_q4_0(raw, n_elems, out),
        GGML_TYPE_Q5_0 => dequant_q5_0(raw, n_elems, out),
        GGML_TYPE_Q8_0 => dequant_q8_0(raw, n_elems, out),
        GGML_TYPE_Q4_K => dequant_q4_k(raw, n_elems, out),
        GGML_TYPE_Q5_K => dequant_q5_k(raw, n_elems, out),
        GGML_TYPE_Q4_K_SOA => dequant_q4_k_soa(raw, n_elems, out),
        GGML_TYPE_Q6_K => dequant_q6_k(raw, n_elems, out),
        GGML_TYPE_IQ4_NL => dequant_iq4_nl(raw, n_elems, out),
        GGML_TYPE_IQ4_XS => dequant_iq4_xs(raw, n_elems, out),
        _ => Err(GgmlDequantError::UnsupportedType),
    }
}

/// Convert one stock Q4_K superblock (144 B) → SoA superblock (160 B).
///
/// Pre-expands the 8 sub-block `(d·scale, dmin·min)` pairs to f16 so the GPU
/// GEMV never runs `get_scale_min_k4` / shared-header decode.
#[inline]
pub fn q4k_block_to_soa(src: &[u8], dst: &mut [u8]) -> Result<(), GgmlDequantError> {
    if src.len() < 144 || dst.len() < BLOCK_Q4K_SOA_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    // qs first (same nibble layout as stock Q4_K).
    dst[..128].copy_from_slice(&src[16..144]);
    let d = half::f16::from_le_bytes([src[0], src[1]]).to_f32();
    let dmin = half::f16::from_le_bytes([src[2], src[3]]).to_f32();
    let scales: [u8; 12] = src[4..16].try_into().unwrap_or([0; 12]);
    for j in 0..8 {
        let mut sc = 0u8;
        let mut m = 0u8;
        get_scale_min_k4(j, &scales, &mut sc, &mut m);
        let d_bits = half::f16::from_f32(d * sc as f32).to_le_bytes();
        let m_bits = half::f16::from_f32(dmin * m as f32).to_le_bytes();
        let o = 128 + j * 2;
        dst[o] = d_bits[0];
        dst[o + 1] = d_bits[1];
        let om = 144 + j * 2;
        dst[om] = m_bits[0];
        dst[om + 1] = m_bits[1];
    }
    Ok(())
}

/// Expand a full Q4_K tensor blob (row-major superblocks) into SoA layout.
/// `n_row_elems` = dims\[0\] (weights per row). `n_rows` = dims\[1\].
pub fn expand_q4k_tensor_to_soa(
    raw: &[u8],
    n_row_elems: usize,
    n_rows: usize,
    out: &mut [u8],
) -> Result<(), GgmlDequantError> {
    let src_row =
        ggml_row_bytes(GGML_TYPE_Q4_K, n_row_elems).ok_or(GgmlDequantError::UnsupportedType)?;
    let dst_row =
        ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_row_elems).ok_or(GgmlDequantError::UnsupportedType)?;
    let need = dst_row
        .checked_mul(n_rows)
        .ok_or(GgmlDequantError::TruncatedInput)?;
    if out.len() < need || raw.len() < src_row.saturating_mul(n_rows) {
        return Err(GgmlDequantError::TruncatedInput);
    }
    let n_blocks = n_row_elems.div_ceil(256);
    for r in 0..n_rows {
        let src_base = r * src_row;
        let dst_base = r * dst_row;
        for b in 0..n_blocks {
            let s = src_base + b * 144;
            let d = dst_base + b * BLOCK_Q4K_SOA_BYTES;
            if s + 144 > raw.len() || d + BLOCK_Q4K_SOA_BYTES > out.len() {
                return Err(GgmlDequantError::TruncatedInput);
            }
            q4k_block_to_soa(&raw[s..s + 144], &mut out[d..d + BLOCK_Q4K_SOA_BYTES])?;
        }
    }
    Ok(())
}

fn dequant_q4_k_soa(
    raw: &[u8],
    n_elems: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    let n_blocks = n_elems.div_ceil(BLOCK_Q4K_SOA_ELEMS);
    if raw.len() < n_blocks * BLOCK_Q4K_SOA_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    let mut out_idx = 0usize;
    for b in 0..n_blocks {
        let block = &raw[b * BLOCK_Q4K_SOA_BYTES..(b + 1) * BLOCK_Q4K_SOA_BYTES];
        let qs = &block[0..128];
        // Pre-expanded scales.
        let mut d_sub = [0f32; 8];
        let mut m_sub = [0f32; 8];
        for j in 0..8 {
            let o = 128 + j * 2;
            d_sub[j] = half::f16::from_le_bytes([block[o], block[o + 1]]).to_f32();
            let om = 144 + j * 2;
            m_sub[j] = half::f16::from_le_bytes([block[om], block[om + 1]]).to_f32();
        }
        // Same element order as stock Q4_K: for each of 4 groups of 64:
        //   32 low nibbles (sub 2g), then 32 high nibbles (sub 2g+1).
        let mut q_off = 0usize;
        for g in 0..4 {
            let sub0 = g * 2;
            let sub1 = g * 2 + 1;
            for l in 0..32 {
                if out_idx >= n_elems {
                    return Ok(out_idx);
                }
                let nib = (qs[q_off + l] & 0xF) as f32;
                out[out_idx] = d_sub[sub0] * nib - m_sub[sub0];
                out_idx += 1;
            }
            for l in 0..32 {
                if out_idx >= n_elems {
                    return Ok(out_idx);
                }
                let nib = (qs[q_off + l] >> 4) as f32;
                out[out_idx] = d_sub[sub1] * nib - m_sub[sub1];
                out_idx += 1;
            }
            q_off += 32;
        }
    }
    Ok(out_idx.min(n_elems))
}

fn dequant_f32(raw: &[u8], n_elems: usize, out: &mut [f32]) -> Result<usize, GgmlDequantError> {
    let need = n_elems * 4;
    if raw.len() < need {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for i in 0..n_elems {
        out[i] = f32::from_le_bytes(raw[i * 4..i * 4 + 4].try_into().unwrap_or([0; 4]));
    }
    Ok(n_elems)
}

fn dequant_f16(raw: &[u8], n_elems: usize, out: &mut [f32]) -> Result<usize, GgmlDequantError> {
    let need = n_elems * 2;
    if raw.len() < need {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for i in 0..n_elems {
        out[i] =
            half::f16::from_le_bytes(raw[i * 2..i * 2 + 2].try_into().unwrap_or([0; 2])).to_f32();
    }
    Ok(n_elems)
}

/// BF16 → f32: shift 16-bit code into the high half of an f32 bit pattern.
fn dequant_bf16(raw: &[u8], n_elems: usize, out: &mut [f32]) -> Result<usize, GgmlDequantError> {
    let need = n_elems * 2;
    if raw.len() < need {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for i in 0..n_elems {
        let bits = u16::from_le_bytes(raw[i * 2..i * 2 + 2].try_into().unwrap_or([0; 2]));
        out[i] = f32::from_bits((bits as u32) << 16);
    }
    Ok(n_elems)
}

fn dequant_q4_0(raw: &[u8], n_elems: usize, out: &mut [f32]) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 32;
    const BLOCK_BYTES: usize = 18;
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for b in 0..n_blocks {
        let bs = b * BLOCK_BYTES;
        let scale = half::f16::from_le_bytes([raw[bs], raw[bs + 1]]).to_f32();
        let half = BLOCK_ELEMS / 2;
        for j in 0..half {
            if b * BLOCK_ELEMS + j >= n_elems {
                break;
            }
            let byte = raw[bs + 2 + j];
            let x0 = (byte & 0x0F) as i32 - 8;
            let x1 = ((byte >> 4) & 0x0F) as i32 - 8;
            out[b * BLOCK_ELEMS + j] = x0 as f32 * scale;
            let hi = b * BLOCK_ELEMS + j + half;
            if hi < n_elems {
                out[hi] = x1 as f32 * scale;
            }
        }
    }
    Ok(n_elems)
}

/// GGML `dequantize_row_iq4_nl`.
///
/// The Qwen3.8 PLE has 160 values per row, so it is five of these compact
/// 18-byte blocks. Each block has one f16 scale and 16 packed codebook pairs.
fn dequant_iq4_nl(raw: &[u8], n_elems: usize, out: &mut [f32]) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 32;
    const BLOCK_BYTES: usize = 18;
    const VALUES: [f32; 16] = [
        -127.0, -104.0, -83.0, -65.0, -49.0, -35.0, -22.0, -10.0, 1.0, 13.0, 25.0, 38.0, 53.0,
        69.0, 89.0, 113.0,
    ];
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for block_index in 0..n_blocks {
        let base = block_index * BLOCK_BYTES;
        let scale = half::f16::from_le_bytes([raw[base], raw[base + 1]]).to_f32();
        let block_start = block_index * BLOCK_ELEMS;
        for lane in 0..16 {
            let packed = raw[base + 2 + lane];
            let lo = block_start + lane;
            if lo < n_elems {
                out[lo] = scale * VALUES[(packed & 0x0f) as usize];
            }
            let hi = lo + 16;
            if hi < n_elems {
                out[hi] = scale * VALUES[(packed >> 4) as usize];
            }
        }
    }
    Ok(n_elems)
}

/// GGML `dequantize_row_iq4_xs`.
///
/// One 136-byte superblock covers 256 values: an f16 master scale, eight
/// packed 6-bit scales and 128 packed non-linear 4-bit codes.  The lane order
/// deliberately mirrors ggml's CPU/CUDA reference: each 32-value group stores
/// four low-nibble values followed by their four high-nibble partners.
fn dequant_iq4_xs(raw: &[u8], n_elems: usize, out: &mut [f32]) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 256;
    const BLOCK_BYTES: usize = 136;
    const VALUES: [f32; 16] = [
        -127.0, -104.0, -83.0, -65.0, -49.0, -35.0, -22.0, -10.0, 1.0, 13.0, 25.0, 38.0, 53.0,
        69.0, 89.0, 113.0,
    ];
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for block_index in 0..n_blocks {
        let base = block_index * BLOCK_BYTES;
        let block = &raw[base..base + BLOCK_BYTES];
        let master = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
        let scales_h = u16::from_le_bytes([block[2], block[3]]);
        let scales_l = &block[4..8];
        let qs = &block[8..136];
        let block_start = block_index * BLOCK_ELEMS;
        for group in 0..8 {
            let low = (scales_l[group / 2] >> (4 * (group % 2))) & 0x0f;
            let high = ((scales_h >> (2 * group)) & 0x03) as u8;
            let scale = master * (((low | (high << 4)) as i32 - 32) as f32);
            let q_base = group * 16;
            let y_base = block_start + group * 32;
            for lane in 0..16 {
                let packed = qs[q_base + lane];
                let lo = y_base + lane;
                if lo < n_elems {
                    out[lo] = scale * VALUES[(packed & 0x0f) as usize];
                }
                let hi = lo + 16;
                if hi < n_elems {
                    out[hi] = scale * VALUES[(packed >> 4) as usize];
                }
            }
        }
    }
    Ok(n_elems)
}

/// `dequantize_row_q5_0` from ggml-quants.c — 5-bit weights, 32 elems per 22-byte block.
fn dequant_q5_0(raw: &[u8], n_elems: usize, out: &mut [f32]) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 32;
    const BLOCK_BYTES: usize = 22;
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for b in 0..n_blocks {
        let bs = b * BLOCK_BYTES;
        let d = half::f16::from_le_bytes([raw[bs], raw[bs + 1]]).to_f32();
        let qh = u32::from_le_bytes([raw[bs + 2], raw[bs + 3], raw[bs + 4], raw[bs + 5]]);
        let qs = &raw[bs + 6..bs + 22];
        let half = BLOCK_ELEMS / 2;
        for j in 0..half {
            let xh_0 = ((qh >> j) << 4) & 0x10;
            let xh_1 = (qh >> (j + 12)) & 0x10;
            let x0 = ((qs[j] & 0x0F) as u32 | xh_0) as i32 - 16;
            let x1 = ((qs[j] >> 4) as u32 | xh_1) as i32 - 16;
            let lo = b * BLOCK_ELEMS + j;
            if lo < n_elems {
                out[lo] = x0 as f32 * d;
            }
            let hi = lo + half;
            if hi < n_elems {
                out[hi] = x1 as f32 * d;
            }
        }
    }
    Ok(n_elems)
}

fn dequant_q8_0(raw: &[u8], n_elems: usize, out: &mut [f32]) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 32;
    const BLOCK_BYTES: usize = 34;
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    for b in 0..n_blocks {
        let bs = b * BLOCK_BYTES;
        let scale = half::f16::from_le_bytes([raw[bs], raw[bs + 1]]).to_f32();
        let elems = BLOCK_ELEMS.min(n_elems - b * BLOCK_ELEMS);
        for j in 0..elems {
            out[b * BLOCK_ELEMS + j] = raw[bs + 2 + j] as i8 as f32 * scale;
        }
    }
    Ok(n_elems)
}

/// `get_scale_min_k4` from ggml-quants.c — unpack 6-bit scale/min pairs.
#[inline]
fn get_scale_min_k4(j: usize, scales: &[u8; 12], sc: &mut u8, m: &mut u8) {
    if j < 4 {
        *sc = scales[j] & 63;
        *m = scales[j + 4] & 63;
    } else {
        *sc = (scales[j + 4] & 0xF) | ((scales[j - 4] >> 6) << 4);
        *m = (scales[j + 4] >> 4) | ((scales[j] >> 6) << 4);
    }
}

fn dequant_q4_k(raw: &[u8], n_elems: usize, out: &mut [f32]) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 256;
    const BLOCK_BYTES: usize = 144;
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }

    let mut out_idx = 0usize;
    for b in 0..n_blocks {
        let block = &raw[b * BLOCK_BYTES..b * BLOCK_BYTES + BLOCK_BYTES];
        let d = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
        let dmin = half::f16::from_le_bytes([block[2], block[3]]).to_f32();
        let scales: [u8; 12] = block[4..16].try_into().unwrap_or([0; 12]);
        let qs = &block[16..144];

        let block_elems = BLOCK_ELEMS.min(n_elems - b * BLOCK_ELEMS);
        let mut q_off = 0usize;
        let mut is = 0usize;
        let mut j = 0usize;
        while j < block_elems && out_idx < n_elems {
            let mut sc = 0u8;
            let mut m = 0u8;
            get_scale_min_k4(is, &scales, &mut sc, &mut m);
            let d1 = d * sc as f32;
            let m1 = dmin * m as f32;
            get_scale_min_k4(is + 1, &scales, &mut sc, &mut m);
            let d2 = d * sc as f32;
            let m2 = dmin * m as f32;

            for l in 0..32 {
                if out_idx >= n_elems || j >= block_elems {
                    break;
                }
                out[out_idx] = d1 * (qs[q_off + l] & 0xF) as f32 - m1;
                out_idx += 1;
                j += 1;
            }
            for l in 0..32 {
                if out_idx >= n_elems || j >= block_elems {
                    break;
                }
                out[out_idx] = d2 * (qs[q_off + l] >> 4) as f32 - m2;
                out_idx += 1;
                j += 1;
            }
            q_off += 32;
            is += 2;
        }
    }
    Ok(out_idx.min(n_elems))
}

/// GGML `dequantize_row_q5_K`.
///
/// `Q5_K` keeps Q4_K's eight scale/min pairs and adds a 32-byte bitplane.
/// The bitplane is indexed as `[bit-plane][lane]`: each successive group of
/// 32 unpacked values consumes the next bit of the same 32 source bytes.
fn dequant_q5_k(raw: &[u8], n_elems: usize, out: &mut [f32]) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 256;
    const BLOCK_BYTES: usize = 176;
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }
    let mut out_idx = 0usize;
    for block_index in 0..n_blocks {
        let block = &raw[block_index * BLOCK_BYTES..(block_index + 1) * BLOCK_BYTES];
        let d = half::f16::from_le_bytes([block[0], block[1]]).to_f32();
        let dmin = half::f16::from_le_bytes([block[2], block[3]]).to_f32();
        let scales: [u8; 12] = block[4..16].try_into().unwrap_or([0; 12]);
        let qh = &block[16..48];
        let qs = &block[48..176];
        let block_elems = BLOCK_ELEMS.min(n_elems - block_index * BLOCK_ELEMS);
        let mut q_off = 0usize;
        let mut scale_index = 0usize;
        let mut element = 0usize;
        while element < block_elems {
            let mut scale = 0u8;
            let mut minimum = 0u8;
            get_scale_min_k4(scale_index, &scales, &mut scale, &mut minimum);
            let d0 = d * scale as f32;
            let m0 = dmin * minimum as f32;
            get_scale_min_k4(scale_index + 1, &scales, &mut scale, &mut minimum);
            let d1 = d * scale as f32;
            let m1 = dmin * minimum as f32;
            for lane in 0..32 {
                if element >= block_elems {
                    break;
                }
                let high = ((qh[lane] >> (element / 32)) & 1) << 4;
                out[out_idx] = d0 * ((qs[q_off + lane] & 0x0f) | high) as f32 - m0;
                out_idx += 1;
                element += 1;
            }
            for lane in 0..32 {
                if element >= block_elems {
                    break;
                }
                let high = ((qh[lane] >> (element / 32)) & 1) << 4;
                out[out_idx] = d1 * ((qs[q_off + lane] >> 4) | high) as f32 - m1;
                out_idx += 1;
                element += 1;
            }
            q_off += 32;
            scale_index += 2;
        }
    }
    Ok(out_idx)
}

fn dequant_q6_k_block(block: &[u8; 210], out: &mut [f32]) {
    let blk = bytemuck::from_bytes::<BlockQ6K>(block);
    let d = half::f16::from_bits(blk.d).to_f32();
    let mut ql_off = 0usize;
    let mut qh_off = 0usize;
    let mut sc_off = 0usize;
    let mut y_off = 0usize;

    for _ in 0..2 {
        for l in 0..32 {
            let is = l / 16;
            let q1 =
                ((blk.ql[ql_off + l] & 0xF) | (((blk.qh[qh_off + l] >> 0) & 3) << 4)) as i8 - 32;
            let q2 = ((blk.ql[ql_off + l + 32] & 0xF) | (((blk.qh[qh_off + l] >> 2) & 3) << 4))
                as i8
                - 32;
            let q3 =
                ((blk.ql[ql_off + l] >> 4) | (((blk.qh[qh_off + l] >> 4) & 3) << 4)) as i8 - 32;
            let q4 = ((blk.ql[ql_off + l + 32] >> 4) | (((blk.qh[qh_off + l] >> 6) & 3) << 4))
                as i8
                - 32;
            let sc = &blk.scales[sc_off..sc_off + 8];
            out[y_off + l] = d * sc[is] as f32 * q1 as f32;
            out[y_off + l + 32] = d * sc[is + 2] as f32 * q2 as f32;
            out[y_off + l + 64] = d * sc[is + 4] as f32 * q3 as f32;
            out[y_off + l + 96] = d * sc[is + 6] as f32 * q4 as f32;
        }
        y_off += 128;
        ql_off += 64;
        qh_off += 32;
        sc_off += 8;
    }
}

fn dequant_q6_k(raw: &[u8], n_elems: usize, out: &mut [f32]) -> Result<usize, GgmlDequantError> {
    const BLOCK_ELEMS: usize = 256;
    const BLOCK_BYTES: usize = 210;
    let n_blocks = n_elems.div_ceil(BLOCK_ELEMS);
    if raw.len() < n_blocks * BLOCK_BYTES {
        return Err(GgmlDequantError::TruncatedInput);
    }

    let mut written = 0usize;
    for b in 0..n_blocks {
        let block: &[u8; 210] = raw[b * BLOCK_BYTES..b * BLOCK_BYTES + BLOCK_BYTES]
            .try_into()
            .map_err(|_| GgmlDequantError::TruncatedInput)?;
        let elems = BLOCK_ELEMS.min(n_elems - written);
        let mut block_out = [0f32; BLOCK_ELEMS];
        dequant_q6_k_block(block, &mut block_out);
        out[written..written + elems].copy_from_slice(&block_out[..elems]);
        written += elems;
    }
    Ok(written)
}

/// Quantize 256 f32 weights into one Q4_K_SOA superblock (160 bytes).
///
/// Layout produced:
/// - `[0..128)`: nibbles (same as Q4_K: 4 groups × 32 bytes, low=even sub, high=odd sub)
/// - `[128..144)`: 8 × f16 effective scale (d_sub[j])
/// - `[144..160)`: 8 × f16 effective min (m_sub[j])
///
/// Dequant formula: `w = d_sub[j] * nibble - m_sub[j]`
/// So: `d_sub[j] = (max_j - min_j) / 15`, `m_sub[j] = -min_j`
/// And: `nibble = round((w + m_sub[j]) / d_sub[j])` clamped to [0, 15].
fn quantize_block_f32_to_q4_k_soa(src: &[f32], out: &mut [u8]) {
    debug_assert!(out.len() >= BLOCK_Q4K_SOA_BYTES);
    // Zero qs region so unused nibbles are 0.
    out[..128].fill(0);

    for j in 0..8 {
        let sub_start = j * 32;
        let sub_end = (sub_start + 32).min(src.len());

        // Find min/max for this sub-block.
        let mut min_val = 0.0f32;
        let mut max_val = 0.0f32;
        if sub_start < sub_end {
            min_val = src[sub_start];
            max_val = src[sub_start];
            for i in sub_start + 1..sub_end {
                let w = src[i];
                if w < min_val {
                    min_val = w;
                }
                if w > max_val {
                    max_val = w;
                }
            }
        }

        let scale = if max_val > min_val {
            (max_val - min_val) / 15.0
        } else {
            1.0
        };
        // Dequant: w = d_sub * nibble - m_sub
        //   nibble=0 → w = -m_sub = min_val  →  m_sub = -min_val
        //   nibble=15 → w = d_sub*15 - m_sub = max_val  →  d_sub = (max_val - min_val)/15 = scale
        let d_sub = half::f16::from_f32(scale);
        let m_sub = half::f16::from_f32(-min_val);
        let d_actual = d_sub.to_f32();
        let m_actual = m_sub.to_f32();

        // Store f16 scale/min in SoA region.
        let d_bytes = d_sub.to_le_bytes();
        let m_bytes = m_sub.to_le_bytes();
        out[128 + j * 2] = d_bytes[0];
        out[128 + j * 2 + 1] = d_bytes[1];
        out[144 + j * 2] = m_bytes[0];
        out[144 + j * 2 + 1] = m_bytes[1];

        // Quantize and pack nibbles.
        // Group g = j / 2; low nibble if j even, high if j odd.
        let g = j / 2;
        let q_off = g * 32;
        let is_high = j % 2 == 1;
        for l in 0..32 {
            let nibble: u8 = if sub_start + l < sub_end {
                let w = src[sub_start + l];
                let q = ((w + m_actual) / d_actual).round();
                q.clamp(0.0, 15.0) as u8
            } else {
                0
            };
            let bi = q_off + l;
            if is_high {
                out[bi] = (out[bi] & 0x0F) | (nibble << 4);
            } else {
                out[bi] = (out[bi] & 0xF0) | (nibble & 0x0F);
            }
        }
    }
}

/// Quantize a full f32 weight matrix to Q4_K_SOA layout.
///
/// `src`: row-major f32 weights, `n_row_elems × n_rows` elements.
/// `out`: destination buffer, must be at least `ggml_row_bytes(Q4_K_SOA, n_row_elems) * n_rows` bytes.
pub fn quantize_f32_to_q4_k_soa_tensor(
    src: &[f32],
    n_row_elems: usize,
    n_rows: usize,
    out: &mut [u8],
) -> Result<(), GgmlDequantError> {
    let dst_row =
        ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_row_elems).ok_or(GgmlDequantError::UnsupportedType)?;
    let need = dst_row
        .checked_mul(n_rows)
        .ok_or(GgmlDequantError::TruncatedInput)?;
    if out.len() < need {
        return Err(GgmlDequantError::BufferTooSmall);
    }
    if src.len() < n_row_elems * n_rows {
        return Err(GgmlDequantError::TruncatedInput);
    }

    let n_blocks = n_row_elems.div_ceil(BLOCK_Q4K_SOA_ELEMS);
    for r in 0..n_rows {
        let src_base = r * n_row_elems;
        let dst_base = r * dst_row;
        for b in 0..n_blocks {
            let block_start = src_base + b * BLOCK_Q4K_SOA_ELEMS;
            let block_end = (block_start + BLOCK_Q4K_SOA_ELEMS).min(src_base + n_row_elems);
            let block_src = &src[block_start..block_end];
            let dst_off = dst_base + b * BLOCK_Q4K_SOA_BYTES;
            quantize_block_f32_to_q4_k_soa(
                block_src,
                &mut out[dst_off..dst_off + BLOCK_Q4K_SOA_BYTES],
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q4k_soa_roundtrip_matches_aos_dequant() {
        // Build one synthetic Q4_K superblock and check SoA dequant ≈ stock dequant.
        let mut aos = [0u8; 144];
        // d = 1.0 f16, dmin = 0.1 f16
        aos[0..2].copy_from_slice(&half::f16::from_f32(1.0).to_le_bytes());
        aos[2..4].copy_from_slice(&half::f16::from_f32(0.1).to_le_bytes());
        // scales: all 1 for sc, 0 for m (low 6 bits)
        for i in 0..4 {
            aos[4 + i] = 1;
            aos[8 + i] = 0;
        }
        for i in 0..4 {
            aos[12 + i] = 0;
        }
        // qs: low nibble = 3, high nibble = 5
        for i in 16..144 {
            aos[i] = 0x53;
        }
        let mut soa = [0u8; BLOCK_Q4K_SOA_BYTES];
        q4k_block_to_soa(&aos, &mut soa).unwrap();
        let mut out_aos = [0f32; 256];
        let mut out_soa = [0f32; 256];
        dequant_q4_k(&aos, 256, &mut out_aos).unwrap();
        dequant_q4_k_soa(&soa, 256, &mut out_soa).unwrap();
        for i in 0..256 {
            let d = (out_aos[i] - out_soa[i]).abs();
            assert!(
                d < 1e-3,
                "elem {i}: aos={} soa={} δ={d}",
                out_aos[i],
                out_soa[i]
            );
        }
        assert_eq!(ggml_row_bytes(GGML_TYPE_Q4_K_SOA, 256), Some(160));
        assert_eq!(ggml_row_bytes(GGML_TYPE_Q4_K_SOA, 512), Some(320));
    }

    #[test]
    fn quantize_f32_to_soa_roundtrip() {
        // Synthetic weights: 256 values with varied ranges per sub-block.
        let mut src = [0f32; 256];
        for j in 0..8 {
            let base = j as f32 * 0.1 - 0.4;
            let amp = (j as f32 + 1.0) * 0.05;
            for l in 0..32 {
                src[j * 32 + l] = base + amp * (l as f32 / 31.0);
            }
        }
        let mut soa = [0u8; BLOCK_Q4K_SOA_BYTES];
        quantize_block_f32_to_q4_k_soa(&src, &mut soa);
        let mut deq = [0f32; 256];
        dequant_q4_k_soa(&soa, 256, &mut deq).unwrap();
        // Q4_K has 4-bit precision (~1/15 of the sub-block range). Check that
        // the round-trip error is within the quantization step size.
        for j in 0..8 {
            let sub_min = src[j * 32..j * 32 + 32]
                .iter()
                .cloned()
                .fold(f32::MAX, f32::min);
            let sub_max = src[j * 32..j * 32 + 32]
                .iter()
                .cloned()
                .fold(f32::MIN, f32::max);
            let step = (sub_max - sub_min) / 15.0;
            for l in 0..32 {
                let i = j * 32 + l;
                let err = (src[i] - deq[i]).abs();
                assert!(
                    err <= step + 1e-4,
                    "elem {i}: src={} deq={} err={} step={}",
                    src[i],
                    deq[i],
                    err,
                    step
                );
            }
        }
    }

    #[test]
    fn quantize_f32_to_soa_tensor_roundtrip() {
        // 2 rows × 512 elements (2 blocks per row).
        let n_row_elems = 512;
        let n_rows = 2;
        let mut src = vec![0f32; n_row_elems * n_rows];
        for i in 0..src.len() {
            src[i] = ((i as f32) * 0.01 - 5.0).sin() * 0.5;
        }
        let dst_row = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_row_elems).unwrap();
        let mut out = vec![0u8; dst_row * n_rows];
        quantize_f32_to_q4_k_soa_tensor(&src, n_row_elems, n_rows, &mut out).unwrap();
        let mut deq = vec![0f32; n_row_elems * n_rows];
        for r in 0..n_rows {
            let off = r * dst_row;
            dequant_q4_k_soa(
                &out[off..off + dst_row],
                n_row_elems,
                &mut deq[r * n_row_elems..(r + 1) * n_row_elems],
            )
            .unwrap();
        }
        // Check average error is reasonable for 4-bit quantization.
        let mut total_err = 0.0;
        for i in 0..src.len() {
            total_err += (src[i] - deq[i]).abs();
        }
        let avg_err = total_err / src.len() as f32;
        assert!(avg_err < 0.05, "avg quantization error too high: {avg_err}");
    }

    #[test]
    fn bf16_row_bytes_and_dequant() {
        // Gemma-4 norms/scales use ggml_type 30 (BF16): 2 bytes/elem.
        assert_eq!(ggml_row_bytes(GGML_TYPE_BF16, 1024), Some(2048));
        // 1.0_bf16 = 0x3F80, -2.0_bf16 = 0xC000
        let raw: [u8; 4] = [0x80, 0x3F, 0x00, 0xC0];
        let mut out = [0.0f32; 2];
        assert_eq!(
            dequantize_row_into(&raw, GGML_TYPE_BF16, 2, &mut out),
            Ok(2)
        );
        assert!((out[0] - 1.0).abs() < 1e-6, "got {}", out[0]);
        assert!((out[1] + 2.0).abs() < 1e-6, "got {}", out[1]);
        let info = GgufTensorInfo {
            dims: [128, 1, 0, 0],
            n_dims: 2,
            ggml_type: GGML_TYPE_BF16,
            byte_offset: 0,
        };
        assert_eq!(tensor_byte_len(&info), Some(256));
    }

    #[test]
    fn q4_0_row_bytes_stride() {
        // hidden_dim=4096 → (4096/32)*18 = 2304
        assert_eq!(ggml_row_bytes(GGML_TYPE_Q4_0, 4096), Some(2304));
    }

    #[test]
    fn q5_0_row_bytes_stride() {
        // SmolLM2 attn_k row: hidden_dim=960 → (960/32)*22 = 660
        assert_eq!(ggml_row_bytes(GGML_TYPE_Q5_0, 960), Some(660));
    }

    #[test]
    fn ternary_tensor_byte_len_is_whole_blob_not_row_strided() {
        // A1b inc 2a: a ternary FFN tensor is ONE `[scale f32][5-trits/byte]` blob over all
        // dims[0]*dims[1] elements (per-tensor scale), so `tensor_byte_len`/`fetch_tensor_bytes`
        // must return the whole-blob length — NOT the (None) row-based path that choked before.
        let info = GgufTensorInfo {
            dims: [960, 2560, 0, 0], // SmolLM2 ffn_gate-shape
            n_dims: 2,
            ggml_type: crate::ternary::GGML_TYPE_TERNARY_158,
            byte_offset: 0,
        };
        let n = 960 * 2560;
        assert_eq!(
            tensor_byte_len(&info),
            Some(crate::ternary::ternary_blob_len(n))
        );

        // and fetch returns exactly that slice from a buffer holding a real ternary blob.
        let weights: Vec<f32> = (0..n).map(|i| (i as f32 * 0.001).sin()).collect();
        let blob = crate::ternary::ternary_blob(&weights);
        assert_eq!(blob.len(), crate::ternary::ternary_blob_len(n));
        let got = fetch_tensor_bytes(&blob, 0, &info).expect("ternary fetch must slice");
        assert_eq!(got.len(), blob.len());
        assert_eq!(&got[..4], &blob[..4]); // scale preserved at the front
    }

    #[test]
    fn q5_0_dequant_matches_gguf_smollm2_row0() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf");
        if !path.exists() {
            return;
        }
        let mmap = std::fs::read(&path).expect("read gguf");
        let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(&mmap);
        let info = index.get_layer_tensors(0).attn_k.expect("blk.0.attn_k");
        let raw = fetch_tensor_bytes(&mmap, index.tensor_data_start, &info).expect("fetch attn_k");
        let row_bytes = ggml_row_bytes(GGML_TYPE_Q5_0, info.dims[0] as usize).unwrap();
        assert_eq!(row_bytes, 660);
        let mut out = [0f32; 960];
        dequantize_row_into(&raw[..row_bytes], GGML_TYPE_Q5_0, 960, &mut out).unwrap();
        // Reference: gguf-py dequantize_row_q5_0 on blk.0.attn_k.weight row 0
        let expected = [
            -0.0f32,
            -0.02502441,
            -0.10009766,
            -0.07507324,
            -0.05004883,
            -0.3503418,
            -0.0,
            0.05004883,
            -0.07507324,
            0.1751709,
        ];
        for (i, &exp) in expected.iter().enumerate() {
            assert!(
                (out[i] - exp).abs() < 1e-5,
                "elem {i}: got {} expected {}",
                out[i],
                exp
            );
        }
    }

    #[test]
    fn smollm_q4km_tensor_types_and_q4k_row0() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf");
        if !path.exists() {
            return;
        }
        let mmap = std::fs::read(&path).expect("read gguf");
        let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(&mmap);
        let emb = index.token_embd_info().expect("token_embd");
        println!("token_embd type={} dims={:?}", emb.ggml_type, emb.dims);
        let lt = index.get_layer_tensors(0);
        for (name, info) in [
            ("attn_q", lt.attn_q),
            ("attn_k", lt.attn_k),
            ("attn_v", lt.attn_v),
            ("ffn_gate", lt.ffn_gate),
            ("ffn_down", lt.ffn_down),
        ] {
            if let Some(i) = info {
                println!("{name} type={} dims={:?}", i.ggml_type, i.dims);
            }
        }
        // token 504 = "The" in naked prompt
        let mut out = [0f32; 960];
        let n = index.dequantize_token_embedding_into(&mmap, 504, &mut out);
        assert_eq!(n, 960);
        println!(
            "token504 first10: {:?}",
            &out[..10]
                .iter()
                .map(|v| format!("{v:.6}"))
                .collect::<Vec<_>>()
        );
        assert!(out.iter().any(|&v| v.is_finite() && v != 0.0));
    }

    #[test]
    fn q4_k_row_bytes_stride() {
        // hidden_dim=2560 → (2560/256)*144 = 1440
        assert_eq!(ggml_row_bytes(GGML_TYPE_Q4_K, 2560), Some(1440));
    }

    #[test]
    fn q5_k_uses_its_high_bit_plane_and_scale_min_pairs() {
        let mut raw = [0u8; 176];
        raw[0..2].copy_from_slice(&half::f16::from_f32(1.0).to_le_bytes());
        raw[2..4].copy_from_slice(&half::f16::from_f32(0.5).to_le_bytes());
        // First low/high 32-value groups: scale=2/min=1 and scale=3/min=2.
        raw[4] = 2;
        raw[5] = 3;
        raw[8] = 1;
        raw[9] = 2;
        raw[16] = 0b0000_0010; // lane zero high bit for the second 32 values.
        raw[48] = 5 | (6 << 4);
        let mut out = [0.0f32; 256];
        assert_eq!(dequant_q5_k(&raw, 256, &mut out), Ok(256));
        assert!((out[0] - 9.5).abs() < 1e-6); // 2*5 - 0.5*1
        assert!((out[32] - 65.0).abs() < 1e-6); // 3*(6|16) - 0.5*2
        assert_eq!(ggml_row_bytes(GGML_TYPE_Q5_K, 256), Some(176));
    }

    #[test]
    fn q6_k_row_bytes_stride() {
        // Gemma 4B token_embd: hidden_dim=2560 → (2560/256)*210 = 2100
        assert_eq!(ggml_row_bytes(GGML_TYPE_Q6_K, 2560), Some(2100));
    }

    #[test]
    fn q6_k_dequant_matches_gguf_smollm2_ffn_down_row0() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf");
        if !path.exists() {
            return;
        }
        let mmap = std::fs::read(&path).expect("read gguf");
        let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(&mmap);
        let info = index.get_layer_tensors(0).ffn_down.expect("blk.0.ffn_down");
        assert_eq!(info.ggml_type, GGML_TYPE_Q6_K);
        let raw =
            fetch_tensor_bytes(&mmap, index.tensor_data_start, &info).expect("fetch ffn_down");
        let row_bytes = ggml_row_bytes(GGML_TYPE_Q6_K, info.dims[0] as usize).unwrap();
        assert_eq!(row_bytes, 2100);
        let mut out = [0f32; 2560];
        dequantize_row_into(&raw[..row_bytes], GGML_TYPE_Q6_K, 2560, &mut out).unwrap();
        // Reference: llama.cpp dequantize_row_q6_K (signed int8 scales) on row 0
        let expected = [
            -0.11712998f32,
            -0.16535997,
            -0.0,
            0.15157998,
            -0.08956999,
            -0.02067,
            0.04133999,
            0.03445,
            0.17913999,
            0.22047997,
        ];
        for (i, &exp) in expected.iter().enumerate() {
            assert!(
                (out[i] - exp).abs() < 1e-5,
                "elem {i}: got {} expected {}",
                out[i],
                exp
            );
        }
        assert!(out.iter().any(|&v| v.is_finite() && v != 0.0));
    }

    #[test]
    fn block_q6k_layout_matches_ggml() {
        assert_eq!(std::mem::size_of::<BlockQ6K>(), BLOCK_Q6K_BYTES);
        assert_eq!(std::mem::align_of::<BlockQ6K>(), 2);
    }

    #[test]
    fn fetch_row_range_bounds() {
        use crate::gguf_sharder::GgufTensorInfo;
        let info = GgufTensorInfo {
            dims: [2560, 100, 0, 0],
            n_dims: 2,
            ggml_type: GGML_TYPE_Q6_K,
            byte_offset: 0,
        };
        let row = tensor_row_byte_len(&info).unwrap();
        let total = tensor_byte_len(&info).unwrap();
        assert_eq!(total, row * 100);
        let fake = vec![0u8; total];
        let chunk = fetch_tensor_row_range_bytes(&fake, 0, &info, 10, 8).unwrap();
        assert_eq!(chunk.len(), row * 8);
    }

    #[test]
    fn tensor_byte_len_covers_all_expert_dimensions() {
        let info = GgufTensorInfo {
            dims: [32, 7, 11, 0],
            n_dims: 3,
            ggml_type: GGML_TYPE_F32,
            byte_offset: 0,
        };
        // 11 experts × 7 rows/expert × 32 f32 values/row.
        assert_eq!(tensor_byte_len(&info), Some(32 * 7 * 11 * 4));

        let ternary = GgufTensorInfo {
            ggml_type: crate::ternary::GGML_TYPE_TERNARY_158,
            ..info
        };
        assert_eq!(
            tensor_byte_len(&ternary),
            Some(crate::ternary::ternary_blob_len(32 * 7 * 11))
        );
    }

    #[test]
    fn q8_0_block_roundtrip() {
        let mut block = [0u8; 34];
        block[0] = 0x00;
        block[1] = 0x3C; // f16 1.0
        for i in 0..32 {
            block[2 + i] = (i + 1) as u8;
        }
        let mut out = [0f32; 32];
        dequant_q8_0(&block, 32, &mut out).unwrap();
        assert!((out[0] - 1.0).abs() < 0.01);
        assert!((out[31] - 32.0).abs() < 0.01);
    }

    #[test]
    fn iq4_xs_block_layout_and_lane_order_match_ggml() {
        assert_eq!(ggml_row_bytes(GGML_TYPE_IQ4_XS, 160), Some(136));
        let info = GgufTensorInfo {
            dims: [160, 2, 0, 0],
            n_dims: 2,
            ggml_type: GGML_TYPE_IQ4_XS,
            byte_offset: 0,
        };
        assert_eq!(tensor_byte_len(&info), Some(272));

        let mut block = [0x88u8; 136]; // nonlinear code 8 == +1
        block[0..2].copy_from_slice(&half::f16::from_f32(1.0).to_le_bytes());
        // group 0 scale code = 33: low four bits in scales_l, high two in scales_h.
        block[2..4].copy_from_slice(&2u16.to_le_bytes());
        block[4] = 1;
        let mut out = [0.0f32; 256];
        assert_eq!(
            dequantize_row_into(&block, GGML_TYPE_IQ4_XS, 256, &mut out),
            Ok(256)
        );
        assert!(out[..32]
            .iter()
            .all(|value| (*value - 1.0).abs() < f32::EPSILON));
        // The next group has the default scale code 0, thus -32 times code 8.
        assert!(out[32..64]
            .iter()
            .all(|value| (*value + 32.0).abs() < f32::EPSILON));
    }

    #[test]
    fn iq4_nl_block_layout_and_lane_order_match_ggml() {
        assert_eq!(ggml_row_bytes(GGML_TYPE_IQ4_NL, 160), Some(90));
        let mut block = [0x88u8; 18]; // code 8 is +1 in the non-linear table.
        block[0..2].copy_from_slice(&half::f16::from_f32(1.0).to_le_bytes());
        let mut out = [0.0f32; 32];
        assert_eq!(
            dequantize_row_into(&block, GGML_TYPE_IQ4_NL, 32, &mut out),
            Ok(32)
        );
        assert!(out.iter().all(|value| (*value - 1.0).abs() < f32::EPSILON));
    }
}
