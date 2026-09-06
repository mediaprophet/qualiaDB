# QDNF Implementation and Conformance

**Status:** Implementation plan and conformance specification 0.1

## 1. Implementation stance

QualiaDB has working encrypted P2P, relationship, semantic, and policy components. It does not yet
have a native below-IP QLink/QRoute stack. QDNF therefore has three categories:

- **reuse:** proven components whose semantics fit;
- **repair/integrate:** incomplete or unsafe boundaries in current code; and
- **new:** raw bearer, neighbor, route, native session, and legacy-gateway components.

## 2. Current-to-target mapping

| Existing component | Reuse | Required work |
|---|---|---|
| `p2p/social_webnet.rs` | Peer lifecycle, WireGuard tests | Retain as `udp-transition-v1`; do not present as native-independent. Replace free-form peer key at security boundaries with verified target/DNI handles. |
| `p2p/wireguard_runtime.rs` | Roaming, reusable buffers, IPv6 validation | Bind observations to QDNF session evidence; keep observed endpoints separate from signed route facts. |
| `p2p/mesh_datagram.rs` | App demux concepts | Map ports to full QDNF service references; implement QFrame/QSession caller-buffered codecs. |
| `p2p/mesh_service.rs` | Background service pattern | Add shared bearer, readiness-driven I/O, route lifecycle, quotas, and cancellation. |
| `p2p/swarm.rs` | Kademlia/mDNS transition adapters | Add signed RAR lifecycle; never use raw stable DIDs in private discovery. |
| `p2p/protocol.rs` | Length framing and deterministic semantic encoding | Remove duplicate protocol-local `NQuin`; complete credential extraction; remove placeholder counts; separate resolution and authorization. |
| `p2p/sync_ops.rs`, `sync_node.rs` | Real signed-op byte transfer | Apply QPolicy before application delivery; bound relay storage/message sizes. |
| `connection_identifier.rs` | Offline `qcx1_`, expiry, hints | Create canonical `qcx2_`; verify DID-method signer authority, allowed hint types, derived addresses, and durable nonce replay. |
| `social_peers.rs`, `social_mesh.rs` | Relationship-to-peer bridge | Store record digests, context, expiry, epoch, verification evidence, and block state separately from display fields. |
| `handshake.rs` | Domain-separated challenge pattern | Bind target, selected DNI, QLink/QRoute/QSession transcript and authorized method key. Embedded key alone fails. |
| `identity/identifier.rs` | Fast Q42 pointer | Expose its role as QRC; use full identifier and strong digest for security. |
| `crypto/verifiable_credential.rs` | Signed bounded credentials | Add route-update, replica, realm, subnet, and gateway profiles with revocation. |
| `foundation/crdt.rs` | Lamport logic, context checks, M-of-N queue | Implement actual DelegatedAccess proof verification before network use. |
| `modalities/*` | Policy, expiry, conflict, uncertainty | Define bounded network-policy frames and stable allow/deny/audit/prioritize/interactive mapping. |
| `.q42` volumes | Content and indexes | Add signed swarm manifests/strong digests; keep topology records distinct from immutable content. |
| `Q42Volume`, `Q42LexMmap`, BIDX/PIDX, range/cursor APIs, WAL | Shared semantic storage, compact indexes, bounded read and mutation primitives | Build scoped network adapters; preserve exact signed records and immutable handles, enforce expiry/revocation, and prove bounded crash recovery before settlement use. |
| CBOR-LD/Q42 lexicon and agreement primitives | Semantic compaction, graph and rule infrastructure | Bind pinned contexts/ontologies/tables/shapes/rules; validate the exact selected CBOR-LD draft, signature semantics, and bounded contract compilation. |
| Commons gates, `modalities/value_flow.rs`, swarm/ILP services | Local obligation arithmetic, resource hints, settlement instruction shapes | Verify independent access authority, attributable metering, accepted terms, payment finality, and durable reconciliation. `Sent` and obligation flags are insufficient; see the source review §7.1. |

## 3. New libraries

These libraries implement network protocols over the existing QualiaDB core. They MUST reuse
[Q42 storage, indexing, and core persistence](./core-storage-and-cache.md) for durable records,
semantic bundles, and accounting facts. They do not create a parallel database, ontology store,
or ledger. Add needed storage guarantees in focused modules under the existing Q42/core owners.

```text
crates/qualia-core-db/src/net/qdnf/
  mod.rs                 # routing/re-exports only
  types.rs               # fixed/bounded common types
  frame.rs               # QFrame caller-buffered codec
  bearer.rs              # bearer traits, no backend code
  link.rs                # QLink state machine
  neighbor.rs            # bounded adjacency table
  realm.rs               # realm constitution and membership
  route.rs               # intra/inter-realm routing
  forward.rs             # hot-path forwarding
  dni.rs                 # DNI derivation
  advertisement.rs       # RAR/withdrawal validation
  resolver.rs            # local-first QResolve
  rendezvous.rs          # rotating private discovery
  session.rs             # QSession handshake/migration
  stream.rs              # reliability/flow control
  capability.rs          # QPolicy bridge
  subnet.rs              # SDR validation
  swarm.rs               # replica authorization
  alias.rs               # Alias Assertions
  errors.rs              # stable outcomes

crates/qualia-core-db/src/net/qdnf/native/
  raw_ethernet.rs
  local_ipc.rs
  ble.rs                 # later profile

crates/qualia-core-db/src/net/qdnf/transition/
  udp.rs
  wireguard.rs
  libp2p.rs
  webrtc.rs

crates/qualia-client-core/src/legacy_gateway/
  mod.rs
  policy.rs
  web.rs
  dns.rs
  socket.rs
  inbound.rs
  receipts.rs
```

Rust implementation files remain focused and below the repository's 500-line code guideline. The
Markdown specifications have no artificial line cap and may be longer where completeness requires
it. Hot forwarding, cold construction, backend I/O, receipts, tests, and temporary artifacts stay
separate.

Contract and economics support adds directory-backed `contracts/` and `economics/` libraries under
the proposed QDNF tree. Separate semantic-bundle loading, CBOR-LD codecs, SHACL/N3 compilation,
quantity validation, quote/obligation state, hot budget counters, durable receipts/reconciliation,
and backend settlement adapters. Reuse existing agreement/value-flow primitives after verification;
keep external payment I/O and ontology construction outside packet/evaluator loops.

The proposed [Qualia Peer Runtime](./peer-runtime.md) adds a `net/peer/` runtime/service library
and a `qualia-peer` facade over these components. Its [API contract](./peer-runtime-api.md) and
[semantic service profiles](./semantic-peer-services.md) define the software alternative to libp2p.
Section 12 extends the programme with P10–P15 without making optional economics a prerequisite for
native connectivity. Feature isolation of the existing core is required for a minimal facade build.

## 4. Work packages

### QDNF-P0 — Canonical boundaries

- Remove/rename duplicate `p2p::protocol::NQuin` and establish one 48-byte ABI.
- Add strong digest types that cannot be confused with `q_hash`/QRC.
- Create bounded identifiers, route arrays, locators, evidence, and extension types.
- Freeze algorithm, bearer, service, and error registries.
- Define the core storage adapter: scoped record keys, exact signed-object retention, compact
  NQuin projections, strong-digest/generation handles, and a tested bounded read/write contract.

Exit: compile-time sizes, fuzzed parsers, collision-boundary tests, and no security API accepting a
bare 60-bit value as proof.

### QDNF-P1 — QFrame and raw bearer

- Implement caller-buffered QFrame encode/decode and authenticated extensions.
- Implement `Bearer` trait, `local-ipc-v1`, and `raw-ethernet-v1` development profile.
- Add MTU, fragmentation/reassembly, rate limits, and interface lifecycle.
- Prove two nodes exchange frames on an isolated bearer with IP disabled.

Exit: Native Independent loopback/raw-link tests pass with no socket/DNS/IP configuration.

### QDNF-P2 — QLink

- Implement ephemeral link IDs, beacons, challenge/proof, key derivation, rekey, close, and bounded
  neighbor table.
- Add pairwise/group HMAC rendezvous tags.
- Bind adapter-observed source locators into the transcript.

Exit: spoof, replay, locator substitution, stale epoch, unauthorized public/private discovery, and
privacy-capture tests pass.

### QDNF-P3 — QRoute

- Implement realm constitutions, signed LSAs, deterministic shortest paths, forwarding, and hop limits.
- Implement signed inter-realm path vectors, loop rejection, multipath, and policy ceilings.
- Add route withdrawal, expiry, mobility, and bounded tables.

Exit: three-realm native network routes without IP; malicious/looped/stale/equivocating advertisements
are rejected or isolated; link failure converges deterministically.

### QDNF-P4 — DNI, RAR, and QResolve

- Implement canonical DNI/RAR/withdrawal/SDR/Alias types and deterministic CBOR vectors.
- Verify DID-method signer purpose and content identifiers.
- Implement ordered caches, local realm resolver, encrypted relationship exchange, QRoute DHT, and
  paraconsistent conflict handling.
- Back those logical caches with Q42/core records and bounded derived generations. Test requester
  scope isolation, negative-result source scope, expiry/withdrawal during lookup, stale handles,
  and restart/compaction without authority resurrection.

Exit: DID/resource resolves after realm move; expiry/withdrawal works; DHT poisoning and embedded-key
substitution fail; alias ambiguity requires selection.

### QDNF-P5 — QSession and QPolicy

- Implement end-to-end session transcript, datagrams, reliable streams, multiplexing, flow control,
  rekey, and path migration.
- Bind persistent target and selected DNI to session keys.
- Gate chat/QDP/share/sync through capabilities, consent, sensitivity, and deontic evaluation.
- Implement real DelegatedAccess signature verification.

Exit: application payload arrives only after policy; route migration preserves authorized stream;
blocked/expired/revoked/wrong-scope operations fail before delivery.

### QDNF-P6 — Swarms and subnets

- Add replica authorization, immutable parallel block fetch, and mutable signed-op validation.
- Add SDR-based mobile subnet routing and child authorization.

Exit: provider loss is tolerated, malicious content fails digest, gateway cannot widen scope, and a
mobile realm changes macro path without renumbering current-epoch internal routes.

### QDNF-P7 — Legacy Internet Gateway

- Implement explicit Web/DNS/socket profiles with separated caches and evidence.
- Add capability-gated inbound publication and QDNF bootstrap import.
- Integrate the sandboxed browser and semantic-ingest provenance.

Exit: native stack operates with gateway removed; native failure never emits DNS; HTTPS evidence and
QDNF controller evidence remain separate; untrusted web content cannot execute in the resolver.

### QDNF-P8 — Transition carriers and browser

- Wrap QFrames in current WireGuard/libp2p paths without altering native semantics.
- Define end-to-end encrypted relay mailbox and WebRTC bearer.
- Label carrier dependencies accurately in UI/telemetry.

Exit: browser/native interop and malicious-relay tests pass; Transition nodes never claim Native
Independent status.

### QDNF-P9 — Ontological contracts and commons economics

Depends on P0/P4/P5; swarm billing additionally depends on P6, and Internet payment adapters on P7.

- Freeze the [CBOR-LD contract profile](./ontological-contracts.md): ontology/context modules,
  compression tables, SHACL shapes, supported N3 rules/evaluator semantics, bounded schemas,
  domain-separated signature inputs, and independent interoperability vectors.
- Implement semantic-bundle verification and bounded cold compilation into existing policy handles.
  Reject missing/mismatched dependencies, unknown duties, and plain-CBOR semantic downgrade.
- Implement [resource economics](./commons-and-resource-economics.md): joules and scoped seconds,
  evidence/unknown states, checked rates/amounts, allocation, and aggregate budget reservation.
- Bind gifts, work, community subsidies, and paid quotes to accepted agreements and specific duties.
- Implement durable contribution/settlement receipts, deduplication, cancellation, refunds, dispute
  isolation, offline exposure limits, and threshold-funded licence transitions.
- Extend the core WAL/Q42 lifecycle to recover exact signed records, projections, reservations,
  and settlement intents consistently. Test torn writes, root publication, concurrent writers,
  pinned-generation reclamation, and platform-specific durability before external submission.
- Verify adapter acknowledgements/finality and crash recovery; replace fire-and-forget success
  claims before using existing ILP dispatch for paid QDNF operations.

Exit: bundled contracts validate/execute offline with reproducible semantics; amended contexts cannot
reinterpret prior acceptance; physical/payment caps hold across concurrent sessions and migration;
retries/crashes cannot duplicate debits; contributions and final settlement reconcile correctly;
and donated/community-funded native operation works with every external payment adapter removed.
All economics and ontological-contract conformance scenarios must pass. These optional features
are not prerequisites for the first native networking slice.

## 5. Migration

### 5.1 SocialWebNet

SocialWebNet remains operational as a transition carrier. Its peer/DID labels become local policy
handles; QDNF session proof supplies authoritative binding. One-socket-per-peer is a tested
small-network mode, while shared-socket or raw-bearer QLink is the scale path.

### 5.2 `qcx1_`

A `qcx1_` invitation may bootstrap only:

- its signature proves consistency with the embedded key;
- DID authorization remains unverified until method/out-of-band proof;
- rendezvous kinds use an allow-list;
- claimed overlay address is recomputed;
- expiry and nonce are durably enforced; and
- verified data is converted to a local RAR rather than kept authoritative forever.

`qcx2_` carries a canonical invitation and RAR or encrypted route pointer.

### 5.3 Existing P2P protocols

`/qualia/sync-ops/1.0.0` remains the signed operation carrier but gains an outer QPolicy gate.
`/qualia/crdt-sync/1.0.0` remains compatibility-only until placeholders/TODOs are removed. New QDNF
protocols negotiate distinct versions and never reinterpret old payloads.

## 6. Conformance classes

### QDNF Core

Implements canonical types/CBOR, strong digests, QFrame, DNI/RAR verification, QResolve outcomes,
QPolicy interface, and byte-exact native/WASM vectors.

### QDNF Native Independent

Implements Core plus one non-IP bearer, QLink, QRoute, QSession datagrams/reliable streams, and local
QResolve. Tests run with DNS/IP networking unavailable. No Internet service is required.

### QDNF Transition

Implements Core semantics over UDP/WireGuard/libp2p/WebRTC and clearly reports the carrier's old-stack
dependency. It cannot advertise Native Independent conformance.

### QDNF Gateway

Implements Native Independent plus at least one LIG profile. Removing the LIG leaves all native
services functional.

### QDNF Realm Gateway

Validates path and subnet delegations, prevents loops/scope widening, enforces quotas, and preserves
end-to-end QSession confidentiality and child QPolicy authority.

### QDNF Ontological Contracts (optional)

Implements the pinned CBOR-LD semantic bundle, signature binding, SHACL/N3 validation/compilation,
offline interpretation, and contract conformance scenarios. Reports exact codec/draft and bundle
digests. A Q42-compacted map alone does not establish this conformance.

### QDNF Commons Economics (optional)

Implements Ontological Contracts plus the resource/obligation/settlement lifecycle and scenarios.
Reports supported funding modes and each adapter's tested settlement/finality behavior separately.
Monetary adapters are optional; claim monetary settlement only for adapters with end-to-end evidence.

## 7. Test matrix

| Area | Required tests |
|---|---|
| Native independence | two- and multi-node operation with IP/DNS disabled; no ARP/NDP/DNS packets emitted |
| QFrame | canonical bytes, truncation, bad tag, extensions, MTU, fragment timeout/quotas |
| QLink | spoof/replay/locator swap, private tag privacy, rekey, expiry, removal |
| QRoute | deterministic paths, convergence, loop, path forgery, policy ceiling, equivocation, hop limit |
| DNI/RAR | wrong signer purpose, altered field, old sequence, expiry, withdrawal, digest collision boundary |
| QResolve | source ordering, partial failure, poisoning, conflict quarantine, no silent LIG fallback |
| QSession | target/DNI/transcript binding, streams, loss/reorder, flow control, migration, reset |
| QPolicy | audience/context/action/purpose, expired/revoked grant, block precedence, real delegation proof |
| Privacy | no stable private ID in beacons, epoch unlinkability, no exact location in public RAR |
| Swarm/subnet | failover, malicious replica, child auth, scope widening, moving realm |
| Alias | multilingual equivalence, mixed script, same label/different target, accessible selection |
| LIG | cache separation, DNS leak prevention, redirect/TLS evidence, explicit boundary, gateway removal |
| Resources | zero-heap Tier-1 paths, parallel allocation tests, 42 MiB ceiling, pre-crypto rate limiting |
| Core/Q42 reuse | bounded range reads, exact signed-byte round-trip, scoped cache keys, stale handles, no expired authority after rebuild, multi-object recovery, generation reclamation |
| Ontological contracts | CBOR-LD vectors, context/ontology/table substitution, unknown duties, SHACL/N3 budgets, exact-byte signatures, offline semantic preservation |
| Commons economics | quantity/rate scope, unknown telemetry, aggregate caps, subsidy/work acceptance, duplicate delivery/debit, crash reconciliation, disputes, offline double spend, threshold release |
| Peer runtime | lease conservation, stale completions, cancel/drain, aggregate flow credit, queue fairness, pinned generations, feature/dependency isolation |
| Semantic subscriptions/sync | snapshot/live atomic cut, scope-filtered deltas, resume gaps, signature coverage, proof/range validation, causal deletion, durable dedup/application |
| Custody/carriers | recipient-key binding, durable receipt stages, expiry/retrieval/replay, reliable-carrier mapping, failed NAT probes, explicit browser dependencies |

## 8. Performance and memory

- Forwarding, neighbor lookup, route lookup, frame parsing, and policy predicates are Tier-1
  zero-heap with caller-owned buffers or fixed arenas.
- Realm construction, DID resolution, alias indexing, and UI work are Tier-2 cold bounded tasks.
- One resolution handles at most 64 raw candidates, verifies at most 16, returns at most eight, and
  races at most three.
- Routing tables and reassembly buffers have configured byte/entry ceilings and deterministic
  eviction within policy classes.
- Expensive verification occurs only after cheap length, version, expiry, replay, and block checks.
- Temporary artifacts use unique RAII directories, explicit promotion, and fail-closed budgets.
- Measure Q42-backed cache occupancy, index/evidence overhead, resident pages, heap/arena peaks,
  decoding buffers, and cold/warm latency separately. Reuse the core's caller-buffered/range APIs;
  whole-volume compatibility loaders and allocating error paths do not qualify as Tier-1.
- Cloudflare's cache figures are comparative prior art, not QDNF measurements or energy evidence;
  see [Core Storage and Cache](./core-storage-and-cache.md#7-evidence-and-economics).

## 9. Documentation repairs

- `qualia-sync-protocol.md` currently combines real behavior, placeholders/TODOs, and completion
  claims. Split it into verified current behavior and future work.
- “CGA-like” hash-derived ULA comments must not imply RFC 3972 conformance.
- “WASM peers use a relay” must name a concrete encrypted relay profile before being marked complete.
- Current WireGuard/IP paths must be labelled Transition, not proof of ARP/DNS independence.
- `did:q42:` must be described as QRC/storage dispatch unless a conforming DID method is specified.
- Existing commons “unbeatable security,” automatic threshold release, and completed payment-rail
  claims require qualification against [source-review evidence](./source-and-current-stack-review.md).
  The QDNF design cannot inherit those as implementation facts.

## 10. Release gates

QDNF 1.0 requires:

1. frozen deterministic record/frame vectors;
2. a non-IP bearer demonstration with IP/DNS disabled;
3. authenticated multi-realm QRoute with adversarial tests;
4. DID-method signer authorization for every claimed method;
5. real delegated-access proof verification;
6. native-to-native and browser/transition interop;
7. privacy review showing no stable private beacon or mandatory location leak;
8. capability denial before application delivery;
9. zero-heap and 42 MiB suites passing in parallel;
10. honest implemented/experimental/proposed documentation; and
11. an independent implementation or harness reproducing wire vectors.

Optional contract/economics claims additionally require P9 evidence and the corresponding profile
scenarios. Document-only semantic identifiers and proposed conformance tests are not passing results.

## 11. First usable native slice

The first coherent release is deliberately local and requires no DHT or gateway:

- QFrame over `local-ipc-v1` and `raw-ethernet-v1` development bearer;
- private/manual QLink discovery and two-peer adjacency;
- one-realm QRoute and bounded forwarding;
- canonical DNI/RAR with encrypted peer exchange;
- QSession datagram plus one reliable stream;
- QPolicy block/capability gate before chat delivery; and
- end-to-end tests with IP disabled covering roaming/rekey, replay, locator spoofing, embedded-key
  substitution, expiry, and block.

That slice proves the central requirement: Qualia nodes can discover, route, resolve, authenticate,
authorize, and communicate without ARP, DNS, DHCP, IP, or a cloud service. The DHT, inter-realm path
vector, swarms, mobile subnets, transition carriers, and Legacy Internet Gateway then extend it
without changing the native trust model.

## 12. Qualia peer runtime programme

The [Qualia Peer Runtime design](./peer-runtime.md) is an embeddable library alternative to libp2p,
not another wire suite. P0–P5 remain the native protocol foundations. The following packages expose
them coherently and add the semantic services. They are proposed work, not completed milestones.

### QDNF-P10 — Runtime, facade and aggregate ownership

**Depends on:** P0–P5 for native end-to-end behavior; the event/lease kernel can be developed against
an in-process deterministic host while those wire implementations are built.

- Introduce the focused `net/peer/` library and `qualia-peer` public facade with no core-to-facade
  dependency cycle. Separate hot kernel, cold preparation, platform hosts and service adapters.
- Implement caller-owned tables/pools, nonwrapping generations, consuming lease APIs, bounded
  events/effects, shared readiness-driven I/O, cancellation and definitive completion.
- Implement one aggregate admission ledger, parent-scope reservation conservation, backed receive
  credit, bounded crypto/compile queues and reserved control/reclamation capacity.
- Publish immutable policy/route generations with bounded pins, retirement and revocation ordering.
- Add verified service descriptors and governed stream/datagram/RPC entry points. The same API works
  on the test host and native IPC/raw Ethernet; private data delivery requires current authority.
- Build dependency feature gates so the native facade excludes libp2p, DNS/IP adapters, GPU/LLM and
  settlement dependencies. Record actual binary/dependency sizes rather than assuming a facade is small.

**Acceptance:** two applications discover/authenticate and exchange a reliable service plus expiring
datagrams through the facade with IP disabled; denied operations never reach the application.
Hot allocation/error tests, 42 MiB admission, flow-credit conservation, cancel/late-completion,
fairness and generation-reclamation tests pass. Constructor and host memory outside the pass are
reported separately. Constrained/WASM capability support is measured by builds, not inferred.

### QDNF-P11 — Semantic subscriptions and recoverable QSync

**Depends on:** P10, P4 core-backed evidence, P6 signed manifest/replica authority, and the semantic
contract portion of P9 for ontology-governed views. It does not depend on monetary settlement.

- Freeze descriptors, filter profile, typed service messages and `semantic-subscriptions` /
  `resumable-sync` negotiation. Define unsupported-profile and output-exhaustion behavior.
- Extend core mutation hooks with a bounded committed-delta journal and atomic snapshot/live cut.
  Implement authorized projections, bounded joins, resume tokens, gaps and reauthorization.
- Implement capped dissemination and digest recovery without exposing private topic semantics or
  hidden graph fields/counts. Test pairwise baseline membership changes before group-key extensions.
- Replace short-hash structural comparisons with full-digest checkpoint/proof exchange, explicit
  continuation and independent test vectors. Prove requested range completeness separately from
  leaf membership. Existing BIDX/PIDX/Bloom indexes remain candidate accelerators.
- Integrate actual full-envelope/controller signatures, atomic inbox/dedup/application/receipt
  publication and bounded crash recovery in core owners. Specify causal dependencies, conflict
  quarantine, tombstones and replica-epoch transitions before compaction.

**Acceptance:** an offline/reconnecting authorized subscriber reaches the declared projection or
receives an explicit gap/resnapshot outcome; revoked/unauthorized fields never leak through deltas,
proofs, counts or cached cursors. Crash/replay tests commit an operation at most once within the
declared durable epoch. Concurrent deletion and a returning stale replica do not resurrect data.

### QDNF-P12 — Independent reachability and encrypted custody

**Depends on:** P8 and P10 for transition hosts; P4/P6/P11 durability and exact-object/operation
identity for resumable custody workflows. P9 is needed only for requested contract/economic duties.

- Build native QLink/QRoute plus a direct UDP transition host using the new runtime's own peer,
  session, discovery and relay machinery. These paths must run with libp2p absent.
- Freeze `reliable-carrier` mapping for selected ordered carriers. A libp2p migration adapter may
  exist in a separate optional package, but is not the implementation foundation or an acceptance
  prerequisite. Replacement completion cannot be satisfied by that adapter.
- Expose observed-versus-signed locator state, three-way maximum dial races, retry/backoff caps,
  loser cleanup and a single application operation across migrated/duplicate connections.
- Specify and test a separate consented direct-probe/rendezvous profile before claiming standalone
  NAT traversal. Unsupported paths remain explicit; no fallback that changes target authority.
- Freeze custody envelope/recipient-key profiles, grants, quotas, retention, distinct receipt
  stages, retrieval authentication and bounded multi-custodian duplication.
- Validate browser carriers and document signaling, Web PKI, relay/IP and host limitations. Do not
  label a browser transport as native-independent.

**Acceptance:** direct/relay success and failure, revoked destination keys, oversized carrier frames,
reliable-stream head-of-line behavior, late dial completion and all custody crash boundaries are
tested. Custody receipts never imply destination application or payment finality. A failed relay or
payment adapter cannot silently widen permissions or select another settlement rail.

### QDNF-P13 — Application migration and comparative evidence

**Depends on:** P10 for core adoption; P11/P12/P9 only for features included in the release claim.

- Migrate chat/share/signed-op sync through the facade, preserving explicit consent and stable
  operation identities. Begin with shadow validation, then one authoritative mutation path per
  service; rollback retains dedup, revocation and settlement records.
- Add source-backed SDK examples for native peers, a transition peer, a restricted graph subscriber,
  a custody workflow and a community-funded service. Label unavailable optional features explicitly.
- Run an independent parser/peer harness against frozen vectors for each claimed profile; publish
  exact feature, schema, dependency and platform versions with the conformance result.
- Compare against the repository's pinned libp2p implementation, initially `0.56.0`, with matched
  payloads, topology, authority checks, persistence, encryption and resource limits. Record the
  actual lockfile revision; this number is not a claim about the latest upstream release.

| Experiment | Required measurements and interpretation |
|---|---|
| Cold bootstrap and warm reconnect | Time to authenticated and authorized service, bytes, verification work, failures; separate native and identical-underlay transition cases |
| Stream/RPC load and malformed ingress | p50/p95/p99 latency, useful throughput, fairness, peak arena/heap/resident memory, reject cost and leaked leases |
| Snapshot then sparse updates | Useful changed facts/bytes, journal/index cost, missed-update recovery; compare equivalent application semantics |
| Partition, churn and stale replicas | Convergence or explicit gap, replay/conflict/tombstone behavior, bounded queues and recovery time |
| Sleep/thermal/resource-constrained host | Attributable joules, device/airtime seconds, uncertainty, wakeups and deadline outcomes; retain unknown telemetry |
| Commons/custody workload | Accepted work, storage/egress cost, reservation exposure and receipt reconciliation; monetary finality only for tested adapters |

No mandatory numerical speedup or energy-saving claim precedes measurements. A 48-byte Quin is
compared as a record representation with its evidence/index costs, not against an entire libp2p
connection object. Additional inner encryption and reliable-carrier head-of-line effects are included.

### QDNF-P14 — Post-quantum replacement security

**Depends on:** P0 canonical crypto/digest boundaries and P1/P2/P5 transport/session integration;
develop alongside P10. PQ semantic/custody claims additionally require the relevant P9/P11/P12 work.

- Reuse and harden the existing `fips203` ML-KEM shim, `fips204` ML-DSA signer, crypto-library dispatch
  and key vault. Add focused nonallocating network adapters, explicit contexts, noncopying zeroizing
  secret ownership and real feature selection; do not reimplement cryptographic primitives.
- Implement the [target PQ profile](./post-quantum-security.md): hybrid ML-KEM-768/X25519, both
  ML-DSA-65/Ed25519 proofs, SHA-384 commitments and HKDF/HMAC-SHA-384. Freeze exact transcript stages,
  combined-secret order, directional labels, Finished checks, paired key/controller authority and
  algorithm-specific validation with independent vectors before assigning a wire suite code.
- Version record/contract/checkpoint schemas for typed full digests and dual COSE proofs. Audit
  every trust-chain edge, including recovery, route/replica grants, capabilities and contracts;
  classical-only roots cannot acquire a PQ claim merely through a hybrid transport.
- Implement the explicit cookie/relationship-gated handshake chunk profile within total byte/work
  quotas. No general unauthenticated reassembly, insecure bootstrap shortcut or MTU-driven downgrade.
- Add key rotation, erasure, downgrade/rollback resistance, evidence-cache invalidation and actual
  primitive/adapter allocation, malformed-input and cancellation tests. Keep optional SLH-DSA root
  evidence and hybrid offline custody behind their own independently tested profiles.

**Acceptance:** independent native QPR peers complete hybrid QLink/QSession and verify dual controller
and service-authority proofs with libp2p absent. Stripping either required component, changing a
digest/context, substituting one paired key, or replaying old authority fails. Complete handshake,
storage evidence and failed verification fit admitted budgets on each claimed platform. Report
algorithm and dependency versions, crypto review, vector results, memory, latency and energy costs.
Classical-only prototypes may demonstrate progress but cannot satisfy the replacement release target.

### QDNF-P15 — Q42 networking modality

**Depends on:** P0/P4 for canonical representations and core evidence; coordinate with P10/P11/P14
for admission plans, authorized projections and PQ proof/digest profiles. This preserves the current
48-byte NQuin ABI and does not require a new physical container version for ordinary graph records.

- Freeze the [networking modality](../q42-network-modality-draft.md) vocabulary and bounded
  CBOR-LD/SHACL profiles, source/derived/observation roles, units and cryptographic evidence binding.
- Implement exact source storage and Quin projections under existing Q42 owners, then compile
  generation-bound admission views for the runtime. Keep identifiers as graph kinds and use only
  existing canonical inline datatypes; allocate no network opcode/type/lane bits without a role audit.
- Reconcile the four-field FrameLayout parity helpers with five-field NQuin persistence through a
  caller/fixture audit and documented migration where necessary. Do not accept either checksum on
  untrusted input, rewrite data blindly, or treat XOR parity as cryptographic authentication.
- Verify 48-byte layout, metadata role exclusivity, full-value collision handling, typed PQ evidence,
  source/projection signature distinctions, zero-heap execution and aggregate generation/lease bounds.
- Keep any 42-byte payload / six-byte parity experiment as a separate reviewed storage/ABI proposal,
  with exact bit allocation, error model and migration evidence; it is not implicit in this modality.

**Acceptance:** network evidence and projections round-trip byte-exactly and compile into bounded
plans; stale policy/authority cannot deliver data or spend against old counters. Nonzero-metadata
parity vectors agree across supported producers/readers. Unsupported profile roles never execute.
The current Q42 header root is not overwritten with a larger PQ digest; independent full evidence
verification succeeds on tampering/substitution and fails on a short-hash-only authorization attempt.

### 12.1 Runtime conformance reporting

Report a **QPR Core Runtime** claim only after P10 and the selected underlying QDNF class pass.
The intended production replacement additionally requires P14 native session/control protection and
a dependency/build/run check demonstrating libp2p is absent. An optional foreign-stack bridge is a
separate product component; neither wrapping libp2p nor completing its carrier is replacement success.
Report semantic subscriptions, resumable sync, custody, reliable carriers and commons economics
individually with their vectors and failure/recovery evidence. Passing one optional profile does
not imply the others. Native-independent conformance still requires a demonstrated non-IP bearer.

Open freeze decisions include exact service CDDL/CBOR-LD tables, checkpoint/tree/continuation bytes,
offline recipient-key evolution, reliable-carrier packet mapping, standalone NAT probe messages,
PQ transcript/chunk/proof/digest encoding, the networking modality/parity integration, and
platform-specific dependency extraction. The architecture chooses their boundaries and tests;
it does not pretend those unimplemented protocols are deployment-ready.
