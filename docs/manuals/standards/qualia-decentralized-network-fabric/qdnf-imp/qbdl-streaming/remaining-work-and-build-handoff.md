# QBDL streaming: remaining work and build handoff

**13 November 2026. Build-focused handoff; the complete streaming programme is unfinished.**

The user reduced the immediate scope because token quota was nearly exhausted: leave a clean build so concurrent repository work can continue, and document everything still needed. Feature implementation and agent delegation were frozen. This document supersedes older checkpoint statements that a worker or build is still running; final execution results are recorded in the build section below. No commit, push, deployment or release is authorized by this handoff.

## 1. Outcome and limits

QBDL streaming is being implemented as a specialization of the QDNF fabric, with HTTP and WebSocket delivery adapters. It is not an HTTP-first replacement network. QPU pre-solved-query reuse belongs within the wider permissive Commons system. WebRTC is an R2 transport extension. HLS/DASH remains optional audiovisual presentation interchange.

There is substantial compiled protocol, crypto, range-admission and runtime infrastructure, but no fully qualified end-to-end streaming product. A passing core build does not establish native two-process service, remote recipient authorization, HTTP/WSS/browser delivery, Commons accounting or release readiness. Do not advertise these as completed capabilities.

The existing format standards and full BLAKE3 repairs are retained. Truncated historical BLAKE3 comparisons are defects, not standardized integrity profiles. Explicit legacy migration reads must remain visibly weaker and must never acquire full-verification status. QBDL SHA-384 transfer proofs and an inner format's full BLAKE3 checks are separate layers; neither licenses inventing omitted inner data.

## 2. Build and test evidence

The current core library and all core test targets pass:

    cargo check -p qualia-core-db --tests --locked --offline -j 1

Exit 0, 4m19s; evidence/core-clean-build-check-retry.log. Repairs at this handoff were the M64 SDK verification expectation (both hash counters are zero for its unhashed fixture) and the publisher test import through existing crypto-library public exports.

The linked core test build also passed (2m49s, evidence/core-build-handoff-linked.log) after a Windows LNK1104 executable-access failure. The subsequent fixture rebuild encountered the same file-access error and also succeeded on retry; both failure logs are preserved separately.

The first current-source run passed provider/carrier 18, endpoint 33 on the normal stack, IPC 4, EMF whole-reader 16, M64 39, mesh 16, parent runtime 27 and managed object 4. Stream tests passed 174 with two newly authored EMF v3 fixture failures: the fixture supplied a guessed Hilbert index outside its declared grid. The fixture now calls the canonical encoder; the final streaming rerun passed all 176 tests. Production validation was not weakened.

The broad model selector completed 12 synthetic cases, then was deliberately stopped during physical-model inference to respect the reduced build-and-handoff scope. Its process exit -1 is interruption, not a completed test failure or pass. The four physical Gemma/Qwen model paths remain unqualified after the supplier alias repair. The separate bounded synthetic selector passed all 12 tests in the final rerun.

Earlier evidence, useful but not a substitute for the final source snapshot:

- Windows actual-source focused conformance: 268 passed, in evidence/runtime-carrier-focused-link-retry.log. Linux actual-source focused conformance: 270 passed, in evidence/wsl-runtime-carrier-focused.log. These exercise shared production modules, not a full native service.
- Pinned supplier model alias regression: 108 passed, evidence/etalon-vendor-alias-regression-retry.log. Canonical Etalon mirrors the repair but was not separately compiled in that increment.
- Previous core binary c4560c081fa48873: stream 148, mesh 16, current-authority guard 3, native wire 6, parent runtime 25 and GGUF sharder 25 passed. That binary predates the final integration batch.
- The same older binary reproduced four physical-model admission failures and an endpoint stack overflow. Alias, stack-construction and error-preservation repairs were subsequently integrated. The large-stack diagnostic was diagnosis only and must not count as normal-stack acceptance.
- Earlier normal-access socket/mesh tests passed; restricted-environment socket/WSL failures were not papered over. No security controls were disabled.

Logs live in evidence/. Distinguish a successful compiler exit, an executed test result, an independently generated fixture and an agent review. Authored tests, routed modules and fixture success are not release gates by themselves. Existing warnings are not represented as a warning-free build.

Final handoff result: **345 affected tests passed across the focused runs**, with no remaining failure in that selected set. The final linked build completed in 1m46s (evidence/core-build-handoff-linked-repair.log). Final rerun: stream 176 and synthetic Gemma 12; unchanged-source groups from the preceding run: provider/carrier 18, endpoint 33, IPC 4, EMF 16, M64 39, mesh 16, runtime 27, managed object 4. Only the EMF test fixture changed between these executions. evidence/build-handoff-execution.json and evidence/build-handoff-execution-repair.json identify the executables and logs. The first record intentionally retains the earlier failures and interrupted physical-model selector; it must not be mistaken for the final selected result.

Scope: Windows default-feature core library/test-target compilation and linked focused regressions. This is not an all-workspace/all-features build, a complete 10,487-test core run, or qualification of every sibling repository. No compiler warnings were suppressed.

## 3. What is implemented and where

All paths in this section are relative to crates/qualia-core-db/src/ unless stated otherwise.

| Component | Actual source / responsibility | Boundary still open |
|---|---|---|
| Shared QBDL admission | bundle/stream/index and whole/progressive reader wrappers | Complete service-wide use and final release corpus |
| Paired PQ signatures and key custody | crypto/network/cose; specialized_libs/cryptographic_library/signing_profile and key_management/managed_signing, managed_control, managed_object | Remote controller recognition and recipient-release policy |
| Transfer trees | bundle/stream/proof, including canonical sibling paths | Live provider scheduling and persisted outboard custody |
| Descriptors and manifests | bundle/stream/descriptor, publish, policy/manifest/transport | Durable publication and full semantic/remote policy closure |
| Fixed runtime | bundle/stream/runtime/state; net/peer/runtime/incarnation | Real service ownership and asynchronous storage jobs |
| Wire/carrier/provider | net/peer/replication/qbdl/wire, carrier, provider/outboard | Complete native provider/client and OS bearer qualification |
| Protected channel | net/qdnf/session/handshake/endpoint/established/protected_channel | Remote source rights and full application orchestration |
| Protected publisher B1 | bundle/stream/protection; managed_object/effect and publication_signing | B2 recipient wraps, received private metadata and durable promotion |
| Sparse storage | bundle/stream/storage and storage/native | Jobs outside runtime/authority lock scope |
| Typed partial data | bundle/stream/formats; q42/emf_container/extension/range; container_10d/mesh_section | M64 v3 and end-to-end consumer coverage |
| Model repairs | gguf_bridge/gemma4; supplier tensor_roles and GGML codec backport | Product token-loop adoption and streaming residency bridge |

Retained runtime state moves the original non-copy handles into the existing runtime and restores them on success, failure and unwind. It does not mint a second budget ledger. Parent identity checks refer to the actual ledger/lease/operation/scheduler values, not only mutex addresses. The channel bridge carries the original endpoint reservation. Carrier assembly has bounded independent data/control lanes, monotonic identifiers, exact parsing and zeroization on error/unwind; it does not itself grant authority or credit.

EMF v3 partial admission uses shared canonical extension-tail and directory validation, complete metadata/PCM closure, and selected-brick verification. A selected brick is not proof that every other brick in a volume is valid. Likewise the 10D path admits a complete selected mesh section, not every optional container section; Q42 ranges require complete authenticated coverage for the requested operation.

The protected publisher uses the existing managed crypto library. It produces encrypted candidates with bounded opaque identifiers, canonical geometry, per-leaf sealing and paired descriptor signatures. Private metadata is erased on failed attempts. A candidate is not an authorized remote publication and is not durably committed merely because its bytes are structurally valid.

## 4. Remaining task register

Only QBS-00 is recorded complete. In-progress tasks contain implemented subcontracts, not completed release acceptance. The machine-readable task-registry.json remains the dependency authority; work-packages.md contains original write scopes and acceptance contracts.

| Task | Recorded status | Remaining acceptance work |
|---|---|---|
| QBS-00 | complete | Baseline accepted. Preserve original dirty-tree evidence; refresh revisions and build fingerprints at each future integration. |
| QBS-01 | in_progress | Finish registry assignments and reconcile wire/CDDL/interfaces with executable limits and failure classes. Freeze only after independent fixtures and service interoperability agree. |
| QBS-02 | in_progress | Shared bounded preflight exists. Qualify every whole/progressive/provider entry point against the same flags, checked extents, duplicate/overlap rules and allocation limits. |
| QBS-03 | in_progress | Paired COSE and library-managed signing exist. Complete protocol-level vectors and cross-provider review; reject classical-only or mismatched-pair authorities throughout service paths. |
| QBS-04 | in_progress | Local paired pins, managed policy and current guards exist. Close owner-incarnation gap, controller-chain/remote-source recognition, revocation/expiry persistence and session-bound key release. |
| QBS-05 | in_progress | Canonical SHA-384 leaf/proof/path implementation exists. Finish provider outboard integration, independent boundary corpus, byte-budget and worst-case traversal evidence. |
| QBS-06 | in_progress | Plain paged descriptor/publisher and protected candidate composition exist. Connect immutable publication, durable sinks, renewable locators/grants and actual provider catalogue; exercise sink failure and cancellation. |
| QBS-07 | in_progress | Canonical manifest/dependency and bounded closure foundations exist. Complete source-controller authorization, consent, Sentinel/Webizen admission, label joins, query completeness and decision-ticket expiry. |
| QBS-08 | in_progress | Fixed runtime slots, reservations, retained state and channel bridge exist. Integrate one real service owner; move blocking storage outside borrowed owner locks without duplicating accounting. |
| QBS-09 | in_progress | Wire, carrier, provider outboard and protected-channel primitives exist. Build real native provider/client over an actual OS bearer with source custody, requests, credits, proofs, cancellation and restart. |
| QBS-10 | in_progress | B1 protected publisher candidate exists. Implement B2 authorized recipient wrapping, private metadata decode, received-leaf custody and durable promotion; no raw-key bypass. |
| QBS-11 | in_progress | Sparse verified storage and killed-writer recovery tests exist. Connect bounded storage jobs, completion ownership, cancellation and authenticated restart to the live runtime. |
| QBS-12 | in_progress | EMF, M64/E64 v1/v2, 10D mesh and Q42 range paths exist. Finish M64 v3 drafts with shared canonical validation and cross-chunk padding policy; qualify each supported partial operation. |
| QBS-13 | pending | Connect complete verified P64/model entry to Etalon staging/residency, alignment, CPU admission and coupled memory budgets. Model-format fixes alone do not constitute streaming integration. |
| QBS-14 | pending | Implement strict HTTP/LIG range provider/client using identical objects/proofs/policy; test 200/206/416, exact Content-Range, representation validators, redirects, truncation and transformed content. |
| QBS-15 | pending | Implement worker ownership, OPFS candidate/verified separation, quota/cancel/restart, bounded WASM transfer and actual browser consumer; qualify named browser engines. |
| QBS-16 | pending | Attach existing Commons exemptions and one authoritative finite-obligation owner to transfer lifecycle. Test replay, cancel, sponsor exhaustion and duplicate/refund behavior. |
| QBS-17 | in_progress | Independent parser/COSE/descriptor/proof/wire corpora exist. Add separate-process Windows/Linux peers, cross-implementation transcripts and release reproducibility. |
| QBS-18 | pending | R2: immutable generation discovery and subscriptions, sensing permissions, expiry, missed updates and bounded retention. Do not mutate an already signed generation. |
| QBS-19 | pending | R2: distributed escrow and finite-recovery accounting under partition/replay/restart; reuse existing ownership and settlement primitives. |
| QBS-20 | pending | R2: generic solver-result Commons lookup, validated witnesses, historical exact-key reuse and QPU escalation. Bind provenance, rights, result validity and accounting. |
| QBS-21 | pending | R2: selective P64/operator execution only where tensor/operator dependencies, alignment and residency are independently admitted. Keep complete-entry fallback. |
| QBS-22 | pending | R3: measure alignment/staging overhead before introducing any container extension. A documented no-extension decision is valid; new flags require negotiated versioning. |
| QBS-23 | pending | Run adversarial, memory, restart, native/browser/platform and policy qualification over actual service. Publish failures and unsupported backends; no proxy from unit-only evidence. |
| QBS-24 | pending | R3: optional HLS/DASH projections for audiovisual representations; preserve QDNF identity/policy and keep generic scientific/model objects native. |
| QBS-25 | pending | Close all R1 gates with independent review, reproducible release commands, migration/interoperability documentation and explicitly supported deployment profiles. |
| QBS-26 | pending | Repair existing WSS incremental framing/masking/shutdown and bind QBS records with original budgets/current authority. Test fragmented/partial writes, WouldBlock, close, replay and zero-heap hot paths. |
| QBS-27 | pending | R2: governed WebRTC signaling/ICE/data channels with bounded reassembly, reliable ordered mapping, reset/cancel and identical application PQ authority; DTLS alone is insufficient. |

## 5. Immediate critical path and concrete fixes

### A. Bind authority to the actual owner value

A known integration gap remains in net/qdnf/authority: record lookup and MutationHandle/ExecutionPermit matching bind slot, generation and principal, while CurrentAuthorityGuard binds the mutex. Replacing the value inside that same mutex with matching reconstructed records can defeat owner identity. This must be closed before remote service qualification.

Reuse the checked nonwrapping incarnation mechanism already introduced for runtime owners. Give AuthorityOwner, issued mutation handles and execution permits a private owner identity. Preserve const construction and bind lazily only after fallible admission checks. Reject unassigned or foreign identities in both record lookup paths and handle/permit matching. Keep public authority APIs narrow; inspect the existing crate-private ExecutionPermit::issue caller before changing its signature.

Acceptance: replace the actual owner inside the same mutex and test issuance/current validation/revoke/retire; reject a foreign equal-generation permit; preserve validity when the original owner value is moved; ensure failed admission does not assign ownership; measure hot-path allocations. A generation number or a pointer-only check is insufficient.

### B. Establish remote source-controller authorization

Read evidence/qbs07-remote-authority-integration.md before implementing a new authorization object. Existing VerifiedLabel verifies byte/digest relationships, not arbitrary issuer legitimacy. ConsentLedger and SignedSensingScope have real sensing/health semantics; they are not already generic PQ authorization for an entire source bundle.

Reuse RecipientRecordA and existing paired-signature/managed-key primitives with a recognized source controller. Bind original label bytes, canonical manifest, requested scope, recipient, expiry and issuer. Introduce a separately versioned decision ticket only where required, binding the signed authority hash, session transcript, request/challenge and current generation. Remote revocation is bounded by freshness/lease semantics; do not promise instantaneous global revocation. Recheck after restart and before effects/key release.

Acceptance: wrong issuer with otherwise valid paired signatures; stale/rotated controller; scope widening; metadata relabeling; replay across sessions/requests; expiry during work; rejection before plaintext/key release. Integrate actual label join, Sentinel/Webizen policy and semantic closure under the same authority owner.

### C. Separate storage work from owner locks

The current synchronous read/write/sync operations must not run while RuntimeParents/current-authority guards hold shared owners. Use bounded submitted jobs with explicit ticket, reserved bytes, cancellation state and exactly-once completion ownership. Keep original charges alive until actual I/O completion, even after cancellation. The adapter must not refund twice, reuse a buffer early or create an alternate ledger.

Acceptance: slow storage, completion after cancel, panic/error, shutdown and restart; prove no owner lock spans blocking I/O and prove accounting conservation. Reuse the sparse store's existing crash-boundary tests.

### D. Connect the real native service, then B2

Join descriptor selection, source custody, manifest authorization, proof/outboard reads, transfer credits, channel framing and sparse-store jobs into one provider/client owner. Exercise an actual OS bearer between separate processes. The in-process IPC pair is useful for unit tests but cannot establish that gate; raw Ethernet needs the actual reliability layer.

Then add protected recipient wrapping, private-M decode and verified plaintext custody through the managed crypto owner. Maintain strict separation of untrusted ciphertext, structurally valid candidate, authenticated object and policy-admitted consumer data. Test failure at each promotion boundary and abort after a final authority check fails. An outer failure can erase candidate memory but cannot undo an external side effect already performed; callbacks must respect candidate-only staging.

### E. Repair and attach HTTP/WSS, then browser and consumers

The current p2p/connectivity/wss.rs adapter uses a fixed masking key, allocates on read/write, and uses exact/all operations that lose partial progress on WouldBlock. Its bearer shutdown and circuit/generation handling need review. Replace these with bounded retained cursors, fresh masking entropy, strict framing and real shutdown before QBS integration. Retain application-layer paired PQ authority regardless of TLS termination.

HTTP ranges must preserve exact representation bytes and re-use QDNF proofs/policy. Validate status/range/length/validators before accepting data and reject content transformations or ambiguous multipart behavior unless explicitly supported. Browser OPFS storage needs distinct candidate and verified states, quota admission, worker cancellation and restart recovery. Qualify actual browsers and WASM builds; do not infer browser behavior from native Rust tests.

Etalon must consume complete verified P64/model entries with alignment and shared residency accounting first. Selective model execution belongs to its later task and must prove operator/tensor closure. Adopt the new explicit token-aware Gemma API in non-smoke product loops; do not emulate it with a hard-coded token.

## 6. M64 v3 prototypes and format follow-up

Three unintegrated drafts were moved, without changing bytes, to evidence/unintegrated-prototypes/m64-v3/. Its README records original paths and SHA-256 hashes. They had no parent module routing and were never part of the compiled feature graph. They are incomplete and are not evidence of supported partial GRM3 admission.

Read evidence/qbs12-m64-v3-partial-map.md. Reuse GRM3 metadata/shape and factor the whole reader's directory/record checks instead of maintaining a second validator. RepeatNearest padding may depend on authenticated cells in another chunk. Either require those dependencies or expose only validated in-bounds samples with precise limits. The codec-0 georaster profile has no per-chunk BLAKE3 trailer: rely on the appropriate authenticated outer range and do not falsely count a full inner hash. Finish negative and boundary tests before restoring/routing drafts.

Preserve the six requested format standards and their distinction between canonical current output, explicit legacy migration and developmental extensions. Avoid redefining historical short hashes as compliant integrity.

## 7. Repository state and publication

This checkout contains concurrent work outside streaming. Baseline evidence and current status snapshots are retained under evidence/. In particular, Q42 georaster/EMF extensions, rendering/portal/anatomy work, native-authority and sensing changes, and Etalon inference/render/shader changes must not be discarded or casually attributed to this programme. Shared files may contain contributions from multiple tasks. Use a reviewed diff against the recorded baseline before staging; do not blanket reset, clean, stage or revert the tree.

The root QualiaDB Cargo patch uses a local sibling etalon-model-io path and the pinned vendor/etalon-model-6ace6cc backport. The vendor keeps the pinned inference-kernel API while adding required codec/alias repairs. Canonical Etalon also contains relevant repairs. A clean local build with those siblings does not prove a fresh standalone clone or CI can resolve them. Before pushing a usable release, coordinate Etalon commits and dependency pins, or deliberately package the required source. Verify the lockfile and vendor provenance after that decision. This handoff creates no commits and does not push either repository.

Other repositories were not globally certified by this task. Their concurrent owners should run their own final integration checks after merging the coordinated changes.

## 8. Resume procedure and economical delegation

1. Read this handoff, the latest checkpoint entry and task-registry.json. Inspect current status and reconcile concurrent changes before editing.
2. Confirm no old compiler/test process remains. Use one heavy core build at a time on this laptop. Do not rebuild or kill the live Synth MCP process.
3. Reproduce the final compiler/test commands below with the same feature set. Broaden only when changed dependencies or failed gates justify it.
4. Take critical path A through E in order, with small explicit acceptance tests. Review existing crypto, policy, runtime and Commons owners before introducing new structures.
5. Update status only after actual acceptance. Retain failed logs and distinguish later successful reruns.

Normal command baseline, from C:/github/qualiaDB:

    cargo check -p qualia-core-db --tests --locked --offline -j 1
    cargo test -p qualia-core-db --lib --no-run --locked --offline -j 1
    cargo test --manifest-path tools/qbdl-stream-conformance/Cargo.toml --locked --offline

For final linked unit tests, use the executable emitted by the current build, not an older matching filename. Run bundle::stream, net::peer::replication::qbdl, protected_channel, net::qdnf::bearer::ipc, q42::emf_container, canonical mesh, runtime and managed-crypto selectors. Run actual physical model tests separately after supplier changes; the bounded handoff runner deliberately selects gguf_bridge::gemma4:: with a trailing module separator so it does not accidentally include gguf_bridge::gemma4_qwen35 physical-model tests. Do not set a permanent RUST_MIN_STACK override to hide regressions.

WSL Ubuntu has Rust 1.99.1, build-essential and pkg-config. Use its Linux target directory separately and explicit ~/.cargo/bin/cargo if PATH omits it. Some local socket/model/WSL operations require normal host access rather than restricted execution; record the distinction instead of weakening tests. Format edited Rust with edition 2021 and avoid whole-repository formatting churn.

To control future cost, default to one integrator and concise source excerpts. Delegate only an independently testable, bounded task. Use a low-cost agent for inventory, fixture transcription or document consistency; a mid-tier agent for a localized implementation with clear invariants; reserve a higher-capability reviewer for a small crypto/ownership diff when justified. Set explicit task limits and stop conditions before starting, avoid whole-history forks for simple inventory, and do not keep expensive agents waiting or re-auditing the same tree. No new workers were started for this build handoff.

The remaining work is multiple integration and qualification sessions, not a short final polish. Estimate each next slice after reproducing its prerequisite tests; do not promise an all-green programme ETA from the number of implemented modules.
