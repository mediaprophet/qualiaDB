//! Streamed Qwen4Exp sparse-attention token mixer.
//!
//! QSA layers are not interchangeable with the recurrent GatedDeltaNet
//! layers.  They retain a caller-owned K/V cache plus raw 128-wide indexer
//! keys.  On every decode step the indexer pools keys in four-token blocks,
//! rectifies four query-head dot products, and exposes only the selected
//! causal cells to grouped-query attention.

use crate::gguf_sharder::{GgufTensorInfo, LayerTensors};

use super::{TrunkNvmeError, TrunkNvmeReader};

/// QSA is 24×256 GQA, separate from the GatedDeltaNet 48×128 geometry.
pub const QWEN4EXP_QSA_HEAD_DIM: usize = 256;
pub const QWEN4EXP_QSA_QUERY_HEADS: usize = 24;
pub const QWEN4EXP_QSA_KV_HEADS: usize = 2;
/// The lightweight indexer remains four 128-wide heads.
pub const QWEN4EXP_QSA_INDEXER_HEAD_DIM: usize = 128;
pub const QWEN4EXP_QSA_INDEXER_HEADS: usize = 4;
pub const QWEN4EXP_QSA_COMPRESS_RATIO: usize = 4;
/// Trained indexer budget in TOKENS (`qwen4exp.attention.indexer.top_k` =
/// 2048): complete four-token blocks compete for `TOP_K / COMPRESS_RATIO`
/// slots and the members of the still-open trailing group are always visible.
pub const QWEN4EXP_QSA_TOP_K: usize = 2048;
/// Complete blocks selectable per step (`index_budget // index_ratio`).
pub const QWEN4EXP_QSA_TOP_BLOCKS: usize = QWEN4EXP_QSA_TOP_K / QWEN4EXP_QSA_COMPRESS_RATIO;
/// Qwen4Exp rotates only the first 64 dims of each attention/indexer head
/// (partial NeoX RoPE, `qwen4exp.rope.dimension_count`).
pub const QWEN4EXP_QSA_ROTARY_DIM: usize = 64;
/// Maximum tokens reachable per step: `token_topk` block members plus the
/// open group's causal tail (`ratio - 1`), matching `select_width`.
pub const QWEN4EXP_QSA_MAX_SELECTED: usize =
    QWEN4EXP_QSA_TOP_K + QWEN4EXP_QSA_COMPRESS_RATIO - 1;
const QUERY_WIDTH: usize = QWEN4EXP_QSA_QUERY_HEADS * QWEN4EXP_QSA_HEAD_DIM;
const KV_WIDTH: usize = QWEN4EXP_QSA_KV_HEADS * QWEN4EXP_QSA_HEAD_DIM;
const INDEX_QUERY_WIDTH: usize = QWEN4EXP_QSA_INDEXER_HEADS * QWEN4EXP_QSA_INDEXER_HEAD_DIM;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamedQsaError {
    NotSparseAttentionLayer,
    MissingTensor,
    ShapeMismatch,
    CacheFull,
    BufferTooSmall,
    Trunk(TrunkNvmeError),
}

impl core::fmt::Display for StreamedQsaError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotSparseAttentionLayer => write!(f, "Qwen4Exp layer is not a QSA layer"),
            Self::MissingTensor => write!(f, "Qwen4Exp QSA tensor is missing"),
            Self::ShapeMismatch => write!(f, "Qwen4Exp QSA tensor shape is invalid"),
            Self::CacheFull => write!(f, "Qwen4Exp QSA caller-owned cache is full"),
            Self::BufferTooSmall => write!(f, "Qwen4Exp QSA caller buffer is too small"),
            Self::Trunk(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for StreamedQsaError {}
impl From<TrunkNvmeError> for StreamedQsaError {
    fn from(value: TrunkNvmeError) -> Self {
        Self::Trunk(value)
    }
}

/// Persistent cache for one QSA layer.  Keys are RoPE-rotated before entry;
/// indexer keys deliberately remain raw because pooling precedes its norm and
/// positional rotation.  Storage is supplied by the runtime owner.
pub struct QsaState<'a> {
    pub keys: &'a mut [f32],
    pub values: &'a mut [f32],
    pub indexer_keys: &'a mut [f32],
    pub tokens: usize,
}

impl QsaState<'_> {
    pub fn capacity(&self) -> usize {
        self.keys.len() / KV_WIDTH
    }
}

/// Reused scratch for one QSA step.  `selected` and `scores` are bounded by
/// the caller-owned cache capacity; no logits or cache-sized heap object is
/// created by the operator.
pub struct StreamedQsaBuffers<'a> {
    pub raw_row: &'a mut [u8],
    pub projection_row: &'a mut [f32],
    pub q_full: &'a mut [f32],
    pub key: &'a mut [f32],
    pub value: &'a mut [f32],
    pub index_query: &'a mut [f32],
    pub index_key: &'a mut [f32],
    pub q_norm: &'a mut [f32],
    pub k_norm: &'a mut [f32],
    pub index_q_norm: &'a mut [f32],
    pub index_k_norm: &'a mut [f32],
    /// Expanded token list: all members of the selected four-token blocks.
    /// Capacity bound is `QWEN4EXP_QSA_MAX_SELECTED` (8192), not the cache.
    pub selected: &'a mut [usize],
    pub scores: &'a mut [f32],
    /// Block-level ranking scratch: `QWEN4EXP_QSA_TOP_K` entries each.
    pub block_selected: &'a mut [usize],
    pub block_scores: &'a mut [f32],
    pub attention: &'a mut [f32],
}

fn tensor(value: Option<GgufTensorInfo>) -> Result<GgufTensorInfo, StreamedQsaError> {
    value.ok_or(StreamedQsaError::MissingTensor)
}

fn is_vector(info: GgufTensorInfo, width: usize) -> bool {
    (info.n_dims == 1 && info.dims[0] as usize == width)
        || (info.n_dims == 2 && info.dims[0] as usize == width && info.dims[1] == 1)
}

fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
}

fn rms_norm_heads(vector: &mut [f32], weights: &[f32], heads: usize, head_dim: usize) {
    for head in 0..heads {
        let offset = head * head_dim;
        let part = &mut vector[offset..offset + head_dim];
        let mut sum = 0.0f32;
        for &value in part.iter() {
            sum += value * value;
        }
        let scale = 1.0 / (sum / head_dim as f32 + 1.0e-6).sqrt();
        // Qwen4Exp q/k/indexer norms are GemmaPlusOneRMSNorm in the source
        // checkpoint, but the GGUF converter pre-folds the +1 into the stored
        // gamma (llama.cpp build_norm): multiply the weight as-is.
        for index in 0..head_dim {
            part[index] *= scale * weights[index % weights.len()];
        }
    }
}

/// NeoX-style partial rotary: only the first `ROTARY_DIM` dims rotate, paired
/// as `(i, i + ROTARY_DIM/2)` with `freq = theta^(-2i/ROTARY_DIM)`.  The rest of
/// each head passes through unchanged, matching `freetoken.layers.rotary`.
fn rope_head(vector: &mut [f32], position: usize, theta: f32) {
    const HALF: usize = QWEN4EXP_QSA_ROTARY_DIM / 2;
    for pair in 0..HALF {
        let frequency = theta.powf(-(2.0 * pair as f32) / QWEN4EXP_QSA_ROTARY_DIM as f32);
        let angle = position as f32 * frequency;
        let (sin, cos) = angle.sin_cos();
        let first = vector[pair];
        let second = vector[pair + HALF];
        vector[pair] = first * cos - second * sin;
        vector[pair + HALF] = first * sin + second * cos;
    }
}

fn rope_heads(vector: &mut [f32], heads: usize, position: usize, theta: f32, head_dim: usize) {
    for head in 0..heads {
        let start = head * head_dim;
        rope_head(&mut vector[start..start + head_dim], position, theta);
    }
}

fn insert_ranked(
    index: usize,
    score: f32,
    selected: &mut [usize],
    scores: &mut [f32],
    used: &mut usize,
) {
    let capacity = selected.len().min(scores.len());
    if capacity == 0 {
        return;
    }
    let mut slot = *used;
    if *used < capacity {
        *used += 1;
    } else if score < scores[capacity - 1]
        || (score == scores[capacity - 1] && index > selected[capacity - 1])
    {
        return;
    } else {
        slot = capacity - 1;
    }
    while slot > 0
        && (score > scores[slot - 1] || (score == scores[slot - 1] && index < selected[slot - 1]))
    {
        if slot < capacity {
            scores[slot] = scores[slot - 1];
            selected[slot] = selected[slot - 1];
        }
        slot -= 1;
    }
    scores[slot] = score;
    selected[slot] = index;
}

/// Execute one causal QSA decode step.  `rope_theta` is explicit so the
/// bounded operator cannot silently invent a positional encoding policy; the
/// Qwen4Exp runtime passes the GGUF model value (10,000,000 for this model).
#[allow(clippy::too_many_arguments)]
pub fn execute_streamed_qsa(
    trunk: &mut TrunkNvmeReader,
    layer: &LayerTensors,
    input: &[f32],
    state: &mut QsaState<'_>,
    rope_theta: f32,
    out: &mut [f32],
    buffers: &mut StreamedQsaBuffers<'_>,
) -> Result<(), StreamedQsaError> {
    if !layer.has_qwen_sparse_attention() {
        return Err(StreamedQsaError::NotSparseAttentionLayer);
    }
    let q_weight = tensor(layer.attn_q)?;
    let k_weight = tensor(layer.attn_k)?;
    let v_weight = tensor(layer.attn_v)?;
    let output_weight = tensor(layer.attn_output)?;
    let q_norm_weight = tensor(layer.attn_q_norm)?;
    let k_norm_weight = tensor(layer.attn_k_norm)?;
    let index_q_weight = tensor(layer.indexer_q)?;
    let index_k_weight = tensor(layer.indexer_k)?;
    let index_q_norm_weight = tensor(layer.indexer_q_norm)?;
    let index_k_norm_weight = tensor(layer.indexer_k_norm)?;
    let hidden = input.len();
    let capacity = state.capacity();
    if hidden == 0
        || !rope_theta.is_finite()
        || rope_theta <= 1.0
        || q_weight.dims != [hidden as u64, (QUERY_WIDTH * 2) as u64, 0, 0]
        || k_weight.dims != [hidden as u64, KV_WIDTH as u64, 0, 0]
        || v_weight.dims != [hidden as u64, KV_WIDTH as u64, 0, 0]
        || output_weight.dims != [QUERY_WIDTH as u64, hidden as u64, 0, 0]
        || index_q_weight.dims != [hidden as u64, INDEX_QUERY_WIDTH as u64, 0, 0]
        || index_k_weight.dims != [hidden as u64, QWEN4EXP_QSA_INDEXER_HEAD_DIM as u64, 0, 0]
        || !is_vector(q_norm_weight, QWEN4EXP_QSA_HEAD_DIM)
        || !is_vector(k_norm_weight, QWEN4EXP_QSA_HEAD_DIM)
        || !is_vector(index_q_norm_weight, QWEN4EXP_QSA_INDEXER_HEAD_DIM)
        || !is_vector(index_k_norm_weight, QWEN4EXP_QSA_INDEXER_HEAD_DIM)
        || state.values.len() < capacity * KV_WIDTH
        || state.indexer_keys.len() < capacity * QWEN4EXP_QSA_INDEXER_HEAD_DIM
    {
        return Err(StreamedQsaError::ShapeMismatch);
    }
    if state.tokens >= capacity {
        return Err(StreamedQsaError::CacheFull);
    }
    let visible = state.tokens + 1;
    // Only CLOSED four-token groups are scored; the open trailing group's
    // members ride the causal tail into every attention step.
    let complete_blocks = visible / QWEN4EXP_QSA_COMPRESS_RATIO;
    let tail_start = complete_blocks * QWEN4EXP_QSA_COMPRESS_RATIO;
    let select_blocks = QWEN4EXP_QSA_TOP_BLOCKS.min(complete_blocks);
    let select_limit = select_blocks * QWEN4EXP_QSA_COMPRESS_RATIO + (visible - tail_start);
    if out.len() < hidden
        || buffers.projection_row.len() < QUERY_WIDTH
        || buffers.q_full.len() < QUERY_WIDTH * 2
        || buffers.key.len() < KV_WIDTH
        || buffers.value.len() < KV_WIDTH
        || buffers.index_query.len() < INDEX_QUERY_WIDTH
        || buffers.index_key.len() < QWEN4EXP_QSA_INDEXER_HEAD_DIM
        || buffers.q_norm.len() < QWEN4EXP_QSA_HEAD_DIM
        || buffers.k_norm.len() < QWEN4EXP_QSA_HEAD_DIM
        || buffers.index_q_norm.len() < QWEN4EXP_QSA_INDEXER_HEAD_DIM
        || buffers.index_k_norm.len() < QWEN4EXP_QSA_INDEXER_HEAD_DIM
        || buffers.block_selected.len() < select_blocks
        || buffers.block_scores.len() < select_blocks
        || buffers.selected.len() < select_limit
        || buffers.scores.len() < select_limit
        || buffers.attention.len() < QUERY_WIDTH
    {
        return Err(StreamedQsaError::BufferTooSmall);
    }

    trunk.read_row_into(
        &q_norm_weight,
        0,
        buffers.raw_row,
        &mut buffers.q_norm[..QWEN4EXP_QSA_HEAD_DIM],
    )?;
    trunk.read_row_into(
        &k_norm_weight,
        0,
        buffers.raw_row,
        &mut buffers.k_norm[..QWEN4EXP_QSA_HEAD_DIM],
    )?;
    trunk.read_row_into(
        &index_q_norm_weight,
        0,
        buffers.raw_row,
        &mut buffers.index_q_norm[..QWEN4EXP_QSA_INDEXER_HEAD_DIM],
    )?;
    trunk.read_row_into(
        &index_k_norm_weight,
        0,
        buffers.raw_row,
        &mut buffers.index_k_norm[..QWEN4EXP_QSA_INDEXER_HEAD_DIM],
    )?;
    trunk.gemv_rows_into(
        &q_weight,
        0,
        QUERY_WIDTH * 2,
        input,
        &mut buffers.q_full[..QUERY_WIDTH * 2],
        buffers.raw_row,
        &mut buffers.projection_row[..hidden],
    )?;
    trunk.gemv_rows_into(
        &k_weight,
        0,
        KV_WIDTH,
        input,
        &mut buffers.key[..KV_WIDTH],
        buffers.raw_row,
        &mut buffers.projection_row[..hidden],
    )?;
    trunk.gemv_rows_into(
        &v_weight,
        0,
        KV_WIDTH,
        input,
        &mut buffers.value[..KV_WIDTH],
        buffers.raw_row,
        &mut buffers.projection_row[..hidden],
    )?;
    trunk.gemv_rows_into(
        &index_q_weight,
        0,
        INDEX_QUERY_WIDTH,
        input,
        &mut buffers.index_query[..INDEX_QUERY_WIDTH],
        buffers.raw_row,
        &mut buffers.projection_row[..hidden],
    )?;
    trunk.gemv_rows_into(
        &index_k_weight,
        0,
        QWEN4EXP_QSA_INDEXER_HEAD_DIM,
        input,
        &mut buffers.index_key[..QWEN4EXP_QSA_INDEXER_HEAD_DIM],
        buffers.raw_row,
        &mut buffers.projection_row[..hidden],
    )?;

    // Indexer cache is raw.  The main K cache is normalized and positioned.
    let position = state.tokens;
    let idx_offset = position * QWEN4EXP_QSA_INDEXER_HEAD_DIM;
    state.indexer_keys[idx_offset..idx_offset + QWEN4EXP_QSA_INDEXER_HEAD_DIM]
        .copy_from_slice(&buffers.index_key[..QWEN4EXP_QSA_INDEXER_HEAD_DIM]);
    // `attn_q.weight` emits per-head [query | gate] pairs: head h's query sits
    // at q_full[h*512 .. h*512+256] and its gate at [h*512+256 .. h*512+512].
    // Norm and RoPE touch only the query half of each pair.
    for head in 0..QWEN4EXP_QSA_QUERY_HEADS {
        let start = head * 2 * QWEN4EXP_QSA_HEAD_DIM;
        rms_norm_heads(
            &mut buffers.q_full[start..start + QWEN4EXP_QSA_HEAD_DIM],
            &buffers.q_norm[..QWEN4EXP_QSA_HEAD_DIM],
            1,
            QWEN4EXP_QSA_HEAD_DIM,
        );
    }
    rms_norm_heads(
        &mut buffers.key[..KV_WIDTH],
        &buffers.k_norm[..QWEN4EXP_QSA_HEAD_DIM],
        QWEN4EXP_QSA_KV_HEADS,
        QWEN4EXP_QSA_HEAD_DIM,
    );
    rms_norm_heads(
        &mut buffers.index_query[..INDEX_QUERY_WIDTH],
        &buffers.index_q_norm[..QWEN4EXP_QSA_INDEXER_HEAD_DIM],
        QWEN4EXP_QSA_INDEXER_HEADS,
        QWEN4EXP_QSA_INDEXER_HEAD_DIM,
    );
    for head in 0..QWEN4EXP_QSA_QUERY_HEADS {
        let start = head * 2 * QWEN4EXP_QSA_HEAD_DIM;
        rope_head(
            &mut buffers.q_full[start..start + QWEN4EXP_QSA_HEAD_DIM],
            position,
            rope_theta,
        );
    }
    rope_heads(
        &mut buffers.key[..KV_WIDTH],
        QWEN4EXP_QSA_KV_HEADS,
        position,
        rope_theta,
        QWEN4EXP_QSA_HEAD_DIM,
    );
    rope_heads(
        &mut buffers.index_query[..INDEX_QUERY_WIDTH],
        QWEN4EXP_QSA_INDEXER_HEADS,
        position,
        rope_theta,
        QWEN4EXP_QSA_INDEXER_HEAD_DIM,
    );
    let kv_offset = position * KV_WIDTH;
    state.keys[kv_offset..kv_offset + KV_WIDTH].copy_from_slice(&buffers.key[..KV_WIDTH]);
    state.values[kv_offset..kv_offset + KV_WIDTH].copy_from_slice(&buffers.value[..KV_WIDTH]);

    // Match the reference: pool each CLOSED group's raw keys, then
    // normalise/rotate the block key at the group's first position; ReLU each
    // indexer head dot product before summing across heads.  Every member
    // token of a selected block becomes visible to GQA, and the open
    // trailing group's tokens are appended unconditionally.
    let mut chosen_blocks = 0usize;
    for block in 0..complete_blocks {
        let mut pooled = [0.0f32; QWEN4EXP_QSA_INDEXER_HEAD_DIM];
        for member in 0..QWEN4EXP_QSA_COMPRESS_RATIO {
            let offset = (block * QWEN4EXP_QSA_COMPRESS_RATIO + member)
                * QWEN4EXP_QSA_INDEXER_HEAD_DIM;
            for dim in 0..QWEN4EXP_QSA_INDEXER_HEAD_DIM {
                pooled[dim] += state.indexer_keys[offset + dim];
            }
        }
        for dim in 0..QWEN4EXP_QSA_INDEXER_HEAD_DIM {
            pooled[dim] *= 1.0 / QWEN4EXP_QSA_COMPRESS_RATIO as f32;
        }
        rms_norm_heads(
            &mut pooled,
            &buffers.index_k_norm[..QWEN4EXP_QSA_INDEXER_HEAD_DIM],
            1,
            QWEN4EXP_QSA_INDEXER_HEAD_DIM,
        );
        rope_head(
            &mut pooled,
            block * QWEN4EXP_QSA_COMPRESS_RATIO,
            rope_theta,
        );
        let mut score = 0.0f32;
        for head in 0..QWEN4EXP_QSA_INDEXER_HEADS {
            let mut dot = 0.0f32;
            let offset = head * QWEN4EXP_QSA_INDEXER_HEAD_DIM;
            for dim in 0..QWEN4EXP_QSA_INDEXER_HEAD_DIM {
                dot += buffers.index_query[offset + dim] * pooled[dim];
            }
            score += dot.max(0.0);
        }
        insert_ranked(
            block,
            score / (QWEN4EXP_QSA_INDEXER_HEAD_DIM as f32).sqrt(),
            buffers.block_selected,
            buffers.block_scores,
            &mut chosen_blocks,
        );
    }
    let mut chosen = 0usize;
    for rank in 0..chosen_blocks {
        let block = buffers.block_selected[rank];
        for member in 0..QWEN4EXP_QSA_COMPRESS_RATIO {
            buffers.selected[chosen] = block * QWEN4EXP_QSA_COMPRESS_RATIO + member;
            chosen += 1;
        }
    }
    for token in tail_start..visible {
        buffers.selected[chosen] = token;
        chosen += 1;
    }
    for value in &mut buffers.attention[..QUERY_WIDTH] {
        *value = 0.0;
    }
    // `selected` is ranked, but the softmax itself is order-independent.  The
    // stable ranking only governs ties and makes the sparse mask reproducible.
    for head in 0..QWEN4EXP_QSA_QUERY_HEADS {
        let q_offset = head * QWEN4EXP_QSA_HEAD_DIM;
        let q_source = head * 2 * QWEN4EXP_QSA_HEAD_DIM;
        let kv_head = head / (QWEN4EXP_QSA_QUERY_HEADS / QWEN4EXP_QSA_KV_HEADS);
        let kv_head_offset = kv_head * QWEN4EXP_QSA_HEAD_DIM;
        let mut max_score = f32::NEG_INFINITY;
        for rank in 0..chosen {
            let token_offset = buffers.selected[rank] * KV_WIDTH + kv_head_offset;
            let mut score = 0.0f32;
            for dim in 0..QWEN4EXP_QSA_HEAD_DIM {
                score += buffers.q_full[q_source + dim] * state.keys[token_offset + dim];
            }
            let scaled = score * (1.0 / (QWEN4EXP_QSA_HEAD_DIM as f32).sqrt());
            buffers.scores[rank] = scaled;
            max_score = max_score.max(scaled);
        }
        let mut denominator = 0.0f32;
        for rank in 0..chosen {
            let weight = (buffers.scores[rank] - max_score).exp();
            buffers.scores[rank] = weight;
            denominator += weight;
        }
        for rank in 0..chosen {
            let weight = buffers.scores[rank] / denominator;
            let token_offset = buffers.selected[rank] * KV_WIDTH + kv_head_offset;
            for dim in 0..QWEN4EXP_QSA_HEAD_DIM {
                buffers.attention[q_offset + dim] += weight * state.values[token_offset + dim];
            }
        }
        for dim in 0..QWEN4EXP_QSA_HEAD_DIM {
            buffers.attention[q_offset + dim] *=
                sigmoid(buffers.q_full[q_source + QWEN4EXP_QSA_HEAD_DIM + dim]);
        }
    }
    trunk.gemv_rows_into(
        &output_weight,
        0,
        hidden,
        &buffers.attention[..QUERY_WIDTH],
        out,
        buffers.raw_row,
        &mut buffers.projection_row[..QUERY_WIDTH],
    )?;
    state.tokens = visible;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rope_is_neox_partial_64() {
        // NeoX pairing: dims (i, i+32) rotate together; dims >= 64 untouched.
        let mut v = [0.0f32; 128];
        v[0] = 1.0;
        v[32] = 2.0;
        v[64] = 7.0;
        rope_head(&mut v, 1, 10_000.0);
        let (sin, cos) = 1.0f32.sin_cos();
        assert!((v[0] - (cos - 2.0 * sin)).abs() < 1.0e-5);
        assert!((v[32] - (sin + 2.0 * cos)).abs() < 1.0e-5);
        assert_eq!(v[64], 7.0);
        // Position zero is the identity.
        let mut w = [3.0f32; 128];
        rope_head(&mut w, 0, 10_000.0);
        assert_eq!(w[0], 3.0);
    }

    #[test]
    fn indexer_budget_is_token_level() {
        // `attention.indexer.top_k = 2048` counts TOKENS: 512 complete
        // four-token blocks plus the open group's causal tail.
        assert_eq!(QWEN4EXP_QSA_TOP_BLOCKS, 512);
        assert_eq!(QWEN4EXP_QSA_MAX_SELECTED, 2048 + 3);
    }

    #[test]
    fn ranked_selection_keeps_lowest_index_on_ties() {
        let mut selected = [usize::MAX; 3];
        let mut scores = [f32::NEG_INFINITY; 3];
        let mut used = 0;
        insert_ranked(4, 1.0, &mut selected, &mut scores, &mut used);
        insert_ranked(2, 1.0, &mut selected, &mut scores, &mut used);
        insert_ranked(3, 2.0, &mut selected, &mut scores, &mut used);
        assert_eq!(selected, [3, 2, 4]);
    }
}
