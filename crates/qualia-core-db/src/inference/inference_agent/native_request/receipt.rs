//! Sealed handoff from cold preparation. No public constructor accepts asserted
//! model/device identity. The complete native input-closure factory is a separate
//! migration gate; a path hash or Qwen checkpoint digest cannot construct this.
use super::super::governed::{InferenceAdmission, PolicyError};
use crate::gguf_bridge::QTensorEngine;

pub struct NativePreparedReceipt {
    pub(super) admission: InferenceAdmission,
    pub(super) actor: u64,
    pub(super) provenance: [u8; 32],
    pub(super) generation: u64,
    pub(super) execution: Box<dyn NativeExecutionSnapshot>,
}

/// Only trusted cold preparation inside this library may implement this closed
/// input-closure comparison. Validation must inspect all actually used inputs,
/// numeric settings and provider generation before returning the bound receipt.
pub(in crate::inference::inference_agent) trait NativeExecutionSnapshot:
    Send
{
    /// Revalidate retained provider/device and immutable-source owners at every
    /// acceptance boundary. No default can substitute startup validation for
    /// live provider continuity after device loss, replacement or cancellation.
    fn live(&self) -> Result<(), PolicyError>;

    fn validate(
        &self,
        engine: &QTensorEngine,
        prompt: &str,
        graph: u64,
        encoded: &[u32],
    ) -> Result<(), PolicyError>;
}
impl NativePreparedReceipt {
    pub(super) fn validate(
        &self,
        engine: &QTensorEngine,
        prompt: &str,
        graph: u64,
        encoded: &[u32],
    ) -> Result<(), PolicyError> {
        if self.generation != super::super::sticky_infer::current_residency_generation()
            || self.admission.graph_scope != graph
            || self.admission.first_position != encoded.len().saturating_sub(1) as u64
            || self.actor == 0
            || self.provenance == [0; 32]
        {
            return Err(PolicyError::InvalidCandidate);
        }
        self.execution.live()?;
        self.execution.validate(engine, prompt, graph, encoded)
    }
}
