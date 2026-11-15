//! Callable phased host API. Preparation produces a request for operator review;
//! only the existing signed policy host can admit it. No policy is auto-installed.
use super::{
    budget::{self, CpuBudgetReceipt, CpuLimits, CpuTables, PromptWorkspace},
    buffers::JobBuffers,
    executor::{self, ExpectedCpuArtifact, VerifiedCpuExecutor},
    model::ImmutableModel,
    preparation::{self, CpuPreparationError, PreparedAssets},
    request::{self, PreparedRequest, RequestBudget, RequestIntent},
    session::{CpuStepError, GovernedCpuSession},
    tokenizer::{self, PromptInput},
    CpuAdmissionError,
};
use crate::{
    inference::inference_agent::governed::{
        evidence::{EvidenceLimits, EvidenceStorage},
        DigestScope, InferenceAdmission, NativePolicyHost, PolicyError, POLICY_COLD_BUDGET_BYTES,
    },
    poet_host::native_authority::{inference::AuthenticatedInferenceAssociation, NativeAuthority},
};
use ed25519_dalek::SigningKey;
use etalon_model::tokenizer::{ExecutionBudget, TokenizerProfile, TOKENIZER_SEMANTICS_VERSION};
use etalon_model_io::lease::LeaseLimits;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct CpuEvidenceStorage<'bytes> {
    inner: EvidenceStorage<'bytes>,
    reservation: usize,
}
impl<'bytes> CpuEvidenceStorage<'bytes> {
    pub fn new(bytes: &'bytes mut [u8], limits: EvidenceLimits) -> Result<Self, PolicyError> {
        Ok(Self {
            inner: EvidenceStorage::new(bytes, limits)?,
            reservation: limits.storage_bytes,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CpuRequestReview {
    pub request_digest: [u8; 32],
    pub model_digest: [u8; 32],
    pub executor_descriptor: [u8; 32],
    pub tokenizer_semantics_version: u32,
    pub tokenizer_profile: TokenizerProfile,
    pub raw_stop_disposition_version: u32,
    pub prompt_tokens: usize,
    pub first_position: u64,
    pub max_new_tokens: usize,
    pub context: usize,
}

pub struct PreparedGovernedCpu<'data, 'tables, 'input> {
    assets: PreparedAssets<'data, 'tables>,
    request: PreparedRequest<'input>,
    executor: VerifiedCpuExecutor,
    ledger: CpuBudgetReceipt,
    limits: CpuLimits,
}

/// All supplied capacities are reserved before executable mapping or cold writes.
/// Expected executable identity is NOT authority; the returned exact request must
/// be approved by the existing independent policy installer before `admit`.
pub fn prepare_governed_cpu<'data, 'tables, 'input>(
    owner: &'data ImmutableModel<'_>,
    expected: ExpectedCpuArtifact,
    tables: CpuTables<'tables, 'data>,
    prompt_workspace: PromptWorkspace<'_, 'input>,
    input: PromptInput<'input>,
    intent: RequestIntent,
    execution_sizing: &JobBuffers<'_>,
    evidence: EvidenceLimits,
    limits: CpuLimits,
    cancelled: &AtomicBool,
) -> Result<PreparedGovernedCpu<'data, 'tables, 'input>, CpuPreparationError> {
    use CpuPreparationError::Admission;
    if intent.model_generation == 0 || intent.graph_scope == 0 {
        return Err(Admission(CpuAdmissionError::InvalidIdentity));
    }
    if cancelled.load(Ordering::Acquire) {
        return Err(CpuPreparationError::Cancelled);
    }
    let ledger = budget::admit_all(
        owner,
        Some(expected.executable.byte_len),
        &tables,
        &prompt_workspace,
        &input,
        execution_sizing,
        Some(evidence),
        limits,
    )
    .map_err(Admission)?;
    let length = expected.executable.byte_len;
    let total = length
        .checked_mul(2)
        .ok_or(Admission(CpuAdmissionError::Overflow))?;
    let executor = executor::identify_current_executor(
        expected,
        LeaseLimits {
            max_source_bytes: length,
            max_mapped_bytes: length,
            max_total_bytes: total,
        },
        cancelled,
    )
    .map_err(CpuPreparationError::Executor)?;
    let assets = preparation::prepare_assets(owner, tables, input.context, ledger, limits)?;
    if cancelled.load(Ordering::Acquire) {
        return Err(CpuPreparationError::Cancelled);
    }
    let request_budget = request_budget(limits, ledger);
    let raw = tokenizer::prepare_prompt(
        &assets.tokenizer,
        input,
        prompt_workspace.encoding,
        prompt_workspace.tokens,
        ExecutionBudget {
            capacity_bytes: ledger.encoding,
            max_work: limits.tokenizer_encode_work,
        },
        request_budget,
    )
    .map_err(CpuPreparationError::Tokenizer)?;
    let request = request::bind_encoded(
        raw,
        assets.model.model_digest,
        assets.tokenizer.digest,
        executor.digest(),
        assets.tokenizer.plan.vocab_count(),
        request_budget,
    )
    .map_err(Admission)?;
    let request = request::bind_intent(request, intent).map_err(Admission)?;
    if cancelled.load(Ordering::Acquire) {
        return Err(CpuPreparationError::Cancelled);
    }
    Ok(PreparedGovernedCpu {
        assets,
        request,
        executor,
        ledger,
        limits,
    })
}

pub(super) fn request_budget(limits: CpuLimits, ledger: CpuBudgetReceipt) -> RequestBudget {
    RequestBudget {
        max_text_bytes: limits.max_text_bytes,
        max_prompt_tokens: limits.max_prompt_tokens,
        max_stop_ids: limits.max_stop_ids,
        capacity_bytes: ledger.prompt,
    }
}

impl<'data, 'tables, 'input> PreparedGovernedCpu<'data, 'tables, 'input> {
    pub fn review(&self) -> CpuRequestReview {
        CpuRequestReview {
            request_digest: self.request.digest,
            model_digest: self.assets.model.model_digest,
            executor_descriptor: self.executor.digest(),
            tokenizer_semantics_version: TOKENIZER_SEMANTICS_VERSION,
            tokenizer_profile: self.assets.tokenizer.plan.profile(),
            raw_stop_disposition_version: request::RAW_STOP_DISPOSITION_VERSION,
            prompt_tokens: self.request.tokens.len(),
            first_position: (self.request.tokens.len() - 1) as u64,
            max_new_tokens: self.request.max_new_tokens,
            context: self.request.context,
        }
    }
    pub fn required_request_scope(&self) -> DigestScope {
        DigestScope::Exact(self.request.digest)
    }
    pub fn budget(&self) -> CpuBudgetReceipt {
        self.ledger
    }

    /// Installer signing capability remains independent of actor association.
    /// The host persists real admission using its existing counters/anchor. A
    /// constructor/prefill failure drops that admission and publishes no token.
    pub fn admit<'job, 'slot, 'bytes, 'buf>(
        &'job self,
        host: &mut NativePolicyHost,
        installer: &SigningKey,
        owner: Arc<NativeAuthority>,
        association: AuthenticatedInferenceAssociation,
        storage: &'slot mut CpuEvidenceStorage<'bytes>,
        buffers: JobBuffers<'buf>,
    ) -> Result<GovernedCpuSession<'job, 'data, 'tables, 'slot, 'bytes, 'buf>, CpuStepError> {
        self.ledger
            .check_job(&buffers, storage.reservation)
            .map_err(CpuStepError::Admission)?;
        let intent = self
            .request
            .intent
            .ok_or(CpuStepError::Admission(CpuAdmissionError::InvalidRequest))?;
        let first = self
            .request
            .tokens
            .len()
            .checked_sub(1)
            .ok_or(CpuStepError::Admission(CpuAdmissionError::InvalidRequest))?;
        let request = InferenceAdmission {
            request_digest: self.request.digest,
            model_digest: self.assets.model.model_digest,
            model_generation: intent.model_generation,
            device_id: self.executor.device_id(),
            device_generation: self.executor.generation(),
            graph_scope: intent.graph_scope,
            first_position: first as u64,
            instrumentation: intent.instrumentation,
        };
        let authority = host
            .admit_evidence(
                installer,
                request,
                POLICY_COLD_BUDGET_BYTES,
                owner,
                association,
                &mut storage.inner,
            )
            .map_err(CpuStepError::Policy)?;
        self.start_admitted(authority, buffers)
    }
    pub(super) fn start_admitted<'job, 'slot, 'bytes, 'buf>(
        &'job self,
        authority: crate::inference::inference_agent::governed::evidence::GovernedEvidenceSession<
            'slot,
            'bytes,
        >,
        buffers: JobBuffers<'buf>,
    ) -> Result<GovernedCpuSession<'job, 'data, 'tables, 'slot, 'bytes, 'buf>, CpuStepError> {
        self.ledger
            .check_job(&buffers, self.ledger.evidence)
            .map_err(CpuStepError::Admission)?;
        let decode = ExecutionBudget {
            capacity_bytes: buffers.piece.len(),
            max_work: self.limits.tokenizer_decode_work,
        };
        GovernedCpuSession::begin(
            &self.assets.model,
            &self.assets.tokenizer,
            self.request,
            &self.executor,
            authority,
            buffers,
            self.ledger.execution,
            decode,
        )
    }
}
