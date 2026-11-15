use super::{
    buffers::{self, JobBuffers},
    model::PreparedCpuModel,
    numerical,
    request::PreparedRequest,
    session::CpuStepError,
    state::HostState,
    tokenizer::PreparedTokenizer,
    CpuAdmissionError,
};

/// Shared actual prompt prefill. Predictive outputs are discarded, never exposed
/// as accepted model tokens. A late error can leave earlier prompt state; the
/// failed job is discarded, not represented as a whole-prefill rollback.
pub(super) fn prefill(
    model: &PreparedCpuModel<'_, '_>,
    tokenizer: &PreparedTokenizer<'_, '_>,
    request: &PreparedRequest<'_>,
    buffers: &mut JobBuffers<'_>,
    transaction_budget: usize,
) -> Result<HostState, CpuStepError> {
    use CpuStepError::Admission;
    if tokenizer.model != model.model_digest
        || request.model != model.model_digest
        || request.tokenizer != tokenizer.digest
        || tokenizer.plan.vocab_count() != model.plan.config().vocabulary
        || request.context != model.plan.config().max_context
    {
        return Err(Admission(CpuAdmissionError::InvalidIdentity));
    }
    if buffers.output.len() < request.max_new_tokens
        || buffers.piece.len() < tokenizer.plan.max_token_bytes()
    {
        return Err(Admission(CpuAdmissionError::Capacity));
    }
    let first = request
        .tokens
        .len()
        .checked_sub(1)
        .ok_or(Admission(CpuAdmissionError::InvalidRequest))?;
    let end = first
        .checked_add(request.max_new_tokens)
        .ok_or(Admission(CpuAdmissionError::Overflow))?;
    if request.max_new_tokens == 0
        || end > request.context
        || request
            .tokens
            .iter()
            .any(|&t| t as usize >= model.plan.config().vocabulary)
    {
        return Err(Admission(CpuAdmissionError::InvalidRequest));
    }
    if buffers::slices(&buffers.workspace)
        .iter()
        .any(|s| s.iter().any(|v| v.to_bits() != 0))
    {
        return Err(Admission(CpuAdmissionError::InvalidRequest));
    }
    let (&last, prefix) = request
        .tokens
        .split_last()
        .ok_or(Admission(CpuAdmissionError::InvalidRequest))?;
    let mut state = HostState {
        position: 0,
        current_token: last,
        generated: 0,
        stopped: false,
        terminal: false,
    };
    for (position, &token) in prefix.iter().enumerate() {
        state.position = position;
        state.current_token = token;
        numerical::propose_step(
            model,
            &state,
            buffers::reborrow(&mut buffers.workspace),
            &mut *buffers.undo,
            transaction_budget,
        )
        .map_err(CpuStepError::Numerical)?
        .commit();
    }
    state.position = first;
    state.current_token = last;
    Ok(state)
}
