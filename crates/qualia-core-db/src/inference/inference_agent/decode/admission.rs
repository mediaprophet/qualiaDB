//! Cold request admission and bounded retained-evidence setup.
use super::super::{
    governed::{
        evidence::{EvidenceLimits, EvidenceStorage},
        PolicyError,
    },
    native_request::{NativeDecodeRequest, PublicationGate},
};
use super::*;
use crate::gguf_bridge::QTensorEngine;

pub(super) const EVIDENCE_BYTES: usize = 65536;
pub(super) fn activate_compute(
    epoch: &crate::compute_universe::ComputeRequestEpoch,
    prompt: &str,
) -> Result<(), PolicyError> {
    if !epoch.activate(|| {
        crate::compute_universe::start_tensor_search_producer();
        crate::compute_universe::publish_query_tensor(crate::tensor::Tensor10D::default(), 0);
        crate::qualia_hybrid::prepare_hybrid_decode(prompt);
    }) {
        return Err(PolicyError::Revoked);
    }
    Ok(())
}
pub(super) fn thinking_budget(control: Option<&DecodeControl>) -> Option<u32> {
    control
        .map(DecodeControl::thinking_token_budget)
        .filter(|&budget| budget > 0)
}
pub(super) fn unsupported(lp: &mut rtrb::Producer<LlmMsg>, trace: StageTrace) -> WorkerResult {
    let _ = lp.push(LlmMsg::Eos);
    (String::from("[unsupported-model]"), 0, None, false, trace)
}
pub(super) fn generation_budget(sieve: bool, control: Option<&DecodeControl>) -> usize {
    let configured = if sieve {
        3
    } else {
        let requested = crate::llm_bench::decode_budget_override();
        if requested > 0 {
            requested as usize
        } else {
            DECODE_TOKEN_BUDGET as usize
        }
    };
    configured.min(
        control
            .map(DecodeControl::token_budget)
            .unwrap_or(usize::MAX),
    )
}
pub(super) fn thermal_wal() -> Option<crate::inference::thermal_wal::ThermalWal> {
    let path = std::env::var("QUALIA_DATA_DIR")
        .map(|root| std::path::PathBuf::from(root).join("thermal_eviction.wal"))
        .unwrap_or_else(|_| std::env::temp_dir().join("thermal_eviction.wal"));
    crate::inference::thermal_wal::ThermalWal::open(&path, 1024).ok()
}
pub(super) fn storage(
    bytes: &mut [u8],
    max_events: u64,
) -> Result<EvidenceStorage<'_>, PolicyError> {
    EvidenceStorage::new(
        bytes,
        EvidenceLimits {
            storage_bytes: EVIDENCE_BYTES + std::mem::size_of::<EvidenceStorage<'_>>(),
            max_payload_bytes: EVIDENCE_BYTES,
            max_artifact_events: max_events,
        },
    )
}
pub(super) fn start<'slot, 'bytes>(
    execution: NativeDecodeRequest,
    epoch: crate::compute_universe::ComputeRequestEpoch,
    engine: &QTensorEngine,
    prompt: &str,
    graph: u64,
    encoded: &[u32],
    storage: &'slot mut EvidenceStorage<'bytes>,
) -> Result<
    (
        PublicationGate<'slot, 'bytes>,
        crate::compute_universe::ComputeRequestEpoch,
    ),
    PolicyError,
> {
    let gate = PublicationGate::admit(execution, engine, prompt, graph, encoded, storage)?;
    if gate.is_governed() {
        crate::llm_bench::domino_reset()
    }
    Ok((gate, epoch))
}
pub(super) fn refused(
    lp: &mut rtrb::Producer<LlmMsg>,
    trace: StageTrace,
    error: PolicyError,
) -> WorkerResult {
    log::error!("LLM_DECODE|native-admission-refused|{error}");
    let _ = lp.push(LlmMsg::Eos);
    (String::new(), 0, None, false, trace)
}
