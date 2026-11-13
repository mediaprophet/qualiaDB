//! Stable actual parent bindings plus per-VALUE incarnations, not copied caps.
use super::super::{RuntimeProfile, TransferRuntime};
use super::{loans::ParentLoans, RetainedRuntime, RuntimeData};
use crate::net::peer::runtime::incarnation::CanonicalIncarnation;
use crate::net::peer::runtime::{
    FairScheduler, LeaseTable, OperationTable, ReservationLedger, ResourceBudget, ResourceGovernor,
};
use crate::net::qdnf::errors::QdnfError as E;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct ParentIdentities {
    pub ledger: CanonicalIncarnation,
    pub leases: CanonicalIncarnation,
    pub operations: CanonicalIncarnation,
    pub scheduler: CanonicalIncarnation,
}
/// Exclusive registration of real table BINDINGS, with no long-lived locks.
/// One retained runtime initially. Independent ledger admissions remain possible.
pub(crate) struct RuntimeParents<'host> {
    pub(super) ledger: &'host Mutex<ReservationLedger>,
    pub(super) leases: &'host Mutex<LeaseTable>,
    pub(super) operations: &'host Mutex<OperationTable>,
    pub(super) scheduler: &'host Mutex<FairScheduler>,
    pub(super) governor: &'host ResourceGovernor,
    pub(super) identities: ParentIdentities,
    pub(super) registration: CanonicalIncarnation,
    pub(super) active: AtomicUsize,
    closed: AtomicBool,
}
impl<'host> RuntimeParents<'host> {
    pub(crate) fn adopt(
        ledger: &'host Mutex<ReservationLedger>,
        leases: &'host mut Mutex<LeaseTable>,
        operations: &'host mut Mutex<OperationTable>,
        scheduler: &'host mut Mutex<FairScheduler>,
        governor: &'host ResourceGovernor,
    ) -> Result<Self, E> {
        let mut loans = ParentLoans::acquire(ledger, leases, operations, scheduler)?;
        // Only initial adoption binds IDs; reattach NEVER repairs/reassigns IDs.
        let identities = ParentIdentities {
            ledger: loans.ledger.bind_incarnation()?,
            leases: loans.leases.bind_incarnation()?,
            operations: loans.operations.bind_incarnation()?,
            scheduler: loans.scheduler.bind_incarnation()?,
        };
        drop(loans);
        Ok(Self {
            ledger,
            leases,
            operations,
            scheduler,
            governor,
            identities,
            registration: CanonicalIncarnation::fresh()?,
            active: AtomicUsize::new(0),
            closed: AtomicBool::new(false),
        })
    }
    pub(crate) fn admit<'p>(
        &'p self,
        profile: RuntimeProfile,
        control: ResourceBudget,
    ) -> Result<RetainedRuntime<'p, 'host>, E> {
        if self.closed.load(Ordering::Acquire) {
            return Err(E::Closed);
        }
        let mut loans = self.loans()?;
        self.active
            .compare_exchange(0, usize::MAX, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| E::Capacity)?;
        // Guard clears ordinary refusal, but retains unknown ownership on panic.
        struct Claim<'a> {
            active: &'a AtomicUsize,
            committed: bool,
        }
        impl Drop for Claim<'_> {
            fn drop(&mut self) {
                if !self.committed && !std::thread::panicking() {
                    self.active.store(0, Ordering::Release);
                }
            }
        }
        let mut claim = Claim {
            active: &self.active,
            committed: false,
        };
        let counted = super::charged_bytes()?;
        let runtime = TransferRuntime::new_counted(
            &mut loans.ledger,
            &mut loans.leases,
            &mut loans.operations,
            &mut loans.scheduler,
            self.governor,
            profile,
            control,
            counted,
        )?;
        let data = RuntimeData::take(runtime);
        let owner = data.owner;
        self.active.store(owner, Ordering::Release);
        claim.committed = true;
        Ok(RetainedRuntime {
            parents: self,
            registration: self.registration,
            data: Some(data),
        })
    }
    pub(super) fn loans(&self) -> Result<ParentLoans<'_>, E> {
        if self.closed.load(Ordering::Acquire) {
            return Err(E::Closed);
        }
        let loans =
            ParentLoans::acquire(self.ledger, self.leases, self.operations, self.scheduler)?;
        if loans.identities() != self.identities {
            return Err(E::StaleGeneration);
        }
        Ok(loans)
    }
    pub(super) fn association(&self, state: &RetainedRuntime<'_, '_>) -> Result<(), E> {
        let data = state.data.as_ref().ok_or(E::Incomplete)?;
        if !core::ptr::eq(self, state.parents)
            || state.registration != self.registration
            || self.active.load(Ordering::Acquire) != data.owner
        {
            return Err(E::StaleGeneration);
        }
        Ok(())
    }
    /// Drop/lost state retains active count and canonical charge, so close refuses.
    pub(crate) fn try_close(&mut self) -> Result<(), E> {
        if self.active.load(Ordering::Acquire) != 0 {
            return Err(E::WouldBlock);
        }
        if self.closed.swap(true, Ordering::AcqRel) {
            return Err(E::DoubleRelease);
        }
        Ok(())
    }
}
