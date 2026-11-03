use super::*;

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
