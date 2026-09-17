//! Invitation import for the first two-host Internet test.
//!
//! Production encoding is length-delimited (`QINV1`). `qcx1_` remains a
//! trusted-private bootstrap import only.

pub use qualia_core_db::net::peer::connectivity::{
    phrase_must_not_mint_wg_keys, sign_invitation, verify_invitation, Invitation,
};

use crate::connection_identifier::ConnectionIdentifier;

/// `qcx1_` is accepted only as a private bootstrap hint, not a public DHT record.
pub fn qcx1_is_private_bootstrap_only() -> bool {
    true
}

/// Decode a pasted `qcx1_` string without treating it as a production invitation.
pub fn import_qcx1_private(s: &str) -> Result<ConnectionIdentifier, String> {
    ConnectionIdentifier::decode(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    #[test]
    fn qinv1_and_qcx1_roles() {
        assert!(phrase_must_not_mint_wg_keys());
        assert!(qcx1_is_private_bootstrap_only());
        let inv = Invitation {
            peer: [1u8; 32],
            proto: 1,
            relay: 0,
            expiry_unix: 2_000_000_000,
            nonce: [2u8; 16],
            policy: 0,
        };
        let mut body = [0u8; 256];
        let n = inv.encode(&mut body).unwrap();
        let sk = SigningKey::from_bytes(&[4u8; 32]);
        let mut signed = [0u8; 320];
        let sn = sign_invitation(&body[..n], &sk, &mut signed).unwrap();
        let back = verify_invitation(&signed[..sn], &sk.verifying_key(), 1_700_000_000).unwrap();
        assert_eq!(back.peer, inv.peer);
    }
}
