//! Product OS entropy and ephemeral-key leases.
//!
//! Production `fill_os` uses the operating-system CSPRNG only. A `getrandom`
//! failure returns `EntropyFailure`. This module never imports or falls back
//! to harness `SeededEntropy`.

use zeroize::Zeroize;

use super::ed25519;
use super::errors::CryptoError;
use super::types::{KeyPurpose, ED25519_SIG_LEN, X25519_LEN};
use super::x25519::X25519Secret;

/// Fill `out` from the OS CSPRNG. Never substitutes deterministic test RNG.
pub fn fill_os(out: &mut [u8]) -> Result<(), CryptoError> {
    match getrandom::fill(out) {
        Ok(()) => Ok(()),
        Err(_) => {
            out.zeroize();
            Err(CryptoError::EntropyFailure)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LeaseState {
    Live,
    Cancelled,
    Destroyed,
}

/// Ephemeral X25519 + Ed25519 material: generate, use, rotate, cancel, destroy.
pub struct EphemeralKeyLease {
    purpose: KeyPurpose,
    x25519: [u8; X25519_LEN],
    ed25519: [u8; 32],
    state: LeaseState,
}

impl EphemeralKeyLease {
    /// Generate a live lease from OS entropy. Failure is `EntropyFailure`.
    pub fn generate(purpose: KeyPurpose) -> Result<Self, CryptoError> {
        let mut x25519 = [0u8; X25519_LEN];
        let mut ed25519 = [0u8; 32];
        if let Err(e) = fill_os(&mut x25519) {
            x25519.zeroize();
            return Err(e);
        }
        if let Err(e) = fill_os(&mut ed25519) {
            x25519.zeroize();
            ed25519.zeroize();
            return Err(e);
        }
        Ok(Self {
            purpose,
            x25519,
            ed25519,
            state: LeaseState::Live,
        })
    }

    pub fn purpose(&self) -> Result<KeyPurpose, CryptoError> {
        self.require_live()?;
        Ok(self.purpose)
    }

    pub fn x25519_public(&self) -> Result<[u8; X25519_LEN], CryptoError> {
        self.require_live()?;
        Ok(X25519Secret::from_bytes(self.x25519).public())
    }

    pub fn ed25519_public(&self) -> Result<[u8; 32], CryptoError> {
        self.require_live()?;
        Ok(ed25519::public_from_seed(&self.ed25519))
    }

    /// Use: X25519 Diffie-Hellman. Rejects all-zero shared secrets via the adapter.
    pub fn diffie_hellman(&self, peer: &[u8; X25519_LEN]) -> Result<[u8; X25519_LEN], CryptoError> {
        self.require_live()?;
        X25519Secret::from_bytes(self.x25519).diffie_hellman(peer)
    }

    /// Use: Ed25519 with the network length-delimited context.
    pub fn sign_ed25519(
        &self,
        message: &[u8],
        context: &[u8],
        sig_out: &mut [u8; ED25519_SIG_LEN],
    ) -> Result<(), CryptoError> {
        self.require_live()?;
        ed25519::sign_with_context(&self.ed25519, message, context, sig_out)
    }

    /// Replace material with a fresh OS draw; previous bytes are destroyed.
    pub fn rotate(&mut self) -> Result<(), CryptoError> {
        self.require_live()?;
        let mut new_x = [0u8; X25519_LEN];
        let mut new_ed = [0u8; 32];
        if let Err(e) = fill_os(&mut new_x) {
            new_x.zeroize();
            return Err(e);
        }
        if let Err(e) = fill_os(&mut new_ed) {
            new_x.zeroize();
            new_ed.zeroize();
            return Err(e);
        }
        self.x25519.zeroize();
        self.ed25519.zeroize();
        self.x25519 = new_x;
        self.ed25519 = new_ed;
        Ok(())
    }

    pub fn cancel(&mut self) {
        if matches!(self.state, LeaseState::Live) {
            self.state = LeaseState::Cancelled;
            self.zeroize_material();
        }
    }

    pub fn destroy(&mut self) {
        self.state = LeaseState::Destroyed;
        self.zeroize_material();
    }

    fn require_live(&self) -> Result<(), CryptoError> {
        match self.state {
            LeaseState::Live => Ok(()),
            LeaseState::Cancelled => Err(CryptoError::Cancelled),
            LeaseState::Destroyed => Err(CryptoError::Revoked),
        }
    }

    fn zeroize_material(&mut self) {
        self.x25519.zeroize();
        self.ed25519.zeroize();
    }
}

impl Drop for EphemeralKeyLease {
    fn drop(&mut self) {
        self.zeroize_material();
    }
}

impl core::fmt::Debug for EphemeralKeyLease {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EphemeralKeyLease")
            .field("purpose", &self.purpose)
            .field("state", &self.state)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_os_succeeds_or_is_entropy_failure() {
        let mut buf = [0u8; 32];
        match fill_os(&mut buf) {
            Ok(()) => {}
            Err(e) => {
                assert_eq!(e, CryptoError::EntropyFailure);
                assert_eq!(buf, [0u8; 32]);
            }
        }
    }

    #[test]
    fn os_failure_never_selects_seeded_entropy() {
        fn product_map(result: Result<(), ()>, out: &mut [u8]) -> Result<(), CryptoError> {
            match result {
                Ok(()) => fill_os(out),
                Err(()) => {
                    out.zeroize();
                    Err(CryptoError::EntropyFailure)
                }
            }
        }
        let mut buf = [0xAAu8; 8];
        assert_eq!(
            product_map(Err(()), &mut buf).unwrap_err(),
            CryptoError::EntropyFailure
        );
        assert_eq!(buf, [0u8; 8]);
    }

    #[test]
    fn destroy_rejects_later_dh_and_sign() {
        let mut lease = EphemeralKeyLease::generate(KeyPurpose::QLinkEphemeral).unwrap();
        let peer = X25519Secret::from_bytes([2u8; 32]);
        let peer_pk = peer.public();
        lease.diffie_hellman(&peer_pk).unwrap();
        let mut sig = [0u8; ED25519_SIG_LEN];
        lease.sign_ed25519(b"msg", b"ctx", &mut sig).unwrap();
        lease.destroy();
        assert_eq!(
            lease.diffie_hellman(&peer_pk).unwrap_err(),
            CryptoError::Revoked
        );
        assert_eq!(
            lease.sign_ed25519(b"msg", b"ctx", &mut sig).unwrap_err(),
            CryptoError::Revoked
        );
        assert_eq!(lease.x25519_public().unwrap_err(), CryptoError::Revoked);
    }

    #[test]
    fn cancel_rejects_use_with_cancelled() {
        let mut lease = EphemeralKeyLease::generate(KeyPurpose::QSessionEphemeral).unwrap();
        let peer_pk = X25519Secret::from_bytes([3u8; 32]).public();
        lease.cancel();
        assert_eq!(
            lease.diffie_hellman(&peer_pk).unwrap_err(),
            CryptoError::Cancelled
        );
        let mut sig = [0u8; ED25519_SIG_LEN];
        assert_eq!(
            lease.sign_ed25519(b"m", b"c", &mut sig).unwrap_err(),
            CryptoError::Cancelled
        );
    }

    #[test]
    fn rotate_replaces_material_and_destroys_old() {
        let mut lease = EphemeralKeyLease::generate(KeyPurpose::QLinkEphemeral).unwrap();
        let pk1 = lease.x25519_public().unwrap();
        let ed1 = lease.ed25519_public().unwrap();
        lease.rotate().unwrap();
        let pk2 = lease.x25519_public().unwrap();
        let ed2 = lease.ed25519_public().unwrap();
        assert_ne!(pk1, pk2);
        assert_ne!(ed1, ed2);
        let peer_pk = X25519Secret::from_bytes([4u8; 32]).public();
        lease.diffie_hellman(&peer_pk).unwrap();
    }

    #[test]
    fn debug_redacts_secret_bytes() {
        let lease = EphemeralKeyLease::generate(KeyPurpose::QLinkEphemeral).unwrap();
        let rendered = format!("{:?}", lease);
        assert!(rendered.contains("EphemeralKeyLease"));
        assert!(rendered.contains("purpose"));
        assert!(!rendered.contains("x25519"));
        assert!(!rendered.contains("ed25519"));
    }
}
