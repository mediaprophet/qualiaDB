//! Zero-copy ordinary-GGUF MoE tensor execution.

use crate::gguf_sharder::GgufTensorInfo;

/// Evaluate one expert from regular 3-D GGUF tensors. All intermediate
/// storage is supplied by the caller; the mmap stays zero-copy.
#[allow(clippy::too_many_arguments)]
pub(super) fn evaluate_gguf_swiglu_expert(
    mmap: &[u8],
    tensor_data_start: u64,
    gate: &GgufTensorInfo,
    up: &GgufTensorInfo,
    down: &GgufTensorInfo,
    expert: usize,
    input: &[f32],
    gate_buf: &mut [f32],
    up_buf: &mut [f32],
    swiglu_buf: &mut [f32],
    out: &mut [f32],
) -> bool {
    let emb_dim = input.len();
    let inter_dim = gate.dims[1] as usize;
    if gate.n_dims < 3
        || up.n_dims < 3
        || down.n_dims < 3
        || gate.dims[0] as usize != emb_dim
        || up.dims[0] as usize != emb_dim
        || up.dims[1] as usize != inter_dim
        || down.dims[0] as usize != inter_dim
        || down.dims[1] as usize != emb_dim
        || expert >= gate.dims[2] as usize
        || expert >= up.dims[2] as usize
        || expert >= down.dims[2] as usize
        || inter_dim > gate_buf.len()
        || inter_dim > up_buf.len()
        || inter_dim > swiglu_buf.len()
        || emb_dim > out.len()
    {
        return false;
    }
    let (Ok(gate_raw), Ok(up_raw), Ok(down_raw)) = (
        crate::ggml_quants::fetch_tensor_bytes(mmap, tensor_data_start, gate),
        crate::ggml_quants::fetch_tensor_bytes(mmap, tensor_data_start, up),
        crate::ggml_quants::fetch_tensor_bytes(mmap, tensor_data_start, down),
    ) else {
        return false;
    };
    let mut row = [0.0f32; crate::inference::moe::dispatch::MAX_INTERMEDIATE_DIM];
    if emb_dim > row.len() {
        return false;
    }
    for r in 0..inter_dim {
        let flat_row = expert * inter_dim + r;
        if crate::ggml_quants::dequant_matrix_row_into(
            gate_raw,
            gate,
            flat_row,
            &mut row[..emb_dim],
        )
        .is_err()
        {
            return false;
        }
        gate_buf[r] = dot(input, &row[..emb_dim]);
        if crate::ggml_quants::dequant_matrix_row_into(up_raw, up, flat_row, &mut row[..emb_dim])
            .is_err()
        {
            return false;
        }
        up_buf[r] = dot(input, &row[..emb_dim]);
        swiglu_buf[r] = crate::inference::moe::dispatch::silu(gate_buf[r]) * up_buf[r];
    }
    for r in 0..emb_dim {
        if crate::ggml_quants::dequant_matrix_row_into(
            down_raw,
            down,
            expert * emb_dim + r,
            &mut row[..inter_dim],
        )
        .is_err()
        {
            return false;
        }
        out[r] = dot(&swiglu_buf[..inter_dim], &row[..inter_dim]);
    }
    true
}

/// Evaluate one expert from Gemma's fused `[gate rows; up rows]` 3-D GGUF tensor.
///
/// The tensor is `[model_dim, 2 * intermediate_dim, expert_count]`; retaining it fused avoids
/// a cold conversion and keeps every expert row directly mapped from the GGUF payload.
#[allow(clippy::too_many_arguments)]
pub(super) fn evaluate_gguf_swiglu_expert_fused_gate_up(
    mmap: &[u8],
    tensor_data_start: u64,
    gate_up: &GgufTensorInfo,
    down: &GgufTensorInfo,
    expert: usize,
    input: &[f32],
    gate_buf: &mut [f32],
    up_buf: &mut [f32],
    swiglu_buf: &mut [f32],
    out: &mut [f32],
) -> bool {
    let emb_dim = input.len();
    let fused_rows = gate_up.dims[1] as usize;
    if gate_up.n_dims < 3
        || down.n_dims < 3
        || fused_rows == 0
        || fused_rows % 2 != 0
        || gate_up.dims[0] as usize != emb_dim
        || down.dims[0] as usize != fused_rows / 2
        || down.dims[1] as usize != emb_dim
        || expert >= gate_up.dims[2] as usize
        || expert >= down.dims[2] as usize
        || fused_rows / 2 > gate_buf.len()
        || fused_rows / 2 > up_buf.len()
        || fused_rows / 2 > swiglu_buf.len()
        || emb_dim > out.len()
    {
        return false;
    }
    let inter_dim = fused_rows / 2;
    let (Ok(gate_up_raw), Ok(down_raw)) = (
        crate::ggml_quants::fetch_tensor_bytes(mmap, tensor_data_start, gate_up),
        crate::ggml_quants::fetch_tensor_bytes(mmap, tensor_data_start, down),
    ) else {
        return false;
    };
    let mut row = [0.0f32; crate::inference::moe::dispatch::MAX_INTERMEDIATE_DIM];
    if emb_dim > row.len() || inter_dim > row.len() {
        return false;
    }
    let expert_start = expert * fused_rows;
    for r in 0..inter_dim {
        if crate::ggml_quants::dequant_matrix_row_into(
            gate_up_raw,
            gate_up,
            expert_start + r,
            &mut row[..emb_dim],
        )
        .is_err()
        {
            return false;
        }
        gate_buf[r] = dot(input, &row[..emb_dim]);
        if crate::ggml_quants::dequant_matrix_row_into(
            gate_up_raw,
            gate_up,
            expert_start + inter_dim + r,
            &mut row[..emb_dim],
        )
        .is_err()
        {
            return false;
        }
        up_buf[r] = dot(input, &row[..emb_dim]);
        swiglu_buf[r] = crate::inference::moe::dispatch::silu(gate_buf[r]) * up_buf[r];
    }
    for r in 0..emb_dim {
        if crate::ggml_quants::dequant_matrix_row_into(
            down_raw,
            down,
            expert * emb_dim + r,
            &mut row[..inter_dim],
        )
        .is_err()
        {
            return false;
        }
        out[r] = dot(&swiglu_buf[..inter_dim], &row[..inter_dim]);
    }
    true
}

#[allow(clippy::too_many_arguments)]
pub(super) fn evaluate_gguf_dense_swiglu(
    mmap: &[u8],
    tensor_data_start: u64,
    gate: &GgufTensorInfo,
    up: &GgufTensorInfo,
    down: &GgufTensorInfo,
    input: &[f32],
    gate_buf: &mut [f32],
    up_buf: &mut [f32],
    swiglu_buf: &mut [f32],
    out: &mut [f32],
) -> bool {
    let emb_dim = input.len();
    let inter_dim = gate.dims[1] as usize;
    if gate.n_dims != 2
        || up.n_dims != 2
        || down.n_dims != 2
        || gate.dims[0] as usize != emb_dim
        || up.dims[0] as usize != emb_dim
        || up.dims[1] as usize != inter_dim
        || down.dims[0] as usize != inter_dim
        || down.dims[1] as usize != emb_dim
        || inter_dim > gate_buf.len()
        || inter_dim > up_buf.len()
        || inter_dim > swiglu_buf.len()
        || emb_dim > out.len()
        || emb_dim > crate::inference::moe::dispatch::MAX_INTERMEDIATE_DIM
    {
        return false;
    }
    let (Ok(gate_raw), Ok(up_raw), Ok(down_raw)) = (
        crate::ggml_quants::fetch_tensor_bytes(mmap, tensor_data_start, gate),
        crate::ggml_quants::fetch_tensor_bytes(mmap, tensor_data_start, up),
        crate::ggml_quants::fetch_tensor_bytes(mmap, tensor_data_start, down),
    ) else {
        return false;
    };
    let mut row = [0.0f32; crate::inference::moe::dispatch::MAX_INTERMEDIATE_DIM];
    for r in 0..inter_dim {
        if crate::ggml_quants::dequant_matrix_row_into(gate_raw, gate, r, &mut row[..emb_dim])
            .is_err()
        {
            return false;
        }
        gate_buf[r] = dot(input, &row[..emb_dim]);
        if crate::ggml_quants::dequant_matrix_row_into(up_raw, up, r, &mut row[..emb_dim]).is_err()
        {
            return false;
        }
        up_buf[r] = dot(input, &row[..emb_dim]);
        swiglu_buf[r] = crate::inference::moe::dispatch::silu(gate_buf[r]) * up_buf[r];
    }
    for r in 0..emb_dim {
        if crate::ggml_quants::dequant_matrix_row_into(down_raw, down, r, &mut row[..inter_dim])
            .is_err()
        {
            return false;
        }
        out[r] = dot(&swiglu_buf[..inter_dim], &row[..inter_dim]);
    }
    true
}

#[inline]
pub(super) fn dot(left: &[f32], right: &[f32]) -> f32 {
    let mut sum = 0.0;
    for i in 0..left.len() {
        sum += left[i] * right[i];
    }
    sum
}

/// Router rows are ordinary GGUF matrix rows. Use their declared quantizer
/// and stride instead of assuming F16 for every non-Q8 model.
pub(super) fn compute_router_logits_from_gguf(
    raw: &[u8],
    info: &GgufTensorInfo,
    input: &[f32],
    num_experts: usize,
    out_logits: &mut [f32],
) -> bool {
    let emb_dim = input.len();
    if info.n_dims < 2
        || info.dims[0] as usize != emb_dim
        || (info.dims[1] as usize) < num_experts
        || num_experts > out_logits.len()
        || emb_dim > crate::inference::moe::dispatch::MAX_INTERMEDIATE_DIM
    {
        return false;
    }
    let mut row = [0.0f32; crate::inference::moe::dispatch::MAX_INTERMEDIATE_DIM];
    for expert in 0..num_experts {
        if crate::ggml_quants::dequant_matrix_row_into(raw, info, expert, &mut row[..emb_dim])
            .is_err()
        {
            return false;
        }
        out_logits[expert] = dot(input, &row[..emb_dim]);
    }
    true
}

/// Compute FTW router logits directly from raw weights without materializing a matrix.
pub(super) fn compute_router_logits_from_raw(
    raw: &[u8],
    ggml_type: u32,
    input: &[f32],
    emb_dim: usize,
    num_experts: usize,
    out_logits: &mut [f32],
) {
    let mut row_buf = [0.0f32; 8192];
    let row_len = emb_dim.min(row_buf.len());
    for e in 0..num_experts {
        let row_offset = match ggml_type {
            crate::ggml_quants::GGML_TYPE_F32 => e * emb_dim * 4,
            crate::ggml_quants::GGML_TYPE_F16 | crate::ggml_quants::GGML_TYPE_BF16 => {
                e * emb_dim * 2
            }
            crate::ggml_quants::GGML_TYPE_Q8_0 => e * (emb_dim / 32) * 34,
            _ => e * emb_dim * 2,
        };
        if row_offset < raw.len()
            && crate::ggml_quants::dequantize_row_into(
                &raw[row_offset..],
                ggml_type,
                row_len,
                &mut row_buf[..row_len],
            )
            .is_ok()
        {
            out_logits[e] = dot(input, &row_buf[..row_len]);
        } else {
            out_logits[e] = 0.0;
        }
    }
}

#[cfg(test)]
#[path = "moe_ffn_tests.rs"]
mod tests;
