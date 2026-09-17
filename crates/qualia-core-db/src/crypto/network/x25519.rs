//! X25519 adapter. Rejects the all-zero shared secret.

use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroize;

use super::errors::CryptoError;

pub struct X25519Secret([u8; 32]);

impl X25519Secret {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    fn secret(&self) -> StaticSecret {
        StaticSecret::from(self.0)
    }

    pub fn public(&self) -> [u8; 32] {
        PublicKey::from(&self.secret()).to_bytes()
    }

    pub fn diffie_hellman(&self, peer: &[u8; 32]) -> Result<[u8; 32], CryptoError> {
        let peer = PublicKey::from(*peer);
        let shared = self.secret().diffie_hellman(&peer);
        let bytes = shared.to_bytes();
        if bytes.iter().all(|&b| b == 0) {
            return Err(CryptoError::CryptoFailure);
        }
        Ok(bytes)
    }
}

impl Drop for X25519Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

pub fn public_from_secret(secret: &[u8; 32]) -> [u8; 32] {
    X25519Secret::from_bytes(*secret).public()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agreement_is_symmetric() {
        let a = X25519Secret::from_bytes([1u8; 32]);
        let b = X25519Secret::from_bytes([2u8; 32]);
        let sa = a.diffie_hellman(&b.public()).unwrap();
        let sb = b.diffie_hellman(&a.public()).unwrap();
        assert_eq!(sa, sb);
    }
}
