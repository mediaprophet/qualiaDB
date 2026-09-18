//! Chunked prompt prefill executor with explicit completion and cancellation status.
//!
//! Enforces that partial prefill execution cannot be committed to cache as completed state.

use super::control::DecodeControl;
use crate::gguf_bridge::{PREFILL_CHUNK_SIZE, PREFILL_CHUNK_STACK_FLOATS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefillOutcome {
    /// All prompt tokens were successfully processed through transformer layers.
    Completed { tokens_processed: usize },
    /// Prefill was aborted before finishing due to cancellation signal.
    Cancelled { tokens_processed: usize },
    /// Engine failed to dispatch a prefill chunk.
    Failed { pos: usize, tokens_processed: usize },
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
    let prompt_len = ctx.len();
    if prompt_len <= 1 {
        return PrefillOutcome::NoPrefillNeeded;
    }

    let Some(idx) = tensor_idx else {
        return PrefillOutcome::Failed { pos: 0, tokens_processed: 0 };
    };

    let prefill_tokens = prompt_len - 1;
    let chunk_cap = (PREFILL_CHUNK_STACK_FLOATS / emb_dim.max(1))
        .min(PREFILL_CHUNK_SIZE)
        .max(1);

    let mut pos = 0usize;
    while pos < prefill_tokens {
        if control.is_some_and(DecodeControl::is_cancelled) {
            return PrefillOutcome::Cancelled { tokens_processed: pos };
        }

        let n = (prefill_tokens - pos).min(chunk_cap);
        let batch_elems = n * emb_dim;

        {
            let Some(mmap) = engine.gguf_mmap.as_deref() else {
                return PrefillOutcome::Failed { pos, tokens_processed: pos };
            };
            for t in 0..n {
                let _ = idx.dequantize_token_embedding_into(
                    mmap,
                    ctx[pos + t],
                    &mut prefill_chunk[t * emb_dim..(t + 1) * emb_dim],
                );
            }
        }

        if !engine.dispatch_prefill_chunk(
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
            return PrefillOutcome::Failed { pos, tokens_processed: pos };
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

        let failed = PrefillOutcome::Failed { pos: 2, tokens_processed: 2 };
        assert!(!failed.is_completed());

        let none_needed = PrefillOutcome::NoPrefillNeeded;
        assert!(!none_needed.is_completed());
    }
}
