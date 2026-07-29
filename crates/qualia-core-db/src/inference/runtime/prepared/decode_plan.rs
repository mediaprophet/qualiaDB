use super::PreparedPlanState;
use crate::inference::runtime::receipt::ExecutionCounters;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparedBackend {
    Wgpu,
    Cuda,
    Metal,
    Cpu,
}

#[derive(Debug)]
pub enum DecodeStepError {
    NotReady(PreparedPlanState),
    InvalidInput,
    BackendFailure,
}

#[derive(Debug)]
pub struct DecodeStepInput<'a> {
    pub token_position: u32,
    pub token_id: u32,
    pub hidden: &'a mut [f32],
    pub kv_page_table: &'a [u32],
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DecodeStepOutput {
    pub token_id: u32,
    pub score: f32,
}

/// Tier-1 execution contract for an already prepared model/backend pair.
pub trait PreparedDecodePlan {
    fn backend(&self) -> PreparedBackend;
    fn state(&self) -> PreparedPlanState;
    fn counters(&self) -> ExecutionCounters;

    /// Execute one autoregressive step.
    ///
    /// Implementations must not allocate, compile, discover tensors, upload immutable data, or
    /// select another backend. A failure is returned to the owner, which records any explicit
    /// fallback outside this call.
    fn run_decode_step(
        &mut self,
        input: DecodeStepInput<'_>,
        output: &mut DecodeStepOutput,
    ) -> Result<(), DecodeStepError>;
}
