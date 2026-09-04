# QDNF Wire Protocol

**Status:** Normative design 0.1

## 1. Encoding

Control objects use deterministic CBOR compatible with RFC 8949 and schemas expressed in CDDL. Q42
lexicon compaction may provide a CBOR-LD view, but signed integer field keys have stable meanings and
never require a remote context fetch.

Rules:

- unsigned integer map keys and shortest integer encodings;
- no duplicate keys, indefinite-length items, or floats in core messages;
- unknown non-critical fields retained for signature verification and otherwise ignored;
- unknown critical fields reject the message;
- maximum object nesting depth 8; and
- domain-separated signatures over deterministic CBOR.

## 2. QFrame

QFrame is carried directly by Q0 bearer adapters. Its fixed base header uses network byte order:

| Field | Bytes | Meaning |
|---|---:|---|
| magic | 4 | `QDNF` |
| version | 1 | `1` |
| frame_type | 1 | discovery, control, route, session, data, or error |
| flags | 2 | critical/fragment/session flags |
| header_len | 2 | complete authenticated header length |
| payload_len | 2 | payload bytes in this frame |
| hop_limit | 1 | zero for link-only frames; decremented by routers |
| next_protocol | 1 | QLink, QRoute, QResolve, QPolicy, QSession, QSync |
| reserved | 2 | zero in version 1 |
| source_link_id | 16 | epoch-scoped QLink selector |
| destination_link_id | 16 | peer/group selector; zero only for permitted discovery |
| flow_id | 8 | per-session/route flow selector |
| sequence | 8 | replay/order value in protocol context |
| header_tag | 16 | link AEAD tag or zero only for initial bounded beacons |

Total base header: 80 bytes. Extensions follow the base header and are covered by `header_tag` after
adjacency. The complete frame must fit the bearer MTU. QLink fragmentation is allowed only for
authenticated peers, at most 16 fragments and 64 KiB reassembly, with timeout and per-peer quotas.

## 3. Frame types

| Code | Type | Forwarded |
|---:|---|---|
| 1 | `DiscoveryBeacon` | no |
| 2 | `LinkChallenge` | no |
| 3 | `LinkProof` | no |
| 4 | `LinkClose` | no |
| 16 | `LinkStateAdvertisement` | within authorized realm |
| 17 | `RealmPathAdvertisement` | gateway-to-gateway |
| 18 | `RouteWithdraw` | bounded routing scope |
| 32 | `ResolveQuery` | per selected route/source |
| 33 | `ResolveAnswer` | response route only |
| 48 | `SessionChallenge` | end-to-end |
| 49 | `SessionProof` | end-to-end |
| 50 | `CapabilityPresent` | end-to-end |
| 51 | `CapabilityDecision` | end-to-end |
| 52 | `ProtocolNegotiate` | end-to-end |
| 64 | `SessionDatagram` | end-to-end |
| 65 | `SessionStream` | end-to-end |
| 66 | `SessionAck` | end-to-end |
| 67 | `SessionReset` | end-to-end |
| 80 | `SyncOperation` | end-to-end after QPolicy |
| 81 | `ContentBlock` | end-to-end after QPolicy |
| 255 | `Error` | link-local or end-to-end by context |

## 4. Signed envelope

```cddl
qdnf-envelope = {
  0: 1,                         ; version
  1: uint,                      ; message type
  2: bstr .size 16,             ; request/message ID
  3: uint,                      ; issued_at
  4: uint,                      ; expires_at
  5: bstr,                      ; deterministic-CBOR payload
  6: tstr .size (1..512),       ; signer DID URL
  7: uint,                      ; signature algorithm
  8: bstr,                      ; signature
  ? 9: [* uint],                ; critical extension keys
  * uint => any
}
```

Algorithm 1 is Ed25519 with a 64-byte signature. Signature input is:

```text
"QDNF-ENVELOPE-V1" || deterministic_cbor(envelope_without_key_8)
```

Transport/link authentication does not make signatures optional for publishable route, delegation,
alias, capability, or semantic operation records.

## 5. QLink messages

### 5.1 DiscoveryBeacon

Contains discovery mode, truncated rotating rendezvous/service tag, ephemeral link ID/public key,
bearer scope digest, protocol versions, MTU class, epoch, expiry, and optional proof-of-rate-limit.
Private beacons contain no stable identifier.

### 5.2 LinkChallenge and LinkProof

The challenge supplies a 256-bit random nonce, both observed locator digests, both ephemeral keys,
bearer scope, epoch, and transcript hash. The proof signs the transcript and contributes ephemeral
key agreement. Private modes prove rendezvous-secret possession.

Derived keys are purpose-separated for frame AEAD, beacon privacy, and rekeying. A received source
bearer locator is never accepted from a claimed payload field; the adapter's observation is bound
into the transcript.

## 6. QRoute messages

### 6.1 LinkStateAdvertisement

Contains network/realm/origin node, up to 32 adjacent link IDs, metric classes, sequence, issue/expiry,
routing-key reference, and signature. Cheap structural/rate checks precede signature verification.

### 6.2 RealmPathAdvertisement

Contains destination network/realm, gateway DNI, ordered realm path (maximum 16), sensitivity/policy
ceiling, service classes, sequence, expiry, prior path digest, and gateway signature. A receiver
rejects repeated realms, its own realm already in the path, unauthorized export, or widened ceilings.

### 6.3 Route forwarding

Forwarded frames include destination network, realm, and node IDs in an authenticated routing
extension. Each router verifies incoming adjacency, decrements hop limit, enforces route policy, and
selects a verified next hop. QSession payload remains end-to-end encrypted.

## 7. QResolve messages

```cddl
resolve-query = {
  0: bstr .size 32,             ; target digest
  ? 1: tstr .size (1..512),     ; full target where disclosure permits
  2: [1*8 uint],                ; acceptable native/transition profiles
  3: uint,                      ; relationship/context Q42 index
  4: uint,                      ; requested operation class
  5: uint                       ; max answers <= 8
}

resolve-answer = {
  0: uint,                      ; outcome
  1: [* bstr],                 ; <= 8 encoded RARs
  2: [* source-evidence],       ; bounded provenance
  ? 3: uint                     ; retry_after seconds
}

source-evidence = { 0: uint, 1: bstr .size 32, 2: uint }
```

The response signer attests only to what it returned. Each embedded RAR is independently verified.

## 8. QSession handshake

```text
Idle -> Resolving -> RouteSelected -> PathConnecting -> LinkAuthenticated
     -> PersistentTargetVerified -> CapabilityNegotiated -> Active
     -> Rekeying/PathMigrating -> Draining -> Closed
```

`SessionChallenge` binds a random nonce, QLink/QRoute transcript digest, persistent target digest,
selected DNI, both ephemeral session keys, and proposed application protocols. `SessionProof` signs
the final transcript with a key authorized by the persistent target's DID method or resource control
record.

An embedded public key without method authorization fails. Link failure may try another verified
route. Capability denial must not try another route because authorization is not a reachability issue.

## 9. QPolicy messages

`CapabilityPresent` supplies bounded capabilities/consent/delegation/selective-disclosure proofs and
the exact requested operation digest. The receiver verifies controller/issuer, presenter, audience,
target, context, action, purpose, expiry, nonce, revocation, sensitivity, and deontic rules.

`CapabilityDecision` is `allow`, `deny`, `needs_human`, or `challenge`, with stable reasons and an
optional signed receipt. Raw private credentials are not logged by default.

## 10. QSession data

### 10.1 Datagrams

QSession datagrams carry service ID, message ID, optional acknowledgement request, expiry, and AEAD
ciphertext. They are unordered and independently replay-protected.

### 10.2 Reliable streams

Streams carry session ID, stream ID, byte offset, FIN/reset flags, receive window, and ciphertext.
ACKs contain bounded selective ranges. Senders implement retransmission timeout, flow control, and
a bearer-appropriate congestion policy. Reliability state is bounded per peer and stream.

The existing Qualia chat reliable-channel logic is a reusable starting point, not proof that the
general stream protocol exists.

### 10.3 Service registry

Core service references map existing application roles without IP ports:

- `qdnf:service:chat`
- `qdnf:service:presence`
- `qdnf:service:share`
- `qdnf:service:qdp`
- `qdnf:service:resolve`
- `qdnf:service:sync`

The 64-bit `service_id` is `q_hash` for fast dispatch. Peers negotiate and retain the full service
reference before use, eliminating hash-only ambiguity.

## 11. Transition carriers

QFrames may be encapsulated over UDP, WireGuard, libp2p Noise/Yamux, or WebRTC. Encapsulation does
not change QResolve/QPolicy identities or permissions. It does make the deployment operationally
dependent on the carrier's IP/DNS/bootstrap mechanisms unless endpoints were supplied directly.

Current identifiers remain during migration:

- `/qualia/crdt-sync/1.0.0` — compatibility sync gate;
- `/qualia/sync-ops/1.0.0` — signed operation transfer;
- proposed `/qualia/qdnf/transition/1.0.0` — QFrame stream encapsulation.

## 12. Error codes

Core errors are `malformed`, `too_large`, `unsupported_version`, `unsupported_method`,
`unsupported_bearer`, `invalid_proof`, `expired`, `replayed`, `not_found`, `not_authorized`,
`blocked`, `rate_limited`, `conflict`, `temporarily_unreachable`, `route_loop`, and `needs_human`.
Diagnostics are bounded plain text and never executed/rendered as markup.

## 13. Resource bounds

| Item | Maximum |
|---|---:|
| Object nesting | 8 |
| Reassembled control object | 64 KiB |
| Fragments per object | 16 |
| Routes per RAR / RARs per answer | 8 / 8 |
| Raw resolver candidates / cryptographic verifications | 64 / 16 |
| Parallel route attempts | 3 |
| LSA adjacencies | 32 |
| Realm path | 16 |
| Native forwarding hop limit | 64 |
| Alias candidates | 16 |
| Critical extensions | 16 |

Tier-1 parsing, lookup, forwarding, packet processing, and policy evaluation use caller-owned fixed
buffers or bounded arenas. Expensive cryptography follows cheap size, expiry, replay, and block
checks. Each pass remains within the 42 MiB Sentinel ceiling.
