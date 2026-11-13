use super::*;
use crate::inference::TensorInfo;

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
    tensor: &TensorInfo,
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
    let base = tensor_data_start
        .checked_add(tensor.byte_offset)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or(ExecutionError::MmapBounds)?;
    let start = (token_id as usize)
        .checked_mul(bytes_per_token)
        .and_then(|n| base.checked_add(n))
        .ok_or(ExecutionError::MmapBounds)?;
    let end = start
        .checked_add(bytes_per_token)
        .ok_or(ExecutionError::MmapBounds)?;
    mmap.get(start..end).ok_or(ExecutionError::MmapBounds)
}

/// Total packed byte length of a GGUF tensor from its shape and `ggml_type`.
pub fn tensor_byte_len(tensor: &TensorInfo) -> Option<usize> {
    let n0 = tensor.dims[0] as usize;
    if n0 == 0 {
        return None;
    }
    // BitNet-1.58b ternary blob (STELLAR §A, P64 FFN): a SINGLE per-tensor `[scale f32][packed
    // trits]` payload over ALL elements — NOT a per-row block format, so `ggml_row_bytes * dims[1]`
    // does not apply. Compute the whole-tensor packed length directly from the element count.
    if tensor.ggml_type == GGML_TYPE_TERNARY_158 {
        let mut n_elems = n0;
        for dim in tensor.dims.iter().take(tensor.n_dims as usize).skip(1) {
            if *dim == 0 {
                return None;
            }
            n_elems = n_elems.checked_mul(*dim as usize)?;
        }
        return Some(crate::p64::convert::codecs::ternary_blob_len(n_elems));
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
    tensor: &TensorInfo,
) -> Result<&'a [u8], ExecutionError> {
    let len = tensor_byte_len(tensor).ok_or(ExecutionError::UnsupportedType)?;
    let start = tensor_data_start
        .checked_add(tensor.byte_offset)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or(ExecutionError::MmapBounds)?;
    let end = start.checked_add(len).ok_or(ExecutionError::MmapBounds)?;
    mmap.get(start..end).ok_or(ExecutionError::MmapBounds)
}

/// Packed byte width of one logical matrix row (`dims[0]` elements).
pub fn tensor_row_byte_len(tensor: &TensorInfo) -> Result<usize, ExecutionError> {
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
    tensor: &TensorInfo,
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
    let base = tensor_data_start
        .checked_add(tensor.byte_offset)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or(ExecutionError::MmapBounds)?;
    let start = row_start
        .checked_mul(row_bytes)
        .and_then(|n| base.checked_add(n))
        .ok_or(ExecutionError::MmapBounds)?;
    let end = rows
        .checked_mul(row_bytes)
        .and_then(|n| start.checked_add(n))
        .ok_or(ExecutionError::MmapBounds)?;
    mmap.get(start..end).ok_or(ExecutionError::MmapBounds)
}

/// Dequantize one matrix row (`row` index along `dims[1]`) into `out`.
pub fn dequant_matrix_row_into(
    raw: &[u8],
    info: &TensorInfo,
    row: usize,
    out: &mut [f32],
) -> Result<usize, GgmlDequantError> {
    let n0 = info.dims[0] as usize;
    let row_bytes = ggml_row_bytes(info.ggml_type, n0).ok_or(GgmlDequantError::UnsupportedType)?;
    let start = row
        .checked_mul(row_bytes)
        .ok_or(GgmlDequantError::TruncatedInput)?;
    let end = start
        .checked_add(row_bytes)
        .ok_or(GgmlDequantError::TruncatedInput)?;
    let row = raw
        .get(start..end)
        .ok_or(GgmlDequantError::TruncatedInput)?;
    dequantize_row_into(row, info.ggml_type, n0, out)
}

#[cfg(test)]
mod ternary_descriptor_tests {
    use super::*;
    use crate::inference::WeightEncoding;
    use crate::p64::convert::codecs::{ternary_blob, ternary_blob_len};

    #[test]
    fn native_ternary_descriptor_preserves_whole_tensor_blob_lengths() {
        assert_eq!(GGML_TYPE_TERNARY_158, 1158);
        for count in [1usize, 5, 6, 32, 257] {
            let tensor = TensorInfo {
                dims: [count as u64, 0, 0, 0],
                n_dims: 1,
                ggml_type: GGML_TYPE_TERNARY_158,
                byte_offset: 0,
            };
            assert_eq!(tensor_byte_len(&tensor), Some(ternary_blob_len(count)));
            assert_eq!(
                ternary_blob(&vec![0.0; count]).len(),
                ternary_blob_len(count)
            );
            assert_eq!(tensor.encoding(), WeightEncoding::Ternary158);
        }
        let tensor = TensorInfo {
            dims: [5, 7, 3, 0],
            n_dims: 3,
            ggml_type: GGML_TYPE_TERNARY_158,
            byte_offset: 0,
        };
        assert_eq!(tensor_byte_len(&tensor), Some(ternary_blob_len(105)));
        let malformed = TensorInfo {
            dims: [5, 0, 3, 0],
            ..tensor
        };
        assert_eq!(tensor_byte_len(&malformed), None);
    }
}
