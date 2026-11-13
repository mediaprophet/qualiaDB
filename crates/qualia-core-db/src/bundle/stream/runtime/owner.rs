//! Scoped borrowing of canonical runtime owners; no copied root budgets.
#[cfg(not(target_arch = "wasm32"))]
use super::recovery_source::{RecoverySourceSlot, MAX_RETAINED_SOURCES};
use super::sparse_workspace::{SparseWorkspaceSlot, MAX_SPARSE_WORKSPACES};
use super::state::RuntimeClosePhase;
use super::types::{schedule_id, Slot};
use super::{RuntimeProfile, TransferState};
mod close;
mod counted;
use crate::net::peer::runtime::{
    FairScheduler, LeaseTable, OperationHandle, OperationTable, ReservationHandle,
    ReservationLedger, ResourceBudget, ResourceGovernor, WorkClass,
};
use crate::net::qdnf::errors::QdnfError;
use core::sync::atomic::AtomicUsize;
static NEXT_OWNER: AtomicUsize = AtomicUsize::new(1);
pub(super) const HOT_SLOTS: usize = 30;

#[must_use = "drain actual I/O and call close before releasing the scoped owners"]
pub struct TransferRuntime<'a> {
    pub(super) ledger: &'a mut ReservationLedger,
    pub(super) leases: &'a mut LeaseTable,
    pub(super) operations: &'a mut OperationTable,
    pub(super) scheduler: &'a mut FairScheduler,
    pub(super) governor: &'a ResourceGovernor,
    pub(super) profile: RuntimeProfile,
    pub(super) slots: [Slot; HOT_SLOTS],
    pub(super) sparse_workspaces: [SparseWorkspaceSlot; MAX_SPARSE_WORKSPACES],
    pub(super) workspace_live: usize,
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) recovery_sources: [RecoverySourceSlot; MAX_RETAINED_SOURCES],
    pub(super) source_live: usize,
    pub(super) owner: usize,
    pub(super) serial: u64,
    pub(super) static_charge: Option<ReservationHandle>,
    pub(super) control_charge: Option<ReservationHandle>,
    pub(super) control_operation: OperationHandle,
    pub(super) static_charge_bytes: u64,
    pub(super) control_budget: ResourceBudget,
    pub(super) close_phase: RuntimeClosePhase,
    pub(super) used_bytes: u64,
    pub(super) live: usize,
    pub(super) pending_records: usize,
    pub(super) closed: bool,
}
impl<'a> TransferRuntime<'a> {
    pub fn new(
        ledger: &'a mut ReservationLedger,
        leases: &'a mut LeaseTable,
        operations: &'a mut OperationTable,
        scheduler: &'a mut FairScheduler,
        governor: &'a ResourceGovernor,
        profile: RuntimeProfile,
        control: ResourceBudget,
    ) -> Result<Self, QdnfError> {
        let static_bytes =
            u64::try_from(core::mem::size_of::<Self>()).map_err(|_| QdnfError::Range)?;
        Self::new_counted(
            ledger,
            leases,
            operations,
            scheduler,
            governor,
            profile,
            control,
            static_bytes,
        )
    }
    pub const fn live_transfers(&self) -> usize {
        self.live
    }
    pub const fn pending_schedule_records(&self) -> usize {
        self.pending_records
    }
    pub const fn charged_window_bytes(&self) -> u64 {
        self.used_bytes
    }
    pub fn leased_slots(&self) -> usize {
        self.leases.occupied_count()
    }
    pub fn parent_used(&self) -> crate::net::peer::runtime::AdmissionScopes {
        self.ledger.used()
    }
    pub fn supported_inflight(&self) -> usize {
        let owned_limit = 30usize
            .saturating_sub(self.workspace_live)
            .saturating_sub(self.source_live);
        let actual_headroom = self.live.saturating_add(self.ledger.available_handles());
        owned_limit
            .min(actual_headroom)
            .min(self.profile.inflight_limit())
    }
    pub(super) fn require_open(&self) -> Result<(), QdnfError> {
        if self.closed || self.close_phase != RuntimeClosePhase::Open {
            Err(QdnfError::Closed)
        } else {
            Ok(())
        }
    }
    pub(super) fn key(&self, slot: usize) -> Result<super::CompletionKey, QdnfError> {
        let s = &self.slots[slot];
        let operation = s.operation.ok_or(QdnfError::Closed)?.id;
        Ok(super::CompletionKey {
            owner: self.owner,
            slot: slot as u8,
            generation: s.generation,
            operation,
        })
    }
    pub(super) fn validate(&self, ticket: &super::TransferTicket) -> Result<usize, QdnfError> {
        self.require_open()?;
        if ticket.spent {
            return Err(QdnfError::DoubleRelease);
        }
        let key = ticket.key;
        if key.owner != self.owner {
            return Err(QdnfError::StaleGeneration);
        }
        let i = usize::from(key.slot);
        let slot = self.slots.get(i).ok_or(QdnfError::StaleGeneration)?;
        if slot.state == TransferState::Vacant
            || slot.generation != key.generation
            || slot.operation.map(|h| h.id) != Some(key.operation)
        {
            return Err(QdnfError::StaleGeneration);
        }
        Ok(i)
    }
}
