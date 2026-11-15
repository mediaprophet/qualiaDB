//! Admission adapter only. All preparation/encoding/decoding is the actual
//! borrowed Etalon tokenizer; no legacy fallback or duplicate BPE implementation.
use super::{model::ImmutableModel, request::*, CpuAdmissionError};
use etalon_model::{
    gguf::{MetadataView, ParseError, ParseLimits},
    tokenizer::{
        self, ExecutionBudget, ExecutionWorkspace, PrepareBudget, TokenizerError, TokenizerPlan,
        TokenizerProfile, TokenizerTables,
    },
};
use sha2::{Digest, Sha256};

pub(super) struct PreparedTokenizer<'data, 'tables> {
    pub(super) plan: TokenizerPlan<'data, 'tables>,
    pub(super) digest: [u8; 32],
    pub(super) model: [u8; 32],
}
#[derive(Debug)]
pub enum TokenizerAdmissionError {
    Parse(ParseError),
    Tokenizer(TokenizerError),
    Admission(CpuAdmissionError),
}

pub(super) fn prepare_tokenizer<'data, 'tables>(
    owner: &'data ImmutableModel<'_>,
    tables: TokenizerTables<'tables, 'data>,
    parse: ParseLimits,
    budget: PrepareBudget,
) -> Result<PreparedTokenizer<'data, 'tables>, TokenizerAdmissionError> {
    let view = MetadataView::parse(owner.bytes(), parse).map_err(TokenizerAdmissionError::Parse)?;
    let plan =
        tokenizer::prepare(&view, tables, budget).map_err(TokenizerAdmissionError::Tokenizer)?;
    let model = owner.identity().content_sha256;
    let mut hash = Sha256::new();
    hash.update(b"qualia:governed-cpu:embedded-tokenizer:v1\0");
    hash.update(tokenizer::TOKENIZER_SEMANTICS_VERSION.to_le_bytes());
    hash.update(model);
    hash.update(owner.identity().byte_len.to_le_bytes());
    hash.update([match plan.profile() {
        TokenizerProfile::Gpt2 => 1,
        TokenizerProfile::Smollm => 2,
    }]);
    hash.update(plan.bos_token_id().to_le_bytes());
    hash.update(plan.eos_token_id().to_le_bytes());
    hash.update([u8::from(plan.add_bos_token())]);
    hash.update((plan.vocab_count() as u64).to_le_bytes());
    // The same complete immutable GGUF hash already binds exact tokens, merges,
    // flags and all metadata bytes. No second unmetered metadata scan is needed.
    Ok(PreparedTokenizer {
        plan,
        model,
        digest: hash.finalize().into(),
    })
}

#[derive(Clone, Copy)]
pub struct PromptInput<'a> {
    pub text: &'a str,
    pub stops: &'a [u32],
    pub max_new_tokens: usize,
    pub context: usize,
    pub capabilities: super::capabilities::RequestedCapabilities,
}

/// Conservative output admission requires one ID per input byte plus possible
/// BOS before mutation. Encoded output has no implicit template/stop insertion.
/// Late work errors invalidate caller scratch/output and produce no request.
pub(super) fn prepare_prompt<'a>(
    tokenizer: &PreparedTokenizer<'_, '_>,
    input: PromptInput<'a>,
    workspace: ExecutionWorkspace<'_>,
    out: &'a mut [u32],
    execution: ExecutionBudget,
    request_budget: RequestBudget,
) -> Result<RawRequest<'a>, TokenizerAdmissionError> {
    use TokenizerAdmissionError::Admission;
    input.capabilities.validate().map_err(Admission)?;
    if input.text.len() > request_budget.max_text_bytes
        || input.stops.len() > request_budget.max_stop_ids
    {
        return Err(Admission(CpuAdmissionError::Budget));
    }
    if input
        .stops
        .iter()
        .any(|&id| id as usize >= tokenizer.plan.vocab_count())
        || input.max_new_tokens == 0
    {
        return Err(Admission(CpuAdmissionError::InvalidRequest));
    }
    let worst = input
        .text
        .len()
        .checked_add(usize::from(tokenizer.plan.add_bos_token()))
        .ok_or(Admission(CpuAdmissionError::Overflow))?;
    if out.len() < worst {
        return Err(Admission(CpuAdmissionError::Capacity));
    }
    let capacity = out
        .len()
        .checked_add(input.stops.len())
        .and_then(|n| n.checked_mul(4))
        .and_then(|n| n.checked_add(input.text.len()))
        .ok_or(Admission(CpuAdmissionError::Overflow))?;
    if capacity > request_budget.capacity_bytes {
        return Err(Admission(CpuAdmissionError::Budget));
    }
    let written = tokenizer
        .plan
        .encode_prompt_into(input.text, workspace, out, execution)
        .map_err(TokenizerAdmissionError::Tokenizer)?;
    Ok(RawRequest {
        text: input.text.as_bytes(),
        encoded: &out[..written],
        stops: input.stops,
        max_new_tokens: input.max_new_tokens,
        context: input.context,
        add_bos: tokenizer.plan.add_bos_token(),
        capabilities: input.capabilities,
    })
}
