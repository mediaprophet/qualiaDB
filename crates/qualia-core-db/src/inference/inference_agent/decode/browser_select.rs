use super::*;
use crate::gguf_bridge::QTensorEngine;
use crate::gguf_sharder::GgufTensorIndex;

pub(super) struct BrowserSelection<'a> {
    pub engine: &'a mut QTensorEngine,
    pub tensor_idx: &'a Option<GgufTensorIndex>,
    pub ctx: &'a Vec<u32>,
    pub cur: u32,
    pub emb_dim: usize,
    pub emb_buf: &'a mut [f32; 8192],
    pub scratch_a: &'a mut [f32; 16384],
    pub scratch_b: &'a mut [f32; 16384],
    pub lora_adapter: &'a Option<crate::lora::LoRAAdapter>,
    pub sieve: &'a Option<crate::neuro_symbolic_sieve::NeuroSymbolicSieve>,
    pub vlen: u32,
    pub sieve_failed: bool,
}

pub(super) fn run(input: BrowserSelection<'_>) -> (usize, f32, bool) {
    let BrowserSelection {
        engine,
        tensor_idx,
        ctx,
        cur,
        emb_dim,
        emb_buf,
        scratch_a,
        scratch_b,
        lora_adapter,
        sieve,
        vlen,
        sieve_failed,
    } = input;
    let failed = (0, f32::NEG_INFINITY, true);
    let Some(idx) = tensor_idx.as_ref() else {
        return failed;
    };
    let hidden_len = engine
        .model_bytes()
        .map(|m| idx.dequantize_token_embedding_into(m, cur, &mut emb_buf[..emb_dim]))
        .unwrap_or(0);
    if hidden_len != emb_dim || hidden_len == 0 {
        return failed;
    }

    if let Some(adapter) = lora_adapter {
        if adapter.meta.n_in == hidden_len && adapter.meta.n_out == hidden_len {
            let snap = *emb_buf;
            let _ = adapter.apply_cpu(&snap[..hidden_len], &mut emb_buf[..hidden_len]);
        }
    }

    let token_idx = ctx.len().saturating_sub(1) as u32;
    engine.dispatch_transformer_forward(
        idx,
        &mut emb_buf[..emb_dim],
        emb_dim,
        scratch_a,
        scratch_b,
        token_idx,
        TEST_TRANSFORMER_LAYER_CAP,
    );
    let _ = engine.apply_output_norm_inplace(idx, &mut emb_buf[..emb_dim], emb_dim);
    let sieve_mask = sieve.as_ref().map(|s| s.current_mask());
    let projected = engine.dispatch_output_argmax_chunked(
        idx,
        &emb_buf[..emb_dim],
        emb_dim,
        &mut scratch_a[..],
        TEST_VOCAB_CHUNK_CAP,
        sieve_mask,
    );
    // Legacy code treated a readback byte count or hidden-state coordinate as a
    // vocabulary token. Neither is a projection result. Synchronous browser GPU
    // dispatch may lack readback support; fail closed and use the async exports.
    match projected {
        Some(argmax) if argmax.max_logit.is_finite() && argmax.best_token_id < vlen => (
            argmax.best_token_id as usize,
            argmax.max_logit,
            sieve_failed,
        ),
        _ => failed,
    }
}
