//! One short guard over the actual canonical owner; no permit/owner escape.
use super::{AuthorityBinding, AuthorityOwner, MutationHandle};
use crate::net::qdnf::errors::QdnfError as E;
use std::sync::{Mutex, MutexGuard, TryLockError};

/// Host-owned synchronous authority scope. Do not await, reenter this mutex,
/// drop its controllers, or call unbounded/blocking peer I/O while held.
/// This guard establishes owner affinity, not consent or remote admission.
pub struct CurrentAuthorityGuard<'a> {
    owner: &'a Mutex<AuthorityOwner>,
    guard: MutexGuard<'a, AuthorityOwner>,
}
impl<'a> CurrentAuthorityGuard<'a> {
    /// Nested/contended scopes refuse promptly rather than relocking.
    pub fn try_lock(owner: &'a Mutex<AuthorityOwner>) -> Result<Self, E> {
        let guard = owner.try_lock().map_err(|e| match e {
            TryLockError::WouldBlock => E::WouldBlock,
            TryLockError::Poisoned(_) => E::Unauthorized,
        })?;
        Ok(Self { owner, guard })
    }
    pub(crate) fn check_affinity(&self, owner: &Mutex<AuthorityOwner>) -> Result<(), E> {
        if !core::ptr::eq(self.owner, owner) {
            return Err(E::Unauthorized);
        }
        Ok(())
    }
    pub(crate) fn check_handle(
        &self,
        owner: &Mutex<AuthorityOwner>,
        handle: MutationHandle,
        binding: AuthorityBinding,
        now: u64,
        ceiling: u64,
    ) -> Result<(), E> {
        self.check_affinity(owner)?;
        self.guard
            .issue_permit(handle, binding, now, ceiling)
            .map(|_| ())
    }
    pub(crate) fn revoke_handle(
        &mut self,
        owner: &Mutex<AuthorityOwner>,
        handle: MutationHandle,
    ) -> Result<(), E> {
        self.check_affinity(owner)?;
        self.guard.revoke(handle)
    }
}
#[cfg(test)]
#[path = "current_guard/tests.rs"]
mod tests;
