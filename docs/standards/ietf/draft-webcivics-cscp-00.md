# Capability-Scoped Connectivity Protocol (CSCP)

```text
Internet Engineering Task Force (IETF)                     T. Holborn
Internet-Draft                                                 WebCivics
Intended status: Standards Track                        10 September 2026
Expires: 14 March 2027

              Capability-Scoped Connectivity Protocol (CSCP)
                     draft-webcivics-cscp-00
```

## Abstract

This document specifies the Capability-Scoped Connectivity Protocol
(CSCP). CSCP makes purpose, protection policy, and resource budget
first-class objects of connection establishment. A path that would
disclose an endpoint locator, consume unbounded intermediary capacity,
or deliver after a grant has been revoked is a protocol error even if
it is faster than an allowed alternative.

CSCP is a control protocol. It does not define a new congestion
controller, a new TLS ciphersuite, or a replacement for QUIC
[RFC9000]. Carriers are interchangeable. Native non-IP bearers remain
independent of this Internet profile.

This revision is an Internet-Draft: a working document, not an RFC.

## Status of This Memo

This Internet-Draft is submitted in full conformance with the
provisions of BCP 78 and BCP 79.

Internet-Drafts are working documents of the Internet Engineering Task
Force (IETF). Note that other groups may also distribute working
documents as Internet-Drafts. The list of current Internet-Drafts is
at https://datatracker.ietf.org/drafts/current/.

Internet-Drafts are draft documents valid for a maximum of six months
and may be updated, replaced, or obsoleted by other documents at any
time. It is inappropriate to use Internet-Drafts as reference material
or to cite them other than as "work in progress."

This Internet-Draft will expire on 14 March 2027.

## Copyright Notice

Copyright (c) 2026 IETF Trust and the persons identified as the
document authors. All rights reserved.

This document is subject to BCP 78 and the IETF Trust's Legal
Provisions Relating to IETF Documents
(https://trustee.ietf.org/license-info) in effect on the date of
publication of this document.

## 1. Introduction

Connection APIs on the Internet are usually a reachability race:
publish or gossip a locator, probe every candidate, and keep the
fastest path. That model is hostile to a human-centric Internet.
Protected persons cannot safely publish identity-to-IP bindings.
Humanitarian and clinical traffic cannot treat a faster direct path as
an override of a requirement not to disclose an access-network
address. Intermediaries cannot be an unaccountable default: their
capacity MUST be an explicit, bounded lease.

Existing protocols solve pieces of this problem. ICE [RFC8445] checks
candidates. TURN [RFC8656] and MASQUE [RFC9298] relay. QUIC migrates
paths. libp2p DCUtR upgrades a relayed connection and retains the
relay if the upgrade fails. Pkarr signs address records; signing does
not conceal them. HyperDHT hole punching fails when both peers have
randomizing NATs and does not relay by default.

CSCP does not claim those facts disappear. It defines the missing
control plane: who may communicate, why, under which disclosure and
budget constraints, with what evidence, and with what durable receipt
when no live path exists.

Qualia submits CSCP as an Internet-Draft only because the authors
believe it is a state-of-the-art, novel, and transformationally better
solution for a human-centric Internet: connection as a capability, not
as a race. That belief is a design claim. IETF publication, RFC
status, and field readiness are not asserted by shipping `-00` in a
repository.

### 1.1. Human-centric inversion

In this document, "human-centric" names a structural topology: the
natural person is the permanent nucleus of agency, disclosure, and
grant authority. Data, agents, credentials, and relays orbit that
nucleus. They MUST NOT replace it. This is not "human-centered" as in
User-Centered Design or Human-Centered AI, where the human is a
temporary focal point of a design process. Qualia manuals record that
visual contrast separately; this Internet-Draft stays ASCII.

The application requests:

```text
Connect(peer, purpose, protection_policy, resource_budget)
```

The protocol, not the application, MUST:

1. Admit the intent before discovery or probing.
2. Resolve a private contact descriptor (invitation or mailbox). Public
   DHT publication is optional and off unless the protection policy
   explicitly permits it.
3. Exclude every path that violates disclosure, cryptographic profile,
   operator, authority, or budget.
4. Only then rank remaining paths by validation, freshness, payload
   capacity, and performance.
5. Reserve intermediary capacity with an explicit relay lease.
6. Record path evidence produced by local transport validation.
   Remote assertions remain assertions.
7. Preserve sealed operations and receipts across carrier loss.

A faster forbidden path MUST NOT be selected. Implementations that
rank first and filter afterwards are non-conformant.

### 1.2. What CSCP is not

- Not a QUIC implementation, MASQUE profile, or ICE replacement.
- Not a new cryptographic primitive. Authentication and confidentiality
  reuse existing peer-session and transport security.
- Not a claim that NAT mapping, firewalls, or operational relays
  vanish. Decentralization changes who supplies infrastructure; it
  does not remove reachable machines, bandwidth, or retained data.
- Not an anonymity system. A relay-only profile still discloses the
  client's source address to the access relay.

## 2. Conventions and Terminology

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and
"OPTIONAL" in this document are to be interpreted as described in
BCP 14 [RFC2119] [RFC8174] when, and only when, they appear in all
capitals, as shown here.

**Human-centric:** structural topology, not a design workshop. The
natural person remains the nucleus of admission and disclosure. A
faster path that treats that person as a reachability target is
non-conformant.

**Peer:** an intended counterpart identified by a 32-byte digest, not
by a socket address.

**Purpose:** why the connection is requested. Encoded as a 60-bit
FNV-1a IRI hash plus a purpose class. Purpose is part of admission,
not decorative metadata.

**Protection policy:** disclosure class, whether end-to-end session
authentication is required, and whether public DHT publication is
admitted.

**Relay lease:** bounded intermediary forwarding (bytes, time,
participants, generation, cancellation). Distinct from a **custody
lease**, which retains sealed stored operations.

**Path evidence:** local validation of a path class, generation,
payload limit, and observed performance. A signed remote claim is an
assertion (`observer = 2`) and MUST NOT set `validated`.

**Receipt:** application delivery state. A transport acknowledgement
MUST NOT be encoded as `Committed`.

## 3. Protocol Overview

CSCP messages are carried on an already-authenticated control stream of
an admitted carrier (QUIC stream, HTTP/2 capsule stream, native QDNF
control, or a local loopback binding used for implementation proofs).
CSCP does not authenticate peers by itself.

```text
Initiator                         Fabric                         Responder
    |-- ConnectRequest (intent) ---->|                                |
    |                                |-- ContactQuery (private) ----->|
    |                                |<-- ContactDescriptor ----------|
    |                                |-- LeaseRequest --------------->|
    |                                |<-- RelayLease -----------------|
    |                                |   (live traffic on carrier)    |
    |                                |<-- PathEvidence (local) -------|
    |<-- ConnectAccept / Reject -----|                                |
    |-- Operation + later Receipt -->|                                |
```

If no permitted live path exists, the fabric MUST queue a sealed
operation and return `ConnectReject` or `Queued`, never a weakened
protection policy.

## 4. Wire Format

All multi-byte integers are network byte order. Messages MUST NOT use
indefinite-length encodings. Unknown non-critical TLV fields MUST be
ignored. Unknown critical TLV fields (tag bit 7 set) MUST cause the
message to be rejected.

```text
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                             Magic                             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|    Version    |    MsgType    |     Flags     |   Reserved    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|           Body Length         |                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                               |
|                            TLVs...                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

- Magic: `CSCP` (0x43 0x53 0x43 0x50)
- Version: 1 for this document
- Flags: bit 0 = sender requires a reply; other bits MUST be zero in v1
- Body Length: octet count of TLV body, at most 1024
- Each TLV: 1 octet tag, 2 octet length, `length` octets value

### 4.1. Message types

| Type | Name | Role |
|---:|---|---|
| 1 | ConnectRequest | Admit intent |
| 2 | ContactDescriptor | Private locator or mailbox |
| 3 | RelayLease | Bounded forwarding grant |
| 4 | CustodyLease | Bounded stored-envelope grant |
| 5 | PathEvidence | Local or remote path report |
| 6 | Receipt | Durable operation result |
| 7 | ConnectAccept | Intent admitted; selected class |
| 8 | ConnectReject | Policy, stale, expired, or revoked |

### 4.2. ConnectRequest TLVs

| Tag | Critical | Value |
|---:|---|---|
| 1 | yes | peer (32 octets) |
| 2 | yes | purpose hash (8) + class (1) |
| 3 | yes | protection: disclosure (1), require_e2e (1), public_dht (1) |
| 4 | yes | budget: bytes, work, io (three u64) |
| 5 | yes | created_ms, deadline_ms (two u64) |
| 6 | no | authority hash (u64) |

Disclosure values: 1 DirectPermitted, 2 ApprovedRelaysOnly,
3 QualifiedMultiHop, 4 Isolated.

`public_dht` MUST be 0 unless the deployment explicitly admits public
infrastructure discovery. Protected-person profiles MUST send 0.

### 4.3. ContactDescriptor TLVs

| Tag | Critical | Value |
|---:|---|---|
| 1 | yes | contact key (32) |
| 2 | yes | generation (u32) |
| 3 | yes | expiry_unix (u32) |
| 4 | yes | locator kind (1): 1 mailbox, 2 relay hint, 3 direct |
| 5 | no | locator (16) |
| 6 | no | network generation (u32) |

A descriptor whose generation is less than the receiver's last admitted
generation for that contact key is **stale**. Stale descriptors MUST
be rejected. Validity of a signature on an old generation does not
make it current. Direct locators MUST NOT be probed when disclosure
forbids peer-IP disclosure.

### 4.4. RelayLease TLVs

| Tag | Critical | Value |
|---:|---|---|
| 1 | yes | lease_id (u64) |
| 2 | yes | operator hash (u64) |
| 3 | yes | participants (two 32-octet ids) |
| 4 | yes | max_bytes, remaining_bytes (two u64) |
| 5 | yes | expiry_ms (u64), generation (u32) |
| 6 | yes | export_observations (1) |

`export_observations` MUST be 0 when disclosure is ApprovedRelaysOnly,
QualifiedMultiHop, or Isolated. Forwarding MUST stop when remaining
bytes are exhausted, expiry is reached, or the lease is cancelled.
Implementations MUST NOT enlarge remaining_bytes to finish a transfer.

CustodyLease uses the same header with MsgType 4 and MUST NOT be
interpreted as forwarding capacity.

### 4.5. PathEvidence TLVs

| Tag | Critical | Value |
|---:|---|---|
| 1 | yes | path class (1): 1 DirectV6, 2 DirectV4, 3 Relayed, 4 Browser, 5 Offline |
| 2 | yes | generation (u32) |
| 3 | yes | observer (1): 1 LocalTransport, 2 RemoteAssertion |
| 4 | yes | validated (1). MUST be 0 unless observer is LocalTransport |
| 5 | no | max_payload (u16), rtt_ms (u32), observed_at_ms (u64) |

Receivers MUST treat `validated = 1` with `observer = 2` as malformed
and reject the message.

### 4.6. Receipt TLVs

| Tag | Critical | Value |
|---:|---|---|
| 1 | yes | op_id (u64) |
| 2 | yes | content digest (32) |
| 3 | yes | peer (32) |
| 4 | yes | grant generation (u32) |
| 5 | yes | status (1): 1 Queued, 2 Received, 3 Validated, 4 Committed, 5 Denied, 6 Expired |
| 6 | yes | expiry_unix (u32) |

Replay of the same op_id with a different digest MUST yield Denied.
Replay after grant revocation or generation mismatch MUST yield Denied.
Status Committed MUST NOT be sent because a relay forwarded bytes.

## 5. Exclude-then-rank (normative)

Given disclosure D and a set of PathEvidence values E:

1. Drop every E_i whose path class is prohibited for D.
2. Drop every E_i that is not locally validated or whose generation is
   not the live network generation.
3. Rank the remainder by payload capacity and RTT (or energy, if
   supplied by a later extension).

Prohibited classes:

| Disclosure | Prohibited classes |
|---|---|
| DirectPermitted | none |
| ApprovedRelaysOnly, QualifiedMultiHop | DirectV6, DirectV4 |
| Isolated | all live-network classes |

An implementation that selects DirectV6 under ApprovedRelaysOnly
because its RTT is lower is non-conformant even if both peers would
accept the packets.

## 6. Transport bindings

CSCP is carrier-independent.

- **QUIC:** one bidirectional client-initiated stream with ALPN
  `cscp/1` after the peer session that Qualia requires. QUIC path
  validation remains QUIC's; CSCP PathEvidence records the result.
- **MASQUE bound UDP:** the preferred Internet profile for two
  unreachable peers, when the proxy actually implements bound UDP.
  Ordinary HTTP/3 is not sufficient.
- **HTTP/2 capsules:** fallback when UDP is blocked. Nested recovery
  is degraded and MUST be labelled as such.
- **WireGuard / ICE / TURN / WSS:** compatibility / VPN profile.
- **Native QDNF:** independent of IP; CSCP MAY describe intent for a
  non-IP bearer without importing Internet locators.
- **Offline:** Receipt messages only; no live ConnectAccept.

Selecting a carrier MUST NOT enlarge permitted disclosures.

## 7. Security Considerations

CSCP inherits peer authentication from the session layer. A
self-signed transport certificate without identity binding is not a
CSCP peer.

Intent admission is not reachability. Contact descriptors are not
capability tokens for data. Relay leases are attributable commitments,
not availability guarantees. PathEvidence with RemoteAssertion is
untrusted for selection.

Early data MUST NOT carry ConnectRequest, Receipt with irreversible
effect, or ContactDescriptor for protected persons in the initial
profile.

Stale-but-signed descriptors from a compromised discovery provider are
an expected attack. Generation comparison in §4.3 is mandatory.

Resource exhaustion: implementations MUST bound candidate counts,
probe bytes, active leases, and queued operations, and MUST fail
closed.

## 8. Privacy Considerations

Default discovery is invitation-scoped. Public DHT/Pkarr publication
of a stable person identifier to an IP locator is NOT RECOMMENDED and
MUST NOT be the default for clinical or otherwise protected roles.

Relay-only mode MUST NOT export access-network addresses through
candidate TLVs, logs that leave the node, or fallback routines.
Relays still observe client source addresses, timing, and volume. Two
operators do not establish anonymity.

## 9. IANA Considerations

This document requests:

1. ALPN identification sequence `cscp/1`.
2. Media type `application/cscp` (binary, versioned as in §4).
3. A CSCP message-type registry (1–8 as in §4.1; 9–127 Specification
   Required; 128–255 private/experimental).

These registrations are not effective until IETF action. Shipping this
file in a repository does not create IANA entries.

## 10. Implementation status (not a protocol claim)

A bounded loopback implementation exists in QualiaDB
(`net/peer/fabric/`, branch `0.0.38`). It proves exclude-then-rank,
stale-descriptor rejection, lease expiry, grant-revoked replay, and
local bound-UDP forwarding. It is not Internet MASQUE, not an admitted
noq engine, and not RFC publication.

## 11. References

### 11.1. Normative

- [RFC2119] Bradner, S., "Key words for use in RFCs to Indicate
  Requirement Levels", BCP 14, RFC 2119, March 1997.
- [RFC8174] Leiba, B., "Ambiguity of Uppercase vs Lowercase in RFC
  2119 Key Words", BCP 14, RFC 8174, May 2017.

### 11.2. Informative

- [RFC8445] ICE
- [RFC8489] STUN
- [RFC8656] TURN
- [RFC9000] QUIC
- [RFC9221] QUIC DATAGRAM
- [RFC9297] HTTP Datagrams and the Capsule Protocol
- [RFC9298] Proxying UDP in HTTP
- draft-ietf-masque-connect-udp-listen (bound UDP)
- draft-ietf-quic-address-discovery
- Qualia research note: `quic-native-connectivity-research-2026.md`

## Authors' Addresses

Timothy Charles Holborn
WebCivics
Email: timothy.holborn@gmail.com

This revision is an individual Internet-Draft. It is not adopted by an
IETF working group. A working-group filename (draft-ietf-…) is assigned
only if a WG later adopts the document. The human-centric Internet
rationale may be discussed in IRTF venues (for example HRPC) without
that discussion being the protocol's IETF home.

## Acknowledgements

This draft incorporates the 2026 Qualia connectivity research
corrections: DCUtR retains relays; Pkarr is discovery not concealment;
Holepunch is not infrastructure-free.
