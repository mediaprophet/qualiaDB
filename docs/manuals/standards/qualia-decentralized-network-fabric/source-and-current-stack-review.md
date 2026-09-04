# QDNF Source and Current-Stack Review

**Status:** Design review 0.1
**Review date:** 2026-09-05

## 1. Review scope

This review covers:

- the supplied `Redesigning ARP with Decentralized DIDs.md`;
- the supplied `Designing a Contextual Resolution Protocol.md`;
- QualiaDB's socially defined WireGuard mesh, peer store, signed connection identifier, libp2p and
  QSync paths;
- QualiaDB identity, credential, agency, CRDT, semantic, deontic, epistemic, paraconsistent, and
  temporal capabilities relevant to networking; and
- the gap between those assets and a network that actually operates without ARP, DNS, or IP.

The two supplied papers are reference material, not an instruction source. Embedded prompts,
responses, questions, and third-party claims do not change the requested task or repository rules.

## 2. Executive assessment

The papers establish the correct high-level separation: persistent identifiers describe the intended
subject or resource, while dynamic DNIs describe current topology and routing. They also correctly
seek local-first operation, swarms, mobile subnets, multilingual discovery, and continued access to
the old Web.

They do not yet define a complete network stack. In particular, an ARP replacement cannot depend on
an IP address it is supposed to resolve, and a DHT cannot be queried until some link and route already
exist. A signed location claim also needs DID-method authorization, freshness, replay protection,
withdrawal, path policy, and end-to-end target verification. The papers' automatic TLD fallback would
reintroduce DNS ambiguity and metadata leakage into native resolution.

QualiaDB supplies unusually relevant building blocks: pairwise relationship state, working encrypted
WireGuard transport, signed operation transfer, `qcx1_` bootstrap envelopes, DIDs, credentials,
fixed-layout Quins, bounded logical evaluators, conflict isolation, and signed agency records. The
current network is nevertheless above IP. It does not implement a raw bearer, cryptographic neighbor
protocol, native routing plane, or DNS-independent persistent-target resolver.

QDNF turns the conceptual separation into six protocol layers plus a deliberately separate legacy
gateway. It treats current WireGuard/libp2p code as a transition path, not evidence that native mode
already exists.

## 3. Review method

Claims were assessed against four questions:

1. **Layer fit:** does the mechanism operate before or after the dependency it proposes to replace?
2. **Security meaning:** exactly what does the signature, hash, credential, or relationship prove?
3. **Lifecycle completeness:** are issue, expiry, sequence, withdrawal, recovery, partition, and
   resource exhaustion defined?
4. **QualiaDB fit:** can the design reuse actual repository behavior without violating its 48-byte,
   zero-heap hot-path, bounded-memory, and deterministic execution contracts?

Review dispositions are `adopt`, `correct`, `narrow`, `separate`, or `reject`.

## 4. Review of “Redesigning ARP with Decentralized DIDs”

### 4.1 Strong ideas

The paper correctly identifies ARP's unauthenticated broadcast binding as a weak boundary and proposes
cryptographic proof, decentralized publication, mobility, and an overlay more structured than a flat
broadcast domain. Those goals are retained.

### 4.2 Claim-by-claim disposition

| Source proposal or claim | Disposition | QDNF treatment |
|---|---|---|
| interface generates its own cryptographic identity | narrow | QLink uses rotating ephemeral link identity; persistent device/person/service identity is separate and selectively disclosed |
| replace MAC namespace with DIDs | correct | the bearer still uses its physical/local delivery selector; QLink authenticates adjacency without turning a public DID into a stable tracking address |
| publish signed IP-to-DID binding | reject for native mode | native QDNF has no IP target; a signed Resource Advertisement Record binds persistent target to DNI route profiles |
| use a localized DHT instead of broadcast | correct | bounded beacons establish first adjacency; only then can a verified DHT or route gossip be reached |
| DHT returns target DID and routing parameters | correct | the DHT returns untrusted signed RARs; the client verifies controller authorization, freshness, scope, and route viability |
| signature match eliminates spoofing | narrow | it prevents unauthorized record modification only after method authorization; it cannot prevent compromised keys, malicious authorized controllers, deception, or traffic analysis |
| DHT lookup is O(log N) | reject as a guarantee | it may be expected under healthy conditions but churn, eclipse attacks, partitions, replication, and adversarial routing invalidate a universal complexity claim |
| no network-wide broadcasts | narrow | bounded link-local discovery may use broadcast/multicast delivery; private rotating tags and rate limits prevent stable-identity broadcast and request storms |
| network portability from stable DID | adopt | persistent targets survive changes to link ID, DNI, bearer, path, and replica set |

### 4.3 Bootstrap circularity

The paper starts DHT gossip through an already functioning IP subnet. That can improve ARP security in
an existing network but cannot replace dependency on ARP/IP. QDNF breaks the circle:

```text
bearer observation
  -> bounded QLink discovery
  -> cryptographic adjacency
  -> realm admission
  -> QRoute reachability
  -> QResolve/DHT access
  -> persistent-target QSession
```

A DHT is consequently a resolution index above reachability, never the first link-discovery primitive.

### 4.4 Privacy correction

A stable DID in every link announcement would be a long-lived correlation beacon. QDNF instead uses
epoch-scoped link identifiers, rotating HMAC rendezvous tags for private discovery, and progressive
disclosure. The persistent target is proved end-to-end only when needed.

## 5. Review of “Designing a Contextual Resolution Protocol”

### 5.1 Strong ideas

The paper contributes five important concepts:

- text aliases should not be the routing or trust root;
- persistent content/entity/resource identifiers should be separated from topology;
- DNIs can represent route sets, swarms, and mobile subnets rather than one host address;
- human discovery should be UTF-8, multilingual, semantic, and local-first; and
- old Internet resources should remain accessible through a bridge.

All five are retained, with stricter boundaries.

### 5.2 Spatial layer

| Source proposal or claim | Disposition | QDNF treatment |
|---|---|---|
| geospatial cells guide local discovery | narrow | location is optional signed service metadata or private query context, never a mandatory public route coordinate |
| prefer services in the immediate physical cell | correct | clients may rank authorized results by disclosed proximity, topology cost, user intent, and policy; “closest” is not always safest or best |
| local mesh works without distant infrastructure | adopt | native bearer/QLink/QRoute/QResolve operation has an explicit offline acceptance test |
| geographic hierarchy supplies routing | reject as the universal model | topology and geography differ; inter-realm QRoute uses authenticated network paths, with optional spatial metrics |

Exact location can expose homes, vulnerable people, valuable equipment, and movement patterns. QDNF
supports coarse, selective, purpose-bound disclosure and privacy-preserving local tags.

### 5.3 brands, organizations, and credentials

| Source proposal or claim | Disposition | QDNF treatment |
|---|---|---|
| DIDs identify brands/organizations/people | broaden | DIDs and content/resource identifiers can refer to agents, organizations, devices, datasets, services, agreements, or other subjects according to method semantics |
| trademark/government/community VCs authenticate a brand | correct | credentials are evidence evaluated under user/realm policy; no issuer is globally mandatory |
| highest-trust trademark VC wins | reject | trust is contextual and multi-dimensional, not a scalar global ranking; ambiguous results are presented with provenance and may require user choice |
| one verified localized name eliminates phishing | reject | cryptographic proof protects binding, not human comprehension; homographs, misleading issuers, compromised keys, and deceptive UI remain threats |
| agent/entity-centric datasets replace websites | narrow | QDNF supports resource/service/content resolution and QSync but does not require one data-hosting model |

### 5.4 aliases and multilingual discovery

Native UTF-8 display and semantic equivalence are valuable, but byte strings cannot be both completely
unrestricted and safe identifiers. QDNF keeps routing language-agnostic by resolving a cryptographic
target. Human aliases are signed, scoped assertions with:

- original UTF-8 form and locale/script metadata;
- normalization and comparison profiles;
- issuer/controller/provenance;
- relationship, spatial, or community scope;
- issue/expiry/withdrawal lifecycle;
- collision and confusable warnings; and
- the exact persistent target.

QualiaDB's multimodal lexicon and NQuin graph can rank semantic candidates, but inference does not
mint ownership. The UI distinguishes “semantic match,” “credential verified,” “known relationship,”
and “controller authenticated.”

### 5.5 intent syntax

The proposed `$brand`, `@place/service`, `~community`, slash namespace, geostring, and natural-language
inputs are useful UI grammars. They are not universal wire identifiers. QDNF treats them as local
query expressions compiled into bounded QResolve constraints. A client can support multiple scripts
and input styles without forcing every router to interpret natural language.

This preserves accessibility while preventing one community taxonomy or language from becoming a
new root hierarchy.

### 5.6 DID/content and DNI/topology separation

This is the source paper's strongest architectural decision and becomes normative:

| Identifier | Stability | Meaning | Security role |
|---|---|---|---|
| DID/resource/content identifier | persistent or content-bound | intended subject/data/service | controller/integrity anchor according to method |
| QRC/NQuin hash | compact local index | lookup/VM coordinate | never sufficient for cryptographic equality |
| DNI | short-lived/topology-bound | network, realm, node, service/path selector | validated routing input, not human identity |
| bearer locator | adjacency-local | interface/MAC/radio/IPC delivery | observed transport fact only |
| alias | mutable/contextual | human discovery label | signed claim with provenance, not identity root |

Applications keep the persistent target throughout the connection. They do not “drop the DID” after
resolution: QSession authenticates the persistent target over the selected DNI route, preventing a
valid but wrong destination from being substituted.

### 5.7 signed topology updates

The paper's `[DID] -> [current DNI] + timestamp + signature` record is necessary but incomplete. QDNF's
RAR also contains network/realm scope, route profile set, sequence, issued/expiry values, service
scope, sensitivity/policy ceiling, controller verification method, derivation/version, withdrawal
linkage, and optional sealed locator data. Verification includes:

- deterministic decoding and size limits;
- full-target digest, not q_hash alone;
- signer authorization under the target method/resource controller;
- sequence/freshness/replay status;
- accepted network/realm and bearer profile;
- revocation/rotation state;
- policy/sensitivity compatibility; and
- proof that the reached peer controls the persistent target during QSession.

### 5.8 DNI swarms

The source correctly allows one target to resolve to multiple replicas. QDNF requires a signed swarm
manifest describing authorized replicas, content root/version, selection policy, expiry, and removal.
Clients may race or combine replicas within resource and privacy policy. A host signature does not
prove that it holds correct content; content digests, controller authorization, and QSync validation
remain necessary.

“Striping like BitTorrent” is optional service behavior, not routing behavior. Restricted data is not
sent to multiple replicas merely because doing so is faster.

### 5.9 DNI subnets

The mobile subnet concept is adopted. A signed Subnet Delegation Record authorizes a gateway to
advertise a bounded prefix/selector space for specified internal targets and services. It includes
scope, epoch, expiry, sensitivity ceiling, onward-delegation rule, and withdrawal authority. The
gateway cannot impersonate an internal target: QSession still verifies the target or a deliberately
delegated service key.

### 5.10 legacy bridge

The paper proposes automatically detecting TLD-shaped input and sending a DNS request, then checking
DNS TXT records for DID anchoring. QDNF separates this behavior for four reasons:

1. syntax is ambiguous, especially across languages and private/community aliases;
2. automatic DNS leaks user intent and makes native misses Internet-dependent;
3. DNS/DNSSEC/Web PKI authority is not automatically DID-method authority; and
4. fallback can turn a safe `not_found` into attacker-controlled legacy content.

Old Internet support remains available through an explicit LIG request and visible `legacy-origin`
result. DNS TXT can be presented as legacy provenance or a method-defined bootstrap hint, but cannot
silently override native QResolve.

## 6. Review of QualiaDB's current social network stack

### 6.1 `p2p/social_webnet.rs`

This is a working userspace WireGuard peer mesh keyed by pairwise relationship material. It includes
peer lifecycle and real end-to-end traffic tests. It is valuable transition infrastructure.

Limitations for QDNF native mode:

- WireGuard outer traffic requires UDP/IP;
- configuration uses endpoint information supplied by existing mechanisms;
- one UDP socket per peer does not scale like a shared native interface;
- the WireGuard key proves transport-key possession, not arbitrary DID-controller authorization; and
- no QLink/QRoute/QResolve lifecycle exists below the tunnel.

Disposition: preserve and label `transition:wireguard`; place QFrame/QSession semantics above it.

### 6.2 `p2p/wireguard_runtime.rs`

The runtime's roaming, observed endpoint updates, IPv6-only inner validation, and buffer reuse are
strong implementation patterns. QDNF reuses the distinction between configured and observed paths,
but binds path migration into authenticated transcripts and never promotes an observed locator into a
signed routing fact.

### 6.3 `p2p/mesh_datagram.rs` and mesh services

The chat/presence/share/QDP demultiplexer and reliable chat ACK/retransmit behavior prove useful
application semantics. Their IPv6/UDP headers and numeric ports are replaced by QSession channel IDs
bound to full service URIs. Reliability generalizes into bounded streams, acknowledgements, flow
control, retransmission, congestion behavior, and migration.

### 6.4 `connection_identifier.rs`

The signed `qcx1_` envelope is a strong offline-introduction shape: it packages expiry, DID, an
Ed25519 public key, WireGuard key, overlay information, and rendezvous data. It detects corruption and
self-inconsistency.

It does not by itself prove that the embedded key is an authorized verification method for the DID.
Route hints are free-form, and durable one-time nonce consumption is not completed in this module.
QDNF therefore defines a future canonical connection package that carries method-authorized proofs,
typed profiles, target/DNI derivation, invitation scope, and replay lifecycle.

### 6.5 `social_peers.rs` and `social_mesh.rs`

These modules make an accepted social relationship operational and maintain usable peer state. Their
JSON/string forms are cold application storage, not wire or hot-path layouts. QDNF separates:

- display/contact relationship;
- QLink rendezvous permission;
- route/transit permission;
- service/operation capabilities;
- expiry/revocation/block state; and
- verified routing evidence.

No friendship/contact edge grants transitive network trust.

### 6.6 libp2p paths

`p2p/swarm.rs` supplies mDNS, Kademlia, and request-response behavior; `sync_node.rs` supplies real
Noise/TCP/Yamux transport. These can carry transition traffic and inform DHT implementation. They do
not remove IP/DNS bootstrap dependencies on their own. Provider records need QDNF signatures,
authorization, expiry, withdrawal, anti-eclipse diversity, and privacy controls.

### 6.7 QSync protocol paths

`p2p/protocol.rs` contains length framing and deterministic semantic encoding, but defines a
protocol-local 48-byte NQuin representation that differs from the canonical six-`u64` Super-Quin,
contains incomplete credential extraction, and retains placeholder response behavior. `sync_ops.rs`
correctly treats the transport as an opaque signed-operation pipe and leaves authorization to the
consumer.

QDNF requires one canonical ABI and a QPolicy gate before operation delivery. Documentation must not
represent a TODO or placeholder as production-complete.

## 7. Review of reusable QualiaDB capabilities

| Capability | QDNF use | Boundary/correction |
|---|---|---|
| canonical 48-byte NQuin | policy, receipts, compact indexes | signed source objects remain available; no lossy projection as sole evidence |
| `q_hash` / lexicon | allocation-free dispatch and semantic indexing | FNV-derived values are not collision-resistant identity/proof |
| `did:q42` topological pointer | local QRC/storage acceleration | not a network DNI or cryptographic persistent ID |
| verifiable credentials | capability, realm, route, replica, gateway evidence | issuer authority and claim truth are evaluated separately |
| author-scoped Merkle roots | signed provenance/operation batches | scope and controller proof required |
| CRDT/Lamport ordering | partitioned state convergence | not sufficient for authorization, consent, or governance conflicts |
| suspended M-of-N queue | threshold admission/governance/ratification | remains bounded and signature-verified |
| deontic logic | obligations, permits, prohibitions, expiry | policy inputs are signed and context-bound |
| epistemic logic | source confidence and uncertainty | uncertainty cannot be converted silently to authorization |
| paraconsistent logic | isolate contradictory claims | contradiction is preserved, not exploded or overwritten |
| LTL evaluator | bounded temporal/network invariants | use real trace evaluator, not legacy single-value pseudo-LTL |
| SLG arena / zero-heap kernels | bounded route/policy evaluation | native network code must retain caller-buffered hot paths |
| multimodal lexicon | multilingual/phonetic/visual discovery | semantic mapping suggests candidates; it does not authenticate targets |

## 8. Critical security gaps before implementation claims

### 8.1 Delegation verification

The reviewed delegated-access path validates context and expiry but currently accepts the proof
without real signature verification. Network-boundary use must automatically deny until issuer key,
signature, audience, subject, action, nonce, revocation, and delegation-chain constraints verify.

### 8.2 Identifier ambiguity

QualiaDB uses several 48/60/64-bit compact forms. QDNF needs type-safe wrappers and full strong
digests so an NQuin object, q_hash, QRC, DNI component, key ID, and content digest cannot be confused.

### 8.3 Duplicate NQuin layout

The protocol-local structure must be removed or explicitly converted with invariant tests. Two
different layouts both called `NQuin` are an ABI and security risk.

### 8.4 Completion language

Existing documentation around sync/mesh capabilities sometimes describes intended behavior beside
working behavior. QDNF conformance is evidence-based: native mode requires below-IP tests with IP and
DNS disabled; transition mode is reported separately.

## 9. Requirements derived from the review

| ID | Requirement | Specification owner |
|---|---|---|
| R-01 | form at least a two-node network over a native bearer without ARP/NDP/DHCP/IP/DNS | QLink; Operations |
| R-02 | use rotating, non-stable pre-authentication link identifiers | QLink; Security |
| R-03 | make DHT/gossip usable only after authenticated reachability exists | QRoute; Identifier Resolution |
| R-04 | separate persistent target, QRC, DNI, bearer locator, and human alias types | Identifier Resolution |
| R-05 | verify every RAR against a method-authorized controller and lifecycle | Identifier Resolution; Crypto |
| R-06 | reauthenticate the persistent target end-to-end after route selection | QSession |
| R-07 | represent swarms and subnets with explicit signed delegation/manifests | QRoute; Identifier Resolution |
| R-08 | use full identifiers beside q_hash dispatch indexes | Wire; Registries |
| R-09 | keep location optional, coarse, selective, and purpose-bound | Security; QRoute |
| R-10 | distinguish semantic match, credentials, relationships, and authentication in UI/policy | Security; Operations |
| R-11 | never map relationship directly to operation or transit authority | QPolicy; Security |
| R-12 | preserve contradictory social claims without global explosion/overwrite | Security; Implementation |
| R-13 | provide explicit legacy access with separate caches/trust/provenance | LIG |
| R-14 | never invoke DNS on a native miss | Identifier Resolution; Operations |
| R-15 | label WireGuard/libp2p/UDP/WebRTC as transition dependencies | Architecture; Conformance |
| R-16 | enforce deterministic bounded parsing and 42 MB execution ceilings | Wire; Implementation |
| R-17 | verify delegation signatures before any network authorization | Implementation; Security |
| R-18 | test native operation with Internet, IP configuration, DHCP, and DNS absent | Conformance; Operations |

## 10. Rejected shortcuts

QDNF explicitly rejects these shortcuts:

- replacing ARP with a DHT that itself requires IP reachability;
- treating a self-signed embedded key as proof of DID authority;
- using a public stable DID as the link-layer address;
- interpreting q_hash or a truncated key hash as collision-resistant identity;
- treating DHT consensus/popularity as truth or authorization;
- selecting a global “highest trust” credential issuer;
- making precise geolocation mandatory routing state;
- claiming that cryptography eliminates social engineering or homographs;
- automatically sending TLD-shaped input to DNS;
- letting DNS TXT or Web PKI silently establish native DID ownership;
- granting service, transit, or administration because a social contact exists;
- lowering signature thresholds during partitions;
- treating WireGuard-over-UDP as independent of IP; and
- documenting planned behavior as implemented conformance.

## 11. Resulting design boundary

The review produces a clean division:

```text
Human intent / multilingual aliases
           |
           v
QResolve candidate targets ---- signed credentials/provenance
           |
           v
Persistent DID/resource/content target
           |
           v
Verified, expiring RAR set -> DNI route candidates
           |
           v
QRoute over authenticated QLink topology
           |
           v
QSession proves the persistent target and requested capability

Explicit separate request -> Legacy Internet Gateway -> DNS/IP/TLS/HTTP
```

This boundary captures the source documents' ambition without leaving ARP, DNS, or IP hidden inside
the replacement path. It also makes QualiaDB's social/semantic capabilities policy inputs rather than
turning the social graph into an unsafe universal trust network.

## 12. Review conclusion

The source concepts are directionally strong, and QualiaDB already has enough cryptographic,
semantic, logical, storage, and transition-network machinery to support an implementation program.
The missing work is foundational networking: native bearer adapters, QLink, QRoute, signed
persistent-target-to-DNI resolution, QSession, policy integration, and the isolated LIG. The QDNF
specification suite defines those pieces and keeps current working components available throughout a
staged migration.
