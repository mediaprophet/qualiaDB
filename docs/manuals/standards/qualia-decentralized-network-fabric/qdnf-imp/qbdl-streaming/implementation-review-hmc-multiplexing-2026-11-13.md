# Implementation review: HMC multiplexing, QDNF streaming and EMF/QAC integration

Review date: 13 November 2026 (Australia/Sydney). This is a source-and-evidence review of an actively changing working tree, not an independent test run or release approval. The original build handoff is historical; another implementation session has since advanced the programme. This review changes documentation only and does not take ownership of the active implementation files.

## 1. Recommended scope correction

The product should be **selective, governed streaming of a heterogeneous HMC bundle over QDNF**, with multiplexed delivery of its entries and their dependencies. The binary HMC profile uses QBDL bytes. M64 is one inner content format and one useful partial-access test case; it must not become the organizing model of the transport.

Explicitly include .10d, .q42, .emf, .qac, .m64/.e64 and .p64, plus manifests, metadata and other authorized opaque assets. Not every entry is a timed media track. Static geometry, query data, scientific observations and model resources have different readiness and interpretation rules. Preserve those distinctions while sharing transport, authenticated selection, credits, cancellation, storage and policy.

The user's “streaming a mux with intelligent capabilities” is the right product description if mux means coordinated multiplexing of related assets. QBDL currently stores intact entry bytes followed by an index; it is not already an interleaved audiovisual mux with a universal clock. Implement multiplexing in the transfer/session planner and typed metadata rather than silently rewriting the container format or replacing it with HLS/DASH.

The .hmc extension also identifies an existing ZIP document profile. It is not byte-compatible with QBDL. This programme presently targets **QBDL-profile HMC**. Supporting ZIP HMC requires a separate, explicit bounded admission/transfer design or a deliberate repackaging operation with new content identity. Do not sniff by extension alone or advertise all HMC profiles as supported by the QBDL reader.

Reference contracts: [HMC standard](../../../hmc-container-format.md), [QAC standard](../../../qac-acoustic-container.md), [full-spectrum field contract](../../../full-spectrum-field-contract.md), [EMF/QAC evolution programme](../../../../../plans/emf-qac-evolution-2026-11-13/README.md).

## 2. Progress observed since the earlier handoff

The evidence below was read from existing files, not rerun during this review. Test counts are evidence for their stated scope, not additive proof of a current release. Source hashes for key reviewed files are in evidence/hmc-mux-review-source-snapshot.json.

| Area | Observed progress | Remaining qualification |
|---|---|---|
| Canonical authority identity | Owner/handle/permit incarnation binding landed; swarm-s1-authority-tests-retry.log records 37 passes | Do not repeat the old handoff's owner-replacement finding as if still unfixed; preserve regression coverage |
| Remote authorization foundations | Recognition, decision and policy modules plus issuer/grant/current lifecycle files exist; S2 run log records 11 remote tests | Newer lifecycle integration must compile and execute; local recognition is not a universal controller-chain qualification |
| Storage jobs | S2 run log records 5 runtime-job tests and 1 native arming test | Existing production write_leaf still performs I/O inside StorageStage; native arming wrapper is not a complete write-job integration |
| Native local stream | Windows named-pipe framing/accept/exchange tests recorded 18 passes | This is local OS-process evidence, not two-host physical QDNF or Linux runtime evidence |
| QBDL service | Plain provider/client, generic Bearer session and two-process leaf demonstration added; progress record reports 2 two-process passes and log records TWO_OS_EXIT=0 | Authenticated remote descriptor bootstrap, protected QSession composition, real runtime credits, multiplexing and consumer commit remain distinct work |
| M64 v3 | M64GeoRasterDirectoryView now routed; shared georaster/partial-admission work is active | Current selected build log includes an M64 link failure; do not infer current acceptance from source routing or review alone |
| Protected B2 wrapping | authorize_and_wrap exists and has a saved review | It currently takes LocalWholeSourceGrant; remote grant/controller integration, receiving, private metadata and durable promotion need their own evidence |
| EMF/QAC evolution | Canonical QAC naming, legacy complete admission, candidate shared envelope and metadata validation are documented and present | Candidate structural access does not establish complete acoustic/electromagnetic scientific admission or streaming adapters |

The reviewed service provider is format-neutral: it uses SelectedPlainEntry, OutboardCatalog, entry offsets and proof geometry. I found no M64-specific transport dependency in this path. The issue is an incomplete integration/acceptance target and outdated format coverage, rather than evidence that all service code needs replacing.

## 3. Findings and required corrections

Priority P1 means resolve before treating this path as production-capable. P2 means scope/coordination work needed for the intended product. “Confirmed” refers to the inspected source or recorded diagnostic; it does not imply an independently reproduced exploit. Active edits may supersede a finding; close it with current source and test evidence.

### R01 — P1: the demonstration service bypasses the protected-channel composition

**Confirmed integration gap.** net/peer/replication/qbdl/service/session.rs constructs Session from a generic Bearer and a caller-supplied transcript digest. send_message encodes ordinary service frames and passes them directly to endpoint.send (lines 126–174). recv_complete parses frames directly. The two-process fixture supplies TRANSCRIPT and reconstructs its selected entry/trust material locally.

A transcript value in a frame is not a cryptographic handshake or peer authentication. The implementation demonstrates framing, process transport and a useful authenticated leaf against pre-established fixture roots; it does not demonstrate a fresh PQ-authenticated peer, protected session or remote acquisition of trusted inventory. Comments calling these frames “sealed” must not be used as evidence of AEAD.

**Correction:** retain the useful framing fixture, but distinguish it clearly from the production service. Compose production I/O through the existing established protected channel and retained runtime bridge, using actual handshake-derived peer/transcript state, current authority and original reservations. Define a narrow production capability boundary that cannot be manufactured from arbitrary digest bytes. Reuse the existing QDNF crypto owner; do not implement a parallel protocol or raw-key API.

**Acceptance:** fresh handshake between processes, wrong-peer/wrong-transcript and downgrade rejection, AEAD corruption/replay tests, expiry/revocation at the application boundary, actual Describe/descriptor/index bootstrap, and one restricted transfer whose authorization is not reconstructed independently by a test fixture. Qualify process and physical-network evidence separately.

### R02 — P1: RequestLeaves does not receive the same identity check as Credit

**Confirmed validation omission.** service/provider.rs answer_leaf validates self.associate(&credit) at line 142, then compares RequestLeaves against Credit for transfer ID, entry, leaf, count, serial and credit bytes at lines 148–159. It does not call self.associate(&request). The descriptor, context, publication generation and policy generation of RequestLeaves are therefore not checked by that method. Carrier transcript/message-order checks do not supply these missing application associations.

**Correction:** validate the full request association independently before use, and validate its relation to the live opened transfer. Include deadline/cancellation semantics and the selected entry. Do not rely on a matching adjacent credit record to authenticate fields that were never compared.

**Acceptance:** start with valid Open and Credit, then change only each of the four identity/generation fields in RequestLeaves using a fresh carrier message ID. Every mutation must refuse before reading/disclosing payload or changing accounting. Add cross-bundle and stale-generation cases.

### R03 — P1: wire credit is not yet conserved runtime credit

**Confirmed integration gap.** PlainProvider contains source, catalogue and policy generation; answer_leaf takes &self. OpenedTransfer is a public Copy record. Credit serials are checked for nonzero and agreement with the following request, but no spent-credit state or canonical ledger ticket is consumed in this path. New carrier message IDs can carry repeated credit fields; carrier anti-replay is not the same as single-use resource credit.

**Correction:** integrate the existing ledger/lease/operation/scheduler ownership. A transfer must refer to an actual admitted, session-bound entry/closure and original reservation. Consume credit once for actual work and retain occupancy through I/O completion/cancellation. Separate request identifiers from session/transfer ownership without fabricating a second accounting subsystem.

**Acceptance:** repeat the same credit serial in fresh messages, reuse it across entries/sessions, cancel while work is pending, retry after WouldBlock and deliver duplicate completions. Reject or handle defined idempotent cases without duplicate disclosure/accounting, overdraw, early buffer reuse or double refund. Demonstrate accounting conservation for multiple streams.

### R04 — P1: authority freshness and deadlines do not cover blocking disclosure

**Confirmed integration gap.** Provider authorization is checked before waiting for Credit/RequestLeaves (provider.rs:137). The eventual payload send is at line 233, with no intervening authority.recheck in that method. Session send/receive loops sleep on WouldBlock with a new five-second timer per frame (session.rs:161–174, 278–291). Service headers use a constant DEADLINE=90 (lines 21 and 350), rather than the admitted operation's clock/deadline. An outer failure after bytes were sent cannot retract those bytes.

**Correction:** move waiting into a cooperative retained state machine. Use the admitted clock/deadline and cancellation epoch, recheck current authorization before each disclosure step, and bound the whole operation as well as each poll. Do not hold CurrentAuthorityGuard or RuntimeParents across sleeping I/O. Preserve partial-write cursors and original sealed data across WouldBlock. The protected-channel/runtime owner should enforce this, not merely a test's after-the-fact callback.

**Acceptance:** expire or revoke after valid Open but before delayed RequestLeaves; expire between output frames; continuously deliver just-before-timeout fragments; cancel another stream while one is blocked. No subsequent unauthorized payload may escape, and other control traffic must remain serviceable.

### R05 — P1: native write arming does not yet own a real durable job

**Confirmed gap.** storage/native/write.rs:88–126 still calls store_verified while holding the StorageStage buffer scope. storage/native/job.rs:14–29 defines ArmedNativeWrite as a reserved_bytes scalar; wait_blocking simply invokes an arbitrary closure. Its absence of guard parameters does not prevent that closure from capturing held guards, and the scalar does not own a backend operation, buffer lease, completion identity or durable result. The new helper is useful scaffolding; its documentation is stronger than the guarantee currently enforced.

**Correction:** connect actual bounded jobs to NativeSparseStore and the canonical runtime completion key. Make the submit/wait/complete ownership explicit, retain charged buffers while pending, perform writes/sync outside parent locks, then reacquire owners and revalidate before verified marking/commit. Decide how incomplete/failed/cancelled writes remain candidates. Replace the live production call path, not only its demonstration hook.

**Acceptance:** instrument actual write/sync sites with lock-availability checks, cancel/revoke during slow I/O, crash at existing payload/proof/bitmap/header boundaries, duplicate completion, disk-full and buffer-reuse tests. Verify exactly-once release and never mark unauthenticated bytes verified after restart.

### R06 — P2: coverage currently permits an M64-centric success story

**Confirmed planning gap.** QBS-12's deliverables list Q42/10D/EMF/M64, but omit QAC. Its acceptance requires one scientific brick and one geometry section; it does not establish a mixed HMC multiplex or simultaneous independent consumers. PlainClient binds one selected entry and the demo transfers one leaf. That is a valuable slice, not completion of the user's container-level requirement.

**Correction:** extend QBS-09/12/13/17/23/25 acceptance with the mixed-bundle scenario in section 6. Keep M64 work as a bounded adapter task. Add authenticated inventory/selection, cross-entry dependencies, multiple active requests, fairness and consumer-ready events to the service integration backlog. Avoid a new M64 transport or separate per-format network protocols.

### R07 — P1/P2: QAC and evolving EMF need explicit versioned admission contracts

**Confirmed coordination gap.** bundle/stream/formats/mod.rs routes EMF v2/v3, M64, 10D and Q42 adapters but no QAC adapter. The new QAC standard distinguishes legacy complete-object numeric validation from opt-in candidate shared-envelope structural access. Candidate field envelopes expose unsupported critical semantics for family adapters to reject before scientific use. These are materially different from the old EMF brick-plus-trailer assumption.

**Correction:** coordinate with the EMF/QAC codec owner before freezing streaming metadata. Dispatch on actual bytes/version/profile, not filename. Reuse audio::qac, q42::field_envelope and field_semantics as appropriate; do not copy those parsers into network code. Initially transfer legacy QAC as a complete authenticated entry and invoke its canonical complete reader. Only add partial legacy views after defining and testing their full interpretation closure. Candidate QAC/EMF partial admission must authenticate header/directory/metadata, selected complete chunks and schema/reference dependencies, then enforce supported domain semantics. Unsupported scientific interpretation may remain explicitly opaque archival data, never silently become playable audio or validated measurements.

Changing .q4au/.bin naming to .qac is not transcoding. Do not automatically rename existing HMC entry keys, rewrite coupling IDs or recalculate artifacts as a side effect of transport. Explicit repackaging produces a new immutable identity and rebuilds affected hashes/manifests. Preserve full BLAKE3 digests; do not invent trailers for legacy QAC, whose payload integrity must come from appropriate outer authentication.

**Acceptance:** mixed old/new profiles, malformed recognized magic with no fallback, unknown critical schema, unresolved clock/frame/calibration references, truncated later planes, last digest-byte corruption, exact signed-zero/sample bytes, opaque preservation and unchanged historical HMC keys. Validate physical units and the full spectral contract; do not reduce EMF to visible colour or QAC to audible playback.

### R08 — P2: multiplex timing and intelligent scheduling are not yet an acceptance contract

**Design requirement, not a claim that every useful scheduler is absent.** Existing runtime scheduling and manifest/dependency code are reuse points. What remains unspecified is the product-level relationship among timed QAC/EMF, static 10D geometry, Q42 data, M64 fields and P64 resources, including what constitutes a ready coherent view.

Use the contract in sections 5–6. Bind relationships to immutable entry/descriptor identities, not coincident filenames, entry order or bare Hilbert indices. Never invent clocks or temporal equivalence where the source lacks them. Distinguish a late optional enhancement from a missing mandatory dependency and from a revoked input.

### R09 — P1 until rerun: newer integration evidence is not yet green

**Recorded diagnostic.** At review, evidence/swarm-s3-s4-two-process-m64.log contains M64_EXIT=101 after LNK1104 and newer E0277 diagnostics in policy/remote/lifecycle_tests.rs:423,525,579,613: unwrap_err requires Debug on the success type (AuthorityReceipt or RemoteDisclosureGrant). The worker may already be correcting this; this review does not assume the transient tree is stable.

**Correction:** use explicit error matching or another assertion that does not require success-capability Debug solely for test convenience. Do not expose sensitive material through derived Debug to satisfy an assertion. Finish the current serialized compile and run nonzero focused selectors against the emitted binary. Keep link-environment failures distinct from source errors, and preserve successful retry evidence. Do not reuse the earlier 345-pass checkpoint as proof for newer files.

### R10 — P2: tracker and old handoff disagree with active progress

The registry's scope_note still describes the earlier pause, while progress-log.md records S1–S4 execution. The old handoff says authority incarnation and two-process work remain open; some of that has now progressed. Several task evidence arrays remain empty. Conversely, an independent review with open_bugs:0 does not prove a whole service has integrated its required owners.

**Correction:** the active integrator should reconcile registry, work packages, current evidence and checkpoint. Preserve the dated handoff as history, add a current continuation pointer, and record per-subcontract source/test scope. Do not mark all of QBS-09 complete because one process fixture works. Distinguish Windows process, Unix process, independent-host network, HTTP/WSS, browser and protected-delivery gates.

## 4. Additional verification questions, not confirmed defects

- B2 authorize_and_wrap currently receives LocalWholeSourceGrant and caller-supplied pin proof. Prove how the recognized source-controller enrollment is carried into production wrap authorization; test a different locally trusted paired pin with the right purpose/audience but no rights over this source. Do not infer this binding from a valid signature alone.
- Re-run the existing WSS audit on the current source before adapting it: fresh client masking, retained partial I/O cursors, strict framing, actual shutdown and circuit/generation binding. Older findings must not be copied forward without checking active changes.
- Qualify the newly restored M64 v3 view's exact promise, including cross-chunk RepeatNearest padding. Selected in-bounds samples are not automatically full brick or volume validation.
- Verify that consumer references and semantic-query completeness distinguish available verified ranges from absent data. A partial Q42 response must not silently claim a complete answer.

## 5. Target composition: an intelligent HMC multiplex

    QDNF discovery + recognized PQ authority
        -> authenticated HMC/QBDL descriptor, index and dependency inventory
        -> bounded selection plan + existing policy and resource reservations
        -> multiplexed entry/range requests with independent credits and control
        -> native protected QSession / HTTP / WSS / later WebRTC adapters
        -> per-entry proofs + optional object decryption
        -> versioned inner-format admission and dependency readiness
        -> consumer commit: scene, scientific view, query, playback or model

Reuse the existing detached descriptor and manifest rather than introducing a second unbound graph. Any required new metadata field must have a versioned, bounded, authenticated contract and unknown-critical behavior. Extend existing interfaces through their owners before allocating new wire types.

The planner needs, where applicable: source descriptor/generation, entry ID and actual format profile; role/representation; declared dependency edges; exact rational timebase/epoch and intervals for timed content; coordinate/frame/unit/calibration references; alternative representation relationships; mandatory/optional classification; resource estimates and limits; permitted consumer purpose. Many static assets legitimately have no time interval.

Initial intelligence should be deterministic and explainable: select requested semantics/time/region, close mandatory dependencies, prioritize interpretation metadata and base representations, schedule by urgency within fairness/resource bounds, adapt concurrency to measured throughput, reuse verified cache entries, and report why an asset is deferred. Coordinate related EMF/QAC/10D/Q42 views without assuming they share the same sampling grid or clock.

Optional learned or solver-assisted planning can propose choices later. It cannot mint authority, bypass proof verification, fabricate absent observations, widen a grant, exceed reservations, or drop mandatory dependencies. Pre-solved-query/QPU Commons reuse is another validated result source; the transport still delivers its artifacts through the same fabric and policy.

Keep layers distinct: **entry**, **semantic segment**, **authenticated leaf**, and **carrier frame** are different units. A segment can require several leaves and sidecars; a leaf must not cross entry/permission boundaries. Multiplexing does not require simultaneously interleaving arbitrary bytes inside every low-level frame: a bounded scheduler can switch at admitted record boundaries while servicing control promptly. Measure head-of-line blocking before changing the carrier contract.

## 6. Required mixed-HMC acceptance scenario

Construct a canonical QBDL-profile HMC with one actual valid entry of each supported selected family: .10d geometry, .q42 data, .emf field, .qac acoustic content, and .m64 field. Add P64 complete-entry coverage as the model consumer gate requires, and one optional opaque asset. Use real canonical writers/readers and authenticated relationships; do not merely label byte blobs with extensions. Start with public content, then compile a separately authorized restricted representation.

1. Bootstrap descriptor/index/dependencies from the provider; the client must not already reconstruct the complete source fixture. Request a scene/analysis selection, not an M64 filename alone.
2. Obtain enough authenticated metadata and geometry to expose a valid partial result without downloading the entire HMC. Track whole-HMC CRC/hash completion separately.
3. Concurrently retrieve an EMF region and QAC time interval where supported, plus their 10D/frame/calibration/Q42 dependencies. Use complete-entry QAC fallback until a precise partial contract is implemented.
4. Delay M64 or P64 delivery and demonstrate that unrelated ready consumers/control messages continue. Shared dependencies are admitted and charged once under their real owners.
5. Seek/change region and cancel obsolete work; retain pending I/O ownership until completion. Reprioritize without double credit, duplicate commit or indefinite starvation.
6. Show a missing/unsupported dependency as incomplete, not valid silence, an empty field or a complete query answer. Unknown optional content stays opaque; unknown critical interpretation refuses.
7. Mutate cross-entry identities, generation, request context, metadata, proof, last digest bytes and scope; reject before consumer commit. Revoke during a blocked send/write and prevent later disclosure/promotion.
8. Demonstrate restart with sparse candidates and revalidation, then the same admitted outputs over native and HTTP/WSS bindings. Browser/WebRTC gates remain separately recorded until executed.
9. Verify original entry bytes and names are preserved where no explicit transform was requested. No hidden resampling, normalization, format upgrade or automatic .qac key rewrite.
10. Record bounded RAM/work/I/O, first useful result, starvation/control latency, cancellation latency and actual requested/transferred ranges. Set workload-specific thresholds before claiming performance success; do not invent universal targets.

## 7. Corrective work sequence for the active integrator

| Slice | Existing package owners | Required output / completion gate |
|---|---|---|
| C0: current integration health | QBS-01/17, integrator | Resolve active compile diagnostics; record exact source/binary and nonzero tests; reconcile stale tracker notes |
| C1: authority and request lifecycle | QBS-04/07/08/09/10 | R01–R04 fixes; real protected channel, request identity, conserved credits, current deadlines and remote grant binding |
| C2: durable jobs | QBS-08/11 | R05 real write pipeline outside owner locks, retained buffers, crash/cancel/restart evidence |
| C3: evolving codec contracts | QBS-12 plus EMF/QAC owners | Version/profile capability matrix, QAC complete fallback, shared-envelope semantics and 10D/Q42 dependency requirements; keep M64 adapter work bounded |
| C4: mixed HMC orchestration | QBS-06/07/08/09/13 | Authenticated bundle inventory, multi-entry selection, readiness and deterministic scheduling; first mixed-bundle native scenario |
| C5: compatibility and consumers | QBS-14/15/16/26 | HTTP/WSS parity, browser storage/worker path, Etalon and Commons ownership; carrier-independent outcomes |
| C6: qualification | QBS-17/23/25 | Mixed-HMC adversarial suite, independent process/host/platform records and release claims matching actual evidence |
| Later extensions | QBS-18/19/20/21/22/24/27 | Live generations, distributed accounting, QPU Commons, selective models, measured format extensions, audiovisual projections and WebRTC |

These are corrective slices within the existing registry, not a competing task-ID system or authority to overwrite another worker's files. The active integrator should translate them into explicit scoped claims. Preserve useful work already done.

To control cost, use one integration owner, a bounded codec liaison/inventory task and small fixture tasks after contracts are fixed. Reserve stronger review for the protected-channel/current-authority/accounting diffs. Do not launch another full-history swarm or repeat the whole kernel audit. No workers, builds, production-code edits or external messages were launched by this review.

## 8. Closing criteria and limits of this review

The correction is complete when HMC-level scope is explicit, current compile diagnostics are closed, R01–R05 production integration is demonstrated, evolving EMF/QAC and existing 10D/Q42 contracts are reconciled, and the mixed-bundle scenario passes with truthful verification states. M64 success alone is insufficient; it remains valuable evidence for its adapter.

This document is not a full cryptographic audit, exhaustive source review or independent platform run. Existing logs may predate subsequent edits. Source line anchors refer to the inspected snapshot; verify hashes/current code before applying a correction. No claim is made that optional future modes, every .hmc profile or all scientific semantics are already implemented.

## Source navigation

- [net/peer/replication/qbdl/service/mod.rs](../../../../../../crates/qualia-core-db/src/net/peer/replication/qbdl/service/mod.rs)
- [net/peer/replication/qbdl/service/session.rs](../../../../../../crates/qualia-core-db/src/net/peer/replication/qbdl/service/session.rs)
- [net/peer/replication/qbdl/service/provider.rs](../../../../../../crates/qualia-core-db/src/net/peer/replication/qbdl/service/provider.rs)
- [net/peer/replication/qbdl/service/client.rs](../../../../../../crates/qualia-core-db/src/net/peer/replication/qbdl/service/client.rs)
- [net/peer/replication/qbdl/service/two_process.rs](../../../../../../crates/qualia-core-db/src/net/peer/replication/qbdl/service/two_process.rs)
- [net/peer/replication/qbdl/carrier/receive.rs](../../../../../../crates/qualia-core-db/src/net/peer/replication/qbdl/carrier/receive.rs)
- [net/qdnf/authority/current_guard.rs](../../../../../../crates/qualia-core-db/src/net/qdnf/authority/current_guard.rs)
- [bundle/stream/storage/native/write.rs](../../../../../../crates/qualia-core-db/src/bundle/stream/storage/native/write.rs)
- [bundle/stream/storage/native/job.rs](../../../../../../crates/qualia-core-db/src/bundle/stream/storage/native/job.rs)
- [bundle/stream/protection/wrap.rs](../../../../../../crates/qualia-core-db/src/bundle/stream/protection/wrap.rs)
- [bundle/stream/policy/remote/recognition.rs](../../../../../../crates/qualia-core-db/src/bundle/stream/policy/remote/recognition.rs)
- [bundle/stream/formats/mod.rs](../../../../../../crates/qualia-core-db/src/bundle/stream/formats/mod.rs)
