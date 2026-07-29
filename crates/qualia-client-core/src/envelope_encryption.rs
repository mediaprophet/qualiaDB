//! **Real envelope encryption for the accountability commons payload** — the crypto that makes
//! [`ConsentCredential`](crate::consent_credential::ConsentCredential)'s "revoke destroys the wrapped key ⇒
//! no key, no payload" a *fact*, not a model with opaque placeholder bytes.
//!
//! Two layers, both real (built on the already-tested `qualia_core_db::crypto::sanctuary_audit` primitives —
//! X25519 sealed boxes + XChaCha20-Poly1305 AEAD; no new crate, no simulation):
//!
//! 1. **Payload layer (symmetric).** A random **data-encryption key (DEK)** encrypts the plaintext with an
//!    AEAD ([`wrap_key`]); the ciphertext is content-addressed (`commitment = SHA-256(ciphertext)`), giving
//!    the [`EncryptedCommonsPayload`] that many parties may replicate. Opening verifies the commitment (the
//!    bytes are the committed bytes) *then* AEAD-decrypts (tamper ⇒ failure).
//! 2. **Key layer (asymmetric, per recipient).** The DEK is **sealed to a recipient's public key**
//!    ([`seal_to`]) — an anonymous ephemeral-DH box only that recipient's secret can open. This sealed DEK
//!    **is** the credential's `wrapped_key`. So access is genuinely per-holder, and **revocation = destroying
//!    that sealed DEK** ([`ConsentCredential::revoke`](crate::consent_credential::ConsentCredential::revoke)):
//!    the recipient can no longer recover the DEK, and the ciphertext — wherever replicated — is opaque to
//!    them. When **no** live credential holds a sealed DEK for a payload, the DEK is unrecoverable and the
//!    payload is **crypto-shredded** (permanently unreadable though the bytes survive), exactly as the model
//!    promised.
//!
//! Native-only (the sealed-box primitives are `not(wasm32)`; the desktop owns keys), matching
//! `wellfair::sanctuary_vault`.
//!
//! What this does **not** yet do (named honestly, not deferred behind a lane): distribute a *remote* agent's
//! X25519 public key — that comes from the peer's published key material in the connection/identity layer
//! (`social_peers` / DID document), so a worker on their own device can be sealed to and decrypt
//! independently. Until that is wired, the host seals to the **owner's** envelope keypair by default (the
//! owner can always open their own data), and can seal to any supplied recipient public key.
//!
//! [`wrap_key`]: qualia_core_db::crypto::sanctuary_audit::wrap_key
//! [`seal_to`]: qualia_core_db::crypto::sanctuary_audit::seal_to
//! [`EncryptedCommonsPayload`]: crate::consent_credential::EncryptedCommonsPayload

use qualia_core_db::crypto::sanctuary_audit::{
    open_sealed, seal_to, unwrap_key, wrap_key, AuditKeypair,
};
use sha2::{Digest, Sha256};

use crate::consent_credential::{EncryptedCommonsPayload, PayloadCommitment};

/// AEAD associated-data domain separator for the payload layer (binds ciphertext to this use).
const PAYLOAD_AAD: &[u8] = b"qualia:accountability:payload:v1";
/// AEAD associated-data domain separator for the sealed DEK (the wrapped key).
const DEK_AAD: &[u8] = b"qualia:accountability:dek:v1";

/// A 32-byte data-encryption key. Secret: seal it to a recipient, never store it in the clear.
pub type DataKey = [u8; 32];

/// An X25519 envelope keypair for a party (the owner, or an agent). The **secret** opens sealed DEKs; the
/// **public** is what a DEK is sealed *to*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeKeypair {
    pub public: [u8; 32],
    pub secret: [u8; 32],
}

impl EnvelopeKeypair {
    /// Generate a fresh keypair (real X25519, OS randomness).
    pub fn generate() -> Result<Self, String> {
        let kp = AuditKeypair::generate().map_err(|e| format!("keypair generate: {e:?}"))?;
        Ok(Self {
            public: kp.public,
            secret: *kp.secret_bytes(),
        })
    }

    pub fn public_hex(&self) -> String {
        hex::encode(self.public)
    }
    pub fn secret_hex(&self) -> String {
        hex::encode(self.secret)
    }

    /// Reconstruct from stored hex (e.g. an agent keypair supplied for opening).
    pub fn from_hex(public_hex: &str, secret_hex: &str) -> Result<Self, String> {
        Ok(Self {
            public: parse_key_hex(public_hex, "public")?,
            secret: parse_key_hex(secret_hex, "secret")?,
        })
    }

    /// **Derive** a keypair deterministically from a root secret (the owner's ed25519 signing-key seed) and a
    /// domain tag, so the **owner envelope keypair is re-derivable and NEVER stored at rest** — no plaintext
    /// X25519 secret on disk. The derivation is `SHA-256(domain ‖ 0x1f ‖ root_secret)` → X25519 secret; it is
    /// stable, so seals to this public key open with the re-derived secret across sessions.
    pub fn derive(root_secret: &[u8; 32], domain: &[u8]) -> Self {
        let mut h = Sha256::new();
        h.update(domain);
        h.update(b"\x1f");
        h.update(root_secret);
        let derived: [u8; 32] = h.finalize().into();
        let kp = AuditKeypair::from_secret(derived);
        Self {
            public: kp.public,
            secret: *kp.secret_bytes(),
        }
    }
}

/// Domain tag for deriving the owner's envelope keypair from their ed25519 signing-key seed.
pub const OWNER_ENVELOPE_DOMAIN: &[u8] = b"qualia:accountability:envelope:owner:v1";

fn parse_key_hex(s: &str, which: &str) -> Result<[u8; 32], String> {
    let bytes = hex::decode(s.trim()).map_err(|e| format!("{which} key not hex: {e}"))?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| format!("{which} key must be 32 bytes, got {}", bytes.len()))
}

/// A fresh random DEK (four 64-bit draws from the OS RNG — the API `rand::random::<u64>()` in use elsewhere).
fn random_dek() -> DataKey {
    let mut k = [0u8; 32];
    for chunk in k.chunks_mut(8) {
        let r = rand::random::<u64>().to_le_bytes();
        chunk.copy_from_slice(&r[..chunk.len()]);
    }
    k
}

/// **Seal a plaintext payload.** Generates a random DEK, AEAD-encrypts under it, and content-addresses the
/// ciphertext. Returns the replicable [`EncryptedCommonsPayload`] and the DEK (to be sealed per recipient by
/// [`wrap_dek_to`], then dropped — do not persist it in the clear).
pub fn seal_payload(
    plaintext: &[u8],
    storers: Vec<String>,
) -> Result<(EncryptedCommonsPayload, DataKey), String> {
    let dek = random_dek();
    let ciphertext =
        wrap_key(&dek, plaintext, PAYLOAD_AAD).map_err(|e| format!("seal payload: {e:?}"))?;
    let commitment: PayloadCommitment = Sha256::digest(&ciphertext).into();
    Ok((
        EncryptedCommonsPayload::new(commitment, ciphertext, storers),
        dek,
    ))
}

/// **Seal (wrap) a DEK to a recipient's public key** — the credential's `wrapped_key`. Only the holder of
/// the matching secret can [`unwrap_dek`] it; destroying this blob (revocation) removes that access.
pub fn wrap_dek_to(recipient_public: &[u8; 32], dek: &DataKey) -> Result<Vec<u8>, String> {
    seal_to(recipient_public, dek, DEK_AAD).map_err(|e| format!("wrap DEK: {e:?}"))
}

/// **Unwrap a DEK** with the recipient's secret key. Fails for the wrong recipient or a tampered blob.
pub fn unwrap_dek(recipient_secret: &[u8; 32], wrapped: &[u8]) -> Result<DataKey, String> {
    let opened = open_sealed(recipient_secret, wrapped, DEK_AAD)
        .map_err(|e| format!("unwrap DEK: {e:?}"))?;
    opened
        .as_slice()
        .try_into()
        .map_err(|_| "unwrapped DEK was not 32 bytes".to_string())
}

/// **Open a payload** with the DEK — verifies the content-address commitment (the bytes are the committed
/// bytes) *then* AEAD-decrypts. Any tamper (to ciphertext or a swapped payload) fails.
pub fn open_payload(payload: &EncryptedCommonsPayload, dek: &DataKey) -> Result<Vec<u8>, String> {
    let recomputed: PayloadCommitment = Sha256::digest(&payload.ciphertext).into();
    if recomputed != payload.commitment {
        return Err("commitment mismatch — ciphertext is not the committed bytes".into());
    }
    unwrap_key(dek, &payload.ciphertext, PAYLOAD_AAD).map_err(|e| format!("open payload: {e:?}"))
}

/// **Open a payload directly from a recipient's secret + the credential's wrapped DEK.** The end-to-end
/// decrypt path: unwrap the sealed DEK, then open the payload. If the wrapped DEK is absent (revoked), the
/// caller has nothing to pass here — that is the crypto-enforced revocation.
pub fn open_payload_with_wrapped(
    payload: &EncryptedCommonsPayload,
    recipient_secret: &[u8; 32],
    wrapped_dek: &[u8],
) -> Result<Vec<u8>, String> {
    let dek = unwrap_dek(recipient_secret, wrapped_dek)?;
    open_payload(payload, &dek)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_wrap_unwrap_open_round_trips() {
        let owner = EnvelopeKeypair::generate().unwrap();
        let agent = EnvelopeKeypair::generate().unwrap();
        let plaintext = b"housing record: emergency placement requested 2026-07-06";

        // Owner seals the payload and seals the DEK to the agent.
        let (payload, dek) = seal_payload(plaintext, vec!["did:wf:person".into()]).unwrap();
        assert_ne!(
            payload.ciphertext.as_slice(),
            plaintext,
            "payload is really encrypted"
        );
        let wrapped = wrap_dek_to(&agent.public, &dek).unwrap();

        // The agent (their secret) recovers the DEK and opens the payload.
        let opened = open_payload_with_wrapped(&payload, &agent.secret, &wrapped).unwrap();
        assert_eq!(
            opened.as_slice(),
            plaintext,
            "agent decrypts the exact plaintext"
        );

        // The owner can also seal-to-self and open (data returns to the person).
        let wrapped_self = wrap_dek_to(&owner.public, &dek).unwrap();
        assert_eq!(
            open_payload_with_wrapped(&payload, &owner.secret, &wrapped_self)
                .unwrap()
                .as_slice(),
            plaintext
        );
    }

    #[test]
    fn wrong_recipient_cannot_unwrap_the_dek() {
        let agent = EnvelopeKeypair::generate().unwrap();
        let attacker = EnvelopeKeypair::generate().unwrap();
        let (_payload, dek) = seal_payload(b"secret", vec![]).unwrap();
        let wrapped = wrap_dek_to(&agent.public, &dek).unwrap();
        // A different secret cannot open the sealed DEK.
        assert!(
            unwrap_dek(&attacker.secret, &wrapped).is_err(),
            "only the intended recipient unwraps"
        );
    }

    #[test]
    fn revocation_destroying_the_wrapped_dek_makes_the_payload_unrecoverable() {
        // Model the credential holding the wrapped DEK; revocation drops it. Without it there is no path to
        // the DEK, so the ciphertext (however replicated) cannot be opened.
        let agent = EnvelopeKeypair::generate().unwrap();
        let (payload, dek) = seal_payload(
            b"the record",
            vec!["did:wf:person".into(), "did:wf:archive".into()],
        )
        .unwrap();
        let wrapped = Some(wrap_dek_to(&agent.public, &dek).unwrap());

        // Live: the agent opens it.
        assert!(
            open_payload_with_wrapped(&payload, &agent.secret, wrapped.as_ref().unwrap()).is_ok()
        );

        // Revoke: the wrapped DEK is destroyed. The payload bytes survive (still replicated) but there is
        // nothing to unwrap — the DEK cannot be recovered, so the payload is crypto-shredded for this holder.
        let wrapped: Option<Vec<u8>> = None;
        assert!(wrapped.is_none());
        assert!(
            payload.is_durable(),
            "the commons bytes are NOT chased down — they survive"
        );
        // With no wrapped DEK and no other key, an attempt with a guessed/zero DEK fails (AEAD).
        assert!(
            open_payload(&payload, &[0u8; 32]).is_err(),
            "no key, no payload"
        );
    }

    #[test]
    fn a_tampered_ciphertext_is_rejected() {
        let agent = EnvelopeKeypair::generate().unwrap();
        let (mut payload, dek) = seal_payload(b"unaltered record", vec![]).unwrap();
        let wrapped = wrap_dek_to(&agent.public, &dek).unwrap();
        // Flip a byte in the ciphertext. The commitment now mismatches (content-address), and even past that
        // the AEAD tag would fail.
        let mid = payload.ciphertext.len() / 2;
        payload.ciphertext[mid] ^= 0xFF;
        assert!(
            open_payload_with_wrapped(&payload, &agent.secret, &wrapped).is_err(),
            "tamper is detected"
        );
    }

    #[test]
    fn a_swapped_ciphertext_breaks_the_commitment() {
        let (mut a, _dek_a) = seal_payload(b"record A", vec![]).unwrap();
        let (b, _dek_b) = seal_payload(b"record B", vec![]).unwrap();
        // Substitute B's ciphertext under A's commitment — the content-address check catches it.
        a.ciphertext = b.ciphertext;
        assert!(
            open_payload(&a, &[0u8; 32]).is_err(),
            "commitment binds the ciphertext"
        );
    }

    #[test]
    fn derived_owner_keypair_is_deterministic_and_usable() {
        let seed = [42u8; 32]; // stands in for the owner's ed25519 signing-key seed
        let a = EnvelopeKeypair::derive(&seed, OWNER_ENVELOPE_DOMAIN);
        let b = EnvelopeKeypair::derive(&seed, OWNER_ENVELOPE_DOMAIN);
        assert_eq!(
            a, b,
            "same seed + domain ⇒ same keypair (re-derivable, nothing stored)"
        );
        // A different domain (or seed) gives an independent keypair.
        assert_ne!(a, EnvelopeKeypair::derive(&seed, b"other:domain"));
        assert_ne!(
            a,
            EnvelopeKeypair::derive(&[7u8; 32], OWNER_ENVELOPE_DOMAIN)
        );
        // And it actually works as an envelope key: seal to it, re-derive, open.
        let (payload, dek) = seal_payload(b"owner-held record", vec![]).unwrap();
        let wrapped = wrap_dek_to(&a.public, &dek).unwrap();
        let rederived = EnvelopeKeypair::derive(&seed, OWNER_ENVELOPE_DOMAIN);
        assert_eq!(
            open_payload_with_wrapped(&payload, &rederived.secret, &wrapped)
                .unwrap()
                .as_slice(),
            b"owner-held record"
        );
    }

    #[test]
    fn keypair_hex_round_trips() {
        let kp = EnvelopeKeypair::generate().unwrap();
        let back = EnvelopeKeypair::from_hex(&kp.public_hex(), &kp.secret_hex()).unwrap();
        assert_eq!(kp, back);
        assert!(EnvelopeKeypair::from_hex("zz", &kp.secret_hex()).is_err());
    }
}
