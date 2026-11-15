# QBS-00 security baseline

Date: 12 November 2026. Author: QBS-00 security inspection worker. Status: source inventory, not cryptographic qualification or package acceptance.

## Scope and provenance

Source HEAD observed: `afbe431195167fde0fadc233ed79fd2fe85f73f0` (QualiaDB). The integrator's [baseline-revisions.json](baseline-revisions.json), captured 2026-11-12T08:23:43.837496+00:00, records branch, tracked dirty paths and their SHA-256 fingerprints. Its tracked dirty set has no crypto/network or net/qdnf/economics path. Adjacent Poet native-authority edits are explicitly outside this inspection and must not be overwritten. Inspected-source byte fingerprints are appended below; HEAD alone is insufficient to identify a dirty workspace.

Read the repository AGENTS.md, streaming architecture, qualification requirements, and QBS-01/03/04/07/10/16 registry entries. This worker ran source/file discovery and reads only: no builds, tests, dependency changes, key generation, network calls, signatures, publication, payments or live-provider effects. The sole authored file is this report. Build evidence belongs to the integrator; none of the tests named below is claimed executed here.

Inventory discipline: searched tracked file inventory for FUNCTIONS/function-index/AGENTS and security owners, read crypto/network/mod.rs as the actual module registry, and searched Hypermedia's existing audit-docs/FUNCTIONS.md for the relevant names. That cross-repository index does not establish completeness for QualiaDB. No whole-tree absence claim follows from it. Initial path probes for net/qdnf/policy and a literal Windows identity wildcard were unsuccessful; they are discovery limitations, not evidence that policy or identity APIs are missing. Parent QDNF owner inspection remains authoritative for those integrations.

## Reusable implementations and precise boundaries

Paths below are relative to QualiaDB's crates/qualia-core-db/src unless stated otherwise.

| Existing implementation | What can be reused | What it does not establish |
|---|---|---|
| crypto/network/dual_sign.rs: DualProof, sign_dual, verify_dual | Fixed ML-DSA-65 and Ed25519 signature arrays; both algorithms must verify; missing/invalid Ed25519 is refused. | The module expressly marks CRY-02.05 PARTIAL; cose_sign1_frozen() returns false. It is not a COSE serializer/parser or a trusted authority verifier. |
| crypto/network/mldsa.rs: generate_keypair, sign, verify | Existing fips204::ml_dsa_65 provider, strict fixed public/private/signature byte types. | sign/verify compress caller context with SHA-384 over QDNF-MLDSA-CTX-V1 length framing before using FIPS-204 context. Passing an empty caller context still does not mean empty primitive context. Re-labeling these signatures COSE is not valid interoperability evidence. |
| crypto/network/ed25519.rs: sign_with_context, verify_with_context, public_from_seed | Existing ed25519-dalek provider and contextual legacy adapter. | It signs a QDNF-ED25519-CTX-V1 length-framed message in a 512-byte local buffer, not a COSE Sig_structure. The bounded message limit must not silently become a descriptor-size limit. |
| crypto/network/pq_handshake.rs: transition, application_data_allowed, initiator share encode/decode | Exact-length initiator encoding, successor-state guard, no initial 0-RTT, distinct qlink/qsession domains; classical_only_authority_is_enough() is false. | A boolean refusal policy and caller-supplied FSM state are not evidence of verified controller authority, Finished MAC confirmation, or a complete live handshake. Its opening comment is explicitly a draft. |
| crypto/network/share_encoding.rs: encode_responder_share, decode_responder_share, hybrid_ikm | Exact 1088-byte ML-KEM ciphertext plus 32-byte X25519 public share; ML-KEM-then-X25519 concatenation; bounded exact-length errors. | Draft shares have no version prefix. Its live KEM test uses OS randomness and is explicitly not a frozen independent vector. The pq_handshake opening comment about responder encoding is older than this real responder implementation. |
| crypto/network/kdf.rs: hkdf_sha384, hmac_sha384, hmac_sha384_verify, hybrid_shared_secret | Caller-buffered HKDF/HMAC-SHA-384, constant-time provider MAC verification, concatenating hybrid combiner with zeroized temporary IKM. | No arbitrary derived bytes become an authorized session exporter or recipient wrapping protocol merely by calling HKDF. Binding, transcript, domains and confirmed-session ownership remain composition work. |
| crypto/network/key_provider/grant.rs: ProviderGrant::authorize | Checks revoked, exact KeyPurpose, controller, now >= deadline, and authority generation. | Fields are public and the grant is Copy. authorize does not verify a signature or PQ chain; it does not inspect cell_epoch. Treat this as a checked local record after trusted admission, never a wire capability. |
| crypto/network/key_provider/software.rs: SoftwareBackend::sign_ed25519 | Enforces ControllerSign grant scope before invoking existing contextual signer; X25519 public derivation. | This inspected backend is not a complete opaque paired-key provider. Hardware signing explicitly delegates to an unsupported capability path; no hardware success is claimed. |
| crypto/network/secret_lease.rs: SecretLease | Non-Copy 64-byte secret lease; purpose checks, explicit destroy, Drop zeroization and redacted Debug. | Suitable building block for a 32-byte DEK; not storage for multi-kilobyte ML-DSA/ML-KEM private keys or a generation-fenced asynchronous provider handle. expose returns bytes; ownership must restrict its callers. |
| crypto/network/aead.rs: encrypt_in_place, decrypt_in_place | Caller-buffered ChaCha20-Poly1305 with separate 16-byte tag and fixed 12-byte nonce. | Decryption explicitly leaves buffer contents unspecified on failure. Object-stream code must own a private output lease, erase/quarantine failures, and never expose plaintext before successful authentication. Primitive AEAD is not a nonce allocator, indexed object format or crash-safe lifecycle. |
| crypto/network/chunks.rs: ChunkTable | Existing flight ceilings: 16 KiB/16 chunks, 32 global pending and 2 per locator; ordered digest, Replay/Conflict and unchanged duplicate deadline. | Crypto-flight bounds must stay independent of 64-KiB descriptor / 96-KiB envelope admission. Do not inflate a live handshake limit to accommodate metadata. |
| net/qdnf/economics/obligation.rs: Obligation, bind_obligation_id, remaining_recovery, authorise_discharge | Non-Copy single owner; SHA-384-bound identity; checked finite recovery arithmetic; S+W+H <= T contract; 16 retained operation records with explicit Capacity. | The inspected owner is an in-memory record; the word durable in its type documentation is not a demonstrated disk transaction protocol. It does not establish >16 persistent deduplicated streaming operations. |
| net/qdnf/economics/consent.rs: ConsentBudget, apply_payment_to_consent, recover_ambiguous | Payment leaves consent unchanged; payment cannot enlarge host capacity; ambiguous work keeps the same TxId; resource-kind/double-charge checks. | These pure checks do not authenticate incoming consent or create a streaming billing basis. |
| net/qdnf/economics/adapter.rs: settle_in_process, settle_aggregate | Existing finite owner integration, capped aggregation and typed SettlementReceipt. | LiveStripe/LiveIlp return Unsupported. Transport ACK, transferred bytes and authenticated plaintext must not be relabeled settlement or custody. No live rail was exercised. |

Governance discovery also found real ConsentGrant signing/revocation and authorize_category_access in governance/consent_contract.rs (including stateful grant/revocation handling). This baseline did not inspect its entire signature/trust chain or qualify it for PQ streaming; it must not be replaced or treated as missing. QBS-07 needs an adapter into the parent policy/Share owner, not an invented consent primitive.

## Feature and test inventory (unexecuted)

The crate manifest declares qdnf = [] and pq-kem = [thiserror]; fips204 with ml-dsa-65/default-rng, ed25519-dalek, x25519-dalek, zeroize, and ChaCha20-Poly1305 are existing dependencies. crypto/mod.rs exports network and separately gates another module on pq-kem. The exact consuming feature closure must be checked with the integrator's qdnf,pq-kem build; do not infer default/WASM/no-default success from source presence.

Existing focused test selectors worth preserving in that real package build:

- crypto::network::dual_sign::tests: generate_sign_verify_ok, wrong_ed25519_sig_is_unauthorized, wrong_context_is_unauthorized, stripped_ed25519_is_unauthorized, and the honest cose_sign1_is_not_frozen assertion.
- crypto::network::mldsa::tests::network_context_is_length_delimited and ed25519::tests::context_mismatch_fails.
- pq_handshake::tests: successor/reverse/skip transitions, zero_rtt_application_data_forbidden, exact share framing, hybrid secret order, domain separation, and classical_only_authority_rejected.
- share_encoding::tests: patterned framing, truncated/malformed/capacity, hybrid order, and live_ml_kem_encapsulate_then_encode_recovers_ct. The last is a provider round-trip, not independent protocol interoperability.
- key_provider::grant::tests::wrong_purpose_is_rejected, software hardware-unsupported test; secret_lease wrong-purpose/destroy tests.
- aead::tests::round_trip_and_aad_mismatch; kdf hybrid/HMAC negative checks; chunks capacity/replay/conflict/expiry tests.
- economics::obligation tests for checked remaining recovery, SHA-384 identity, zero-identity refusal, bounded discharge and funding; economics::consent payment/capacity/TxId tests.

Test-source existence is not execution evidence, vector independence, constant-time validation, measured zero allocation, workspace-stack accounting, or an external cryptographic review.

## Package-specific missing evidence

| Package | Concrete next evidence required |
|---|---|
| QBS-03 | Exact deterministic-CBOR paired COSE bytes, protected headers, key encodings, Sig_structure and primitive context/external-AAD policy; independently generated -49/-19 positives and mutation negatives; feature/dependency/errata and workspace evidence. Preserve legacy dual_sign framing and its honest unfrozen flag until a separately reviewed profile is implemented. |
| QBS-04 | Full paired-key pins or independently trusted PQ controller chains, audience/purpose/validity/revocation/rotation and downgrade tests; scoped nonforgeable key handles with late-completion fencing; integrated real hybrid handshake and Finished confirmation. No existing source inspection here establishes those release gates. |
| QBS-07 | Authenticated semantic closure and Share-to-local-grant compilation; unknown-critical-field and bounded graph/page rejection; current policy check at disclosure and consumer commit; forged parity/unsigned proof_value tests; mixed-rights raw-origin/gap/index disclosure refusal. Medical consent source presence is not this integration. |
| QBS-10 | Reviewed acyclic publication context, entry key derivation, exact 4+8-byte nonce/AAD frame, per-key limits and aggregate forgery budget, exact retained ciphertext retries, crash/rekey refusal, confidential inventory, scoped hybrid wraps/exporter and private plaintext lease failure behavior. Existing AEAD/KDF/lease primitives are sufficient building blocks, not proof of construction security. |
| QBS-16 | Bind exemption/sponsor/corporate decisions to authenticated admission; immutable billing unit and rounding independent of split/coalesce/retry; >16 operations with durable tombstones/replay protection; crash and cap-conservation fixtures; consent remains effective after settlement. Existing single-owner obligation must remain canonical; do not create copied balances or delete old operation identities to free live slots. |

These are integration/qualification gaps, not claims that cryptography, governance or economics primitives are absent. QBS-00 can close as a bounded source inventory while the owning packages retain their release gates.

## Recommended QBS-01 frozen-interface decisions

1. Keep legacy contextual dual signatures as their own profile. Introduce a focused crypto/network/cose wire adapter with bounded parse/encode/verify interfaces, routing explicit raw signing profiles through the existing CryptographicLibrary and managed key owner. Do not globally change mldsa::sign or ed25519::sign_with_context and break old signatures.
2. Freeze a PairedProofBinding schema with exact descriptor/body SHA-384, both key references and algorithms, controller epoch, profile version, purpose and audience. Both Sign1 objects carry the same binding bytes and the agreed external AAD. Freeze protected/unprotected header placement, tag handling, detached-versus-attached payload, permitted key representation and all byte/field limits before coding interoperability. The planned -49/-19 identifiers come from architecture.md; this inspection does not independently certify their standards profile.
3. Freeze separate newtypes for DescriptorId, PublicationContextId, EntryId, RepresentationId and KeyEpoch. Verified authority/descriptor/plaintext handles must have private constructors; process-local slot plus generation is not serialized authority. ProviderGrant remains an internal checked input, with a trusted admission boundary supplying it.
4. Use cold borrowed verification over bounded input and caller scratch, with nonallocating error classes (Malformed, UnsupportedCritical/Profile, Capacity, Unauthorized, Expired, Revoked, StaleGeneration). Public failure detail must not expose key or private-index oracles. Freeze when a crypto failure consumes the failed-verification budget.
5. Define an object-key provider operation by purpose, descriptor/context/entry, epoch, audience, authority generation, cancellation generation and deadline. A confirmed session exporter is a distinct capability from a naked byte slice. Use a non-Copy DEK lease and zeroizing private plaintext lease; do not Copy large signing/KEM private keys into stream records.
6. Freeze per-entry 16-KiB logical leaf framing and ciphertext+tag commitment separately from transport records. Publication context excludes ciphertext roots/signatures; key/prefix/counter uniqueness belongs to an explicit owner. Existing AEAD nonce bytes are 12, so the candidate 4-byte prefix plus u64 big-endian counter is unambiguous only after its exact derivation/AAD is frozen.
7. Keep authority admission, transfer completion, proof verification, plaintext authentication, format admission, policy acceptance, consumer commit, custody and settlement separate typed receipts. Billing references a stable accepted logical operation, not packets, HTTP ranges or Merkle leaf count.
8. Keep parent crypto-flight limits unchanged. Budget descriptor body, paired signatures, public keys/certificates, cryptographic scratch and aggregate hostile verification separately. The architecture's 64/96-KiB bounds and provisional encrypted-frame ceiling need independent checks, not silent promotion to production permission.

## First independent fixtures

Start with one tiny fixed public descriptor body, one exact binding, and a trusted full paired-key pin. An independent reference encoder must publish the exact canonical body, protected-header bytes, external AAD, both Sig_structure byte strings, primitive-context bytes, public keys and signatures with provenance. Cross-check raw provider verification in a second implementation; a second binary importing the production encoder is not independent.

Negative cases: stripped member, same algorithm twice, substituted paired key, wrong body digest, audience, purpose, controller epoch or external AAD; duplicate/out-of-order/nonminimal CBOR keys, indefinite lengths, unknown critical header, trailing data and oversized signature/key; legacy QDNF-context signatures deliberately rejected as new-profile COSE. Include a positive control that fails when either signature check is intentionally disabled.

Next fixtures: stale/revoked trusted pin and cancelled key result; exact final/empty leaf and wrong AEAD context/tag with no observable plaintext; crash after ciphertext creation but before persistence forcing fresh context/key or exact retained-byte reuse. Accounting fixtures compare one accepted operation with split/coalesced/retried delivery and a 17th operation, proving bounded refusal until durable deduplication exists. Each fixture records source and byte hashes and keeps secrets strictly test-only.

## Inspected byte fingerprints

SHA-256 captured during report creation; no source writes performed by this worker.

| Path | SHA-256 |
|---|---|
| `crates/qualia-core-db/Cargo.toml` | `e38ae280cc238b57c686eb17883709a04b72145e5fc74ebdb59d89a617adc34a` |
| `crates/qualia-core-db/src/crypto/mod.rs` | `a5943a7bb27b91ff989fbc9778ab10c881bda64c1fda1089855a8f41aff2261c` |
| `crates/qualia-core-db/src/crypto/network/mod.rs` | `2c2d74e9b1870e291f099435e889fa2e4e5694dcd571c55f14e49ecc6a1593df` |
| `crates/qualia-core-db/src/crypto/network/dual_sign.rs` | `b95cca09d34b0092a043aaba15ec771828531868bc90c4fc7ba0e2a2319c907f` |
| `crates/qualia-core-db/src/crypto/network/mldsa.rs` | `667a791cf6a1e750a49bc52ac1d6d098b024d60f8f571e808daca4c23ddd93a3` |
| `crates/qualia-core-db/src/crypto/network/ed25519.rs` | `3b1ae85d9df8e1ff32fef391d4a258a96bc105cb8454eeb6861fa6b1dafb6e4f` |
| `crates/qualia-core-db/src/crypto/network/pq_handshake.rs` | `75adc371d11c29631c076a33cc8f6609f9f3ad3aa7e0219bbbfd4e22d24bda02` |
| `crates/qualia-core-db/src/crypto/network/share_encoding.rs` | `3825b5c4b4b716bad19913ea7c7d8d90d5797e611d962aa2b3577bd83d344057` |
| `crates/qualia-core-db/src/crypto/network/key_provider/grant.rs` | `c6f29e6b0972d58f2cf2f8b7910b12d8dd42b343276150fe0202720be5d9a6ba` |
| `crates/qualia-core-db/src/crypto/network/key_provider/software.rs` | `386d1506a6d518d2b6e9d97e357f900ba40758724bd6dcadfea9f6230d8b203d` |
| `crates/qualia-core-db/src/crypto/network/secret_lease.rs` | `0c807cf0e8de21025b61d4650e50bebea82a04a2283859f537b2695e07085467` |
| `crates/qualia-core-db/src/crypto/network/aead.rs` | `e69817cd5c561d490d12fe009e3c57d93aa30d5a807bf7dafb3a94c0f94a9e54` |
| `crates/qualia-core-db/src/crypto/network/kdf.rs` | `78f75a54e7841433d49865d9a2e88183c7bea32f47c05746b6706fba1e0ee853` |
| `crates/qualia-core-db/src/crypto/network/chunks.rs` | `47b0daa944376f09737868acbc2f84b0b66dc15177bc050e77a45802c68a891a` |
| `crates/qualia-core-db/src/net/qdnf/economics/obligation.rs` | `816834de134817a3cd9c7df7c38a2098bf4aabb1ccfb91e0814045b796fa24ba` |
| `crates/qualia-core-db/src/net/qdnf/economics/consent.rs` | `fc3551468d4d13d3d7b92a3387871b2851cbe96791c4ef23f75fb067a353253b` |
| `crates/qualia-core-db/src/net/qdnf/economics/adapter.rs` | `9660404b44a00c0a8b27c360823ef8883cdb07883b9cabd60aacc406b58ae571` |

## Crypto-library reuse correction (12 November 2026)

This baseline was a scoped network inventory, not evidence that QualiaDB lacked a crypto library. See [crypto-library-reuse.md](crypto-library-reuse.md) for inspected CryptographicLibrary, MlDsaSigner, KeyManager, KeyVault, full-digest AuthorityOwner and QSession APIs and the concrete adapter correction. The did:qi GenesisPayload already binds ML-DSA and ML-KEM hashes alongside classical keys; rejection of classical-only proof evidence does not mean did:qi architecture is classical-only. Verified full paired controller-chain composition remains a distinct integration gate. The baseline hashes above describe the earlier inventory snapshot.
