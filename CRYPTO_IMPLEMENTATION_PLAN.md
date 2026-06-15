# Crypto Implementation Plan — QualiaDB

> **Status (crate `qualia-core-db` 0.0.13, 2026-06-15):** the first wave of real crypto is
> **done and verified** — BLAKE3, ChaCha20-Poly1305 / XChaCha20-Poly1305, HKDF-SHA256, and
> real FIPS-204 **ML-DSA-65**. The remaining work is spec'd as **Tasks 5–9** in
> [PART 2 — The Remainder](#part-2--the-remainder-tasks-59). A
> [Known pre-existing test failures](#known-pre-existing-test-failures-not-caused-by-the-crypto-work)
> section at the end records unrelated suite failures.
>
> Canonical current-state companion doc: [`docs/CRYPTO_STATUS_2026-06-15.md`](docs/CRYPTO_STATUS_2026-06-15.md).

---

## Current crypto state

The crypto lives in two places in `crates/qualia-core-db/src/`:

1. **`specialized_libs/cryptographic_library.rs`** — the "Cryptographic Library" specialized
   lib (~3,900 lines, mostly enum/struct scaffolding around a smaller set of real primitives).
2. **`fiduciary_crypto.rs`** — the post-quantum signing module (real ML-DSA-65 via `fips204`).

### What is REAL (safe to rely on)

- **Ed25519** sign/verify — `cryptographic_library.rs` (for `KeyAlgorithm::EdDSA` / non-MLDSA
  keys), `wal.rs` (WAL signing), `webizen_identifiers.rs` (`verify_signature` / `verify_strict`).
- **ML-DSA-65 (FIPS-204, post-quantum)** — `fiduciary_crypto.rs` via the `fips204` crate.
  - `MlDsaSigner::{generate_keypair, sign, verify}` + byte-level `sign_with_secret` /
    `verify_with_public`; `FiduciaryCrypto` facade (`generate_key`/`sign`/`verify`/`hash_token`).
  - FIPS-204 serialized sizes: **pk 1952 B, sk 4032 B, sig 3309 B** (interoperable).
  - A `CryptoContext { domain, purpose, timestamp, nonce }` is bound via the ML-DSA context
    string (SHA3-512 of the fields). **Sign and verify must use an equal context.**
  - `CryptographicLibrary::generate_mldsa_key_pair` makes a real keypair; `sign_data` /
    `verify_signature` route `KeyAlgorithm::MLDSA` through it.
- **AEAD encryption** — AES-256-GCM, ChaCha20-Poly1305 (12-byte nonce), XChaCha20-Poly1305
  (24-byte nonce). 32-byte keys, 16-byte tags. `encrypt_data_with_algorithm()` selects the
  cipher; `decrypt_data` dispatches on the algorithm stored in `EncryptedData`.
  - Limitation: AAD is **not** persisted in `EncryptedData`, so `decrypt_data` cannot
    re-supply it — round-trips currently use `None` AAD. (Fixed by Task 5.)
- **Hashing** — SHA-256, SHA-512, BLAKE3 (32-byte digest); `compute_hash_blake3()`.
- **KDF** — HKDF-SHA256 (`KeyDerivation::derive_hkdf`).

### What is SCAFFOLDING ONLY (do NOT rely on — enum variants without real backends)

- `KeyAlgorithm::{Kyber, NTRU, SPHINCS, RSA, ECDSA}` — keygen returns random bytes; no real
  algorithm. (Tasks 8–9.)
- `ProofEngine` zk-SNARKs in `cryptographic_library.rs` — `generate_proof_data` produces
  SHA-256 commitments and `verify_proof_data` only re-checks the public-input hash (never the
  witness), so a proof is **forgeable**. Not sound, not zero-knowledge, despite the
  Groth16/PLONK/Halo2/Bulletproofs type names. A separate `zk_proofs.rs` does Pedersen-style
  *structural* validation — also not a real proof system. (Task 7.)
- `EncryptionAlgorithm::Custom(..)` — returns `UnsupportedAlgorithm`.

### Package facts

- Crate: `qualia-core-db` 0.0.13 (all six workspace crates are in lockstep at 0.0.13).
- Crypto deps (`crates/qualia-core-db/Cargo.toml`): `ed25519-dalek` 2.1, `sha2` 0.10,
  `sha3` 0.10, `aes-gcm` 0.10, `x25519-dalek` 2, `hkdf` 0.12, `blake3` 1,
  `chacha20poly1305` 0.10, `fips204` 0.4 (features `ml-dsa-65`, `default-rng`),
  `rand` 0.10.1, `hex` 0.4.3, `getrandom` (0.2/0.3/0.4).
- Build: `cargo build -p qualia-core-db` · WASM: `cargo build -p qualia-core-db --target wasm32-unknown-unknown`
- Test: `cargo test -p qualia-core-db --lib` · crypto only:
  `cargo test -p qualia-core-db --lib -- cryptographic_library:: fiduciary_crypto::`
- Tests live in the `mod tests` block at the bottom of each file; match the existing style.

**Invariant note (`CLAUDE.md` §6):** the "no `Vec`/`String`/`Box` in hot paths" rule is about
the zero-copy NQuin ABI in the graph engine. The `specialized_libs` crypto code uses
`Vec`/`String`/`HashMap` freely — it is *not* a hot path. Keep using `Vec<u8>` here; do not
refactor it to the NQuin ABI.

**WASM note:** this crate compiles to WASM. Every crypto crate in use (`blake3`,
`chacha20poly1305`, `fips204`, …) is WASM-compatible; entropy comes from `getrandom` with the
`js`/`wasm_js` features already configured. Add new crates with `default-features = false`
and re-run the WASM build after each change.

---

## Completed work — Tasks 1–4 (2026-06-15, v0.0.13)

Recorded here as a change log; the code is in place and tested.

- **Task 1 — BLAKE3.** Added the `blake3` crate; added a `"BLAKE3"` arm to
  `HashEngine::compute_hash` and the public `CryptographicLibrary::compute_hash_blake3()`.
  Test: `test_blake3_hash_computation` (empty-input known-answer vector + determinism +
  distinctness from SHA-256).
- **Task 2 — DID signing / stale comment.** `SparqlDidHandler::sign_with_did` now **fails
  closed** (returns `Err`) because the SPARQL query layer holds no private keys — it
  previously returned a forged all-zero 64-byte signature. Corrected the inaccurate "stub"
  doc comment on `webizen_identifiers.rs::verify_signature` (the body already used
  `ed25519-dalek`).
- **Task 3 — ChaCha20-Poly1305 + HKDF.** Added `chacha20poly1305`; `EncryptionEngine` now
  dispatches AES-256-GCM / ChaCha20-Poly1305 / XChaCha20-Poly1305 (nonce length per
  algorithm), exposed via `encrypt_data_with_algorithm()`; `decrypt_data` dispatches on the
  stored algorithm. Wired HKDF-SHA256 in `KeyDerivation::derive_hkdf`. Tests: round-trip,
  24-byte-nonce path, tamper-detection, and the RFC-5869 HKDF known-answer vector.
- **Task 4 — real ML-DSA.** Replaced the SHA3 *simulation* in `fiduciary_crypto.rs` with real
  ML-DSA-65 via `fips204` (removed the fake lattice helpers; key/sig structs now hold raw
  FIPS-204 bytes). `CryptographicLibrary` routes `KeyAlgorithm::MLDSA` keygen/sign/verify
  through it; Ed25519 remains for other algorithms. Tests: real keygen/sign/verify round-trip
  plus tampered-message and wrong-context rejection.

**Verification:** 21/21 crypto tests pass; host build and `wasm32-unknown-unknown` build both
green. (See the known-issues section for unrelated suite failures.)

**Docs updated alongside:** `ARCHITECTURE.md` §37, `docs/release-targets.md`, `CLAUDE.md` §8,
`README.md`, `docs/CRYPTO_STATUS_2026-06-15.md`, and (cross-repo)
`webizen-browser/QUALIA_DB_LOGIC_AUDIT.md` + `webizen-browser/QUALIADB_CRYPTO_STATUS.md`.

---

# PART 2 — The Remainder (Tasks 5–9)

> Tasks 1–4 are done. The tasks below are the remaining crypto work, ordered easiest→hardest.
> Each is self-contained. Line numbers drift as files change — anchor on function/struct names
> and re-grep. Same Definition of Done applies (host build + WASM build + tests + nothing fake
> presented as real).

---

## TASK 5 — Persist AAD so AEAD additional-data round-trips  ⭐ SMALL, do first

**Why:** `encrypt_data`/`encrypt_data_with_algorithm` accept `additional_data: Option<&[u8]>`,
but `EncryptedData` does not store it and `decrypt_data` passes `None`. So any caller using
AAD gets a decryption failure (this is why `test_chacha20poly1305_roundtrip` uses `None`).
Affects the AES path equally.

**Where:** `crates/qualia-core-db/src/specialized_libs/cryptographic_library.rs`.

1. Add a field to `EncryptedData`: `pub aad: Vec<u8>,` (empty = no AAD).
2. In `EncryptionEngine::encrypt_data_with`, set `aad: additional_data.unwrap_or(b"").to_vec()`.
3. In `EncryptionEngine::decrypt_data`, pass `Some(&encrypted_data.aad)` (or `None` if empty)
   into `decrypt_with_key` instead of the hard-coded `None`.
4. Update the ChaCha test to use `Some(b"aad")` and assert the round-trip succeeds; add a
   negative test where decrypt with a *different* AAD fails.

**Note:** AAD is authenticated, not encrypted — storing it in `EncryptedData` is fine and
normal. Verify, commit `feat(crypto): persist + authenticate AEAD additional data`.

---

## TASK 6 — Wire ML-DSA into Verifiable Credential issuance + multi-Quin signature storage

**Why:** The ML-DSA-65 *signing primitive* is real (Task 4), but nothing stores a 3309-byte
signature into the credential graph. Ed25519 fits in the WAL/metadata anchor; ML-DSA does not.

**Context to read first:** `ARCHITECTURE.md §37` (Fiduciary Crypto) and the VC section;
`wal.rs` (how Ed25519 proofs are anchored today); `provenance.rs` / `temporal_graph.rs` for
how multi-Quin linked structures are built.

**Steps:**
1. **Storage strategy.** A 3309-byte ML-DSA sig spans ~70 NQuin object fields (48 B each).
   Define a fragment layout: a head Quin (predicate `q_hash("vc:proof/mldsa")`, object =
   total length + fragment count) linked to N fragment Quins (predicate
   `q_hash("vc:proof/mldsa/frag")`, object = 48 raw bytes, ordered via the temporal/DAG
   `prev` link). Mirror the Merkle-DAG pattern already in `wal.rs`/`provenance.rs`.
2. **Issuer side.** Add `issue_vc_mldsa(claim_quins, issuer_sk)` that signs the canonical
   claim-graph bytes via `MlDsaSigner::sign_with_secret` and writes the head + fragment Quins.
3. **Verifier side.** Add `verify_vc_mldsa(head_quin)` that reassembles the fragments in
   order and calls `MlDsaSigner::verify_with_public`.
4. **Tests:** issue→verify round-trip over a small claim graph; tamper one fragment → fail;
   wrong issuer key → fail.

Effort: medium. Verify host+WASM. Commit `feat(crypto): ML-DSA VC issuance + fragment storage`.

---

## TASK 7 — Real zero-knowledge proofs (replace the SHA-256 commitment stub)  ⚠️ LARGE

**Why / current state (do NOT trust the existing code):** in `cryptographic_library.rs`,
`ProofEngine::generate_proof_data` only stores two **SHA-256 commitments**
(`H(circuit_id‖witness)`, `H(public_inputs)`) + a version byte, and `verify_proof_data`
**only re-hashes the public inputs** and compares — it never checks the witness commitment.
This is **not sound and not zero-knowledge**: a "valid" proof can be forged for any public
input with no witness. The Groth16/PLONK/Halo2/Bulletproofs enum names are cosmetic. A
*separate* `zk_proofs.rs` does Pedersen-style structural validation — also not a real proof
system; decide which module is canonical first.

**This is a multi-day project. Do not start without product sign-off.** Outline:
1. **Pick a backend.** Recommended: `arkworks` (`ark-groth16` + `ark-bn254`) for SNARKs, or
   `halo2_proofs` (no trusted setup) — both pure-Rust. **WASM is the gating constraint:**
   verify the chosen crate builds for `wasm32-unknown-unknown` *before* committing to it
   (proving is heavy in WASM; verification is the realistic in-browser path).
2. **Circuit compiler — the hard part.** Define how a Quin predicate/claim maps to an
   arithmetic circuit (R1CS for Groth16, or a PLONKish config for Halo2). Start with ONE
   concrete statement (e.g. "this VC claim's value is in range [a,b]" for selective
   disclosure) rather than a general compiler.
3. **Trusted setup / SRS.** Groth16 needs a per-circuit trusted setup; Halo2 needs a
   universal SRS. Document how the params are generated, stored, and distributed.
4. **Integrate.** Replace `generate_proof_data`/`verify_proof_data` bodies (keep the public
   `ProofEngine` API), and reconcile with `zk_proofs.rs`. Until a circuit exists, make the
   stub **fail closed** (return `Err`/`false`) rather than pretend-verify.
5. **Tests:** real soundness tests — a proof for a false statement must FAIL to verify; a
   proof with the wrong public inputs must FAIL; a valid proof must verify.

Verify host+WASM. Commit per milestone. Update `release-targets.md` lines marked
"Structural only" and `docs/CRYPTO_STATUS_2026-06-15.md`.

---

## TASK 8 — Real post-quantum KEM + alt signatures: Kyber / SPHINCS+ (NTRU optional)

**Why:** `KeyAlgorithm::{Kyber, NTRU, SPHINCS}` are enum-only; keygen returns random bytes.

- **ML-KEM (Kyber, FIPS-203):** use the sibling crate **`fips203`** (same author/shape as the
  `fips204` we already use). Add an encapsulate/decapsulate path (KEM, not signatures) — this
  is what's needed for post-quantum key exchange, complementing `x25519-dalek`.
- **SLH-DSA (SPHINCS+, FIPS-205):** use **`fips205`** for a hash-based signature alternative
  to ML-DSA. Wire it parallel to the ML-DSA path in `SignatureEngine` (branch on
  `key_algorithm`), mirroring Task 4's structure exactly.
- **NTRU:** no FIPS standard / weaker crate ecosystem — recommend dropping the enum variant
  rather than implementing, unless there's a specific requirement.

Each sub-item is independent. Confirm WASM builds. Tests: keygen + encap/decap (Kyber) or
sign/verify + tamper (SPHINCS+). Commit per algorithm.

---

## TASK 9 — RSA / ECDSA (low priority — only if interop requires it)

`KeyAlgorithm::{RSA, ECDSA}` are enum-only. These are classical (not post-quantum) and the
project already has Ed25519, so only implement if an external system requires RSA/ECDSA
interop. Crates: `rsa`, `ecdsa`+`p256`. Same `SignatureEngine` branching pattern as Task 4.
Otherwise, consider removing the variants to stop implying support.

---

## Definition of done (per task)
1. Code compiles for host: `cargo build -p qualia-core-db`.
2. Code compiles for WASM: `cargo build -p qualia-core-db --target wasm32-unknown-unknown`.
3. New tests pass: `cargo test -p qualia-core-db --lib`.
4. No fake/zeroed crypto left presented as real — either it's real, or it fails loudly.
5. Commit with a message naming the algorithm and the file. Bump crate version if releasing.

## Suggested order (remaining)
TASK 5 (AAD, small) → TASK 6 (ML-DSA VC wiring) → TASK 8 (Kyber/SPHINCS+, parallels Task 4) →
TASK 7 (real ZK, largest, needs sign-off) → TASK 9 (RSA/ECDSA, only if required).

---

## Known pre-existing test failures (NOT caused by the crypto work)

The full `cargo test -p qualia-core-db --lib` suite reports ~26–28 nondeterministically
failing lib tests. They are **pre-existing and unrelated to this crypto work** — host and
WASM builds are clean, all crypto + `sparql_did` tests pass, and every failing test is in a
subsystem the crypto pass never touched.

The full list, root-cause triage, and a suggested first fix live in the canonical tracker:
**[`KNOWN_ISSUES.md`](KNOWN_ISSUES.md)**. (Kept out of this plan to avoid two drifting copies.)
