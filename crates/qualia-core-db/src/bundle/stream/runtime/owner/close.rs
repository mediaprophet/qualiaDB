//! Once-only actual control completion/charge release, shared by both owners.
use super::super::state::RuntimeClosePhase;
use super::TransferRuntime;
use crate::net::{peer::runtime::ReservationHandle, qdnf::errors::QdnfError as E};
impl TransferRuntime<'_> {
    pub fn close(&mut self) -> Result<(), E> {
        if self.closed {
            return Err(E::DoubleRelease);
        }
        if self.live != 0
            || self.pending_records != 0
            || self.workspace_live != 0
            || self.source_live != 0
        {
            return Err(E::WouldBlock);
        }
        if self.close_phase == RuntimeClosePhase::Open {
            match self.operations.complete(
                self.control_operation.id,
                self.control_operation.generation,
                self.leases,
            ) {
                Ok(()) | Err(E::Cancelled) => {}
                Err(e) => return Err(e),
            }
            self.close_phase = RuntimeClosePhase::ControlReleased;
        }
        if self.close_phase == RuntimeClosePhase::ControlReleased {
            let next = self
                .used_bytes
                .checked_sub(self.control_budget.bytes)
                .ok_or(E::Conflict)?;
            let handle = self.control_charge.take().ok_or(E::Incomplete)?;
            let original = handle.as_ref();
            if let Err(e) = self
                .ledger
                .release_checked_bytes(handle, self.control_budget.bytes)
            {
                self.control_charge = Some(ReservationHandle::from_ref(original));
                return Err(e);
            }
            self.used_bytes = next;
            self.close_phase = RuntimeClosePhase::ControlChargeReleased;
        }
        if self.close_phase == RuntimeClosePhase::ControlChargeReleased {
            let next = self
                .used_bytes
                .checked_sub(self.static_charge_bytes)
                .ok_or(E::Conflict)?;
            if next != 0 {
                return Err(E::Conflict);
            }
            let handle = self.static_charge.take().ok_or(E::Incomplete)?;
            let original = handle.as_ref();
            if let Err(e) = self
                .ledger
                .release_checked_bytes(handle, self.static_charge_bytes)
            {
                self.static_charge = Some(ReservationHandle::from_ref(original));
                return Err(e);
            }
            self.used_bytes = next;
            self.close_phase = RuntimeClosePhase::Closed;
            self.closed = true;
        }
        Ok(())
    }
}
