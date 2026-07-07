//! Sanctuary audit primitives (vault v2, first slice) — the crypto under the decoy-mirroring design.
//!
//! Three isolated, independently-tested primitives. Nothing here touches the vault yet; the vault
//! wiring is a later slice. See `docs/plans/adr-sanctuary-vault-v2-cbor-decoy-mirroring.md`.
//!
//! 1. **Blind write-only audit channel** — an **X25519 sealed box** (anonymous ECIES): a decoy
//!    session, holding only the audit *public* key, can [`seal_to`] a record so that only the holder
//!    of the audit *secret* (the real lane) can [`open_sealed`] it. The writer cannot read back what
//!    it wrote, and cannot forge or tamper without detection. This is how a coercer's actions get
//!    logged into a channel they can append to but never read.
//! 2. **One-way key wrapping** — [`wrap_key`]/[`unwrap_key`]: the real lane key wraps the decoy lane
//!    key (and the audit secret), so a real session can reach *down* into the decoy to curate it,
//!    but the decoy can never reach *up*.
//! 3. **Hash-chained content addressing** — [`chain_hash`]: an append-only, tamper-evident DAG link
//!    (BLAKE3 over `parent ‖ payload`); rewriting or dropping a record breaks every link after it.
//!
//! Symmetric AEAD is XChaCha20-Poly1305 (24-byte nonce). Sealed-box key/nonce are derived from the
//! ECDH shared secret via BLAKE3 `derive_key` (domain-separated); each seal uses a fresh ephemeral
//! key, so sealing is non-deterministic (no plaintext-equality leak) and nonce reuse is impossible.

use chacha20poly1305::aead::{AeadInOut, KeyInit};
use chacha20poly1305::XChaCha20Poly1305;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

const TAG_BYTES: usize = 16;
const EPK_BYTES: usize = 32;
const XNONCE_BYTES: usize = 24;

const SEAL_KEY_CTX: &str = "q42:sanctuary:audit:seal:key:v1";
const SEAL_NONCE_CTX: &str = "q42:sanctuary:audit:seal:nonce:v1";
const CHAIN_HASH_LEN: usize = 32;

/// Genesis parent for a fresh hash chain / DAG branch.
pub const GENESIS_PARENT: [u8; CHAIN_HASH_LEN] = [0u8; CHAIN_HASH_LEN];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SanctuaryAuditError {
    Rng,
    Encrypt,
    Decrypt,
    Malformed,
}

fn rand_bytes<const N: usize>() -> Result<[u8; N], SanctuaryAuditError> {
    let mut b = [0u8; N];
    getrandom::fill(&mut b).map_err(|_| SanctuaryAuditError::Rng)?;
    Ok(b)
}

/// An audit keypair. The **public** key is exposed to any session (including the decoy) so it can
/// append sealed records; the **secret** lives only in the real lane (wrapped under the real key)
/// and is the sole means of reading them.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct AuditKeypair {
    #[zeroize(skip)]
    pub public: [u8; 32],
    secret: [u8; 32],
}

impl AuditKeypair {
    pub fn generate() -> Result<Self, SanctuaryAuditError> {
        let secret = rand_bytes::<32>()?;
        let public = PublicKey::from(&StaticSecret::from(secret)).to_bytes();
        Ok(Self { public, secret })
    }

    /// Construct a keypair from a caller-supplied 32-byte secret (e.g. a KDF-derived key). Lets an envelope
    /// keypair be **re-derived from a root secret** rather than stored at rest. `StaticSecret::from` clamps
    /// the scalar deterministically, so any 32 bytes are a valid secret and `seal_to(public)` /
    /// `open_sealed(secret)` stay consistent.
    pub fn from_secret(secret: [u8; 32]) -> Self {
        let public = PublicKey::from(&StaticSecret::from(secret)).to_bytes();
        Self { public, secret }
    }

    /// The secret half — hold this only inside the real lane.
    pub fn secret_bytes(&self) -> &[u8; 32] {
        &self.secret
    }
}

fn derive_seal_key_nonce(
    shared: &[u8],
    ephemeral_public: &[u8; 32],
    recipient_public: &[u8; 32],
) -> ([u8; 32], [u8; XNONCE_BYTES]) {
    // Bind the derivation to both endpoints so a record can't be replayed under another recipient.
    let mut km = Vec::with_capacity(shared.len() + 64);
    km.extend_from_slice(shared);
    km.extend_from_slice(ephemeral_public);
    km.extend_from_slice(recipient_public);
    let key = blake3::derive_key(SEAL_KEY_CTX, &km);
    let nonce_full = blake3::derive_key(SEAL_NONCE_CTX, &km);
    let mut nonce = [0u8; XNONCE_BYTES];
    nonce.copy_from_slice(&nonce_full[..XNONCE_BYTES]);
    (key, nonce)
}

fn cipher_for(key: &[u8; 32]) -> Result<XChaCha20Poly1305, SanctuaryAuditError> {
    let key =
        <&chacha20poly1305::Key>::try_from(&key[..]).map_err(|_| SanctuaryAuditError::Encrypt)?;
    Ok(XChaCha20Poly1305::new(key))
}

fn aead_seal(
    key: &[u8; 32],
    nonce: &[u8; XNONCE_BYTES],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, SanctuaryAuditError> {
    let cipher = cipher_for(key)?;
    let nonce = <&chacha20poly1305::XNonce>::try_from(&nonce[..])
        .map_err(|_| SanctuaryAuditError::Encrypt)?;
    let mut buffer = plaintext.to_vec();
    let tag = cipher
        .encrypt_inout_detached(nonce, aad, buffer.as_mut_slice().into())
        .map_err(|_| SanctuaryAuditError::Encrypt)?;
    buffer.extend_from_slice(tag.as_slice());
    Ok(buffer)
}

fn aead_open(
    key: &[u8; 32],
    nonce: &[u8; XNONCE_BYTES],
    ct_and_tag: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, SanctuaryAuditError> {
    if ct_and_tag.len() < TAG_BYTES {
        return Err(SanctuaryAuditError::Malformed);
    }
    let split = ct_and_tag.len() - TAG_BYTES;
    let (ct, tag_bytes) = ct_and_tag.split_at(split);
    let cipher = cipher_for(key)?;
    let nonce = <&chacha20poly1305::XNonce>::try_from(&nonce[..])
        .map_err(|_| SanctuaryAuditError::Decrypt)?;
    let tag = <&chacha20poly1305::Tag>::try_from(tag_bytes)
        .map_err(|_| SanctuaryAuditError::Malformed)?;
    let mut buffer = ct.to_vec();
    cipher
        .decrypt_inout_detached(nonce, aad, buffer.as_mut_slice().into(), tag)
        .map_err(|_| SanctuaryAuditError::Decrypt)?;
    Ok(buffer)
}

/// Seal `plaintext` so that only the holder of the secret matching `recipient_public` can open it.
/// Anonymous: the sealer needs no identity, only the recipient's public key. Output layout:
/// `ephemeral_public(32) ‖ ciphertext ‖ tag(16)`. Non-deterministic (fresh ephemeral key per call).
pub fn seal_to(
    recipient_public: &[u8; 32],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, SanctuaryAuditError> {
    let ephemeral_secret = StaticSecret::from(rand_bytes::<32>()?);
    let ephemeral_public = PublicKey::from(&ephemeral_secret).to_bytes();
    let shared = ephemeral_secret.diffie_hellman(&PublicKey::from(*recipient_public));
    let (key, nonce) =
        derive_seal_key_nonce(shared.as_bytes(), &ephemeral_public, recipient_public);
    let body = aead_seal(&key, &nonce, plaintext, aad)?;
    let mut out = Vec::with_capacity(EPK_BYTES + body.len());
    out.extend_from_slice(&ephemeral_public);
    out.extend_from_slice(&body);
    Ok(out)
}

/// Open a sealed box produced by [`seal_to`]. Requires the recipient secret; the public key alone
/// cannot open it (that is the whole point — the decoy session writes but cannot read).
pub fn open_sealed(
    recipient_secret: &[u8; 32],
    sealed: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, SanctuaryAuditError> {
    if sealed.len() < EPK_BYTES + TAG_BYTES {
        return Err(SanctuaryAuditError::Malformed);
    }
    let mut ephemeral_public = [0u8; 32];
    ephemeral_public.copy_from_slice(&sealed[..EPK_BYTES]);
    let body = &sealed[EPK_BYTES..];

    let secret = StaticSecret::from(*recipient_secret);
    let recipient_public = PublicKey::from(&secret).to_bytes();
    let shared = secret.diffie_hellman(&PublicKey::from(ephemeral_public));
    let (key, nonce) =
        derive_seal_key_nonce(shared.as_bytes(), &ephemeral_public, &recipient_public);
    aead_open(&key, &nonce, body, aad)
}

/// Wrap `key_material` under `wrapping_key` (AEAD). Output: `nonce(24) ‖ ciphertext ‖ tag(16)`.
/// Used for the one-way hierarchy: the real lane key wraps the decoy key + the audit secret.
pub fn wrap_key(
    wrapping_key: &[u8; 32],
    key_material: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, SanctuaryAuditError> {
    let nonce = rand_bytes::<XNONCE_BYTES>()?;
    let body = aead_seal(wrapping_key, &nonce, key_material, aad)?;
    let mut out = Vec::with_capacity(XNONCE_BYTES + body.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&body);
    Ok(out)
}

/// Unwrap a blob produced by [`wrap_key`]. Fails on the wrong key, wrong AAD, or tampering.
pub fn unwrap_key(
    wrapping_key: &[u8; 32],
    wrapped: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, SanctuaryAuditError> {
    if wrapped.len() < XNONCE_BYTES + TAG_BYTES {
        return Err(SanctuaryAuditError::Malformed);
    }
    let mut nonce = [0u8; XNONCE_BYTES];
    nonce.copy_from_slice(&wrapped[..XNONCE_BYTES]);
    aead_open(wrapping_key, &nonce, &wrapped[XNONCE_BYTES..], aad)
}

/// One append-only DAG link: `BLAKE3(parent ‖ payload)`. Rewriting or reordering any record changes
/// its hash and breaks the parent link of everything after it (tamper-evidence). Start a branch from
/// [`GENESIS_PARENT`].
pub fn chain_hash(parent: &[u8; CHAIN_HASH_LEN], payload: &[u8]) -> [u8; CHAIN_HASH_LEN] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(parent);
    hasher.update(payload);
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_box_round_trips() {
        let kp = AuditKeypair::generate().unwrap();
        let msg = b"decoy session 1: coercer added note at 12:04";
        let sealed = seal_to(&kp.public, msg, b"branch:session-1").unwrap();
        let opened = open_sealed(kp.secret_bytes(), &sealed, b"branch:session-1").unwrap();
        assert_eq!(opened, msg);
    }

    #[test]
    fn only_the_secret_holder_can_open() {
        let kp = AuditKeypair::generate().unwrap();
        let other = AuditKeypair::generate().unwrap();
        let sealed = seal_to(&kp.public, b"evidence", b"").unwrap();
        assert!(open_sealed(other.secret_bytes(), &sealed, b"").is_err());
    }

    #[test]
    fn public_key_alone_cannot_read() {
        // The whole point: the decoy session holds only the public key and must not be able to read
        // what it sealed. Using the public bytes as if they were the secret does not recover it.
        let kp = AuditKeypair::generate().unwrap();
        let sealed = seal_to(&kp.public, b"only-the-real-lane-reads-this", b"").unwrap();
        match open_sealed(&kp.public, &sealed, b"") {
            Err(_) => {}
            Ok(pt) => assert_ne!(pt.as_slice(), b"only-the-real-lane-reads-this"),
        }
    }

    #[test]
    fn tampered_sealed_box_is_rejected() {
        let kp = AuditKeypair::generate().unwrap();
        let mut sealed = seal_to(&kp.public, b"unaltered", b"").unwrap();
        let mid = sealed.len() / 2;
        sealed[mid] ^= 0xFF;
        assert!(open_sealed(kp.secret_bytes(), &sealed, b"").is_err());
    }

    #[test]
    fn aad_is_bound() {
        let kp = AuditKeypair::generate().unwrap();
        let sealed = seal_to(&kp.public, b"m", b"branch:session-1").unwrap();
        assert!(open_sealed(kp.secret_bytes(), &sealed, b"branch:session-2").is_err());
    }

    #[test]
    fn sealing_is_non_deterministic() {
        // Fresh ephemeral key per seal => identical plaintext yields distinct ciphertext (no
        // equality leak to whoever can see the audit region).
        let kp = AuditKeypair::generate().unwrap();
        let a = seal_to(&kp.public, b"same-note", b"").unwrap();
        let b = seal_to(&kp.public, b"same-note", b"").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn key_wrap_round_trips_and_binds_key_and_aad() {
        let real_key = rand_bytes::<32>().unwrap();
        let decoy_key = [7u8; 32];
        let wrapped = wrap_key(&real_key, &decoy_key, b"role:decoy-lane-key").unwrap();
        assert_eq!(
            unwrap_key(&real_key, &wrapped, b"role:decoy-lane-key").unwrap(),
            decoy_key
        );
        // Wrong wrapping key (the decoy cannot reach up).
        assert!(unwrap_key(&[9u8; 32], &wrapped, b"role:decoy-lane-key").is_err());
        // Wrong AAD.
        assert!(unwrap_key(&real_key, &wrapped, b"role:something-else").is_err());
    }

    #[test]
    fn hash_chain_is_deterministic_and_tamper_evident() {
        let r1 = chain_hash(&GENESIS_PARENT, b"session-1 opened");
        let r2 = chain_hash(&r1, b"note added: 'call me'");
        let r3 = chain_hash(&r2, b"note edited");

        // Deterministic.
        assert_eq!(r1, chain_hash(&GENESIS_PARENT, b"session-1 opened"));
        assert_ne!(r1, r2);
        assert_ne!(r2, r3);

        // Rewrite record 2's payload => r2 changes => r3's parent link no longer matches.
        let r2_tampered = chain_hash(&r1, b"note added: 'do NOT call'");
        assert_ne!(r2, r2_tampered);
        assert_ne!(r3, chain_hash(&r2_tampered, b"note edited"));
    }
}
