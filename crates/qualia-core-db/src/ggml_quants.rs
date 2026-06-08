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
pub const GGML_TYPE_Q8_0: u32 = 8;
pub const GGML_TYPE_Q4_K: u32 = 12;
pub const GGML_TYPE_Q6_K: u32 = 14;

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
        GGML_TYPE_Q8_0 => Some(GgmlBlockLayout {
            block_elems: 32,
            block_bytes: 34,
        }),
        GGML_TYPE_Q4_K => Some(GgmlBlockLayout {
            block_elems: 256,
            block_bytes: 144,
        }),
        GGML_TYPE_Q6_K => Some(GgmlBlockLayout {
            block_elems: 256,
            block_bytes: 210,
        }),
        _ => None,
    }
}

/// Packed byte length of one logical row (`n_elems` weights) for the given GGML type.
pub fn ggml_row_bytes(ggml_type: u32, n_elems: usize) -> Option<usize> {
    match ggml_type {
        GGML_TYPE_F32 => Some(n_elems.checked_mul(4)?),
        GGML_TYPE_F16 => Some(n_elems.checked_mul(2)?),
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
    let row = ggml_row_bytes(tensor.ggml_type, n0)?;
    if tensor.n_dims <= 1 || tensor.dims[1] == 0 {
        Some(row)
    } else {
        Some(row.checked_mul(tensor.dims[1] as usize)?)
    }
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
        GGML_TYPE_Q4_0 => dequant_q4_0(raw, n_elems, out),
        GGML_TYPE_Q8_0 => dequant_q8_0(raw, n_elems, out),
        GGML_TYPE_Q4_K => dequant_q4_k(raw, n_elems, out),
        GGML_TYPE_Q6_K => dequant_q6_k(raw, n_elems, out),
        _ => Err(GgmlDequantError::UnsupportedType),
    }
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

fn dequant_q6_k_block(block: &[u8; 210], out: &mut [f32]) {
    let d = half::f16::from_le_bytes([block[208], block[209]]).to_f32();
    let mut ql_off = 0usize;
    let mut qh_off = 128usize;
    let mut sc_off = 192usize;
    let mut y_off = 0usize;

    for _ in 0..2 {
        for l in 0..32 {
            let is = l / 16;
            let q1 = ((block[ql_off + l] & 0xF) | (((block[qh_off + l] >> 0) & 3) << 4)) as i8 - 32;
            let q2 =
                ((block[ql_off + l + 32] & 0xF) | (((block[qh_off + l] >> 2) & 3) << 4)) as i8 - 32;
            let q3 = ((block[ql_off + l] >> 4) | (((block[qh_off + l] >> 4) & 3) << 4)) as i8 - 32;
            let q4 =
                ((block[ql_off + l + 32] >> 4) | (((block[qh_off + l] >> 6) & 3) << 4)) as i8 - 32;
            out[y_off + l] = d * block[sc_off + is] as f32 * q1 as f32;
            out[y_off + l + 32] = d * block[sc_off + is + 2] as f32 * q2 as f32;
            out[y_off + l + 64] = d * block[sc_off + is + 4] as f32 * q3 as f32;
            out[y_off + l + 96] = d * block[sc_off + is + 6] as f32 * q4 as f32;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q4_0_row_bytes_stride() {
        // hidden_dim=4096 → (4096/32)*18 = 2304
        assert_eq!(ggml_row_bytes(GGML_TYPE_Q4_0, 4096), Some(2304));
    }

    #[test]
    fn q4_k_row_bytes_stride() {
        // hidden_dim=2560 → (2560/256)*144 = 1440
        assert_eq!(ggml_row_bytes(GGML_TYPE_Q4_K, 2560), Some(1440));
    }

    #[test]
    fn q6_k_row_bytes_stride() {
        // Gemma 4B token_embd: hidden_dim=2560 → (2560/256)*210 = 2100
        assert_eq!(ggml_row_bytes(GGML_TYPE_Q6_K, 2560), Some(2100));
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
}
