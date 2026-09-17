//! Explicit zeroizing secret leases. Secrets are not Copy.

use super::errors::CryptoError;
use super::types::KeyPurpose;
use zeroize::Zeroize;

pub const SECRET_LEASE_BYTES: usize = 64;

pub struct SecretLease {
    bytes: [u8; SECRET_LEASE_BYTES],
    len: u8,
    purpose: KeyPurpose,
    live: bool,
}

impl SecretLease {
    pub fn new(purpose: KeyPurpose, secret: &[u8]) -> Result<Self, CryptoError> {
        if secret.len() > SECRET_LEASE_BYTES {
            return Err(CryptoError::Capacity);
        }
        let mut bytes = [0u8; SECRET_LEASE_BYTES];
        bytes[..secret.len()].copy_from_slice(secret);
        Ok(Self {
            bytes,
            len: secret.len() as u8,
            purpose,
            live: true,
        })
    }

    pub fn purpose(&self) -> Result<KeyPurpose, CryptoError> {
        if !self.live {
            return Err(CryptoError::Revoked);
        }
        Ok(self.purpose)
    }

    pub fn expose(&self, expected: KeyPurpose) -> Result<&[u8], CryptoError> {
        if !self.live {
            return Err(CryptoError::Revoked);
        }
        if self.purpose != expected {
            return Err(CryptoError::Unauthorized);
        }
        Ok(&self.bytes[..self.len as usize])
    }

    pub fn destroy(&mut self) {
        self.live = false;
        self.bytes.zeroize();
        self.len = 0;
    }
}

impl Drop for SecretLease {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

impl core::fmt::Debug for SecretLease {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SecretLease")
            .field("purpose", &self.purpose)
            .field("live", &self.live)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_purpose_is_unauthorized() {
        let lease = SecretLease::new(KeyPurpose::QLinkAead, &[1u8; 32]).unwrap();
        assert_eq!(
            lease.expose(KeyPurpose::QSessionAead).unwrap_err(),
            CryptoError::Unauthorized
        );
    }

    #[test]
    fn destroy_rejects_later_use() {
        let mut lease = SecretLease::new(KeyPurpose::QLinkAead, &[1u8; 32]).unwrap();
        lease.destroy();
        assert_eq!(
            lease.expose(KeyPurpose::QLinkAead).unwrap_err(),
            CryptoError::Revoked
        );
    }
}
