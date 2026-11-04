use super::CpuAdmissionError;
use etalon_inference_contract::CandidateKey;

/// Job-private host state. Output slots and lineage storage are admitted before
/// executing a proposal; failure never advances these values.
pub(super) struct HostState {
    pub position: usize,
    pub current_token: u32,
    pub generated: usize,
    pub stopped: bool,
    pub terminal: bool,
}
impl HostState {
    fn after_token(
        &self,
        token: u32,
        stop: bool,
        max_new_tokens: usize,
        context: usize,
    ) -> Result<Self, CpuAdmissionError> {
        if self.terminal || self.stopped {
            return Err(CpuAdmissionError::Terminal);
        }
        let generated = self
            .generated
            .checked_add(1)
            .ok_or(CpuAdmissionError::Overflow)?;
        let position = self
            .position
            .checked_add(1)
            .ok_or(CpuAdmissionError::Overflow)?;
        if generated > max_new_tokens || position > context {
            return Err(CpuAdmissionError::Capacity);
        }
        Ok(Self {
            position,
            current_token: token,
            generated,
            stopped: stop || generated == max_new_tokens,
            terminal: false,
        })
    }
}
pub(super) struct StagedHost<'a> {
    state: &'a mut HostState,
    next: HostState,
    token_slot: &'a mut u32,
    lineage_slot: &'a mut Option<CandidateKey>,
}
impl<'a> StagedHost<'a> {
    pub(super) fn matches(&self, token: u32, position: u64) -> bool {
        self.next.current_token == token && u64::try_from(self.state.position) == Ok(position)
    }
    pub(super) fn prepare(
        state: &'a mut HostState,
        token_slot: &'a mut u32,
        lineage_slot: &'a mut Option<CandidateKey>,
        token: u32,
        stop: bool,
        max_new_tokens: usize,
        context: usize,
    ) -> Result<Self, CpuAdmissionError> {
        if lineage_slot.is_some() {
            return Err(CpuAdmissionError::Capacity);
        }
        let next = state.after_token(token, stop, max_new_tokens, context)?;
        Ok(Self {
            state,
            next,
            token_slot,
            lineage_slot,
        })
    }
    /// Only called after the actual contract has committed under the same gate.
    /// References are prevalidated; this has no indexing, allocation or callback.
    pub(super) fn finish(self, key: CandidateKey) {
        *self.token_slot = self.next.current_token;
        *self.lineage_slot = Some(key);
        *self.state = self.next;
    }
}

pub(super) struct StagedDeveloperHost<'a> {
    state: &'a mut HostState,
    next: HostState,
    token_slot: &'a mut u32,
}
impl<'a> StagedDeveloperHost<'a> {
    pub(super) fn prepare(
        state: &'a mut HostState,
        token_slot: &'a mut u32,
        token: u32,
        stop: bool,
        max_new_tokens: usize,
        context: usize,
    ) -> Result<Self, CpuAdmissionError> {
        let next = state.after_token(token, stop, max_new_tokens, context)?;
        Ok(Self {
            state,
            next,
            token_slot,
        })
    }
    pub(super) fn finish(self) {
        *self.token_slot = self.next.current_token;
        *self.state = self.next;
    }
}
