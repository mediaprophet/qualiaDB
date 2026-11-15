# QBDL streaming work packages

Revision 1, 12 November 2026. **Implementation in progress.** The [registry](task-registry.json) is canonical. Status complete requires recorded acceptance; active work may implement reviewed sub-contracts before complete integration dependencies. Shared files belong to root.

## QBS-00 — Baseline and prerequisite evidence

**Status:** complete · **Release:** R1 · **Lead tier:** senior · **Planning cap:** 40,000 tokens (estimate, not measured use).

**Depends on:** none. **Reviewer:** integrator + domain reviewer.

**Proposed write scope:** docs: baseline evidence only.

Deliverables:

- [x] Inventory actual source/API/features across QualiaDB, Etalon, Synth and Hypermedia; capture commits and dirty fingerprints.
- [x] Map parent FND/CRY/NET/RT/SEM/ECO gates to executable evidence; inspect existing function indexes before proposing replacements.

Acceptance:

- [x] List real reusable exports, missing integration evidence and pre-existing failures with commands.
- [x] Preserve all unrelated changes; no module or dependency edits in this task.

Evidence: [evidence/baseline-revisions.json](evidence/baseline-revisions.json), [evidence/baseline-network.md](evidence/baseline-network.md), [evidence/baseline-security.md](evidence/baseline-security.md), [evidence/baseline-consumers.md](evidence/baseline-consumers.md), [evidence/baseline-acceptance.md](evidence/baseline-acceptance.md).

## QBS-01 — Freeze interfaces, schemas and ownership

**Status:** in_progress · **Release:** R1 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-00. **Reviewer:** protocol architect + crypto/policy owners.

**Proposed write scope:** docs: ADRs and schema drafts; shared Rust exports/registries through integrator only.

Deliverables:

- [ ] Freeze handle/error/limit interfaces, carrier variants, descriptor page schema and version negotiation.
- [ ] Assign exact file claims; create CDDL/wire fixtures, registry proposals and security-class labels.

Acceptance:

- [ ] Independent owners agree byte domains, version boundaries and cancellation/lease semantics.
- [ ] No unallocated opcode, MIME type or cryptographic profile is claimed registered.

## QBS-02 — Unify bounded QBDL preflight

**Status:** in_progress · **Release:** R1 · **Lead tier:** senior · **Planning cap:** 40,000 tokens (estimate, not measured use).

**Depends on:** QBS-01. **Reviewer:** parser/security reviewer.

**Proposed write scope:** QualiaDB: bundle/reader_limits.rs; QualiaDB: bundle/streaming_loader.rs; QualiaDB: bundle/stream/index/ (proposed).

Deliverables:

- [ ] Share index validation between full and progressive paths; support existing flags 0 and 1 only.
- [ ] Reject malformed/missing required digests; separate remote u64 extent from resident limits; preserve accepted legacy CBOR bytes.

Acceptance:

- [ ] Duplicate keys, overlap, bad alignment, overflow, count mismatch, trailing input and allocation bombs fail.
- [ ] Existing writer fixtures pass both paths; no digest-less arrival is marked verified; no unrelated table ceilings raised.

## QBS-03 — Freeze paired PQ envelope and primitive contracts

**Status:** in_progress · **Release:** R1 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-01. **Reviewer:** independent crypto reviewer; external review at release.

**Proposed write scope:** QualiaDB: crypto/network/dual_sign.rs; QualiaDB: crypto/network/cose/ (proposed).

Deliverables:

- [ ] Complete parent CRY-02.05 proof policy: required ML-DSA-65 and Ed25519 over one binding object.
- [ ] Pin -49/-19 COSE profiles, signing context/AAD, exact key encodings, feature closure, dependency errata and primitive workspace limits.

Acceptance:

- [ ] Independent positive/negative vectors reject stripping, substitution, duplicate algorithm, context/epoch and encoding attacks.
- [ ] No frozen-success flag until exact vectors and security review pass; primitives are reused, not rewritten.

## QBS-04 — PQ controller authority and scoped key access

**Status:** in_progress · **Release:** R1 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-03. **Reviewer:** independent authority + crypto reviewer.

**Proposed write scope:** QualiaDB: crypto/network/key_provider/; QualiaDB: net/peer/ identity adapters (exact files claimed after 00); docs: did:qi PQ binding ADR.

Deliverables:

- [ ] Implement trusted paired-key pins and accepted PQ authority chains, rotation/revocation and scoped key handles.
- [ ] Integrate fresh reviewed QSession hybrid key confirmation; link incomplete CRY/NET prerequisites rather than bypass them.

Acceptance:

- [ ] Classical-only root, untrusted self-signed key, wrong audience, expired grant and revoked epoch fail.
- [ ] No private data before confirmation; canceled/late key completions cannot reuse handles; secrets not logged/copied casually.

## QBS-05 — SHA-384 entry trees and proof codec

**Status:** in_progress · **Release:** R1 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-01. **Reviewer:** independent proof implementer/reviewer.

**Proposed write scope:** QualiaDB: bundle/stream/proof/ (proposed); vectors: qbdl-stream-pq-1/trees.

Deliverables:

- [ ] Freeze domain-separated entry-relative leaves, empty/final rules and non-power-of-two split behavior.
- [ ] Implement iterative caller-buffered verifier and bounded incremental outboard builder; no permission-crossing leaves.

Acceptance:

- [ ] Independent implementation agrees for empty, 1, 2, 3 and large leaf counts.
- [ ] Every proof/context/index/length mutation, extra sibling and cross-entry/generation splice fails; measured hot allocations zero.

## QBS-06 — Publisher and detached paged QASD

**Status:** in_progress · **Release:** R1 · **Lead tier:** senior · **Planning cap:** 40,000 tokens (estimate, not measured use).

**Depends on:** QBS-02, QBS-03, QBS-05. **Reviewer:** format + crypto reviewer.

**Proposed write scope:** QualiaDB: bundle/stream/descriptor/ (proposed); QualiaDB: bundle/stream/publish/ (proposed).

Deliverables:

- [ ] Build immutable body/pages, exact header/index/entry commitments and separate renewable publication proofs.
- [ ] Compile public projections; validate all metadata disclosure and preserve source bundle bytes.

Acceptance:

- [ ] Repeat publication is deterministic for pinned inputs/context; signature randomness is reported separately.
- [ ] Large inventories stay under cumulative budgets; changed inner bytes invalidate commitments; no circular hash dependency.

## QBS-07 — Policy and semantic dependency admission

**Status:** in_progress · **Release:** R1 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-04, QBS-06. **Reviewer:** semantic governance + privacy reviewer.

**Proposed write scope:** QualiaDB: bundle/stream/policy/ (proposed); QualiaDB: bundle/stream/selection/ (proposed).

Deliverables:

- [ ] Compile pinned semantic closure, Share grants and compatible dependency DAG; bind permission epochs and purposes.
- [ ] Separate perceptual/quantized alternatives from physical state variants; classify metadata and payload disclosure.

Acceptance:

- [ ] Cycles, incomplete closures, stale policy, unsigned proof_value and forged parity tokens cannot authorize use.
- [ ] Mixed raw bundles never reach public endpoints; no live ontology dereference; finite-cost fulfilment cannot override rights.

## QBS-08 — Bounded runtime slots and coupled reservations

**Status:** in_progress · **Release:** R1 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-01. **Reviewer:** runtime concurrency reviewer.

**Proposed write scope:** QualiaDB: net/peer/runtime/ streaming adapter (new focused files); QualiaDB: bundle/stream/runtime/ (proposed).

Deliverables:

- [ ] Implement cold plan and leased hot transfer table; aggregate network/proof/storage/consumer charges.
- [ ] Add credits, fair scheduling, deadlines, generation-fenced cancellation and idempotent completion.

Acceptance:

- [ ] Success/rejection/cancel/timeout hot paths allocate zero with positive-control tests.
- [ ] Capacity failure precedes request; late completion cannot revive a canceled slot; control reserve cannot be consumed by bulk data.

## QBS-09 — Native content-swarm provider and client

**Status:** in_progress · **Release:** R1 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-04, QBS-06, QBS-07, QBS-08, QBS-11. **Reviewer:** independent networking reviewer.

**Proposed write scope:** QualiaDB: net/peer/replication/qbdl/ (proposed); QualiaDB: net/qdnf/session integration via owner.

Deliverables:

- [ ] Bind Describe/Open/RequestLeaves/LeafData/Credit/Cancel/Resume to existing service/session ownership.
- [ ] Build native provider/client vertical slice and negotiated reliable-carrier behavior.

Acceptance:

- [ ] Transfer useful partial object over real native bearer with HTTP/DNS/IP sockets and libp2p disabled.
- [ ] Loss/reorder/path change/revocation cannot corrupt state or duplicate commit; no application retransmission over reliable carrier.

## QBS-10 — Indexed object encryption and recipient wraps

**Status:** in_progress · **Release:** R1 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-03, QBS-04, QBS-05, QBS-06. **Reviewer:** independent crypto composition reviewer.

**Proposed write scope:** QualiaDB: crypto/network/object_stream/ (proposed); QualiaDB: bundle/stream/protection/ (proposed).

Deliverables:

- [ ] Specify acyclic context/key/nonce/AAD/cipher-root construction and exact usage limits; encrypt entries independently.
- [ ] Add confidential metadata, scoped recipient wrapping, zeroizing private output and prospective key rotation.

Acceptance:

- [ ] Wrong tag/proof/context/epoch, counter reuse, crash/retry and key-rotation vectors fail safely.
- [ ] No nonce width ambiguity, plaintext-before-authentication, leaked private digests or claim that removing a wrap revokes a learned key.

## QBS-11 — Verified sparse storage and restart

**Status:** in_progress · **Release:** R1 · **Lead tier:** senior · **Planning cap:** 40,000 tokens (estimate, not measured use).

**Depends on:** QBS-05, QBS-08. **Reviewer:** independent durability reviewer.

**Proposed write scope:** QualiaDB: bundle/stream/storage/ (proposed); QualiaDB: existing storage owner adapters via claimed files.

Deliverables:

- [ ] Maintain separate verified bitmap/checkpoint and sparse data; enforce flush/order/generation protocol.
- [ ] Revalidate authority and stored bytes after lost exclusion; promote only admitted closures; preserve holds/pins.

Acceptance:

- [ ] Crash injection between every data/bitmap/commit step cannot turn holes or torn writes into verified bytes.
- [ ] External mutation, disk full, cancellation and eviction release leases exactly once; no unsupported atomicity claim.

## QBS-12 — Format-specific partial admission

**Status:** in_progress · **Release:** R1 · **Lead tier:** senior · **Planning cap:** 40,000 tokens (estimate, not measured use).

**Depends on:** QBS-02, QBS-05, QBS-07. **Reviewer:** codec owner + independent validation reviewer.

**Proposed write scope:** QualiaDB: bundle/stream/formats/ (proposed); existing codecs: narrow adapters only through owner.

Deliverables:

- [ ] Add Q42/10D/EMF/M64 partial admission adapters without duplicating codecs; expose precise verification state.
- [ ] Include BLAKE3 trailers and actual 10D 24-byte descriptors; enforce semantic chunk/section dependencies.

Acceptance:

- [ ] Authenticated leaf is never mislabeled whole-container verification; corrupted last digest byte/trailer fails.
- [ ] At least one complete scientific brick and one geometry-section closure can be used without downloading the whole entry.

## QBS-13 — Etalon complete-model and resource bridge

**Status:** pending · **Release:** R1 · **Lead tier:** senior · **Planning cap:** 40,000 tokens (estimate, not measured use).

**Depends on:** QBS-08, QBS-11, QBS-12. **Reviewer:** Etalon owner + lifetime reviewer.

**Proposed write scope:** QualiaDB: bundle/stream/consumer/ (proposed); Etalon: resources/model-io/inference-contract adapters under exact claims.

Deliverables:

- [ ] Reserve staging, storage and VRAM; use existing excluded-writer Snapshot/ModelLease and governed commit.
- [ ] R1 P64 path obtains a complete verified entry; check actual alignment and ETL metadata through canonical preparation.

Acceptance:

- [ ] No unverified sparse tensor/alternate view reaches GPU; device loss and canceled commit preserve ownership.
- [ ] Misaligned outer entries use bounded staging/file extraction, not mandatory whole-model RAM copies; CPU and claimed GPU targets tested.

## QBS-14 — HTTP/LIG byte-exact provider and client

**Status:** pending · **Release:** R1 · **Lead tier:** senior · **Planning cap:** 40,000 tokens (estimate, not measured use).

**Depends on:** QBS-09, QBS-10, QBS-12. **Reviewer:** HTTP + privacy reviewer.

**Proposed write scope:** QualiaDB: bundle/stream/http/ (proposed); LIG/fetch owner integration by exact claim.

Deliverables:

- [ ] Expose descriptor/pages/proofs/immutable representations; strict single-range HTTP client and bounded full-entry fallback.
- [ ] Integrate CSCP redirects, validators, privacy-partitioned caching and identical proof/policy handling.

Acceptance:

- [ ] 206 bounds/encoding/length exact; huge 200 aborted; 416, multipart, changed validators and redirects handled without authority bypass.
- [ ] Native/HTTP fixtures agree on security outcomes; HTTP/1.1 required, HTTP/2/3 separately reported.

## QBS-15 — Browser worker OPFS and WASM admission

**Status:** pending · **Release:** R1 · **Lead tier:** senior · **Planning cap:** 40,000 tokens (estimate, not measured use).

**Depends on:** QBS-13, QBS-14, QBS-26. **Reviewer:** browser + resource reviewer.

**Proposed write scope:** QualiaDB: bundle/stream/browser/ (proposed); QualiaPortal worker entry points via owner.

Deliverables:

- [ ] Implement dedicated-worker hydration/storage, CORS-aware exact-byte fetch, transferable fallback and WebGPU limit checks.
- [ ] Document forbidden headers and opaque redirects; bound application queues and probe capabilities before bulk work.

Acceptance:

- [ ] Named browser tests cover OPFS denial/quota, missing SAB/COI, CORS denial, tab closure and device loss.
- [ ] Rust hot allocation claims measured separately from browser internals; no hidden full-file WASM allocation.

## QBS-16 — Commons exemptions and finite single-owner accounting

**Status:** pending · **Release:** R1 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-07, QBS-08, QBS-09. **Reviewer:** economics invariant + governance reviewer.

**Proposed write scope:** QualiaDB: net/qdnf/economics/ streaming adapter (new focused files); QualiaDB: bundle/stream/receipts/ (proposed).

Deliverables:

- [ ] Compile existing exemption/sponsor/corporate rules and billing basis into admitted handles.
- [ ] Add durable idempotent accounting across more than the existing 16 live operation slots without erasing replay protection; separate receipt types.

Acceptance:

- [ ] Personal/protected/humanitarian paths do not wait on recovery-fee availability.
- [ ] Split/coalesced/retried transfers charge equally, cap invariant holds, revoked consent refuses even after payment; no packet ACK settlement.

## QBS-17 — Independent fixtures and peer harness

**Status:** in_progress · **Release:** R1 · **Lead tier:** senior · **Planning cap:** 40,000 tokens (estimate, not measured use).

**Depends on:** QBS-01. **Reviewer:** independent verification owner.

**Proposed write scope:** tests: qbdl-stream independent peer and fixtures; vectors: qbdl-stream-pq-1 independent reference encoder.

Deliverables:

- [ ] Build independent descriptor/proof encoder/verifier and hostile provider, fault injection and carrier parity harness.
- [ ] Use separate parsing/proof implementation, not the same production library wrapped in another binary.

Acceptance:

- [ ] Fixtures include positive controls and every boundary in qualification.md; provenance and source digests recorded.
- [ ] Harness distinguishes unsupported target from pass and can run without changing the users live network.

## QBS-18 — Immutable generations, subscriptions and sensing

**Status:** pending · **Release:** R2 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-25. **Reviewer:** distributed-state + sensing reviewer.

**Proposed write scope:** QualiaDB: bundle/stream/generations/ (proposed); QualiaDB: qualia-etalon-sensing-bridge adapter via owner.

Deliverables:

- [ ] Bind pubsub/QSync announcements to immutable generation/predecessor/policy/calibration state.
- [ ] Implement explicit retention gaps, discontinuities and clock/consent invalidation; no unfinished v1 append stream.

Acceptance:

- [ ] Out-of-order/duplicate/forked announcements and revoked sensing consent cannot create false continuity.
- [ ] No append-consistency claim for arbitrary rewritten bundle trees; independent sensing fixtures retain units/calibration.

## QBS-19 — Distributed finite-recovery escrow

**Status:** pending · **Release:** R2 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-16, QBS-25. **Reviewer:** independent distributed-economics reviewer.

**Proposed write scope:** QualiaDB: net/qdnf/economics/escrow/ (proposed); durable obligation owner through claimed interface.

Deliverables:

- [ ] Specify grant conservation, persistent deduplication, fencing/return and tail coordination before implementation.
- [ ] Model partitions/crashes/clocks; distinguish allocated rights from subholds to avoid double counting.

Acceptance:

- [ ] Machine-checked or exhaustive bounded model maintains cap under duplicate/reordered/crashed operations.
- [ ] Expiry alone never reissues spendable partitioned rights; free/exempt paths remain available; financial release needs independent domain review.

## QBS-20 — Solver-result Commons and QPU escalation adapter

**Status:** pending · **Release:** R2 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-16, QBS-25. **Reviewer:** optimization/math + privacy reviewer.

**Proposed write scope:** QualiaDB: existing qubo/qpu pipeline adapters via owner; QualiaDB: bundle/stream/solver_commons/ (proposed).

Deliverables:

- [ ] Local solve, exact canonical problem key, scoped history lookup and local witness verification precede QPU dispatch.
- [ ] Classify exact hit/family candidate/in-flight/insufficient quality/scoped miss/unavailable; retain private mappings.

Acceptance:

- [ ] Poisoned witnesses, approximate-normalization aliases, topology leakage and unapproved provider spend are rejected.
- [ ] Separate feasible/heuristic/bounded/certified/sample results; QPU provenance alone never certifies optimum.

## QBS-21 — Selective P64 and operator-package execution

**Status:** pending · **Release:** R2 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-13, QBS-25. **Reviewer:** Etalon inference + numerical reviewer.

**Proposed write scope:** Etalon: model/operator_package, model-io and inference adapters by exact claim; QualiaDB: bundle/stream/consumer/model_ranges/ (proposed).

Deliverables:

- [ ] Use existing validated operator/tensor range owners, metadata dependencies, reservations and governed commit.
- [ ] Qualify MoE/PLE selection, alternate precision, key epochs and KV checkpoint behavior under streamed storage.

Acceptance:

- [ ] Only required verified ranges enter execution; missing dependencies/stale leases/alternate-length uncertainty fail.
- [ ] Measured cold start and residency improve over complete-entry R1 without changing admitted mathematical fidelity.

## QBS-22 — Optional QBDL alignment extension

**Status:** pending · **Release:** R3 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-13, QBS-25. **Reviewer:** format compatibility reviewer.

**Proposed write scope:** QualiaDB: bundle format/writer/readers via one owner; standards: versioned alignment ADR and fixtures.

Deliverables:

- [ ] Benchmark bounded staging versus new 16 KiB or extended alignment; select compatible version/critical-feature mechanism.
- [ ] If justified, update every reader/writer/probe and inner alignment admission together; otherwise record a no-extension decision.

Acceptance:

- [ ] Existing v1 bytes retain interpretation; old readers reject unsupported new profile predictably.
- [ ] No unreviewed use of reserved byte 40 or auto-promotion based solely on filename/kind.

## QBS-23 — Adversarial and platform qualification

**Status:** pending · **Release:** R1 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-02, QBS-03, QBS-04, QBS-05, QBS-06, QBS-07, QBS-08, QBS-09, QBS-10, QBS-11, QBS-12, QBS-13, QBS-14, QBS-15, QBS-16, QBS-17, QBS-26. **Reviewer:** independent qualification owner + external crypto reviewer.

**Proposed write scope:** tests/evidence only plus narrowly assigned fix followups.

Deliverables:

- [ ] Run all R1 gates, independent peer/carrier parity and resource/performance campaigns on pinned source states.
- [ ] Commission independent cryptographic composition review; route defects back to owner and reopen dependencies.

Acceptance:

- [ ] All mandatory qualification gates have evidence; no fixture-only native path, false PQ claim or unexecuted test marked passed.
- [ ] Security, privacy, memory and accounting failures block release; platform exclusions are explicit.

## QBS-24 — Optional HLS/DASH presentation adapters

**Status:** pending · **Release:** R3 · **Lead tier:** senior · **Planning cap:** 40,000 tokens (estimate, not measured use).

**Depends on:** QBS-18, QBS-25. **Reviewer:** media interoperability + policy reviewer.

**Proposed write scope:** LIG/loopback audiovisual presentation adapter via owner; standards: media projection profile.

Deliverables:

- [ ] Project authorized audiovisual entries into established media representations/playlists with pinned provenance.
- [ ] Keep native QDNF object identity/policy distinct from media player presentation and codec support.

Acceptance:

- [ ] Stock player interoperability tested only for supported media; no claim players decode Q42/P64/EMF.
- [ ] Transcoding/projection permissions, live gaps and cache boundaries remain enforced.

## QBS-25 — R1 integration and release record

**Status:** pending · **Release:** R1 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-23. **Reviewer:** programme integrator + release reviewer.

**Proposed write scope:** docs: release record, supported feature matrix, migration and examples; shared integration only by integrator.

Deliverables:

- [ ] Publish reproducible build/feature/test instructions and actual support matrix; reconcile parent programme evidence.
- [ ] Update format/protocol docs from actual behavior and prepare reviewable release artifacts.

Acceptance:

- [ ] All R1 packages accepted with independent review; examples use real native, HTTP and WSS paths.
- [ ] No automatic deployment/publication/charges; no pending required security gate hidden as future enhancement.

## QBS-26 — WebSocket reliable delivery binding

**Status:** pending · **Release:** R1 · **Lead tier:** senior · **Planning cap:** 40,000 tokens (estimate, not measured use).

**Depends on:** QBS-04, QBS-08, QBS-09, QBS-10. **Reviewer:** WebSocket security + runtime reviewer.

**Proposed write scope:** QualiaDB: bundle/stream/websocket/ (proposed); browser WSS adapter via exact owner claim.

Deliverables:

- [ ] Implement binary negotiated records over WSS, bounded authentication/control, credits, priority and object-bound reconnect.
- [ ] Use reviewed reliable-carrier behavior and PQ application protection; avoid duplicate transport recovery.

Acceptance:

- [ ] Origin/credential abuse, compression, fragmentation, huge messages, queue floods, replay and reconnect tests pass.
- [ ] No URL tokens; browser API limitations explicit; WSS/TLS alone never labeled PQ; parity with native and HTTP.

## QBS-27 — WebRTC data-channel binding

**Status:** pending · **Release:** R2 · **Lead tier:** expert · **Planning cap:** 60,000 tokens (estimate, not measured use).

**Depends on:** QBS-04, QBS-09, QBS-10, QBS-15, QBS-25, QBS-26. **Reviewer:** WebRTC networking + privacy + crypto reviewer.

**Proposed write scope:** QualiaDB: bundle/stream/webrtc/ (proposed); QDNF signaling/CSCP carrier adapters via exact claim.

Deliverables:

- [ ] Implement reliable ordered binary mode with authenticated signaling and reviewed PQ session/object boundary.
- [ ] Admit ICE candidate disclosure and STUN/TURN selection before gathering; support bounded fragmentation and credit.

Acceptance:

- [ ] Direct/relay, maxMessageSize, buffering, glare, restart, replay, channel reset and consent-loss tests pass.
- [ ] No privacy-downgrade fallback; DTLS is not claimed PQ; no double recovery over reliable SCTP; independent browser peers interoperate.

