//! Narrow forwarding view; no DerefMut/raw runtime/parent owner escape.
use super::super::sparse_workspace::{SparsePageWorkspace, WorkspaceReleaseError};
use super::super::{
    CancelReason, ColdPlan, CompletionKey, FinishOutcome, RequestCredit, RuntimePoll, StorageStage,
    TransferRuntime, TransferState, TransferTicket,
};
use crate::bundle::stream::storage::StorageIdentity;
use crate::net::{
    peer::runtime::{AdmissionScopes, CancelEpoch},
    qdnf::{errors::QdnfError as E, types::Generation},
};
pub(crate) struct RuntimeAccess<'borrow, 'parents> {
    runtime: &'borrow mut TransferRuntime<'parents>,
    ledger_owner: &'borrow std::sync::Mutex<crate::net::peer::runtime::ReservationLedger>,
    ledger_incarnation: crate::net::peer::runtime::incarnation::CanonicalIncarnation,
}
impl<'b, 'p> RuntimeAccess<'b, 'p> {
    pub(super) fn new(
        runtime: &'b mut TransferRuntime<'p>,
        ledger_owner: &'b std::sync::Mutex<crate::net::peer::runtime::ReservationLedger>,
        ledger_incarnation: crate::net::peer::runtime::incarnation::CanonicalIncarnation,
    ) -> Self {
        Self {
            runtime,
            ledger_owner,
            ledger_incarnation,
        }
    }
    /// Same actual held ledger: validation only, never permission/new charge.
    pub(crate) fn verify_external_reservation(
        &self,
        owner: &std::sync::Mutex<crate::net::peer::runtime::ReservationLedger>,
        incarnation: crate::net::peer::runtime::incarnation::CanonicalIncarnation,
        handle: &crate::net::peer::runtime::ReservationHandle,
        expected: crate::net::peer::runtime::ResourceBudget,
    ) -> Result<(), E> {
        if !core::ptr::eq(owner, self.ledger_owner)
            || !incarnation.assigned()
            || incarnation != self.ledger_incarnation
            || self.runtime.ledger.existing_incarnation() != incarnation
        {
            return Err(E::StaleGeneration);
        }
        self.runtime.ledger.check_reservation(handle, expected)
    }
    pub(crate) fn admit(
        &mut self,
        plan: &ColdPlan<'_>,
        index: usize,
        now: u64,
        policy: Generation,
    ) -> Result<TransferTicket, E> {
        self.runtime.admit(plan, index, now, policy)
    }
    pub(crate) fn poll(&mut self, now: u64) -> Result<RuntimePoll, E> {
        self.runtime.poll(now)
    }
    pub(crate) fn issue_credit(
        &mut self,
        ticket: &TransferTicket,
        now: u64,
        policy: Generation,
    ) -> Result<RequestCredit, E> {
        self.runtime.issue_credit(ticket, now, policy)
    }
    pub(crate) fn state(&self, ticket: &TransferTicket) -> Result<TransferState, E> {
        self.runtime.state(ticket)
    }
    pub(crate) fn cancel_epoch(&self, ticket: &TransferTicket) -> Result<CancelEpoch, E> {
        self.runtime.cancel_epoch(ticket)
    }
    pub(crate) fn cancel(&mut self, ticket: &TransferTicket) -> Result<(), E> {
        self.runtime.cancel(ticket)
    }
    pub(crate) fn expire(&mut self, now: u64) -> Result<usize, E> {
        self.runtime.expire(now)
    }
    pub(crate) fn complete_io(
        &mut self,
        ticket: &mut TransferTicket,
        key: CompletionKey,
        now: u64,
    ) -> Result<FinishOutcome, E> {
        self.runtime.complete_io(ticket, key, now)
    }
    pub(crate) fn live_transfers(&self) -> usize {
        self.runtime.live_transfers()
    }
    pub(crate) fn pending_schedule_records(&self) -> usize {
        self.runtime.pending_schedule_records()
    }
    pub(crate) fn charged_window_bytes(&self) -> u64 {
        self.runtime.charged_window_bytes()
    }
    pub(crate) fn supported_inflight(&self) -> usize {
        self.runtime.supported_inflight()
    }
    pub(crate) fn parent_used(&self) -> AdmissionScopes {
        self.runtime.parent_used()
    }
    pub(crate) fn acquire_sparse_page(
        &mut self,
        identity: StorageIdentity,
        page: u64,
    ) -> Result<SparsePageWorkspace, E> {
        self.runtime.acquire_sparse_page(identity, page)
    }
    pub(crate) fn release_sparse_page(
        &mut self,
        page: SparsePageWorkspace,
    ) -> Result<(), WorkspaceReleaseError> {
        self.runtime.release_sparse_page(page)
    }
    /// Bounded memory/proof checks ONLY in this short scope. Do not call actual
    /// synchronous native read/write/flush here. Storage job handoff is separate.
    pub(crate) fn storage_stage<'s>(
        &'s mut self,
        ticket: &'s TransferTicket,
        payload: &'s [u8],
        proof: &'s [u8],
        scratch: &'s mut [u8],
        now: u64,
        policy: Generation,
    ) -> Result<StorageStage<'s, 'p>, E> {
        self.runtime
            .storage_stage(ticket, payload, proof, scratch, now, policy)
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn release_recovery_source<'root>(
        &mut self,
        source: super::super::RetainedRecoverySource<'root>,
    ) -> Result<(), super::super::SourceReleaseError<'root>> {
        self.runtime.release_recovery_source(source)
    }
    pub(super) fn observe_cancellation(&mut self) -> Result<(), E> {
        // Reuse the actual canonical cancellation transition; no copied kernel.
        for index in 0..self.runtime.slots.len() {
            let Some(op) = self.runtime.slots[index].operation else {
                continue;
            };
            if self.runtime.operations.is_cancelled(op.id) {
                self.runtime.cancel_slot(index, CancelReason::Cancelled)?;
            }
        }
        Ok(())
    }
}
