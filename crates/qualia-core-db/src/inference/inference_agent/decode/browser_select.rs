use super::*;
use crate::gguf_bridge::{QTensor, QTensorEngine};
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
        mut sieve_failed,
    } = input;
    // 1) Embedding lookup → hidden state (stack dequant).
    let hidden_ok = tensor_idx
        .as_ref()
        .and_then(|idx| {
            engine
                .model_bytes()
                .map(|m| idx.dequantize_token_embedding_into(m, cur, &mut emb_buf[..emb_dim]))
        })
        .unwrap_or(0);

    // 1b) LoRA delta — additive correction to the embedding vector.
    // Applied after dequantize so the base model is unmodified.
    // Silently skipped if dimensions don't match (wrong adapter for model).
    if hidden_ok > 0 {
        if let Some(ref adapter) = lora_adapter {
            if adapter.meta.n_in == hidden_ok && adapter.meta.n_out == hidden_ok {
                let snap: Vec<f32> = emb_buf[..hidden_ok].to_vec();
                let _ = adapter.apply_cpu(&snap, &mut emb_buf[..hidden_ok]);
            }
        }
    }

    let (top_i, top_v) = if hidden_ok > 0 {
        if let Some(idx) = tensor_idx.as_ref() {
            let token_idx = ctx.len().saturating_sub(1) as u32;
            // CUDA mega-pass: attempt single-fence all-layer forward.
            // Returns Some(u32::MAX) sentinel when forward is done but logits
            // still needed (hidden state read back into emb_buf).
            #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
            let mega_token: Option<u32> =
                engine.try_cuda_mega_pass_decode(idx, &mut emb_buf[..emb_dim], emb_dim, token_idx);
            #[cfg(any(target_arch = "wasm32", not(feature = "cuda")))]
            let mega_token: Option<u32> = None;

            if let Some(mt) = mega_token.filter(|&t| t != u32::MAX) {
                (mt as usize, 0.0f32)
            } else if mega_token.is_some() {
                // Sentinel: forward done, logits needed.
                let sieve_mask = sieve.as_ref().map(|s| s.current_mask());
                if let Some(argmax) = engine.dispatch_output_argmax_chunked(
                    idx,
                    &emb_buf[..emb_dim],
                    emb_dim,
                    &mut scratch_a[..],
                    TEST_VOCAB_CHUNK_CAP,
                    sieve_mask,
                ) {
                    if argmax.max_logit > f32::NEG_INFINITY {
                        (argmax.best_token_id as usize, argmax.max_logit)
                    } else {
                        sieve_failed = true;
                        (0usize, f32::NEG_INFINITY)
                    }
                } else if let Some(tok) = engine.browser_top1_readback_bytes(vlen as usize) {
                    (tok as usize, 0.0)
                } else {
                    emb_buf[..emb_dim].iter().enumerate().fold(
                        (0usize, f32::NEG_INFINITY),
                        |(bi, bv), (i, &v)| {
                            if v > bv {
                                (i, v)
                            } else {
                                (bi, bv)
                            }
                        },
                    )
                }
            } else {
                let _layers = engine.dispatch_transformer_forward(
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
                if let Some(argmax) = engine.dispatch_output_argmax_chunked(
                    idx,
                    &emb_buf[..emb_dim],
                    emb_dim,
                    &mut scratch_a[..],
                    TEST_VOCAB_CHUNK_CAP,
                    sieve_mask,
                ) {
                    if argmax.max_logit > f32::NEG_INFINITY {
                        (argmax.best_token_id as usize, argmax.max_logit)
                    } else {
                        sieve_failed = true;
                        (0usize, f32::NEG_INFINITY)
                    }
                } else if let Some(tok) = engine.browser_top1_readback_bytes(vlen as usize) {
                    (tok as usize, 0.0)
                } else {
                    emb_buf[..emb_dim].iter().enumerate().fold(
                        (0usize, f32::NEG_INFINITY),
                        |(bi, bv), (i, &v)| {
                            if v > bv {
                                (i, v)
                            } else {
                                (bi, bv)
                            }
                        },
                    )
                }
            }
        } else {
            (0usize, 0.0)
        }
    } else {
        let wt = QTensor::new(vec![emb_dim, emb_dim], 0, true);
        let logits = embedding_fallback_logits(
            &engine,
            tensor_idx.as_ref(),
            lora_adapter.as_ref(),
            cur,
            emb_dim,
            &mut emb_buf[..],
            &wt,
        );
        logits
            .iter()
            .enumerate()
            .fold((0usize, f32::NEG_INFINITY), |(bi, bv), (i, &v)| {
                if v > bv {
                    (i, v)
                } else {
                    (bi, bv)
                }
            })
    };

    // anomaly 0x01 = normal. Removed IEEE mantissa 0x99 check (random ~1/256 fire).
    let anomaly = 0x01u8;
    let _ = anomaly; // reserved for real governance signals

    (top_i, top_v, sieve_failed)
}
