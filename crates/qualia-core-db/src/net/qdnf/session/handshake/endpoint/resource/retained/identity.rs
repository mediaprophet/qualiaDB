//! Original actual ledger identity/budget checks, with no relock or new charge.
use super::super::ENDPOINT_BUDGET;
use super::RetainedEndpointReservation;
use crate::bundle::stream::runtime::RuntimeAccess;
use crate::net::{peer::runtime::ReservationLedger, qdnf::errors::QdnfError as E};
impl RetainedEndpointReservation<'_> {
    pub(in super::super::super) fn check_identity(
        &self,
        ledger: &ReservationLedger,
    ) -> Result<(), E> {
        if !self.incarnation.assigned() || ledger.existing_incarnation() != self.incarnation {
            return Err(E::StaleGeneration);
        }
        ledger.check_reservation(self.handle.as_ref().ok_or(E::Closed)?, ENDPOINT_BUDGET)
    }
    pub(in super::super::super) fn check_runtime_access(
        &self,
        access: &RuntimeAccess<'_, '_>,
    ) -> Result<(), E> {
        access.verify_external_reservation(
            self.ledger,
            self.incarnation,
            self.handle.as_ref().ok_or(E::Closed)?,
            ENDPOINT_BUDGET,
        )
    }
}
