//! Read-only preflight BEFORE taking the only active state out of its shell.
use super::super::{sparse_workspace::SparsePageWorkspace, TransferState};
use super::{data::RuntimeClosePhase, loans::ParentLoans, RuntimeData};
use crate::net::{peer::runtime::ResourceBudget, qdnf::errors::QdnfError as E};
impl RuntimeData {
    pub fn preflight(&self, loans: &ParentLoans<'_>) -> Result<(), E> {
        let mut accounted = 0u64;
        if let Some(h) = &self.static_charge {
            loans.ledger.check_reservation(
                h,
                ResourceBudget {
                    bytes: self.static_charge_bytes,
                    work: 0,
                    io: 0,
                },
            )?;
            accounted = accounted
                .checked_add(self.static_charge_bytes)
                .ok_or(E::Range)?;
        } else if !self.closed {
            return Err(E::Conflict);
        }
        if let Some(h) = &self.control_charge {
            loans.ledger.check_reservation(h, self.control_budget)?;
            accounted = accounted
                .checked_add(self.control_budget.bytes)
                .ok_or(E::Range)?;
        } else if self.close_phase == RuntimeClosePhase::Open
            || self.close_phase == RuntimeClosePhase::ControlReleased
        {
            return Err(E::Conflict);
        }
        if self.close_phase == RuntimeClosePhase::Open {
            loans.operations.check_storage(
                self.control_operation,
                &loans.leases,
                u32::try_from(self.control_budget.bytes).map_err(|_| E::Range)?,
            )?;
        }
        let (mut live, mut queued) = (0usize, 0usize);
        for slot in &self.slots {
            if slot.state == TransferState::Vacant {
                if slot.reservation.is_some() || slot.operation.is_some() || slot.queued {
                    return Err(E::Conflict);
                }
                continue;
            }
            live += 1;
            loans
                .ledger
                .check_reservation(slot.reservation.as_ref().ok_or(E::Conflict)?, slot.budget)?;
            let observed = loans.operations.check_storage(
                slot.operation.ok_or(E::Conflict)?,
                &loans.leases,
                u32::try_from(slot.budget.bytes).map_err(|_| E::Range)?,
            )?;
            if observed.epoch < slot.cancel_epoch {
                return Err(E::StaleGeneration);
            }
            if matches!(slot.state, TransferState::Cancelling(_)) && !observed.cancelled {
                return Err(E::Conflict);
            }
            accounted = accounted.checked_add(slot.budget.bytes).ok_or(E::Range)?;
            if slot.queued {
                queued += 1;
                if !loans.scheduler.contains_work(slot.work.ok_or(E::Conflict)?) {
                    return Err(E::Conflict);
                }
            }
        }
        let mut pages = 0usize;
        for slot in &self.sparse_workspaces {
            if let Some(h) = &slot.charge {
                pages += 1;
                let bytes =
                    u64::try_from(SparsePageWorkspace::charged_bytes()).map_err(|_| E::Range)?;
                loans.ledger.check_reservation(
                    h,
                    ResourceBudget {
                        bytes,
                        work: 0,
                        io: 0,
                    },
                )?;
                accounted = accounted.checked_add(bytes).ok_or(E::Range)?;
            }
        }
        let mut sources = 0usize;
        #[cfg(not(target_arch = "wasm32"))]
        for slot in &self.recovery_sources {
            if let Some(h) = &slot.charge {
                sources += 1;
                let bytes = u64::try_from(super::super::RetainedRecoverySource::charged_bytes())
                    .map_err(|_| E::Range)?;
                loans.ledger.check_reservation(
                    h,
                    ResourceBudget {
                        bytes,
                        work: 16,
                        io: 4,
                    },
                )?;
                accounted = accounted.checked_add(bytes).ok_or(E::Range)?;
            }
        }
        if live != self.live
            || queued != self.pending_records
            || pages != self.workspace_live
            || sources != self.source_live
            || accounted != self.used_bytes
        {
            return Err(E::Conflict);
        }
        Ok(())
    }
}
