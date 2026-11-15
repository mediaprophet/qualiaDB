//! Shared prepared scalar runner and live host acceptance. This additive entrypoint
//! awaits consuming and independent qualification; legacy callers remain separate.
use super::{
    buffers::{self, JobBuffers},
    commit::DenseAcceptanceCommit,
    executor::VerifiedCpuExecutor,
    model::PreparedCpuModel,
    numerical::{self, StepError},
    observation::{self, EventContext},
    request::PreparedRequest,
    state::{HostState, StagedHost},
    tokenizer::PreparedTokenizer,
    CpuAdmissionError,
};
use crate::inference::inference_agent::governed::{
    evidence::{AcceptedEvidence, AcceptedKey, GovernedEvidenceSession},
    PolicyError,
};
use etalon_inference_contract::SessionIdentity;
use etalon_model::tokenizer::{ExecutionBudget, TokenizerError};
use sha2::{Digest, Sha256};

/// Returned only by the existing opaque authority session. Metadata for adapter
/// matching, not a live grant; prepare and acceptance still revalidate authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::inference::inference_agent) struct CpuProjectionBinding {
    pub identity: SessionIdentity,
    pub request_digest: [u8; 32],
    pub actor: u64,
    pub graph: u64,
    pub max_payload: usize,
    pub first_position: u64,
}

#[derive(Debug)]
pub enum CpuStepError {
    Admission(CpuAdmissionError),
    Numerical(StepError),
    Tokenizer(TokenizerError),
    Policy(PolicyError),
}

pub struct GovernedCpuSession<'model, 'data, 'tables, 'slot, 'bytes, 'buf> {
    model: &'model PreparedCpuModel<'data, 'tables>,
    tokenizer: &'model PreparedTokenizer<'data, 'tables>,
    request: PreparedRequest<'model>,
    _executor: &'model VerifiedCpuExecutor,
    authority: GovernedEvidenceSession<'slot, 'bytes>,
    binding: CpuProjectionBinding,
    buffers: JobBuffers<'buf>,
    transaction_budget: usize,
    decode_budget: ExecutionBudget,
    provenance: [u8; 32],
    state: HostState,
    terminal: bool,
    #[cfg(test)]
    before_accept: Option<Box<dyn FnOnce() + Send>>,
}

impl<'m, 'd, 't, 's, 'b, 'w> GovernedCpuSession<'m, 'd, 't, 's, 'b, 'w> {
    /// All objects remain exclusively owned/borrowed for the job. Prompt numerical
    /// prefill is private; its predicted outputs are neither accepted nor exposed.
    pub(super) fn begin(
        model: &'m PreparedCpuModel<'d, 't>,
        tokenizer: &'m PreparedTokenizer<'d, 't>,
        request: PreparedRequest<'m>,
        executor: &'m VerifiedCpuExecutor,
        authority: GovernedEvidenceSession<'s, 'b>,
        mut buffers: JobBuffers<'w>,
        execution_bytes: usize,
        decode_budget: ExecutionBudget,
    ) -> Result<Self, CpuStepError> {
        use CpuStepError::Admission;
        let binding = authority
            .cpu_projection_binding()
            .map_err(CpuStepError::Policy)?;
        let transaction_budget =
            buffers::admit_buffers(&buffers, execution_bytes).map_err(Admission)?;
        if binding.identity.model_id != model.model_digest
            || tokenizer.model != model.model_digest
            || request.model != model.model_digest
            || request.tokenizer != tokenizer.digest
            || request.executor != executor.digest()
            || binding.request_digest != request.digest
            || binding.identity.device_id != executor.device_id()
            || binding.identity.device_generation != executor.generation()
            || tokenizer.plan.vocab_count() != model.plan.config().vocabulary
            || request.context != model.plan.config().max_context
        {
            return Err(Admission(CpuAdmissionError::InvalidIdentity));
        }
        if buffers.output.len() < request.max_new_tokens
            || buffers.lineage.len() < request.max_new_tokens
            || buffers.piece.len() < tokenizer.plan.max_token_bytes()
            || binding.max_payload < tokenizer.plan.max_token_bytes()
            || decode_budget.capacity_bytes < buffers.piece.len()
            || buffers.lineage.iter().any(Option::is_some)
        {
            return Err(Admission(CpuAdmissionError::Capacity));
        }
        let state = super::initialize::prefill(
            model,
            tokenizer,
            &request,
            &mut buffers,
            transaction_budget,
        )?;
        #[cfg(test)]
        super::test_hooks::after_prefill();
        // Prefill is bounded but can outlast a grant. Recheck every current
        // authority predicate before returning a runnable session, even though
        // no generated proposal or output has yet been published.
        if authority
            .cpu_projection_binding()
            .map_err(CpuStepError::Policy)?
            != binding
        {
            return Err(Admission(CpuAdmissionError::InvalidIdentity));
        }
        let first =
            u64::try_from(state.position).map_err(|_| Admission(CpuAdmissionError::Overflow))?;
        let intent = request
            .intent
            .ok_or(Admission(CpuAdmissionError::InvalidRequest))?;
        if binding.first_position != first
            || binding.graph != intent.graph_scope
            || binding.identity.model_generation != intent.model_generation
        {
            return Err(Admission(CpuAdmissionError::InvalidIdentity));
        }
        let mut hash = Sha256::new();
        hash.update(b"qualia:governed-cpu:actual-provider:v1\0");
        hash.update(model.model_digest);
        hash.update(tokenizer.digest);
        hash.update(request.digest);
        hash.update(executor.digest());
        let provenance = hash.finalize().into();
        Ok(Self {
            model,
            tokenizer,
            request,
            _executor: executor,
            authority,
            binding,
            buffers,
            transaction_budget,
            decode_budget,
            provenance,
            state,
            terminal: false,
            #[cfg(test)]
            before_accept: None,
        })
    }

    pub fn is_finished(&self) -> bool {
        self.state.stopped
    }
    pub fn is_terminal(&self) -> bool {
        self.terminal || self.authority.is_terminal()
    }
    pub fn cancel(&mut self) {
        self.terminal = true;
        self.authority.cancel();
    }
    /// Test-only injection outside authority gates, after genuine preparation.
    #[cfg(test)]
    pub(super) fn before_accept_once(&mut self, hook: impl FnOnce() + Send + 'static) {
        self.before_accept = Some(Box::new(hook));
    }
    #[cfg(test)]
    pub(crate) fn numerical_state(&self) -> [&[f32]; 13] {
        buffers::slices(&self.buffers.workspace)
    }
    pub fn generated_tokens(&self) -> usize {
        self.state.generated
    }

    /// Includes EOS/control/empty pieces: no stop is committed before approval.
    /// Every error/unwind leaves the job terminal, even when numerical undo works.
    pub fn next(&mut self) -> Result<AcceptedCpuToken<'_, 'b>, CpuStepError> {
        use CpuStepError::Admission;
        if self.terminal || self.authority.is_terminal() || self.state.stopped {
            return Err(Admission(CpuAdmissionError::Terminal));
        }
        self.terminal = true;
        let index = self.state.generated;
        let slot = self
            .buffers
            .output
            .get_mut(index)
            .ok_or(Admission(CpuAdmissionError::Capacity))?;
        let lineage = self
            .buffers
            .lineage
            .get_mut(index)
            .ok_or(Admission(CpuAdmissionError::Capacity))?;
        let sequence = u64::try_from(index)
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or(Admission(CpuAdmissionError::Overflow))?;
        let position = u64::try_from(self.state.position)
            .map_err(|_| Admission(CpuAdmissionError::Overflow))?;
        let pending = numerical::propose_step(
            self.model,
            &self.state,
            buffers::reborrow(&mut self.buffers.workspace),
            &mut *self.buffers.undo,
            self.transaction_budget,
        )
        .map_err(CpuStepError::Numerical)?;
        let output = pending.output();
        let length = self
            .tokenizer
            .plan
            .decode_token_bytes_into(
                output.token_id,
                &mut *self.buffers.piece,
                self.decode_budget,
            )
            .map_err(CpuStepError::Tokenizer)?;
        let piece = self
            .buffers
            .piece
            .get(..length)
            .ok_or(Admission(CpuAdmissionError::Capacity))?;
        let proposal = observation::token_proposal(
            EventContext {
                identity: self.binding.identity,
                request: self.request.digest,
                actor: self.binding.actor,
                graph: self.binding.graph,
                sequence,
                position,
                provenance: self.provenance,
            },
            output.token_id,
            output.max_logit,
            piece,
        )
        .map_err(Admission)?;
        let stop = self.request.stops.contains(&output.token_id);
        let staged = StagedHost::prepare(
            &mut self.state,
            slot,
            lineage,
            output.token_id,
            stop,
            self.request.max_new_tokens,
            self.request.context,
        )
        .map_err(Admission)?;
        let commit = DenseAcceptanceCommit::bind(
            pending,
            staged,
            proposal,
            self.binding.max_payload as u64,
            self.model,
            &self.request,
        )
        .map_err(Admission)?;
        let ticket = self
            .authority
            .prepare_token(proposal)
            .map_err(CpuStepError::Policy)?;
        #[cfg(test)]
        if let Some(hook) = self.before_accept.take() {
            hook();
        }
        let record = self
            .authority
            .accept_dense(ticket, commit)
            .map_err(CpuStepError::Policy)?;
        Ok(AcceptedCpuToken {
            record,
            terminal: &mut self.terminal,
            output,
            position,
        })
    }
}

/// No user callback runs in the acceptance gate. Dropping this guard leaves the
/// job terminal and lets the underlying receipt invalidate its retained slot.
pub struct AcceptedCpuToken<'drain, 'bytes> {
    record: AcceptedEvidence<'drain, 'bytes>,
    terminal: &'drain mut bool,
    output: qualia_inference_kernel::dense_cpu::DenseStep,
    position: u64,
}
impl AcceptedCpuToken<'_, '_> {
    pub fn output(&self) -> qualia_inference_kernel::dense_cpu::DenseStep {
        self.output
    }
    pub fn position(&self) -> u64 {
        self.position
    }
    pub fn bytes(&self) -> &[u8] {
        self.record.bytes()
    }
    pub fn key(&self) -> AcceptedKey {
        self.record.key()
    }
    pub fn complete(self, delivered_bytes: usize) -> Result<(), PolicyError> {
        self.record.complete(delivered_bytes)?;
        *self.terminal = false;
        Ok(())
    }
}
