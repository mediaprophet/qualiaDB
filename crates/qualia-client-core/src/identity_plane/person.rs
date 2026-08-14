//! Natural-person principal — not a machine and not an OS login.
//!
//! At rest the secret material is sealed with the install vault key
//! (XChaCha20-Poly1305 via `wrap_key`). Cleartext files from older builds are
//! accepted once and rewritten sealed.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use qualia_core_db::crypto::sanctuary_audit::{unwrap_key, wrap_key};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const PERSON_AAD: &[u8] = b"qualia:person_identity:v1";
const SEALED_FORMAT: &str = "qualia.person.sealed.v1";

/// On-disk person principal (secret material).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonPrincipal {
    /// Stable DID: `did:q42:person:{verifying_key_hex}`.
    pub person_id: String,
    /// Raw Ed25519 signing secret (32 bytes).
    pub ed25519_secret: [u8; 32],
    #[serde(default)]
    pub created_at_unix: u64,
    /// Optional preferred label (not a legal name).
    #[serde(default)]
    pub display_hint: String,
}

/// Public half of a person principal (safe to show in UI / share with peers).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonPublic {
    pub person_id: String,
    pub verifying_key_hex: String,
    #[serde(default)]
    pub created_at_unix: u64,
    #[serde(default)]
    pub display_hint: String,
}

/// Portable bundle for installing the *same person* on another machine.
///
/// Treat like recovery material: never post publicly; import only on devices
/// the person controls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonTransferBundle {
    pub format: String,
    pub person: PersonPrincipal,
}

pub const PERSON_TRANSFER_FORMAT: &str = "qualia.person.transfer.v1";

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn person_path() -> PathBuf {
    crate::state::app_meta_dir().join("person_identity.json")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SealedPersonFile {
    format: String,
    ciphertext_hex: String,
}

/// Derive a 32-byte wrapping key for person identity at rest.
/// Prefers the install KeyVault when APP_STATE is live; otherwise a
/// machine-local key file under app meta (so cold tools can still open).
fn person_wrapping_key() -> Result<[u8; 32], String> {
    if let Some(state) = crate::state::APP_STATE.get() {
        if let Ok(vault) = state.key_vault.lock() {
            if !vault.is_locked() {
                return Ok(vault.derive_key("person_identity_v1").to_bytes());
            }
        }
    }
    // Fallback: dedicated local wrapping secret (not OS account).
    let path = crate::state::app_meta_dir().join("person_wrap.key");
    if path.exists() {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        if bytes.len() == 32 {
            let mut k = [0u8; 32];
            k.copy_from_slice(&bytes);
            return Ok(k);
        }
    }
    let mut k = [0u8; 32];
    rand::fill(&mut k);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&path, k).map_err(|e| e.to_string())?;
    Ok(k)
}

impl PersonPrincipal {
    pub fn signing_key(&self) -> SigningKey {
        SigningKey::from_bytes(&self.ed25519_secret)
    }

    pub fn verifying_key_hex(&self) -> String {
        hex::encode(VerifyingKey::from(&self.signing_key()).to_bytes())
    }

    pub fn to_public(&self) -> PersonPublic {
        PersonPublic {
            person_id: self.person_id.clone(),
            verifying_key_hex: self.verifying_key_hex(),
            created_at_unix: self.created_at_unix,
            display_hint: self.display_hint.clone(),
        }
    }

    pub fn did_from_verifying_key_bytes(vk: &[u8; 32]) -> String {
        format!("did:q42:person:{}", hex::encode(vk))
    }

    pub fn generate(display_hint: impl Into<String>) -> Result<Self, String> {
        let mut secret = [0u8; 32];
        rand::fill(&mut secret);
        let sk = SigningKey::from_bytes(&secret);
        let vk = VerifyingKey::from(&sk);
        let person_id = Self::did_from_verifying_key_bytes(&vk.to_bytes());
        Ok(Self {
            person_id,
            ed25519_secret: secret,
            created_at_unix: now_unix(),
            display_hint: display_hint.into(),
        })
    }

    /// Load existing person identity, or mint a new one (first install).
    pub fn load_or_create(display_hint: Option<&str>) -> Result<Self, String> {
        let path = person_path();
        if path.exists() {
            let bytes = std::fs::read(&path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
            if let Ok(person) = Self::load_from_bytes(&bytes) {
                // Migrate cleartext → sealed if needed.
                let _ = person.persist();
                return Ok(person);
            }
        }
        let person = Self::generate(display_hint.unwrap_or(""))?;
        person.persist()?;
        Ok(person)
    }

    fn load_from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if let Ok(sealed) = serde_json::from_slice::<SealedPersonFile>(bytes) {
            if sealed.format == SEALED_FORMAT {
                let wrap = person_wrapping_key()?;
                let ct = hex::decode(&sealed.ciphertext_hex)
                    .map_err(|e| format!("person ciphertext hex: {e}"))?;
                let plain = unwrap_key(&wrap, &ct, PERSON_AAD)
                    .map_err(|e| format!("person unwrap failed: {e:?}"))?;
                let person: PersonPrincipal = serde_json::from_slice(&plain)
                    .map_err(|e| format!("person plaintext parse: {e}"))?;
                if person.person_id.starts_with("did:q42:person:")
                    && person.ed25519_secret != [0u8; 32]
                {
                    return Ok(person);
                }
                return Err("sealed person identity failed integrity checks".into());
            }
        }
        // Legacy cleartext
        let person: PersonPrincipal =
            serde_json::from_slice(bytes).map_err(|e| format!("person identity parse: {e}"))?;
        if person.person_id.starts_with("did:q42:person:") && person.ed25519_secret != [0u8; 32] {
            return Ok(person);
        }
        Err("invalid person identity file".into())
    }

    pub fn persist(&self) -> Result<(), String> {
        let path = person_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
        }
        let plain = serde_json::to_vec(self)
            .map_err(|e| format!("failed to encode person identity: {e}"))?;
        let wrap = person_wrapping_key()?;
        let sealed = wrap_key(&wrap, &plain, PERSON_AAD)
            .map_err(|e| format!("person seal failed: {e:?}"))?;
        let file = SealedPersonFile {
            format: SEALED_FORMAT.to_string(),
            ciphertext_hex: hex::encode(sealed),
        };
        let json = serde_json::to_string_pretty(&file)
            .map_err(|e| format!("failed to encode sealed person: {e}"))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
        Ok(())
    }

    /// Sign a message with the person principal (for fleet job envelopes).
    pub fn sign_message(&self, message: &[u8]) -> [u8; 64] {
        self.signing_key().sign(message).to_bytes()
    }

    pub fn verify_message(
        person_id: &str,
        verifying_key_hex: &str,
        message: &[u8],
        sig: &[u8; 64],
    ) -> Result<(), String> {
        let key_hex = verifying_key_hex.trim().to_ascii_lowercase();
        let expected = format!("did:q42:person:{key_hex}");
        if person_id != expected {
            return Err("person_id does not match verifying key".into());
        }
        let bytes = hex::decode(&key_hex).map_err(|e| format!("verifying key hex: {e}"))?;
        if bytes.len() != 32 {
            return Err("verifying key must be 32 bytes".into());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        let vk = VerifyingKey::from_bytes(&arr).map_err(|e| format!("verifying key: {e}"))?;
        let signature = Signature::from_bytes(sig);
        vk.verify(message, &signature)
            .map_err(|_| "person signature invalid".to_string())
    }

    pub fn replace_with(person: PersonPrincipal) -> Result<PersonPrincipal, String> {
        if !person.person_id.starts_with("did:q42:person:") {
            return Err("person_id must be a did:q42:person:… identifier".into());
        }
        let expected = Self::did_from_verifying_key_bytes(
            &VerifyingKey::from(&person.signing_key()).to_bytes(),
        );
        if person.person_id != expected {
            return Err("person_id does not match signing key material".into());
        }
        person.persist()?;
        Ok(person)
    }

    pub fn transfer_bundle(&self) -> PersonTransferBundle {
        PersonTransferBundle {
            format: PERSON_TRANSFER_FORMAT.to_string(),
            person: self.clone(),
        }
    }

    pub fn from_transfer_bundle(bundle: PersonTransferBundle) -> Result<Self, String> {
        if bundle.format != PERSON_TRANSFER_FORMAT {
            return Err(format!(
                "unsupported person transfer format: {}",
                bundle.format
            ));
        }
        Self::replace_with(bundle.person)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn person_id_is_not_an_os_username() {
        let p = PersonPrincipal::generate("").unwrap();
        assert!(p.person_id.starts_with("did:q42:person:"));
        assert!(!p.person_id.contains('\\'));
        assert_ne!(p.person_id, whoami_username_fallback());
    }

    fn whoami_username_fallback() -> String {
        std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_default()
    }

    #[test]
    fn public_half_exposes_no_secret() {
        let p = PersonPrincipal::generate("Ada").unwrap();
        let pub_ = p.to_public();
        let json = serde_json::to_string(&pub_).unwrap();
        assert!(!json.contains("ed25519_secret"));
        assert_eq!(pub_.person_id, p.person_id);
        assert_eq!(pub_.verifying_key_hex.len(), 64);
    }
}
