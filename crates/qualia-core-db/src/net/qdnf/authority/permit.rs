//! Local execution permit. Capability for this runtime, not an identity claim.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, OperationId, StrongDigest};

use super::owner::MutationHandle;
use super::policy::TemporalGrant;
use super::verified::{AuthorisedContact, InstalledSessionKeys, VerifiedCredential};

/// Compiled permit. Fields are private; only [`super::AuthorityOwner`] issues it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionPermit {
    operation: OperationId,
    principal: StrongDigest,
    recipient: StrongDigest,
    purpose: StrongDigest,
    instrument: StrongDigest,
    policy_generation: Generation,
    identity_generation: Generation,
    expires_unix: u64,
    resource_ceiling: u64,
}

impl ExecutionPermit {
    /// Issues a permit binding contact+credential+grant+handle+keys.
    pub(crate) fn issue(
        operation: OperationId,
        principal: StrongDigest,
        recipient: StrongDigest,
        purpose: StrongDigest,
        instrument: StrongDigest,
        policy_generation: Generation,
        identity_generation: Generation,
        expires_unix: u64,
        resource_ceiling: u64,
    ) -> Result<Self, QdnfError> {
        if operation.is_zero()
            || principal.is_zero()
            || recipient.is_zero()
            || purpose.is_zero()
            || instrument.is_zero()
        {
            return Err(QdnfError::Unauthorized);
        }
        if principal == recipient {
            return Err(QdnfError::Conflict);
        }
        if policy_generation == Generation::ZERO || identity_generation == Generation::ZERO {
            return Err(QdnfError::Unauthorized);
        }
        if resource_ceiling == 0 {
            return Err(QdnfError::Capacity);
        }
        Ok(Self {
            operation,
            principal,
            recipient,
            purpose,
            instrument,
            policy_generation,
            identity_generation,
            expires_unix,
            resource_ceiling,
        })
    }

    #[inline]
    /// Operation ID.
    pub const fn operation(self) -> OperationId {
        self.operation
    }

    #[inline]
    /// Principal digest.
    pub const fn principal(self) -> StrongDigest {
        self.principal
    }

    #[inline]
    /// Recipient digest.
    pub const fn recipient(self) -> StrongDigest {
        self.recipient
    }

    #[inline]
    /// Purpose digest.
    pub const fn purpose(self) -> StrongDigest {
        self.purpose
    }

    #[inline]
    /// Instrument digest.
    pub const fn instrument(self) -> StrongDigest {
        self.instrument
    }

    #[inline]
    /// Policy generation.
    pub const fn policy_generation(self) -> Generation {
        self.policy_generation
    }

    #[inline]
    /// Identity generation.
    pub const fn identity_generation(self) -> Generation {
        self.identity_generation
    }

    #[inline]
    /// Expiry epoch.
    pub const fn expires_unix(self) -> u64 {
        self.expires_unix
    }

    #[inline]
    /// Resource ceiling.
    pub const fn resource_ceiling(self) -> u64 {
        self.resource_ceiling
    }

    /// `Ok(())` when the permit is current at `now_unix`.
    pub fn current_at(&self, now_unix: u64) -> Result<(), QdnfError> {
        if now_unix >= self.expires_unix {
            Err(QdnfError::Expired)
        } else {
            Ok(())
        }
    }

    /// `Ok` when `contact` matches the permitted contact; `QdnfError` otherwise.
    pub fn matches_contact(&self, contact: &AuthorisedContact) -> Result<(), QdnfError> {
        contact.require_active()?;
        if self.principal != contact.local() || self.recipient != contact.remote() {
            return Err(QdnfError::Unauthorized);
        }
        Ok(())
    }

    /// `Ok` when `cred` matches the permitted credential.
    pub fn matches_credential(&self, cred: &VerifiedCredential) -> Result<(), QdnfError> {
        if self.principal != cred.principal() || self.instrument != cred.instrument() {
            return Err(QdnfError::Unauthorized);
        }
        if self.identity_generation != cred.generation() {
            return Err(QdnfError::StaleGeneration);
        }
        Ok(())
    }

    /// `Ok` when `grant` is within the permit's temporal window at `now_unix`.
    pub fn matches_grant(&self, grant: &TemporalGrant, now_unix: u64) -> Result<(), QdnfError> {
        grant.current_at(now_unix)?;
        if grant.purpose_digest != self.purpose || grant.audience_digest != self.recipient {
            return Err(QdnfError::Unauthorized);
        }
        Ok(())
    }

    /// `Ok` when `handle` matches the permitted mutation handle.
    pub fn matches_handle(&self, handle: &MutationHandle) -> Result<(), QdnfError> {
        handle.matches_permit(self)
    }

    /// `Ok` when `keys` satisfy the permit's session-key requirement.
    pub fn require_keys(&self, keys: &InstalledSessionKeys) -> Result<(), QdnfError> {
        if !keys.confirmed() {
            return Err(QdnfError::Unauthorized);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expired_permit_is_rejected() {
        let p = ExecutionPermit::issue(
            OperationId([1u8; 16]),
            StrongDigest([1u8; 48]),
            StrongDigest([2u8; 48]),
            StrongDigest([3u8; 48]),
            StrongDigest([4u8; 48]),
            Generation(1),
            Generation(1),
            10,
            256,
        )
        .unwrap();
        assert_eq!(p.current_at(10), Err(QdnfError::Expired));
        assert!(p.current_at(9).is_ok());
    }
}
