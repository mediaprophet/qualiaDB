//! ONE canonical admission algorithm used by legacy and retained scope owners.
use super::super::{state::RuntimeClosePhase, RuntimeProfile};
use super::{schedule_id, Slot, TransferRuntime, NEXT_OWNER};
use crate::net::peer::runtime::{
    FairScheduler, LeaseTable, OperationTable, ReservationLedger, ResourceBudget, ResourceGovernor,
    WorkClass,
};
use crate::net::qdnf::errors::QdnfError as E;
use core::sync::atomic::Ordering;
impl<'a> TransferRuntime<'a> {
    pub(in super::super) fn new_counted(
        ledger: &'a mut ReservationLedger,
        leases: &'a mut LeaseTable,
        operations: &'a mut OperationTable,
        scheduler: &'a mut FairScheduler,
        governor: &'a ResourceGovernor,
        profile: RuntimeProfile,
        control: ResourceBudget,
        static_bytes: u64,
    ) -> Result<Self, E> {
        if control.bytes == 0 || control.work == 0 || control.io == 0 {
            return Err(E::Malformed);
        }
        if static_bytes < u64::try_from(core::mem::size_of::<Self>()).map_err(|_| E::Range)? {
            return Err(E::Range);
        }
        let control_capacity = u32::try_from(control.bytes).map_err(|_| E::Range)?;
        let used_bytes = static_bytes.checked_add(control.bytes).ok_or(E::Range)?;
        if used_bytes > profile.window_bytes() {
            return Err(E::BudgetExhausted);
        }
        let owner = NEXT_OWNER
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| E::StaleGeneration)?;
        let control_id = schedule_id(owner, 0)?;
        let static_charge = governor.admit_and_reserve(
            ledger,
            ResourceBudget {
                bytes: static_bytes,
                work: 0,
                io: 0,
            },
            WorkClass::Background,
            true,
        )?;
        let control_charge =
            match governor.admit_and_reserve(ledger, control, WorkClass::Control, true) {
                Ok(h) => h,
                Err(e) => {
                    ledger.release(static_charge)?;
                    return Err(e);
                }
            };
        let control_lease = match leases.acquire(control_capacity, true) {
            Ok(l) => l,
            Err(e) => {
                let a = ledger.release(control_charge);
                let b = ledger.release(static_charge);
                a?;
                b?;
                return Err(e);
            }
        };
        let control_operation = match operations.admit(control_id, control_lease.handle) {
            Ok(h) => h,
            Err(e) => {
                let a = leases.release(control_lease.handle);
                let b = ledger.release(control_charge);
                let c = ledger.release(static_charge);
                a?;
                b?;
                c?;
                return Err(e);
            }
        };
        Ok(Self {
            ledger,
            leases,
            operations,
            scheduler,
            governor,
            profile,
            slots: core::array::from_fn(|_| Slot::empty()),
            sparse_workspaces: core::array::from_fn(|_| {
                super::super::sparse_workspace::SparseWorkspaceSlot::empty()
            }),
            workspace_live: 0,
            #[cfg(not(target_arch = "wasm32"))]
            recovery_sources: core::array::from_fn(|_| {
                super::super::recovery_source::RecoverySourceSlot::empty()
            }),
            source_live: 0,
            owner,
            serial: 0,
            static_charge: Some(static_charge),
            control_charge: Some(control_charge),
            control_operation,
            static_charge_bytes: static_bytes,
            control_budget: control,
            used_bytes,
            live: 0,
            pending_records: 0,
            closed: false,
            close_phase: RuntimeClosePhase::Open,
        })
    }
}
