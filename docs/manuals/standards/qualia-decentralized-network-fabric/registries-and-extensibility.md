# QDNF Registries and Extensibility

**Status:** Normative design 0.1
**Registry epoch:** 1
**Applies to:** QDNF native and transition profiles

## 1. Purpose

QDNF needs stable wire meanings without creating a central naming authority that can become a
technical or political dependency. This specification defines the small numeric registries needed
for deterministic packet processing, the rules for adding values, and the mechanism by which a
network or realm can adopt extensions without breaking peers that do not understand them.

The registries are protocol coordination data. They are not a DNS replacement, a directory of
people, an ownership registry, or a root of trust. Persistent names and keys remain controlled by
DIDs and signed resources; admission remains controlled by each network or realm constitution.

## 2. Registry principles

Every registry defined here follows these rules:

1. A numeric value has one stable meaning for the lifetime of a major protocol version.
2. Published values are never silently reassigned.
3. Unknown optional values can be ignored only where the containing message explicitly permits it.
4. Unknown critical values fail closed with a bounded error.
5. Hash-derived dispatch values are always confirmed against their full canonical identifier.
6. A realm may define private-use values but cannot require other realms to interpret them.
7. Adoption is explicit through feature negotiation or a signed realm/network constitution.
8. Security semantics cannot be weakened through an optional extension.
9. Parsing limits apply before extension processing.
10. A decentralized registry record is accepted because its proof and governance policy verify,
    not because it was retrieved from a popular server.

## 3. Allocation classes

| Range or class | Meaning | Change process |
|---|---|---|
| core | required for the base protocol | QDNF major/minor specification process |
| standards-track | interoperable public extension | reviewed proposal plus test vectors |
| realm-profile | signed network/realm definition | constitution-authorized change |
| experimental | temporary trial; no compatibility promise | experiment identifier and expiry |
| private-use | bilateral or closed-realm use | local agreement |
| reserved | unavailable | no allocation |

Core and standards-track allocations must have a canonical specification URI, security analysis,
resource limits, and at least one positive and negative test vector. Experimental values carry an
expiry epoch and must not become permanent through indefinite renewal without standards review.

## 4. Protocol versioning

QDNF uses three related version identifiers:

- `qframe_version`, an 8-bit on-wire framing version;
- `protocol_version`, a semantic `major.minor` pair for QLink, QRoute, QResolve, QPolicy, QSession,
  or QSync; and
- `profile_version`, a version attached to a bearer, cryptographic suite, service, or extension.

A major version changes a previously valid interpretation. A minor version only adds optional
behavior or allocations. Patch numbers belong to documents and implementations and do not appear
in the core wire negotiation.

Version 1 implementations must not guess how to parse another QFrame version. Before adjacency,
they may return a bounded `unsupported_version` response containing at most eight supported
versions. After adjacency, the response is authenticated.

## 5. Feature identifiers

A feature has:

```text
feature_uri       canonical UTF-8 identifier, 1..255 bytes
feature_id        low 64 bits of q_hash(feature_uri), used for fast dispatch
major             incompatible semantic generation
minor             backward-compatible generation
criticality       optional | required-for-message | required-for-session
parameters_digest SHA-256 of deterministic-CBOR parameters
```

The full URI and version are exchanged before a feature is used. `feature_id` is never sufficient
to resolve a collision. Core identifiers use the `qdnf:` URI scheme as protocol constants; this
does not imply that `qdnf:` is resolved by a global naming service.

## 6. Negotiation

Peers exchange bounded offer and selection objects:

```cddl
feature-offer = {
  0: 1,
  1: [1*64 feature-entry],
  ? 2: [* uint],                 ; critical feature indices
  ? 3: bstr .size 32             ; complete offer digest
}

feature-entry = {
  0: uint,                       ; feature_id
  1: tstr .size (1..255),        ; feature_uri
  2: uint,                       ; major
  3: uint,                       ; minimum minor
  4: uint,                       ; maximum minor
  ? 5: bstr .size 32             ; parameter digest
}

feature-selection = {
  0: 1,
  1: [* selected-feature],
  ? 2: [* rejected-feature]
}

selected-feature = [uint, tstr, uint, uint]
rejected-feature = [uint, uint]
```

Selection uses an exact major-version match and the highest mutually supported minor version unless
policy selects a lower version. Both transcripts are authenticated by QLink or QSession. A critical
feature that cannot be selected terminates the affected message or session; it does not downgrade.

## 7. Critical extensions

The signed envelope lists critical extension map keys in field `9`. The QFrame extension area uses
a critical bit on each type-length-value entry. A receiver must:

1. validate lengths and duplicate types;
2. retain unknown bytes when verifying the enclosing signature;
3. reject an unknown critical extension;
4. ignore an unknown optional extension without interpreting its content; and
5. never copy an unvalidated extension into a new signed assertion.

An extension cannot redefine a base field, alter signature input, disable replay checks, increase a
resource ceiling without negotiation, or convert a deny/challenge into allow.

## 8. QFrame type registry

| Code | Name | Scope | Allocation |
|---:|---|---|---|
| 0 | reserved | — | reserved |
| 1 | `DiscoveryBeacon` | link | core |
| 2 | `LinkChallenge` | link | core |
| 3 | `LinkProof` | link | core |
| 4 | `LinkClose` | link | core |
| 5–15 | link extension | link | standards-track |
| 16 | `LinkStateAdvertisement` | realm | core |
| 17 | `RealmPathAdvertisement` | inter-realm | core |
| 18 | `RouteWithdraw` | routing scope | core |
| 19–31 | routing extension | routed | standards-track |
| 32 | `ResolveQuery` | routed | core |
| 33 | `ResolveAnswer` | response path | core |
| 34–47 | resolution extension | routed | standards-track |
| 48 | `SessionChallenge` | end-to-end | core |
| 49 | `SessionProof` | end-to-end | core |
| 50 | `CapabilityPresent` | end-to-end | core |
| 51 | `CapabilityDecision` | end-to-end | core |
| 52 | `ProtocolNegotiate` | end-to-end | core |
| 53–63 | handshake/policy extension | end-to-end | standards-track |
| 64 | `SessionDatagram` | end-to-end | core |
| 65 | `SessionStream` | end-to-end | core |
| 66 | `SessionAck` | end-to-end | core |
| 67 | `SessionReset` | end-to-end | core |
| 68–79 | session extension | end-to-end | standards-track |
| 80 | `SyncOperation` | service | core |
| 81 | `ContentBlock` | service | core |
| 82–127 | service frame | service | standards-track |
| 128–191 | experimental | negotiated | experimental |
| 192–254 | private use | negotiated | private-use |
| 255 | `Error` | contextual | core |

## 9. Next-protocol registry

| Code | Protocol | Forwarding boundary |
|---:|---|---|
| 0 | none/padding | never dispatched |
| 1 | QLink | one adjacency |
| 2 | QRoute | routing control plane |
| 3 | QResolve | authorized request/response path |
| 4 | QPolicy | end-to-end policy exchange |
| 5 | QSession | end-to-end data plane |
| 6 | QSync | QSession service or direct negotiated profile |
| 7–127 | standards-track | declared by extension |
| 128–191 | experimental | negotiated |
| 192–254 | private use | negotiated |
| 255 | error payload | contextual |

Routers dispatch only QRoute control traffic and the authenticated route extension needed to
forward QSession/QResolve traffic. They must not inspect end-to-end service plaintext.

## 10. QFrame flag registry

| Bit | Name | Meaning |
|---:|---|---|
| 0 | `CRITICAL_EXT` | at least one critical header extension exists |
| 1 | `FRAGMENT` | authenticated fragment metadata present |
| 2 | `GROUP_DEST` | destination link ID denotes an authorized group |
| 3 | `ROUTED` | authenticated routing extension present |
| 4 | `E2E_PROTECTED` | payload is protected by QSession or message signature |
| 5 | `ACK_ELICITING` | receiver should acknowledge under selected protocol |
| 6 | `PADDED` | authenticated padding is present |
| 7 | `TRANSITION` | received through a declared transition carrier |
| 8–11 | standards-track | negotiated meaning |
| 12–15 | private-use | scoped to a bilateral/realm profile |

Reserved or unknown flag bits are rejected unless the negotiated extension assigns them.

## 11. Bearer profile registry

| Code | URI | Native status | Notes |
|---:|---|---|---|
| 1 | `qdnf:bearer:ethernet` | native | EtherType-framed QFrames; no ARP, NDP, or IP |
| 2 | `qdnf:bearer:ipc` | native | local sockets/shared memory with OS principal binding |
| 3 | `qdnf:bearer:serial` | native | framed point-to-point constrained link |
| 4 | `qdnf:bearer:ble-l2cap` | native | direct L2CAP profile where platform permits |
| 5 | `qdnf:bearer:radio-frame` | native | radio-specific adapter below network addressing |
| 16 | `qdnf:transition:udp` | transition | depends on IP; endpoint supplied out of band or by legacy means |
| 17 | `qdnf:transition:wireguard` | transition | encrypted IP carrier; not QDNF identity authority |
| 18 | `qdnf:transition:libp2p` | transition | transport dependency declared by multiaddress/profile |
| 19 | `qdnf:transition:webrtc` | transition | signaling/ICE dependencies declared |
| 128–191 | — | experimental | negotiated |
| 192–254 | — | profile-local | private use |

An implementation may support both native and transition profiles. Conformance reporting lists them
separately so a UDP-only build cannot claim native independence.

## 12. Cryptographic suite registry

| Code | URI | Status |
|---:|---|---|
| 0 | none | only permitted for bounded pre-authentication beacons |
| 1 | `qdnf:suite:x25519-ed25519-hkdf-sha256-chacha20poly1305` | original version 1 classical suite; does not satisfy the QPR PQ target |
| 2–31 | future core/standards suites | unassigned |
| 32–127 | standards-track suites | unassigned |
| 128–191 | experimental suites | negotiated |
| 192–254 | private suites | closed scope only |
| 255 | reserved | unavailable |

Algorithms inside suite 1 follow the Cryptographic Profile. A signature algorithm identifier is not
interchangeable with a complete key-agreement/AEAD suite identifier.

The proposed replacement profile reserves the descriptive URI
`qdnf:suite:mlkem768-x25519-mldsa65-ed25519-sha384-chacha20poly1305` for review, with no numeric
assignment yet. Its working profile name is `qpr-pq-1`. P14 must freeze its QLink/QSession handshake,
paired authority/record proofs, SHA-384 digest schemas and constrained-bearer bootstrap before code
allocation/interoperability. New profile versions must not reinterpret suite 1 fields. See
[Post-Quantum Security](./post-quantum-security.md). The replacement's default policy requires this
profile's guarantees; compatibility-only classical operation cannot claim QPR PQ conformance.

## 13. Signature, key, and digest algorithms

| Registry | Code 1 | Meaning |
|---|---:|---|
| signature | 1 | Ed25519 |
| key agreement | 1 | X25519 |
| KDF | 1 | HKDF-SHA-256 |
| digest | 1 | SHA-256 |
| MAC | 1 | HMAC-SHA-256 |
| AEAD | 1 | ChaCha20-Poly1305 |
| envelope | 1 | COSE Sign1 profile |
| sealed object | 1 | HPKE base/auth profile selected by message semantics |

Values are scoped by registry; the number `1` alone has no meaning without its field. New algorithms
require transcript, encoding, downgrade, key-validation, and test-vector definitions.

For `qpr-pq-1`, add explicitly versioned algorithm references for ML-KEM-768, ML-DSA-65, SHA-384,
HKDF/HMAC-SHA-384 and the required dual-proof policy. QDNF local numeric values remain unassigned.
COSE algorithm values come from their own standards registry, including RFC 9964 ML-DSA-65 `-49`;
never copy that signed value into a QDNF unsigned algorithm field. The optional SLH-DSA root and
hybrid custody profiles require separate identifiers and vectors, not an inferred parameter switch.

## 14. Route metric registry

Metrics are unsigned fixed-point or integer values. Floating point is prohibited on the wire.

| Code | Metric | Unit/encoding |
|---:|---|---|
| 1 | administrative cost | unsigned policy weight |
| 2 | latency | microseconds, saturated at `u32::MAX` |
| 3 | loss | failures per million |
| 4 | available capacity | kibibits per second |
| 5 | energy cost | realm-defined normalized 0..65535 |
| 6 | monetary cost | declared currency/resource class plus integer amount |
| 7 | privacy exposure | monotonic risk class 0..255 |
| 8 | trust/policy distance | monotonic realm-defined class 0..255 |
| 9 | stability | expected remaining lifetime seconds |
| 10 | sensitivity ceiling | public/restricted/classified or extension |

Unknown metrics are retained in signed advertisements but cannot improve route preference. Route
policy must define deterministic precedence and tie-breaking rather than combining incomparable
metrics into an undocumented score.

Metric 5 remains a coarse class, not a joule meter. Its profile binds the underlying quantity/work
basis, interval, evidence state, and normalization; unknown values are absent with an explicit
unknown state in the negotiated profile. Metric 6 requires asset/issuer, scale, and billing basis
before comparison. Neither metric authorizes a debit. Exact contract quantities and rates use the
separate [commons resource accounting model](./commons-and-resource-economics.md#3-energy-and-time-units).
No existing numeric meaning is reassigned by that model.

## 15. QSession frame registry

QSession's encrypted inner frames use a separate variable-integer namespace:

| Code | Frame | Critical |
|---:|---|---|
| 0 | `PADDING` | no |
| 1 | `PING` | no |
| 2 | `ACK` | yes for loss recovery state |
| 3 | `CRYPTO` | yes during handshake/rekey |
| 4 | `STREAM` | yes |
| 5 | `RESET_STREAM` | yes |
| 6 | `STOP_SENDING` | yes |
| 7 | `MAX_DATA` | yes |
| 8 | `MAX_STREAM_DATA` | yes |
| 9 | `MAX_STREAMS` | yes |
| 10 | `DATAGRAM` | negotiated |
| 11 | `SERVICE_OPEN` | yes |
| 12 | `SERVICE_CLOSE` | yes |
| 13 | `PATH_CHALLENGE` | yes during migration |
| 14 | `PATH_RESPONSE` | yes during migration |
| 15 | `NEW_PATH` | yes during migration |
| 16 | `RETIRE_PATH` | yes during migration |
| 17 | `KEY_UPDATE` | yes |
| 18 | `GOAWAY` | yes |
| 19 | `CONNECTION_CLOSE` | yes |
| 20–127 | standards-track | per extension |
| 128–191 | experimental | negotiated |
| 192–255 | private use | negotiated |

## 16. Core service registry

| Canonical service URI | Required transport behavior | Purpose |
|---|---|---|
| `qdnf:service:resolve` | request/response, bounded | QResolve access where session protection is required |
| `qdnf:service:policy` | reliable | capabilities, consent, decisions, receipts |
| `qdnf:service:sync` | reliable streams | signed CRDT and semantic operation synchronization |
| `qdnf:service:qdp` | reliable streams plus optional blocks | Qualia data protocol |
| `qdnf:service:content` | reliable or acknowledged datagram | content-addressed block retrieval |
| `qdnf:service:chat` | reliable message channel | human messaging |
| `qdnf:service:presence` | expiring datagrams | consented presence assertions |
| `qdnf:service:share` | reliable streams | authorized object/file sharing |
| `qdnf:service:governance` | reliable and signed | constitutions, votes, ratifications, withdrawals |
| `qdnf:service:diagnostics` | capability-gated | bounded operational diagnostics |

The dispatch ID is `q_hash(canonical_service_uri)`. Both parties bind the complete URI and version
into the QSession transcript before opening a channel. No service is identified solely by a port.

### 16.1 Optional contract and economics profiles

| Canonical identifier | Kind | Behavior |
|---|---|---|
| `qdnf:feature:ontological-contracts` | feature, major 1 | Required for ontology-defined contract channels; pinned CBOR-LD semantic bundle and exact-byte signature binding |
| `qdnf:feature:commons-economics` | feature, major 1 | Requires ontological contracts; funding, resource/spend reservations, contribution and settlement lifecycle |
| `qdnf:service:economics` | optional service | Reliable QSession streams carrying governed CBOR-LD economic records |

These draft profiles require the schema/ontology/table freeze and vectors in QDNF-P9 before
interoperability claims. They add no QFrame code, NQuin opcode, or required payment rail. A contract
feature is critical for its affected channel, without becoming mandatory for unrelated services.
The profile parameters bind content digests for contexts, ontologies, SHACL shapes, N3 rules,
compression mappings, and the evaluator/codec versions. Unsupported semantics cannot fall back to
plain CBOR or a different context. See [Ontological Contracts](./ontological-contracts.md).

### 16.2 Optional peer-runtime service profiles

| Canonical identifier | Kind | Behavior |
|---|---|---|
| `qdnf:feature:semantic-subscriptions` | feature, major 1 | Authorized event feeds or snapshot-plus-delta graph views, explicit gaps/resume and bounded dissemination; graph contracts require ontological-contracts |
| `qdnf:feature:resumable-sync` | feature, major 1 | QSync operation identity, scoped checkpoint/proof exchange, bounded continuation, causal deletion and durable admission |
| `qdnf:feature:encrypted-custody` | feature, major 1 | Expiring recipient-key envelope profile, bounded durable storage and distinct custody/retrieval/application receipts |
| `qdnf:feature:reliable-carrier` | feature, major 1 | Explicit QSession mapping onto a reliable ordered carrier, with outer loss/congestion control and reliable services only in the initial profile |
| `qdnf:service:pubsub` | optional service | Reliable subscription/control exchange and bounded event delivery under the selected subscription profile |
| `qdnf:service:custody` | optional service | Reliable deposit/retrieval/control streams for authorized encrypted envelopes |

These are proposed identifiers for the [QPR service design](./semantic-peer-services.md), not
claims of implemented protocols. They allocate no numeric QFrame values or NQuin opcodes. QPR itself
is a library name, not a wire feature. Existing `sync`, `content`, and `qdp` service URIs are reused.
Each feature is critical only where its semantics are requested and MUST fail closed if unsupported.
CBOR-LD contracts and economic obligations additionally require their corresponding features.

The P11/P12 work packages freeze exact message schemas, profile parameters, deterministic encoding,
signature coverage, proof/tree vectors, receipt states and carrier behavior before publication.
An event-feed-only implementation cannot claim graph-projection recovery. A custody implementation
cannot claim forward secrecy without an independently tested key-evolution profile.

### 16.3 Q42 networking semantic profile

`urn:qualia:q42:profile:network:1` is the working storage/semantic profile identifier from
[Q42 Networking Modality](../q42-network-modality-draft.md). Its proposed vocabulary namespace is
`urn:qualia:q42:network:`. P15 freezes its ontology/shapes/CBOR-LD mapping, evidence references and
compiler roles. It allocates no QFrame type, NQuin opcode, inline datatype, metadata nibble or physical
container version. Generic graph storage is not authority to execute an unsupported network profile.

## 17. Capability operation registry

Core operation URIs include:

- `qdnf:op:discover`;
- `qdnf:op:connect`;
- `qdnf:op:resolve`;
- `qdnf:op:publish-route`;
- `qdnf:op:relay`;
- `qdnf:op:read`;
- `qdnf:op:append`;
- `qdnf:op:mutate`;
- `qdnf:op:sync`;
- `qdnf:op:share`;
- `qdnf:op:administer`;
- `qdnf:op:bridge-legacy`; and
- `qdnf:op:audit`.

Capabilities name the full operation URI. Implementations may use q_hash only after collision-safe
negotiation. Unknown operations are denied, never mapped to a nearby permission.

The optional economics profile adds `qdnf:op:quote`, `qdnf:op:accept-terms`, `qdnf:op:spend`,
`qdnf:op:attest-contribution`, and `qdnf:op:reconcile`. These draft operation URIs are scoped to the
negotiated profile. None is implied by read, connect, relay, or administer permission. A spend grant
also binds payer, payee, asset, amount, purpose, expiry, and aggregate exposure limits.

The optional peer service profiles add `qdnf:op:publish-event`, `qdnf:op:subscribe`, `qdnf:op:derive-projection`,
`qdnf:op:store-custody`, and `qdnf:op:retrieve-custody`. Subscribe also requires permission to inspect
the requested source/projection; ciphertext custody does not imply plaintext read permission.
Deriving and signing a filtered view requires explicit projector authority, separate from read or
subscribe permission; a projector's signature cannot be represented as the source author's signature.
Existing read/append/mutate/sync and relay operations retain their distinct scope. These are draft
full URIs with no new numeric opcode allocation.

## 18. Outcomes and error registry

Outcome codes are stable across QResolve, QPolicy, QSession, and gateways where applicable:

| Code | Symbol | Retry class |
|---:|---|---|
| 0 | `ok` | none |
| 1 | `malformed` | after correction |
| 2 | `too_large` | after reducing request |
| 3 | `unsupported_version` | after compatible negotiation |
| 4 | `unsupported_method` | after changing method |
| 5 | `unsupported_bearer` | after changing path |
| 6 | `invalid_proof` | not automatically |
| 7 | `expired` | with fresh object |
| 8 | `replayed` | with fresh nonce/message ID |
| 9 | `not_found` | after topology/resource change |
| 10 | `not_authorized` | only after authorization changes |
| 11 | `blocked` | not automatically |
| 12 | `rate_limited` | after authenticated retry interval |
| 13 | `conflict` | after explicit resolution |
| 14 | `temporarily_unreachable` | bounded backoff |
| 15 | `route_loop` | after route change |
| 16 | `needs_human` | after a human decision |
| 17 | `challenge` | after satisfying named challenge class |
| 18 | `revoked` | only with replacement authority |
| 19 | `policy_stale` | after policy synchronization |
| 20 | `internal_failure` | bounded backoff; no private details |

Additional diagnostic data is advisory, bounded to 256 UTF-8 bytes, non-markup, and excluded from
automated authorization decisions.

## 19. Sensitivity and routing lanes

QDNF preserves QualiaDB's existing stable sensitivity classes and permissive routing lanes when a
network object is projected into NQuin form:

| Value | Sensitivity |
|---:|---|
| 0 | public |
| 1 | restricted |
| 2 | classified |
| 3–255 | realm-defined only after negotiation |

| Bits | Routing lane |
|---:|---|
| `00` | passthrough |
| `01` | commons |
| `10` | bilateral micro-commons |
| `11` | spatial |

Projection does not replace the signed source object. Unknown or locally defined sensitivity
classes can only narrow forwarding, never broaden it.

A commons lane does not imply public plaintext, and a bilateral lane does not require a monetary
payment. Gift, work, subsidy, and payment evidence can discharge agreed duties. Lane/obligation bits
are compiled results; they do not independently prove funding, consent, or settlement finality.

## 20. DID methods, resource methods, and aliases

QDNF does not keep a global allow-list of DID methods. Each constitution defines accepted method
profiles, proof purposes, algorithms, controller rules, recovery behavior, and verification
budgets. Method implementations are identified by canonical URI and version.

Alias schemes are similarly scoped. An alias resolver result must name the persistent target and
carry provenance. No alias scheme may override a direct DID/resource identifier or a locally pinned
binding without an explicit, authorized change.

## 21. Registry records

A portable allocation is a signed deterministic-CBOR record:

```cddl
registry-record = {
  0: 1,
  1: tstr .size (1..255),        ; registry URI
  2: uint,                       ; numeric allocation
  3: tstr .size (1..255),        ; canonical semantic URI
  4: uint,                       ; allocation class
  5: uint,                       ; major
  6: uint,                       ; minor
  7: bstr .size 32,              ; specification content digest
  8: bstr .size 32,              ; security analysis digest
  9: bstr .size 32,              ; test-vector bundle digest
  10: uint,                      ; issued_at
  ? 11: uint,                    ; expires_at for experimental allocation
  12: tstr .size (1..512),       ; governance signer DID URL
  13: bstr                       ; signature/envelope
}
```

Records are content-addressed and distributable through QSync, removable media, direct exchange, or
legacy publication. Retrieval location has no effect on validity.

## 22. Decentralized change governance

There is no mandatory global registry server. Interoperable public allocations follow a transparent
proposal process whose artifacts can be mirrored by anyone:

1. publish the proposal, threat analysis, deterministic schemas, and test vectors;
2. allocate an experimental value for implementation trials;
3. obtain independent interoperable implementations;
4. record objections, minority positions, and compatibility evidence;
5. ratify through the current QDNF standards constitution;
6. publish a signed registry record and immutable artifact digest; and
7. allow networks to adopt the record explicitly.

Networks remain free not to adopt an extension. A governance signature establishes the allocation's
history, not universal authority over a network.

## 23. Realm and network profiles

A signed profile can select:

- permitted bearer and transition profiles;
- cryptographic suites and deprecation dates;
- accepted DID/resource verification methods;
- route metrics and deterministic preference order;
- service versions and resource ceilings;
- sensitivity classes and export rules;
- time-confidence requirements;
- admission, revocation, and recovery policy; and
- adopted standards-track registry-record digests.

Profiles cannot exceed an implementation's hard safety ceilings. A profile asking for a 4 GiB
control message remains invalid even if correctly signed.

## 24. Compatibility rules

Two peers are compatible for a function when they share:

1. a QFrame major version on the selected bearer;
2. a cryptographic suite acceptable to both policies;
3. a common major version of the relevant Q protocol;
4. all critical extensions required for the operation;
5. a common service major version; and
6. mutually satisfiable capability and sensitivity policy.

Failure at steps 1–5 is a protocol incompatibility. Failure at step 6 is a policy decision. The two
must be reported separately so operators do not "fix" an authorization denial by downgrading.

## 25. Deprecation and emergency disablement

Deprecation records name the affected value, reason, earliest refusal epoch, safe replacements, and
authorizing governance proof. Implementations may enforce an earlier local refusal. They must never
silently substitute another algorithm or protocol.

Emergency disablement is local-first: a node can deny a compromised suite, method, signer, service,
or extension immediately. Signed realm/network notices distribute the decision. When time is
uncertain, security-critical disablement may be monotonic and require an explicit recovery action
instead of expiring automatically.

## 26. Implementation requirements

An implementation maintains bounded, caller-supplied or statically provisioned tables for hot-path
dispatch. Cold profile loading may compile signed registry records into those tables. It must:

- reject duplicate numeric or semantic assignments in one active scope;
- retain the full URI beside every hash-derived ID;
- expose active versions and extensions to diagnostics without private credentials;
- distinguish native, transition, experimental, and private-use support;
- refuse unnegotiated private-use values at inter-realm boundaries; and
- produce a deterministic conformance manifest.

## 27. Required tests

Registry conformance includes:

- exact parsing of every core numeric allocation;
- collision handling for two service URIs with the same dispatch hash;
- rejection of unknown critical and reserved values;
- safe ignore behavior for an unknown optional extension;
- no downgrade when a required feature is unavailable;
- exact-major/highest-compatible-minor negotiation;
- signed registry-record verification and content-digest checking;
- expiry of an experimental allocation;
- realm profile unable to raise hard resource ceilings;
- transition-only build reported as non-native;
- deprecated suite rejected according to local policy; and
- stable deterministic conformance output across runs.

Contract/economics profiles additionally test missing or mismatched semantic-bundle dependencies,
full-term collisions, incompatible units/assets, unknown duties, and refusal of plain-CBOR downgrade.

## 28. Initial registry publication

Until a signed machine-readable registry bundle is implemented, the tables in this document and the
other QDNF 0.1 specifications are the authoritative initial allocation set. Implementations must pin
their content digest at build time and report it. A future registry bundle may encode these exact
assignments but cannot retroactively change their meanings within QDNF major version 1.
