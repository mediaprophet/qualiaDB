//! Required semantic-input owner for the native numerical request. Implemented
//! only by trusted scoped hybrid/context preparation, never caller assertions.
use super::super::governed::PolicyError;
use crate::tensor::resident_substrate::{global_resident_substrate, ResidentSnapshot};
use sha2::{Digest, Sha256};
use std::sync::Arc;

struct OwnedContext {
    source: Arc<ResidentSnapshot>,
    facts: Arc<[crate::quant_graph_grounding::GroundingFact]>,
    epoch: u64,
    request: [u8; 32],
    digest: [u8; 32],
    retained_bytes: usize,
}
fn request_digest(prompt: &str, graph: u64, encoded: &[u32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"qualia:context-request:v1\0");
    h.update(graph.to_le_bytes());
    h.update((prompt.len() as u64).to_le_bytes());
    h.update(prompt.as_bytes());
    h.update((encoded.len() as u64).to_le_bytes());
    for token in encoded {
        h.update(token.to_le_bytes());
    }
    h.finalize().into()
}
pub(in crate::inference::inference_agent) fn capture(
    epoch: &crate::compute_universe::ComputeRequestEpoch,
    facts: &crate::quant_graph_grounding::scoped::GroundingScope,
    prompt: &str,
    graph: u64,
    encoded: &[u32],
    budget: usize,
) -> Result<Box<dyn NativeContextSnapshot>, PolicyError> {
    let gpu_bytes = {
        #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
        {
            if crate::gpu_context::try_shared_gpu().is_some() {
                crate::tensor::volume_gpu::owned::RETAINED_BYTES
            } else {
                0
            }
        }
        #[cfg(not(all(not(target_arch = "wasm32"), feature = "gpu-runtime")))]
        {
            0
        }
    };
    let gpu_scratch = if gpu_bytes > 0 {
        crate::tensor::resident_substrate::MAX_RESIDENT_NODES * 40
    } else {
        0
    };
    let reserved = facts
        .retained_bytes()
        .checked_add(std::mem::size_of::<OwnedContext>())
        .and_then(|n| n.checked_add(gpu_bytes))
        .and_then(|n| n.checked_add(gpu_scratch))
        .ok_or(PolicyError::Budget)?;
    let source_budget = budget.checked_sub(reserved).ok_or(PolicyError::Budget)?;
    let source = Arc::new(
        global_resident_substrate()
            .snapshot_owned(source_budget)
            .map_err(|_| PolicyError::Budget)?,
    );
    let retained_bytes = source
        .retained_bytes()
        .checked_add(reserved)
        .ok_or(PolicyError::Budget)?;
    if retained_bytes > budget {
        return Err(PolicyError::Budget);
    }
    if !epoch.bind_source(
        Arc::clone(&source),
        gpu_bytes
            .checked_add(gpu_scratch)
            .ok_or(PolicyError::Budget)?,
    ) {
        return Err(PolicyError::Revoked);
    }
    let request = request_digest(prompt, graph, encoded);
    let mut h = Sha256::new();
    h.update(b"qualia:owned-semantic-inputs:v1\0");
    h.update(source.digest());
    h.update(facts.digest());
    h.update(request);
    Ok(Box::new(OwnedContext {
        source,
        facts: facts.owner(),
        epoch: epoch.identity(),
        request,
        digest: h.finalize().into(),
        retained_bytes,
    }))
}
impl NativeContextSnapshot for OwnedContext {
    fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
    fn digest(&self) -> [u8; 32] {
        self.digest
    }
    fn live(&self) -> Result<(), PolicyError> {
        if crate::compute_universe::context_live(self.epoch, self.source.digest())
            && crate::quant_graph_grounding::scoped::owns(&self.facts)
        {
            Ok(())
        } else {
            Err(PolicyError::Revoked)
        }
    }
    fn validate_request(
        &self,
        prompt: &str,
        graph: u64,
        encoded: &[u32],
    ) -> Result<(), PolicyError> {
        self.live()?;
        if self.request == request_digest(prompt, graph, encoded) {
            Ok(())
        } else {
            Err(PolicyError::InvalidCandidate)
        }
    }
}

pub(in crate::inference::inference_agent) trait NativeContextSnapshot:
    Send
{
    fn retained_bytes(&self) -> usize;
    /// Binds initial owned graph/hybrid inputs and the admitted producer schema.
    fn digest(&self) -> [u8; 32];
    /// Rechecks scope/producer ownership and any per-step admitted input records.
    fn live(&self) -> Result<(), PolicyError>;
    fn validate_request(
        &self,
        prompt: &str,
        graph: u64,
        encoded: &[u32],
    ) -> Result<(), PolicyError>;
}
