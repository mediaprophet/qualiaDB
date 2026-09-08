# QDNF Implementation Programme Progress

## 2026-09-07 — Plan construction — in progress

- Scope: expand QDNF/Webizen/core design into dependency-ordered implementation checklists with
  agent ownership, focused libraries, handoffs and acceptance evidence.
- Built: initial 30-package register; detailed workstreams and shared integration rules in progress.
- Measurement: implementation not started; no runtime security/capacity/performance result claimed.
- Filesystem: normal directory creation was denied; an approved escalation created only the requested
  qdnf-imp, workstreams and templates directories. Documentation writes then proceeded normally.
- Human input needed: none for preparing the requested plan.
- Next: complete checklists, dependency crosswalk and executable structural validation.

## 2026-09-07 — Plan assembled and initial structural validation — passed

- Saved eight workstreams, the dependency roadmap, library and swarm rules, validation matrix,
  register and four task/decision/evidence/handoff templates. There are 30 pending packages and
  418 unchecked child items covering P0–P21 through domain owners.
- Two focused agents completed crypto/network and semantics/services/economics/evidence plans.
  Their subsequent independent integration reviews reached the usage limit; those reviews did
  not complete and are not claimed as completed independent acceptance.
- Automatic approval review rejected the validator write at the account usage limit. Nineteen
  files had already been saved. After the user's follow-up, disk inspection confirmed those files;
  the same validator write subsequently succeeded without an alternate write mechanism.
- Ran `pwsh -NoProfile -File docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/validate-plan.ps1`:
  passed 30 packages, 418 child checks, 71 dependency edges, 18 Markdown files and 164 local links;
  P0–P21 each have domain implementation owners. No runtime tests or performance claims follow.
- Added entry links from the main fabric README and implementation-conformance document.
- Next: complete final parent consistency, compatibility and whitespace checks; runtime remains pending.

## 2026-09-07 — Requested implementation plan — complete

- Parent review added RT-01 as a completion dependency of NET-01: real runtime admission must
  back native buffer/completion reservations. Updated the register, workstream and roadmap together.
- Final PowerShell 7 validator run passed: 30 packages, 418 child checks, 72 dependency edges,
  18 Markdown files, 164 local links and full P0–P21 domain-owner coverage. The package contains
  20 saved files including the JSON register and executable validator.
- Independently compared every roadmap dependency row with the register; all matched. Checked
  trailing whitespace in every new file and scoped `git diff --check`; both passed.
- Windows PowerShell refused script execution under this workstation's policy. No policy was
  changed; the documented command uses the existing PowerShell 7 host, which passed validation.
- All 30 runtime packages and 418 implementation checks remain pending. The requested plan is
  complete; no Rust code, runtime tests, deployment, commit or push was performed for this step.
- Next authorized implementation starts with FND-01 and bounded claims under the swarm protocol.
  No missing user input or unresolved save failure remains for the planning deliverable.

## 2026-09-07 — Placeholder and replacement terminology review

- Searched the design/plan suite for placeholders, mocks, stubs, Kademlia, Yamux and libp2p.
  Confirmed the live core Cargo manifest still declares libp2p with Yamux/Kademlia features;
  the documentation work has not implemented or removed that runtime dependency.
- Clarified P8's older carrier wording: libp2p migration is optional and separately packaged.
  Clarified the distinction between the proposed Kademlia lookup algorithm and libp2p's code;
  QSession supplies replacement stream machinery. No routing algorithm or runtime code changed.
- Documented API/layout/profile/command placeholders as explicit pending freeze inputs and kept
  real-boundary acceptance separate from fixture/mock tests. No task was marked implemented.

## 2026-09-07 — QSR algorithm design — first draft saved

- Designed scoped semantic lanes, authenticated radix coverage, bounded query execution,
  publication/withdrawal, epoch handover, cell placement and explicit snapshot/incomplete outcomes
  in `qualia-scoped-rendezvous.md`. The target replaces the proposed Kademlia lookup overlay.
- Reviewed primary Kademlia and distributed ordered-index research for comparison; gains remain
  hypotheses. No implementation, benchmark or cryptographic security proof has been produced.
- A read-only agent is reviewing bootstrap, coverage, privacy and adversarial failure assumptions.
- Human input needed: none for the draft. Next: comparison/acceptance design, integration and review.

## 2026-09-07 — QSR review and implementation integration — done

- Added comparison methodology and ten concrete failure/update traces in `qsr-evaluation.md`.
  The independent review identified bootstrap, index-completeness, epoch-fork, hot-key and
  handover requirements; incorporated those into the algorithm rather than claiming signatures
  prove completeness or prefix splits solve a popular exact key.
- Replaced target-design DHT wording with QSR across resolution/routing, operations and security;
  retained explicit historical/legacy Kademlia references. Added NET-04.15–30 and focused qsr
  library boundaries. The programme now has 434 checks across the same 30 packages.
- All gains remain unmeasured hypotheses. No executable networking implementation or benchmark
  was produced. Human input needed: none this step. Next: final review and structural validation.

## 2026-09-07 — QSR final review corrections — done

- Final independent review found five concrete gaps. Corrected write-loss ordering by fencing and
  draining the old generation before final transfer/readiness; delegated checkpoint publication
  explicitly through immutable covers; separated lane/key mapping and per-trie stage bounds;
  carried completeness trust levels into negative results; removed a secondary-cover hash cycle.
- Updated the failed-handover trace and traversal analysis. Kept the scope-level epoch-writer
  availability/censorship dependency explicit rather than claiming decentralized consensus.
- Implementation remains pending. Human input needed: none this step. Next: final saved-file
  checks and delivery of the algorithm plus its evaluation/implementation requirements.

## 2026-09-07 — QSR design delivery — complete

- Saved the algorithm, evaluation protocol, ten worked traces and 16 added implementation checks;
  integrated the selected QSR lookup target through the design suite.
- Final plan validation passed 30 packages, 434 child checks, 72 dependency edges and 166 local
  plan links. Both new algorithm documents passed their five local links, fence and whitespace
  checks; scoped `git diff --check` passed. Verified 96 radix digits per trie and the 524,288-byte
  descriptor sizing example. These are structural/arithmetic checks, not runtime benchmarks.
- All requested design work is complete. Implementation, independent executable experiments and
  measured superiority remain future checklist work. Human input needed: none for this deliverable.

## 2026-09-07 — Finite compensation and socially defined protection — requirements integrated

- Audited existing threshold economics, humanitarian permissions, scoped guardianship and privacy.
  Those foundations did not explicitly specify the full requested finite compensation model or
  child/PEP protection workflows. Added finite-project-compensation.md and socially-defined-protection.md.
- Defined accepted contribution costs/value and capped return, personal/humanitarian exemptions,
  represented-incorporated usage, operating/fee/recovery allocation, one shared remaining target,
  atomic final payments, beneficiary payout records and terminal fulfilment without repeated recovery.
- Defined protected discovery/contact, minimal-disclosure eligibility, guardian/office role limits,
  confidential reporting, independently reachable help and freshness requirements for queued delivery.
- Added 46 numbered implementation checks across SEM/ECO/NET/EVD/OPS/QA, bringing the programme to
  480 checks; linked the profiles into the economics, contracts, security, QSR and main plan documents.
- Independent review found residual-waiver/zero-target handling, stale mailbox authority and abusive
  controller removal of help safeguards needed strengthening. Added explicit non-cash discharge,
  freshness-gated sensitive delivery and independent safeguard amendment/appeal authority.
- Automatic approval review rejected that final correction write at the account usage limit.
  After the user's continuation, read disk state and completed the same scoped write; the two
  profile drafts and added checks had already been saved. No alternate write mechanism was used.
- Runtime behavior and financial/legal/security outcomes remain unimplemented and unmeasured.
  Human input needed: none for the requested requirements update. Next: final validation and delivery.

## 2026-09-07 — Compensation/protection update — complete and validated

- Final validator passed 30 packages, 480 child checks, 72 dependency edges, 18 plan Markdown
  documents and 168 local plan links, with P0–P21 domain-owner coverage. All implementation
  checks remain pending. Scoped git diff whitespace checks passed.
- Both new application profiles passed eight local links, code-fence and whitespace checks.
  Decimal arithmetic checks confirmed the documented final-payment quotes (2.21 then 1.21),
  shared reservation cap, residual waiver and zero-target behavior. These validate the example,
  not a running settlement engine, security implementation or legal outcome.
- Independent review corrections are saved; no missing write or user input remains for this update.
  The requested requirements are explicit in the design and agent implementation plan. Runtime
  implementation and conformance evidence are the next separate work, not claimed complete.

## 2026-09-07 — Known-peer confidential clinical exchange — definition made explicit

- Confirmed the older Sanctuary manual explicitly includes medical records of PEPs, and the
  current design already has private-pairwise relationships, clinician roles and QSession.
  Added the concrete known-peer workflow to socially-defined-protection.md §6.1.
- Defined private pairing/key binding, standing bilateral care permissions, selected medical
  disclosure, optional explicitly authorized care teams, actual institutional decryption boundaries,
  offline delivery/freshness and role/key transitions. No public patient or VIP-association indexing.
- Added ten pending checks across SEM/NET/OPS/QA; the programme now has 490 checks. Distinguished
  delivered bytes from clinician review and care, and onward sharing/recording/AI use from routine consent.
- Human input needed: none for this clarification. Runtime implementation remains pending; next:
  validate links, checklist structure and whitespace, then deliver the updated definition.

## 2026-09-07 — Known-peer clinical definition — validated and complete

- Plan validation passed 30 packages, 490 child checks, 72 dependency edges, 18 plan Markdown
  documents and 169 local links, including the clinical workflow heading anchor. Scoped
  git diff whitespace checks passed. The existing Sanctuary source link resolves locally.
- The definition and ten implementation checks are saved; no missing user input remains.
  No runtime clinical exchange, endpoint security test or medical/legal compliance result is claimed.

## 2026-09-08 — Native QPR slice (libp2p replacement code) — in progress, not complete

- Step: first native vertical slice (FND types/errors, CRY adapters, NET frame/IPC/QLink/QRoute/QSR/QSession, RT leases, `qualia-peer` host). Status: **partial**. Programme packages remain pending; no checkboxes marked complete.
- Built: `crates/qualia-core-db/src/net/qdnf/`, `crypto/network/`, `net/peer/` (leases + `NativePeer`), facade `crates/qualia-peer`. Inherited Swarm/Kad/Yamux/mDNS stay behind `libp2p-compat`. Native modules do not `use libp2p`.
- Measured: `cargo +stable test -p qualia-core-db --lib -- net::qdnf net::peer crypto::network` → **59 passed**. `cargo +stable test -p qualia-peer` → **2 passed**. Example `native_ipc_peers` printed `native ipc exchanged 11 bytes (libp2p not used)`. These are in-process `local-ipc-v1` results, not Ethernet/IP-absent hardware, not end-to-end application migration, not a full Native Independent cargo-tree closure (core-db default still includes `libp2p-compat` and GPU features).
- Human input needed: none this step. Raw Ethernet remains `PlatformUnsupported` without CAP_NET_RAW.
- Next: remaining NET/RT/SVC packages; isolate `qualia-peer` from default libp2p/GPU closure (RT-03.07).

## 2026-09-08 — Wave 1 swarm claim

- Integrator claims Wave 1 (FND-02, QA-01, CORE-02) plus provisional continuation of CRY-01 and RT-01. Status: **in_progress**. Packages stay open.
- Disjoint writes: FND-02 `net/qdnf/authority.rs` + `contracts/` + new `fabric/`; QA-01 `net/qdnf/harness/` only; CORE-02 new `q42/volume/network_quanta.rs` + `volume/mod.rs`; CRY-01 `crypto/network/` only; RT-01 `net/peer/runtime/`.
- Shared files forbidden to workers: Cargo.toml, `registries.rs`, design suite, AGENTS.md, `p2p/`.
- Human input needed: none this step.
- Next: integrate child patches, run `cargo +stable` tests, record measurements.

## 2026-09-08 — Wave 1 swarm integrated — partial, packages remain open

- Five disjoint implementers: FND-02 (fabric join/revocation/bootstrap), QA-01 (`FaultPipe`, partition, allocation counter), CORE-02 (`ScanBudget`/`NetworkCursor`), CRY-01 (ephemeral entropy lease, hardware fail-closed, rotation), RT-01 (DRR scheduler, cancel, generation exhaustion).
- Integrator: re-exported network quanta from `q42_volume`; removed unused import. Design suite and programme checkboxes were not rewritten.
- Measured: `cargo +stable test -p qualia-core-db --lib -- net::qdnf net::peer crypto::network q42::q42_volume::volume::network_quanta` → **148 passed**, 0 failed. `qualia-peer` → **2 passed**. Not Ethernet, not Native Independent closure, not package completion.
- Human input needed: none this step.
- Next: Wave 2 (CORE-01, CORE-03, remaining CRY-01) with the same disjoint-file swarm.

## 2026-09-08 — Wave 2 swarm claim

- Integrator claims Wave 2 provisional: CORE-01 arena admission, CORE-03 WAL intent markers, NET-01 bearer lifecycle. Packages stay **open**.
- Disjoint writes: CORE-01 `governance/webizen/arena_admit.rs` + webizen/mod.rs one line; CORE-03 new `wal_intent.rs` beside `wal.rs` (do not grow wal.rs); NET-01 `net/qdnf/bearer/` only.
- Human input needed: none this step.

## 2026-09-08 — Wave 2 swarm integrated — partial, packages remain open

- Three disjoint implementers: CORE-01 (`ArenaAdmit` lease/scope wrapper; does not allocate 42 MiB or replace `SlgArena` Vec backing), CORE-03 (`wal_intent::IntentTable` 32-slot SHA-384 markers; `wal.rs` unchanged), NET-01 (`BearerLifecycle` + `LeasedIpc` tx/rx leases). Integrator restored the `Bearer` trait import required for `RawEthernet::shutdown`.
- Measured: `cargo +stable test -p qualia-core-db --lib -- governance::webizen::arena_admit wal_intent net::qdnf::bearer net::qdnf net::peer crypto::network q42::q42_volume::volume::network_quanta` → **182 passed**, 0 failed. `qualia-peer` → **2 passed**. Not Ethernet, not Native Independent closure, not package completion. `IpcEndpoint::recv` still dequeues before checking output length (documented).
- Human input needed: none this step.
- Next: Wave 3 (FND-03 golden fixtures, CRY-02 handshake FSM/vectors, RT-02 cell admission) with disjoint writes.

## 2026-09-08 — Wave 3 swarm claim

- Integrator claims Wave 3 provisional: FND-03 fixtures, CRY-02 handshake transitions, RT-02 cell admission. Packages stay **open**.
- Disjoint writes: FND-03 `net/qdnf/fixtures/` only; CRY-02 `crypto/network/pq_handshake.rs` (+ `crypto/network/mod.rs` one line); RT-02 `net/peer/cells/` only (`net/peer/mod.rs` one line).
- Shared files forbidden to workers: Cargo.toml, `registries.rs`, design suite, AGENTS.md, `p2p/`, `wal.rs`, `arena.rs`.
- Human input needed: none this step.
- Next: integrate child patches, run `cargo +stable` tests, record measurements.

## 2026-09-08 — Wave 3 swarm integrated — partial, packages remain open

- Three disjoint implementers: FND-03 (empty DiscoveryBeacon golden `QDNF`/v1/80-byte prefix, unknown-profile fail-closed, NetworkCursor bind), CRY-02 (`pq_handshake` successive FSM, 0-RTT denied, draft initiator-share encoding, ML-KEM-then-X25519 concat), RT-02 (`CellTable` 4 slots, NetworkSmall cannot take 42 MiB, reuse RT-01 ledger/leases).
- Measured: `cargo +stable test -p qualia-core-db --lib -- governance::webizen::arena_admit wal_intent net::qdnf::bearer net::qdnf::fixtures crypto::network::pq_handshake net::peer::cells net::qdnf net::peer crypto::network q42::q42_volume::volume::network_quanta` → **204 passed**, 0 failed. Not Ethernet, not Native Independent closure, not package completion. CRY-02.12 independent KEM vectors remain open.
- Human input needed: none this step.
- Next: Wave 4 (NET-02 discovery cookies, ECO-01 payment-vs-budget, remaining QSR/session bounds) with disjoint writes.

## 2026-09-08 — Wave 4 swarm claim

- Integrator claims Wave 4 provisional: NET-02 cookies/amplification, ECO-01 payment cannot enlarge consent, NET-04 QSR query bound. Packages stay **open**.
- Disjoint writes: NET-02 `net/qdnf/link/cookies.rs` + link/mod.rs one line; ECO-01 `net/qdnf/economics/consent.rs`; NET-04 `net/qdnf/resolve/query.rs` (do not rewrite qsr.rs).
- Shared files forbidden: Cargo.toml, `registries.rs`, design suite, AGENTS.md, `p2p/`.
- Human input needed: none this step.

## 2026-09-08 — Wave 4 swarm integrated — partial, packages remain open

- Three disjoint implementers: NET-02 (`CookieJar` HMAC-SHA-384 reachability cookies, 32 slots, 2 per locator, reply cap 3; membership/application grants stay false), ECO-01 (`apply_payment_to_consent` is a no-op on budgets; host remaining from RT-01 ledger; aborted TxId recovers as Ambiguous), NET-04 (`QueryBudget` 64/16/8/3 caps; `lookup_with_budget` still uses authenticated `lookup_exact`; compact hash is not authority).
- Measured: `cargo +stable test -p qualia-core-db --lib -- net::qdnf::link::cookies net::qdnf::economics net::qdnf::resolve net::qdnf net::peer crypto::network wal_intent governance::webizen::arena_admit q42::q42_volume::volume::network_quanta` → **229 passed**, 0 failed. Not Ethernet, not Native Independent closure, not package completion.
- Human input needed: none this step.
- Next: Wave 5 candidates remain NET-03 forwarding/session races, CRY-02.12 independent vectors, SVC QSync, RT-03.07 libp2p/GPU-free `qualia-peer` closure. Packages stay open.

## 2026-09-08 — Wave 5 swarm claim

- Integrator claims Wave 5 provisional: NET-03 generation overlap + A-B-C forward, CRY-02.12 transcript/KDF vectors, SVC-01 operation identity. Integrator owns RT-03.07 `qualia-peer` feature closure (Cargo.toml). Packages stay **open**.
- Disjoint writes: NET-03 `net/qdnf/route/overlap.rs` + `net/qdnf/route/flood.rs` (do not rewrite `spf.rs`/`forwarding.rs`); CRY-02 `crypto/network/vectors.rs`; SVC-01 `net/peer/replication/` only.
- Shared files forbidden to workers: Cargo.toml, `registries.rs`, design suite, AGENTS.md, `p2p/`, `wal.rs`, `arena.rs`.
- Human input needed: none this step.
- Next: integrate child patches, run `cargo +stable` tests, record measurements.

## 2026-09-08 — Wave 5 swarm integrated — partial, packages remain open

- Three disjoint implementers plus integrator: NET-03 (`GenerationPair` old/new overlap, `FloodTable` LSA admit/conflict/suppression, A-B-C lookup dest 2 → hop 1), CRY-02.12 (`vectors.rs` frozen transcript/HKDF/Finished hex vs independent sha2/hkdf oracle; live ML-KEM encapsulate still open), SVC-01 (`OpTable` SHA-384 operation ids, identical retry, sequence wrap Range). Integrator: `qualia-peer` now depends on core-db **without** `libp2p-compat`. `cargo +stable tree -p qualia-peer -i libp2p` → package not in graph. GPU/LLM remain (`gpu-runtime`); `--no-default-features --features qdnf` still fails (wgpu unguarded, 1528 errors).
- Measured: `cargo +stable test -p qualia-core-db --lib -- net::qdnf::route crypto::network::vectors net::peer::replication net::qdnf net::peer crypto::network wal_intent governance::webizen::arena_admit q42::q42_volume::volume::network_quanta` → **255 passed**, 0 failed. `qualia-peer` → **2 passed**.
- Human input needed: none this step.
- Next: Wave 6 candidates: NET-05 session dial races, remaining SVC checkpoints, GPU/wgpu feature-gating for full RT-03.07, raw Ethernet. Packages stay open.

## 2026-09-08 — Wave 6 swarm claim

- Integrator claims Wave 6 provisional: NET-05 path races + stream credit, SVC-01 checkpoints. Packages stay **open**.
- Disjoint writes: NET-05 `net/qdnf/session/paths.rs` and `net/qdnf/session/credit.rs` (do not edit `session/mod.rs` — integrator wires); SVC `net/peer/replication/checkpoint.rs` + replication/mod.rs one line.
- Shared files forbidden: Cargo.toml, `registries.rs`, design suite, AGENTS.md, `p2p/`.
- Human input needed: none this step.

## 2026-09-08 — Wave 6 swarm integrated — partial, packages remain open

- Three disjoint implementers: NET-05.10 (`PathTable` max 3 Active, 8 race slots, `lose_generation` stale, operation id preserved across path change, compat-carrier duplicate recovery off), NET-05.07 (`CreditTable` 64 streams/dir, one aggregate `BufferLease` per direction vs 32 `LEASE_SLOTS`, window vs `ReservationLedger`, purchased service does not disable congestion, ACK range cap 8), SVC-01.07 (`build_checkpoint` SHA-384, empty ≠ ZERO, duplicates Conflict, membership ≠ completeness). Integrator wired `session/{paths,credit}` and `replication/checkpoint`.
- Measured: `cargo +stable test -p qualia-core-db --lib -- net::qdnf::session::paths net::qdnf::session::credit net::peer::replication net::qdnf net::peer crypto::network wal_intent governance::webizen::arena_admit q42::q42_volume::volume::network_quanta` → **273 passed**, 0 failed. `qualia-peer` → **2 passed**. Not Ethernet, not Native Independent closure, not package completion. `IpcEndpoint::recv` still dequeues before checking output length.
- Human input needed: none this step.
- Next: Wave 7 (NET-05 packet/replay/nonce, NET-05 ACK/PTO, CRY-02 responder-share encoding + live KEM). Packages stay open.

## 2026-09-08 — Wave 7 swarm claim

- Integrator claims Wave 7 provisional: NET-05.04 packet/replay/nonce, NET-05.08 ACK/loss/PTO, CRY-02.03 responder-share encoding. Packages stay **open**.
- Disjoint writes: NET-05 `net/qdnf/session/packet.rs` and `net/qdnf/session/loss.rs` (do not edit `session/mod.rs` — integrator wires); CRY-02 `crypto/network/share_encoding.rs` (do not edit `pq_handshake.rs` / `handshake.rs` / `mod.rs` — integrator wires).
- Shared files forbidden to workers: Cargo.toml, `registries.rs`, design suite, AGENTS.md, `p2p/`, `wal.rs`, `arena.rs`.
- Human input needed: none this step.
- Next: integrate child patches, run `cargo +stable` tests, record measurements.

## 2026-09-08 — Wave 7 swarm integrating

- Integrator wires `session/{packet,loss}`, `crypto/network/share_encoding`, plus integrator-owned `replication/tombstone` (SVC-01.11 partial). Packages stay **open**.
- Human input needed: none this step.

## 2026-09-08 — Wave 7 swarm integrated — partial, packages remain open

- Three disjoint implementers plus integrator: NET-05.04 (`PacketSpace` 64-bit replay window, nonce = 4 zero ‖ u64be pn, QUIC varint rejects overlong), NET-05.08 (`AckFrame` 8 ranges no merge, `SentTable` 32, PTO `rtt*2^n` cap 8, unknown RTT not zero), CRY-02.03 (`ml_kem_ct || x25519_pk`, hybrid IKM kem-then-x25519, live encapsulate is **not** a frozen vector). Integrator: SVC-01.11 `TombstoneTable` until authorized frontier; transport ACK is not compaction.
- Measured: `cargo +stable test -p qualia-core-db --lib -- net::qdnf::session::packet net::qdnf::session::loss crypto::network::share_encoding net::peer::replication net::qdnf net::peer crypto::network wal_intent governance::webizen::arena_admit q42::q42_volume::volume::network_quanta` → **299 passed**, 0 failed. `qualia-peer` → **2 passed**. Not Ethernet, not Native Independent closure, not package completion.
- Human input needed: none this step.
- Next: Wave 8 (NET-05.12 rekey vs authority, SVC-01.08 membership proofs, CRY-02.08 minima/no classical retry). Packages stay open.

## 2026-09-08 — Wave 8 swarm claim

- Integrator claims Wave 8 provisional: NET-05.12 key update, SVC-01.08 membership proofs, CRY-02.08 minima. Packages stay **open**.
- Disjoint writes: NET-05 `net/qdnf/session/rekey.rs` (do not edit `session/mod.rs`); SVC `net/peer/replication/proof.rs` (do not edit `replication/mod.rs`); CRY-02 `crypto/network/minima.rs` (do not edit `crypto/network/mod.rs`).
- Shared files forbidden: Cargo.toml, `registries.rs`, design suite, AGENTS.md, `p2p/`.
- Human input needed: none this step.

## 2026-09-08 — Wave 8 swarm integrating

- Integrator wires `session/rekey`, `replication/proof`, `crypto/network/minima`. Packages stay **open**.
- Human input needed: none this step.

## 2026-09-08 — Wave 8 swarm integrated — partial, packages remain open

- Three disjoint implementers: NET-05.12 (`RekeyTable` current+2 old, rekey/replayed Allow do not renew grants, Draining → Closed), SVC-01.08 (`MembershipProof` digest+counts, membership ≠ range coverage, forged count Malformed), CRY-02.08 (`MinimaCache` 8 slots, classical/stripped/unknown fail closed, no automatic classical retry).
- Measured: `cargo +stable test -p qualia-core-db --lib -- net::qdnf::session::rekey net::peer::replication::proof crypto::network::minima net::qdnf net::peer crypto::network wal_intent governance::webizen::arena_admit q42::q42_volume::volume::network_quanta` → **321 passed**, 0 failed. `qualia-peer` → **2 passed**. Not Ethernet, not Native Independent closure, not package completion.
- Human input needed: none this step.
- Next: Wave 9 (NET-05.01 session bind, SVC-01.10 merge, CRY-02.09 admission chunks). Packages stay open.

## 2026-09-08 — Wave 9 swarm claim

- Integrator claims Wave 9 provisional: NET-05.01 session context bind, SVC-01.10 merge/conflict, CRY-02.09 admission chunks. Packages stay **open**.
- Disjoint writes: NET-05 `net/qdnf/session/bind.rs`; SVC `net/peer/replication/merge.rs`; CRY-02 `crypto/network/chunks.rs`. Do not edit the parent `mod.rs` files — integrator wires.
- Shared files forbidden: Cargo.toml, `registries.rs`, design suite, AGENTS.md, `p2p/`.
- Human input needed: none this step.

## 2026-09-08 — Wave 9 swarm integrating

- Integrator wires `session/bind`, `replication/merge`, `crypto/network/chunks`. Packages stay **open**.
- Human input needed: none this step.

## 2026-09-08 — Wave 9 swarm integrated — partial, packages remain open

- Three disjoint implementers: NET-05.01 (`bind_complete` refuses zero identifiers and non-PQ profile; connection id is not a grant), SVC-01.10 (`MergeSet` 4 alts; LWW/wall-clock are not authority; `len>1` is Conflict), CRY-02.09/10 (`ChunkTable` 16 KiB/16 chunks/32 flights/2 per locator; duplicate Replay does not extend deadline).
- Measured: `cargo +stable test -p qualia-core-db --lib -- net::qdnf::session::bind net::peer::replication::merge crypto::network::chunks net::qdnf net::peer crypto::network wal_intent governance::webizen::arena_admit q42::q42_volume::volume::network_quanta` → **347 passed**, 0 failed. `qualia-peer` → **2 passed**. Not Ethernet, not Native Independent closure, not package completion.
- Human input needed: none this step.
- Next: Wave 10 (CRY-02.05 dual proofs, SVC-01.12 manifests, NET-05.06 IRI/receipt stages). Packages stay open.

## 2026-09-08 — Wave 10 swarm claim

- Integrator claims Wave 10 provisional: CRY-02.05 dual proofs, SVC-01.12 content manifests, NET-05.06 datagram IRI/receipt. Packages stay **open**.
- Disjoint writes: CRY-02 `crypto/network/dual_sign.rs`; SVC `net/peer/replication/manifest.rs`; NET-05 `net/qdnf/session/iri.rs`. Do not edit parent `mod.rs` files.
- Shared files forbidden: Cargo.toml, `registries.rs`, design suite, AGENTS.md, `p2p/`.
- Human input needed: none this step.

## 2026-09-08 — Wave 10 swarm integrating

- Integrator wires `crypto/network/dual_sign`, `replication/manifest`, `session/iri`. Packages stay **open**.
- Human input needed: none this step.

## 2026-09-08 — Wave 10 swarm integrated — partial, packages remain open

- Three disjoint implementers: CRY-02.05 (`DualProof` requires ML-DSA-65 and Ed25519; COSE_Sign1 not frozen), SVC-01.12 (`ContentManifest` SHA-384, 8 ranges, 4 MiB decoded bound, container generation ≠ QSync root), NET-05.06 (`ChannelTable` IRI/service collision, transport ACK is not durable).
- Measured: `cargo +stable test -p qualia-core-db --lib -- crypto::network::dual_sign net::peer::replication::manifest net::qdnf::session::iri net::qdnf net::peer crypto::network wal_intent governance::webizen::arena_admit q42::q42_volume::volume::network_quanta` → **367 passed**, 0 failed. `qualia-peer` → **2 passed**. Not Ethernet, not Native Independent closure, not package completion.
- Human input needed: none this step.
- Next: remaining NET-05.16 cancel/reconnect, SVC-01.13 transfer, RT-03.07 wgpu gating. Packages stay open.

## 2026-09-08 — Wave 11 swarm claim — in progress

- Claimed exclusive write sets (one file each; do not rewrite design suite; do not tick qdnf-imp checkboxes):
  1. `crates/qualia-core-db/src/net/qdnf/session/recovery.rs` — NET-05.16 reconnect/cancel: reconnect must not duplicate durable actions; transport delivery is not accepted work or payment; reuse `net/peer/runtime/cancel.rs` OperationTable/CancelEpoch, do not rewrite that kernel.
  2. `crates/qualia-core-db/src/net/peer/replication/transfer.rs` — SVC-01.13 bounded transfer: shared receive/storage/verify credit; retries and old+new generations charged to parent reservation.
  3. `crates/qualia-core-db/src/net/peer/replication/resume.rs` — SVC-01.14 resume only verified blocks of a pinned manifest; container generation ≠ QSync root.
  4. `crates/qualia-core-db/src/crypto/network/malformed.rs` — CRY-02.13 truncation, noncanonical, unknown critical, signature stripping, all-zero DH, malformed KEM, reordered flights fail closed.
  5. `crates/qualia-core-db/src/net/qdnf/session/freshness.rs` — NET-05.18 recheck policy freshness at queued/offline/migration; sensitive stays pending; help still accessible.
  6. `crates/qualia-core-db/src/net/peer/replication/receipts.rs` — SVC-01.04 identity/effect/receipt: acknowledge only the durability class achieved; transport ACK is not that class.
- Integrator (this agent) owns `session/mod.rs`, `replication/mod.rs`, `crypto/network/mod.rs`, Cargo.toml, and wgpu cfg-gating investigation (RT-03.07 remainder).
- Status: claimed. Packages remain open.

## 2026-09-08 — Wave 11 swarm integrating

- Integrator wires `session::{recovery,freshness}`, `replication::{transfer,resume,receipts}`, `crypto/network/malformed`. Combined tests not yet run this revision. Packages remain open.
- Human input needed: none this step.

## 2026-09-08 — Wave 12 swarm claim — RT-03.07 wgpu cfg (in progress)

- Disjoint exclusive write sets (do not edit Cargo.toml, lib.rs, design suite, AGENTS.md, p2p/, qdnf-imp checkboxes). Gate wgpu behind existing `feature = "gpu-runtime"` (tighten `not(wasm32)` that always pulls wgpu). Default-feature builds must stay green.
  A. `gguf_bridge/` GPU files listed in the worker prompt
  B. `platform/` GPU/NPU/compute_bridge GPU files
  C. `inference/` GPU + wgsl/cuda call sites
  D. `gpu_context.rs` + `gpu_context/`
  E. leaf GPU: graph_accel, lora/webgpu, tensor/volume_gpu, modalities/diffusion, net/host_topology
  F. `poet_host/invoke/render/` GPU files + calculus GPU fallout
- Integrator owns lib.rs and Cargo.toml (including later dropping gpu-runtime from qualia-peer).
- Status: claimed. Packages remain open.

## 2026-09-08 — Wave 13 swarm claim — remaining gpu-runtime fallout

- After Wave 12, `cargo +stable check -p qualia-core-db --no-default-features --features qdnf` is **63 errors / ~34 files** (down from 1528). Default-feature lib check is green. Wave 11 combined native filter: **422 passed**. `qualia-peer` **2 passed**.
- Disjoint remaining write sets: poet_host invoke GPU dispatch; inference decode/probes/timeline; compute_bridge execute/policy; webizen vm GPU integrator; graph_accel gpu_available; gpu_state/backend. Integrator owns lib.rs (device_benchmark/npu_ffi already gated).
- Status: claimed. Packages remain open. Not Ethernet, not Native Independent closure.

