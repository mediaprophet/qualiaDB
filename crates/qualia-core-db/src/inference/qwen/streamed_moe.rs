//! Streamed Qwen4Exp mixture-of-experts execution.
//!
//! The Qwen4Exp expert tensors have the GGUF layout `[input, intermediate,
//! expert]`.  A decode step must never turn that into a mapped 512-expert
//! resident allocation: this module evaluates only the top-k planes selected
//! by the router and uses buffers owned by its caller.

use crate::gguf_sharder::LayerTensors;

use super::{
    QwenExpertTileError, QwenExpertTilePlaneKind, QwenExpertTileReader, TrunkNvmeError,
    TrunkNvmeReader,
};

pub const QWEN4EXP_EXPERT_COUNT: usize = 512;
pub const QWEN4EXP_TOP_EXPERTS: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamedMoeError {
    MissingTensor,
    InvalidShape,
    BufferTooSmall,
    Trunk(TrunkNvmeError),
    Tile(QwenExpertTileError),
    TileIdentityMismatch,
}

impl core::fmt::Display for StreamedMoeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingTensor => write!(f, "Qwen4Exp MoE tensor is missing"),
            Self::InvalidShape => write!(f, "Qwen4Exp MoE tensor shape is invalid"),
            Self::BufferTooSmall => write!(f, "Qwen4Exp MoE caller buffer is too small"),
            Self::Trunk(error) => error.fmt(f),
            Self::Tile(error) => error.fmt(f),
            Self::TileIdentityMismatch => write!(
                f,
                "cached Qwen expert tile does not match its routed layer/expert"
            ),
        }
    }
}

impl std::error::Error for StreamedMoeError {}

impl From<TrunkNvmeError> for StreamedMoeError {
    fn from(value: TrunkNvmeError) -> Self {
        Self::Trunk(value)
    }
}

impl From<QwenExpertTileError> for StreamedMoeError {
    fn from(value: QwenExpertTileError) -> Self {
        Self::Tile(value)
    }
}

/// Fixed top-k selection without a heap or an unstable sort.  Equal logits
/// prefer the lower expert number, keeping decode receipts reproducible.
pub fn select_top_experts(
    logits: &[f32],
    indices: &mut [usize; QWEN4EXP_TOP_EXPERTS],
    selected_logits: &mut [f32; QWEN4EXP_TOP_EXPERTS],
) -> Result<(), StreamedMoeError> {
    if logits.len() < QWEN4EXP_TOP_EXPERTS {
        return Err(StreamedMoeError::BufferTooSmall);
    }
    for slot in 0..QWEN4EXP_TOP_EXPERTS {
        indices[slot] = usize::MAX;
        selected_logits[slot] = f32::NEG_INFINITY;
    }
    for (expert, &logit) in logits.iter().enumerate() {
        for slot in 0..QWEN4EXP_TOP_EXPERTS {
            let better = logit > selected_logits[slot]
                || (logit == selected_logits[slot] && expert < indices[slot]);
            if better {
                for shift in (slot + 1..QWEN4EXP_TOP_EXPERTS).rev() {
                    indices[shift] = indices[shift - 1];
                    selected_logits[shift] = selected_logits[shift - 1];
                }
                indices[slot] = expert;
                selected_logits[slot] = logit;
                break;
            }
        }
    }
    Ok(())
}

/// Softmax only the selected top-k router scores.  The model routes through
/// these normalized coefficients, not through a sparse unnormalised gate.
pub fn normalize_top_experts(
    logits: &[f32; QWEN4EXP_TOP_EXPERTS],
    out: &mut [f32; QWEN4EXP_TOP_EXPERTS],
) {
    let mut max = f32::NEG_INFINITY;
    for &value in logits {
        if value > max {
            max = value;
        }
    }
    let mut sum = 0.0f32;
    for index in 0..QWEN4EXP_TOP_EXPERTS {
        let value = (logits[index] - max).exp();
        out[index] = value;
        sum += value;
    }
    if sum > 0.0 && sum.is_finite() {
        for value in out {
            *value /= sum;
        }
    } else {
        for value in out.iter_mut() {
            *value = 0.0;
        }
    }
}

fn swiglu_into(gate: &[f32], up: &[f32], out: &mut [f32]) -> Result<(), StreamedMoeError> {
    if gate.len() != up.len() || out.len() < gate.len() {
        return Err(StreamedMoeError::BufferTooSmall);
    }
    for index in 0..gate.len() {
        let value = gate[index];
        out[index] = (value / (1.0 + (-value).exp())) * up[index];
    }
    Ok(())
}

/// Source-only compatibility path. New native callers should use
/// [`execute_streamed_moe_with_tiles`] to supply any validated C: tiles.
#[allow(clippy::too_many_arguments)]
pub fn execute_streamed_moe(
    reader: &mut TrunkNvmeReader,
    tensors: &LayerTensors,
    input: &[f32],
    out: &mut [f32],
    router_logits: &mut [f32],
    top_indices: &mut [usize; QWEN4EXP_TOP_EXPERTS],
    top_logits: &mut [f32; QWEN4EXP_TOP_EXPERTS],
    top_weights: &mut [f32; QWEN4EXP_TOP_EXPERTS],
    gate: &mut [f32],
    up: &mut [f32],
    activation: &mut [f32],
    expert_out: &mut [f32],
    raw_row: &mut [u8],
    dequantized_row: &mut [f32],
) -> Result<(), StreamedMoeError> {
    execute_streamed_moe_with_tiles(
        reader,
        tensors,
        input,
        out,
        router_logits,
        top_indices,
        top_logits,
        top_weights,
        &[None; QWEN4EXP_TOP_EXPERTS],
        gate,
        up,
        activation,
        expert_out,
        raw_row,
        dequantized_row,
    )
}

/// Decode one Qwen4Exp MoE block directly from the source trunk.
///
/// `router_logits` is at least 512 elements. `gate`, `up`, and `activation`
/// are at least the expert intermediate width; `expert_out` and `out` are at
/// least the hidden width. `raw_row` and `dequantized_row` are reused for each
/// disk row. No expert tensor, temporary vector, or router sorting allocation
/// is created here.
#[allow(clippy::too_many_arguments)]
pub fn execute_streamed_moe_with_tiles(
    reader: &mut TrunkNvmeReader,
    tensors: &LayerTensors,
    input: &[f32],
    out: &mut [f32],
    router_logits: &mut [f32],
    top_indices: &mut [usize; QWEN4EXP_TOP_EXPERTS],
    top_logits: &mut [f32; QWEN4EXP_TOP_EXPERTS],
    top_weights: &mut [f32; QWEN4EXP_TOP_EXPERTS],
    cached_tiles: &[Option<&QwenExpertTileReader>; QWEN4EXP_TOP_EXPERTS],
    gate: &mut [f32],
    up: &mut [f32],
    activation: &mut [f32],
    expert_out: &mut [f32],
    raw_row: &mut [u8],
    dequantized_row: &mut [f32],
) -> Result<(), StreamedMoeError> {
    let router = tensors
        .moe_router
        .as_ref()
        .ok_or(StreamedMoeError::MissingTensor)?;
    let gate_experts = tensors
        .moe_gate_exps
        .as_ref()
        .ok_or(StreamedMoeError::MissingTensor)?;
    let up_experts = tensors
        .moe_up_exps
        .as_ref()
        .ok_or(StreamedMoeError::MissingTensor)?;
    let down_experts = tensors
        .moe_down_exps
        .as_ref()
        .ok_or(StreamedMoeError::MissingTensor)?;
    let shared_gate = tensors
        .moe_shared_gate
        .as_ref()
        .ok_or(StreamedMoeError::MissingTensor)?;
    let shared_up = tensors
        .moe_shared_up
        .as_ref()
        .ok_or(StreamedMoeError::MissingTensor)?;
    let shared_down = tensors
        .moe_shared_down
        .as_ref()
        .ok_or(StreamedMoeError::MissingTensor)?;
    let shared_gate_input = tensors
        .moe_shared_gate_input
        .as_ref()
        .ok_or(StreamedMoeError::MissingTensor)?;

    let hidden = input.len();
    let intermediate = gate_experts.dims[1] as usize;
    if hidden == 0
        || router.n_dims != 2
        || router.dims[0] as usize != hidden
        || router.dims[1] as usize != QWEN4EXP_EXPERT_COUNT
        || gate_experts.n_dims != 3
        || up_experts.n_dims != 3
        || down_experts.n_dims != 3
        || gate_experts.dims[0] as usize != hidden
        || up_experts.dims[0] as usize != hidden
        || gate_experts.dims[1] as usize != intermediate
        || up_experts.dims[1] as usize != intermediate
        || gate_experts.dims[2] as usize != QWEN4EXP_EXPERT_COUNT
        || up_experts.dims[2] as usize != QWEN4EXP_EXPERT_COUNT
        || down_experts.dims[0] as usize != intermediate
        || down_experts.dims[1] as usize != hidden
        || down_experts.dims[2] as usize != QWEN4EXP_EXPERT_COUNT
        || shared_gate.n_dims != 2
        || shared_up.n_dims != 2
        || shared_down.n_dims != 2
        || shared_gate.dims[0] as usize != hidden
        || shared_up.dims[0] as usize != hidden
        || shared_gate.dims[1] as usize != intermediate
        || shared_up.dims[1] as usize != intermediate
        || shared_down.dims[0] as usize != intermediate
        || shared_down.dims[1] as usize != hidden
        || shared_gate_input.dims[0] as usize != hidden
        || !((shared_gate_input.n_dims == 1)
            || (shared_gate_input.n_dims == 2 && shared_gate_input.dims[1] == 1))
    {
        return Err(StreamedMoeError::InvalidShape);
    }
    if out.len() < hidden
        || router_logits.len() < QWEN4EXP_EXPERT_COUNT
        || gate.len() < intermediate
        || up.len() < intermediate
        || activation.len() < intermediate
        || expert_out.len() < hidden
    {
        return Err(StreamedMoeError::BufferTooSmall);
    }

    reader.gemv_rows_into(
        router,
        0,
        QWEN4EXP_EXPERT_COUNT,
        input,
        router_logits,
        raw_row,
        dequantized_row,
    )?;
    select_top_experts(
        &router_logits[..QWEN4EXP_EXPERT_COUNT],
        top_indices,
        top_logits,
    )?;
    normalize_top_experts(top_logits, top_weights);
    for value in &mut out[..hidden] {
        *value = 0.0;
    }

    for route in 0..QWEN4EXP_TOP_EXPERTS {
        let expert = top_indices[route];
        if let Some(tile) = cached_tiles[route] {
            if tile.descriptor.layer != tensors.layer_idx as u16
                || tile.descriptor.expert != expert as u16
            {
                return Err(StreamedMoeError::TileIdentityMismatch);
            }
            tile.gemv_into(QwenExpertTilePlaneKind::Gate, input, gate, dequantized_row)?;
            tile.gemv_into(QwenExpertTilePlaneKind::Up, input, up, dequantized_row)?;
            swiglu_into(&gate[..intermediate], &up[..intermediate], activation)?;
            tile.gemv_into(
                QwenExpertTilePlaneKind::Down,
                &activation[..intermediate],
                expert_out,
                dequantized_row,
            )?;
        } else {
            reader.gemv_expert_plane_into(
                gate_experts,
                expert,
                input,
                gate,
                raw_row,
                dequantized_row,
            )?;
            reader.gemv_expert_plane_into(
                up_experts,
                expert,
                input,
                up,
                raw_row,
                dequantized_row,
            )?;
            swiglu_into(&gate[..intermediate], &up[..intermediate], activation)?;
            reader.gemv_expert_plane_into(
                down_experts,
                expert,
                &activation[..intermediate],
                expert_out,
                raw_row,
                dequantized_row,
            )?;
        }
        for index in 0..hidden {
            out[index] += top_weights[route] * expert_out[index];
        }
    }

    reader.gemv_rows_into(
        shared_gate,
        0,
        intermediate,
        input,
        gate,
        raw_row,
        dequantized_row,
    )?;
    reader.gemv_rows_into(
        shared_up,
        0,
        intermediate,
        input,
        up,
        raw_row,
        dequantized_row,
    )?;
    swiglu_into(&gate[..intermediate], &up[..intermediate], activation)?;
    reader.gemv_rows_into(
        shared_down,
        0,
        hidden,
        &activation[..intermediate],
        expert_out,
        raw_row,
        dequantized_row,
    )?;
    let mut shared_gate_logit = [0.0f32; 1];
    reader.gemv_rows_into(
        shared_gate_input,
        0,
        1,
        input,
        &mut shared_gate_logit,
        raw_row,
        dequantized_row,
    )?;
    let shared_scale = 1.0 / (1.0 + (-shared_gate_logit[0]).exp());
    for index in 0..hidden {
        out[index] += shared_scale * expert_out[index];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_experts_are_deterministic_and_normalized() {
        let mut indices = [0usize; QWEN4EXP_TOP_EXPERTS];
        let mut selected = [0.0f32; QWEN4EXP_TOP_EXPERTS];
        let mut logits = [0.0f32; QWEN4EXP_EXPERT_COUNT];
        logits[42] = 3.0;
        logits[7] = 3.0;
        logits[256] = 5.0;
        select_top_experts(&logits, &mut indices, &mut selected).unwrap();
        assert_eq!(&indices[..3], &[256, 7, 42]);
        let mut weights = [0.0f32; QWEN4EXP_TOP_EXPERTS];
        normalize_top_experts(&selected, &mut weights);
        let sum: f32 = weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn swiglu_uses_silu_gate() {
        let mut output = [0.0f32; 2];
        swiglu_into(&[0.0, 1.0], &[4.0, 2.0], &mut output).unwrap();
        assert_eq!(output[0], 0.0);
        assert!(output[1] > 1.4 && output[1] < 1.5);
    }
}
