# QBDL streaming programme progress

## 2026-11-12 — Implementation planning complete

Prepared the programme, architecture, 28 claimable work packages, machine-readable dependency/status registry, capability/cost-aware swarm protocol and release qualification matrix. All implementation packages remain pending. The user's follow-ups added WebSockets to R1 and WebRTC data channels to R2, with shared PQ/proof/policy invariants and distinct browser/network tests.

Read the original advisory brief, supplied expert proposal, completed format standards, parent QDNF swarm/crypto programme and current source anchors for service IDs, crypto envelopes, replication limits, obligation ownership and Etalon residency/model leases. Current source still reports `cose_sign1_frozen() == false`; this is an explicit prerequisite, not missing crypto primitives. Existing replication small-object constants are preserved. The actual SHA-384 ServiceId representation does not automatically allocate a feature identity.

Verified primary references for ML-KEM, hybrid TLS prior art, COSE ML-DSA and fully specified Ed25519, COSE hash envelopes/receipts, HTTP, Fetch, WebSockets and WebRTC. Corrected the advisor's polymorphic Ed25519 identifier in the target profile; distinguished TLS from QSession security, browser forbidden headers from native HTTP control, and entry-specific encrypted representations from raw QBDL. Required independent review before claims about indexed AEAD or distributed escrow security.

Plan decisions defer new QBDL alignment flags, inline/multiproof optimization and HLS/DASH projection; include exact native/HTTP/WSS parity first; and keep QPU historical-result reuse subordinate to the general Commons/fabric architecture. Lower-cost sub-agents handle frozen-contract inventory/fixtures/docs; senior agents implement bounded modules; expert agents own sensitive design and independent review. No agents were launched or implementation spending authorized by writing this plan.

Validation performed for the planning deliverable: JSON/schema field checks, unique IDs, dependency closure and cycle checks, release dependency checks, checklist/registry parity, local Markdown links and whitespace. Protocol code was not changed and new streaming tests were not executed. Existing 61 format / 33 Etalon results remain historical regression inputs, not streaming qualification.

Next executable step: claim QBS-00, inventory exact live revisions/dirty fingerprints and accept the prerequisite evidence map. Only then freeze QBS-01 interfaces and dispatch disjoint QBS-02/03/05 work. Follow the registry and parent swarm protocol; no production protocol identifiers or security conformance are granted by this document.

## 2026-11-12 — implementation wave 1

User authorized the full implementation programme with scoped agents. QBS-00 inventory accepted; revisions and dirty-file fingerprints recorded. Root owns routing, registry and the serial build slot. Senior worker implements shared index admission; expert worker implements a distinct paired COSE adapter; lower-cost worker produces independent malformed parser fixtures. Root implements SHA-384 entry proofs and test integration.

Executed: format conformance 61 tests pass. Initial core baseline build exited 101 for a missing concurrent EMF extension tests module; that file has since appeared and revalidation is underway. Focused direct-production-source proof harness passed 7 tests (including three existing QDNF type tests), independent Node recursive SHA-384 roots/proofs for nine cases, mutation rejection, and zero-allocation checks with an allocating positive control. This is initial proof evidence; independent review and whole-core integration are pending. No protocol release or other package acceptance is inferred.

Wave1 follow-up: direct-source harness now passes37 tests, including repaired outboard failure/poisoning and independent25-vector parser corpus. Independent Node/OpenSSL ML-DSA-65 and Ed25519 signature fixture plus independently encoded COSE bytes pass2 further focused tests; five cryptographic context/signature negatives and15 syntax negatives refuse. Crypto verification hot paths allocate zero in the measured harness. No external composition review or trusted publisher assertion follows from these fixtures.

User reiterated reuse of QualiaDB crypto library. Wider targeted library ownership audit assigned before further crypto integration. Runtime review found scheduler-prefix collision could swallow unrelated work; exact-record tracking, queue-drain fencing and a canonical read-only scheduler identity check replace prefix classification. Runtime execution is pending.

Runtime focused run passed40 tests: new adapter lifecycle/capacity tests, canonical parent regression tests and allocation/conservation test. Effective in-flight support is8 constrained/30 desktop, preserving parent32-slot and control/static reservations;64-row cold planning does not imply64 simultaneous I/O. QBS06 plaintext candidate schema approved; wire implementation underway. Central crypto-library routing correction remains a prerequisite for signing integration.

- Focused phase-A conformance:110 passed,0 failed; independent QASD bytes/inventory, semantic graph limits and storage candidate bits/checkpoints included. Earlier Windows linker LNK1104 failure retained; retry passed after prior sessions were collected. Existing core bundle selector58 passed; new full-core build completed exit0 in26m08s; managed/library/network/publisher selectors now executing. No package/release completion is claimed.

## 2026-11-13 — Grok swarm resume (Wave S0/S1)

User approved the remaining-work swarm plan. Integrator is this Grok 4.6 session. No commit/push. Live Synth MCP is left alone. Shared QualiaDB tree with exclusive file claims (etalon path patch forbids `.grok` worktrees).

Live claims:

- Support inventory: `evidence/swarm-s0-claim-board.md` (read-only).
- Integrator: `cargo check -p qualia-core-db --tests --locked --offline -j 1` log `evidence/swarm-s0-core-check.log`. No overlapping cargo observed at launch.
- Wave S1 (not started until check completes): exclusive write `net/qdnf/authority/{owner.rs,permit.rs,current_guard.rs,current_guard/tests.rs,owner_tests.rs}` plus optional `owner/identity.rs`. Forbidden: policy/storage/wire/Cargo/registry.

Critical path A remains open: `CurrentAuthorityGuard::check_affinity` is mutex `ptr::eq` only.

## 2026-11-13 — Wave S1 implementation + independent review

S1 identity bind landed on `AuthorityOwner` / `MutationHandle` / `ExecutionPermit` using `CanonicalIncarnation`. `check_affinity` now requires assigned identity in addition to mutex `ptr::eq`; handle/permit paths go through `record()` identity. Summary: `evidence/swarm-s1-owner-identity-summary.md`. Independent review: `evidence/swarm-s1-owner-identity-review.md` (`open_bugs: 0`, `pass: true`). Cargo was not part of the review.

First focused `cargo test -p qualia-core-db --lib --locked --offline -j 1 net::qdnf::authority` hit Windows LNK1104 on `target/debug/deps/qualia_core_db-c4560c081fa48873.exe` (preserved `evidence/swarm-s1-authority-tests-lnk1104.log`, cargo EXIT=101). Retry passed: **37 passed, 0 failed** in 0.03s after 1m24s link (`evidence/swarm-s1-authority-tests-retry.log`, EXIT=0). New identity tests all ok, including mutex-replace, foreign equal-generation, moved original, failed-admission, and current-guard replace. AllocGuard zero-allocation test still ok.

Critical path A is closed in source+focused tests. Wave S2 dispatched with exclusive claims; integrator still owns `mod.rs` routing and the cargo slot. Synth MCP left running. No commit/push.

Wave S2 live children (file-disjoint, no cargo):

- B remote R1 `01a1d385-5b64-7be2-bc42-fd850ba588bf` — landed. Integrator will add `mod remote;` after C compile slot frees. Fixed `Generation::next().map_err(E::Authority)` (was `E::Crypto`, would not compile). Review `01a1d394-f7d5-7403-a93f-f28e98c10553`.
- C storage jobs senior `01a1d384-18c4-7133-9532-30d62bd472ab` — landed. Integrator routed `runtime/mod.rs` + `storage/native/mod.rs`. `write.rs` unchanged (armed wait is a new path; production `write_leaf` still holds stage). Review `01a1d393-bb2b-79d3-9280-6e16a230fd65`. Tests `evidence/swarm-s2-storage-jobs-tests.log`.
- Support fixtures `01a1d384-18c5-70f3-901c-5cdeed8f22c3` — complete. Provisional JSON + `evidence/swarm-s2-fixture-notes.md`. Field names copied from source; guessed hex tagged UNVERIFIED. Not a codec freeze.

Claims: `evidence/swarm-s2-claim-board.md`. Integrator routed `runtime/mod.rs`, `storage/native/mod.rs`, and `policy/mod.rs` (`mod remote`). C review `open_bugs: 0`. B review `open_bugs: 0`. First C compile hit E0502 in wait-site test (`try_lock` on `&mut Mutex` still borrowed by `RuntimeParents::adopt`); test now uses ledger/authority `try_lock` plus later `try_with_runtime`. Retry log `evidence/swarm-s2-jobs-and-remote-tests.log` compiled then LNK1104 on `qualia_core_db-c4560c081fa48873.exe` (preserved `-lnk1104.log`). Relink ok. Combined `|` filter ran 0 tests (substring, not regex). Separate filters: remote **11 passed**, jobs **5 passed**, native job **1 passed** (`evidence/swarm-s2-jobs-and-remote-tests-run.log`). Wave S2 B+C+support closed in source+focused tests. Residual: production `write_leaf` still holds stage across OS wait; R2 remote signing/current grant not started.

## 2026-11-13 — Wave S3 dispatched then split

Monolith expert `01a1d3a9-72e4-7c31-94f3-b9bb480768ba` was cancelled (still reading; no production writes). QBS-09 is a pipeline:

- S3.1 parallel: frame codec ∥ in-process service assembly ∥ two-process protocol notes
- S3.2: Windows named pipe ∥ Unix domain (after frame types exist)
- S3.3: two-process join
- S3.4: networking review

Service tests **5 passed**. Stream **10 passed, 8 failed**: split-read test stopped one WouldBlock early; Windows `bind()` → `PlatformUnsupported` (likely unaligned ACL `[u8; N]`). Fixes in `tests.rs` (split-read loop) and `windows.rs` (aligned token/SID/ACL buffers, map ERROR_INVALID_PARAMETER). Retest `evidence/swarm-s3-local-stream-retest.log`. Concurrent other cargo uses `target/qac-legacy-fix-20261113` (left running).

## 2026-11-13 — S3.2 Windows bind closed; S3.3 + M64 v3 live

After alignment, `local_stream` ran **17 passed, 1 failed** (`evidence/swarm-s3-local-stream-retest2.log`, EXIT=101). Windows named-pipe bind/accept/framed exchange all passed. The remaining failure was `parent_generates_qbdl_pipe_name`: `\\.\pipe\qbdl-` has **4** backslashes, not 3. Assertion now matches `PIPE_NAME_PREFIX`. Retest `evidence/swarm-s3-local-stream-retest3.log`: **18 passed, 0 failed**, EXIT=0. Integrator exported Windows/Unix types from `local_stream/mod.rs`.

S3.3 expert `01a1d3f2-ca7f-71d3-b6db-e26e6cf3f5ac`: generic `Session<'a, B: Bearer = IpcEndpoint>`, WouldBlock retry (PIPE_NOWAIT), same-binary spawn, useful tail leaf over named pipe. File-disjoint from M64.

S4 M64 v3 senior `01a1d3f3-38e1-7493-a255-dcaedd4757fb`: partial GRM3 directory view; RepeatNearest padding unclaimed unless neighbor cells authenticated. Integrator routes `formats/mod.rs` and `georaster/mod.rs` after authors stop.

B2 recipient wrap waits for S3.3 review. No commit/push. Synth MCP left running.

## 2026-11-13 — S3.3 two-process green; B2 wrap reviewed; R2 routed

`two_os_process` focused run: **2 passed** (useful leaf + length-prefixed frame) after 14m13s link. M64 v3 + `qbdl::service::` cargo still in the same serial slot (rebuild after wrap.rs).

S4 B2 `authorize_and_wrap` landed. Integrator fixed `map_policy` (`PolicyError::Authority` carries `QdnfError`, not `CryptoError`). Independent review `evidence/swarm-s4-b2-wrap-review.md`: **pass: true, open_bugs: 0**. Wrap cargo not yet run.

S2 R2 new files `policy/remote/{issuer,grant,current,lifecycle_tests}.rs`. Integrator routed `mod issuer; mod grant; mod current;` plus re-exports in `remote/mod.rs`. Independent R2 review next; then focused `lifecycle_tests` + `protection::wrap`.

Still open: production `write_leaf` holds `StorageStage`; WSS/HTTP (S5); browser/Etalon/Commons (S6); peers/qualification (S7); R1 release (S8); R2/R3 remainder (S9–S10). No commit/push.

## 2026-11-13 — continue after compaction; user asked to finish with independent reviews at open_bugs: 0

No pause. Integrator patched R2 review bugs (`join_enrollment` in issuer+grant; live `ControllerRecognition::recognize` in `RemoteDisclosureGrant::recheck`). Added three fail-closed tests for those attacks. Retry cargo `3c39e76d` (m64 → service → wrap → lifecycle_tests, `--locked --offline -j 1`, UTF-8 log `evidence/swarm-s4-retry-m64-wrap-r2.log`).

Live children (file-disjoint, no cargo):

- S5 WSS framing `01a1d435` — exclusive `p2p/connectivity/wss.rs`
- S2 write_leaf drop-stage `01a1d439-60cf` — exclusive `storage/native/{write,checkpoint,job}.rs`
- S5 HTTP inventory `01a1d439-e9c7` — evidence only

Independent R2 re-review after focused lifecycle_tests are green. HTTP implementer waits for WSS author stop + inventory. Synth MCP left running. No commit/push.

## 2026-11-13 — HMC multiplexing review absorbed

Read [implementation-review-hmc-multiplexing-2026-11-13.md](implementation-review-hmc-multiplexing-2026-11-13.md). Continuation pointer: [continuation-2026-11-13.md](continuation-2026-11-13.md).

Product is selective governed streaming of a **heterogeneous QBDL-profile HMC**, not an M64-centric transport. Findings mapped:

- **R01** two-process service is a framing fixture, not protected QSession → C1 after C0.
- **R02** `answer_leaf` associates Credit but not RequestLeaves → C1 `provider.rs`.
- **R03** wire credit ≠ runtime ledger → C1.
- **R04** no recheck before delayed disclosure; constant DEADLINE=90 → C1.
- **R05** write_leaf still holds StorageStage; ArmedNativeWrite is a scalar wait hook → C2 child in flight.
- **R06/R08** mixed-bundle mux/readiness not an acceptance contract yet → C4.
- **R07** no QAC streaming adapter; coordinate EMF/QAC codec owner → C3.
- **R09** E0277 unwrap_err already patched; this retry compiled M64 **5 passed** then hit torn-tree E0583 (`q42/emf_storage`, concurrent) and E0432 (`ArmedNativeLeafWrite`, write_leaf mid-edit). Preserve log. Retry after authors stop.
- **R10** tracker pointer added on the historical handoff.

HTTP/LIG inventory exists; **do not start HTTP until C1**. Independent WSS review launched on current framing source. No commit/push.

## 2026-11-13 — C2 write_leaf review pass; WSS BUG-1 patched; C0 retry

C2 independent review `evidence/swarm-s2-write-leaf-drop-review.md`: **pass: true, open_bugs: 0**.

S5 WSS independent review `evidence/swarm-s5-wss-review.md`: **pass: false, open_bugs: 1** (`seal_close` aborted a pending datagram). Integrator patch: pending non-close is `QdnfError::Conflict`; bytes and `off` stay. Test `pending_datagram_then_seal_close_is_conflict`.

C0 compile was blocked by concurrent QAC `audio/qac/signal/view.rs` E0283 (`channels.into()` / `t.bins.into()` vs `u64` PartialEq with serde_json). Integrator used `u64::from(...)`. Did not touch deprecated `fetch_update` warnings.

C0 retry cargo `swarm-c0-retry-qac-wss.log`: m64_v3, qbdl::service::, protection::wrap, lifecycle_tests, storage::native::job, p2p::connectivity::wss. No commit/push.

## 2026-11-13 — HMC review is live scope; C0 relaunch after writer tests

User pointed at [implementation-review-hmc-multiplexing-2026-11-13.md](implementation-review-hmc-multiplexing-2026-11-13.md). Product remains selective governed streaming of a heterogeneous QBDL-profile HMC over QDNF. M64 is one inner format. ZIP `.hmc` is a different profile. Two-process fixture is not QBS-09 complete.

C0 previous retries: QAC E0283 fixed (`u64::from`); write.rs copy-before-split; R2 two-lifetime `InstallRequest`; torn DICOM `rle` `mod tests` (now present). Concurrent QAC `writer/tests.rs` has landed (7982 bytes). No remaining `mod tests` without a file.

C0 cargo relaunch `evidence/swarm-c0-retry-writer-landed.log` (runner 42604): sequential m64_v3 → qbdl::service:: → protection::wrap → lifecycle_tests → storage::native::job → storage::native::write → p2p::connectivity::wss. Concurrent QAC cargo left on `target/qac-legacy-fix-20261113`.

C1 draft (unrouted, not compiled): `service/protected.rs` composes `ProtectedChannel::with_resources` + `ChannelIo::with_current` / `receive_record`. Plan `evidence/swarm-c1-protected-channel-plan.md`. R02 associate already in provider. HTTP still waits. C3 inventory `evidence/swarm-c3-qac-adapter-inventory.md` (`LegacyQacView::parse` complete-entry fallback). No commit/push.

## 2026-11-13 — C0 paused: concurrent EMF/QAC owns those files

User confirmed EMF-related systems are in active work. WRAP retry had already failed on a torn `DisplayEmfCell` / `EmfSliceUniform.projection_mode` pair (`swarm-c0-retry-writer-landed.log`); those types later appeared in source. Integrator killed the follow-up default-target C0 retry and will not edit `render/gpu/emf_pipeline.rs`, `portal_emf_state`, `portal_load_emf`, `etalon-render` EMF, `audio::qac`, or the portal-qac harness. Foreign cargo left running (`target/qac-legacy-fix-20261113` and `docs/plans/emf-qac-evolution-2026-11-13/evidence/portal-qac`). C0 retry waits until that `qualia_core_db` rustc exits. C1 continues only in unrouted `service/protected.rs`. C3 waits for the EMF/QAC codec owner.

## 2026-11-13 — paused overnight (user powering down)

Stopped integrator C0 cargo/watch. Left Synth MCP and other sessions. No commit/push. Morning: C0 focused cargo first (see continuation pause block), then R2 re-review, then route `service/protected.rs` and C1.

## 2026-11-14 — morning resume

Cargo slot was free (no `qualia-core-db` rustc). Synth MCP left running. EMF types still present (`DisplayEmfCell`, `projection_mode`); those files not edited. C0 relaunch `evidence/swarm-c0-2026-11-14.log` (runner 26852), same sequential filters, `--locked --offline -j 1`. `service/protected.rs` stays unrouted until this compile is green.

## 2026-11-14 — C0 blocked on git-pinned etalon-render; C1 fixture path advanced

Both morning C0 runs died E0560 `projection_mode` (`swarm-c0-2026-11-14.log`, `-b.log`). Root cause: workspace `etalon-render` is git rev `6ace6cc` (field `_pad`). Local `C:\github\etalon` already has `projection_mode`, and `qualia-core-db` `emf_pipeline.rs` writes it, but `[patch.etalon.git]` does not include `etalon-render`. Not editing EMF/render/Cargo patch.

C1 fixture path (does not need ProtectedChannel compile): client Credit/RequestLeaves reuse Open transfer id; provider refuses foreign Credit transfer id; `authorize`/`recheck` after recv, before payload send. New test `credit_transfer_id_must_match_opened_transfer`. `protected.rs` still unrouted.

## 2026-11-14 — EMF/QAC owner unblocked C0; compile in flight

Codec owner patched `etalon-render` (and sibling etalon crates) onto `[patch."https://github.com/mediaprophet/etalon.git"]`, verified wasm check + 205 QAC/EMF tests in non-default targets, and paused `qualia-core-db` source edits. C0 retry `evidence/swarm-c0-2026-11-14-c.log` (runner 52472). C3 inventory now uses `QacStorageView::open` (qac v1); no LegacyQacView re-export requested yet.

## 2026-11-14 — EMF/QAC owner C0-clear handoff absorbed

Codec owner confirmed Streaming C0 can compile, lockfile `--locked --offline` is good, `EmfSliceUniform.projection_mode` and `DisplayEmfCell` match, and they will not touch `bundle/stream/**`, `qbdl/service/**`, or default `target/`. They keep EMF/QAC, `q42/field_*`, portal/render EMF, medical, sensing/acquisition, `etalon-render` EMF, and the workspace etalon `[patch]`.

C0 `-14-c.log` compiled green then ran filters: M64 5/5, SERVICE 11 (1+10), JOB 3/3, WRITE 0, WSS 11/11, WRAP 3 pass 1 fail (`authorized_wrap` Unauthorized at fake clock 20 vs key `created_at=now`), R2 21 pass 1 fail (rotate pin Conflict; later patched `audience: b"reader-rotated"`). Wrap source now sets recipient `not_before=0`, `expires=4_000_000_000`, wrap at `|| 20`, unwrap clock `SystemTime::now()`. Recargo wrap then `lifecycle_tests`: `evidence/swarm-wrap-r2-2026-11-14.log`. `protected.rs` stays unrouted until that is green.

C3 will call `QacStorageView::open` + `signal::admit`. Do not ask for `LegacyQacView` re-export unless a UA4Q historical leaf remains after C1. HTTP still waits for C1. No commit/push.

## 2026-11-14 — wrap 4/4, R2 22/22, C1 routing started

Recargo `evidence/swarm-wrap-r2-2026-11-14.log`: wrap **4 passed** (`authorized_wrap_is_exact_envelope_then_private_handle` ok after `SystemTime::now()` unwrap clock). `lifecycle_tests` **22 passed**, including `recognition_rotate_before_recheck_denied`. Independent R2 re-review spawned (write `swarm-s2-remote-r2-review-2.md`).

C1: routed `qbdl/service/mod.rs` `mod protected;` (`ProtectedSession`). `from_qdnf(Replay) -> Credit`. `PlainProvider::answer_leaf_after_credit` issues one-shot runtime credit after recheck and before LeafData. Service recargo **11/11** (`swarm-c1-service-retry-2026-11-14.log`) after `SessionOwner::lanes()` (E0451). Handshake `protected_session_tests.rs` (attach/foreign AAD/WouldBlock) first compile E0425 `StrongDigest`; import added; retry `swarm-c1-protected-session-retry-2026-11-14.log`. R2 re-review `swarm-s2-remote-r2-review-2.md` **pass: true, open_bugs: 0**. No commit/push.

## 2026-11-14 — C1 production path implemented then tested once

User asked to finish implementation before another cargo. Remaining production wiring landed first:

- `ServiceIo` implemented for `Session` and `ProtectedSession`; provider/client generic over it.
- `associate_until` carries the admitted slot: Grant copies Open `header.deadline`; LeafData copies `request.deadline`; client `send_open_until` / `recv_grant_send_credit_request_until`.
- Expire-after-Open: grant `expires: 90`, Cell clock 20→91, `recheck` fails, no leaf (`expire_after_open_before_request_leaves_refuses_without_leaf`).
- Expire-after-attach: Cell clock past session deadline, send refused (`expire_after_attach_refuses_send_without_plaintext`).

First cargo hit E0282 (expire-test closure needed `Result<(), PolicyError>`). Recargo `evidence/swarm-c1-full-2026-11-14.log`: **17 passed, 0 failed**, EXIT=0. Filters `protected_session_tests` + `qbdl::service::` (includes two-process fixture). HTTP still waits. C3 QAC adapter is next after C1. No commit/push.

## 2026-11-14 — C3–C6 swarm Wave 1 launched

C1 is not the complete HMC mux. User asked for full swarm setup. Integrator wrote `evidence/swarm-c3-c6-pipeline.md` + claim board + three briefs. Wave 1 (NEW files, no cargo, exclusive claims):

- C3 QAC complete-entry adapter (`formats/qac.rs`)
- C4 mux planner (`bundle/stream/plan/`)
- C5 HTTP/LIG (`bundle/stream/http/`)

Wave 2 queued: Describe bootstrap, protected two-process, remote wrap binding, independent reviews. Wave 3: mixed-HMC §6 fixture + C6 evidence classes. Integrator owns parent `mod.rs` and the cargo slot. No commit/push. Synth MCP left running.

## 2026-11-14 — Wave 2 source, then Wave 2.5 swarm (cargo still held)

User: implement remaining non-EMF/QAC work fully; cargo after everything is done; EMF/QAC continuing, they will notify. Gaps become work.

Wave 2 already in source (untested): Describe class 1/7, `authorize_and_wrap_remote`, in-process ProtectedSession useful-leaf. `two_process.rs` still Session+transcript. C2 crash wrapper still takes `&mut StorageStage`.

Wave 2.5 file-disjoint swarm (no cargo, shared tree):

- D `describe.rs` class 8 body / class 9 entry_page
- P `two_process.rs` ProtectedSession + binary key package `QBDL-PSK1`
- C2 `write.rs` + crash tests drop stage during wait (`WriteStep` names frozen)

Integrator: planner tests with **many entries per family** (HMC rule; §6 one-of-each is a minimum first mixed-bundle fixture, not a uniqueness invariant). Mixed-HMC **binary** still Wave 3, waiting on real EMF/QAC writers.

Cargo hold until user notify **and** remaining implementation is complete. No commit/push. Synth MCP left running.

## 2026-11-15 — Wave 3 work packets prepared (not launched)

User rejected a “few days” remainder. Remaining product is one mux join: drive `Schedule::next` from service, publish a real N-per-family QBDL-profile HMC, HTTP Component tests of the same plan. Packets first written as M/F/H.

## 2026-11-15 — Wave 3 packets revised: EMF and QAC supports required

User: the plan must incorporate EMF and QAC supports, not Opaque stand-ins. R07 streaming remaining is now first-class 3A work:

- **E** [swarm-wave3-e-emf-brief.md](evidence/swarm-wave3-e-emf-brief.md) — NEW `formats/emf_stream.rs` (v2/v3 dispatch, region, present, truncated/digest)
- **Q** [swarm-wave3-q-qac-brief.md](evidence/swarm-wave3-q-qac-brief.md) — `admit_signal` on `QacCompleteView` + NEW `qac_stream.rs` (complete-entry fallback stays the transfer unit)
- **M** mux must transfer **two real QAC + two real EMF** in one schedule; delay M64; admit before `present`
- F/H moved to **3A2** and must use E/Q admit

Codec owner files still off-limits. Call `QacStorageView::open`, `signal::admit`, `EmfDirectoryView`. Pipeline: [evidence/swarm-wave3-pipeline.md](evidence/swarm-wave3-pipeline.md). No commit/push. Synth MCP left running.

## 2026-11-15 — Wave 3A accepted, Wave 3A2 launching

User: QualiaDB pushed (`91a4c3746`); launch swarm, check deliverables before accept.

- **E** accepted and routed (`mod emf_stream` / `emf_present_id`, `EmfStreamView`).
- **Q** accepted and routed (`mod qac_stream` / `qac_present_id`, `QacStreamView`; `admit_signal` on `QacCompleteView`).
- **M** accepted after file review: `mux.rs` + `mux_tests.rs` transfer two real QAC + two real EMF, PlanError map, grant one-shot + `run_one_leaf`. Integrator routed `mod mux` and `pub use mux::{transfer_scheduled, BoundEntry, MuxReport, MuxTransfer}`.
- Wave 3A2 **F** mixed HMC + **H** HTTP mixed launching (isolation none, exclusive claims, no cargo).

## 2026-11-15 — Wave 3A2 F accepted; H Q42 HTTP Range addendum

- **F** accepted `01a1dcc0-6cbd-7813-85ec-16855662946e`. Routed `pub mod mixed`. Default corpus: TenD×2, Emf×2, Qac×2, Q42, M64, Opaque.
- **H** first drop five QAC/EMF HTTP tests; user requires Q42 queryable via HTTP Range. Addendum `01a1dcce-20ae-71b3-8596-5fd7c8105501` (`Q42AuthenticatedRangeSource` + C5 `InclusiveRange`). `http/mod.rs` unrouted until that lands.
- EMF/QAC codecs done — no codec deferral. Cargo after H accept + `mod mixed;` in `http/mod.rs`.

## 2026-11-15 — Wave 3A2 H accepted; locked cargo started

- **H** accepted after Q42 HTTP Range addendum (`http_q42_range_matches_authenticated_source`, `Q42AuthenticatedRangeSource`, not `HttpRangeSource`). Routed `mod mixed;` in `http/mod.rs`.
- Locked cargo started: `cargo test -p qualia-core-db --lib --locked --offline -j 1` log `evidence/swarm-wave3-cargo-2026-11-15.log`. LNK1104 retry once. No commit/push. Synth MCP left running.

## 2026-11-15 — cargo stopped; finish source first

User: fully implement, then cargo sessions. Killed the in-flight lib cargo. DescriptorPage session/protected/describe buffers lifted from `MAX_LEAF_RECORD_BYTES` (18137) to wire `MAX_RECORD_BYTES` (65776). Leaf I/O stays 18137. Cargo remains off until this source pass is accepted as complete.

## 2026-11-15 — remaining source landed (cargo next)

User: continue the full implementation of the leftover areas. File-disjoint children then integrator join. **Did not cargo this land.** Foreign rustc is compiling `qualia_core_db` into `hypermedia-production/target` and `synth/target-prod` (not QualiaDB default `target/`).

Accepted after review:

- **C2** `01a1dce0-0932-7331-b9de-9a8e71cd9dbc` — `fixtures::write` prepare/drop/wait/commit/finish. Recovery destination `wait_leaf_write` outside `with_recovery_buffers`. Source `read_exact_at` still a loan (honest leftover).
- **E2** `01a1dce0-0933-7143-9c87-d7d4e7786d6d` — `AuthenticatedRange::subrange`; `EmfStreamView` inner enum v4/v2/v3; QAC `declared_interval` from envelope `flags()&2`.
- **W** `01a1dce0-0933-7143-9c87-d7e5c05b18f7` — `http/wss_mixed.rs`. Integrator routed `mod wss_mixed;`.

Integrator source:

- `Intent::decode` already landed; `two_process` PSK parse now calls it.
- NEW `formats/p64_stream.rs` (`P64StreamView` whole-entry, `has_p64_magic` + `from_p64_with_integrity(Metadata)`, `p64_present_id`). Routed `mod p64_stream`.
- NEW `mixed/transfer.rs` joins `MixedHmc` → `MuxTransfer` / `BoundEntry` / `transfer_scheduled`. Default corpus now includes P64 (`MIXED_ID_P64=41`, ten entries). HTTP `entry_admits_family` admits P64.

Next: one locked `cargo test -p qualia-core-db --lib --locked --offline -j 1` into default `target/` once that slot is free. LNK1104 retry once. Then independent reviews + C6. No commit/push. Synth MCP left running.

## 2026-11-15 — remaining listed source (write_leaf + recovery copy-out)

User: continue remaining source tracks; ZIP `.hmc` is legacy and does not need streaming support (platform unreleased). HMC wire format stays two existing profiles; current product is QBDL (`QBDL` magic). No ZIP adapter. No PhysicalNetwork fake. No browser/WASM claim.

Landed (no cargo this land):

- Production `write_leaf` / `write_leaf_with_hook` take `TransferRuntime` + ticket + buffers. Short stage prepare, drop, `wait_leaf_write`, new short stage commit. `WriteStep` names unchanged. Callers: `tests/{state,leaves,allocation}.rs`.
- `recover_leaf_from` copies source payload/proof into `CopiedLeafMaterial` under the `RangeRecoveryRead` loan, drops the loan, then verifies/arms on a new short stage. Destination wait still has no stage.
- `reverify_and_rewrite_leaf` reads store session payload/proof into `CopiedLeafMaterial` with no stage (session is store-owned), then short-stage verify/arm. `resume_hint_for_leaf` unchanged.

Cargo next into default `target/`. Reviews + C6 after cargo. No commit/push.

