//! Private-field local tickets and callback identities; never wire capabilities.
use crate::net::peer::runtime::{
    CancelEpoch, OperationHandle, PollOutcome, ReservationHandle, ResourceBudget, ScheduledWork,
};
use crate::net::qdnf::{
    errors::QdnfError,
    types::{Generation, OperationId},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeProfile {
    Constrained,
    Desktop,
}
impl RuntimeProfile {
    pub const fn inflight_limit(self) -> usize {
        match self {
            Self::Constrained => 8,
            Self::Desktop => 30,
        }
    }
    pub const fn window_bytes(self) -> u64 {
        match self {
            Self::Constrained => 8 * 1024 * 1024,
            Self::Desktop => 32 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelReason {
    Cancelled,
    Expired,
    Revoked,
}
impl CancelReason {
    pub(super) const fn error(self) -> QdnfError {
        match self {
            Self::Cancelled => QdnfError::Cancelled,
            Self::Expired => QdnfError::Expired,
            Self::Revoked => QdnfError::Revoked,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferState {
    Vacant,
    Queued,
    Ready,
    InFlight,
    Cancelling(CancelReason),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinishOutcome {
    Released,
    Cancelled,
    Expired,
    Revoked,
}
/// A callback key alone cannot reclaim a reservation or authorize a request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletionKey {
    pub(super) owner: usize,
    pub(super) slot: u8,
    pub(super) generation: Generation,
    pub(super) operation: OperationId,
}
impl CompletionKey {
    pub const fn operation(self) -> OperationId {
        self.operation
    }
}
#[must_use = "retain the ticket through cancellation and actual I/O completion"]
#[derive(Debug)]
pub struct TransferTicket {
    pub(super) key: CompletionKey,
    pub(super) spent: bool,
}
impl TransferTicket {
    pub const fn completion_key(&self) -> CompletionKey {
        self.key
    }
}
#[must_use = "credit must be combined with current source/policy authority before sending"]
#[derive(Debug)]
pub struct RequestCredit {
    pub(super) key: CompletionKey,
    pub(super) bytes: u32,
    pub(super) cancel_epoch: CancelEpoch,
}
impl RequestCredit {
    pub const fn completion_key(&self) -> CompletionKey {
        self.key
    }
    pub const fn bytes(&self) -> u32 {
        self.bytes
    }
    pub const fn cancel_epoch(&self) -> CancelEpoch {
        self.cancel_epoch
    }
    pub const fn operation(&self) -> OperationId {
        self.key.operation
    }
}
#[derive(Debug)]
pub enum RuntimePoll {
    Ready(CompletionKey),
    Cancelled(CompletionKey, CancelReason),
    External(PollOutcome),
}

pub(super) struct Slot {
    pub generation: Generation,
    pub state: TransferState,
    pub operation: Option<OperationHandle>,
    pub work: Option<ScheduledWork>,
    pub queued: bool,
    pub reservation: Option<ReservationHandle>,
    pub budget: ResourceBudget,
    pub deadline: u64,
    pub policy: Generation,
    pub credit: u32,
    pub proof_bytes: u64,
    pub storage_bytes: u64,
    pub cancel_epoch: CancelEpoch,
}
impl Slot {
    pub fn empty() -> Self {
        Self {
            generation: Generation::ZERO,
            state: TransferState::Vacant,
            operation: None,
            work: None,
            queued: false,
            reservation: None,
            budget: ResourceBudget::ZERO,
            deadline: 0,
            policy: Generation::ZERO,
            credit: 0,
            proof_bytes: 0,
            storage_bytes: 0,
            cancel_epoch: CancelEpoch::ZERO,
        }
    }
    pub fn clear(&mut self) {
        let generation = self.generation;
        *self = Self::empty();
        self.generation = generation;
    }
}
pub(super) fn schedule_id(owner: usize, serial: u64) -> Result<OperationId, QdnfError> {
    let owner = u64::try_from(owner).map_err(|_| QdnfError::Range)?;
    let mut id = [0; 16];
    id[..8].copy_from_slice(&owner.to_le_bytes());
    id[8..].copy_from_slice(&serial.to_le_bytes());
    Ok(OperationId(id))
}
