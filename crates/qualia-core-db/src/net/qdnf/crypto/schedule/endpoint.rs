//! Endpoint-v1 domain separation over the existing hybrid schedule/HKDF.
use super::derive_handshake_keys;
use crate::crypto::network::{
    errors::CryptoError, kdf::hkdf_sha384, secret_lease::SecretLease, transcript::Transcript,
    types::KeyPurpose,
};
use crate::net::qdnf::types::StrongDigest;
use zeroize::{Zeroize, Zeroizing};
pub struct EndpointSecrets {
    base: SecretLease,
    finished: SecretLease,
}
impl EndpointSecrets {
    pub fn derive(shared: &SecretLease, t0: &Transcript) -> Result<Self, CryptoError> {
        let shared = shared.expose(KeyPurpose::QSessionEphemeral)?;
        if shared.len() != 64 {
            return Err(CryptoError::Malformed);
        }
        let mut keys = derive_handshake_keys(
            shared[..32].try_into().unwrap(),
            shared[32..].try_into().unwrap(),
            t0,
        )?;
        let mut base = Zeroizing::new([0u8; 64]);
        base[..32].copy_from_slice(&keys.initiator_to_responder);
        base[32..].copy_from_slice(&keys.responder_to_initiator);
        keys.initiator_to_responder.zeroize();
        keys.responder_to_initiator.zeroize();
        let mut finished = Zeroizing::new([0u8; 64]);
        hkdf_sha384(
            Some(&t0.digest().0),
            &base[..32],
            b"qpr-pq-1/endpoint-v1/finished/i2r",
            &mut finished[..32],
        )?;
        hkdf_sha384(
            Some(&t0.digest().0),
            &base[32..],
            b"qpr-pq-1/endpoint-v1/finished/r2i",
            &mut finished[32..],
        )?;
        Ok(Self {
            base: SecretLease::new(KeyPurpose::QSessionEphemeral, &base[..])?,
            finished: SecretLease::new(KeyPurpose::QSessionEphemeral, &finished[..])?,
        })
    }
    pub fn finished_key(&self, initiator: bool) -> Result<&[u8; 32], CryptoError> {
        let bytes = self.finished.expose(KeyPurpose::QSessionEphemeral)?;
        Ok(if initiator {
            &bytes[..32]
        } else {
            &bytes[32..]
        }
        .try_into()
        .map_err(|_| CryptoError::Malformed)?)
    }
    pub fn traffic(&self, t2: StrongDigest) -> Result<SecretLease, CryptoError> {
        let base = self.base.expose(KeyPurpose::QSessionEphemeral)?;
        let mut traffic = Zeroizing::new([0u8; 64]);
        hkdf_sha384(
            Some(&t2.0),
            &base[..32],
            b"qpr-pq-1/endpoint-v1/traffic/i2r",
            &mut traffic[..32],
        )?;
        hkdf_sha384(
            Some(&t2.0),
            &base[32..],
            b"qpr-pq-1/endpoint-v1/traffic/r2i",
            &mut traffic[32..],
        )?;
        SecretLease::new(KeyPurpose::QSessionAead, &traffic[..])
    }
}
