//! **Connection identifier** — the single signed payload that underlies every connection method (email
//! string, magic link, DNS record, token metadata). It extends the connect-invite with the WireGuard
//! peering material + ordered rendezvous hints, and encodes to a compact, copy-pasteable
//! `qcx1_<base64url>` string. Self-certifying: ed25519-signed over its own fields, so a recipient verifies
//! it without any third party. See `docs/plans/social-network-plan.md` §1.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use base64::Engine as _;

pub const CI_VERSION: u8 = 1;
const CI_PREFIX: &str = "qcx1_";
const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// One rendezvous hint — where to try to reach the peer, tried in list order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RendezvousHint {
    /// `"domain" | "edge" | "nym" | "relay" | "libp2p" | "mailbox" | "mdns"`.
    pub kind: String,
    pub value: String,
}

/// The universal, self-certifying connection payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionIdentifier {
    pub version: u8,
    /// The peer's front-door DID (identifier, not identity).
    pub front_door_did: String,
    /// ed25519 identity public key (hex) — verifies `signature_hex`.
    pub identity_pubkey_hex: String,
    /// WireGuard public key (hex) — the peering material.
    pub wireguard_pubkey_hex: String,
    /// Derived overlay address (CGA-like, from the WireGuard pubkey).
    pub overlay_addr: String,
    /// Ordered rendezvous hints (domain, edge, nym, relay, libp2p…).
    pub rendezvous: Vec<RendezvousHint>,
    /// Proposed relationship type (`spc:relationType` id) — the agreement seed.
    pub relation_type: String,
    pub display_name: String,
    pub created_at: u64,
    /// 0 = no expiry.
    pub expires_at: u64,
    /// Single-use nonce (anti-replay).
    pub nonce: String,
    /// ed25519 signature (hex) over [`signing_payload`](Self::signing_payload).
    #[serde(default)]
    pub signature_hex: String,
}

impl ConnectionIdentifier {
    /// The canonical bytes the signature covers (everything except the signature itself).
    pub fn signing_payload(&self) -> Vec<u8> {
        let rv: Vec<String> = self
            .rendezvous
            .iter()
            .map(|r| format!("{}={}", r.kind, r.value))
            .collect();
        format!(
            "qcx1|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.version,
            self.front_door_did,
            self.identity_pubkey_hex,
            self.wireguard_pubkey_hex,
            self.overlay_addr,
            rv.join(","),
            self.relation_type,
            self.display_name,
            self.created_at,
            self.expires_at,
            self.nonce,
        )
        .into_bytes()
    }

    /// Sign the identifier with the ed25519 identity key (sets `identity_pubkey_hex` + `signature_hex`).
    pub fn sign(&mut self, key: &SigningKey) {
        self.identity_pubkey_hex = hex::encode(VerifyingKey::from(key).to_bytes());
        let sig = key.sign(&self.signing_payload());
        self.signature_hex = hex::encode(sig.to_bytes());
    }

    /// Verify the self-certifying signature. Does **not** check expiry (see [`is_expired`](Self::is_expired)).
    pub fn verify(&self) -> Result<(), String> {
        let pk = hex::decode(&self.identity_pubkey_hex).map_err(|e| format!("bad identity key: {e}"))?;
        let pk: [u8; 32] = pk.as_slice().try_into().map_err(|_| "identity key must be 32 bytes".to_string())?;
        let vk = VerifyingKey::from_bytes(&pk).map_err(|e| format!("bad identity key: {e}"))?;
        let sig = hex::decode(&self.signature_hex).map_err(|e| format!("bad signature: {e}"))?;
        let sig: [u8; 64] = sig.as_slice().try_into().map_err(|_| "signature must be 64 bytes".to_string())?;
        vk.verify(&self.signing_payload(), &Signature::from_bytes(&sig))
            .map_err(|_| "signature verification failed".to_string())
    }

    pub fn is_expired(&self, now_unix: u64) -> bool {
        self.expires_at != 0 && now_unix > self.expires_at
    }

    /// Encode to a compact, copy-pasteable `qcx1_<base64url>` string (CBOR under the hood).
    pub fn encode(&self) -> Result<String, String> {
        let mut buf = Vec::new();
        ciborium::into_writer(self, &mut buf).map_err(|e| format!("encode: {e}"))?;
        Ok(format!("{CI_PREFIX}{}", B64.encode(&buf)))
    }

    /// Decode a `qcx1_…` string. Does not verify — call [`verify`](Self::verify) after.
    pub fn decode(s: &str) -> Result<Self, String> {
        let body = s
            .trim()
            .strip_prefix(CI_PREFIX)
            .ok_or("not a qcx1 connection identifier")?;
        let bytes = B64.decode(body).map_err(|e| format!("base64: {e}"))?;
        ciborium::from_reader(&bytes[..]).map_err(|e| format!("decode: {e}"))
    }
}

/// Derive a deterministic ULA IPv6 overlay address (`fd00::/8`) from a WireGuard public key — a
/// self-certifying (CGA-like) address: it *is* a hash of the key, so it cannot be spoofed.
pub fn derive_overlay_addr(wireguard_pubkey: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let h = Sha256::digest(wireguard_pubkey);
    let mut addr = [0u8; 16];
    addr[0] = 0xfd;
    addr[1..16].copy_from_slice(&h[0..15]);
    std::net::Ipv6Addr::from(addr).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ConnectionIdentifier {
        ConnectionIdentifier {
            version: CI_VERSION,
            front_door_did: "did:qualia:frontdoor:alice".into(),
            identity_pubkey_hex: String::new(),
            wireguard_pubkey_hex: "aa".repeat(32),
            overlay_addr: derive_overlay_addr(&[0xaau8; 32]),
            rendezvous: vec![
                RendezvousHint { kind: "domain".into(), value: "alice.example".into() },
                RendezvousHint { kind: "edge".into(), value: "https://edge.alice.example".into() },
            ],
            relation_type: "spc:GuardianshipArrangement".into(),
            display_name: "Alice".into(),
            created_at: 1_700_000_000,
            expires_at: 1_700_604_800,
            nonce: "n-123".into(),
            signature_hex: String::new(),
        }
    }

    #[test]
    fn sign_then_verify_ok() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let mut id = sample();
        id.sign(&key);
        assert!(!id.signature_hex.is_empty());
        assert!(!id.identity_pubkey_hex.is_empty());
        id.verify().expect("valid signature verifies");
    }

    #[test]
    fn tampering_breaks_the_signature() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let mut id = sample();
        id.sign(&key);
        // Flip a signed field after signing.
        id.wireguard_pubkey_hex = "bb".repeat(32);
        assert!(id.verify().is_err(), "tampered payload must fail verification");
    }

    #[test]
    fn encode_decode_roundtrips_and_stays_verified() {
        let key = SigningKey::from_bytes(&[9u8; 32]);
        let mut id = sample();
        id.sign(&key);
        let s = id.encode().expect("encode");
        assert!(s.starts_with("qcx1_"));
        let back = ConnectionIdentifier::decode(&s).expect("decode");
        assert_eq!(back, id, "lossless round-trip");
        back.verify().expect("still verifies after decode");
    }

    #[test]
    fn expiry_is_checked_separately() {
        let id = sample();
        assert!(!id.is_expired(1_700_000_050));
        assert!(id.is_expired(1_700_604_801));
        // 0 = never expires.
        let mut forever = sample();
        forever.expires_at = 0;
        assert!(!forever.is_expired(u64::MAX));
    }

    #[test]
    fn overlay_addr_is_deterministic_ula() {
        let a = derive_overlay_addr(&[1u8; 32]);
        let b = derive_overlay_addr(&[1u8; 32]);
        assert_eq!(a, b, "deterministic");
        assert!(a.starts_with("fd"), "ULA fd00::/8");
        assert_ne!(a, derive_overlay_addr(&[2u8; 32]), "different key → different address");
    }

    #[test]
    fn decode_rejects_non_qcx1() {
        assert!(ConnectionIdentifier::decode("hello").is_err());
        assert!(ConnectionIdentifier::decode("qcx1_!!!not-base64!!!").is_err());
    }
}
