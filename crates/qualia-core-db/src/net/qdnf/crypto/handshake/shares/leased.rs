//! Lease-based overloads of the existing hybrid share owner; no secret export.
use super::*;
use crate::crypto::network::{
    entropy::EphemeralKeyLease, secret_lease::SecretLease, types::KeyPurpose,
};
use zeroize::Zeroizing;
fn pack(kem: &[u8; 32], dh: &[u8; 32]) -> Result<SecretLease, CryptoError> {
    reject_zero(kem)?;
    reject_zero(dh)?;
    let mut bytes = Zeroizing::new([0u8; 64]);
    bytes[..32].copy_from_slice(kem);
    bytes[32..].copy_from_slice(dh);
    SecretLease::new(KeyPurpose::QSessionEphemeral, &bytes[..])
}
pub fn initiator_share_leased(
    lease: &EphemeralKeyLease,
    pk: &[u8; ML_KEM_768_PK_LEN],
) -> Result<InitiatorShare, CryptoError> {
    if lease.purpose()? != KeyPurpose::QSessionEphemeral {
        return Err(CryptoError::Unauthorized);
    }
    let x25519_pk = lease.x25519_public()?;
    reject_all_zero_dh(&x25519_pk)?;
    Ok(InitiatorShare {
        ml_kem_pk: *pk,
        x25519_pk,
    })
}
pub fn responder_complete_leased(
    peer: &InitiatorShare,
    lease: &EphemeralKeyLease,
) -> Result<(ResponderShare, SecretLease), CryptoError> {
    if lease.purpose()? != KeyPurpose::QSessionEphemeral {
        return Err(CryptoError::Unauthorized);
    }
    let local_pk = lease.x25519_public()?;
    reject_complete_failures(&local_pk, &peer.x25519_pk, ML_KEM_768_CT_LEN)?;
    let (kem, ct) = encapsulate(&peer.ml_kem_pk)?;
    let kem = Zeroizing::new(kem);
    let dh = Zeroizing::new(lease.diffie_hellman(&peer.x25519_pk)?);
    Ok((
        ResponderShare {
            ml_kem_ct: ct,
            x25519_pk: local_pk,
        },
        pack(&kem, &dh)?,
    ))
}
pub fn initiator_complete_leased(
    kem: &MlKem768Secret,
    lease: &EphemeralKeyLease,
    peer: &ResponderShare,
) -> Result<SecretLease, CryptoError> {
    if lease.purpose()? != KeyPurpose::QSessionEphemeral {
        return Err(CryptoError::Unauthorized);
    }
    reject_complete_failures(
        &lease.x25519_public()?,
        &peer.x25519_pk,
        peer.ml_kem_ct.len(),
    )?;
    let kem = Zeroizing::new(kem.decapsulate(&peer.ml_kem_ct)?);
    let dh = Zeroizing::new(lease.diffie_hellman(&peer.x25519_pk)?);
    pack(&kem, &dh)
}
