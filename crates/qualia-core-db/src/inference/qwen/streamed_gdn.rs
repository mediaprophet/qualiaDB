//! Streamed trained Qwen4Exp GatedDeltaNet token mixer.
//!
//! Qwen4Exp has 36 recurrent token-mixer layers.  Their 10,240-channel
//! causal convolution and 48 independent 128×128 delta-rule states are
//! caller-owned state, while every learned projection is streamed row-wise
//! from the immutable trunk.  That prevents either the source GGUF or a
//! whole layer's weights from becoming an accidental resident allocation.

use crate::gguf_sharder::{GgufTensorInfo, LayerTensors};

use super::{TrunkNvmeError, TrunkNvmeReader};

pub const QWEN4EXP_GDN_HEADS: usize = 48;
pub const QWEN4EXP_GDN_KEY_HEADS: usize = 16;
pub const QWEN4EXP_GDN_HEAD_DIM: usize = 128;
// Qwen4Exp uses 16 query heads, 16 key heads, and 48 value heads.  The
// value-head count is deliberately not used for the first Q slice.
const QKV_WIDTH: usize =
    QWEN4EXP_GDN_KEY_HEADS * QWEN4EXP_GDN_HEAD_DIM * 2 + QWEN4EXP_GDN_HEADS * QWEN4EXP_GDN_HEAD_DIM;
const VALUE_WIDTH: usize = QWEN4EXP_GDN_HEADS * QWEN4EXP_GDN_HEAD_DIM;
const CONV_HISTORY: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatedDeltaError {
    MissingTensor,
    ShapeMismatch,
    BufferTooSmall,
    Trunk(TrunkNvmeError),
}

impl core::fmt::Display for GatedDeltaError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingTensor => write!(f, "Qwen4Exp GatedDeltaNet tensor is missing"),
            Self::ShapeMismatch => write!(f, "Qwen4Exp GatedDeltaNet tensor shape is invalid"),
            Self::BufferTooSmall => write!(f, "Qwen4Exp GatedDeltaNet caller buffer is too small"),
            Self::Trunk(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for GatedDeltaError {}
impl From<TrunkNvmeError> for GatedDeltaError {
    fn from(value: TrunkNvmeError) -> Self {
        Self::Trunk(value)
    }
}

/// Persistent recurrence state for one GatedDeltaNet layer.  The runtime
/// owner supplies these slices explicitly; no evaluator allocation occurs.
pub struct GatedDeltaState<'a> {
    /// Three previous unactivated QKV values for each QKV channel.
    pub convolution: &'a mut [f32],
    /// `[value_head][value_channel][key_channel]` delta-rule state.
    pub delta: &'a mut [f32],
}

/// Scratch buffers reused for every GatedDeltaNet decode step.
pub struct GatedDeltaBuffers<'a> {
    pub raw_row: &'a mut [u8],
    /// Must hold the widest source tensor row (the 6,144-wide output input).
    pub projection_row: &'a mut [f32],
    pub qkv: &'a mut [f32],
    pub gate: &'a mut [f32],
    pub alpha: &'a mut [f32],
    pub beta: &'a mut [f32],
    pub convolved: &'a mut [f32],
    pub head_norm: &'a mut [f32],
    pub head_vector: &'a mut [f32],
    pub output_inner: &'a mut [f32],
    pub head_a: &'a mut [f32],
    pub dt_bias: &'a mut [f32],
}

fn tensor(value: Option<GgufTensorInfo>) -> Result<GgufTensorInfo, GatedDeltaError> {
    value.ok_or(GatedDeltaError::MissingTensor)
}

fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
}

fn softplus(value: f32) -> f32 {
    if value > 20.0 {
        value
    } else if value < -20.0 {
        value.exp()
    } else {
        value.exp().ln_1p()
    }
}

fn normalize_l2(vector: &mut [f32]) {
    let mut squared = 0.0f32;
    for &value in vector.iter() {
        squared += value * value;
    }
    let scale = 1.0 / (squared + 1.0e-6).sqrt();
    for value in vector {
        *value *= scale;
    }
}

fn tensor_is_vector(info: GgufTensorInfo, width: usize) -> bool {
    (info.n_dims == 1 && info.dims[0] as usize == width)
        || (info.n_dims == 2 && info.dims[0] as usize == width && info.dims[1] == 1)
}

/// Execute one trained Qwen4Exp GatedDeltaNet recurrence.
///
/// The delta state update is the model's column-wise gated delta rule:
/// `S <- decay*S + key*(value - Sᵀ*key)*beta`.  Query/key L2 normalization,
/// learned per-head decay, causal convolution, output RMS norm, and the
/// learned gate all run before the streamed output projection.
#[allow(clippy::too_many_arguments)]
pub fn execute_streamed_gated_delta(
    trunk: &mut TrunkNvmeReader,
    layer: &LayerTensors,
    input: &[f32],
    state: &mut GatedDeltaState<'_>,
    out: &mut [f32],
    buffers: &mut GatedDeltaBuffers<'_>,
) -> Result<(), GatedDeltaError> {
    let qkv_weight = tensor(layer.attn_qkv)?;
    let gate_weight = tensor(layer.attn_gate)?;
    let alpha_weight = tensor(layer.ssm_alpha)?;
    let beta_weight = tensor(layer.ssm_beta)?;
    let conv_weight = tensor(layer.ssm_conv1d)?;
    // This checkpoint stores its depthwise convolution without a bias.  Keep
    // the optional path for converter variants that expose one, but absence is
    // a learned graph property—not a reason to invent a zero tensor in the
    // source contract.
    let conv_bias = layer.ssm_conv1d_bias;
    let dt_bias_weight = tensor(layer.ssm_dt_bias)?;
    let head_a_weight = tensor(layer.ssm_a)?;
    let norm_weight = tensor(layer.ssm_norm)?;
    let output_weight = tensor(layer.ssm_out)?;
    let hidden = input.len();

    if hidden == 0
        || qkv_weight.n_dims != 2
        || qkv_weight.dims[0] as usize != hidden
        || qkv_weight.dims[1] as usize != QKV_WIDTH
        || gate_weight.n_dims != 2
        || gate_weight.dims[0] as usize != hidden
        || gate_weight.dims[1] as usize != VALUE_WIDTH
        || alpha_weight.n_dims != 2
        || beta_weight.n_dims != 2
        || alpha_weight.dims != [hidden as u64, QWEN4EXP_GDN_HEADS as u64, 0, 0]
        || beta_weight.dims != [hidden as u64, QWEN4EXP_GDN_HEADS as u64, 0, 0]
        || conv_weight.n_dims != 2
        || conv_weight.dims[0] as usize != 4
        || conv_weight.dims[1] as usize != QKV_WIDTH
        || conv_bias.is_some_and(|info| !tensor_is_vector(info, QKV_WIDTH))
        || !tensor_is_vector(dt_bias_weight, QWEN4EXP_GDN_HEADS)
        || !tensor_is_vector(head_a_weight, QWEN4EXP_GDN_HEADS)
        || !tensor_is_vector(norm_weight, QWEN4EXP_GDN_HEAD_DIM)
        || output_weight.n_dims != 2
        || output_weight.dims[0] as usize != VALUE_WIDTH
        || output_weight.dims[1] as usize != hidden
    {
        return Err(GatedDeltaError::ShapeMismatch);
    }
    if out.len() < hidden
        || state.convolution.len() < QKV_WIDTH * CONV_HISTORY
        || state.delta.len() < QWEN4EXP_GDN_HEADS * QWEN4EXP_GDN_HEAD_DIM * QWEN4EXP_GDN_HEAD_DIM
        || buffers.projection_row.len() < VALUE_WIDTH
        || buffers.qkv.len() < QKV_WIDTH
        || buffers.gate.len() < VALUE_WIDTH
        || buffers.alpha.len() < QWEN4EXP_GDN_HEADS
        || buffers.beta.len() < QWEN4EXP_GDN_HEADS
        || buffers.convolved.len() < QKV_WIDTH
        || buffers.head_norm.len() < QWEN4EXP_GDN_HEAD_DIM
        || buffers.head_vector.len() < QWEN4EXP_GDN_HEAD_DIM
        || buffers.output_inner.len() < VALUE_WIDTH
        || buffers.head_a.len() < QWEN4EXP_GDN_HEADS
        || buffers.dt_bias.len() < QWEN4EXP_GDN_HEADS
    {
        return Err(GatedDeltaError::BufferTooSmall);
    }

    trunk.gemv_rows_into(
        &qkv_weight,
        0,
        QKV_WIDTH,
        input,
        &mut buffers.qkv[..QKV_WIDTH],
        buffers.raw_row,
        &mut buffers.projection_row[..hidden],
    )?;
    trunk.gemv_rows_into(
        &gate_weight,
        0,
        VALUE_WIDTH,
        input,
        &mut buffers.gate[..VALUE_WIDTH],
        buffers.raw_row,
        &mut buffers.projection_row[..hidden],
    )?;
    trunk.gemv_rows_into(
        &alpha_weight,
        0,
        QWEN4EXP_GDN_HEADS,
        input,
        &mut buffers.alpha[..QWEN4EXP_GDN_HEADS],
        buffers.raw_row,
        &mut buffers.projection_row[..hidden],
    )?;
    trunk.gemv_rows_into(
        &beta_weight,
        0,
        QWEN4EXP_GDN_HEADS,
        input,
        &mut buffers.beta[..QWEN4EXP_GDN_HEADS],
        buffers.raw_row,
        &mut buffers.projection_row[..hidden],
    )?;
    for value in &mut buffers.convolved[..QKV_WIDTH] {
        *value = 0.0;
    }
    if let Some(conv_bias) = conv_bias {
        trunk.read_row_into(
            &conv_bias,
            0,
            buffers.raw_row,
            &mut buffers.convolved[..QKV_WIDTH],
        )?;
    }
    trunk.read_row_into(
        &head_a_weight,
        0,
        buffers.raw_row,
        &mut buffers.head_a[..QWEN4EXP_GDN_HEADS],
    )?;
    trunk.read_row_into(
        &dt_bias_weight,
        0,
        buffers.raw_row,
        &mut buffers.dt_bias[..QWEN4EXP_GDN_HEADS],
    )?;
    trunk.read_row_into(
        &norm_weight,
        0,
        buffers.raw_row,
        &mut buffers.head_norm[..QWEN4EXP_GDN_HEAD_DIM],
    )?;

    // The source tensor stores one 4-tap depthwise kernel per QKV channel.
    // Shift history only after consuming it, so position zero starts from an
    // all-zero causal state exactly as the trained recurrence expects.
    for channel in 0..QKV_WIDTH {
        trunk.read_row_into(
            &conv_weight,
            channel,
            buffers.raw_row,
            &mut buffers.head_vector[..4],
        )?;
        let history = channel * CONV_HISTORY;
        let raw = buffers.qkv[channel];
        let value = buffers.head_vector[0] * raw
            + buffers.head_vector[1] * state.convolution[history]
            + buffers.head_vector[2] * state.convolution[history + 1]
            + buffers.head_vector[3] * state.convolution[history + 2]
            + buffers.convolved[channel];
        state.convolution[history + 2] = state.convolution[history + 1];
        state.convolution[history + 1] = state.convolution[history];
        state.convolution[history] = raw;
        buffers.convolved[channel] = value / (1.0 + (-value).exp());
    }

    // The source layout is `[query(16), key(16), value(48)]`; query starts at
    // zero, then key follows its 2,048 values, then the 6,144 value channels.
    let query_offset = 0;
    let key_offset = query_offset + QWEN4EXP_GDN_KEY_HEADS * QWEN4EXP_GDN_HEAD_DIM;
    let value_offset = key_offset + QWEN4EXP_GDN_KEY_HEADS * QWEN4EXP_GDN_HEAD_DIM;
    for key_head in 0..QWEN4EXP_GDN_KEY_HEADS {
        let start = key_offset + key_head * QWEN4EXP_GDN_HEAD_DIM;
        normalize_l2(&mut buffers.convolved[start..start + QWEN4EXP_GDN_HEAD_DIM]);
    }
    for query_head in 0..QWEN4EXP_GDN_KEY_HEADS {
        let start = query_offset + query_head * QWEN4EXP_GDN_HEAD_DIM;
        normalize_l2(&mut buffers.convolved[start..start + QWEN4EXP_GDN_HEAD_DIM]);
    }

    for value_head in 0..QWEN4EXP_GDN_HEADS {
        let key_head = value_head % QWEN4EXP_GDN_KEY_HEADS;
        let query_start = query_offset + key_head * QWEN4EXP_GDN_HEAD_DIM;
        let key_start = key_offset + key_head * QWEN4EXP_GDN_HEAD_DIM;
        let value_start = value_offset + value_head * QWEN4EXP_GDN_HEAD_DIM;
        let decay = (buffers.head_a[value_head]
            * softplus(buffers.alpha[value_head] + buffers.dt_bias[value_head]))
        .exp();
        let beta = sigmoid(buffers.beta[value_head]);
        let state_head = value_head * QWEN4EXP_GDN_HEAD_DIM * QWEN4EXP_GDN_HEAD_DIM;
        for column in 0..QWEN4EXP_GDN_HEAD_DIM {
            let column_state = state_head + column * QWEN4EXP_GDN_HEAD_DIM;
            let mut prediction = 0.0f32;
            for row in 0..QWEN4EXP_GDN_HEAD_DIM {
                prediction += state.delta[column_state + row] * buffers.convolved[key_start + row];
            }
            let change = (buffers.convolved[value_start + column] - prediction) * beta;
            let mut response = 0.0f32;
            for row in 0..QWEN4EXP_GDN_HEAD_DIM {
                let updated = decay * state.delta[column_state + row]
                    + buffers.convolved[key_start + row] * change;
                state.delta[column_state + row] = updated;
                response += updated * buffers.convolved[query_start + row];
            }
            buffers.output_inner[value_head * QWEN4EXP_GDN_HEAD_DIM + column] =
                response * (1.0 / (QWEN4EXP_GDN_HEAD_DIM as f32).sqrt());
        }
        let output_start = value_head * QWEN4EXP_GDN_HEAD_DIM;
        let mut mean_square = 0.0f32;
        for offset in 0..QWEN4EXP_GDN_HEAD_DIM {
            let value = buffers.output_inner[output_start + offset];
            mean_square += value * value;
        }
        let scale = 1.0 / (mean_square / QWEN4EXP_GDN_HEAD_DIM as f32 + 1.0e-6).sqrt();
        for offset in 0..QWEN4EXP_GDN_HEAD_DIM {
            let index = output_start + offset;
            buffers.output_inner[index] *=
                scale * buffers.head_norm[offset] * sigmoid(buffers.gate[index]);
        }
    }
    trunk.gemv_rows_into(
        &output_weight,
        0,
        hidden,
        &buffers.output_inner[..VALUE_WIDTH],
        out,
        buffers.raw_row,
        &mut buffers.projection_row[..VALUE_WIDTH],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l2_normalization_has_unit_length() {
        let mut vector = [3.0f32, 4.0];
        normalize_l2(&mut vector);
        let norm = vector[0] * vector[0] + vector[1] * vector[1];
        assert!((norm - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn softplus_is_stable_at_extremes() {
        assert!(softplus(-100.0) >= 0.0);
        assert!((softplus(30.0) - 30.0).abs() < 1.0e-4);
    }
}
