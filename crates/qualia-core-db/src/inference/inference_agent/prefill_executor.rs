//! Chunked prompt prefill executor with explicit completion and cancellation status.
//!
//! Enforces that partial prefill execution cannot be committed to cache as completed state.

use super::control::DecodeControl;
use crate::gguf_bridge::{PREFILL_CHUNK_SIZE, PREFILL_CHUNK_STACK_FLOATS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefillFailure {
    MissingTensorIndex,
    MissingModelMap,
    EmbeddingDequantization { token_id: u32 },
    EngineRejectedChunk { failure: crate::gguf_bridge::PrefillDispatchFailure },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefillOutcome {
    /// All prompt tokens were successfully processed through transformer layers.
    Completed { tokens_processed: usize },
    /// Prefill was aborted before finishing due to cancellation signal.
    Cancelled { tokens_processed: usize },
    /// Engine failed to dispatch a prefill chunk.
    Failed {
        pos: usize,
        tokens_processed: usize,
        reason: PrefillFailure,
    },
    /// Prompt had <= 1 token; no prefill needed.
    NoPrefillNeeded,
}

impl PrefillOutcome {
    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed { .. })
    }
}

/// Execute chunked prefill across prompt tokens [0..prompt_len-1).
///
/// If `control` signals cancellation, the loop halts immediately and returns `PrefillOutcome::Cancelled`,
/// preventing incomplete KV states from being published as complete.
#[allow(clippy::too_many_arguments)]
pub fn execute_chunked_prefill(
    engine: &mut crate::gguf_bridge::QTensorEngine,
    tensor_idx: Option<&crate::inference::gguf_sharder::GgufTensorIndex>,
    ctx: &[u32],
    emb_dim: usize,
    prefill_chunk: &mut [f32],
    scratch_a: &mut [f32],
    scratch_b: &mut [f32],
    control: Option<&DecodeControl>,
    layer_cap: u32,
) -> PrefillOutcome {
    execute_chunked_prefill_with_offset(
        engine,
        tensor_idx,
        ctx,
        0,
        emb_dim,
        prefill_chunk,
        scratch_a,
        scratch_b,
        control,
        layer_cap,
    )
}

/// Execute chunked prefill starting from `start_pos`.
///
/// When a graph-assisted prefix hit occurs, `start_pos` matches the shared prefix token length,
/// completely skipping redundant prefill computation for tokens already held in the paged KV cache.
#[allow(clippy::too_many_arguments)]
pub fn execute_chunked_prefill_with_offset(
    engine: &mut crate::gguf_bridge::QTensorEngine,
    tensor_idx: Option<&crate::inference::gguf_sharder::GgufTensorIndex>,
    ctx: &[u32],
    start_pos: usize,
    emb_dim: usize,
    prefill_chunk: &mut [f32],
    scratch_a: &mut [f32],
    scratch_b: &mut [f32],
    control: Option<&DecodeControl>,
    layer_cap: u32,
) -> PrefillOutcome {
    let prompt_len = ctx.len();
    if prompt_len <= 1 || start_pos >= prompt_len.saturating_sub(1) {
        return PrefillOutcome::NoPrefillNeeded;
    }

    let Some(idx) = tensor_idx else {
        return PrefillOutcome::Failed {
            pos: start_pos,
            tokens_processed: start_pos,
            reason: PrefillFailure::MissingTensorIndex,
        };
    };

    let prefill_tokens = prompt_len - 1;
    let chunk_cap = (PREFILL_CHUNK_STACK_FLOATS / emb_dim.max(1))
        .min(PREFILL_CHUNK_SIZE)
        .max(1);

    let mut pos = start_pos;
    while pos < prefill_tokens {
        if control.is_some_and(DecodeControl::is_cancelled) {
            return PrefillOutcome::Cancelled { tokens_processed: pos };
        }

        let n = (prefill_tokens - pos).min(chunk_cap);
        let batch_elems = n * emb_dim;

        {
            let Some(mmap) = engine.gguf_mmap.as_deref() else {
                return PrefillOutcome::Failed {
                    pos,
                    tokens_processed: pos,
                    reason: PrefillFailure::MissingModelMap,
                };
            };
            for t in 0..n {
                let written = idx.dequantize_token_embedding_into(
                    mmap,
                    ctx[pos + t],
                    &mut prefill_chunk[t * emb_dim..(t + 1) * emb_dim],
                );
                if written != emb_dim {
                    crate::gguf_bridge::wlog(&format!(
                        "[llm] PREFILL token embedding dequant failed for token {} at pos={}",
                        ctx[pos + t],
                        pos + t
                    ));
                    return PrefillOutcome::Failed {
                        pos: pos + t,
                        tokens_processed: pos,
                        reason: PrefillFailure::EmbeddingDequantization {
                            token_id: ctx[pos + t],
                        },
                    };
                }
            }
        }

        if let Err(failure) = engine.dispatch_prefill_chunk(
            idx,
            &mut prefill_chunk[..batch_elems],
            emb_dim,
            n as u32,
            pos as u32,
            scratch_a,
            scratch_b,
            layer_cap,
        ) {
            crate::gguf_bridge::wlog(&format!(
                "[llm] PREFILL chunk FAILED pos={pos} n={n}"
            ));
            return PrefillOutcome::Failed {
                pos,
                tokens_processed: pos,
                reason: PrefillFailure::EngineRejectedChunk { failure },
            };
        }

        pos += n;
    }

    PrefillOutcome::Completed { tokens_processed: pos }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefill_outcome_states() {
        let completed = PrefillOutcome::Completed { tokens_processed: 10 };
        assert!(completed.is_completed());

        let cancelled = PrefillOutcome::Cancelled { tokens_processed: 5 };
        assert!(!cancelled.is_completed());

        let failed = PrefillOutcome::Failed {
            pos: 2,
            tokens_processed: 2,
            reason: PrefillFailure::EngineRejectedChunk {
                failure: crate::gguf_bridge::PrefillDispatchFailure::EmptyInput,
            },
        };
        assert!(!failed.is_completed());

        let none_needed = PrefillOutcome::NoPrefillNeeded;
        assert!(!none_needed.is_completed());
    }
}
