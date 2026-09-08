# Cryptography and key-provider implementation workstream

**Status:** Implementation plan only. Every checklist item is pending; no protocol security claim follows from this plan.

This workstream supplies audited-library adapters, scoped key operations, versioned hybrid
handshakes and exact proof vectors for the independent Qualia Peer Runtime. It owns no new
cryptographic primitive and does not replace existing crypto libraries with network-local copies.

Read [Post-Quantum Security](../../post-quantum-security.md),
[Cryptographic Profile](../../cryptographic-profile.md),
[Identifier Fabric Integration](../../identifier-fabric-integration.md),
[Network Cell](../../network-cell.md) and
[Electronic Evidence and Retention](../../electronic-evidence-and-retention.md).
The classical profile is compatibility evidence only; the proposed replacement uses `qpr-pq-1`.

Follow the parent [library layout](../library-layout.md),
[swarm protocol](../swarm-protocol.md) and [validation matrix](../validation-matrix.md).
Future Rust paths below are ownership proposals, subject to that shared layout, not existing files.

### Swarm boundaries and implementation directions

Assign one owner to each parent task and a separate reviewer for security and failure behavior.
Child agents may own a primitive adapter, provider backend or vector family after claiming its
files and interfaces. They must not independently alter common enums, Cargo features, registries,
proof domains or transcript layouts. Submit those changes through the owning integrator.

Use directory-backed libraries. Every `mod.rs` contains module routing and public re-exports only.
Keep implementations below 500 lines, splitting sooner when responsibilities differ. Separate
hot execution, cold preparation, key-provider backends, lifecycle state, tests and vector artifacts.
Do not grow `fiduciary_crypto.rs` into the handshake engine or place all primitives in one adapter file.

All Tier-1 paths, including rejection and cancellation, use caller-owned buffers and bounded
nonallocating errors. Cold authoring uses declared bounded workspaces. Charge dependency scratch,
worker stacks, queued proofs and secret lifetimes to the admitted cell and host budgets. Ordinary
cells remain at or below 512 MiB; Webizen evaluation passes retain their independent 42 MiB budget.
Existing constants or fixed-size wrappers are not measurements of these properties.

### Dependency and ownership table

| Parent task | Required predecessors | Principal output |
|---|---|---|
| CRY-01 | FND-01, FND-02 | Existing-crypto adapters and scoped key-provider interface |
| CRY-02 | CRY-01, FND-03 | Versioned hybrid transcripts, paired proofs and independent vectors |

FND-03 establishes the jointly reviewed interface baseline: types, ownership, errors, limits and
version negotiation points. It need not finalize every cryptographic byte before CRY-02 starts.
CRY-02 owns byte-level refinement and returns versioned changes to the foundation integrator.
Downstream agents can use explicit test fixtures meanwhile; fixture success cannot certify security.

## CRY-01 — Existing crypto and purpose-scoped key provider

**Dependencies:** FND-01, FND-02. **Owner:** crypto integration agent; independent key-lifecycle reviewer.

**Scope:** Reuse the core KEM, signature and key-management implementations through bounded network
interfaces. Establish ownership and authorization before any QLink or QSession requests signatures.

- [ ] CRY-01.01 Inventory actual locked dependencies, enabled features, supported parameter sets and native/WASM paths;
  record each reusable implementation, allocation gap and required adapter without treating library names as certification.
- [ ] CRY-01.02 Publish typed algorithm, digest, key-reference, epoch, purpose and operation identifiers with FND-02;
  distinguish full security digests from compact lookup hashes and reject unknown algorithm/length pairs.
- [ ] CRY-01.03 Define the key-provider request/completion contract with cell epoch, audience, authority generation,
  operation digest, deadline, reservation and exact input/output ranges; reserve completion space before admission.
- [ ] CRY-01.04 Bind provider grants to permitted operations and paired key epochs; reject wrong-purpose signing,
  cross-controller substitution, revoked grants and stale completions rather than exposing an arbitrary signing oracle.
- [ ] CRY-01.05 Adapt ML-KEM-768 using the existing primitive owner, fixed caller outputs and typed failures;
  preserve implicit rejection and reject invalid public encodings without exposing secret-dependent failure detail.
- [ ] CRY-01.06 Replace secret `Copy`/`Debug` exposure at the integration boundary with explicit zeroizing ownership;
  audit dependency copies, logging, moves and worker scratch rather than claiming wrapper erasure covers all copies.
- [ ] CRY-01.07 Provide separate ML-DSA-65 and Ed25519 adapters with exact signing-mode/context contracts;
  isolate any unavoidable bounded allocation from Tier-1 calls and account for the full dependency workspace.
- [ ] CRY-01.08 Add versioned, length-delimited network signing contexts beside legacy context behavior;
  demonstrate that historical records keep their old verifier and cannot silently acquire the new interpretation.
- [ ] CRY-01.09 Reuse established SHA-384, HKDF/HMAC-SHA-384, X25519 and ChaCha20-Poly1305 primitives;
  reject all-zero DH results, validate lengths and define typed exhaustion/failure outcomes without custom cryptography.
- [ ] CRY-01.10 Implement entropy-provider failure and ephemeral-key lifecycle contracts: generation, lease, use,
  rotation, cancellation and destruction; production failures must never select deterministic test randomness.
- [ ] CRY-01.11 Specify paired controller-key rotation and authorized recovery, including revoked/expired roots;
  distinguish recovered key possession from authority to use the recovered instrument for a requested operation.
- [ ] CRY-01.12 Partition provider interfaces, software backend and future hardware backend adapters into separate files;
  unsupported hardware/WASM operations return explicit capability errors and cannot fall back across trust boundaries.
- [ ] CRY-01.13 Reserve simultaneous proof, key, scratch, stack and queue memory before work; enforce per-source and
  aggregate compute/deadline limits, including cancellation where an individual primitive cannot be safely preempted.
- [ ] CRY-01.14 Measure allocations on hot success, malformed-input, capacity, revocation and cancellation paths;
  separately report bounded cold allocations, peak stack and concurrent cell/host charges against admitted limits.
- [ ] CRY-01.15 Run primitive known-answer and malformed-input suites plus provider misuse, entropy failure,
  use-after-revocation, restart, stale lease and duplicate-completion cases; verify output buffers and secrets on failure.
- [ ] CRY-01.16 Deliver a dependency/advisory and side-channel review with unresolved issues and feature constraints;
  require independent approval of the adapter evidence before downstream production use, without claiming FIPS validation.

### Planned library construction

Place network-facing adapters beside existing owners under
`crates/qualia-core-db/src/crypto/network/`: `types.rs`, `errors.rs`, `kem.rs`, `mldsa.rs`,
`ed25519.rs`, `kdf.rs`, `aead.rs` and `secret_lease.rs`. Split again if any primitive adapter gains
multiple lifecycles. Reuse existing implementations rather than moving unrelated callers en masse.

Use `key_provider/` for `request.rs`, `grant.rs`, `lifecycle.rs` and `backends/software.rs`;
hardware integration receives its own backend module. Provider queue scheduling belongs to the
runtime owner; the crypto provider exposes bounded work/cancellation contracts to that scheduler.
Keep unit tests in a sibling `tests/` directory and binary known-answer fixtures outside production code.

### Deliverables, acceptance and handoff

Deliver the API/error/ownership contract, source reuse map, allocation/work measurements, primitive
vector results and key-lifecycle review. Identify exact features and targets covered by the evidence.
The acceptance gate is correct primitive results plus demonstrated rejection, secret handling,
purpose enforcement and bounded concurrent work; passing self-generated round trips is insufficient.

Hand CRY-02 stable primitive and provider interfaces, key-pair/epoch fixtures and failure semantics.
Give FND-02 the typed-digest/provider exports and runtime owners reservation/cancellation requirements.
Any unresolved secret exposure, hot allocation or authorization bypass remains a blocking defect
for the affected capability; do not hide it by marking the entire provider complete.

## CRY-02 — PQ transcripts, dual proofs and independent vectors

**Dependencies:** CRY-01, FND-03. **Owner:** protocol crypto agent; independent transcript/vector reviewer.

**Scope:** Freeze the proposed `qpr-pq-1` construction using existing primitives. QLink and QSession
have distinct secrets/domains. Online security, native control authority and offline custody are
separate claims; this task cannot enable unspecified custody or invent a new composite primitive.

- [ ] CRY-02.01 Reconcile the classical and PQ schemas against FND-03 and publish explicit version/profile boundaries;
  mark unresolved byte choices as draft until reviewed, without requiring FND-03 to have finalized those bytes already.
- [ ] CRY-02.02 Define the state transitions from reachability admission through shares, handshake keys, encrypted
  proofs and Finished to traffic; enumerate forbidden transitions and prohibit all initial-profile 0-RTT application data.
- [ ] CRY-02.03 Freeze exact initiator/responder share encodings, role ordering and the ML-KEM-then-X25519 secret input;
  derive directional handshake keys from the share transcript before encrypting proofs, avoiding circular dependencies.
- [ ] CRY-02.04 Freeze length-delimited transcripts binding offered/selected suites, nonces, retry chain, observed bearer,
  target, RAR/DNI, service/purpose and critical features, with distinct QLink/QSession domains and Finished labels.
- [ ] CRY-02.05 Define the exact deterministic-CBOR binding object and two COSE_Sign1 signing inputs under the selected
  ML-DSA COSE/context rules; both ML-DSA-65 and Ed25519 must verify over the same bound policy, keys and payload.
- [ ] CRY-02.06 Verify the complete required controller, delegation, capability, route and recovery trust paths against
  their PQ minimums; reject classical-only authority bottlenecks rather than inferring PQ authentication from the KEM.
- [ ] CRY-02.07 Version all PQ authority, operation, semantic-bundle and Merkle commitments with typed SHA-384 digests;
  recompute from original exact bytes and reject padding or rehashing old compact/SHA-256 identifiers as an upgrade.
- [ ] CRY-02.08 Bind negotiation to cached authorized peer/realm minima and rollback policy; test stripped offers,
  alternate proof pairs, wrong contexts, revoked roots and downgrade attempts with no automatic classical retry.
- [ ] CRY-02.09 Specify cookie/relationship-gated admission chunks jointly with NET-02: 16 KiB per flight, at most
  16 chunks, 32 global pending handshakes and two per admitted locator, further constrained by shared reservations.
- [ ] CRY-02.10 Freeze duplicate/conflicting chunk, length, expiry and retry behavior before allocating proof space;
  retain exact retransmission bytes, never extend deadlines on duplicates, and bind complete ordered flights before trust.
- [ ] CRY-02.11 Define traffic nonce/packet-number exhaustion, bounded old-key retention, fresh restart and erasure;
  distinguish traffic key update from recovery requiring fresh hybrid exchange and reverified authority.
- [ ] CRY-02.12 Produce byte-exact share, transcript, KDF, proof, Finished and record vectors with intermediate values
  derived from test keys only; obtain independent reproduction rather than using the same encoder as its sole oracle.
- [ ] CRY-02.13 Exercise truncation, noncanonical encodings, unknown critical fields, signature stripping, cross-role
  substitution, bad Finished, all-zero DH, malformed KEM inputs and reordered flights with consistent failure handling.
- [ ] CRY-02.14 Exercise short MTUs, oversized/uncached authority chains, queue floods, entropy/provider failures,
  cancellation at each stage and cell restart; measure denial work, amplification, live bytes and secret cleanup.
- [ ] CRY-02.15 Review durable-proof preservation and algorithm migration with evidence owners; retain original signed
  material and provenance, exclude traffic secrets, and keep unspecified hybrid custody/SLH-DSA profiles disabled.
- [ ] CRY-02.16 Publish reviewed schemas, immutable vector digests, supported claims and outstanding security findings;
  gate NET-02/NET-05 integration on those versions and require independent protocol review before production claims.

### Planned library construction

Under `crates/qualia-core-db/src/net/qdnf/`, use `crypto/handshake/` with `state.rs`,
`share_encoding.rs`, `transcript.rs`, `schedule.rs`, `finished.rs` and `admission_chunks.rs`.
Use `crypto/proof/` with `binding.rs`, `cose.rs` and `trust_path.rs`, and a separate
`crypto/record_digest.rs`. Shared state enums have one owner. QLink/QSession own their protocol
drivers and timers; the crypto library owns primitive execution and private-key lifecycle.

Keep vector generation, independent verification and malformed fixtures in separate test tooling.
Deterministic fixture keys must never enter production entropy/provider code. Large vector tables
are data artifacts, not justification for a monolithic protocol implementation file.

### Deliverables, acceptance and handoff

Deliver versioned encoding tables, complete state/error transitions, exact vectors with provenance,
independent comparison results and full-flight byte/work/memory measurements. Record explicit
unsupported cases. Acceptance requires dual-proof trust-path validation and downgrade/replay/fault
tests, not merely a successful encrypted exchange between two instances of the same implementation.

Hand NET-02 the admitted-bootstrap codec, QLink labels and required confirmation states; hand
NET-04 record-proof/digest profiles and authority-validation results; hand NET-05 QSession labels,
Finished, nonce and refresh rules. Return final version changes to FND-03 and release owners.
Any subsequent byte change invalidates affected vectors and requires a coordinated revision before
dependent checklists can close. No item in this document authorizes deploying an unreviewed protocol.
