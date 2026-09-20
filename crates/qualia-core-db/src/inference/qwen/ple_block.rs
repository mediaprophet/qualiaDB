//! Exact streamed Qwen4Exp PLE residual update.
//!
//! The trained PLE table stays on C: and contributes only sixteen selected
//! rows per token.  All projections and state remain caller-buffered; the
//! source trunk is read a row at a time from E:.

use crate::gguf_sharder::{GgufTensorInfo, LayerTensors, Qwen4ExpPleConfig};

use super::{
    group_rms_norm_in_place, group_rms_norm_into, PleNgramError, PleNvmeError, PleNvmeReader,
    PleTokenHistory, QwenNumericError, TrunkNvmeError, TrunkNvmeReader,
};

pub const QWEN4EXP_HYPER_STREAMS: usize = 4;
pub const QWEN4EXP_PLE_HISTORY: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PleBlockError {
    MissingTensor,
    ShapeMismatch,
    BufferTooSmall,
    Ngram(PleNgramError),
    Ple(PleNvmeError),
    Trunk(TrunkNvmeError),
    Numeric(QwenNumericError),
}

impl core::fmt::Display for PleBlockError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingTensor => write!(f, "Qwen4Exp PLE layer is missing a required tensor"),
            Self::ShapeMismatch => write!(
                f,
                "Qwen4Exp PLE tensor layout does not match its runtime contract"
            ),
            Self::BufferTooSmall => write!(f, "Qwen4Exp PLE caller buffer is too small"),
            Self::Ngram(error) => error.fmt(f),
            Self::Ple(error) => write!(f, "PLE NVMe reader error: {error:?}"),
            Self::Trunk(error) => error.fmt(f),
            Self::Numeric(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for PleBlockError {}

impl From<PleNgramError> for PleBlockError {
    fn from(value: PleNgramError) -> Self {
        Self::Ngram(value)
    }
}
impl From<PleNvmeError> for PleBlockError {
    fn from(value: PleNvmeError) -> Self {
        Self::Ple(value)
    }
}
impl From<TrunkNvmeError> for PleBlockError {
    fn from(value: TrunkNvmeError) -> Self {
        Self::Trunk(value)
    }
}
impl From<QwenNumericError> for PleBlockError {
    fn from(value: QwenNumericError) -> Self {
        Self::Numeric(value)
    }
}

/// All hot-path PLE storage is provided by the decode owner.  The block does
/// not allocate or retain any of these vectors.
pub struct PleBlockBuffers<'a> {
    pub raw_ple_row: &'a mut [u8],
    pub raw_trunk_row: &'a mut [u8],
    pub projection_row: &'a mut [f32],
    pub embedding: &'a mut [f32],
    pub key: &'a mut [f32],
    pub value: &'a mut [f32],
    pub query_norm: &'a mut [f32],
    pub gated: &'a mut [f32],
    pub conv_norm: &'a mut [f32],
    pub conv_weights: &'a mut [f32],
    pub norm_key: &'a mut [f32],
    pub norm_query: &'a mut [f32],
    pub norm_conv: &'a mut [f32],
}

fn required_tensor(tensor: Option<GgufTensorInfo>) -> Result<GgufTensorInfo, PleBlockError> {
    tensor.ok_or(PleBlockError::MissingTensor)
}

fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
}

fn signed_root(value: f32) -> f32 {
    value.abs().max(1.0e-6).sqrt().copysign(value)
}

fn vector_weight(
    reader: &mut TrunkNvmeReader,
    info: &GgufTensorInfo,
    out: &mut [f32],
    raw: &mut [u8],
) -> Result<(), PleBlockError> {
    if info.n_dims != 1 || info.dims[0] as usize > out.len() {
        return Err(PleBlockError::ShapeMismatch);
    }
    reader.read_row_into(info, 0, raw, out)?;
    Ok(())
}

/// Execute the trained PLE update inserted before the configured layer.
///
/// The residual has four contiguous hidden streams.  `history` is nine such
/// residual-width entries and is advanced only after all reads and arithmetic
/// complete successfully.
#[allow(clippy::too_many_arguments)]
pub fn execute_ple_block(
    config: &Qwen4ExpPleConfig,
    layer: &LayerTensors,
    token: u32,
    token_history: &mut PleTokenHistory,
    ple: &mut PleNvmeReader,
    trunk: &mut TrunkNvmeReader,
    residual: &mut [f32],
    history: &mut [f32],
    buffers: &mut PleBlockBuffers<'_>,
) -> Result<(), PleBlockError> {
    if !config.is_complete() || config.layer_count == 0 || layer.layer_idx != config.layers[0] {
        return Err(PleBlockError::ShapeMismatch);
    }
    let key_projection = required_tensor(layer.ple_key)?;
    let value_projection = required_tensor(layer.ple_value)?;
    let norm_key = required_tensor(layer.ple_norm_key)?;
    let norm_query = required_tensor(layer.ple_norm_query)?;
    let norm_conv = required_tensor(layer.ple_norm_conv)?;
    let convolution = required_tensor(layer.ple_conv1d)?;

    let rows = (config.ngram_size as usize - 1) * config.heads_per_ngram as usize;
    let hidden = rows
        .checked_mul(config.embedding_row_width as usize)
        .ok_or(PleBlockError::ShapeMismatch)?;
    let wide = hidden
        .checked_mul(QWEN4EXP_HYPER_STREAMS)
        .ok_or(PleBlockError::ShapeMismatch)?;
    if rows == 0
        || hidden == 0
        || residual.len() < wide
        || history.len() < QWEN4EXP_PLE_HISTORY * wide
        || buffers.embedding.len() < hidden
        || buffers.key.len() < wide
        || buffers.value.len() < hidden
        || buffers.query_norm.len() < wide
        || buffers.gated.len() < wide
        || buffers.conv_norm.len() < wide
        || buffers.conv_weights.len() < wide * 4
        || buffers.norm_key.len() < wide
        || buffers.norm_query.len() < wide
        || buffers.norm_conv.len() < wide
        || buffers.projection_row.len() < hidden
        || key_projection.n_dims != 2
        || key_projection.dims[0] as usize != hidden
        || key_projection.dims[1] as usize != wide
        || value_projection.n_dims != 2
        || value_projection.dims[0] as usize != hidden
        || value_projection.dims[1] as usize != hidden
        || convolution.n_dims != 2
        || convolution.dims[0] != 4
        || convolution.dims[1] as usize != wide
    {
        return Err(PleBlockError::BufferTooSmall);
    }

    let mut next_history = *token_history;
    let mut rows_out = [0u64; 32];
    let selected = next_history.select_and_push(config, token, ple.row_count(), &mut rows_out)?;
    if selected != rows {
        return Err(PleBlockError::ShapeMismatch);
    }
    ple.gather_rows_into(
        &rows_out[..rows],
        buffers.raw_ple_row,
        &mut buffers.embedding[..hidden],
    )?;
    trunk.gemv_rows_into(
        &key_projection,
        0,
        wide,
        &buffers.embedding[..hidden],
        &mut buffers.key[..wide],
        buffers.raw_trunk_row,
        &mut buffers.projection_row[..hidden],
    )?;
    trunk.gemv_rows_into(
        &value_projection,
        0,
        hidden,
        &buffers.embedding[..hidden],
        &mut buffers.value[..hidden],
        buffers.raw_trunk_row,
        &mut buffers.projection_row[..hidden],
    )?;
    vector_weight(
        trunk,
        &norm_key,
        &mut buffers.norm_key[..wide],
        buffers.raw_trunk_row,
    )?;
    vector_weight(
        trunk,
        &norm_query,
        &mut buffers.norm_query[..wide],
        buffers.raw_trunk_row,
    )?;
    vector_weight(
        trunk,
        &norm_conv,
        &mut buffers.norm_conv[..wide],
        buffers.raw_trunk_row,
    )?;
    for channel in 0..wide {
        trunk.read_row_into(
            &convolution,
            channel,
            buffers.raw_trunk_row,
            &mut buffers.conv_weights[channel * 4..channel * 4 + 4],
        )?;
    }
    group_rms_norm_in_place(
        &mut buffers.key[..wide],
        QWEN4EXP_HYPER_STREAMS,
        hidden,
        &buffers.norm_key[..wide],
        1.0e-6,
    )?;
    group_rms_norm_into(
        &residual[..wide],
        QWEN4EXP_HYPER_STREAMS,
        hidden,
        &buffers.norm_query[..wide],
        1.0e-6,
        &mut buffers.query_norm[..wide],
    )?;
    for stream in 0..QWEN4EXP_HYPER_STREAMS {
        let base = stream * hidden;
        let mut score = 0.0f32;
        for index in 0..hidden {
            score += buffers.key[base + index] * buffers.query_norm[base + index];
        }
        let gate = sigmoid(signed_root(score / (hidden as f32).sqrt()));
        for index in 0..hidden {
            buffers.gated[base + index] = buffers.value[index] * gate;
        }
    }
    group_rms_norm_into(
        &buffers.gated[..wide],
        QWEN4EXP_HYPER_STREAMS,
        hidden,
        &buffers.norm_conv[..wide],
        1.0e-6,
        &mut buffers.conv_norm[..wide],
    )?;
    for channel in 0..wide {
        let kernel = &buffers.conv_weights[channel * 4..channel * 4 + 4];
        let value = buffers.conv_norm[channel] * kernel[3]
            + history[channel] * kernel[0]
            + history[3 * wide + channel] * kernel[1]
            + history[6 * wide + channel] * kernel[2];
        residual[channel] += buffers.gated[channel] + value * sigmoid(value);
    }
    history.copy_within(wide..QWEN4EXP_PLE_HISTORY * wide, 0);
    history[(QWEN4EXP_PLE_HISTORY - 1) * wide..QWEN4EXP_PLE_HISTORY * wide]
        .copy_from_slice(&buffers.conv_norm[..wide]);
    *token_history = next_history;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_root_gate_preserves_score_direction() {
        assert!(signed_root(9.0) > 0.0);
        assert!(signed_root(-9.0) < 0.0);
        assert!(sigmoid(signed_root(9.0)) > 0.5);
        assert!(sigmoid(signed_root(-9.0)) < 0.5);
    }
}
