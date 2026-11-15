# QBS-03 paired COSE candidate contract

12 November 2026. Implementation candidate v1; independent vectors, feature/build evidence, resource measurements and composition review remain open. This is a strict application profile, not a general COSE implementation or a new registry allocation. Legacy dual_sign/mldsa/ed25519 adapters remain byte-for-byte unchanged.

## Standards checked

- [RFC 9052 sections 4.2, 4.4 and 9](https://www.rfc-editor.org/rfc/rfc9052.html): tagged Sign1 uses tag 18, and signing bytes are deterministic CBOR [Signature1, protected_bstr, external_aad_bstr, payload_bstr]. Both members need their own protected bytes; their payload and external AAD are identical.
- [RFC 9964 section 5](https://www.rfc-editor.org/rfc/rfc9964.html#section-5): ML-DSA-65 is -49, public/signature sizes are 1952/3309 bytes, and primitive ctx is empty. Use fips204 directly with empty ctx; no legacy context compression or prehash mode. Public COSE_Key is AKP (kty 7). No private COSE_Key serialization is introduced.
- [RFC 9864 section 2.2](https://www.rfc-editor.org/rfc/rfc9864.html#section-2.2): use fully specified Ed25519 -19, ordinary Ed25519 over Sig_structure, not -8 or Ed25519ph. Verification uses the provider's strict verification policy.

## Exact bytes

All values have definite lengths and shortest encodings. Map keys occur exactly in the order below; extra, duplicate, out-of-order, nonminimal, truncated and trailing encodings fail. No floats, indefinite objects, recursive generic decoding or live URL resolution.

Outer envelope is the CBOR array of exactly two tagged Sign1 objects, ML-DSA first and Ed25519 second:

    [18([bstr(protected_mldsa), {}, bstr(binding), bstr(sig3309)]),
     18([bstr(protected_ed),    {}, bstr(binding), bstr(sig64)])]

Each protected map is exactly {1: algorithm, 4: bstr(key_id48)}. No extra protected/unprotected headers are accepted, including crit: this profile defines no extensions. Thus unknown critical headers fail before cryptography. Missing tag, extra/nested tag, nil/detached payload, swapped/duplicate algorithm, mismatched payload or key role fail. All profile-required fields are protected or in the signed payload.

External AAD is the exact ASCII bytes qbdl-stream-pq-1/paired-sign1/v1, with no terminator; it is fixed by this profile and is never taken from the envelope. Primitive ML-DSA context is empty and is distinct from this COSE external AAD.

Binding is exactly this seven-entry map:

    {0: "qbdl-stream-pq-1", 1: 1, 2: bstr(descriptor_digest48),
     3: [[-49, bstr(mldsa_key_id48)], [-19, bstr(ed25519_key_id48)]],
     4: controller_epoch_uint64, 5: bstr(purpose), 6: bstr(audience)}

Purpose is 1..64 bytes and audience 1..128 bytes, both opaque application bytes without implicit text normalization. Epoch is u64, including zero (its authorization semantics belong to QBS-04). Descriptor digest is the exact 48-byte digest supplied by the descriptor owner; this adapter does not silently rehash or invent its domain. Verification requires a complete caller-expected binding, so all fields must match before a proof result is returned.

Key IDs are SHA-384 of ASCII QBDL-COSE-KEY-ID-V1 followed by one zero byte followed by the exact deterministic public COSE_Key below. These are application-domain key references, NOT RFC9679 thumbprints:

    ML-DSA: {1: 7, 3: -49, -1: bstr(public1952)}
    prefix hex a3 01 07 03 38 30 20 59 07 a0
    Ed25519: {1: 1, 3: -19, -1: 6, -2: bstr(public32)}
    prefix hex a4 01 01 03 32 20 06 21 58 20

Raw typed arrays are the external public-key API; no generic COSE_Key parser is added in this wave. IDs are always recomputed from the role-specific public keys and checked against both binding and protected kid. Public signing consumes a ManagedPairedSigning capability admitted by the existing CryptographicLibrary/KeyManager. It binds exact key IDs, algorithms, policies, grant epoch/generation and the complete expected COSE binding. The primitive profile uses decoded provider key owners scoped to that capability; no serialized private key is exported or cloned. Arbitrary-key signing is a cfg(test), crate-private fixture helper.

## Interface and limits

The wire adapter lives under crypto/network/cose/. The reuse correction adds explicit raw profile and managed-key adapters inside the existing cryptographic_library; root owns parent routing. Public interfaces encode_binding/decode_binding, encode_pair/decode_pair, sign_pair/verify_pair, PairedPublicKeys::key_ids and explicit caller Workspace/PairSignatures reuse bounded borrowed/caller-owned storage. Parsed values are explicitly unverified. VerifiedPair has a private constructor and only indicates both signatures and exact expected binding matched; it is not an authority or resource grant.

Binding limit 512 bytes; protected limit 64 bytes; signing structure limit 704 bytes; envelope limit 4608 bytes. These tighter proof-binding bounds do not change the architecture's descriptor body 64-KiB/complete proof envelope 96-KiB bounds. All output capacity is checked, lengths use checked arithmetic or profile-capped values, and borrowed parsing creates no heap collections. Workspace carries binding, Sig_structure and signature buffers. Provider internal stack and allocation behavior still require measured evidence; this implementation makes no measured zero-heap claim. Errors reuse CryptoError. A failed encode/sign call leaves output invalid and caller must discard it; no prefix is a proof.

## Trust and release limits

Caller-supplied public keys do not establish publisher authority. QBS-04 must independently pin the full pair or verify accepted PQ controller chains and current purpose/audience/expiry/revocation/epoch before protected use. This layer performs no network fetch, root enrollment, rotation, credential acceptance or permission minting. Payment and transport security never satisfy this boundary.

The fixed profile is a candidate until an independent implementation confirms exact encoding and primitive vectors, malformed mutation corpus and provider/version/errata review. Tests authored here are local regression sources only until the root executes them. Existing cose_sign1_frozen() remains false; this wave must not change historical qualification flags.

## Implementation handoff

Candidate source added under crypto/network/cose only. Nine local regression tests authored for canonical/max-size binding, malformed binding, Sig_structure framing/capacity, every truncated envelope prefix, role/header/critical attacks, payload/kid mismatch, both actual signature checks, caller scope/digest/epoch/key mismatch, and legacy-format rejection. These are NOT independent vectors. Root owns module routing and build execution.

Executed by this worker: source inspection, primary RFC lookups, SHA-256 preservation check of dual_sign.rs/mldsa.rs/ed25519.rs, and rustfmt --edition 2021 crates/qualia-core-db/src/crypto/network/cose/mod.rs (exit 0). No Cargo build/test run. Existing providers observed in lock/cache are fips204 0.4.6 and direct ed25519-dalek 3.0.0; comprehensive provider errata review remains open. Borrowed fips204 private owner has ZeroizeOnDrop; no new secret copy owner was added.

Root commands, using the single owned target/build configuration:

    cargo test -p qualia-core-db --lib --features qdnf,pq-kem --locked --offline crypto::network::cose::tests
    cargo test -p qualia-core-db --lib --features qdnf,pq-kem --locked --offline crypto::network::dual_sign::tests

Formatted candidate fingerprints (source state before independent review):

| File | Lines | SHA-256 |
|---|---|---|
| mod.rs | 94 | 310c1dae5c9ef9b2ab01544d9ee95ffa8fb7821cc9e0902217723a5eeaa66971 |
| cbor.rs | 132 | a1d0ba300dce41dce129371bf412a622dba5527156d76263959bf86ac123c94e |
| binding.rs | 120 | 5c2818d35899c0ae012df0f5360690577c8099f8bf48204c80bc3137c047b6ad |
| envelope.rs | 198 | 1e2719a29fa26dd0e5b93698d12184c9c5ffda9fe489d0b80faafd2868f3ac2d |
| signing.rs | 127 | edceffcd7d4507c686a864623b7f8d755951c65c769a21dd8a5cf19174365d06 |
| tests.rs | 337 | 370b520f3858f66cd31e4478eb49362070e105023f3e219ef723fb814954e584 |

## Managed-library routing amendment (12 November 2026)

Wire bytes, algorithms, AAD, canonical keys and empty ML-DSA primitive context remain unchanged. COSE now delegates raw primitive sign/verify to cryptographic_library/signing_profile.rs; legacy sign_data/contextual methods are unchanged. Public sign_pair requires a single-use ManagedPairedSigning from KeyManager or CryptographicLibrary. Both named policies must explicitly allow signing; algorithm/type/ID and key validity are checked, and a trusted local host supplies authenticated methods, IP and a side-effect-free Unix clock. Caller-claimed wire methods are never authentication. New raw signing primitives are crate-private; public verification does not mint authority.

The capability immutably borrows its existing key owner, preventing key/policy replacement while it exists. A shared SigningControl provides terminal cancellation and monotonic authority-generation fencing. Final publication locks that control, samples time, rechecks policy/key/grant validity and then encodes the bounded output. Cancellation/generation updates during primitive signing prevent publication; updates after successful publication do not retroactively erase already-issued signatures. The capability is consumed; scratch signatures are cleared on returned errors. The trusted host must connect its revocation owner to this control; merely constructing a control or ProviderGrant is not a semantic execution permit.

Cold provisioning reuses the existing algorithm-specific KeyGenerator and existing KeyManager store, returning only public keys. It does not use the generic random32-and-resize rotator, and does not claim enrollment/rotation authority. KeyVault remains an existing custody owner; no second vault is introduced and no unnecessary vault API was added in this correction. Historical candidate fingerprints below/above predate this amendment. Tests added for policy absence/denial/auth/date/IP, algorithm/ID/binding mismatch, cancel/generation/deadline and successful managed COSE verification; execution belongs to root.

## Verification/custody source boundary

The combined cose/signing.rs has been split without changing profile bytes or public production APIs. types.rs owns fixed constants, Binding/KeyIds/PairSignatures and Workspace; verify.rs owns VerifiedPair and verify_pair; raw_signing.rs owns internal primitive preparation plus cfg(test) raw fixture signing; managed_signing.rs alone imports ManagedPairedSigning and exports production sign_pair. Production mod.rs routes all modules. A focused consumer/harness can compile the actual codec/types/verify/central signing_profile sources without importing managed custody; fixture tests add the actual raw_signing source. No production raw-signing feature bypass or replacement key-manager/NQuin stubs are introduced. The original nine COSE regressions remain in tests.rs. rustfmt --check passed after the split; no build/test execution by this worker.
