//! Read-only actual lease backing validation.
use super::LeaseTable;
use crate::net::{peer::runtime::LeaseHandle, qdnf::errors::QdnfError as E};
impl LeaseTable {
    pub(crate) fn check_lease(
        &self,
        handle: LeaseHandle,
        capacity: u32,
        exclusive: bool,
    ) -> Result<(), E> {
        let slot = self
            .slots
            .get(handle.slot as usize)
            .ok_or(E::StaleGeneration)?;
        if !slot.occupied || slot.generation != handle.generation {
            return Err(E::StaleGeneration);
        }
        if slot.capacity != capacity || slot.exclusive != exclusive {
            return Err(E::Conflict);
        }
        Ok(())
    }
}
