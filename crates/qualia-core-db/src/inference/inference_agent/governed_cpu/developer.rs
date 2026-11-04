//! Explicit policy-free entrypoint using the same real preparation and numerical
//! transaction. No missing authority branch selects it and no policy ID is made up.
use super::{
    budget::{self, CpuBudgetReceipt, CpuLimits, CpuTables, PromptWorkspace},
    buffers::{self, JobBuffers},
    facade::request_budget,
    initialize,
    model::ImmutableModel,
    numerical,
    preparation::{self, CpuPreparationError, PreparedAssets},
    request::{self, PreparedRequest},
    session::CpuStepError,
    state::{HostState, StagedDeveloperHost},
    tokenizer::{self, PromptInput},
    CpuAdmissionError,
};
use etalon_model::tokenizer::{ExecutionBudget, TokenizerProfile, TOKENIZER_SEMANTICS_VERSION};
use qualia_inference_kernel::dense_cpu::DenseStep;
use std::sync::atomic::{AtomicBool, Ordering};

/// Must be chosen explicitly by the standalone developer caller. No Default.
pub enum StandaloneDeveloper {
    Explicit,
}
pub struct PreparedDeveloperCpu<'data, 'tables, 'input> {
    assets: PreparedAssets<'data, 'tables>,
    request: PreparedRequest<'input>,
    ledger: CpuBudgetReceipt,
    limits: CpuLimits,
}
#[derive(Clone, Copy, Debug)]
pub struct DeveloperCpuReview {
    pub model_digest: [u8; 32],
    pub request_digest: [u8; 32],
    pub tokenizer_semantics_version: u32,
    pub tokenizer_profile: TokenizerProfile,
    pub prompt_tokens: usize,
    pub max_new_tokens: usize,
}

pub fn prepare_developer_cpu<'data, 'tables, 'input>(
    _mode: StandaloneDeveloper,
    owner: &'data ImmutableModel<'_>,
    tables: CpuTables<'tables, 'data>,
    prompt_workspace: PromptWorkspace<'_, 'input>,
    input: PromptInput<'input>,
    execution_sizing: &JobBuffers<'_>,
    limits: CpuLimits,
    cancelled: &AtomicBool,
) -> Result<PreparedDeveloperCpu<'data, 'tables, 'input>, CpuPreparationError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(CpuPreparationError::Cancelled);
    }
    let ledger = budget::admit_all(
        owner,
        None,
        &tables,
        &prompt_workspace,
        &input,
        execution_sizing,
        None,
        limits,
    )
    .map_err(CpuPreparationError::Admission)?;
    let assets = preparation::prepare_assets(owner, tables, input.context, ledger, limits)?;
    if cancelled.load(Ordering::Acquire) {
        return Err(CpuPreparationError::Cancelled);
    }
    let raw = tokenizer::prepare_prompt(
        &assets.tokenizer,
        input,
        prompt_workspace.encoding,
        prompt_workspace.tokens,
        ExecutionBudget {
            capacity_bytes: ledger.encoding,
            max_work: limits.tokenizer_encode_work,
        },
        request_budget(limits, ledger),
    )
    .map_err(CpuPreparationError::Tokenizer)?;
    let request = request::bind_developer(
        raw,
        assets.model.model_digest,
        assets.tokenizer.digest,
        assets.tokenizer.plan.vocab_count(),
        request_budget(limits, ledger),
    )
    .map_err(CpuPreparationError::Admission)?;
    if cancelled.load(Ordering::Acquire) {
        return Err(CpuPreparationError::Cancelled);
    }
    Ok(PreparedDeveloperCpu {
        assets,
        request,
        ledger,
        limits,
    })
}
impl<'data, 'tables, 'input> PreparedDeveloperCpu<'data, 'tables, 'input> {
    pub fn review(&self) -> DeveloperCpuReview {
        DeveloperCpuReview {
            model_digest: self.assets.model.model_digest,
            request_digest: self.request.digest,
            tokenizer_semantics_version: TOKENIZER_SEMANTICS_VERSION,
            tokenizer_profile: self.assets.tokenizer.plan.profile(),
            prompt_tokens: self.request.tokens.len(),
            max_new_tokens: self.request.max_new_tokens,
        }
    }
    pub fn budget(&self) -> CpuBudgetReceipt {
        self.ledger
    }
    pub fn begin<'job, 'buf>(
        &'job self,
        mut buffers: JobBuffers<'buf>,
    ) -> Result<DeveloperCpuSession<'job, 'data, 'tables, 'buf>, CpuStepError> {
        self.ledger
            .check_job(&buffers, 0)
            .map_err(CpuStepError::Admission)?;
        let transaction_budget = buffers::admit_buffers(&buffers, self.ledger.execution)
            .map_err(CpuStepError::Admission)?;
        let state = initialize::prefill(
            &self.assets.model,
            &self.assets.tokenizer,
            &self.request,
            &mut buffers,
            transaction_budget,
        )?;
        let decode_budget = ExecutionBudget {
            capacity_bytes: buffers.piece.len(),
            max_work: self.limits.tokenizer_decode_work,
        };
        Ok(DeveloperCpuSession {
            assets: &self.assets,
            request: self.request,
            buffers,
            state,
            transaction_budget,
            decode_budget,
            terminal: false,
        })
    }
}

pub struct DeveloperCpuSession<'job, 'data, 'tables, 'buf> {
    assets: &'job PreparedAssets<'data, 'tables>,
    request: PreparedRequest<'job>,
    buffers: JobBuffers<'buf>,
    state: HostState,
    transaction_budget: usize,
    decode_budget: ExecutionBudget,
    terminal: bool,
}
impl DeveloperCpuSession<'_, '_, '_, '_> {
    pub fn is_finished(&self) -> bool {
        self.state.stopped
    }
    pub fn is_terminal(&self) -> bool {
        self.terminal
    }
    pub fn cancel(&mut self) {
        self.terminal = true;
    }
    pub fn numerical_state(&self) -> [&[f32]; 13] {
        buffers::slices(&self.buffers.workspace)
    }
    pub fn generated_tokens(&self) -> usize {
        self.state.generated
    }
    pub fn next(&mut self) -> Result<DeveloperToken<'_>, CpuStepError> {
        use CpuStepError::Admission;
        if self.terminal || self.state.stopped {
            return Err(Admission(CpuAdmissionError::Terminal));
        }
        self.terminal = true;
        let token_slot = self
            .buffers
            .output
            .get_mut(self.state.generated)
            .ok_or(Admission(CpuAdmissionError::Capacity))?;
        let position = self.state.position;
        let pending = numerical::propose_step(
            &self.assets.model,
            &self.state,
            buffers::reborrow(&mut self.buffers.workspace),
            &mut *self.buffers.undo,
            self.transaction_budget,
        )
        .map_err(CpuStepError::Numerical)?;
        let output = pending.output();
        let length = self
            .assets
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
        let staged = StagedDeveloperHost::prepare(
            &mut self.state,
            token_slot,
            output.token_id,
            self.request.stops.contains(&output.token_id),
            self.request.max_new_tokens,
            self.request.context,
        )
        .map_err(Admission)?;
        pending.commit();
        staged.finish();
        Ok(DeveloperToken {
            output,
            position,
            piece,
            terminal: &mut self.terminal,
        })
    }
}

/// Developer numerical output, deliberately not a Sentinel receipt or CandidateKey.
pub struct DeveloperToken<'a> {
    output: DenseStep,
    position: usize,
    piece: &'a [u8],
    terminal: &'a mut bool,
}
impl DeveloperToken<'_> {
    pub fn output(&self) -> DenseStep {
        self.output
    }
    pub fn position(&self) -> usize {
        self.position
    }
    pub fn bytes(&self) -> &[u8] {
        self.piece
    }
    pub fn complete(self, delivered_bytes: usize) -> Result<(), CpuAdmissionError> {
        if delivered_bytes != self.piece.len() {
            return Err(CpuAdmissionError::InvalidRequest);
        }
        *self.terminal = false;
        Ok(())
    }
}
