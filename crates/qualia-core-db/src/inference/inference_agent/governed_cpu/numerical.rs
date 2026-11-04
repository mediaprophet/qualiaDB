//! Actual numerical stage, independent of tokenizer lifecycle. The concrete
//! prepared-tokenizer runner binds its output to retained evidence before the
//! live host acceptance gate may commit this transaction.
use super::{model::PreparedCpuModel, state::HostState, CpuAdmissionError};
use etalon_inference::{
    cpu_gguf::{GgmlCpuKernels, GgufCpuError},
    runtime::dense_transaction::{propose_dense, PendingDense, TransactionError},
};
use qualia_inference_kernel::dense_cpu::{DenseStep, DenseWorkspace};

/// Construction is confined to actual execution below. The model and input
/// position cannot be asserted later by a caller holding an arbitrary step.
pub(super) struct NumericalProposal<'w, 'u> {
    pending: PendingDense<'w, 'u>,
    model: [u8; 32],
    position: usize,
}
impl NumericalProposal<'_, '_> {
    pub(super) fn output(&self) -> DenseStep {
        self.pending.proposal()
    }
    pub(super) fn model(&self) -> [u8; 32] {
        self.model
    }
    pub(super) fn position(&self) -> usize {
        self.position
    }
    pub(super) fn commit(self) {
        self.pending.commit();
    }
}

#[derive(Debug)]
pub enum StepError {
    Admission(CpuAdmissionError),
    Execution(TransactionError<GgufCpuError>),
}

pub(super) fn propose_step<'w, 'u>(
    model: &PreparedCpuModel<'_, '_>,
    state: &HostState,
    workspace: DenseWorkspace<'w>,
    undo: &'u mut [f32],
    full_transaction_budget: usize,
) -> Result<NumericalProposal<'w, 'u>, StepError> {
    if state.terminal || state.stopped {
        return Err(StepError::Admission(CpuAdmissionError::Terminal));
    }
    // Concrete, stateless actual GGML provider: no arbitrary callback may mutate
    // hidden state outside PendingDense's complete numerical snapshot.
    let pending = propose_dense(
        &model.plan,
        &mut GgmlCpuKernels,
        workspace,
        undo,
        state.current_token,
        state.position,
        full_transaction_budget,
    )
    .map_err(StepError::Execution)?;
    Ok(NumericalProposal {
        pending,
        model: model.model_digest,
        position: state.position,
    })
}
