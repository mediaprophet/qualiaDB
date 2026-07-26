//! Caller-buffered ragged decode batches.
//!
//! The scheduler lowers active sequences and their page tables into flat ABI records. A backend
//! receives one call per scheduling round and must return one output for every item in slot order.

#[repr(C)]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable,
)]
pub struct RaggedBatchItem {
    pub request_id: u64,
    pub slot: u32,
    pub token_id: u32,
    pub position: u32,
    pub block_table_offset: u32,
    pub logical_pages: u32,
    pub _reserved: u32,
}

#[repr(C)]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable,
)]
pub struct RaggedBatchOutput {
    pub request_id: u64,
    pub slot: u32,
    pub next_token_id: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RaggedBatchReceipt {
    pub batch_size: u32,
    pub backend_launches: u32,
    pub device_to_host_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RaggedBackendError {
    OutputTooSmall,
    Rejected,
}

/// Prepared backend boundary for one continuous-batching decode round.
///
/// `items` and `block_tables` are immutable caller storage. `out` has at least `items.len()`
/// entries. Implementations may not retain any slice after returning.
pub trait RaggedDecodeBackend {
    fn execute_ragged(
        &mut self,
        items: &[RaggedBatchItem],
        block_tables: &[u32],
        out: &mut [RaggedBatchOutput],
    ) -> Result<RaggedBatchReceipt, RaggedBackendError>;
}
