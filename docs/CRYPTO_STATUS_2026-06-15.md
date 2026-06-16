# QualiaDB Cryptography — Status & Agent Guide (2026-06-15, Updated for 0.0.15)

**Audience:** agents/engineers working in **both** `C:\Projects\qualiaDB` and
`C:\Projects\webizen-browser`. This is the single source of truth for "what crypto is
real vs. scaffolding" after the 0.0.12 crypto pass. If older docs or your memory say
"ML-DSA is a SHA3 simulation" or "BLAKE3 is declared but not wired" — that is now stale.

**Last Updated:** 2026-06-15 (Implementation from 0.0.12, confirmed present in 0.0.15)

Canonical companion: [`CRYPTO_IMPLEMENTATION_PLAN.md`](../CRYPTO_IMPLEMENTATION_PLAN.md)
(the work order + remaining backlog).

---

## TL;DR — what changed in crate `qualia-core-db` 0.0.12 (confirmed present in 0.0.15)

| Area | Before | Now |
|------|--------|-----|
| **BLAKE3** | enum variant only; `compute_hash` rejected it | ✅ real (`blake3` crate); `"BLAKE3"` in `HashEngine::compute_hash`, `compute_hash_blake3()` |
| **ChaCha20-Poly1305 / XChaCha20-Poly1305** | enum variants only; AES-256-GCM was the only cipher | ✅ real (`chacha20poly1305` crate); chosen via `encrypt_data_with_algorithm()` |
| **HKDF-SHA256 KDF** | `KeyDerivation` had an empty `initialize()` | ✅ real `KeyDerivation::derive_hkdf()` (RFC-5869 verified) |
| **ML-DSA (post-quantum)** | SHA3-based *fake* "simulation"; `MLDSA` keys were actually random ed25519 | ✅ real **ML-DSA-65 (FIPS-204)** via `fips204` crate; wired end-to-end |
| **`sign_with_did`** | returned a forged all-zero 64-byte signature | ✅ fails closed (`Err`) — query layer has no keys |

**Verification:** 21/21 crypto tests pass; host build + `wasm32-unknown-unknown` build both green.

**Version Status:** This implementation from 0.0.12 remains present and functional in 0.0.15. No breaking changes to the crypto layer have been made between these versions.

---

## What is REAL (safe to rely on)

All in `crates/qualia-core-db/src/`:

- **Ed25519** sign/verify — `cryptographic_library.rs` (for `KeyAlgorithm::EdDSA`/non-MLDSA
  keys), `wal.rs` (WAL entry signing), `webizen_identifiers.rs` (`verify_signature`,
  `verify_strict`).
- **ML-DSA-65 (FIPS-204, post-quantum)** — `fiduciary_crypto.rs` via the `fips204` crate.
  - `MlDsaSigner::generate_keypair()` → `(MlDsaPrivateKey, MlDsaPublicKey)` (real keypair).
  - `MlDsaSigner::{sign, verify}` and byte-level `sign_with_secret(sk_bytes,…)` /
    `verify_with_public(pk_bytes,…)`.
  - `FiduciaryCrypto` facade: `generate_key` / `sign` / `verify` / `hash_token`.
  - Sizes (interoperable FIPS-204 bytes): **pk 1952 B, sk 4032 B, sig 3309 B**.
  - Context binding: a `CryptoContext { domain, purpose, timestamp, nonce }` is hashed
    (SHA3-512) into the ML-DSA context string. **Sign and verify must use an equal context.**
- **ML-DSA via the high-level library** — `CryptographicLibrary::generate_mldsa_key_pair`
  makes a real keypair; `sign_data` / `verify_signature` route `KeyAlgorithm::MLDSA` through
  ML-DSA (signing the message directly — no SHA-256 prehash), everything else through Ed25519.
- **AEAD encryption** — `cryptographic_library.rs`: AES-256-GCM, ChaCha20-Poly1305 (12-byte
  nonce), XChaCha20-Poly1305 (24-byte nonce). All 32-byte keys, 16-byte tags.
  - `decrypt_data` dispatches on the algorithm stored in `EncryptedData`.
  - ⚠️ **AAD is not persisted** by `EncryptedData`, so `decrypt_data` cannot re-supply it —
    round-trips must use `None` AAD (same limitation as the pre-existing AES path).
- **Hashing** — SHA-256, SHA-512, BLAKE3 (32-byte digest).
- **KDF** — HKDF-SHA256 (`KeyDerivation::derive_hkdf`).

## What is SCAFFOLDING ONLY (do NOT rely on — enum variants without real backends)

- `KeyAlgorithm::{Kyber, NTRU, SPHINCS, RSA, ECDSA}` — key generation returns random bytes;
  no real algorithm behind them.
- ZK proofs in `cryptographic_library.rs` (`ProofEngine` / `generate_proof_data`) — produces
  **SHA-256 commitments, not real zk-SNARK/STARK proofs**, despite the
  Groth16/PLONK/Halo2/Bulletproofs type names. (A separate `zk_proofs.rs` does Pedersen-style
  *structural* validation — also not a full proof system.)
- `EncryptionAlgorithm::Custom(..)` — returns `UnsupportedAlgorithm`.

---

## How to use it (quick recipes)

```rust
use qualia_core_db::specialized_libs::cryptographic_library::{
    CryptographicLibrary, EncryptionAlgorithm, SecurityLevel,
};

let mut lib = CryptographicLibrary::new();
lib.initialize().unwrap();

// BLAKE3
let h = lib.compute_hash_blake3(b"data").unwrap();          // 32 bytes

// Post-quantum ML-DSA-65 sign/verify
let kp = lib.generate_mldsa_key_pair("alice".into(), SecurityLevel::High).unwrap();
let sig = lib.sign_data("alice_private", b"msg").unwrap();
let ok  = lib.verify_signature("alice_public", &sig.result, b"msg").unwrap();

// ChaCha20-Poly1305 (key must be a stored Symmetric key; AAD = None for round-trip)
let enc = lib.encrypt_data_with_algorithm("symkey", b"secret", None,
                                          EncryptionAlgorithm::ChaCha20Poly1305).unwrap();
let dec = lib.decrypt_data("symkey", &enc.result).unwrap();
```

```rust
// Lower-level ML-DSA directly:
use qualia_core_db::fiduciary_crypto::MlDsaSigner;
let (sk, pk) = MlDsaSigner::generate_keypair().unwrap();   // real FIPS-204 keys
```

---

## Guidance for `webizen-browser` agents

- The browser/desktop consumes qualiaDB's WASM + native builds. **All of the above compiles
  to `wasm32-unknown-unknown`** (the `fips204`, `blake3`, `chacha20poly1305` crates are
  WASM-clean; entropy comes from `getrandom` with the `js` feature already configured).
- If a Webizen feature needs **post-quantum signatures**, use ML-DSA-65 as above — it is real
  now. Do **not** route through the old "simulation" assumptions.
- **DID signing does not happen in the SPARQL query layer.** `SparqlDidHandler::sign_with_did`
  fails closed by design (it holds no keys). Sign in the identity/key-vault layer.
- `QUALIA_DB_LOGIC_AUDIT.md` line ~31 has been corrected to match this status; if you find
  other webizen-browser docs asserting "post-quantum ML-DSA" or "zk-SNARK proofs" as
  finished, link them here and mark the zk-SNARK part as scaffolding.

---

## Remaining backlog (needs product sign-off before starting)

1. **ML-DSA storage + VC issuance wiring** — 3309-byte signatures don't fit one NQuin field;
   needs a fragment/Merkle storage strategy and the VC-graph anchor (see
   `ARCHITECTURE.md §37`). The signing primitive is ready; the credential wiring is not.
2. **Real ZK proof backend** (Halo2 / arkworks) to replace the SHA-256 commitment stub.
3. **Kyber/NTRU/SPHINCS, RSA/ECDSA** key algorithms (currently enum-only).
4. **AAD persistence** in `EncryptedData` so `decrypt_data` can authenticate AAD.
5. Optional: bump ML-DSA-65 → ML-DSA-87 (swap the `fips204::ml_dsa_65` import + the
   `ml-dsa-65` Cargo feature).

---

## Files changed in this pass (for reviewers)

- `crates/qualia-core-db/Cargo.toml` — added `blake3`, `chacha20poly1305`,
  `fips204 (features ml-dsa-65, default-rng)`.
- `crates/qualia-core-db/src/specialized_libs/cryptographic_library.rs` — BLAKE3 arm +
  `compute_hash_blake3`; AEAD algorithm dispatch + `encrypt_data_with_algorithm`;
  `KeyDerivation::derive_hkdf`; real ML-DSA routing in `generate_mldsa_key_pair` /
  `sign_data` / `verify_signature`; new tests.
- `crates/qualia-core-db/src/fiduciary_crypto.rs` — real ML-DSA-65 (fips204); removed fake
  lattice helpers; new key/sig byte structs; updated tests.
- `crates/qualia-core-db/src/sparql_library/sparql_did.rs` — `sign_with_did` fails closed.
- `crates/qualia-core-db/src/webizen_identifiers.rs` — corrected stale doc comment.
