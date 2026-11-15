use super::{capabilities::RequestedCapabilities, CpuAdmissionError};
use sha2::{Digest, Sha256};

pub const EXECUTOR_VERSION: u32 = 1;
pub const PROJECTION_VERSION: u32 = 1;
/// Approve and expose exact decoded stop-token bytes, THEN stop generation.
/// This is not the legacy chat-display filtering or heuristic stop-set contract.
pub const RAW_STOP_DISPOSITION_VERSION: u32 = 1;

/// Explicit raw-completion profile; no internal chat template is executed.
/// Callers supplying already rendered text must bind its template separately
/// through a later supported profile, not label it as automatic chat support.
#[derive(Clone, Copy)]
pub struct RawRequest<'a> {
    pub text: &'a [u8],
    pub encoded: &'a [u32],
    pub stops: &'a [u32],
    pub max_new_tokens: usize,
    pub context: usize,
    pub add_bos: bool,
    pub capabilities: RequestedCapabilities,
}
#[derive(Clone, Copy)]
pub struct RequestBudget {
    pub max_text_bytes: usize,
    pub max_prompt_tokens: usize,
    pub max_stop_ids: usize,
    pub capacity_bytes: usize,
}

/// Immutable interpretation of already validated and encoded input. Construction
/// is private to the adapter: arbitrary callers cannot assert tokenizer parity.
#[derive(Clone, Copy)]
pub(super) struct PreparedRequest<'a> {
    pub(super) _text: &'a [u8],
    pub(super) tokens: &'a [u32],
    pub(super) stops: &'a [u32],
    pub(super) digest: [u8; 32],
    pub(super) model: [u8; 32],
    pub(super) tokenizer: [u8; 32],
    pub(super) executor: [u8; 32],
    pub(super) max_new_tokens: usize,
    pub(super) context: usize,
    pub(super) intent: Option<RequestIntent>,
}

#[derive(Clone, Copy, Debug)]
pub struct RequestIntent {
    pub model_generation: u64,
    pub graph_scope: u64,
    pub instrumentation: etalon_inference_contract::Instrumentation,
}

/// Called only after neutral tokenizer preparation and actual encoding. The
/// complete supplied slices are charged even if another caller retained slack.
pub(super) fn bind_encoded<'a>(
    input: RawRequest<'a>,
    model: [u8; 32],
    tokenizer: [u8; 32],
    executor: [u8; 32],
    vocabulary: usize,
    budget: RequestBudget,
) -> Result<PreparedRequest<'a>, CpuAdmissionError> {
    bind_mode(input, model, tokenizer, executor, vocabulary, budget, true)
}

pub(super) fn bind_developer<'a>(
    input: RawRequest<'a>,
    model: [u8; 32],
    tokenizer: [u8; 32],
    vocabulary: usize,
    budget: RequestBudget,
) -> Result<PreparedRequest<'a>, CpuAdmissionError> {
    bind_mode(input, model, tokenizer, [0; 32], vocabulary, budget, false)
}

fn bind_mode<'a>(
    input: RawRequest<'a>,
    model: [u8; 32],
    tokenizer: [u8; 32],
    executor: [u8; 32],
    vocabulary: usize,
    budget: RequestBudget,
    governed: bool,
) -> Result<PreparedRequest<'a>, CpuAdmissionError> {
    input.capabilities.validate()?;
    let arrays = input
        .encoded
        .len()
        .checked_add(input.stops.len())
        .and_then(|n| n.checked_mul(core::mem::size_of::<u32>()))
        .ok_or(CpuAdmissionError::Overflow)?;
    let capacity = arrays
        .checked_add(input.text.len())
        .ok_or(CpuAdmissionError::Overflow)?;
    if capacity > budget.capacity_bytes
        || input.text.len() > budget.max_text_bytes
        || input.encoded.len() > budget.max_prompt_tokens
        || input.stops.len() > budget.max_stop_ids
    {
        return Err(CpuAdmissionError::Budget);
    }
    if model == [0; 32] || tokenizer == [0; 32] || (governed && executor == [0; 32]) {
        return Err(CpuAdmissionError::InvalidIdentity);
    }
    let last_position = input
        .encoded
        .len()
        .checked_add(input.max_new_tokens)
        .and_then(|n| n.checked_sub(1))
        .ok_or(CpuAdmissionError::Overflow)?;
    if input.encoded.is_empty()
        || input.max_new_tokens == 0
        || last_position > input.context
        || input
            .encoded
            .iter()
            .chain(input.stops)
            .any(|&t| t as usize >= vocabulary)
        || core::str::from_utf8(input.text).is_err()
    {
        return Err(CpuAdmissionError::InvalidRequest);
    }
    // Preserve exact ordered stop policy, including explicit empty stop list.
    // EOS metadata alone is not the existing instruct heuristic stop set.
    let mut hash = Sha256::new();
    hash.update(if governed {
        &b"qualia:governed-cpu:raw-request:v1\0"[..]
    } else {
        &b"qualia:standalone-cpu:raw-request:v1\0"[..]
    });
    hash.update(EXECUTOR_VERSION.to_le_bytes());
    hash.update(PROJECTION_VERSION.to_le_bytes());
    hash.update(RAW_STOP_DISPOSITION_VERSION.to_le_bytes());
    hash.update(etalon_model::tokenizer::TOKENIZER_SEMANTICS_VERSION.to_le_bytes());
    hash.update(model);
    hash.update(tokenizer);
    hash.update(executor);
    hash.update((input.context as u64).to_le_bytes());
    hash.update((input.max_new_tokens as u64).to_le_bytes());
    hash.update([u8::from(input.add_bos)]);
    hash.update((input.text.len() as u64).to_le_bytes());
    hash.update(input.text);
    for ids in [input.encoded, input.stops] {
        hash.update((ids.len() as u64).to_le_bytes());
        for id in ids {
            hash.update(id.to_le_bytes());
        }
    }
    Ok(PreparedRequest {
        _text: input.text,
        tokens: input.encoded,
        stops: input.stops,
        digest: hash.finalize().into(),
        model,
        tokenizer,
        executor,
        max_new_tokens: input.max_new_tokens,
        context: input.context,
        intent: None,
    })
}

/// Completes the signed request identity before the operator installs policy.
/// The first prediction position is derived from actual encoded prompt length.
pub(super) fn bind_intent(
    mut request: PreparedRequest<'_>,
    intent: RequestIntent,
) -> Result<PreparedRequest<'_>, CpuAdmissionError> {
    if intent.model_generation == 0 || intent.graph_scope == 0 || request.executor == [0; 32] {
        return Err(CpuAdmissionError::InvalidIdentity);
    }
    let first = request
        .tokens
        .len()
        .checked_sub(1)
        .ok_or(CpuAdmissionError::InvalidRequest)?;
    let mut h = Sha256::new();
    h.update(b"qualia:governed-cpu:complete-request:v1\0");
    h.update(request.digest);
    h.update(intent.model_generation.to_le_bytes());
    h.update(intent.graph_scope.to_le_bytes());
    h.update((first as u64).to_le_bytes());
    h.update([
        u8::from(intent.instrumentation.tracing),
        u8::from(intent.instrumentation.profiling),
    ]);
    request.digest = h.finalize().into();
    request.intent = Some(intent);
    Ok(request)
}
