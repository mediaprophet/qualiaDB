use crate::gguf_bridge::QTensorEngine;
use crate::gguf_sharder::{GgufTensorIndex, GgufTokenizer};

pub(super) struct Diagnostic<'a> {
    pub step: usize,
    pub eos: u32,
    pub vlen: u32,
    pub cur: u32,
    pub top_i: usize,
    pub top_v: f32,
    pub tok: &'a GgufTokenizer,
    pub engine: &'a mut QTensorEngine,
    pub tensor_idx: &'a Option<GgufTensorIndex>,
    pub emb_buf: &'a [f32; 8192],
    pub emb_dim: usize,
}
pub(super) fn run(input: Diagnostic<'_>) -> () {
    let Diagnostic {
        step,
        eos,
        vlen,
        cur,
        top_i,
        top_v,
        tok,
        engine,
        tensor_idx,
        emb_buf,
        emb_dim,
    } = input;
    // #48 diagnostic: reveal eos vs argmax for the first step (gated, native).
    if step == 0 && std::env::var("QUALIA_LLM_DEBUG_DECODE").is_ok() {
        eprintln!(
            "[decode-dbg] step0 eos={} vlen={} prompt_last={} top_i={} top_v={} decoded={:?}",
            eos,
            vlen,
            cur,
            top_i,
            top_v,
            tok.decode(&[top_i as u32])
        );
        #[cfg(feature = "gpu-runtime")]
        if let Some(idx) = tensor_idx.as_ref() {
            if let Some(top5) =
                engine.dispatch_output_topk_chunked(idx, &emb_buf[..emb_dim], emb_dim, 5)
            {
                for it in &top5 {
                    eprintln!(
                        "[top5] id={} logit={:.3} dec={:?}",
                        it.token_id,
                        it.logit,
                        tok.decode(&[it.token_id])
                    );
                }
            }
        }
    }
}
