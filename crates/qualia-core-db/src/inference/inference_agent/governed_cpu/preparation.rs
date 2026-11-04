use super::{
    budget::{CpuBudgetReceipt, CpuLimits, CpuTables},
    executor::ExecutorError,
    model::{self, ImmutableModel, ModelPreparationError, PreparedCpuModel},
    tokenizer::{self, PreparedTokenizer, TokenizerAdmissionError},
    CpuAdmissionError,
};
use etalon_inference::cpu_gguf::BindingLimits;
use etalon_model::tokenizer::PrepareBudget;

#[derive(Debug)]
pub enum CpuPreparationError {
    Admission(CpuAdmissionError),
    Executor(ExecutorError),
    Model(ModelPreparationError),
    Tokenizer(TokenizerAdmissionError),
    Cancelled,
}
pub(super) struct PreparedAssets<'data, 'tables> {
    pub model: PreparedCpuModel<'data, 'tables>,
    pub tokenizer: PreparedTokenizer<'data, 'tables>,
}
pub(super) fn prepare_assets<'data, 'tables>(
    owner: &'data ImmutableModel<'_>,
    tables: CpuTables<'tables, 'data>,
    context: usize,
    ledger: CpuBudgetReceipt,
    limits: CpuLimits,
) -> Result<PreparedAssets<'data, 'tables>, CpuPreparationError> {
    let model = model::prepare_model(
        owner,
        tables.layers,
        context,
        limits.parse,
        BindingLimits {
            byte_budget: ledger.layers,
            max_tensor_visits: limits.binding_visits,
        },
        ledger.workspace,
    )
    .map_err(CpuPreparationError::Model)?;
    let tokenizer = tokenizer::prepare_tokenizer(
        owner,
        tables.tokenizer,
        limits.parse,
        PrepareBudget {
            capacity_bytes: ledger.tokenizer,
            max_work: limits.tokenizer_prepare_work,
        },
    )
    .map_err(CpuPreparationError::Tokenizer)?;
    if model.plan.config().vocabulary != tokenizer.plan.vocab_count() {
        return Err(CpuPreparationError::Admission(
            CpuAdmissionError::InvalidIdentity,
        ));
    }
    Ok(PreparedAssets { model, tokenizer })
}
