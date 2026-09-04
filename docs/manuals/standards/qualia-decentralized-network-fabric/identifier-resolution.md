# QDNF Identifier and Resolution

**Status:** Normative design 0.1

## 1. Identifier roles

| Identifier | Stability | Answers | Security authority |
|---|---|---|---|
| DID / DID URL | Persistent according to its method | What controlled subject/resource is intended? | DID method and authorized verification relationships |
| Content identifier | Immutable for given bytes | What exact content is intended? | Collision-resistant digest |
| Canonical resource IRI | Persistent by publisher policy | What semantic resource is intended? | Signed mapping to DID/content identifier |
| Q42 Resource Coordinate (QRC) | Local/storage-layout scoped | Where is a local Q42 object/index entry? | None by itself; 60-bit index only |
| DNI | Short-lived and topology scoped | How can a node/service be routed to now? | Signed Route Advertisement Record plus session proof |
| Alias | Mutable, contextual, multilingual | What target might a person mean? | Provenance and proof; never route authority by itself |

A DID can identify a thing, service, organization, dataset, relationship persona, claim, or abstract
resource. A natural person is not reducible to a DID and should normally use multiple
pairwise/contextual identifiers.

## 2. Security invariants

1. Full canonical identifiers are retained at security boundaries.
2. `q_hash` and low-60-bit values MAY index Qualia tables but MUST NOT decide cryptographic equality,
   key authorization, content integrity, or route ownership.
3. Strong digests use an explicit algorithm. The initial profile uses SHA-256 to align with current
   Qualia primitives.
4. A route publisher key MUST be authorized by the target DID document for
   `capabilityInvocation` or a registered QDNF route-update verification relationship.
5. A session `authentication` key cannot update routes unless it is separately authorized.
6. A VC proves issuer-controlled origin and integrity, not the objective truth of its claims.
7. DHT, directory, relay, gateway, and introducer provenance never replaces controller proof.

## 3. Q42 Resource Coordinates

The current `did:q42:` parser hashes the payload into 60 bits and sets bit 63 for VM dispatch. QDNF
calls this role a **Q42 Resource Coordinate (QRC)**.

- Existing syntax remains readable for ABI compatibility.
- A QRC MAY index a local volume, resolver cache, or signed-record blob.
- A QRC is not a content digest, DID-method result, global route, or cryptographic DNI.
- A security decision dereferences the QRC and verifies the full identifier, strong digest, and proof.
- Public documentation must not claim W3C DID-method conformance until a `did:q42` method defines
  create/read/update/deactivate operations, resolution metadata, verification relationships, and
  security/privacy properties.

## 4. DNI structure

A DNI route entry contains:

| Field | Type | Meaning |
|---|---|---|
| `network_id` | 16 bytes | Truncated strong digest of the QDNF network constitution |
| `realm_id` | 16 bytes | Self-certifying local routing realm |
| `node_id` | 16 bytes | Epoch-scoped topological node key |
| `service_id` | 64-bit Q42 index | Fast service selector; full service reference is retained |
| `route_epoch` | u64 | Mobility/privacy epoch |
| `transport_key` | bounded bytes | QLink/QSession public material |
| `path_hints` | bounded array | Candidate gateways or direct adjacencies |
| `audience` | enum/reference | Private relationship, closed group, community, or public |
| `not_before`, `expires_at` | u64 | Inclusive activation and exclusive expiry |
| `extensions` | bounded map | Registered optional data |

The route identifier is:

```text
dni_id = SHA-256("qdnf:dni:v1" || deterministic_cbor(route_entry))
```

Its diagnostic representation is `dni:qdnf:1:<base64url(dni_id)>`. The digest string alone is not
sufficient to route; the verified entry is required. A transport key, realm, node, epoch, or service
change creates a new `dni_id`.

## 5. Route Advertisement Record

A Route Advertisement Record (RAR) binds a persistent target to at most eight DNI entries:

| Field | Requirement |
|---|---|
| `target` | Full canonical DID, DID URL, content ID, or resource IRI |
| `target_digest` | Strong digest of the canonical target |
| `sequence` | Monotonically increasing controller sequence |
| `route_epoch` | Epoch shared by this route set |
| `issued_at`, `expires_at` | Bounded validity interval |
| `context` | Full relationship/world reference plus optional Q42 index |
| `routes` | One to eight DNI entries |
| `previous_digest` | Optional hash link to the prior RAR |
| `revocation_ref` | Optional method/controller revocation resource |
| `signer` | Authorized verification-method DID URL |
| `proof` | Explicit algorithm and domain-separated signature |

The maximum encoded RAR is 16 KiB. Public records should expire within 15 minutes. Private
relationship records may last up to 24 hours if local policy permits. Observed link-locator changes
may update an active adjacency without being republished as controller-signed facts.

### 5.1 Verification order

1. Reject size, depth, count, or canonical-CBOR violations.
2. Canonicalize and verify `target_digest`.
3. Recompute every `dni_id`.
4. Resolve the target's identifier method or content-integrity rule.
5. Verify that `signer` had route-update authority at issuance.
6. Verify the signature and domain separator.
7. Check activation, expiry, sequence, hash chain, withdrawal, and key revocation.
8. Validate network/realm membership and path/transport bindings.
9. Apply audience, block, relationship, sensitivity, and local route policy.

Failure rejects only that record; independent candidates may still be evaluated.

### 5.2 Equivocation

Two valid different RAR digests for the same target and sequence are a security-relevant conflict.
Neither arrival order nor ordinary LWW may select a winner. The resolver stores both in an isolated
paraconsistent context, declines new sensitive sessions, and waits for a later authorized record or
human/governance decision.

## 6. Publication scopes

### 6.1 Private relationship

The RAR is end-to-end encrypted to a peer or group and carried over an established QLink/QSession,
manual invitation, or opaque relay mailbox. Public participants learn neither target nor route.

### 6.2 Local rendezvous

Private discovery derives a rotating tag:

```text
tag = HMAC-SHA-256(shared_discovery_key,
                  "qdnf:local:v1" || epoch_window || bearer_scope)
```

The beacon contains a truncated tag, protocol version, ephemeral link ID, and short expiry. A matching
peer performs QLink challenge-response before receiving the encrypted RAR. Public services may opt
into service-class beacons.

### 6.3 QRoute DHT

After a node has QRoute connectivity, it may use the decentralized record key:

```text
SHA-256("qdnf:rar:v1" || canonical_target)
```

The DHT value is a signed RAR or bounded provider pointer. DHT nodes enforce structural quotas but
are not trust authorities. High-value resolution compares diverse providers or independent sources.
Absence means only that a particular lookup found nothing.

### 6.4 Introducers and directories

An introducer may attest that it received a specific RAR digest and may forward it under an explicit
introduction capability. It cannot modify the route, grant application capability, or make the
controller signature optional. Community directories have the same restriction.

## 7. QResolve algorithm

Inputs are a target or alias candidate set, requester context, requested operation/sensitivity,
supported profiles, time, and caller-owned result/evidence buffers.

Procedure:

1. Reject malformed/overlong targets and check block/revocation state.
2. Expand an alias into at most 16 candidates; require selection when ambiguity remains.
3. Query, in order: active sessions, encrypted relationship cache, local realm, authorized
   introducers/directories, QRoute DHT, and explicit LIG.
4. Verify each record independently of its source.
5. Deduplicate by full RAR digest and `dni_id`.
6. Quarantine equivocation and unknown critical extensions.
7. Filter by route visibility, requested policy preconditions, transport, and sensitivity.
8. Rank locally and return at most eight routes, with at most three eligible for parallel dialing.
9. Return structured evidence for accepted and rejected candidates.

The resolver never silently converts a failed native target into a DNS query.

## 8. Ranking

Default order:

1. policy eligibility and validity;
2. direct/local route before relay or gateway;
3. active authenticated session;
4. compatible sensitivity ceiling;
5. locally observed reachability and latency;
6. declared cost/energy hints;
7. deterministic tie-break by full `dni_id`.

Remote self-asserted latency, emergency, importance, or humanitarian status cannot force priority.

## 9. Caches

Separate caches are mandatory:

- controller-signed RARs;
- local observed link/path state, never republished as controller fact;
- source-specific negative results, default maximum 60 seconds;
- revocation and withdrawal evidence; and
- encrypted relationship-disclosure state.

A verified higher sequence or withdrawal invalidates older route state immediately. Clock uncertainty
is returned as evidence rather than handled by widening validity silently.

## 10. Withdrawal, rotation, and recovery

A signed Route Withdrawal Record identifies the target, withdrawn RAR/dni digests, new sequence,
effective time, reason code, signer, and proof.

- Transport-key, node, realm, or epoch changes mint a new DNI.
- Controller-key rotation follows the target DID method and need not change the persistent target.
- Recovery quorum output authorizes a controller transition only through an explicit method or
  governance operation.
- Private recovery shares never enter route records, DHT, aliases, or audit logs.

## 11. Swarms and subnet delegation

A route swarm lists multiple DNI providers for one resource. Immutable blocks require strong digest
verification. Mutable replicas require a controller-issued replica authorization and signed operation
validation.

A Subnet Delegation Record (SDR) binds parent target, gateway DNI, child realm/node scope, allowed
services/audience, hop limit, sequence, epoch, expiry, revocation, and optional M-of-N proof. It proves
routing delegation only—not ownership, guardianship, consent, or child-resource write permission.

## 12. Multilingual Alias Assertions

An Alias Assertion contains UTF-8 text, BCP 47 language, script, namespace, target identifier/digest,
issuer, evidence type, optional consented coarse region, validity, and proof.

Namespaces are `personal`, `relationship`, `community`, `institution`, and `legacy`. There is no
global uniqueness or TLD. Results show source, language/script, controller, relationship context, and
verification separately. Mixed-script/confusable aliases warn and require confirmation; they never
override the canonical target.

## 13. Resolution outcomes

`resolved`, `not_found`, `temporarily_unreachable`, `not_authorized`, `blocked`, `expired`,
`conflict`, `unsupported_method`, `unsupported_bearer`, `invalid_proof`, and `ambiguous_alias` are
stable outcomes. Failures include bounded evidence and never executable remote text.

## 14. NQuin projection

RARs, DNIs, SDRs, withdrawals, observations, and aliases may be indexed as Quins for policy and
querying. The signed canonical record remains stored by strong digest and is reverified for security
actions. Recommended predicates are `qdnf:hasRouteAdvertisement`, `qdnf:hasDni`,
`qdnf:hasTransport`, `qdnf:delegatesSubnet`, `qdnf:withdrawsRoute`, `qdnf:aliasFor`,
`qdnf:observedReachability`, and `qdnf:resolutionConflict`.

No opcode is allocated by this document. Opcode allocation requires canonical `frame_layout.rs` ABI
review.
