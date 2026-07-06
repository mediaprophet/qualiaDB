//! Mutual challenge-response handshake proving peer identity.
//!
//! When Alice connects to Bob, she needs to know the peer answering is *actually*
//! Bob and not an interposer. This module implements a minimal challenge-response:
//! Alice sends a random [`Challenge`] naming a nonce; the responder signs a
//! canonical message binding that nonce to its own DID with its ed25519 key and
//! returns a [`ChallengeResponse`]. Alice verifies the signature against the
//! embedded public key, then confirms the responder DID is the one she expected
//! out-of-band (Bob's known DID) via [`responder_is`].
//!
//! Signature scheme: ed25519 (`ed25519-dalek` v2). The signed message is
//! domain-separated with the `qhs1` tag (Qualia HandShake v1) so a signature made
//! here cannot be replayed as some other ed25519 signature over unrelated bytes.
//!
//! Pure: no filesystem, no network. All I/O of keys/signatures is via hex strings
//! so the structs serialize cleanly with serde.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

/// A connection challenge issued by the initiator (e.g. Alice).
///
/// `nonce` is a freshly generated, single-use value the responder must sign.
/// `from_did` records who issued the challenge (informational; the security
/// property rides on the nonce + responder signature).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Challenge {
    pub nonce: String,
    pub from_did: String,
}

/// The responder's answer, proving control of the key bound to `responder_did`.
///
/// `responder_pubkey_hex` is the 32-byte ed25519 public key (hex); `signature_hex`
/// is the 64-byte ed25519 signature (hex) over [`signed_message`]`(nonce, responder_did)`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChallengeResponse {
    pub nonce: String,
    pub responder_did: String,
    pub responder_pubkey_hex: String,
    pub signature_hex: String,
}

/// Construct a challenge from the initiator's DID and a caller-supplied nonce.
///
/// The nonce must be fresh and single-use; nonce generation is the caller's
/// responsibility so this function stays pure and deterministic.
pub fn make_challenge(from_did: &str, nonce: &str) -> Challenge {
    Challenge {
        nonce: nonce.to_string(),
        from_did: from_did.to_string(),
    }
}

/// Canonical, domain-separated message that the responder signs.
///
/// Binding both the nonce and the responder DID means a valid signature proves
/// "the holder of this key asserts *this DID* in response to *this nonce*" — a
/// signature cannot be lifted to a different nonce or re-attributed to a
/// different DID.
pub fn signed_message(nonce: &str, responder_did: &str) -> Vec<u8> {
    format!("qhs1|{nonce}|{responder_did}").into_bytes()
}

/// Sign a challenge, producing a [`ChallengeResponse`] the initiator can verify.
///
/// Signs [`signed_message`]`(challenge.nonce, responder_did)` with `key` and
/// embeds the corresponding public key so the verifier needs nothing but the
/// response and the expected DID.
pub fn answer_challenge(
    challenge: &Challenge,
    responder_did: &str,
    key: &SigningKey,
) -> ChallengeResponse {
    let msg = signed_message(&challenge.nonce, responder_did);
    let sig: Signature = key.sign(&msg);
    let vk = VerifyingKey::from(key);
    ChallengeResponse {
        nonce: challenge.nonce.clone(),
        responder_did: responder_did.to_string(),
        responder_pubkey_hex: hex::encode(vk.to_bytes()),
        signature_hex: hex::encode(sig.to_bytes()),
    }
}

/// Verify that a response answers the given challenge and carries a valid signature.
///
/// Checks nonce agreement, decodes the embedded public key (32 bytes) and
/// signature (64 bytes), then verifies the signature over the canonical message.
/// Returns `Ok(())` only when the response is cryptographically sound. A caller
/// must still confirm *which* DID responded via [`responder_is`].
pub fn verify_response(challenge: &Challenge, resp: &ChallengeResponse) -> Result<(), String> {
    if resp.nonce != challenge.nonce {
        return Err(format!(
            "nonce mismatch: expected {:?}, got {:?}",
            challenge.nonce, resp.nonce
        ));
    }

    let pk_bytes = hex::decode(&resp.responder_pubkey_hex)
        .map_err(|e| format!("invalid pubkey hex: {e}"))?;
    let pk_arr: [u8; 32] = pk_bytes
        .as_slice()
        .try_into()
        .map_err(|_| format!("pubkey must be 32 bytes, got {}", pk_bytes.len()))?;
    let vk = VerifyingKey::from_bytes(&pk_arr).map_err(|e| format!("invalid pubkey: {e}"))?;

    let sig_bytes =
        hex::decode(&resp.signature_hex).map_err(|e| format!("invalid signature hex: {e}"))?;
    let sig_arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| format!("signature must be 64 bytes, got {}", sig_bytes.len()))?;
    let sig = Signature::from_bytes(&sig_arr);

    let msg = signed_message(&resp.nonce, &resp.responder_did);
    vk.verify(&msg, &sig)
        .map_err(|e| format!("signature verification failed: {e}"))
}

/// The "actually Bob" check: does the response come from the DID we expected?
///
/// [`verify_response`] proves the signature is valid for the DID the responder
/// *claims*; this confirms that claimed DID matches the one the initiator knows
/// out-of-band. Both together establish "when Alice connects to Bob, it's Bob".
pub fn responder_is(resp: &ChallengeResponse, expected_did: &str) -> bool {
    resp.responder_did == expected_did
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    const BOB_DID: &str = "did:qhs:bob";
    const ALICE_DID: &str = "did:qhs:alice";

    #[test]
    fn valid_handshake_verifies() {
        let key = test_key();
        let challenge = make_challenge(ALICE_DID, "nonce-abc-123");
        let resp = answer_challenge(&challenge, BOB_DID, &key);
        assert_eq!(verify_response(&challenge, &resp), Ok(()));
    }

    #[test]
    fn wrong_nonce_errs() {
        let key = test_key();
        let challenge = make_challenge(ALICE_DID, "nonce-abc-123");
        let mut resp = answer_challenge(&challenge, BOB_DID, &key);
        resp.nonce = "different-nonce".to_string();
        assert!(verify_response(&challenge, &resp).is_err());
    }

    #[test]
    fn tampered_responder_did_errs() {
        let key = test_key();
        let challenge = make_challenge(ALICE_DID, "nonce-abc-123");
        let mut resp = answer_challenge(&challenge, BOB_DID, &key);
        // Tamper with the DID *after* signing: the signature covered the original
        // DID, so verification over the new message must fail.
        resp.responder_did = "did:qhs:mallory".to_string();
        assert!(verify_response(&challenge, &resp).is_err());
    }

    #[test]
    fn responder_is_matches_real_did_only() {
        let key = test_key();
        let challenge = make_challenge(ALICE_DID, "nonce-abc-123");
        let resp = answer_challenge(&challenge, BOB_DID, &key);
        assert!(responder_is(&resp, BOB_DID));
        assert!(!responder_is(&resp, "did:qhs:mallory"));
    }
}
