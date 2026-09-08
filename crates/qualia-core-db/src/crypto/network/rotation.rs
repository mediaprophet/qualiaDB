//! Paired controller-key rotation and authorized recovery.
//!
//! Recovered key possession is not authority to use that material as a network
//! signing instrument. Authority is a matching `ProviderGrant` generation on
//! the admitted epoch. Signing with a revoked old epoch returns `Revoked`.

use super::ed25519;
use super::errors::CryptoError;
use super::key_provider::ProviderGrant;
use super::secret_lease::SecretLease;
use super::types::{KeyEpoch, KeyPurpose, ED25519_SIG_LEN};

/// Overlapping controller epochs. `revoked_old` forbids signatures under `old`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RotationState {
    pub old: KeyEpoch,
    pub new: KeyEpoch,
    pub revoked_old: bool,
}

impl RotationState {
    /// Pair old/new epochs for one controller and purpose. `new.epoch` must advance.
    pub fn paired(old: KeyEpoch, new: KeyEpoch) -> Result<Self, CryptoError> {
        if old.controller != new.controller {
            return Err(CryptoError::Unauthorized);
        }
        if old.purpose != new.purpose {
            return Err(CryptoError::Unauthorized);
        }
        if new.epoch <= old.epoch {
            return Err(CryptoError::StaleGeneration);
        }
        Ok(Self {
            old,
            new,
            revoked_old: false,
        })
    }

    pub fn revoke_old(&mut self) {
        self.revoked_old = true;
    }

    /// Sign under `requested` only when that epoch is admitted and the grant matches.
    pub fn sign_ed25519(
        &self,
        grant: &ProviderGrant,
        requested: KeyEpoch,
        seed: &[u8; 32],
        message: &[u8],
        context: &[u8],
        now_unix: u64,
        controller: u64,
        authority_generation: u64,
        sig_out: &mut [u8; ED25519_SIG_LEN],
    ) -> Result<(), CryptoError> {
        if requested == self.old && self.revoked_old {
            return Err(CryptoError::Revoked);
        }
        if requested != self.old && requested != self.new {
            return Err(CryptoError::Unauthorized);
        }
        grant.authorize(
            requested.purpose,
            controller,
            now_unix,
            authority_generation,
        )?;
        if grant.epoch != requested {
            return Err(CryptoError::Unauthorized);
        }
        ed25519::sign_with_context(seed, message, context, sig_out)
    }

    /// Recovery material may confirm a rotation onto `new`; it cannot be the grant.
    pub fn admit_recovered_rotation(
        &self,
        recovered: &RecoveredPossession,
        grant: &ProviderGrant,
        now_unix: u64,
        authority_generation: u64,
    ) -> Result<(), CryptoError> {
        recovered.confirms_controller(self.new.controller)?;
        if grant.epoch != self.new {
            return Err(CryptoError::Unauthorized);
        }
        grant.authorize(
            self.new.purpose,
            self.new.controller,
            now_unix,
            authority_generation,
        )
    }
}

/// Held recovered secret bytes. Possession does not confer network signing authority.
pub struct RecoveredPossession {
    controller: u64,
    material: SecretLease,
}

impl RecoveredPossession {
    pub fn from_bytes(controller: u64, secret: &[u8]) -> Result<Self, CryptoError> {
        Ok(Self {
            controller,
            material: SecretLease::new(KeyPurpose::ControllerSign, secret)?,
        })
    }

    pub fn confirms_controller(&self, controller: u64) -> Result<(), CryptoError> {
        let _ = self.material.purpose()?;
        if self.controller != controller {
            return Err(CryptoError::Unauthorized);
        }
        Ok(())
    }

    /// Recovered bytes are never a network signing oracle.
    pub fn sign_ed25519(
        &self,
        _message: &[u8],
        _context: &[u8],
        _sig_out: &mut [u8; ED25519_SIG_LEN],
    ) -> Result<(), CryptoError> {
        let _ = self.material.purpose()?;
        Err(CryptoError::Unauthorized)
    }

    pub fn destroy(&mut self) {
        self.material.destroy();
    }
}

impl core::fmt::Debug for RecoveredPossession {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RecoveredPossession")
            .field("controller", &self.controller)
            .field("material", &self.material)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::ed25519::public_from_seed;
    use crate::crypto::network::ed25519::verify_with_context;

    fn epochs() -> (KeyEpoch, KeyEpoch) {
        let old = KeyEpoch {
            purpose: KeyPurpose::ControllerSign,
            epoch: 1,
            controller: 9,
        };
        let new = KeyEpoch {
            purpose: KeyPurpose::ControllerSign,
            epoch: 2,
            controller: 9,
        };
        (old, new)
    }

    fn grant_for(epoch: KeyEpoch, generation: u64) -> ProviderGrant {
        ProviderGrant {
            epoch,
            cell_epoch: 1,
            authority_generation: generation,
            deadline_unix: 100,
            revoked: false,
        }
    }

    #[test]
    fn paired_epochs_must_share_controller_and_advance() {
        let (old, mut new) = epochs();
        new.controller = 8;
        assert_eq!(
            RotationState::paired(old, new).unwrap_err(),
            CryptoError::Unauthorized
        );
        let (old, mut new) = epochs();
        new.epoch = 1;
        assert_eq!(
            RotationState::paired(old, new).unwrap_err(),
            CryptoError::StaleGeneration
        );
    }

    #[test]
    fn revoked_old_epoch_sign_fails_revoked() {
        let (old, new) = epochs();
        let mut state = RotationState::paired(old, new).unwrap();
        state.revoke_old();
        let grant = grant_for(old, 1);
        let seed = [3u8; 32];
        let mut sig = [0u8; ED25519_SIG_LEN];
        assert_eq!(
            state.sign_ed25519(&grant, old, &seed, b"m", b"c", 10, 9, 1, &mut sig),
            Err(CryptoError::Revoked)
        );
        assert_eq!(sig, [0u8; ED25519_SIG_LEN]);
    }

    #[test]
    fn new_epoch_sign_requires_matching_authority_generation() {
        let (old, new) = epochs();
        let state = RotationState::paired(old, new).unwrap();
        let grant = grant_for(new, 4);
        let seed = [3u8; 32];
        let mut sig = [0u8; ED25519_SIG_LEN];
        assert_eq!(
            state.sign_ed25519(&grant, new, &seed, b"m", b"c", 10, 9, 3, &mut sig),
            Err(CryptoError::StaleGeneration)
        );
        state
            .sign_ed25519(&grant, new, &seed, b"m", b"c", 10, 9, 4, &mut sig)
            .unwrap();
        let pk = public_from_seed(&seed);
        verify_with_context(&pk, b"m", b"c", &sig).unwrap();
    }

    #[test]
    fn recovered_possession_is_not_signing_authority() {
        let seed = [3u8; 32];
        let recovered = RecoveredPossession::from_bytes(9, &seed).unwrap();
        let mut sig = [0u8; ED25519_SIG_LEN];
        assert_eq!(
            recovered.sign_ed25519(b"m", b"c", &mut sig),
            Err(CryptoError::Unauthorized)
        );
        assert_eq!(sig, [0u8; ED25519_SIG_LEN]);
        let (old, new) = epochs();
        let state = RotationState::paired(old, new).unwrap();
        let grant_old = grant_for(old, 1);
        assert_eq!(
            state.admit_recovered_rotation(&recovered, &grant_old, 10, 1),
            Err(CryptoError::Unauthorized)
        );
        let grant_new = grant_for(new, 1);
        state
            .admit_recovered_rotation(&recovered, &grant_new, 10, 1)
            .unwrap();
        assert_eq!(
            state.admit_recovered_rotation(&recovered, &grant_new, 10, 9),
            Err(CryptoError::StaleGeneration)
        );
    }

    #[test]
    fn recovered_destroy_rejects_later_confirm() {
        let mut recovered = RecoveredPossession::from_bytes(9, &[3u8; 32]).unwrap();
        recovered.destroy();
        assert_eq!(
            recovered.confirms_controller(9).unwrap_err(),
            CryptoError::Revoked
        );
        let mut sig = [0u8; ED25519_SIG_LEN];
        assert_eq!(
            recovered.sign_ed25519(b"m", b"c", &mut sig),
            Err(CryptoError::Revoked)
        );
    }

    #[test]
    fn grant_epoch_must_match_requested_slot() {
        let (old, new) = epochs();
        let state = RotationState::paired(old, new).unwrap();
        let grant_new = grant_for(new, 1);
        let seed = [3u8; 32];
        let mut sig = [0u8; ED25519_SIG_LEN];
        assert_eq!(
            state.sign_ed25519(&grant_new, old, &seed, b"m", b"c", 10, 9, 1, &mut sig),
            Err(CryptoError::Unauthorized)
        );
    }

    #[test]
    fn debug_redacts_recovered_secret() {
        let recovered = RecoveredPossession::from_bytes(9, &[3u8; 32]).unwrap();
        let rendered = format!("{:?}", recovered);
        assert!(rendered.contains("RecoveredPossession"));
        assert!(rendered.contains("SecretLease"));
        assert!(!rendered.contains("bytes"));
    }
}
