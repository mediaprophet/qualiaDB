//! One complete streamed Qwen4Exp recurrent/MoE transformer layer.
//!
//! This is the executable bridge between the separately validated trained
//! Hyper-Connection, GatedDeltaNet, and routed MoE operators.  It deliberately
//! accepts only a GatedDeltaNet layer: QSA layers have a distinct KV/indexer
//! contract and must not be substituted with this recurrence.

use crate::gguf_sharder::LayerTensors;

use super::{
    execute_streamed_gated_delta, execute_streamed_hyper_connection, execute_streamed_moe,
    execute_streamed_qsa, inject_hyper_update, GatedDeltaBuffers, GatedDeltaError, GatedDeltaState,
    QsaState, StreamedHyperBuffers, StreamedHyperError, StreamedMoeError, StreamedQsaBuffers,
    StreamedQsaError, TrunkNvmeReader, QWEN4EXP_TOP_EXPERTS,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamedLayerError {
    NotGatedDeltaLayer,
    NotSparseAttentionLayer,
    BufferTooSmall,
    Hyper(StreamedHyperError),
    GatedDelta(GatedDeltaError),
    Qsa(StreamedQsaError),
    Moe(StreamedMoeError),
}

impl core::fmt::Display for StreamedLayerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotGatedDeltaLayer => {
                write!(f, "Qwen4Exp layer is not a complete GatedDeltaNet layer")
            }
            Self::NotSparseAttentionLayer => {
                write!(
                    f,
                    "Qwen4Exp layer is not a complete QSA sparse-attention layer"
                )
            }
            Self::BufferTooSmall => write!(f, "Qwen4Exp layer caller buffer is too small"),
            Self::Hyper(error) => error.fmt(f),
            Self::GatedDelta(error) => error.fmt(f),
            Self::Qsa(error) => error.fmt(f),
            Self::Moe(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for StreamedLayerError {}
impl From<StreamedHyperError> for StreamedLayerError {
    fn from(value: StreamedHyperError) -> Self {
        Self::Hyper(value)
    }
}
impl From<GatedDeltaError> for StreamedLayerError {
    fn from(value: GatedDeltaError) -> Self {
        Self::GatedDelta(value)
    }
}
impl From<StreamedQsaError> for StreamedLayerError {
    fn from(value: StreamedQsaError) -> Self {
        Self::Qsa(value)
    }
}
impl From<StreamedMoeError> for StreamedLayerError {
    fn from(value: StreamedMoeError) -> Self {
        Self::Moe(value)
    }
}

pub struct StreamedMoeBuffers<'a> {
    pub raw_row: &'a mut [u8],
    pub dequantized_row: &'a mut [f32],
    pub router_logits: &'a mut [f32],
    pub top_indices: &'a mut [usize; QWEN4EXP_TOP_EXPERTS],
    pub top_logits: &'a mut [f32; QWEN4EXP_TOP_EXPERTS],
    pub top_weights: &'a mut [f32; QWEN4EXP_TOP_EXPERTS],
    pub gate: &'a mut [f32],
    pub up: &'a mut [f32],
    pub activation: &'a mut [f32],
    pub expert_out: &'a mut [f32],
}

pub struct StreamedGdnMoeLayerBuffers<'a> {
    pub attn_hyper: StreamedHyperBuffers<'a>,
    pub ffn_hyper: StreamedHyperBuffers<'a>,
    pub gdn: GatedDeltaBuffers<'a>,
    pub moe: StreamedMoeBuffers<'a>,
    pub attn_mixed: &'a mut [f32],
    pub token_mixer_branch: &'a mut [f32],
    pub ffn_mixed: &'a mut [f32],
    pub moe_branch: &'a mut [f32],
    pub attn_inject: &'a mut [f32],
    pub ffn_inject: &'a mut [f32],
}

/// Scratch for the QSA/MoE layer.  The QSA state persists across decode
/// tokens while all buffers are caller-owned and reused.
pub struct StreamedQsaMoeLayerBuffers<'a> {
    pub attn_hyper: StreamedHyperBuffers<'a>,
    pub ffn_hyper: StreamedHyperBuffers<'a>,
    pub qsa: StreamedQsaBuffers<'a>,
    pub moe: StreamedMoeBuffers<'a>,
    pub attn_mixed: &'a mut [f32],
    pub token_mixer_branch: &'a mut [f32],
    pub ffn_mixed: &'a mut [f32],
    pub moe_branch: &'a mut [f32],
    pub attn_inject: &'a mut [f32],
    pub ffn_inject: &'a mut [f32],
}

/// Execute one non-QSA Qwen4Exp layer in trained order:
/// Hyper-Connection mix → GatedDeltaNet → attention inject → Hyper-Connection
/// mix → top-10 routed/shared MoE → FFN inject.
#[allow(clippy::too_many_arguments)]
pub fn execute_streamed_gdn_moe_layer(
    trunk: &mut TrunkNvmeReader,
    layer: &LayerTensors,
    residual: &mut [f32],
    gdn_state: &mut GatedDeltaState<'_>,
    buffers: &mut StreamedGdnMoeLayerBuffers<'_>,
) -> Result<(), StreamedLayerError> {
    if !layer.is_hybrid_ssm_layer() {
        return Err(StreamedLayerError::NotGatedDeltaLayer);
    }
    if residual.is_empty() || residual.len() % 4 != 0 {
        return Err(StreamedLayerError::BufferTooSmall);
    }
    let hidden = residual.len() / 4;
    if buffers.attn_mixed.len() < hidden
        || buffers.token_mixer_branch.len() < hidden
        || buffers.ffn_mixed.len() < hidden
        || buffers.moe_branch.len() < hidden
        || buffers.attn_inject.len() < 4
        || buffers.ffn_inject.len() < 4
    {
        return Err(StreamedLayerError::BufferTooSmall);
    }
    execute_streamed_hyper_connection(
        trunk,
        layer,
        false,
        residual,
        &mut buffers.attn_mixed[..hidden],
        Some(&mut buffers.attn_inject[..4]),
        &mut buffers.attn_hyper,
    )?;
    execute_streamed_gated_delta(
        trunk,
        layer,
        &buffers.attn_mixed[..hidden],
        gdn_state,
        &mut buffers.token_mixer_branch[..hidden],
        &mut buffers.gdn,
    )?;
    inject_hyper_update(
        residual,
        &buffers.token_mixer_branch[..hidden],
        &buffers.attn_inject[..4],
        4,
        hidden,
    )?;
    execute_streamed_hyper_connection(
        trunk,
        layer,
        true,
        residual,
        &mut buffers.ffn_mixed[..hidden],
        Some(&mut buffers.ffn_inject[..4]),
        &mut buffers.ffn_hyper,
    )?;
    execute_streamed_moe(
        trunk,
        layer,
        &buffers.ffn_mixed[..hidden],
        &mut buffers.moe_branch[..hidden],
        buffers.moe.router_logits,
        buffers.moe.top_indices,
        buffers.moe.top_logits,
        buffers.moe.top_weights,
        buffers.moe.gate,
        buffers.moe.up,
        buffers.moe.activation,
        buffers.moe.expert_out,
        buffers.moe.raw_row,
        buffers.moe.dequantized_row,
    )?;
    inject_hyper_update(
        residual,
        &buffers.moe_branch[..hidden],
        &buffers.ffn_inject[..4],
        4,
        hidden,
    )?;
    Ok(())
}

/// Execute one full QSA Qwen4Exp layer in trained order:
/// Hyper-Connection mix → sparse attention → attention inject →
/// Hyper-Connection mix → top-10 routed/shared MoE → FFN inject.
#[allow(clippy::too_many_arguments)]
pub fn execute_streamed_qsa_moe_layer(
    trunk: &mut TrunkNvmeReader,
    layer: &LayerTensors,
    residual: &mut [f32],
    qsa_state: &mut QsaState<'_>,
    rope_theta: f32,
    buffers: &mut StreamedQsaMoeLayerBuffers<'_>,
) -> Result<(), StreamedLayerError> {
    if !layer.has_qwen_sparse_attention() {
        return Err(StreamedLayerError::NotSparseAttentionLayer);
    }
    if residual.is_empty() || residual.len() % 4 != 0 {
        return Err(StreamedLayerError::BufferTooSmall);
    }
    let hidden = residual.len() / 4;
    if buffers.attn_mixed.len() < hidden
        || buffers.token_mixer_branch.len() < hidden
        || buffers.ffn_mixed.len() < hidden
        || buffers.moe_branch.len() < hidden
        || buffers.attn_inject.len() < 4
        || buffers.ffn_inject.len() < 4
    {
        return Err(StreamedLayerError::BufferTooSmall);
    }
    execute_streamed_hyper_connection(
        trunk,
        layer,
        false,
        residual,
        &mut buffers.attn_mixed[..hidden],
        Some(&mut buffers.attn_inject[..4]),
        &mut buffers.attn_hyper,
    )?;
    execute_streamed_qsa(
        trunk,
        layer,
        &buffers.attn_mixed[..hidden],
        qsa_state,
        rope_theta,
        &mut buffers.token_mixer_branch[..hidden],
        &mut buffers.qsa,
    )?;
    inject_hyper_update(
        residual,
        &buffers.token_mixer_branch[..hidden],
        &buffers.attn_inject[..4],
        4,
        hidden,
    )?;
    execute_streamed_hyper_connection(
        trunk,
        layer,
        true,
        residual,
        &mut buffers.ffn_mixed[..hidden],
        Some(&mut buffers.ffn_inject[..4]),
        &mut buffers.ffn_hyper,
    )?;
    execute_streamed_moe(
        trunk,
        layer,
        &buffers.ffn_mixed[..hidden],
        &mut buffers.moe_branch[..hidden],
        buffers.moe.router_logits,
        buffers.moe.top_indices,
        buffers.moe.top_logits,
        buffers.moe.top_weights,
        buffers.moe.gate,
        buffers.moe.up,
        buffers.moe.activation,
        buffers.moe.expert_out,
        buffers.moe.raw_row,
        buffers.moe.dequantized_row,
    )?;
    inject_hyper_update(
        residual,
        &buffers.moe_branch[..hidden],
        &buffers.ffn_inject[..4],
        4,
        hidden,
    )?;
    Ok(())
}
