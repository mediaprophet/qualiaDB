//! Node cryptographic identity — the node's persisted signing + WireGuard keys.
//!
//! A [`NodeIdentity`] holds two 32-byte secrets:
//! - `ed25519_secret` — the ed25519 signing key used to sign connection
//!   identifiers and challenge responses (its public half is the node's
//!   stable identity pubkey).
//! - `wg_secret` — the Curve25519 secret for this node's WireGuard keypair,
//!   from which the overlay address is derived.
//!
//! The identity is persisted as pretty-printed JSON at
//! `app_meta_dir()/node_identity.json` and loaded (or freshly generated) via
//! [`NodeIdentity::load_or_create`]. Generation uses the OS CSPRNG.

use ed25519_dalek::{SigningKey, VerifyingKey};

#[cfg(not(target_arch = "wasm32"))]
use qualia_core_db::p2p::wireguard_userspace::WgKeypair;

/// The node's persisted cryptographic identity.
///
/// Two independent 32-byte secrets: an ed25519 signing key (identity /
/// challenge signing) and a Curve25519 WireGuard secret (transport / overlay
/// addressing). Serialized to JSON for on-disk persistence.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct NodeIdentity {
    /// Raw 32-byte ed25519 signing (secret) key.
    pub ed25519_secret: [u8; 32],
    /// Raw 32-byte Curve25519 WireGuard secret key.
    pub wg_secret: [u8; 32],
}

impl NodeIdentity {
    /// Reconstruct the ed25519 [`SigningKey`] from the stored secret bytes.
    pub fn signing_key(&self) -> SigningKey {
        SigningKey::from_bytes(&self.ed25519_secret)
    }

    /// The node's stable identity public key, lowercase hex (64 chars).
    ///
    /// This is the ed25519 verifying key that peers use to check signatures on
    /// connection identifiers and challenge responses.
    pub fn identity_pubkey_hex(&self) -> String {
        hex::encode(VerifyingKey::from(&self.signing_key()).to_bytes())
    }

    /// This node's WireGuard public key, lowercase hex (64 chars).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn wireguard_pubkey_hex(&self) -> String {
        WgKeypair::from_secret_bytes(self.wg_secret).public_hex()
    }

    /// The node's overlay address, derived from its WireGuard public key.
    ///
    /// This is an `fd…` ULA IPv6 string produced by
    /// [`crate::connection_identifier::derive_overlay_addr`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn overlay_addr(&self) -> String {
        crate::connection_identifier::derive_overlay_addr(
            &WgKeypair::from_secret_bytes(self.wg_secret).public_bytes(),
        )
    }

    /// Load the persisted node identity, or generate and persist a fresh one.
    ///
    /// Reads `app_meta_dir()/node_identity.json`. If the file exists and parses,
    /// it is returned as-is. Otherwise a new identity is generated from the OS
    /// CSPRNG, written to that path (creating the parent directory as needed) as
    /// pretty-printed JSON, and returned. All errors are mapped to `String`.
    ///
    /// Available on all targets: mesh WG helpers remain native-only, but the
    /// Ed25519 apparatus identity is needed for multi-device person/fleet wiring.
    pub fn load_or_create() -> Result<NodeIdentity, String> {
        let path = crate::state::app_meta_dir().join("node_identity.json");

        if path.exists() {
            let bytes = std::fs::read(&path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
            if let Ok(identity) = serde_json::from_slice::<NodeIdentity>(&bytes) {
                return Ok(identity);
            }
            // Fall through to regeneration if the existing file does not parse.
        }

        let mut e = [0u8; 32];
        rand::fill(&mut e);
        let mut w = [0u8; 32];
        rand::fill(&mut w);
        let identity = NodeIdentity {
            ed25519_secret: e,
            wg_secret: w,
        };

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(&identity)
            .map_err(|err| format!("failed to serialize node identity: {err}"))?;
        std::fs::write(&path, json)
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;

        Ok(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, Verifier};

    fn sample() -> NodeIdentity {
        NodeIdentity {
            ed25519_secret: [7u8; 32],
            wg_secret: [9u8; 32],
        }
    }

    #[test]
    fn identity_pubkey_hex_is_64_chars_and_deterministic() {
        let id = sample();
        let a = id.identity_pubkey_hex();
        let b = id.identity_pubkey_hex();
        assert_eq!(a.len(), 64, "identity pubkey hex must be 64 chars");
        assert!(
            a.chars().all(|c| c.is_ascii_hexdigit()),
            "identity pubkey hex must be all hex digits: {a}"
        );
        assert_eq!(a, b, "identity pubkey hex must be deterministic");
    }

    #[test]
    fn signing_key_produces_verifiable_signatures() {
        let id = sample();
        let sk = id.signing_key();
        let msg = b"node challenge response";
        let sig = sk.sign(msg);
        let vk = VerifyingKey::from(&sk);
        assert!(
            vk.verify(msg, &sig).is_ok(),
            "signature must verify under the derived verifying key"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn wireguard_pubkey_hex_is_64_chars() {
        let id = sample();
        let wg = id.wireguard_pubkey_hex();
        assert_eq!(wg.len(), 64, "wireguard pubkey hex must be 64 chars");
        assert!(
            wg.chars().all(|c| c.is_ascii_hexdigit()),
            "wireguard pubkey hex must be all hex digits: {wg}"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn overlay_addr_is_ula() {
        let id = sample();
        let addr = id.overlay_addr();
        assert!(
            addr.starts_with("fd"),
            "overlay address must be an fd… ULA: {addr}"
        );
    }
}
