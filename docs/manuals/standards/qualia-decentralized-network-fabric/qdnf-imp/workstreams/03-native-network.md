# Native network implementation workstream

**Status:** Implementation plan only. All tasks and acceptance checks remain pending.

Build an independent QDNF transport and routing implementation using QualiaDB's semantic core and
the reviewed crypto adapters. The replacement's native dependency closure must contain no libp2p.
Optional compatibility carriers belong in separate adapters and cannot satisfy Native Independent
acceptance. Semantics express service purpose, authority and resource conditions; the protocol
implements reviewed bounded realizations without turning topology or payment into personal identity.

Read [Wire Protocol](../../wire-protocol.md), [QLink and Bearers](../../qlink-and-bearers.md),
[QRoute](../../qroute.md), [Identifier Resolution](../../identifier-resolution.md),
[QSession and Services](../../qsession-and-services.md), [PQ Security](../../post-quantum-security.md),
[Identifier Fabric Integration](../../identifier-fabric-integration.md),
[Semantic Network Roles](../../semantic-network-roles.md) and [Network Cell](../../network-cell.md).
Follow the parent [library layout](../library-layout.md), [swarm protocol](../swarm-protocol.md)
and [validation matrix](../validation-matrix.md).

### Ownership and execution directions

Use one integrator per parent task and separately assigned reviewers for wire/state invariants.
Agents may implement disjoint codecs, state machines, OS backends and test harnesses after claiming
their files. Shared registries, dependency manifests and public types remain single-owner changes.
Future paths below are relative to `crates/qualia-core-db/src/net/qdnf/`; they are planned Rust paths.

Each capability is a directory-backed library whose `mod.rs` routes modules and re-exports only.
Keep implementation files below 500 lines and split mixed responsibilities earlier. Hot transitions,
cold construction, backends, publication and tests occupy separate files; do not create a universal
`network.rs`, protocol megafile or backend-dependent state machine. Reuse core paging and evidence
handles rather than implementing another database or requiring the entire ontology to be resident.

Hot success/error paths use caller buffers, fixed capacity and deterministic bounded iteration,
without `Vec`, `String` or `Box`. Admit cold work and dependency scratch explicitly. Ordinary cells
have ceilings up to 512 MiB under host reservations; Webizen retains its separate 42 MiB pass budget.
Multiple cells shard ownership while sharing role allowances; they do not multiply spend or memory
credits. Runtime owners enforce leases/scheduling; each protocol supplies measurable bounds.

| Parent task | Required predecessors | Primary output |
|---|---|---|
| NET-01 | FND-03, RT-01 | QFrame codec and native IPC/raw-Ethernet bearers |
| NET-02 | NET-01, CRY-02 | Authenticated bounded QLink adjacency |
| NET-03 | NET-02, FND-02 | Intra/inter-realm routes and subnet delegation |
| NET-04 | NET-03, CRY-02, CORE-03, SEM-01 | Verified identity-aware resolution |
| NET-05 | NET-03, CRY-02, SEM-01 | Governed end-to-end sessions |

FND-03 is the joint interface baseline, not a prerequisite that all final crypto bytes already exist.
Freeze versioned codec inputs with CRY-02 before interoperable PQ acceptance. NET-05 can begin with
provisioned verified route fixtures; production discovery integration consumes NET-04's contract.

## NET-01 — QFrame and native IPC/raw bearers

**Dependencies:** FND-03, RT-01. **Owner:** framing/bearer integrator; separate native-backend reviewer.

Codec/backend work may start against reviewed lease interfaces, but NET-01 acceptance requires
RT-01's real admission ledger for buffer/completion reservations. A fixture cannot certify that gate.

- [ ] NET-01.01 Inventory existing frame, IPC and raw-I/O implementations and declare reuse/file ownership; exclude libp2p from the native build feature closure.
- [ ] NET-01.02 Freeze QFrame versions, base-header/extension offsets, byte order, reserved fields and critical-feature handling with FND-03; retain exact golden bytes.
- [ ] NET-01.03 Build caller-buffered encode/decode views with checked arithmetic, input/output overlap rules and no unsafe casts from unaligned or mutable untrusted memory.
- [ ] NET-01.04 Define typed malformed, truncated, unsupported, capacity, would-block and closed outcomes; specify output validity and consumed-byte behavior for every failure.
- [ ] NET-01.05 Define bearer contracts for observed locators, scope epochs, MTU, group delivery, ordering and duplication; payload claims must never replace observed source metadata.
- [ ] NET-01.06 Implement IPC framing and access checks with bounded queues, boot-scoped namespaces and validated lease ranges; OS credentials supplement protocol authority only.
- [ ] NET-01.07 Implement each supported raw-Ethernet backend separately, using configured development EtherType/group settings and explicit unsupported-platform errors.
- [ ] NET-01.08 Implement interface initialization, MTU changes, degradation, drain and shutdown; release filters and owned resources on partial initialization and backend failure.
- [ ] NET-01.09 Bound frame/control sizes, parse steps and nested extensions before copying; distinguish general authenticated fragmentation from the CRY-02 admission chunk profile.
- [ ] NET-01.10 Reserve transmit, receive, completion and cleanup capacity through runtime leases; test simultaneous producers, short output buffers and backpressure without hidden allocation.
- [ ] NET-01.11 Export scoped byte/retry/airtime observations and explicit unknown energy/compute values where unavailable; keep contracts and settlement outside bearer hot paths.
- [ ] NET-01.12 Add deterministic round-trip and independent golden-codec tests plus fuzz cases for lengths, offsets, critical flags, truncated extensions and integer overflow.
- [ ] NET-01.13 Measure success/rejection allocations and queue/work bounds during saturation, interface loss, cancelled I/O and restart, including buffers still owned by OS/DMA.
- [ ] NET-01.14 Demonstrate raw QFrame exchange with IP unconfigured on the tested interface and capture no emitted ARP/NDP/DHCP/DNS/IP; label IPC and unsupported-host results separately.

**Planned files:** `frame/{header,extensions,decode,encode,errors}.rs`;
`bearer/{contract,metadata,lifecycle}.rs`; `bearer/ipc/` and `bearer/raw_ethernet/` each split into
backend files and bounded I/O adapters. Place codec fuzz targets and capture tests outside production modules.

**Deliverables and acceptance:** Exact frame vectors, API/error matrix, native feature closure,
backend support table and real interface captures with allocation/queue results. Synthetic IPC tests
alone cannot close raw-Ethernet acceptance; record unavailable hardware as an open target gate.

**Handoff:** NET-02 receives observed-source guarantees, MTU/error behavior and leased frame APIs;
runtime owners receive I/O completion/cleanup obligations. Registry changes go through FND-03's owner.

## NET-02 — QLink discovery and authenticated adjacency

**Dependencies:** NET-01, CRY-02. **Owner:** adjacency integrator; separate bootstrap/security reviewer.

- [ ] NET-02.01 Freeze PQ discovery/bootstrap profiles against CRY-02 without interpreting classical beacon fields as hybrid shares; publish valid state transitions and timeout rules.
- [ ] NET-02.02 Implement scoped public/manual/private discovery with bounded cadence, epoch acceptance and relationship-secret rotation; exclude stable personal identifiers from beacons.
- [ ] NET-02.03 Validate observed source, scope, role and MTU before adjacency binding; handle simultaneous open with deterministic transcript ordering and complete-key-tuple deduplication.
- [ ] NET-02.04 Enforce pre-validation reply/amplification limits and cookie expiry using observed locators; a cookie proves reachability only and cannot grant membership or application access.
- [ ] NET-02.05 Gate PQ chunks through CRY-02 admission slots, limiting each flight to 16 KiB/16 chunks and pending handshakes to 32 globally/two per admitted locator, or tighter local bounds.
- [ ] NET-02.06 Reject conflicting chunks/lengths, stale retries and invalid cookies; duplicates neither extend deadlines nor allocate another flight, and full tables return bounded admission failure.
- [ ] NET-02.07 Integrate fresh hybrid shares, paired authority verification and Finished with separate link keys; activate adjacency only after the required bidirectional confirmation.
- [ ] NET-02.08 Implement authenticated framing, replay windows, packet/nonce exhaustion and bounded key-phase overlap; erase ephemeral material after its defined retransmission lifetime.
- [ ] NET-02.09 Separate post-authentication data fragmentation from admission chunks; bound reassembly, reject contradictory overlaps and expire abandoned messages without trusting partial content.
- [ ] NET-02.10 Handle locator/MTU change, key expiry, revocation, neighbor loss and drain through explicit transitions; never silently downgrade PQ or carry application data during reauthentication.
- [ ] NET-02.11 Emit bounded adjacency observations and current generation handles to routing; link possession and local discovery must not imply persistent controller or router-role authority.
- [ ] NET-02.12 Charge concurrent handshakes, retry buffers, crypto workers and neighbor tables before admission; preserve bounded cleanup/control progress under flood and cancellation.
- [ ] NET-02.13 Test spoofed sources, beacon correlation controls, replay, role substitution, malformed shares, missing Finished, unsupported MTUs and short/oversized authority chains.
- [ ] NET-02.14 Demonstrate two native neighbors establishing and rotating PQ adjacency under loss/reordering/restart with measured bounds and no classical or libp2p fallback.

**Planned files:** `link/discovery/` for beacon codec and rendezvous logic;
`link/admission/` for cookies and slot ownership; `link/adjacency/` for states, replay and rotation;
`link/reassembly/` for authenticated data only. Reuse CRY-02's chunk/transcript code, with QLink
owning its timers and driver. Keep transport simulation and security fixtures in separate tests.

**Deliverables and acceptance:** Transition table, bootstrap/rekey vectors, privacy disclosure map,
DoS measurements and raw-bearer integration results. Unverified or half-confirmed neighbors must
never appear as active forwarding adjacencies, even during retry or restoration from cached state.

**Handoff:** NET-03 receives expiring generation-bound adjacency handles, authenticated MTU/metrics,
down/revoke events and control quotas; crypto owners receive any transcript/vector discrepancies.

## NET-03 — QRoute, inter-realm routing and subnet delegation

**Dependencies:** NET-02, FND-02. **Owner:** routing integrator; separate convergence/policy reviewer.

- [ ] NET-03.01 Implement versioned constitution, realm genesis and membership validation using full profile digests; separate compact coordinates, routing role and application authority.
- [ ] NET-03.02 Define LSA boot/sequence/expiry/conflict transitions and bidirectional adjacency eligibility; verify authentic conflicts before quarantine or peer penalties.
- [ ] NET-03.03 Build bounded flooding, per-origin admission, jitter, digest suppression and partition summary exchange; missing/expired records cannot invent reachability.
- [ ] NET-03.04 Construct deterministic bounded SPF tables in cold workspaces with checked metric aggregation, stable digest tie breaks and at most three configured next hops.
- [ ] NET-03.05 Publish complete immutable forwarding generations atomically; retain charged old generations until readers release them and reject incomplete or cancelled computation.
- [ ] NET-03.06 Implement hot forwarding with hop limits, current adjacency checks and bounded error replies; routers must preserve QSession ciphertext and avoid per-packet proof verification.
- [ ] NET-03.07 Verify inter-realm RPA chains, gateway authority, predecessor and expiry; reject repeated realms, paths over 16, inconsistent full digests and widened policy/sensitivity ceilings.
- [ ] NET-03.08 Validate SDR parent/gateway/child-scope, services/audience, hop limit, epoch, sequence, expiry and revocation; subnet reachability delegation must never grant child data access.
- [ ] NET-03.09 Implement route withdrawal, gateway movement and mobile-child-realm handover with bounded overlapping generations; old membership cannot reappear through a new boot ID.
- [ ] NET-03.10 Preserve metric units/profiles and unknown states; apply semantic eligibility before ranking, and consume preauthorized transit allowances without wallet or ontology execution per packet.
- [ ] NET-03.11 Enforce configured neighbor/LSA/forwarding/inter-realm table bounds and work quotas; reserve full recomputation/old-new overlap under cell/host limits and Webizen pass limits where invoked.
- [ ] NET-03.12 Define single-owner forwarding shards and generation-transfer contracts for multiple cells; migration cannot duplicate route authority, role spend allowance or live buffer ownership.
- [ ] NET-03.13 Test forged one-sided links, path loops, stale/revoked SDRs, sequence conflicts, metric overflow, widening attempts and LSA floods without default-route or LIG/DNS fallback.
- [ ] NET-03.14 Demonstrate A-to-C native forwarding through B, inter-realm convergence and mobile-subnet continuity under partitions, cell drain and link loss with repeatable table digests.

**Planned files:** `route/membership/`, `route/link_state/`, `route/spf/`,
`route/inter_realm/` and `route/delegation/`, each separating record validation from lifecycle;
`route/forwarding/{lookup,generation,errors}.rs`. Runtime owns cell scheduling; routing owns
table/shard invariants. Cold SPF construction must not share a file with the forwarding loop.

**Deliverables and acceptance:** Record/state vectors, deterministic table fixtures, bounded flood
and convergence reports, and real native multi-hop captures. Show that revocation/withdrawal and
failed construction cannot leave a partially authorized live table. Parallel simulation is not OS isolation evidence.

**Handoff:** NET-04/NET-05 receive verified path/SDR handles, route generation and expiry semantics;
semantic/runtime owners receive policy inputs, shard-transfer contracts and resource observations.

## NET-04 — DNI, RAR, QResolve, aliases and QSR

**Dependencies:** NET-03, CRY-02, CORE-03, SEM-01. **Owner:** resolution integrator; separate identity/privacy reviewer.

- [ ] NET-04.01 Define versioned DNI/RAR/Alias/withdrawal records and typed resolution outcomes; reconcile classical digest recipes with the selected PQ profile instead of silently widening fields.
- [ ] NET-04.02 Verify full identifier/digest, controller role, delegation, time/epoch, signature and revocation before accepting routing evidence; short Q42 hashes and DNI coordinates are lookup aids only.
- [ ] NET-04.03 Preserve distinctions among NaturalAgent, instrument, controller, endpoint, service, operator and payer; alias/classifier suggestions and `sameAs` cannot merge people or grant authority.
- [ ] NET-04.04 Implement the bounded lookup pipeline: 64 collected candidates, 16 cryptographic verifications, eight returned routes and at most three connection races, with explicit incomplete outcomes.
- [ ] NET-04.05 Bound alias expansion to 16 contextual candidates, preserve language/provenance/normalization and ambiguity, and require selection when unresolved; no automatic DNS interpretation.
- [ ] NET-04.06 Verify SDR and independent provider grants for delegated subnet/swarm candidates; a faster replica or introducer cannot replace the persistent target or widen the intended service scope.
- [ ] NET-04.07 Integrate exact signed evidence and collision-checked projections through CORE-03 handles; cache keys include requester context, policy/authority versions and source generation.
- [ ] NET-04.08 Implement expiry, withdrawal, rotation and current-minimum checks for positive/negative caches; provider silence means unavailable/unknown, and stale IPC results cannot restore revoked routes.
- [ ] NET-04.09 Authenticate same-sequence conflicts before durable quarantine, preserve bounded evidence references and emit explicit conflict outcomes; invalid input cannot poison a valid publisher's state.
- [ ] NET-04.10 Implement QSR scoped exact/semantic lookup over QRoute using authenticated cover manifests and core posting indexes; publisher/target quotas, expiry and source authority remain independently enforced.
- [ ] NET-04.11 Separate public, realm and private overlays with scoped keys/disclosure and authorized diversely routed queries; enforce lookup byte/work/deadline caps even against cookie-valid Sybil floods.
- [ ] NET-04.12 Apply SEM-01's symbolic authority checks and semantic provider eligibility before cost/performance ranking; unsupported or incomplete policy cannot become an allow decision.
- [ ] NET-04.13 Test compact-hash collisions, forged aliases, ambiguous names, index poisoning, equivocation injection, stale recovery chains, cross-context cache reuse and disclosure to unauthorized observers.
- [ ] NET-04.14 Demonstrate bounded resolution/rotation during partition and cell restart with durable evidence generations; distinguish no route, denied, ambiguous, conflict, unsupported and budget-exhausted outcomes.
- [ ] NET-04.15 Freeze scope/bootstrap/authority-transition records and QSR message bindings with FND-03/CRY owners; exact lookup must bootstrap from usable DNIs without recursive directory dependence.
- [ ] NET-04.16 Implement versioned exact/facet/item keys and purpose/field/key-generation domain separation; full identifiers and collision checks remain authoritative over Q42 handles.
- [ ] NET-04.17 Build authenticated radix cover readers with full child-interval validation, path compression, snapshot roots and strict prefix progress; reject overlaps, gaps presented as coverage and cross-epoch proofs.
- [ ] NET-04.18 Persist accepted epoch checkpoints, detect same-epoch root equivocation and require authorized key recovery/transition; do not select a fork using arrival order or an unverified highest epoch.
- [ ] NET-04.19 Bind each posting root to its declared input snapshot and indexing/compiler policy; distinguish publisher-asserted completeness from independently reconstructed input-to-index coverage.
- [ ] NET-04.20 Implement recoverable source publication, per-lane/per-replica receipts, withdrawal/tombstones and known-minimum version checks; submitted, durable and indexed are distinct outcomes.
- [ ] NET-04.21 Compile bounded supported conjunction/disjunction plans through SEM-01; estimates select execution order only, while unindexed entailments and unsupported negation cannot silently prune results.
- [ ] NET-04.22 Implement iterative cover/page traversal, bounded visited state, response bindings and reservation lineage; epochs, retries, branches and continuations cannot reset work or byte caps.
- [ ] NET-04.23 Implement authenticated snapshot cursors, declared cross-shard snapshot vectors and typed incomplete/empty/stale outcomes; stopped scans and partial OR queries cannot claim complete coverage.
- [ ] NET-04.24 Implement hot-key serving replication/coalescing and secondary posting covers separately from prefix splitting; preserve disclosure checks and aggregate credits across consumers and replicas.
- [ ] NET-04.25 Implement leased partition split/merge and a readiness-bound handover certificate; enforce one authorized generation writer, old/new overlap budgets and safe interrupted-cutover recovery.
- [ ] NET-04.26 Integrate directory/index service roles with core artifacts, runtime admission, funding and retention interfaces; role advertisements and paid priority cannot enlarge authority.
- [ ] NET-04.27 Exercise cold bootstrap, equivocation, omitted postings, deletion during pagination, hot keys, migration failure, token rotation and exhausted-frontier traces with independent fixtures.
- [ ] NET-04.28 Measure bounded hot-path allocations, real proof/scan work, full Webizen/cell/host budgets and multi-cell ownership under hostile queries and datasets larger than RAM.
- [ ] NET-04.29 Compare QSR against asynchronous cached Kademlia with matched transport, crypto, semantic indexing, freshness, replication and aggregate budgets; include bootstrap and index-maintenance costs.
- [ ] NET-04.30 Publish workload-specific performance/availability/coverage results and tradeoffs before superiority claims; remove target Kademlia lookup dependency and reject undocumented fallback.
- [ ] NET-04.31 Exclude protected-person age/status/location/relationship facets and unauthorized existence proofs from public QSR views; protect humanitarian eligibility from pricing-driven discovery.
- [ ] NET-04.32 Require scoped discovery and introduction grants independent of group membership and funding; identifier knowledge does not grant contact or third-party introduction.
- [ ] NET-04.33 Rate-limit invitations and notifications before expensive crypto/human attention, with nondisclosing rejection and no hidden-account or block-status oracle.
- [ ] NET-04.34 Test blocked actors using aliases, relays, group members and paid index roles; apply scoped evasion controls without a global person-correlation registry.

The [QSR algorithm](../../qualia-scoped-rendezvous.md) and
[evaluation protocol](../../qsr-evaluation.md) govern NET-04.15–30. Keep keys, cover validation,
publication, query planning, lookup execution, placement and handover in separate directory-backed
libraries under resolve/qsr; core/runtime/semantic/crypto ownership remains with the corresponding
dependency owners. Performance experiments belong to QA-02 integration and can proceed against
the already reviewed runtime contracts without a circular dependency on RT-03.

**Planned files:** `resolve/records/` for independent DNI/RAR/alias/withdrawal codecs;
`resolve/validation/`, `resolve/lookup/`, `resolve/cache/` and `resolve/qsr/` for their separate
lifecycles. Shared SDR validation remains NET-03-owned; storage remains CORE-03-owned. Keep
lexical normalization/candidate preparation outside hot route lookup and identity authorization.

**Deliverables and acceptance:** Record vectors, authority/projection/cache binding contracts,
adversarial lookup results and private-overlay disclosure tests. Neither booleans named “proof
verified” nor cached identity confidence substitute for the selected verifier and current grant.

**Handoff:** NET-05/runtime receive expiring verified route bundles and typed unresolved outcomes;
core/evidence owners receive exact-source provenance and quarantine-retention needs, without a second log store.

## NET-05 — QSession and QPolicy service admission

**Dependencies:** NET-03, CRY-02, SEM-01. **Owner:** session integrator; separate transport/authority reviewer.

- [ ] NET-05.01 Freeze connection/stream/packet identifiers, encryption spaces and state transitions against CRY-02; bind complete target, route, service, purpose, profile and policy context.
- [ ] NET-05.02 Complete hybrid authentication and Finished before application delivery; accept only current purpose-scoped authority, and disable every initial-profile 0-RTT application path.
- [ ] NET-05.03 Map QPolicy allow/deny/challenge/needs-human and incomplete/error outcomes explicitly; only valid allow handles activate service traffic, never a connection or payment alone.
- [ ] NET-05.04 Implement caller-buffered packet/frame codecs, authenticated demultiplexing, replay windows and nonce exhaustion; reject unknown critical frames and malformed bounded varints.
- [ ] NET-05.05 Implement bounded stream ranges, overlap equality, final-size consistency, reset and exactly-once live-session delivery; preserve application operation IDs across reconnects.
- [ ] NET-05.06 Implement expiring datagrams and service-channel binding with full IRI collision checks; separate transport ACK, consumption, durable application and economic receipt semantics.
- [ ] NET-05.07 Back every stream/connection receive credit with aggregate reservations, including out-of-order buffers; enforce 64 streams per direction class and configured window ceilings without hidden overcommit.
- [ ] NET-05.08 Freeze executable ACK/loss/PTO/pacing/congestion vectors with at most eight ACK ranges; retransmit semantic frames under fresh packet numbers and bound sent history and per-path work.
- [ ] NET-05.09 Preserve independently reserved control/revocation/closure progress under bulk transfer or stalled consumers; priorities and purchased service cannot disable congestion or fairness controls.
- [ ] NET-05.10 Validate each migration path and material DNI/RAR change before use; preserve current grants, accepted provider terms and aggregate allowances, with at most three negotiated active paths.
- [ ] NET-05.11 If reliable-carrier compatibility is enabled, bind one end-to-end ordered carrier in the transcript and disable duplicate recovery; forbid initial in-session recovery-mode switching/multipath.
- [ ] NET-05.12 Implement bounded key update, capability refresh, expiry/revocation, drain and fresh-key restart; neither rekey nor replayed cached acceptance renews service authority.
- [ ] NET-05.13 Bind semantic-bundle/agreement digests and preauthorized resource/spend handles to governed channels; pause new billable work at exhaustion while allowing bounded reconciliation and cleanup.
- [ ] NET-05.14 Test hostile ACKs, range overlap, final-size conflicts, credit overflow, replay, nonce exhaustion, downgrade, unauthorized migration, stale allows and cross-service capability reuse.
- [ ] NET-05.15 Demonstrate native multi-hop authorized streams/datagrams under loss, reorder, stalled application, revocation and cell handoff; measure allocation, memory/work limits and independent recovery correctness.
- [ ] NET-05.16 Deliver application-facing cancellation/completion semantics and reconnect tests proving transport delivery cannot duplicate durable actions or imply accepted work/payment; coordinate persistence with core/service owners.
- [ ] NET-05.17 Enforce protected contact-request, consent, active, suspended and blocked states using current operation-specific grants; paying peers cannot bypass contact/location restrictions.
- [ ] NET-05.18 Recheck protective policy and required authorization freshness at queued/offline delivery, migration, forwarded content and recovery; sensitive delivery stays pending when freshness is unavailable, while separately authorized help remains accessible.
- [ ] NET-05.19 Prevent unconsented attachment/embed/LIG fetches from disclosing private locators or location; group admission cannot imply private-message consent.
- [ ] NET-05.20 Test guardian/organizational mandate revocation and blocked-peer intermediaries across real session/custody boundaries with reserved revocation/help capacity.
- [ ] NET-05.21 Implement private pairing and direct route exchange for known patients/clinicians without public patient, VIP-association or relationship indexing.
- [ ] NET-05.22 Apply bilateral standing permissions to routine selected medical transfers/replies with current authority checks; additional recipients or record scopes require the applicable authorization.
- [ ] NET-05.23 Enforce ciphertext-only intermediary handling and declared clinical-service decryption boundaries, with medical payload excluded from generic network/payment logs.
- [ ] NET-05.24 Test revoked clinician grants, staff/key changes and pending encrypted mailbox delivery without unapproved recipient substitution; distinguish stored, delivered and clinician-reviewed states.

**Planned files:** `session/handshake/`, `session/packet/`, `session/streams/`,
`session/datagrams/`, `session/recovery/`, `session/flow_control/` and `session/paths/`;
`contracts/admission/` validates compiled semantic decisions, while `session/services/` owns channel
bindings. Separate RTT, loss, congestion and pacing files; no monolithic transport implementation.
Reuse CRY-02 crypto and SEM-01 policy compilation. Put deterministic network simulation in test support.

**Deliverables and acceptance:** Frozen state/recovery vectors, governed-channel contracts, native
multi-hop test evidence, adversarial authority/flow tests and measured bounds. Compatibility-carrier
results remain separately labeled. Production discovery must integrate NET-04's verified result
contract before release, even though provisioned route fixtures permit earlier session development.

**Handoff:** Runtime/service/release owners receive stable stream/datagram APIs, resource and ownership
obligations, typed close/retry outcomes, durable-effect boundaries and the remaining target-specific
gates. Changes to shared errors, versions or lease semantics return to the foundation integrator.
