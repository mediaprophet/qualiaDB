//! Model memory footprint descriptors and static device overhead.
//!
//! Accounts for model weights, scratch activation buffers, device runtime context,
//! and static graph capture headroom before allocating dynamic KV pools.

use super::BudgetError;

/// Static memory profile of a loaded model on a device backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelMemoryProfile {
    /// Bytes occupied by static model weights and quantization tables.
    pub weight_bytes: u64,
    /// Working memory for intermediate activation tensors during decode/prefill.
    pub scratch_activation_bytes: u64,
    /// Fixed driver / runtime context overhead (e.g. CUDA context, DirectML device).
    pub device_context_overhead_bytes: u64,
    /// Headroom reserved for static execution graphs (CUDA graphs / DirectML graphs).
    pub graph_capture_headroom_bytes: u64,
}

impl ModelMemoryProfile {
    pub fn new(
        weight_bytes: u64,
        scratch_activation_bytes: u64,
        device_context_overhead_bytes: u64,
        graph_capture_headroom_bytes: u64,
    ) -> Result<Self, BudgetError> {
        // Enforce checked addition across all static requirements.
        let _ = weight_bytes
            .checked_add(scratch_activation_bytes)
            .and_then(|t| t.checked_add(device_context_overhead_bytes))
            .and_then(|t| t.checked_add(graph_capture_headroom_bytes))
            .ok_or(BudgetError::IntegerOverflow)?;

        Ok(Self {
            weight_bytes,
            scratch_activation_bytes,
            device_context_overhead_bytes,
            graph_capture_headroom_bytes,
        })
    }

    /// Total bytes required by the static model runtime before dynamic KV pools.
    pub fn static_device_bytes(&self) -> u64 {
        self.weight_bytes
            .saturating_add(self.scratch_activation_bytes)
            .saturating_add(self.device_context_overhead_bytes)
            .saturating_add(self.graph_capture_headroom_bytes)
    }
}
