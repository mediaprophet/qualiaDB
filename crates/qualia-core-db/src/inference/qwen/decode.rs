//! Persistent Qwen4Exp decode session: per-layer recurrent state and the
//! complete 48-layer token step.
//!
//! A session owns every piece of mutable state the trained graph needs —
//! 36 GatedDeltaNet convolutions and delta-rule matrices, 12 QSA K/V and
//! raw indexer caches, the PLE n-gram context and dilated-conv history, and
//! the four-stream residual.  All weight bytes are streamed row-wise from
//! the E: trunk and C: PLE payload; nothing here maps or stages the model.

use std::time::Instant;

use crate::gguf_sharder::GgufTensorIndex;

use super::{
    execute_ple_block, execute_streamed_final_hyper_connection,
    execute_streamed_gdn_moe_layer, execute_streamed_qsa_moe_layer, GatedDeltaBuffers,
    GatedDeltaError, GatedDeltaState, PleBlockBuffers, PleBlockError, PleTokenHistory, QsaState,
    Qwen4ExpNativeRuntime, StreamedArgmax, StreamedGdnMoeLayerBuffers, StreamedHyperBuffers,
    StreamedHyperError, StreamedLayerError, StreamedMoeBuffers, StreamedQsaBuffers,
    StreamedQsaMoeLayerBuffers, TrunkNvmeError, QWEN4EXP_GDN_HEAD_DIM, QWEN4EXP_GDN_HEADS,
    QWEN4EXP_GDN_KEY_HEADS, QWEN4EXP_HYPER_STREAMS, QWEN4EXP_PLE_HISTORY,
    QWEN4EXP_QSA_HEAD_DIM, QWEN4EXP_QSA_INDEXER_HEAD_DIM, QWEN4EXP_QSA_INDEXER_HEADS,
    QWEN4EXP_QSA_KV_HEADS, QWEN4EXP_QSA_MAX_SELECTED, QWEN4EXP_QSA_QUERY_HEADS,
    QWEN4EXP_QSA_TOP_BLOCKS,
};

const GDN_CONV_WIDTH: usize =
    QWEN4EXP_GDN_KEY_HEADS * QWEN4EXP_GDN_HEAD_DIM * 2 + QWEN4EXP_GDN_HEADS * QWEN4EXP_GDN_HEAD_DIM;
const GDN_CONV_HISTORY: usize = 3;
const GDN_DELTA_WIDTH: usize =
    QWEN4EXP_GDN_HEADS * QWEN4EXP_GDN_HEAD_DIM * QWEN4EXP_GDN_HEAD_DIM;
const QSA_KV_WIDTH: usize = QWEN4EXP_QSA_KV_HEADS * QWEN4EXP_QSA_HEAD_DIM;
const QSA_INDEX_QUERY_WIDTH: usize =
    QWEN4EXP_QSA_INDEXER_HEADS * QWEN4EXP_QSA_INDEXER_HEAD_DIM;
const HYPER_LOW_RANK: usize = 320;
const MOE_INTERMEDIATE: usize = 640;
const ROUTER_EXPERTS: usize = 512;
const RAW_ROW_BYTES: usize = 65_536;

#[derive(Debug)]
pub enum Qwen4ExpDecodeError {
    MissingTensor,
    MissingTokenMixer { layer: u32 },
    Ple(PleBlockError),
    Layer(StreamedLayerError),
    Hyper(StreamedHyperError),
    Trunk(TrunkNvmeError),
}

impl core::fmt::Display for Qwen4ExpDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingTensor => {
                write!(f, "Qwen4Exp decode is missing a required trunk tensor")
            }
            Self::MissingTokenMixer { layer } => {
                write!(f, "Qwen4Exp layer {layer} has no executable token mixer")
            }
            Self::Ple(error) => error.fmt(f),
            Self::Layer(error) => error.fmt(f),
            Self::Hyper(error) => error.fmt(f),
            Self::Trunk(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Qwen4ExpDecodeError {}
impl From<PleBlockError> for Qwen4ExpDecodeError {
    fn from(value: PleBlockError) -> Self {
        Self::Ple(value)
    }
}
impl From<StreamedLayerError> for Qwen4ExpDecodeError {
    fn from(value: StreamedLayerError) -> Self {
        Self::Layer(value)
    }
}
impl From<StreamedHyperError> for Qwen4ExpDecodeError {
    fn from(value: StreamedHyperError) -> Self {
        Self::Hyper(value)
    }
}
impl From<TrunkNvmeError> for Qwen4ExpDecodeError {
    fn from(value: TrunkNvmeError) -> Self {
        Self::Trunk(value)
    }
}
impl From<GatedDeltaError> for Qwen4ExpDecodeError {
    fn from(value: GatedDeltaError) -> Self {
        Self::Layer(value.into())
    }
}

/// Owned mutable state for a live Qwen4Exp sequence.  Allocated once at
/// session start; the per-token step only mutates slices inside it.
pub struct Qwen4ExpSession {
    hidden: usize,
    n_layers: usize,
    residual: Vec<f32>,
    /// GatedDeltaNet state per layer: 3-deep conv history + 48×128×128 delta.
    gdn_convolution: Vec<Vec<f32>>,
    gdn_delta: Vec<Vec<f32>>,
    /// QSA state per layer: rotated K, raw V, raw 128-dim indexer keys.
    qsa_keys: Vec<Vec<f32>>,
    qsa_values: Vec<Vec<f32>>,
    qsa_indexer_keys: Vec<Vec<f32>>,
    qsa_tokens: Vec<usize>,
    /// PLE dilated-conv input history (`(kernel-1) * dilation` wide entries).
    ple_conv_history: Vec<f32>,
    ple_tokens: PleTokenHistory,
    ple_layer: Option<u32>,
    /// Tokens processed so far (prompt + generated).
    pub position: usize,
}

impl Qwen4ExpSession {
    /// Allocate persistent state for every layer in the validated graph.
    /// `qsa_capacity` bounds each QSA layer's retained context in tokens.
    pub fn new(index: &GgufTensorIndex, qsa_capacity: usize) -> Result<Self, Qwen4ExpDecodeError> {
        let hidden = index.emb_dim();
        let n_layers = index.hyperparams.n_layer as usize;
        if hidden == 0 || n_layers == 0 || qsa_capacity == 0 {
            return Err(Qwen4ExpDecodeError::MissingTensor);
        }
        let ple_config = index.ple_config.ok_or(Qwen4ExpDecodeError::MissingTensor)?;
        let ple_layer = (ple_config.layer_count > 0).then(|| ple_config.layers[0]);
        let mut session = Self {
            hidden,
            n_layers,
            residual: vec![0.0; QWEN4EXP_HYPER_STREAMS * hidden],
            gdn_convolution: Vec::with_capacity(n_layers),
            gdn_delta: Vec::with_capacity(n_layers),
            qsa_keys: Vec::with_capacity(n_layers),
            qsa_values: Vec::with_capacity(n_layers),
            qsa_indexer_keys: Vec::with_capacity(n_layers),
            qsa_tokens: Vec::with_capacity(n_layers),
            ple_conv_history: vec![
                0.0;
                QWEN4EXP_PLE_HISTORY * QWEN4EXP_HYPER_STREAMS * hidden
            ],
            ple_tokens: PleTokenHistory::default(),
            ple_layer,
            position: 0,
        };
        for layer_idx in 0..n_layers as u32 {
            let layer = index.get_layer_tensors(layer_idx);
            if layer.is_hybrid_ssm_layer() {
                session
                    .gdn_convolution
                    .push(vec![0.0; GDN_CONV_WIDTH * GDN_CONV_HISTORY]);
                session.gdn_delta.push(vec![0.0; GDN_DELTA_WIDTH]);
            } else {
                session.gdn_convolution.push(Vec::new());
                session.gdn_delta.push(Vec::new());
            }
            if layer.has_qwen_sparse_attention() {
                session.qsa_keys.push(vec![0.0; qsa_capacity * QSA_KV_WIDTH]);
                session
                    .qsa_values
                    .push(vec![0.0; qsa_capacity * QSA_KV_WIDTH]);
                session
                    .qsa_indexer_keys
                    .push(vec![0.0; qsa_capacity * QWEN4EXP_QSA_INDEXER_HEAD_DIM]);
                session.qsa_tokens.push(0);
            } else {
                session.qsa_keys.push(Vec::new());
                session.qsa_values.push(Vec::new());
                session.qsa_indexer_keys.push(Vec::new());
                session.qsa_tokens.push(0);
            }
        }
        Ok(session)
    }

    /// Resident bytes held by the persistent per-layer state and residual.
    pub fn state_bytes(&self) -> u64 {
        let mut total = (self.residual.len() + self.ple_conv_history.len()) as u64 * 4;
        for layer in 0..self.n_layers {
            total += (self.gdn_convolution[layer].len()
                + self.gdn_delta[layer].len()
                + self.qsa_keys[layer].len()
                + self.qsa_values[layer].len()
                + self.qsa_indexer_keys[layer].len()) as u64
                * 4;
        }
        total
    }
}

/// All per-step scratch, allocated once and shared across every layer.  The
/// GDN and QSA paths run sequentially, so one set of buffers serves both.
pub struct Qwen4ExpDecodeScratch {
    raw_row: Vec<u8>,
    attn_raw: Vec<u8>,
    ffn_raw: Vec<u8>,
    embedding: Vec<f32>,
    mixed: Vec<f32>,
    attn_hyper_norm: Vec<f32>,
    attn_hyper_normalized: Vec<f32>,
    attn_hyper_low_rank: Vec<f32>,
    attn_hyper_gates: Vec<f32>,
    attn_hyper_projection: Vec<f32>,
    ffn_hyper_norm: Vec<f32>,
    ffn_hyper_normalized: Vec<f32>,
    ffn_hyper_low_rank: Vec<f32>,
    ffn_hyper_gates: Vec<f32>,
    ffn_hyper_projection: Vec<f32>,
    gdn_raw: Vec<u8>,
    gdn_projection: Vec<f32>,
    gdn_qkv: Vec<f32>,
    gdn_gate: Vec<f32>,
    gdn_alpha: Vec<f32>,
    gdn_beta: Vec<f32>,
    gdn_convolved: Vec<f32>,
    gdn_head_norm: Vec<f32>,
    gdn_head_vector: Vec<f32>,
    gdn_output_inner: Vec<f32>,
    gdn_head_a: Vec<f32>,
    gdn_dt_bias: Vec<f32>,
    qsa_raw: Vec<u8>,
    qsa_projection: Vec<f32>,
    qsa_q_full: Vec<f32>,
    qsa_key: Vec<f32>,
    qsa_value: Vec<f32>,
    qsa_index_query: Vec<f32>,
    qsa_index_key: Vec<f32>,
    qsa_q_norm: Vec<f32>,
    qsa_k_norm: Vec<f32>,
    qsa_index_q_norm: Vec<f32>,
    qsa_index_k_norm: Vec<f32>,
    qsa_selected: Vec<usize>,
    qsa_scores: Vec<f32>,
    qsa_block_selected: Vec<usize>,
    qsa_block_scores: Vec<f32>,
    qsa_attention: Vec<f32>,
    moe_raw: Vec<u8>,
    moe_dequantized: Vec<f32>,
    moe_router_logits: Vec<f32>,
    moe_top_indices: [usize; 10],
    moe_top_logits: [f32; 10],
    moe_top_weights: [f32; 10],
    moe_gate: Vec<f32>,
    moe_up: Vec<f32>,
    moe_activation: Vec<f32>,
    moe_expert_out: Vec<f32>,
    attn_mixed: Vec<f32>,
    token_mixer_branch: Vec<f32>,
    ffn_mixed: Vec<f32>,
    moe_branch: Vec<f32>,
    attn_inject: [f32; 4],
    ffn_inject: [f32; 4],
    ple_raw: Vec<u8>,
    ple_projection: Vec<f32>,
    ple_embedding: Vec<f32>,
    ple_key: Vec<f32>,
    ple_value: Vec<f32>,
    ple_query_norm: Vec<f32>,
    ple_gated: Vec<f32>,
    ple_conv_norm: Vec<f32>,
    ple_conv_weights: Vec<f32>,
    ple_norm_key: Vec<f32>,
    ple_norm_query: Vec<f32>,
    ple_norm_conv: Vec<f32>,
    argmax_row: Vec<f32>,
    footprint: u64,
}

/// Counting allocator for the fixed decode-scratch set, so receipts report a
/// measured footprint rather than a hand-computed constant.
struct ScratchAlloc {
    bytes: u64,
}

impl ScratchAlloc {
    fn f32s(&mut self, n: usize) -> Vec<f32> {
        self.bytes += (n * 4) as u64;
        vec![0.0; n]
    }
    fn u8s(&mut self, n: usize) -> Vec<u8> {
        self.bytes += n as u64;
        vec![0u8; n]
    }
    fn usizes(&mut self, n: usize) -> Vec<usize> {
        self.bytes += (n * 8) as u64;
        vec![0usize; n]
    }
}

impl Qwen4ExpDecodeScratch {
    pub fn new(hidden: usize) -> Self {
        let mut alloc = ScratchAlloc { bytes: 0 };
        let wide = QWEN4EXP_HYPER_STREAMS * hidden;
        let q_width = QWEN4EXP_QSA_QUERY_HEADS * QWEN4EXP_QSA_HEAD_DIM;
        let kv_width = QWEN4EXP_QSA_KV_HEADS * QWEN4EXP_QSA_HEAD_DIM;
        let gdn_inner = QWEN4EXP_GDN_HEADS * QWEN4EXP_GDN_HEAD_DIM;
        Self {
            raw_row: alloc.u8s(RAW_ROW_BYTES),
            attn_raw: alloc.u8s(RAW_ROW_BYTES),
            ffn_raw: alloc.u8s(RAW_ROW_BYTES),
            embedding: alloc.f32s(hidden),
            mixed: alloc.f32s(hidden),
            attn_hyper_norm: alloc.f32s(wide),
            attn_hyper_normalized: alloc.f32s(wide),
            attn_hyper_low_rank: alloc.f32s(HYPER_LOW_RANK),
            attn_hyper_gates: alloc.f32s(wide),
            attn_hyper_projection: alloc.f32s(wide),
            ffn_hyper_norm: alloc.f32s(wide),
            ffn_hyper_normalized: alloc.f32s(wide),
            ffn_hyper_low_rank: alloc.f32s(HYPER_LOW_RANK),
            ffn_hyper_gates: alloc.f32s(wide),
            ffn_hyper_projection: alloc.f32s(wide),
            gdn_raw: alloc.u8s(RAW_ROW_BYTES),
            gdn_projection: alloc.f32s(gdn_inner),
            gdn_qkv: alloc.f32s(GDN_CONV_WIDTH),
            gdn_gate: alloc.f32s(gdn_inner),
            gdn_alpha: alloc.f32s(QWEN4EXP_GDN_HEADS),
            gdn_beta: alloc.f32s(QWEN4EXP_GDN_HEADS),
            gdn_convolved: alloc.f32s(GDN_CONV_WIDTH),
            gdn_head_norm: alloc.f32s(QWEN4EXP_GDN_HEAD_DIM),
            gdn_head_vector: alloc.f32s(QWEN4EXP_GDN_HEAD_DIM),
            gdn_output_inner: alloc.f32s(gdn_inner),
            gdn_head_a: alloc.f32s(QWEN4EXP_GDN_HEADS),
            gdn_dt_bias: alloc.f32s(QWEN4EXP_GDN_HEADS),
            qsa_raw: alloc.u8s(RAW_ROW_BYTES),
            qsa_projection: alloc.f32s(q_width),
            qsa_q_full: alloc.f32s(q_width * 2),
            qsa_key: alloc.f32s(kv_width),
            qsa_value: alloc.f32s(kv_width),
            qsa_index_query: alloc.f32s(QSA_INDEX_QUERY_WIDTH),
            qsa_index_key: alloc.f32s(QWEN4EXP_QSA_INDEXER_HEAD_DIM),
            qsa_q_norm: alloc.f32s(QWEN4EXP_QSA_HEAD_DIM),
            qsa_k_norm: alloc.f32s(QWEN4EXP_QSA_HEAD_DIM),
            qsa_index_q_norm: alloc.f32s(QWEN4EXP_QSA_INDEXER_HEAD_DIM),
            qsa_index_k_norm: alloc.f32s(QWEN4EXP_QSA_INDEXER_HEAD_DIM),
            qsa_selected: alloc.usizes(QWEN4EXP_QSA_MAX_SELECTED),
            qsa_scores: alloc.f32s(QWEN4EXP_QSA_MAX_SELECTED),
            qsa_block_selected: alloc.usizes(QWEN4EXP_QSA_TOP_BLOCKS),
            qsa_block_scores: alloc.f32s(QWEN4EXP_QSA_TOP_BLOCKS),
            qsa_attention: alloc.f32s(q_width),
            moe_raw: alloc.u8s(RAW_ROW_BYTES),
            moe_dequantized: alloc.f32s(hidden),
            moe_router_logits: alloc.f32s(ROUTER_EXPERTS),
            moe_top_indices: [0; 10],
            moe_top_logits: [0.0; 10],
            moe_top_weights: [0.0; 10],
            moe_gate: alloc.f32s(MOE_INTERMEDIATE),
            moe_up: alloc.f32s(MOE_INTERMEDIATE),
            moe_activation: alloc.f32s(MOE_INTERMEDIATE),
            moe_expert_out: alloc.f32s(hidden),
            attn_mixed: alloc.f32s(hidden),
            token_mixer_branch: alloc.f32s(hidden),
            ffn_mixed: alloc.f32s(hidden),
            moe_branch: alloc.f32s(hidden),
            attn_inject: [0.0; 4],
            ffn_inject: [0.0; 4],
            ple_raw: alloc.u8s(RAW_ROW_BYTES),
            ple_projection: alloc.f32s(wide),
            ple_embedding: alloc.f32s(hidden),
            ple_key: alloc.f32s(wide),
            ple_value: alloc.f32s(hidden),
            ple_query_norm: alloc.f32s(wide),
            ple_gated: alloc.f32s(wide),
            ple_conv_norm: alloc.f32s(wide),
            ple_conv_weights: alloc.f32s(wide * 4),
            ple_norm_key: alloc.f32s(wide),
            ple_norm_query: alloc.f32s(wide),
            ple_norm_conv: alloc.f32s(wide),
            argmax_row: alloc.f32s(hidden),
            footprint: alloc.bytes,
        }
    }

    /// Measured bytes held by this scratch set (fixed at construction).
    pub fn scratch_bytes(&self) -> u64 {
        self.footprint
    }

    /// The hidden vector produced by the most recent final mixer step —
    /// what the vocabulary projection consumed.
    pub fn final_mixed(&self) -> &[f32] {
        &self.mixed
    }
}

/// One observed tensor inside a token step, for lab-style localization of a
/// divergent stage.  `layer` is `u32::MAX` for the embedding/final stages.
#[derive(Clone, Debug)]
pub struct Qwen4ExpTraceRecord {
    pub step: usize,
    pub layer: u32,
    pub stage: &'static str,
    pub fingerprint: u64,
    pub rms: f32,
    pub abs_max: f32,
    /// Stage values, populated only when the caller passes `capture=true`;
    /// kept empty on the production decode path so it stays allocation-free.
    pub values: Vec<f32>,
}

/// Layer sentinel for records that do not belong to a numbered block.
pub const QWEN4EXP_TRACE_META_LAYER: u32 = u32::MAX;

fn push_trace(
    trace: &mut Vec<Qwen4ExpTraceRecord>,
    step: usize,
    layer: u32,
    stage: &'static str,
    data: &[f32],
    capture: bool,
) {
    let mut fingerprint = 0xcbf2_9ce4_8422_2325u64;
    let mut sum_sq = 0.0f64;
    let mut abs_max = 0.0f32;
    for &value in data {
        fingerprint ^= value.to_bits() as u64;
        fingerprint = fingerprint.wrapping_mul(0x1000_0000_01b3);
        sum_sq += (value as f64) * (value as f64);
        abs_max = abs_max.max(value.abs());
    }
    trace.push(Qwen4ExpTraceRecord {
        step,
        layer,
        stage,
        fingerprint,
        rms: (sum_sq / data.len().max(1) as f64).sqrt() as f32,
        abs_max,
        values: if capture { data.to_vec() } else { Vec::new() },
    });
}

/// Measurement receipt for a real multi-token decode run.  `trunk_*` and
/// `ple_*` counts come from the readers' own counters, not estimates.
#[derive(Clone, Debug, Default)]
pub struct Qwen4ExpDecodeReceipt {
    pub prompt_tokens: usize,
    pub generated_tokens: Vec<u32>,
    pub elapsed: std::time::Duration,
    pub trunk_reads: u64,
    pub trunk_rows: u64,
    pub trunk_bytes: u64,
    pub ple_reads: u64,
    pub ple_rows: u64,
    pub ple_bytes: u64,
    pub state_bytes: u64,
    pub scratch_bytes: u64,
}

/// Run one complete token step through the trained Qwen4Exp graph and return
/// the streamed vocabulary argmax.  Every prompt and generated token passes
/// the same path: embedding → optional PLE residual → per-layer HC/mixer/HC/
/// MoE → final Hyper-Connection mixer → `output.weight` argmax.
pub fn decode_step(
    runtime: &mut Qwen4ExpNativeRuntime,
    session: &mut Qwen4ExpSession,
    scratch: &mut Qwen4ExpDecodeScratch,
    token: u32,
    trace: &mut Vec<Qwen4ExpTraceRecord>,
    capture: bool,
) -> Result<StreamedArgmax, Qwen4ExpDecodeError> {
    let index = &runtime.index;
    let hidden = session.hidden;
    let embedding = *index
        .token_embd_info()
        .ok_or(Qwen4ExpDecodeError::MissingTensor)?;
    let output_norm = index
        .tensor_info(b"output_hc_norm.weight")
        .ok_or(Qwen4ExpDecodeError::MissingTensor)?;
    let output_down = index
        .tensor_info(b"output_hc_down.weight")
        .ok_or(Qwen4ExpDecodeError::MissingTensor)?;
    let output_up = index
        .tensor_info(b"output_hc_up.weight")
        .ok_or(Qwen4ExpDecodeError::MissingTensor)?;
    let logits = *index
        .logits_projection_info()
        .ok_or(Qwen4ExpDecodeError::MissingTensor)?;
    if token as u64 >= embedding.dims[1] {
        return Err(Qwen4ExpDecodeError::MissingTensor);
    }

    runtime.trunk.read_row_into(
        &embedding,
        token as usize,
        &mut scratch.raw_row,
        &mut scratch.embedding[..hidden],
    )?;
    for stream in 0..QWEN4EXP_HYPER_STREAMS {
        let start = stream * hidden;
        session.residual[start..start + hidden]
            .copy_from_slice(&scratch.embedding[..hidden]);
    }
    let step = session.position;
    push_trace(
        trace,
        step,
        QWEN4EXP_TRACE_META_LAYER,
        "embed",
        &session.residual,
        capture,
    );

    let rope_theta = index.hyperparams.effective_rope_freq_base();
    let ple_config = index.ple_config;
    for layer_idx in 0..session.n_layers as u32 {
        let layer = index.get_layer_tensors(layer_idx);
        if session.ple_layer == Some(layer_idx) {
            let config = ple_config.ok_or(Qwen4ExpDecodeError::MissingTensor)?;
            let mut ple_buffers = PleBlockBuffers {
                raw_ple_row: &mut scratch.ple_raw,
                raw_trunk_row: &mut scratch.raw_row,
                projection_row: &mut scratch.ple_projection,
                embedding: &mut scratch.ple_embedding,
                key: &mut scratch.ple_key,
                value: &mut scratch.ple_value,
                query_norm: &mut scratch.ple_query_norm,
                gated: &mut scratch.ple_gated,
                conv_norm: &mut scratch.ple_conv_norm,
                conv_weights: &mut scratch.ple_conv_weights,
                norm_key: &mut scratch.ple_norm_key,
                norm_query: &mut scratch.ple_norm_query,
                norm_conv: &mut scratch.ple_norm_conv,
            };
            execute_ple_block(
                &config,
                &layer,
                token,
                &mut session.ple_tokens,
                &mut runtime.ple,
                &mut runtime.trunk,
                &mut session.residual,
                &mut session.ple_conv_history,
                &mut ple_buffers,
            )?;
            push_trace(trace, step, layer_idx, "post_ple", &session.residual, capture);
        }
        if layer.is_hybrid_ssm_layer() {
            let mut state = GatedDeltaState {
                convolution: &mut session.gdn_convolution[layer_idx as usize],
                delta: &mut session.gdn_delta[layer_idx as usize],
            };
            let mut buffers = StreamedGdnMoeLayerBuffers {
                attn_hyper: StreamedHyperBuffers {
                    raw_row: &mut scratch.attn_raw,
                    projection_row: &mut scratch.attn_hyper_projection,
                    norm_weight: &mut scratch.attn_hyper_norm,
                    normalized: &mut scratch.attn_hyper_normalized,
                    low_rank: &mut scratch.attn_hyper_low_rank,
                    gates: &mut scratch.attn_hyper_gates,
                },
                ffn_hyper: StreamedHyperBuffers {
                    raw_row: &mut scratch.ffn_raw,
                    projection_row: &mut scratch.ffn_hyper_projection,
                    norm_weight: &mut scratch.ffn_hyper_norm,
                    normalized: &mut scratch.ffn_hyper_normalized,
                    low_rank: &mut scratch.ffn_hyper_low_rank,
                    gates: &mut scratch.ffn_hyper_gates,
                },
                gdn: GatedDeltaBuffers {
                    raw_row: &mut scratch.gdn_raw,
                    projection_row: &mut scratch.gdn_projection,
                    qkv: &mut scratch.gdn_qkv,
                    gate: &mut scratch.gdn_gate,
                    alpha: &mut scratch.gdn_alpha,
                    beta: &mut scratch.gdn_beta,
                    convolved: &mut scratch.gdn_convolved,
                    head_norm: &mut scratch.gdn_head_norm,
                    head_vector: &mut scratch.gdn_head_vector,
                    output_inner: &mut scratch.gdn_output_inner,
                    head_a: &mut scratch.gdn_head_a,
                    dt_bias: &mut scratch.gdn_dt_bias,
                },
                moe: StreamedMoeBuffers {
                    raw_row: &mut scratch.moe_raw,
                    dequantized_row: &mut scratch.moe_dequantized,
                    router_logits: &mut scratch.moe_router_logits,
                    top_indices: &mut scratch.moe_top_indices,
                    top_logits: &mut scratch.moe_top_logits,
                    top_weights: &mut scratch.moe_top_weights,
                    gate: &mut scratch.moe_gate,
                    up: &mut scratch.moe_up,
                    activation: &mut scratch.moe_activation,
                    expert_out: &mut scratch.moe_expert_out,
                },
                attn_mixed: &mut scratch.attn_mixed,
                token_mixer_branch: &mut scratch.token_mixer_branch,
                ffn_mixed: &mut scratch.ffn_mixed,
                moe_branch: &mut scratch.moe_branch,
                attn_inject: &mut scratch.attn_inject,
                ffn_inject: &mut scratch.ffn_inject,
            };
            execute_streamed_gdn_moe_layer(
                &mut runtime.trunk,
                &layer,
                &mut session.residual,
                &mut state,
                &mut buffers,
            )?;
        } else if layer.has_qwen_sparse_attention() {
            let slot = layer_idx as usize;
            let mut state = QsaState {
                keys: &mut session.qsa_keys[slot],
                values: &mut session.qsa_values[slot],
                indexer_keys: &mut session.qsa_indexer_keys[slot],
                tokens: session.qsa_tokens[slot],
            };
            let mut buffers = StreamedQsaMoeLayerBuffers {
                attn_hyper: StreamedHyperBuffers {
                    raw_row: &mut scratch.attn_raw,
                    projection_row: &mut scratch.attn_hyper_projection,
                    norm_weight: &mut scratch.attn_hyper_norm,
                    normalized: &mut scratch.attn_hyper_normalized,
                    low_rank: &mut scratch.attn_hyper_low_rank,
                    gates: &mut scratch.attn_hyper_gates,
                },
                ffn_hyper: StreamedHyperBuffers {
                    raw_row: &mut scratch.ffn_raw,
                    projection_row: &mut scratch.ffn_hyper_projection,
                    norm_weight: &mut scratch.ffn_hyper_norm,
                    normalized: &mut scratch.ffn_hyper_normalized,
                    low_rank: &mut scratch.ffn_hyper_low_rank,
                    gates: &mut scratch.ffn_hyper_gates,
                },
                qsa: StreamedQsaBuffers {
                    raw_row: &mut scratch.qsa_raw,
                    projection_row: &mut scratch.qsa_projection,
                    q_full: &mut scratch.qsa_q_full,
                    key: &mut scratch.qsa_key,
                    value: &mut scratch.qsa_value,
                    index_query: &mut scratch.qsa_index_query,
                    index_key: &mut scratch.qsa_index_key,
                    q_norm: &mut scratch.qsa_q_norm,
                    k_norm: &mut scratch.qsa_k_norm,
                    index_q_norm: &mut scratch.qsa_index_q_norm,
                    index_k_norm: &mut scratch.qsa_index_k_norm,
                    selected: &mut scratch.qsa_selected,
                    scores: &mut scratch.qsa_scores,
                    block_selected: &mut scratch.qsa_block_selected,
                    block_scores: &mut scratch.qsa_block_scores,
                    attention: &mut scratch.qsa_attention,
                },
                moe: StreamedMoeBuffers {
                    raw_row: &mut scratch.moe_raw,
                    dequantized_row: &mut scratch.moe_dequantized,
                    router_logits: &mut scratch.moe_router_logits,
                    top_indices: &mut scratch.moe_top_indices,
                    top_logits: &mut scratch.moe_top_logits,
                    top_weights: &mut scratch.moe_top_weights,
                    gate: &mut scratch.moe_gate,
                    up: &mut scratch.moe_up,
                    activation: &mut scratch.moe_activation,
                    expert_out: &mut scratch.moe_expert_out,
                },
                attn_mixed: &mut scratch.attn_mixed,
                token_mixer_branch: &mut scratch.token_mixer_branch,
                ffn_mixed: &mut scratch.ffn_mixed,
                moe_branch: &mut scratch.moe_branch,
                attn_inject: &mut scratch.attn_inject,
                ffn_inject: &mut scratch.ffn_inject,
            };
            execute_streamed_qsa_moe_layer(
                &mut runtime.trunk,
                &layer,
                &mut session.residual,
                &mut state,
                rope_theta,
                &mut buffers,
            )?;
            session.qsa_tokens[slot] = state.tokens;
        } else {
            return Err(Qwen4ExpDecodeError::MissingTokenMixer { layer: layer_idx });
        }
        // The fused layer call leaves every intermediate in scratch, so the
        // trace sees each stage without the operators knowing about it.
        push_trace(trace, step, layer_idx, "attn_in", &scratch.attn_mixed, capture);
        push_trace(
            trace,
            step,
            layer_idx,
            "mixer_out",
            &scratch.token_mixer_branch,
            capture,
        );
        push_trace(trace, step, layer_idx, "attn_inject", &scratch.attn_inject, capture);
        push_trace(trace, step, layer_idx, "ffn_in", &scratch.ffn_mixed, capture);
        push_trace(trace, step, layer_idx, "moe_out", &scratch.moe_branch, capture);
        push_trace(trace, step, layer_idx, "ffn_inject", &scratch.ffn_inject, capture);
        push_trace(trace, step, layer_idx, "post_layer", &session.residual, capture);
    }

    execute_streamed_final_hyper_connection(
        &mut runtime.trunk,
        output_norm,
        output_down,
        output_up,
        &session.residual,
        &mut scratch.mixed[..hidden],
        &mut StreamedHyperBuffers {
            raw_row: &mut scratch.attn_raw,
            projection_row: &mut scratch.attn_hyper_projection,
            norm_weight: &mut scratch.attn_hyper_norm,
            normalized: &mut scratch.attn_hyper_normalized,
            low_rank: &mut scratch.attn_hyper_low_rank,
            gates: &mut scratch.attn_hyper_gates,
        },
    )?;
    push_trace(
        trace,
        step,
        QWEN4EXP_TRACE_META_LAYER,
        "final_mixed",
        &scratch.mixed[..hidden],
        capture,
    );
    let winner = runtime.trunk.argmax_rows(
        &logits,
        &scratch.mixed[..hidden],
        &mut scratch.raw_row,
        &mut scratch.argmax_row,
    )?;
    session.position += 1;
    Ok(winner)
}

/// Drive a full decode: every prompt token updates the recurrent state, then
/// `max_tokens` argmax steps generate the output.  Stops early on a stop token
/// supplied by the caller (GGUF EOS / chat-end set).
pub fn decode_tokens(
    runtime: &mut Qwen4ExpNativeRuntime,
    session: &mut Qwen4ExpSession,
    scratch: &mut Qwen4ExpDecodeScratch,
    prompt: &[u32],
    max_tokens: usize,
    is_stop: &dyn Fn(u32) -> bool,
    receipt: &mut Qwen4ExpDecodeReceipt,
    trace: &mut Vec<Qwen4ExpTraceRecord>,
) -> Result<(), Qwen4ExpDecodeError> {
    let trunk_start = runtime.trunk.io_stats();
    let ple_start = runtime.ple.io_stats();
    let start = Instant::now();
    // The argmax over the last prompt token IS the first generated token;
    // re-feeding it would push that token through the recurrent/PLE state
    // twice.
    let (&last_prompt, prefix) = prompt
        .split_last()
        .ok_or(Qwen4ExpDecodeError::MissingTensor)?;
    for &token in prefix {
        decode_step(runtime, session, scratch, token, trace, false)?;
    }
    let mut winner = decode_step(runtime, session, scratch, last_prompt, trace, false)?;
    receipt.prompt_tokens = prompt.len();
    for _ in 0..max_tokens {
        if is_stop(winner.token_id) {
            break;
        }
        receipt.generated_tokens.push(winner.token_id);
        winner = decode_step(runtime, session, scratch, winner.token_id, trace, false)?;
    }
    receipt.elapsed = start.elapsed();
    let trunk_end = runtime.trunk.io_stats();
    let ple_end = runtime.ple.io_stats();
    receipt.trunk_reads = trunk_end.reads - trunk_start.reads;
    receipt.trunk_rows = trunk_end.rows - trunk_start.rows;
    receipt.trunk_bytes = trunk_end.bytes - trunk_start.bytes;
    receipt.ple_reads = ple_end.reads - ple_start.reads;
    receipt.ple_rows = ple_end.rows - ple_start.rows;
    receipt.ple_bytes = ple_end.bytes - ple_start.bytes;
    receipt.state_bytes = session.state_bytes();
    receipt.scratch_bytes = scratch.scratch_bytes();
    Ok(())
}
