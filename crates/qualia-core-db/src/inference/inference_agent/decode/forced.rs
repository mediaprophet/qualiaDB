use super::*;

use crate::gguf_sharder::GgufTokenizer;

pub(super) struct Forced<'a, 'slot, 'bytes> {
    pub gate: &'a mut super::super::native_request::PublicationGate<'slot, 'bytes>,
    pub prompt_owned: &'a str,
    pub tok: &'a GgufTokenizer,
    pub control_thread: &'a Option<DecodeControl>,
    pub vlen: u32,
    pub out_ids: &'a mut Vec<u32>,
    pub ctx: &'a mut Vec<u32>,
    pub stream_tx_thread: &'a Option<std::sync::mpsc::SyncSender<String>>,
    pub streamed_len: usize,
    pub gen_budget: usize,
    pub lp: &'a rtrb::Producer<LlmMsg>,
}
pub(super) fn run(input: Forced<'_, '_, '_>) -> (bool, usize) {
    let Forced {
        gate,
        prompt_owned,
        tok,
        control_thread,
        vlen,
        out_ids,
        ctx,
        stream_tx_thread,
        mut streamed_len,
        gen_budget,
        lp,
    } = input;
    // QUALIA_GRAPH_FORCE=1: emit grounded repair tokens without model decode.
    let mut graph_force_emitted = false;
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(forced) = crate::qualia_hybrid::force_fact_tokens(&prompt_owned, &|s| tok.encode(s))
    {
        for &tid in &forced {
            if lp.is_abandoned() {
                gate.invalidate();
                break;
            }
            if control_thread
                .as_ref()
                .is_some_and(DecodeControl::is_cancelled)
            {
                gate.invalidate();
                break;
            }
            let mut accepted = None;
            if gate
                .token(
                    tid % vlen.max(1),
                    ctx.len().saturating_sub(1) as u64,
                    super::super::governed::evidence::CandidateSource::GraphForcedToken,
                    None,
                    tok,
                    &mut accepted,
                )
                .is_err()
            {
                break;
            }
            let next = accepted
                .expect("successful forced token output transfer")
                .token_id();
            out_ids.push(next);
            ctx.push(next);
            if let Some(ref tx) = stream_tx_thread {
                let full = tok.decode(&out_ids);
                if full.len() > streamed_len {
                    let delta = full[streamed_len..].to_string();
                    streamed_len = full.len();
                    if tx.send(delta).is_err() {
                        gate.invalidate();
                        break;
                    }
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
