use std::fmt;
use std::sync::Arc;

use crate::gguf_sharder::{GgufTensorIndex, GgufTokenizer};

use super::{CPU_WASM_DEFAULT_CONTEXT, CPU_WASM_MAX_CONTEXT};

const MAX_EMBEDDING: usize = 8192;
const MAX_FFN: usize = 16_384;
const MIN_REAL_VOCAB: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpuWasmError {
    InvalidModel(String),
    UnsupportedModel(String),
    ContextExceeded { position: usize, max_context: usize },
    MissingTensor { layer: u32, role: &'static str },
    KernelFailed { layer: u32, role: &'static str },
}

impl fmt::Display for CpuWasmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidModel(reason) => write!(f, "invalid model: {reason}"),
            Self::UnsupportedModel(reason) => write!(f, "unsupported CPU-WASM model: {reason}"),
            Self::ContextExceeded {
                position,
                max_context,
            } => write!(
                f,
                "token position {position} exceeds CPU-WASM context {max_context}"
            ),
            Self::MissingTensor { layer, role } => {
                write!(f, "layer {layer} is missing required tensor {role}")
            }
            Self::KernelFailed { layer, role } => {
                write!(f, "layer {layer} CPU kernel failed for {role}")
            }
        }
    }
}

impl std::error::Error for CpuWasmError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CpuWasmStep {
    pub token_id: u32,
    pub max_logit: f32,
}

/// Cold-built, allocation-stable CPU decode plan.
pub struct CpuWasmEngine {
    pub(super) model: Arc<[u8]>,
    pub(super) index: GgufTensorIndex,
    pub(super) tokenizer: GgufTokenizer,
    pub(super) max_context: usize,
    pub(super) n_embd: usize,
    pub(super) n_ffn: usize,
    pub(super) n_layer: usize,
    pub(super) n_head: usize,
    pub(super) n_kv_head: usize,
    pub(super) head_dim: usize,
    pub(super) kv_plane_elems: usize,
    pub(super) kv: Vec<f32>,
    pub(super) hidden: Vec<f32>,
    pub(super) normed: Vec<f32>,
    pub(super) norm_weight: Vec<f32>,
    pub(super) q: Vec<f32>,
    pub(super) k: Vec<f32>,
    pub(super) v: Vec<f32>,
    pub(super) attention: Vec<f32>,
    pub(super) projection: Vec<f32>,
    pub(super) gate: Vec<f32>,
    pub(super) up: Vec<f32>,
    pub(super) scores: Vec<f32>,
    pub(super) logits: Vec<f32>,
}

impl CpuWasmEngine {
    pub fn new(model: Arc<[u8]>) -> Result<Self, CpuWasmError> {
        Self::new_with_context(model, CPU_WASM_DEFAULT_CONTEXT)
    }

    pub fn new_with_context(model: Arc<[u8]>, max_context: usize) -> Result<Self, CpuWasmError> {
        if max_context == 0 || max_context > CPU_WASM_MAX_CONTEXT {
            return Err(CpuWasmError::UnsupportedModel(format!(
                "context {max_context} is outside 1..={CPU_WASM_MAX_CONTEXT}"
            )));
        }
        let (tokenizer, index) = if crate::p64_weight::has_p64_magic(&model) {
            let p64 = crate::p64_weight::P64TensorIndex::from_p64(&model)
                .map_err(|e| CpuWasmError::InvalidModel(format!("P64 index: {e}")))?;
            let tokenizer = GgufTokenizer::from_p64_section(p64.tokenizer_bytes(&model))
                .ok_or_else(|| CpuWasmError::InvalidModel("P64 has no Q42T tokenizer".into()))?;
            (tokenizer, p64.to_gguf_index())
        } else {
            (
                GgufTokenizer::from_gguf(&model),
                GgufTensorIndex::from_gguf(&model),
            )
        };

        if (tokenizer.vocab_len() as usize) < MIN_REAL_VOCAB {
            return Err(CpuWasmError::InvalidModel(format!(
                "tokenizer vocabulary {} is fallback-only",
                tokenizer.vocab_len()
            )));
        }
        index
            .hyperparams
            .decode_supported()
            .map_err(CpuWasmError::UnsupportedModel)?;

        let n_embd = index.emb_dim();
        let n_layer = index.hyperparams.n_layer as usize;
        let n_head = index.hyperparams.n_head as usize;
        let n_kv_head = index.hyperparams.effective_n_kv_head() as usize;
        let head_dim = index.hyperparams.head_dim() as usize;
        if n_embd == 0 || n_embd > MAX_EMBEDDING || n_layer == 0 || n_head == 0 {
            return Err(CpuWasmError::UnsupportedModel(format!(
                "dimensions emb={n_embd}, layers={n_layer}, heads={n_head}"
            )));
        }
        if n_head * head_dim != n_embd || n_kv_head == 0 || n_head % n_kv_head != 0 {
            return Err(CpuWasmError::UnsupportedModel(format!(
                "non-Llama attention shape heads={n_head}, kv_heads={n_kv_head}, head_dim={head_dim}, emb={n_embd}"
            )));
        }
        let layer0 = index.get_layer_tensors(0);
        let n_ffn = layer0
            .ffn_gate
            .map(|t| t.dims[1] as usize)
            .filter(|&n| n > 0 && n <= MAX_FFN)
            .ok_or_else(|| CpuWasmError::UnsupportedModel("missing or oversized FFN".into()))?;
        let vocab = index.vocab_dim();
        if vocab == 0 || index.logits_projection_info().is_none() {
            return Err(CpuWasmError::InvalidModel(
                "missing output projection".into(),
            ));
        }

        let per_token_kv = n_layer
            .checked_mul(n_kv_head)
            .and_then(|n| n.checked_mul(head_dim))
            .and_then(|n| n.checked_mul(2))
            .ok_or_else(|| CpuWasmError::UnsupportedModel("KV dimensions overflow".into()))?;
        let kv_plane_elems = n_layer
            .checked_mul(max_context)
            .and_then(|n| n.checked_mul(n_kv_head))
            .and_then(|n| n.checked_mul(head_dim))
            .ok_or_else(|| CpuWasmError::UnsupportedModel("KV allocation overflow".into()))?;
        let _kv_bytes = per_token_kv
            .checked_mul(max_context)
            .and_then(|n| n.checked_mul(core::mem::size_of::<f32>()))
            .ok_or_else(|| CpuWasmError::UnsupportedModel("KV working-set overflow".into()))?;

        Ok(Self {
            model,
            index,
            tokenizer,
            max_context,
            n_embd,
            n_ffn,
            n_layer,
            n_head,
            n_kv_head,
            head_dim,
            kv_plane_elems,
            kv: vec![0.0; kv_plane_elems * 2],
            hidden: vec![0.0; n_embd],
            normed: vec![0.0; n_embd],
            norm_weight: vec![0.0; n_embd],
            q: vec![0.0; n_embd],
            k: vec![0.0; n_kv_head * head_dim],
            v: vec![0.0; n_kv_head * head_dim],
            attention: vec![0.0; n_embd],
            projection: vec![0.0; n_embd.max(n_ffn)],
            gate: vec![0.0; n_ffn],
            up: vec![0.0; n_ffn],
            scores: vec![0.0; max_context],
            logits: vec![0.0; vocab],
        })
    }

    pub fn tokenizer(&self) -> &GgufTokenizer {
        &self.tokenizer
    }

    pub fn vocab_len(&self) -> u32 {
        self.tokenizer.vocab_len()
    }

    pub fn max_context(&self) -> usize {
        self.max_context
    }

    pub fn working_set_bytes(&self) -> usize {
        (self.kv.capacity()
            + self.hidden.capacity()
            + self.normed.capacity()
            + self.norm_weight.capacity()
            + self.q.capacity()
            + self.k.capacity()
            + self.v.capacity()
            + self.attention.capacity()
            + self.projection.capacity()
            + self.gate.capacity()
            + self.up.capacity()
            + self.scores.capacity()
            + self.logits.capacity())
            * core::mem::size_of::<f32>()
    }

    pub fn reset(&mut self) {
        self.kv.fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_memory_domain_is_independent_from_slg_sentinel() {
        assert_eq!(CPU_WASM_DEFAULT_CONTEXT, 512);
        assert_eq!(CPU_WASM_MAX_CONTEXT, 4096);
    }
}
