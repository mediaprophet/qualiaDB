//! Borrow actual caller buffers under the original in-flight reservation.
//! This synchronous stage neither allocates a second ledger nor grants rights.
mod sparse;
mod controlled;
pub use controlled::{StorageBuffers, StorageStageControl, ProofStorageBuffers};

use super::{CancelReason, TransferRuntime, TransferState, TransferTicket};
use crate::net::qdnf::{errors::QdnfError, types::Generation};

/// Scoped storage work. Drop returns borrows; it does not complete I/O or
/// release reservations. The runtime/ticket remain live until explicit drain.
/// The exclusive runtime borrow prevents concurrent completion/slot reuse.
/// Hosts must sample current policy and time again before accepting effects;
/// this guard does not manufacture those trusted inputs or authenticate bytes.
pub struct StorageStage<'s, 'owners> {
    runtime: &'s mut TransferRuntime<'owners>,
    ticket: &'s TransferTicket,
    payload: &'s [u8],
    proof: &'s [u8],
    scratch: &'s mut [u8],
}
impl<'owners> TransferRuntime<'owners> {
    /// Reuses the received payload's network charge. Proof and storage scratch
    /// must fit their separately admitted components. No hidden copy is made.
    /// Intended for bounded synchronous operations, not an async owner handoff.
    pub fn storage_stage<'s>(
        &'s mut self,
        ticket: &'s TransferTicket,
        payload: &'s [u8],
        proof: &'s [u8],
        scratch: &'s mut [u8],
        now: u64,
        current_policy: Generation,
    ) -> Result<StorageStage<'s, 'owners>, QdnfError> {
        self.check_storage_stage(ticket, now, current_policy)?;
        let slot = &self.slots[self.validate(ticket)?];
        if payload.len() as u64 > u64::from(slot.credit)
            || proof.len() as u64 > slot.proof_bytes
            || scratch.len() as u64 > slot.storage_bytes
        {
            return Err(QdnfError::Capacity);
        }
        Ok(StorageStage {
            runtime: self,
            ticket,
            payload,
            proof,
            scratch,
        })
    }
    fn check_storage_stage(
        &mut self,
        ticket: &TransferTicket,
        now: u64,
        current_policy: Generation,
    ) -> Result<(), QdnfError> {
        let i = self.validate(ticket)?;
        if now >= self.slots[i].deadline {
            self.cancel_slot(i, CancelReason::Expired)?;
        }
        if current_policy != self.slots[i].policy {
            self.cancel_slot(i, CancelReason::Revoked)?;
        }
        match self.slots[i].state {
            TransferState::InFlight => Ok(()),
            TransferState::Cancelling(reason) => Err(reason.error()),
            _ => Err(QdnfError::WouldBlock),
        }
    }
}
impl StorageStage<'_, '_> {
    /// Recheck immediately before a write/accepted checkpoint using host inputs.
    pub fn recheck(&mut self, now: u64, current_policy: Generation) -> Result<(), QdnfError> {
        self.runtime
            .check_storage_stage(self.ticket, now, current_policy)
    }
    /// Process cancellation through the same runtime while this stage is held.
    pub fn cancel(&mut self) -> Result<(), QdnfError> {
        self.runtime.cancel(self.ticket)
    }
    /// Borrow input and scratch only after a fresh lifecycle check. These bytes
    /// remain unverified until the canonical proof/format owners admit them.
    pub fn buffers(
        &mut self,
        now: u64,
        current_policy: Generation,
    ) -> Result<(&[u8], &[u8], &mut [u8]), QdnfError> {
        self.recheck(now, current_policy)?;
        Ok((self.payload, self.proof, self.scratch))
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod recovery;
