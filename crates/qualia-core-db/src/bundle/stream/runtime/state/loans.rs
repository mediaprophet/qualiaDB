//! Fixed try-lock order; later refusal drops all earlier guards unchanged.
use super::parents::ParentIdentities;
use crate::net::peer::runtime::{FairScheduler, LeaseTable, OperationTable, ReservationLedger};
use crate::net::qdnf::errors::QdnfError as E;
use std::sync::{Mutex, MutexGuard, TryLockError};
pub(super) struct ParentLoans<'s> {
    pub ledger: MutexGuard<'s, ReservationLedger>,
    pub leases: MutexGuard<'s, LeaseTable>,
    pub operations: MutexGuard<'s, OperationTable>,
    pub scheduler: MutexGuard<'s, FairScheduler>,
}
fn error<T>(e: TryLockError<T>) -> E {
    match e {
        TryLockError::WouldBlock => E::WouldBlock,
        TryLockError::Poisoned(_) => E::Unauthorized,
    }
}
impl<'s> ParentLoans<'s> {
    pub fn acquire(
        ledger: &'s Mutex<ReservationLedger>,
        leases: &'s Mutex<LeaseTable>,
        operations: &'s Mutex<OperationTable>,
        scheduler: &'s Mutex<FairScheduler>,
    ) -> Result<Self, E> {
        let ledger = ledger.try_lock().map_err(error)?;
        let leases = leases.try_lock().map_err(error)?;
        let operations = operations.try_lock().map_err(error)?;
        let scheduler = scheduler.try_lock().map_err(error)?;
        Ok(Self {
            ledger,
            leases,
            operations,
            scheduler,
        })
    }
    pub fn identities(&self) -> ParentIdentities {
        ParentIdentities {
            ledger: self.ledger.existing_incarnation(),
            leases: self.leases.existing_incarnation(),
            operations: self.operations.existing_incarnation(),
            scheduler: self.scheduler.existing_incarnation(),
        }
    }
}
