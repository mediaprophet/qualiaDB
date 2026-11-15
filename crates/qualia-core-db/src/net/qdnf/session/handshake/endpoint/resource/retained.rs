//! Consumes the original parent handle, without releasing/re-admitting it.
use super::{EndpointReservation, ENDPOINT_RESERVATION_BYTES};
use crate::net::{
    peer::runtime::{ReservationHandle, ReservationLedger},
    qdnf::errors::QdnfError as E,
};
use std::sync::{Mutex, TryLockError};
mod identity;

/// Lost owners retain the original canonical charge; Drop never fakes drain.
/// Only the actual channel's successful shutdown may call release_after_shutdown.
pub(in super::super) struct RetainedEndpointReservation<'a> {
    ledger: &'a Mutex<ReservationLedger>,
    handle: Option<ReservationHandle>,
    incarnation: crate::net::peer::runtime::incarnation::CanonicalIncarnation,
}
impl<'a> EndpointReservation<'a> {
    pub(in super::super) fn retain_for_channel(
        &mut self,
    ) -> Result<RetainedEndpointReservation<'a>, E> {
        let handle = self.handle.take().ok_or(E::Closed)?;
        Ok(RetainedEndpointReservation {
            ledger: self.ledger,
            handle: Some(handle),
            incarnation: self.incarnation,
        })
    }
}
fn lock_error<T>(e: TryLockError<T>) -> E {
    match e {
        TryLockError::WouldBlock => E::WouldBlock,
        TryLockError::Poisoned(_) => E::Unauthorized,
    }
}
impl RetainedEndpointReservation<'_> {
    pub(in super::super) fn with_ledger<T>(
        &mut self,
        effect: impl for<'s> FnOnce(&'s mut ReservationLedger) -> Result<T, E>,
    ) -> Result<T, E> {
        if self.handle.is_none() {
            return Err(E::Closed);
        }
        let mut ledger = self.ledger.try_lock().map_err(lock_error)?;
        self.check_identity(&ledger)?;
        effect(&mut ledger)
    }
    pub(in super::super) fn release_after_shutdown(&mut self) -> Result<(), E> {
        if self.handle.is_none() {
            return Err(E::DoubleRelease);
        }
        let mut ledger = self.ledger.try_lock().map_err(lock_error)?;
        self.check_identity(&ledger)?;
        let handle = self.handle.take().ok_or(E::DoubleRelease)?;
        let exact = handle.as_ref();
        if let Err(e) = ledger.release_checked_bytes(handle, ENDPOINT_RESERVATION_BYTES) {
            // This parent function refuses BEFORE mutation. Restore only the
            // exact consumed private handle, never an amount/caller reference.
            self.handle = Some(ReservationHandle::from_ref(exact));
            return Err(e);
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "retained/tests.rs"]
mod tests;
