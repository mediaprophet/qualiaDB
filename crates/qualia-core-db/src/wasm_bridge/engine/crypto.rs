//! Cryptographic hashing & key derivation exports — SHA-256, SHA-512, SHA3-256,
//! BLAKE3, and HKDF-SHA256.
//!
//! These are the deterministic, RNG-free RustCrypto primitives (`sha2`, `sha3`,
//! `blake3`, `hkdf`) the native `cryptographic_library` uses, called directly so
//! they run identically in the browser. Key generation and signing
//! (Ed25519, ML-DSA-65) need a browser RNG and are deliberately NOT exposed here —
//! that wiring is a separate piece of work.
#![cfg(target_arch = "wasm32")]

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use super::jserr;

/// Message bytes from `{ text }` (UTF-8) or `{ hex }` (exactly one).
#[derive(Deserialize)]
struct BytesIn {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    hex: Option<String>,
}

fn from_hex(s: &str) -> Result<Vec<u8>, JsValue> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err(JsValue::from_str("hex string must have even length"));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| JsValue::from_str(&format!("invalid hex: {e}")))
        })
        .collect()
}

fn to_hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for byte in b {
        s.push_str(&format!("{byte:02x}"));
    }
    s
}

impl BytesIn {
    fn bytes(&self) -> Result<Vec<u8>, JsValue> {
        match (&self.text, &self.hex) {
            (Some(t), None) => Ok(t.as_bytes().to_vec()),
            (None, Some(h)) => from_hex(h),
            (Some(_), Some(_)) => Err(JsValue::from_str("provide exactly one of `text` or `hex`")),
            (None, None) => Err(JsValue::from_str("provide `text` (UTF-8) or `hex`")),
        }
    }
}

#[derive(Serialize)]
struct HashOut {
    algorithm: &'static str,
    hex: String,
    bytes: usize,
}

fn hash_out(algorithm: &'static str, digest: Vec<u8>) -> Result<JsValue, JsValue> {
    let bytes = digest.len();
    Ok(serde_wasm_bindgen::to_value(&HashOut {
        algorithm,
        hex: to_hex(&digest),
        bytes,
    })?)
}

/// SHA-256 digest of `{ text } | { hex }` → `{ algorithm, hex, bytes }`.
#[wasm_bindgen]
pub fn crypto_sha256(val: JsValue) -> Result<JsValue, JsValue> {
    use sha2::{Digest, Sha256};
    let p: BytesIn = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let mut h = Sha256::new();
    h.update(p.bytes()?);
    hash_out("SHA-256", h.finalize().to_vec())
}

/// SHA-512 digest.
#[wasm_bindgen]
pub fn crypto_sha512(val: JsValue) -> Result<JsValue, JsValue> {
    use sha2::{Digest, Sha512};
    let p: BytesIn = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let mut h = Sha512::new();
    h.update(p.bytes()?);
    hash_out("SHA-512", h.finalize().to_vec())
}

/// SHA3-256 (Keccak) digest.
#[wasm_bindgen]
pub fn crypto_sha3_256(val: JsValue) -> Result<JsValue, JsValue> {
    use sha3::{Digest, Sha3_256};
    let p: BytesIn = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let mut h = Sha3_256::new();
    h.update(p.bytes()?);
    hash_out("SHA3-256", h.finalize().to_vec())
}

/// BLAKE3 digest (256-bit).
#[wasm_bindgen]
pub fn crypto_blake3(val: JsValue) -> Result<JsValue, JsValue> {
    let p: BytesIn = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    hash_out("BLAKE3", blake3::hash(&p.bytes()?).as_bytes().to_vec())
}

/// HKDF-SHA256 key derivation (RFC 5869). Input
/// `{ ikm:{text|hex}, salt?:{text|hex}, info?:{text|hex}, length }` →
/// `{ algorithm, okm_hex, length }`. `length` is output bytes (1..=8160).
#[wasm_bindgen]
pub fn crypto_hkdf_sha256(val: JsValue) -> Result<JsValue, JsValue> {
    use hkdf::Hkdf;
    use sha2::Sha256;
    #[derive(Deserialize)]
    struct In {
        ikm: BytesIn,
        #[serde(default)]
        salt: Option<BytesIn>,
        #[serde(default)]
        info: Option<BytesIn>,
        length: usize,
    }
    let inp: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    if inp.length == 0 || inp.length > 255 * 32 {
        return Err(JsValue::from_str("length must be in 1..=8160 bytes"));
    }
    let ikm = inp.ikm.bytes()?;
    let salt = match &inp.salt {
        Some(s) => Some(s.bytes()?),
        None => None,
    };
    let info = match &inp.info {
        Some(i) => i.bytes()?,
        None => Vec::new(),
    };
    let hk = Hkdf::<Sha256>::new(salt.as_deref(), &ikm);
    let mut okm = vec![0u8; inp.length];
    hk.expand(&info, &mut okm)
        .map_err(|e| JsValue::from_str(&format!("hkdf expand: {e}")))?;
    #[derive(Serialize)]
    struct Out {
        algorithm: &'static str,
        okm_hex: String,
        length: usize,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        algorithm: "HKDF-SHA256",
        okm_hex: to_hex(&okm),
        length: inp.length,
    })?)
}

// ── AEAD (authenticated encryption) ─────────────────────────────────────────
// Caller supplies the 32-byte key and the nonce; the demo is explicit that nonce
// reuse under a fixed key breaks the security guarantee. The output ciphertext is
// `ciphertext || 16-byte Poly1305/GCM tag`.

/// Resolve a 32-byte key + an algorithm-correct nonce, then seal. Validates all
/// lengths and fails closed (no panic) before touching the fixed-size GenericArrays.
fn aead_seal(alg: &str, key: &[u8], nonce: &[u8], pt: &[u8], aad: &[u8]) -> Result<Vec<u8>, JsValue> {
    use aes_gcm::aead::generic_array::GenericArray;
    use aes_gcm::aead::{Aead, KeyInit, Payload};
    if key.len() != 32 {
        return Err(JsValue::from_str("key must be 32 bytes (64 hex chars)"));
    }
    let payload = Payload { msg: pt, aad };
    let fail = || JsValue::from_str("AEAD seal failed");
    match alg.to_ascii_lowercase().as_str() {
        "aes256gcm" | "aes-256-gcm" => {
            if nonce.len() != 12 {
                return Err(JsValue::from_str("aes256gcm nonce must be 12 bytes (24 hex)"));
            }
            let c = aes_gcm::Aes256Gcm::new(GenericArray::from_slice(key));
            c.encrypt(GenericArray::from_slice(nonce), payload).map_err(|_| fail())
        }
        "chacha20poly1305" | "chacha" => {
            if nonce.len() != 12 {
                return Err(JsValue::from_str("chacha20poly1305 nonce must be 12 bytes (24 hex)"));
            }
            let c = chacha20poly1305::ChaCha20Poly1305::new(GenericArray::from_slice(key));
            c.encrypt(GenericArray::from_slice(nonce), payload).map_err(|_| fail())
        }
        "xchacha20poly1305" | "xchacha" => {
            if nonce.len() != 24 {
                return Err(JsValue::from_str("xchacha20poly1305 nonce must be 24 bytes (48 hex)"));
            }
            let c = chacha20poly1305::XChaCha20Poly1305::new(GenericArray::from_slice(key));
            c.encrypt(GenericArray::from_slice(nonce), payload).map_err(|_| fail())
        }
        other => Err(JsValue::from_str(&format!(
            "unknown algorithm '{other}' (aes256gcm | chacha20poly1305 | xchacha20poly1305)"
        ))),
    }
}

/// Open (decrypt + verify). Same validation; returns the plaintext or fails closed
/// on a bad tag / wrong key / wrong nonce / tampered ciphertext.
fn aead_open(alg: &str, key: &[u8], nonce: &[u8], ct: &[u8], aad: &[u8]) -> Result<Vec<u8>, JsValue> {
    use aes_gcm::aead::generic_array::GenericArray;
    use aes_gcm::aead::{Aead, KeyInit, Payload};
    if key.len() != 32 {
        return Err(JsValue::from_str("key must be 32 bytes (64 hex chars)"));
    }
    let payload = Payload { msg: ct, aad };
    let fail = || JsValue::from_str("AEAD open failed (bad tag / wrong key, nonce, or aad)");
    match alg.to_ascii_lowercase().as_str() {
        "aes256gcm" | "aes-256-gcm" => {
            if nonce.len() != 12 {
                return Err(JsValue::from_str("aes256gcm nonce must be 12 bytes (24 hex)"));
            }
            let c = aes_gcm::Aes256Gcm::new(GenericArray::from_slice(key));
            c.decrypt(GenericArray::from_slice(nonce), payload).map_err(|_| fail())
        }
        "chacha20poly1305" | "chacha" => {
            if nonce.len() != 12 {
                return Err(JsValue::from_str("chacha20poly1305 nonce must be 12 bytes (24 hex)"));
            }
            let c = chacha20poly1305::ChaCha20Poly1305::new(GenericArray::from_slice(key));
            c.decrypt(GenericArray::from_slice(nonce), payload).map_err(|_| fail())
        }
        "xchacha20poly1305" | "xchacha" => {
            if nonce.len() != 24 {
                return Err(JsValue::from_str("xchacha20poly1305 nonce must be 24 bytes (48 hex)"));
            }
            let c = chacha20poly1305::XChaCha20Poly1305::new(GenericArray::from_slice(key));
            c.decrypt(GenericArray::from_slice(nonce), payload).map_err(|_| fail())
        }
        other => Err(JsValue::from_str(&format!(
            "unknown algorithm '{other}' (aes256gcm | chacha20poly1305 | xchacha20poly1305)"
        ))),
    }
}

/// AEAD encrypt. Input `{ algorithm, key:{text|hex}, nonce:{text|hex},
/// plaintext:{text|hex}, aad?:{text|hex} }` → `{ algorithm, ciphertext_hex, bytes }`.
/// `algorithm` ∈ aes256gcm | chacha20poly1305 | xchacha20poly1305. Key is 32 bytes;
/// nonce 12 (24 for xchacha). The caller owns the nonce — NEVER reuse a (key, nonce).
#[wasm_bindgen]
pub fn crypto_aead_encrypt(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        algorithm: String,
        key: BytesIn,
        nonce: BytesIn,
        plaintext: BytesIn,
        #[serde(default)]
        aad: Option<BytesIn>,
    }
    let inp: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let aad = match &inp.aad {
        Some(a) => a.bytes()?,
        None => Vec::new(),
    };
    let ct = aead_seal(
        &inp.algorithm,
        &inp.key.bytes()?,
        &inp.nonce.bytes()?,
        &inp.plaintext.bytes()?,
        &aad,
    )?;
    #[derive(Serialize)]
    struct Out {
        algorithm: String,
        ciphertext_hex: String,
        bytes: usize,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        algorithm: inp.algorithm,
        bytes: ct.len(),
        ciphertext_hex: to_hex(&ct),
    })?)
}

/// AEAD decrypt + verify. Input `{ algorithm, key:{text|hex}, nonce:{text|hex},
/// ciphertext:{text|hex}, aad?:{text|hex} }` → `{ algorithm, plaintext_hex,
/// plaintext_utf8?, bytes }`. Fails closed on a bad tag / wrong key, nonce, or aad.
#[wasm_bindgen]
pub fn crypto_aead_decrypt(val: JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    struct In {
        algorithm: String,
        key: BytesIn,
        nonce: BytesIn,
        ciphertext: BytesIn,
        #[serde(default)]
        aad: Option<BytesIn>,
    }
    let inp: In = serde_wasm_bindgen::from_value(val).map_err(jserr)?;
    let aad = match &inp.aad {
        Some(a) => a.bytes()?,
        None => Vec::new(),
    };
    let pt = aead_open(
        &inp.algorithm,
        &inp.key.bytes()?,
        &inp.nonce.bytes()?,
        &inp.ciphertext.bytes()?,
        &aad,
    )?;
    #[derive(Serialize)]
    struct Out {
        algorithm: String,
        plaintext_hex: String,
        plaintext_utf8: Option<String>,
        bytes: usize,
    }
    Ok(serde_wasm_bindgen::to_value(&Out {
        algorithm: inp.algorithm,
        bytes: pt.len(),
        plaintext_utf8: String::from_utf8(pt.clone()).ok(),
        plaintext_hex: to_hex(&pt),
    })?)
}
