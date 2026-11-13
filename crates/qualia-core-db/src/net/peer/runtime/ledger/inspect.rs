//! Read-only exact reservation validation; never a release/admission.
use super::{ReservationHandle, ReservationLedger, ResourceBudget};
use crate::net::qdnf::errors::QdnfError as E;
impl ReservationLedger {
    pub(crate) fn check_reservation(
        &self,
        handle: &ReservationHandle,
        expected: ResourceBudget,
    ) -> Result<(), E> {
        let slot = self
            .handles
            .get(handle.slot as usize)
            .ok_or(E::StaleGeneration)?;
        if !slot.occupied || slot.generation != handle.generation {
            return Err(E::StaleGeneration);
        }
        if slot.budget != expected {
            return Err(E::Conflict);
        }
        Ok(())
    }
}
