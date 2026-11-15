//! Non-Copy job ticket bound to the original transfer; not a second ledger.
use super::super::{CompletionKey, TransferTicket};
use crate::net::qdnf::{errors::QdnfError, types::Generation};

/// Matches the runtime hot-slot bound. Not an extra reservation table.
pub const MAX_STORAGE_JOBS: usize = 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum JobState {
    Vacant,
    Submitted,
    Cancelling,
    Failed,
}

#[derive(Clone, Copy)]
pub(super) struct JobSlot {
    pub generation: Generation,
    pub state: JobState,
    pub key: CompletionKey,
    pub reserved_bytes: u64,
}

impl JobSlot {
    pub const fn empty() -> Self {
        Self {
            generation: Generation::ZERO,
            state: JobState::Vacant,
            key: CompletionKey {
                owner: 0,
                slot: 0,
                generation: Generation::ZERO,
                operation: crate::net::qdnf::types::OperationId::ZERO,
            },
            reserved_bytes: 0,
        }
    }
    pub fn clear(&mut self) {
        let generation = self.generation;
        *self = Self::empty();
        self.generation = generation;
    }
}

/// Generation-fenced worker identity. Completion still requires the original
/// non-Copy [`TransferTicket`]. Copy would allow a second complete/refund.
#[must_use = "retain the job ticket through cancellation and actual I/O completion"]
#[derive(Debug)]
pub struct StorageJobTicket {
    pub(super) key: CompletionKey,
    pub(super) job_generation: Generation,
    pub(super) job_slot: u8,
    pub(super) reserved_bytes: u64,
    pub(super) spent: bool,
}

impl StorageJobTicket {
    pub const fn completion_key(&self) -> CompletionKey {
        self.key
    }
    pub const fn reserved_bytes(&self) -> u64 {
        self.reserved_bytes
    }
    pub(super) fn matches_transfer(&self, ticket: &TransferTicket) -> Result<(), QdnfError> {
        if self.spent {
            return Err(QdnfError::DoubleRelease);
        }
        if self.key != ticket.completion_key() {
            return Err(QdnfError::StaleGeneration);
        }
        Ok(())
    }
}
