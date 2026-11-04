//! Checked reservation for all supplied model/metadata/prompt/execution regions.
//! Counts source bytes plus mapped address ranges and retained snapshot disk, not
//! RSS, allocator headers, filesystem metadata, OS caches or whole thread stacks.
use super::{
    buffers::{self, JobBuffers},
    model::ImmutableModel,
    CpuAdmissionError,
};
use crate::{
    inference::inference_agent::governed::{
        evidence::{EvidenceLimits, EvidenceStorage},
        POLICY_COLD_BUDGET_BYTES,
    },
    modalities::logic::WebizenVM,
    NQuin,
};
use etalon_inference::cpu_gguf::PackedTensor;
use etalon_model::{
    gguf::ParseLimits,
    tokenizer::{
        ExecutionWorkspace, MergeEntry, PretokenSpan, SpecialEntry, SymbolSpan, TokenizerTables,
        VocabEntry,
    },
};
use qualia_inference_kernel::dense_cpu::DenseLayer;
use std::mem::size_of;

pub struct CpuTables<'work, 'data> {
    pub layers: &'work mut [DenseLayer<PackedTensor<'data>>],
    pub tokenizer: TokenizerTables<'work, 'data>,
}
pub struct PromptWorkspace<'scratch, 'tokens> {
    pub encoding: ExecutionWorkspace<'scratch>,
    pub tokens: &'tokens mut [u32],
}
#[derive(Clone, Copy)]
pub struct CpuLimits {
    pub aggregate_bytes: usize,
    pub max_snapshot_disk_bytes: u64,
    pub parse: ParseLimits,
    pub binding_visits: u64,
    pub tokenizer_prepare_work: u64,
    pub tokenizer_encode_work: u64,
    pub tokenizer_decode_work: u64,
    pub max_text_bytes: usize,
    pub max_prompt_tokens: usize,
    pub max_stop_ids: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct CpuBudgetReceipt {
    total: usize,
    disk: u64,
    pub(super) layers: usize,
    pub(super) tokenizer: usize,
    pub(super) encoding: usize,
    pub(super) prompt: usize,
    pub(super) execution: usize,
    pub(super) workspace: usize,
    pub(super) evidence: usize,
    shape: [usize; 17],
}
impl CpuBudgetReceipt {
    pub fn admitted_bytes(&self) -> usize {
        self.total
    }
    pub fn snapshot_disk_bytes(&self) -> u64 {
        self.disk
    }
    pub(super) fn check_job(
        &self,
        buffers: &JobBuffers<'_>,
        evidence: usize,
    ) -> Result<(), CpuAdmissionError> {
        if shape(buffers) != self.shape || evidence > self.evidence {
            return Err(CpuAdmissionError::Budget);
        }
        buffers::admit_buffers(buffers, self.execution)?;
        Ok(())
    }
}

fn bytes<T>(n: usize) -> Result<usize, CpuAdmissionError> {
    n.checked_mul(size_of::<T>())
        .ok_or(CpuAdmissionError::Overflow)
}
fn sum(values: &[usize]) -> Result<usize, CpuAdmissionError> {
    values.iter().try_fold(0usize, |n, v| {
        n.checked_add(*v).ok_or(CpuAdmissionError::Overflow)
    })
}
fn shape(b: &JobBuffers<'_>) -> [usize; 17] {
    let mut lengths = [0; 17];
    for (dst, src) in lengths[..13].iter_mut().zip(buffers::slices(&b.workspace)) {
        *dst = src.len();
    }
    lengths[13] = b.undo.len();
    lengths[14] = b.piece.len();
    lengths[15] = b.output.len();
    lengths[16] = b.lineage.len();
    lengths
}

/// Runs before current_exe lookup, metadata table mutation or prompt encoding.
/// The model owner was already acquired under its own lease/snapshot limits;
/// here its retained source/map/disk reservation is included in the job ledger.
pub(super) fn admit_all(
    model: &ImmutableModel<'_>,
    executable_length: Option<u64>,
    tables: &CpuTables<'_, '_>,
    prompt: &PromptWorkspace<'_, '_>,
    input: &super::tokenizer::PromptInput<'_>,
    buffers: &JobBuffers<'_>,
    evidence: Option<EvidenceLimits>,
    limits: CpuLimits,
) -> Result<CpuBudgetReceipt, CpuAdmissionError> {
    input.capabilities.validate()?;
    if input.text.len() > limits.max_text_bytes || input.stops.len() > limits.max_stop_ids {
        return Err(CpuAdmissionError::Budget);
    }
    let model_length = model.identity().byte_len;
    let disk = if matches!(model, ImmutableModel::Snapshot(_)) {
        model_length
    } else {
        0
    };
    if disk > limits.max_snapshot_disk_bytes {
        return Err(CpuAdmissionError::Budget);
    }
    let backing = model_length
        .checked_mul(2)
        .and_then(|n| n.checked_add(executable_length.unwrap_or(0).checked_mul(2)?))
        .and_then(|n| n.checked_add(disk))
        .ok_or(CpuAdmissionError::Overflow)?;
    let backing = usize::try_from(backing).map_err(|_| CpuAdmissionError::Overflow)?;
    let layers = bytes::<DenseLayer<PackedTensor<'_>>>(tables.layers.len())?;
    let tokenizer = sum(&[
        bytes::<VocabEntry<'_>>(tables.tokenizer.vocab.len())?,
        bytes::<MergeEntry<'_>>(tables.tokenizer.merges.len())?,
        bytes::<SpecialEntry<'_>>(tables.tokenizer.specials.len())?,
    ])?;
    let encoding = sum(&[
        bytes::<PretokenSpan>(prompt.encoding.spans.len())?,
        prompt.encoding.encoded.len(),
        bytes::<SymbolSpan>(prompt.encoding.symbols.len())?,
        bytes::<u32>(prompt.tokens.len())?,
    ])?;
    let prompt_bytes = sum(&[
        input.text.len(),
        bytes::<u32>(input.stops.len())?,
        bytes::<u32>(prompt.tokens.len())?,
    ])?;
    let lengths = shape(buffers);
    let workspace = bytes::<f32>(sum(&lengths[..13])?)?;
    let transaction = sum(&[workspace, bytes::<f32>(buffers.undo.len())?])?;
    let execution = sum(&[
        transaction,
        buffers.piece.len(),
        bytes::<u32>(buffers.output.len())?,
        bytes::<Option<etalon_inference_contract::CandidateKey>>(buffers.lineage.len())?,
        bytes::<f32>(etalon_inference::cpu_gguf::MAX_INPUT_WIDTH)?,
    ])?;
    buffers::admit_buffers(buffers, execution)?;
    let evidence_bytes = if let Some(e) = evidence {
        if e.storage_bytes < size_of::<EvidenceStorage<'_>>() || e.max_payload_bytes == 0 {
            return Err(CpuAdmissionError::Capacity);
        }
        e.storage_bytes
    } else {
        0
    };
    let policy = if evidence.is_some() {
        // Existing store's bounded cold reservation plus actual VM/graph scratch.
        sum(&[
            POLICY_COLD_BUDGET_BYTES,
            bytes::<WebizenVM>(2)?,
            bytes::<NQuin>(4 * 256)?,
        ])?
    } else {
        0
    };
    // Encoding includes the output IDs, so do not count them a second time.
    let total = sum(&[
        backing,
        layers,
        tokenizer,
        encoding,
        input.text.len(),
        bytes::<u32>(input.stops.len())?,
        execution,
        evidence_bytes,
        policy,
    ])?;
    if total > limits.aggregate_bytes {
        return Err(CpuAdmissionError::Budget);
    }
    Ok(CpuBudgetReceipt {
        total,
        disk,
        layers,
        tokenizer,
        encoding,
        prompt: prompt_bytes,
        execution,
        workspace,
        evidence: evidence_bytes,
        shape: lengths,
    })
}
