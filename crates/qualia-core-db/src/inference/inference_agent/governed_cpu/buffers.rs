use super::CpuAdmissionError;
use etalon_inference_contract::CandidateKey;
use qualia_inference_kernel::dense_cpu::DenseWorkspace;

pub struct JobBuffers<'a> {
    pub workspace: DenseWorkspace<'a>,
    pub undo: &'a mut [f32],
    pub piece: &'a mut [u8],
    pub output: &'a mut [u32],
    pub lineage: &'a mut [Option<CandidateKey>],
}

/// Execution-region accounting only. Immutable source/maps, metadata tables,
/// prompt preparation and policy cold storage have separate explicit admissions.
/// The top-level constructor must combine these with their retained receipts.
pub(super) fn admit_buffers(
    buffers: &JobBuffers<'_>,
    budget: usize,
) -> Result<usize, CpuAdmissionError> {
    let mut floats = 0usize;
    for s in slices(&buffers.workspace) {
        floats = floats
            .checked_add(s.len())
            .ok_or(CpuAdmissionError::Overflow)?;
    }
    if buffers.undo.len() < floats {
        return Err(CpuAdmissionError::Capacity);
    }
    let transaction = floats
        .checked_add(buffers.undo.len())
        .and_then(|n| n.checked_mul(4))
        .ok_or(CpuAdmissionError::Overflow)?;
    let total = transaction
        .checked_add(buffers.piece.len())
        .and_then(|n| n.checked_add(buffers.output.len().checked_mul(4)?))
        .and_then(|n| {
            n.checked_add(
                buffers
                    .lineage
                    .len()
                    .checked_mul(core::mem::size_of::<Option<CandidateKey>>())?,
            )
        })
        // Fixed concrete provider row scratch; not an RSS measurement.
        .and_then(|n| n.checked_add(etalon_inference::cpu_gguf::MAX_INPUT_WIDTH.checked_mul(4)?))
        .ok_or(CpuAdmissionError::Overflow)?;
    if total > budget {
        return Err(CpuAdmissionError::Budget);
    }
    Ok(transaction)
}

pub(super) fn slices<'a>(w: &'a DenseWorkspace<'_>) -> [&'a [f32]; 13] {
    [
        w.kv,
        w.hidden,
        w.normed,
        w.norm_weight,
        w.q,
        w.k,
        w.v,
        w.attention,
        w.projection,
        w.gate,
        w.up,
        w.scores,
        w.logits,
    ]
}
pub(super) fn reborrow<'a>(w: &'a mut DenseWorkspace<'_>) -> DenseWorkspace<'a> {
    DenseWorkspace {
        kv: w.kv,
        hidden: w.hidden,
        normed: w.normed,
        norm_weight: w.norm_weight,
        q: w.q,
        k: w.k,
        v: w.v,
        attention: w.attention,
        projection: w.projection,
        gate: w.gate,
        up: w.up,
        scores: w.scores,
        logits: w.logits,
    }
}
