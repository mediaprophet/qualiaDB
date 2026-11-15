//! The original runtime fields MOVE here between polls; never cloned/rebuilt.
use super::super::owner::HOT_SLOTS;
#[cfg(not(target_arch = "wasm32"))]
use super::super::recovery_source::{RecoverySourceSlot, MAX_RETAINED_SOURCES};
use super::super::sparse_workspace::{SparseWorkspaceSlot, MAX_SPARSE_WORKSPACES};
use super::super::types::Slot;
use super::super::{RuntimeProfile, TransferRuntime};
use crate::net::peer::runtime::{
    FairScheduler, LeaseTable, OperationHandle, OperationTable, ReservationHandle,
    ReservationLedger, ResourceBudget, ResourceGovernor,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum RuntimeClosePhase {
    Open,
    ControlReleased,
    ControlChargeReleased,
    Closed,
}
pub(super) struct RuntimeData {
    pub profile: RuntimeProfile,
    pub slots: [Slot; HOT_SLOTS],
    pub sparse_workspaces: [SparseWorkspaceSlot; MAX_SPARSE_WORKSPACES],
    pub workspace_live: usize,
    #[cfg(not(target_arch = "wasm32"))]
    pub recovery_sources: [RecoverySourceSlot; MAX_RETAINED_SOURCES],
    pub source_live: usize,
    pub owner: usize,
    pub serial: u64,
    pub static_charge: Option<ReservationHandle>,
    pub control_charge: Option<ReservationHandle>,
    pub control_operation: OperationHandle,
    pub static_charge_bytes: u64,
    pub control_budget: ResourceBudget,
    pub used_bytes: u64,
    pub live: usize,
    pub pending_records: usize,
    pub closed: bool,
    pub close_phase: RuntimeClosePhase,
}
impl RuntimeData {
    /// Infallible ownership move. TransferRuntime must remain without a custom
    /// Drop: this destructuring is compiler-checked when fields are added.
    pub fn take(runtime: TransferRuntime<'_>) -> Self {
        let TransferRuntime {
            ledger: _,
            leases: _,
            operations: _,
            scheduler: _,
            governor: _,
            profile,
            slots,
            sparse_workspaces,
            workspace_live,
            #[cfg(not(target_arch = "wasm32"))]
            recovery_sources,
            source_live,
            owner,
            serial,
            static_charge,
            control_charge,
            control_operation,
            static_charge_bytes,
            control_budget,
            used_bytes,
            live,
            pending_records,
            closed,
            close_phase,
        } = runtime;
        Self {
            profile,
            slots,
            sparse_workspaces,
            workspace_live,
            #[cfg(not(target_arch = "wasm32"))]
            recovery_sources,
            source_live,
            owner,
            serial,
            static_charge,
            control_charge,
            control_operation,
            static_charge_bytes,
            control_budget,
            used_bytes,
            live,
            pending_records,
            closed,
            close_phase,
        }
    }
    /// Reattach ONLY after actual parent identity/handle preflight. No admission,
    /// counter reset, token reconstruction, callbacks, allocation or fallible work.
    pub fn attach<'a>(
        self,
        ledger: &'a mut ReservationLedger,
        leases: &'a mut LeaseTable,
        operations: &'a mut OperationTable,
        scheduler: &'a mut FairScheduler,
        governor: &'a ResourceGovernor,
    ) -> TransferRuntime<'a> {
        TransferRuntime {
            ledger,
            leases,
            operations,
            scheduler,
            governor,
            profile: self.profile,
            slots: self.slots,
            sparse_workspaces: self.sparse_workspaces,
            workspace_live: self.workspace_live,
            #[cfg(not(target_arch = "wasm32"))]
            recovery_sources: self.recovery_sources,
            source_live: self.source_live,
            owner: self.owner,
            serial: self.serial,
            static_charge: self.static_charge,
            control_charge: self.control_charge,
            control_operation: self.control_operation,
            static_charge_bytes: self.static_charge_bytes,
            control_budget: self.control_budget,
            used_bytes: self.used_bytes,
            live: self.live,
            pending_records: self.pending_records,
            closed: self.closed,
            close_phase: self.close_phase,
        }
    }
}
