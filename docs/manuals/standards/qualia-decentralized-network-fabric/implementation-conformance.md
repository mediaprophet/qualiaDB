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

## 3. New libraries

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

## 4. Work packages

### QDNF-P0 — Canonical boundaries

- Remove/rename duplicate `p2p::protocol::NQuin` and establish one 48-byte ABI.
- Add strong digest types that cannot be confused with `q_hash`/QRC.
- Create bounded identifiers, route arrays, locators, evidence, and extension types.
- Freeze algorithm, bearer, service, and error registries.

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

## 9. Documentation repairs

- `qualia-sync-protocol.md` currently combines real behavior, placeholders/TODOs, and completion
  claims. Split it into verified current behavior and future work.
- “CGA-like” hash-derived ULA comments must not imply RFC 3972 conformance.
- “WASM peers use a relay” must name a concrete encrypted relay profile before being marked complete.
- Current WireGuard/IP paths must be labelled Transition, not proof of ARP/DNS independence.
- `did:q42:` must be described as QRC/storage dispatch unless a conforming DID method is specified.

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
