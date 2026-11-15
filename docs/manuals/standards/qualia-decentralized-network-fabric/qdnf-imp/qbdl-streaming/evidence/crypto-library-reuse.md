# Crypto-library reuse audit — QBDL streaming

Source inspection on 2026-11-12; baseline HEAD `afbe431195167fde0fadc233ed79fd2fe85f73f0`. This is a targeted reuse correction, not a qualification result. No builds, tests, dependency changes, or source edits were performed for this audit.

**QualiaDB already has a real cryptographic library and key owners.** The new COSE code should be a bounded wire/profile adapter over that library. Its direct provider calls implement the required bytes, but bypass the repository's central signing/key-management façade. Correct that routing before adding another crypto subsystem. The new public-pin checks must compose with existing grant/session owners, not replace them.

All source paths below are relative to `crates/qualia-core-db/src/`.

## Search coverage

Read `AGENTS.md`, `crypto/DIRECTORY_INDEX.md`, and `specialized_libs/cryptographic_library/DIRECTORY_INDEX.md` before absence claims. The generated indices predate this work, so actual source is authoritative. Enumerated crypto-related Rust files across `crates/`; searched all Rust sources there for COSE object/profile names, then inspected matched implementations and relevant signing, DID, native-authority, key-manager, QSession, provider, rotation, and lease sources. Also inspected `crates/vibe/src/crypto.rs` (value/encoding/hash helpers, not the central signing owner).

The repository-wide COSE search found the new adapter and legacy network documentation/`cose_sign1_frozen()`; it did not identify an older paired COSE implementation under those names. This is a scoped search result, not a claim that cryptography or authority was absent.

## Existing APIs and exact compatibility

| Existing owner/API | Observed behavior and reuse consequence |
| --- | --- |
| `specialized_libs/cryptographic_library/library.rs:140` `CryptographicLibrary::sign_data`, `:172` `verify_signature`; module reexports in `mod.rs:32` | Central library already routes key IDs through its key manager and signature engine. Retain this façade and legacy behavior. Its allocating result/metrics/key-copy path is a cold API, not the bounded streaming signing path. |
| `.../signing.rs:216` `SignatureEngine::sign_data`, `:329` `verify_signature` | ML-DSA delegates to real `MlDsaSigner` with a fixed fiduciary context. The Ed25519 fallback signs/verifies SHA-256(data). Neither profile is interchangeable with raw COSE Sig_structure signing. |
| `crypto/fiduciary_crypto.rs:121` `MlDsaSigner::generate_keypair`, `:196` `sign_with_secret`, `:219` `verify_with_public` | Real FIPS-204 ML-DSA-65. `derive_ctx` at `:186` always produces a SHA3-512 context (64 bytes), even if textual context fields are empty. COSE's empty primitive context needs a separately named profile API. Preserve historical signatures. `MlDsaKeyManager::get_signer` at `:293` already provides a managed signer owner. |
| `crypto/network/mldsa.rs:36,53`; `ed25519.rs:11,26`; `dual_sign.rs` | Existing bounded network adapters are real. ML-DSA hashes its length-delimited context to 48 bytes; Ed25519 adds network framing. They cannot be called and relabeled as COSE. Reuse their error/size conventions, not their signed bytes. |
| `identity/key_vault.rs:391` `KeyVault::sign_payload`, `:409` `verify_signature` | Already provides raw Ed25519 payload signing. Signing accepts a caller-provided SigningKey; it is not by itself an owner-scoped authorization boundary. Verification uses `verify`; the new COSE candidate deliberately uses strict verification. Keep that stricter profile requirement explicit. `derive_key_for_role`/`request_role_seed` at `:361,369` and native-secure vault construction are existing custody integration points. |
| `.../key_management/manager.rs:46,75,80` `KeyManager::{generate_key_pair,get_key,rotate_key}`; `generator.rs:203,245,284` | Real algorithm-specific key generation and public-key derivation exist, including ML-DSA. `KeyStorage::get_key` clones the key. `get_key_with_access` at `manager.rs:174` checks policies only when policies exist. A strict COSE owner path must explicitly require admission rather than inheriting this optional-policy default. |
| `.../key_management/rotation.rs:138` `KeyRotator::rotate_key` | This implementation fills 32 random bytes and resizes to the prior key length while preserving its algorithm label. It is not valid ML-DSA successor key generation. Route a paired successor through the existing algorithm-specific generator, then enforce the existing network epoch/rotation checks; do not call this generic operation and claim a valid paired rotation. |
| `net/qdnf/authority/owner.rs:111,185,231` `AuthorityOwner::{install_grant_with_ceiling,issue_permit,current_grant}` | Existing full-digest principal/recipient/purpose/instrument/scope/resource owner and revocation/generation checks. It remains the semantic permission owner. A verified publisher pin is evidence for that owner, not an ExecutionPermit. |
| `crypto/network/key_provider/grant.rs` `ProviderGrant::authorize`; `rotation.rs:24` `RotationState::paired`; `net/peer/runtime/leases.rs:36,91` `LeaseTable::{acquire,release}` | Existing purpose/controller/deadline/generation checks, monotonic epoch admission, and generation-fenced lease slots. New authority code already reuses them. Keep cell/epoch association and late-completion rechecks at their composition boundary. |
| `net/qdnf/crypto/handshake/shares.rs:61,75,95`; `crypto/schedule.rs:22`; `crypto/finished.rs:42`; `session/handshake/mod.rs:60` | Existing initiator/responder hybrid shares, transcript-bound HKDF-SHA384 directional schedule, directional Finished verification, and `SessionBinding::from_permit`. Integrate these; do not add a second handshake or manufacture installed keys from pin verification. |

The general library also owns encryption/hashing/proofs and HKDF; this audit does not propose replacing them. The inspected `EncryptionEngine` allocates result vectors. The existing network KDF/AEAD path is the bounded profile layer. `KeyVault::encapsulate_for_recipient` (`identity/key_vault.rs:756`) is explicitly ephemeral X25519 + AES-GCM, not the hybrid QSession exchange.

## DID and trust correction

`identity/did_qi.rs:20` **already defines a PQ-bound GenesisPayload**: controller Ed25519 key, ML-DSA-65 hash, session Ed25519 key, X25519 key, ML-KEM-768 hash, nonce. `canonical_genesis_bytes` and `did_qi_from_genesis_payload` bind those fields. Earlier “classical-only DID” wording must mean insufficient *classical-only proof evidence*, never that the did:qi architecture lacks PQ binding.

The separately inspected current `did_qi/document.rs:43` document representation has a 32-byte controller key; `did_qi/method.rs:36–54` signs/verifies Ed25519 over its domain-separated document digest. These functions alone do not demonstrate a verified full paired-key controller chain. Connect future chain verification to the existing genesis/document/transition owners and verify the complete PQ key preimage, epoch and relationship; keep unsupported-chain refusal until that composition is implemented and independently tested.

Existing native project authority is also real: `poet_host/native_authority/enrollment/signer.rs:41` derives a signer from a native-secure unlocked KeyVault; `enrollment/owner.rs:337` checks it against the installed project anchor before signing admission. This is valuable custody/admin precedent, but its Ed25519 project pin is not an interchangeable FULL ML-DSA+Ed25519 publisher pin.

## Concrete next refactor (proposed APIs, not existing claims)

1. Add a small explicit raw-signature profile surface **inside the existing cryptographic library**, e.g. `signing_profile.rs`, reexported by its current module. Provide bounded `sign_mldsa65_empty_context_into` / `verify_mldsa65_empty_context` and `sign_ed25519_raw_into` / `verify_ed25519_raw_strict`, using existing FIPS-204/dalek providers, borrowed typed keys and fixed signature arrays. No message prehash, network wrapper, new algorithm implementation, or changed default.
2. Make `crypto/network/cose/signing.rs` call that profile surface; leave CBOR, headers, Sig_structure and paired binding in COSE. Replace the public arbitrary-secret-pair entry point used by production with a **managed borrowed signing capability** obtained from the existing key owner. Keep a low-level borrowed-key helper private/crate-private for adapter tests. Independent raw signature fixtures must continue to pass.
3. Add the minimal key-owner operation for that capability: algorithm-checked, policy-required, generation/deadline-fenced borrowing/signing by key ID, with fixed output buffers and no cloned/exported secret material. Have `CryptographicLibrary` route to its current `KeyManager`; use the existing vault/managed signer for custody. Add an owner-scoped raw Ed25519 role operation to KeyVault rather than treating `sign_payload(&external_key,...)` as admission. Do not automatically trust caller key IDs or derive a PQ authority from a classical role key.
4. Treat the new `PinAuthority` as a bounded **publisher public-key trust registry/profile adapter**. It holds public keys, not a competing private-key vault. Connect its receipts to the existing AuthorityOwner/ProviderGrant and revocation owners; do not let the registry independently issue semantic grants, installed session keys or stream execution permits. Local pin installation stays an explicit bootstrap-admin boundary. Persistence and chain resolution belong to the established host/DID owners.
5. Retain current canceled/late-operation generation checks, but consume them in the actual managed-key operation before releasing a signature/key result. The current `KeyCompletion` is metadata only. No handshake/secret-delivery qualification follows from it.

Next source claims should therefore include the central library profile/reexports, minimal existing key-owner/vault adapters, and the COSE call site; root must coordinate those shared files before edits. Tests should cover raw-profile independent vectors, old-profile compatibility, mismatched algorithm/key ID, absent policy, revoke/cancel/rotation between admission and completion, and no secret export. No report finding authorizes changes to unrelated general-library rotation users.

## Evidence limits

Current COSE and authority files are uncommitted candidates. Source inspection distinguishes “real API exists” from “its composition has passed qualification.” This report records no executed test result. Persisted pin antirollback, complete verified PQ controller chains, independently driven hybrid sessions, and production key-owner routing remain integration work.

Inspected working-tree SHA-256 fingerprints (not commit identities):

- `crates/qualia-core-db/src/specialized_libs/cryptographic_library/signing.rs`: `b3457288c48321676788e1908abf927b216799671097cb4315176b552017aec1`
- `crates/qualia-core-db/src/specialized_libs/cryptographic_library/key_management/rotation.rs`: `622dce491f8e0a9e32feac390e3bb9ce429db6f99f1374722ae1a23ecccd341d`
- `crates/qualia-core-db/src/identity/did_qi.rs`: `4f4329bdb8c338ebfe3ed32b0e97196cce919256ca7f96c1f94812158d05b5e4`
- `crates/qualia-core-db/src/crypto/network/cose/signing.rs`: `edceffcd7d4507c686a864623b7f8d755951c65c769a21dd8a5cf19174365d06`
- `crates/qualia-core-db/src/crypto/network/authority/pins.rs`: `2742f55891fb4451e6189d86c3a64dd291e8c96d99532e96145d7f4832de5995`

## Implemented reuse correction — source handoff

The proposed central routing is now implemented in candidate source. Public COSE signing consumes ManagedPairedSigning from the existing KeyManager/CryptographicLibrary. Arbitrary-key signing exists only as a cfg(test), crate-private fixture helper. The central profile functions use the existing FIPS-204/dalek primitives. New strict policy evaluation uses existing AccessPolicy/AuthenticationMethod types and requires trusted host-authenticated context; it does not authenticate request claims. Existing policy checkers retain their prior semantics.

KeyManager exposes only an internal borrowed storage lookup; no serialized secret clone/export is introduced by this path. The capability retains an immutable owner borrow and scoped decoded provider keys. The central facade provisions via the existing algorithm-specific generator and store, returning public keys only. No separate vault or generator was added. Generic rotation behavior is unchanged.

SigningControl is a local operation fence, not an authority grant. Its terminal cancellation and monotonic authority generation are checked at admission, before signing and under the final output-publication lock. The final clock sample occurs while holding that lock; trusted clocks must not reenter the control. The host must connect this control to its existing revocation owner. PinAuthority metadata receipts, signed controller chains, KeyVault-backed paired custody and actual session/key delivery remain separate integration work.

Authored tests: exact policy absence/denial/auth/date/IP and positive admission; managed algorithm/key-ID/binding checks; cancellation and generation fencing; post-sign deadline expiry clearing signature scratch without changing output; and actual managed paired-signature roundtrip. rustfmt --check passed for owned Rust files (excluding parent-recursive module checks). No Cargo build/test was run by this worker. Root owns module routes and execution.

Post-correction candidate working-tree fingerprints:

| Source under crates/qualia-core-db/src | Lines | SHA-256 |
| --- | ---: | --- |
| `specialized_libs/cryptographic_library/signing_profile.rs` | 58 | `9b716a882992a32bf81c7e8cf8549412f656ffc2bbdd76af72f6e4ae72e54aea` |
| `specialized_libs/cryptographic_library/key_management/managed_signing.rs` | 443 | `3b5774f3227efc814177b3abc4a42ba0950f78f39c6e8f6ef65bde0adaa8a339` |
| `specialized_libs/cryptographic_library/key_management/access/managed_policy.rs` | 100 | `2b9337999fa09b35b4e13e21de1b01663f06b9bc3db813dc0ed0e55d406b06b9` |
| `specialized_libs/cryptographic_library/library/paired_signing.rs` | 67 | `723d8a62d8efb93806211b994d8658a706b2080ba3ded07def765ed776beb2c5` |
| `specialized_libs/cryptographic_library/key_management/manager.rs` | 233 | `073ddf442cf4bf00daeb3ddabd564ab2a37f22d52391d74f80401b9ffa3ed204` |
| `crypto/network/cose/signing.rs` | 150 | `826da36175890ce384fb87ad9ee516f9df90fac488eb958607f805d6f24f6b55` |
| `crypto/network/cose/mod.rs` | 96 | `368c3c27f0d84bdc6cfabfa1ca854665da32b44abf49cb2bc08ca84b04d59980` |
| `crypto/network/cose/tests.rs` | 337 | `b7a33a8929cc7234d8c46cf292fcf88950edd39460eb2cf43cdb1f6ebf2ccd30` |

## Verification/custody source boundary

The combined cose/signing.rs has been split without changing profile bytes or public production APIs. types.rs owns fixed constants, Binding/KeyIds/PairSignatures and Workspace; verify.rs owns VerifiedPair and verify_pair; raw_signing.rs owns internal primitive preparation plus cfg(test) raw fixture signing; managed_signing.rs alone imports ManagedPairedSigning and exports production sign_pair. Production mod.rs routes all modules. A focused consumer/harness can compile the actual codec/types/verify/central signing_profile sources without importing managed custody; fixture tests add the actual raw_signing source. No production raw-signing feature bypass or replacement key-manager/NQuin stubs are introduced. The original nine COSE regressions remain in tests.rs. rustfmt --check passed after the split; no build/test execution by this worker.

Post-split source fingerprints (supersede the removed combined signing.rs entry):

- `crypto/network/cose/mod.rs` (26 lines): `42034db71179991f30e66c9d274b6486c003e5c4c0501f228b2542647c89d08d`
- `crypto/network/cose/types.rs` (76 lines): `4c9d4442e4d777c4a32e369ef2d644e0471f568a0bc17f6ff41d05b7316906ef`
- `crypto/network/cose/verify.rs` (58 lines): `737cc8a308aaca63c083876d87b243043e42640d0e270e70c991389111f13bc3`
- `crypto/network/cose/raw_signing.rs` (77 lines): `b362000ffbf21aafa857aad67488af28952465c79950c5395c8278c2b141a074`
- `crypto/network/cose/managed_signing.rs` (27 lines): `d0c52abd9fb67dea4234bd4b97ed6d1b24dbab1fcad21388c9adc2f368fe6c71`
