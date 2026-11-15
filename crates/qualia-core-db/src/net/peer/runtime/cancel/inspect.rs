//! Exact operation generation/cancel snapshot plus actual held lease.
use super::{CancelEpoch, OperationHandle, OperationTable};
use crate::net::{peer::runtime::LeaseTable, qdnf::errors::QdnfError as E};
#[derive(Clone, Copy)]
pub(crate) struct OperationSnapshot {
    pub cancelled: bool,
    pub epoch: CancelEpoch,
}
impl OperationTable {
    pub(crate) fn check_storage(
        &self,
        handle: OperationHandle,
        leases: &LeaseTable,
        capacity: u32,
    ) -> Result<OperationSnapshot, E> {
        let slot = self.find(handle.id).ok_or(E::StaleGeneration)?;
        if slot.generation != handle.generation || !slot.storage_held {
            return Err(E::StaleGeneration);
        }
        leases.check_lease(slot.lease, capacity, true)?;
        Ok(OperationSnapshot {
            cancelled: slot.cancelled,
            epoch: slot.cancel_epoch,
        })
    }
}
