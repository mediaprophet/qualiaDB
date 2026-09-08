//! CRY-02.12 **PARTIAL** — frozen transcript / HKDF / Finished vectors from **test keys only**.
//!
//! Independent reproduction: production `Transcript` / `hkdf_sha384` / `hybrid_shared_secret`
//! / `finished_mac` are compared to oracles in this module that call `sha2` / `hkdf` directly
//! and to the hex constants below. Those oracles must not call the production wrappers.
//!
//! Not a security proof. Share encodings, dual proofs, record vectors, and live ML-KEM
//! encapsulate remain OPEN. Do not treat these bytes as production keys.

use hkdf::Hkdf;
use sha2::{Digest, Sha384};

use crate::net::qdnf::types::StrongDigest;

/// Test-only ML-KEM shared-secret stand-in. Not a live encapsulate output.
pub const TEST_KEM_SS: [u8; 32] = [0x11; 32];
/// Test-only X25519 shared-secret stand-in. Not a live Diffie–Hellman output.
pub const TEST_X25519_SS: [u8; 32] = [0x22; 32];

const TRANSCRIPT_DOMAIN: &[u8] = b"QPR-TRANSCRIPT-PQ-1";
const HKDF_SALT: &[u8] = b"qpr-pq-1";
const HYBRID_INFO_I2R: &[u8] = b"qpr-pq-1/qlink/i2r";
const FINISHED_ROLE: &[u8] = b"qsession";

const TEST_ITEMS: [(&[u8], &[u8]); 2] = [(b"suite", b"qpr-pq-1"), (b"role", b"initiator")];

/// SHA-384(`QPR-TRANSCRIPT-PQ-1` || length-delimited suite/role items). 48 bytes.
const FROZEN_TRANSCRIPT_SHA384: &str =
    "ee2f945dd01bbc31ae228d7c290e6e0af0565c340e9ce9aee937134108e830b1735ea4b105926b219c1ffbec2b448af5";

/// HKDF-SHA-384 OKM-32; salt `qpr-pq-1`; IKM = kem_ss || x25519_ss; info i2r.
const FROZEN_HYBRID_I2R: &str = "e4c3fcd4ce69a212c883d7114078ce410751cf110fb37d61ee618df833e1bea5";

/// SHA-384(transcript_digest[48] || `qsession` padded into a 64-byte buffer).
const FROZEN_FINISHED_MAC: &str =
    "e54cbc5e7a76090d363870c177dcae3585212d137ad915883472ab3707387f88ef2aaec684bd799d9ad65ad074d707d8";

/// Independent transcript digest: SHA-384 over domain || `u16be(label)||label||u32be(value)||value`.
/// Must not call [`super::transcript::Transcript`].
pub fn oracle_transcript_digest(items: &[(&[u8], &[u8])]) -> StrongDigest {
    let mut hasher = Sha384::new();
    hasher.update(TRANSCRIPT_DOMAIN);
    for &(label, value) in items {
        let label_len = u16::try_from(label.len()).expect("test label fits u16");
        let value_len = u32::try_from(value.len()).expect("test value fits u32");
        hasher.update(label_len.to_be_bytes());
        hasher.update(label);
        hasher.update(value_len.to_be_bytes());
        hasher.update(value);
    }
    digest_from_sha384(hasher)
}

fn oracle_hkdf_sha384(salt: Option<&[u8]>, ikm: &[u8], info: &[u8], okm: &mut [u8]) {
    let hk = Hkdf::<Sha384>::new(salt, ikm);
    hk.expand(info, okm)
        .expect("test-only HKDF expand within SHA-384 limit");
}

fn oracle_hybrid_shared_secret(
    ml_kem_ss: &[u8; 32],
    x25519_ss: &[u8; 32],
    info: &[u8],
    out: &mut [u8],
) {
    let mut ikm = [0u8; 64];
    ikm[..32].copy_from_slice(ml_kem_ss);
    ikm[32..].copy_from_slice(x25519_ss);
    oracle_hkdf_sha384(Some(HKDF_SALT), &ikm, info, out);
}

fn oracle_finished_mac(transcript_digest: &StrongDigest, role: &[u8]) -> StrongDigest {
    let mut buf = [0u8; 64];
    buf[..48].copy_from_slice(&transcript_digest.0);
    let role_len = role.len().min(16);
    buf[48..48 + role_len].copy_from_slice(&role[..role_len]);
    let mut hasher = Sha384::new();
    hasher.update(buf);
    digest_from_sha384(hasher)
}

fn digest_from_sha384(hasher: Sha384) -> StrongDigest {
    let out = hasher.finalize();
    let mut d = StrongDigest::ZERO;
    d.0.copy_from_slice(&out);
    d
}

fn decode_hex<const N: usize>(hex: &str) -> [u8; N] {
    assert_eq!(hex.len(), N * 2, "frozen hex length");
    let mut out = [0u8; N];
    let bytes = hex.as_bytes();
    for i in 0..N {
        let hi = from_hex_nibble(bytes[i * 2]);
        let lo = from_hex_nibble(bytes[i * 2 + 1]);
        out[i] = (hi << 4) | lo;
    }
    out
}

fn from_hex_nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => panic!("non-hex nibble in frozen vector"),
    }
}

fn production_transcript() -> super::transcript::Transcript {
    let mut t = super::transcript::Transcript::new();
    t.append(TEST_ITEMS[0].0, TEST_ITEMS[0].1)
        .expect("test transcript item 0");
    t.append(TEST_ITEMS[1].0, TEST_ITEMS[1].1)
        .expect("test transcript item 1");
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::errors::CryptoError;
    use crate::crypto::network::kdf::{hkdf_sha384, hybrid_shared_secret};
    use crate::crypto::network::transcript::{Transcript, MAX_TRANSCRIPT};
    use crate::crypto::network::x25519::X25519Secret;
    use crate::net::qdnf::crypto::handshake::finished_mac;

    #[test]
    fn transcript_matches_oracle_and_frozen_hex() {
        let production = production_transcript().digest();
        let oracle = oracle_transcript_digest(&TEST_ITEMS);
        let frozen = StrongDigest(decode_hex::<48>(FROZEN_TRANSCRIPT_SHA384));
        assert_eq!(production, oracle);
        assert_eq!(production, frozen);
        assert_eq!(oracle, frozen);
    }

    #[test]
    fn hybrid_shared_secret_matches_oracle_and_is_not_xor() {
        let mut production = [0u8; 32];
        hybrid_shared_secret(
            &TEST_KEM_SS,
            &TEST_X25519_SS,
            HYBRID_INFO_I2R,
            &mut production,
        )
        .unwrap();

        let mut via_hkdf = [0u8; 32];
        let mut ikm = [0u8; 64];
        ikm[..32].copy_from_slice(&TEST_KEM_SS);
        ikm[32..].copy_from_slice(&TEST_X25519_SS);
        hkdf_sha384(Some(HKDF_SALT), &ikm, HYBRID_INFO_I2R, &mut via_hkdf).unwrap();

        let mut oracle = [0u8; 32];
        oracle_hybrid_shared_secret(&TEST_KEM_SS, &TEST_X25519_SS, HYBRID_INFO_I2R, &mut oracle);

        let frozen = decode_hex::<32>(FROZEN_HYBRID_I2R);
        assert_eq!(production, via_hkdf);
        assert_eq!(production, oracle);
        assert_eq!(production, frozen);

        let mut xor = [0u8; 32];
        for i in 0..32 {
            xor[i] = TEST_KEM_SS[i] ^ TEST_X25519_SS[i];
        }
        assert_eq!(xor, [0x33; 32]);
        assert_ne!(production, xor);
        assert_ne!(production, TEST_KEM_SS);
        assert_ne!(production, TEST_X25519_SS);
    }

    #[test]
    fn finished_mac_is_stable() {
        let digest = production_transcript().digest();
        let production = finished_mac(&digest, FINISHED_ROLE);
        let oracle = oracle_finished_mac(&digest, FINISHED_ROLE);
        let frozen = StrongDigest(decode_hex::<48>(FROZEN_FINISHED_MAC));
        assert_eq!(production, oracle);
        assert_eq!(production, frozen);
        let again = finished_mac(&digest, FINISHED_ROLE);
        assert_eq!(production, again);
    }

    #[test]
    pub fn all_zero_x25519_rejected() {
        let secret = X25519Secret::from_bytes([0x42; 32]);
        let err = secret
            .diffie_hellman(&[0u8; 32])
            .expect_err("all-zero peer public must fail closed");
        assert_eq!(err, CryptoError::CryptoFailure);
    }

    #[test]
    pub fn truncated_transcript_item_is_not_hashed_as_success() {
        let mut t = Transcript::new();
        t.append(TEST_ITEMS[0].0, TEST_ITEMS[0].1).unwrap();
        t.append(TEST_ITEMS[1].0, TEST_ITEMS[1].1).unwrap();
        let success = t.digest();
        let success_len = t.as_bytes().len();

        let overflow = [0u8; MAX_TRANSCRIPT];
        assert_eq!(t.append(b"overflow", &overflow), Err(CryptoError::Capacity));
        assert_eq!(t.as_bytes().len(), success_len);
        assert_eq!(t.digest(), success);
        assert_eq!(success, oracle_transcript_digest(&TEST_ITEMS));
        assert_eq!(
            success,
            StrongDigest(decode_hex::<48>(FROZEN_TRANSCRIPT_SHA384))
        );

        let truncated_prefix: &[(&[u8], &[u8])] = &[TEST_ITEMS[0]];
        let truncated = oracle_transcript_digest(truncated_prefix);
        assert_ne!(truncated, success);
    }
}
