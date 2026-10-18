//! Verified credential, authorised contact and installed keys.
//!
//! Constructors are crate-private. External crates cannot mint Active/Allow.

use crate::crypto::network::types::AEAD_KEY_LEN;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};
use zeroize::Zeroize;

use super::policy::{ContactState, TemporalGrant};

/// Credential whose digest bindings were checked by [`super::AuthorityOwner`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedCredential {
    principal: StrongDigest,
    issuer: StrongDigest,
    instrument: StrongDigest,
    generation: Generation,
}

impl VerifiedCredential {
    /// Creates a default `VerifiedCredential`.
    pub(crate) fn new(
        principal: StrongDigest,
        issuer: StrongDigest,
        instrument: StrongDigest,
        generation: Generation,
    ) -> Result<Self, QdnfError> {
        if principal.is_zero() || issuer.is_zero() || instrument.is_zero() {
            return Err(QdnfError::Unauthorized);
        }
        if generation == Generation::ZERO {
            return Err(QdnfError::Unauthorized);
        }
        Ok(Self {
            principal,
            issuer,
            instrument,
            generation,
        })
    }

    #[inline]
    /// Credential principal digest.
    pub const fn principal(self) -> StrongDigest {
        self.principal
    }

    #[inline]
    /// Credential issuer digest.
    pub const fn issuer(self) -> StrongDigest {
        self.issuer
    }

    #[inline]
    /// Credential instrument digest.
    pub const fn instrument(self) -> StrongDigest {
        self.instrument
    }

    #[inline]
    /// Credential generation.
    pub const fn generation(self) -> Generation {
        self.generation
    }
}

/// Current authorised contact. Inactive states cannot release content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthorisedContact {
    local: StrongDigest,
    remote: StrongDigest,
    state: ContactState,
    generation: Generation,
}

impl AuthorisedContact {
    /// Creates a default `AuthorisedContact`.
    pub(crate) fn new(
        local: StrongDigest,
        remote: StrongDigest,
        state: ContactState,
        generation: Generation,
    ) -> Result<Self, QdnfError> {
        if local.is_zero() || remote.is_zero() || local == remote {
            return Err(QdnfError::Unauthorized);
        }
        if generation == Generation::ZERO {
            return Err(QdnfError::Unauthorized);
        }
        Ok(Self {
            local,
            remote,
            state,
            generation,
        })
    }

    #[inline]
    /// Local digest.
    pub const fn local(self) -> StrongDigest {
        self.local
    }

    #[inline]
    /// Remote digest.
    pub const fn remote(self) -> StrongDigest {
        self.remote
    }

    #[inline]
    /// Contact state.
    pub const fn state(self) -> ContactState {
        self.state
    }

    #[inline]
    /// Contact generation.
    pub const fn generation(self) -> Generation {
        self.generation
    }

    /// `Ok(())` when the contact is active.
    pub fn require_active(&self) -> Result<(), QdnfError> {
        match self.state {
            ContactState::Active => Ok(()),
            ContactState::Blocked => Err(QdnfError::Denied),
            ContactState::Suspended => Err(QdnfError::Denied),
            ContactState::Request | ContactState::Consent => Err(QdnfError::Unauthorized),
        }
    }
}

/// Confirmed directional traffic secrets. Not Copy; zeroized on drop.
pub struct InstalledSessionKeys {
    generation: Generation,
    send: [u8; AEAD_KEY_LEN],
    recv: [u8; AEAD_KEY_LEN],
    confirmed: bool,
}

impl InstalledSessionKeys {
    /// Creates a default `InstalledSessionKeys`.
    pub(crate) fn new(
        generation: Generation,
        send: [u8; AEAD_KEY_LEN],
        recv: [u8; AEAD_KEY_LEN],
        confirmed: bool,
    ) -> Result<Self, QdnfError> {
        if generation == Generation::ZERO || !confirmed {
            return Err(QdnfError::Unauthorized);
        }
        if send == [0u8; AEAD_KEY_LEN] || recv == [0u8; AEAD_KEY_LEN] || send == recv {
            return Err(QdnfError::CryptoFailure);
        }
        Ok(Self {
            generation,
            send,
            recv,
            confirmed,
        })
    }

    #[inline]
    /// Key generation.
    pub const fn generation(&self) -> Generation {
        self.generation
    }

    #[inline]
    /// `true` when the keys are confirmed.
    pub const fn confirmed(&self) -> bool {
        self.confirmed
    }

    #[inline]
    /// Send-direction AEAD key.
    pub fn send_key(&self) -> &[u8; AEAD_KEY_LEN] {
        &self.send
    }

    #[inline]
    /// Receive-direction AEAD key.
    pub fn recv_key(&self) -> &[u8; AEAD_KEY_LEN] {
        &self.recv
    }
}

impl Drop for InstalledSessionKeys {
    fn drop(&mut self) {
        self.send.zeroize();
        self.recv.zeroize();
        self.confirmed = false;
    }
}

/// `true` when the grant matches the contact.
pub fn grant_matches_contact(grant: &TemporalGrant, contact: &AuthorisedContact) -> bool {
    grant.audience_digest == contact.remote()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_principal_is_rejected() {
        assert_eq!(
            VerifiedCredential::new(
                StrongDigest::ZERO,
                StrongDigest([1u8; 48]),
                StrongDigest([2u8; 48]),
                Generation(1)
            ),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn blocked_contact_cannot_be_required_active() {
        let c = AuthorisedContact::new(
            StrongDigest([1u8; 48]),
            StrongDigest([2u8; 48]),
            ContactState::Blocked,
            Generation(1),
        )
        .unwrap();
        assert_eq!(c.require_active(), Err(QdnfError::Denied));
        let grant = TemporalGrant {
            purpose_digest: StrongDigest([3u8; 48]),
            audience_digest: StrongDigest([2u8; 48]),
            authority_generation: 1,
            not_before_unix: 0,
            expires_unix: 10,
            profile: crate::net::qdnf::types::ProfileId::QPR_PQ_1,
        };
        assert!(grant_matches_contact(&grant, &c));
    }
}
