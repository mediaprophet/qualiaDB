use super::*;

use crate::gguf_sharder::GgufTokenizer;

pub(super) struct Forced<'a> {
    pub prompt_owned: &'a str,
    pub tok: &'a GgufTokenizer,
    pub control_thread: &'a Option<DecodeControl>,
    pub vlen: u32,
    pub out_ids: &'a mut Vec<u32>,
    pub ctx: &'a mut Vec<u32>,
    pub stream_tx_thread: &'a Option<std::sync::mpsc::SyncSender<String>>,
    pub streamed_len: usize,
    pub gen_budget: usize,
}
pub(super) fn run(input: Forced<'_>) -> (bool, usize) {
    let Forced {
        prompt_owned,
        tok,
        control_thread,
        vlen,
        out_ids,
        ctx,
        stream_tx_thread,
        mut streamed_len,
        gen_budget,
    } = input;
    // QUALIA_GRAPH_FORCE=1: emit grounded repair tokens without model decode.
    let mut graph_force_emitted = false;
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(forced) = crate::qualia_hybrid::force_fact_tokens(&prompt_owned, &|s| tok.encode(s))
    {
        for &tid in &forced {
            if control_thread
                .as_ref()
                .is_some_and(DecodeControl::is_cancelled)
            {
                break;
            }
            let next = tid % vlen.max(1);
            out_ids.push(next);
            ctx.push(next);
            if let Some(ref tx) = stream_tx_thread {
                let full = tok.decode(&out_ids);
                if full.len() > streamed_len {
                    let delta = full[streamed_len..].to_string();
                    streamed_len = full.len();
                    let _ = tx.send(delta);
                }
            }
            let fixed = crate::llm_bench::decode_budget_fixed_tokens();
            if out_ids.len() >= gen_budget || (!fixed && tok.is_stop_token(next)) {
                break;
            }
        }
        graph_force_emitted = true;
    }

    (graph_force_emitted, streamed_len)
}
