//! Pre-effect recheck and explicit revoke/retire. No wrap/send/plaintext consumer.
use super::grant::RemoteDisclosureGrant;
use super::{ControllerRecognition, DisclosureAction, PolicyError as E};
use crate::crypto::network::authority::{AuthorityReceipt, PinAuthority};
use crate::net::qdnf::authority::{AuthorityOwner, CurrentAuthorityGuard};
use crate::net::qdnf::errors::QdnfError;
use std::sync::{Mutex, TryLockError};

impl RemoteDisclosureGrant<'_> {
    /// Recheck immediately before wrap/send/plaintext (those callers are R3/B2).
    pub fn recheck(
        &self,
        guard: &CurrentAuthorityGuard<'_>,
        owner: &Mutex<AuthorityOwner>,
        pins: &PinAuthority,
        recognition: &ControllerRecognition,
        receipt: &AuthorityReceipt,
        now: u64,
        expected_action: DisclosureAction,
    ) -> Result<(), E> {
        if self.closed || self.revoked {
            return Err(E::Authority(QdnfError::Revoked));
        }
        if !core::ptr::eq(self.owner, owner) {
            return Err(E::Authority(QdnfError::Unauthorized));
        }
        guard
            .check_affinity(owner)
            .and_then(|_| guard.check_handle(owner, self.handle, self.binding, now, self.ceiling))
            .map_err(E::Authority)?;
        let descriptor = receipt.descriptor_digest();
        if descriptor != &self.descriptor_a && descriptor != &self.descriptor_ticket {
            return Err(E::Reference);
        }
        pins.recheck(receipt, descriptor, now).map_err(E::Crypto)?;
        pins.recheck(&self.receipt_a, &self.descriptor_a, now)
            .map_err(E::Crypto)?;
        pins.recheck(&self.receipt_ticket, &self.descriptor_ticket, now)
            .map_err(E::Crypto)?;
        if now >= self.decision.expires() {
            return Err(E::Crypto(
                crate::crypto::network::errors::CryptoError::Expired,
            ));
        }
        if self.decision.action() != expected_action {
            return Err(E::Reference);
        }
        let live = recognition.recognize(
            self.recognized.principal(),
            self.recognized.source(),
            self.recognized.source_length(),
            self.recognized.policy(),
            self.recognized.publisher(),
            self.recognized.purpose(),
            self.recognized.operation(),
            expected_action,
            now,
        )?;
        if live.pin() != self.recognized.pin()
            || live.controller_generation() != self.recognized.controller_generation()
        {
            return Err(E::Authority(QdnfError::StaleGeneration));
        }
        Ok(())
    }

    /// Revoke under the caller's current guard. Retire is a separate step because
    /// `CurrentAuthorityGuard` does not expose `AuthorityOwner::retire_revoked`.
    pub fn revoke_current(
        &mut self,
        guard: &mut CurrentAuthorityGuard<'_>,
        owner: &Mutex<AuthorityOwner>,
    ) -> Result<(), E> {
        if !core::ptr::eq(self.owner, owner) {
            return Err(E::Authority(QdnfError::Unauthorized));
        }
        guard
            .revoke_handle(owner, self.handle)
            .map_err(E::Authority)?;
        self.revoked = true;
        Ok(())
    }

    /// Reclaim the revoked slot. Caller must not hold `CurrentAuthorityGuard`.
    pub fn retire_current(&mut self, owner: &Mutex<AuthorityOwner>) -> Result<(), E> {
        if !core::ptr::eq(self.owner, owner) {
            return Err(E::Authority(QdnfError::Unauthorized));
        }
        if !self.revoked {
            return Err(E::Authority(QdnfError::Denied));
        }
        if self.closed {
            return Ok(());
        }
        let mut inner = owner.try_lock().map_err(|e| match e {
            TryLockError::WouldBlock => E::Authority(QdnfError::WouldBlock),
            TryLockError::Poisoned(_) => E::Authority(QdnfError::Unauthorized),
        })?;
        inner.retire_revoked(self.handle).map_err(E::Authority)?;
        self.closed = true;
        Ok(())
    }

    /// Revoke then retire when the mutex is free. Used by tests and Drop.
    pub fn close_now(&mut self, owner: &Mutex<AuthorityOwner>) -> Result<(), E> {
        {
            let mut guard = CurrentAuthorityGuard::try_lock(owner).map_err(E::Authority)?;
            self.revoke_current(&mut guard, owner)?;
        }
        self.retire_current(owner)
    }
}

impl Drop for RemoteDisclosureGrant<'_> {
    fn drop(&mut self) {
        if self.closed {
            return;
        }
        // Without the lock, leave the slot occupied. Do not mutate unlocked.
        match CurrentAuthorityGuard::try_lock(self.owner) {
            Ok(mut guard) => {
                let _ = self.revoke_current(&mut guard, self.owner);
                drop(guard);
                let _ = self.retire_current(self.owner);
            }
            Err(_) => {}
        }
    }
}
