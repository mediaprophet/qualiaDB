//! Capture original ledger VALUE identity at actual admission, never at reattach.
use super::{EndpointReservation, ENDPOINT_RESERVATION_BYTES};
use crate::net::{
    peer::runtime::{ReservationLedger, ResourceBudget, ResourceGovernor},
    qdnf::errors::QdnfError as E,
};
use std::sync::Mutex;
pub(in super::super) const ENDPOINT_BUDGET: ResourceBudget = ResourceBudget {
    bytes: ENDPOINT_RESERVATION_BYTES,
    work: 16,
    io: 24,
};
impl<'a> EndpointReservation<'a> {
    pub(super) fn admit_bound(ledger: &'a Mutex<ReservationLedger>) -> Result<Self, E> {
        let mut owner = ledger.lock().map_err(|_| E::Unauthorized)?;
        let incarnation = owner.bind_incarnation()?;
        let handle = ResourceGovernor::charge_unknown(&mut owner, ENDPOINT_BUDGET)?;
        drop(owner);
        Ok(Self {
            ledger,
            handle: Some(handle),
            incarnation,
        })
    }
    pub(super) fn release_on_drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            if let Ok(mut ledger) = self.ledger.lock() {
                // Replaced same-address/equal-valued ledger is NOT our owner.
                if ledger.existing_incarnation() == self.incarnation {
                    let _ = ledger.release_checked_bytes(handle, ENDPOINT_BUDGET.bytes);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
