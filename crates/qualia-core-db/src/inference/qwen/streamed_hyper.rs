//! Streamed trained Qwen4Exp four-stream Hyper-Connection.
//!
//! This executes the learned normalize → rank-320 gate → mix path without
//! making its C:/E: weight source resident. All vector storage belongs to the
//! caller so it can be reused by the token mixer and MoE stages.

use crate::gguf_sharder::{GgufTensorInfo, LayerTensors};

use super::{group_rms_norm_into, QwenNumericError, TrunkNvmeError, TrunkNvmeReader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamedHyperError {
    MissingTensor,
    ShapeMismatch,
    BufferTooSmall,
    Trunk(TrunkNvmeError),
    Numeric(QwenNumericError),
}

impl core::fmt::Display for StreamedHyperError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingTensor => write!(f, "Qwen4Exp Hyper-Connection tensor is missing"),
            Self::ShapeMismatch => write!(f, "Qwen4Exp Hyper-Connection tensor shape is invalid"),
            Self::BufferTooSmall => {
                write!(f, "Qwen4Exp Hyper-Connection caller buffer is too small")
            }
            Self::Trunk(error) => error.fmt(f),
            Self::Numeric(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for StreamedHyperError {}
impl From<TrunkNvmeError> for StreamedHyperError {
    fn from(value: TrunkNvmeError) -> Self {
        Self::Trunk(value)
    }
}
impl From<QwenNumericError> for StreamedHyperError {
    fn from(value: QwenNumericError) -> Self {
        Self::Numeric(value)
    }
}

pub struct StreamedHyperBuffers<'a> {
    pub raw_row: &'a mut [u8],
    pub projection_row: &'a mut [f32],
    pub norm_weight: &'a mut [f32],
    pub normalized: &'a mut [f32],
    pub low_rank: &'a mut [f32],
    pub gates: &'a mut [f32],
}

fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
}

fn tensor(value: Option<GgufTensorInfo>) -> Result<GgufTensorInfo, StreamedHyperError> {
    value.ok_or(StreamedHyperError::MissingTensor)
}

/// Form the hidden-width mixer input from the learned per-stream gates.
pub fn mix_hyper_streams(
    normalized: &[f32],
    gates: &[f32],
    streams: usize,
    hidden: usize,
    mixed: &mut [f32],
) -> Result<(), StreamedHyperError> {
    let wide = streams
        .checked_mul(hidden)
        .ok_or(StreamedHyperError::BufferTooSmall)?;
    if normalized.len() < wide || gates.len() < wide || mixed.len() < hidden {
        return Err(StreamedHyperError::BufferTooSmall);
    }
    for index in 0..hidden {
        let mut sum = 0.0f32;
        for stream in 0..streams {
            let offset = stream * hidden + index;
            sum += normalized[offset] * sigmoid(gates[offset]);
        }
        mixed[index] = sum / streams as f32;
    }
    Ok(())
}

/// Scatter a token-mixer/MoE branch back into each residual stream.  Qwen4Exp
/// uses one learned inject scalar per stream, divided by the stream count.
pub fn inject_hyper_update(
    residual: &mut [f32],
    branch: &[f32],
    inject: &[f32],
    streams: usize,
    hidden: usize,
) -> Result<(), StreamedHyperError> {
    let wide = streams
        .checked_mul(hidden)
        .ok_or(StreamedHyperError::BufferTooSmall)?;
    if residual.len() < wide || branch.len() < hidden || inject.len() < streams {
        return Err(StreamedHyperError::BufferTooSmall);
    }
    for stream in 0..streams {
        let scale = 2.0 * sigmoid(inject[stream] / streams as f32);
        let offset = stream * hidden;
        for index in 0..hidden {
            residual[offset + index] += branch[index] * scale;
        }
    }
    Ok(())
}

/// Stream one attention or FFN Hyper-Connection mix from the trunk.
#[allow(clippy::too_many_arguments)]
pub fn execute_streamed_hyper_connection(
    trunk: &mut TrunkNvmeReader,
    layer: &LayerTensors,
    ffn: bool,
    residual: &[f32],
    mixed: &mut [f32],
    inject: Option<&mut [f32]>,
    buffers: &mut StreamedHyperBuffers<'_>,
) -> Result<(), StreamedHyperError> {
    let (norm, down, up, inject_weight) = if ffn {
        (
            tensor(layer.hc_ffn_norm)?,
            tensor(layer.hc_ffn_down)?,
            tensor(layer.hc_ffn_up)?,
            tensor(layer.hc_ffn_inject)?,
        )
    } else {
        (
            tensor(layer.hc_attn_norm)?,
            tensor(layer.hc_attn_down)?,
            tensor(layer.hc_attn_up)?,
            tensor(layer.hc_attn_inject)?,
        )
    };
    let wide = residual.len();
    if wide == 0 || wide % 4 != 0 || norm.n_dims != 1 || norm.dims[0] as usize != wide {
        return Err(StreamedHyperError::ShapeMismatch);
    }
    let streams = 4usize;
    let hidden = wide / streams;
    let rank = down.dims[1] as usize;
    if down.n_dims != 2
        || up.n_dims != 2
        || inject_weight.n_dims != 2
        || down.dims[0] as usize != wide
        || up.dims[0] as usize != rank
        || up.dims[1] as usize != wide
        || inject_weight.dims[0] as usize != wide
        || inject_weight.dims[1] as usize != streams
        || buffers.norm_weight.len() < wide
        || buffers.normalized.len() < wide
        || buffers.gates.len() < wide
        || buffers.low_rank.len() < rank
        || buffers.projection_row.len() < wide
        || mixed.len() < hidden
    {
        return Err(StreamedHyperError::BufferTooSmall);
    }
    trunk.read_row_into(&norm, 0, buffers.raw_row, &mut buffers.norm_weight[..wide])?;
    group_rms_norm_into(
        residual,
        streams,
        hidden,
        &buffers.norm_weight[..wide],
        1.0e-6,
        &mut buffers.normalized[..wide],
    )?;
    trunk.gemv_rows_into(
        &down,
        0,
        rank,
        &buffers.normalized[..wide],
        &mut buffers.low_rank[..rank],
        buffers.raw_row,
        &mut buffers.projection_row[..wide],
    )?;
    for value in &mut buffers.low_rank[..rank] {
        let scaled = *value / streams as f32;
        *value = scaled * sigmoid(scaled);
    }
    trunk.gemv_rows_into(
        &up,
        0,
        wide,
        &buffers.low_rank[..rank],
        &mut buffers.gates[..wide],
        buffers.raw_row,
        &mut buffers.projection_row[..rank],
    )?;
    mix_hyper_streams(
        &buffers.normalized[..wide],
        &buffers.gates[..wide],
        streams,
        hidden,
        mixed,
    )?;
    if let Some(inject_out) = inject {
        if inject_out.len() < streams {
            return Err(StreamedHyperError::BufferTooSmall);
        }
        trunk.gemv_rows_into(
            &inject_weight,
            0,
            streams,
            &buffers.normalized[..wide],
            &mut inject_out[..streams],
            buffers.raw_row,
            &mut buffers.projection_row[..wide],
        )?;
    }
    Ok(())
}

/// Execute Qwen4Exp's final global Hyper-Connection mixer.  Unlike a layer
/// mixer it has no injection projection: the collapsed hidden vector feeds
/// the vocabulary projection directly in place of an `output_norm` tensor.
#[allow(clippy::too_many_arguments)]
pub fn execute_streamed_final_hyper_connection(
    trunk: &mut TrunkNvmeReader,
    norm: GgufTensorInfo,
    down: GgufTensorInfo,
    up: GgufTensorInfo,
    residual: &[f32],
    mixed: &mut [f32],
    buffers: &mut StreamedHyperBuffers<'_>,
) -> Result<(), StreamedHyperError> {
    let wide = residual.len();
    if wide == 0 || wide % 4 != 0 || norm.n_dims != 1 || norm.dims[0] as usize != wide {
        return Err(StreamedHyperError::ShapeMismatch);
    }
    let streams = 4usize;
    let hidden = wide / streams;
    let rank = down.dims[1] as usize;
    if down.n_dims != 2
        || up.n_dims != 2
        || down.dims[0] as usize != wide
        || up.dims[0] as usize != rank
        || up.dims[1] as usize != wide
        || buffers.norm_weight.len() < wide
        || buffers.normalized.len() < wide
        || buffers.gates.len() < wide
        || buffers.low_rank.len() < rank
        || buffers.projection_row.len() < wide
        || mixed.len() < hidden
    {
        return Err(StreamedHyperError::BufferTooSmall);
    }
    trunk.read_row_into(&norm, 0, buffers.raw_row, &mut buffers.norm_weight[..wide])?;
    group_rms_norm_into(
        residual,
        streams,
        hidden,
        &buffers.norm_weight[..wide],
        1.0e-6,
        &mut buffers.normalized[..wide],
    )?;
    trunk.gemv_rows_into(
        &down,
        0,
        rank,
        &buffers.normalized[..wide],
        &mut buffers.low_rank[..rank],
        buffers.raw_row,
        &mut buffers.projection_row[..wide],
    )?;
    for value in &mut buffers.low_rank[..rank] {
        let scaled = *value / streams as f32;
        *value = scaled * sigmoid(scaled);
    }
    trunk.gemv_rows_into(
        &up,
        0,
        wide,
        &buffers.low_rank[..rank],
        &mut buffers.gates[..wide],
        buffers.raw_row,
        &mut buffers.projection_row[..rank],
    )?;
    mix_hyper_streams(
        &buffers.normalized[..wide],
        &buffers.gates[..wide],
        streams,
        hidden,
        mixed,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learned_inject_uses_one_gate_per_stream() {
        let mut residual = [0.0f32; 4];
        inject_hyper_update(&mut residual, &[2.0], &[0.0; 4], 4, 1).unwrap();
        assert_eq!(residual, [2.0; 4]);
    }
}
