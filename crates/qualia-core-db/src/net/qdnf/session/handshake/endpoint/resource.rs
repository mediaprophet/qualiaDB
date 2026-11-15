//! Retains a real parent reservation; no second budget ledger.
use super::E;
use crate::net::peer::runtime::{ReservationHandle, ReservationLedger};
use std::sync::Mutex;
mod identity;
pub(super) use identity::ENDPOINT_BUDGET;
mod retained;
pub(super) use retained::RetainedEndpointReservation;
/// Fixed endpoint+workspace charge; provider-internal allocation is separately qualified.
pub const ENDPOINT_RESERVATION_BYTES: u64 = 128 * 1024;
pub struct EndpointReservation<'a> {
    ledger: &'a Mutex<ReservationLedger>,
    handle: Option<ReservationHandle>,
    incarnation: crate::net::peer::runtime::incarnation::CanonicalIncarnation,
}
impl<'a> EndpointReservation<'a> {
    pub fn admit(ledger: &'a Mutex<ReservationLedger>) -> Result<Self, E> {
        Self::admit_bound(ledger)
    }
}
impl Drop for EndpointReservation<'_> {
    fn drop(&mut self) {
        self.release_on_drop();
    }
}

/// Mechanically counted retained owners plus explicit local transient allowance.
/// Provider-internal stack/heap remains the existing provider's separate budget;
/// no provider allocation qualification is inferred from these type sizes.
pub const COUNTED_ENDPOINT_BYTES: usize = core::mem::size_of::<super::EndpointWorkspace>()
    + core::mem::size_of::<super::Initiator<'static>>()
    + core::mem::size_of::<super::Responder<'static>>()
    + core::mem::size_of::<
        crate::specialized_libs::cryptographic_library::ManagedPairedSigning<'static>,
    >()
    + 24 * 1024;
const _: () = assert!(COUNTED_ENDPOINT_BYTES <= ENDPOINT_RESERVATION_BYTES as usize);
