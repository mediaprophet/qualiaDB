//! Owned grammar/mask preparation before native admission or numerical prefill.
use super::super::{governed::PolicyError, native_request::NativeDecodeRequest};
use super::*;
use crate::inference::inference_bench::prepared_domino::PreparedDominoScope;
use crate::{
    gguf_sharder::GgufTokenizer,
    neuro_symbolic_sieve::{NeuroSymbolicSieve, SieveLexSpec},
};

pub(super) struct Inputs {
    pub ctx: Vec<u32>,
    pub sieve: Option<NeuroSymbolicSieve>,
    pub gen_budget: usize,
    pub grammar_scope: Option<PreparedDominoScope>,
}
pub(super) fn prepare(
    execution: &NativeDecodeRequest,
    tok: &GgufTokenizer,
    prompt: &str,
    use_sieve: bool,
    spec: Option<&SieveLexSpec>,
    lex_path: Option<&str>,
    control: Option<&DecodeControl>,
) -> Result<Inputs, PolicyError> {
    let ctx = tok.encode_chat_prompt(prompt);
    let sieve = if use_sieve {
        build_sieve(tok, spec, lex_path)
    } else {
        None
    };
    let gen_budget = admission::generation_budget(sieve.is_some(), control);
    let grammar_scope = if let Some(budget) = execution.input_budget() {
        Some(
            crate::llm_bench::prepare_domino_scope(budget.domino).map_err(|error| {
                log::error!("LLM_DECODE|grammar-preparation-refused|{error}");
                PolicyError::Budget
            })?,
        )
    } else {
        None
    };
    Ok(Inputs {
        ctx,
        sieve,
        gen_budget,
        grammar_scope,
    })
}
