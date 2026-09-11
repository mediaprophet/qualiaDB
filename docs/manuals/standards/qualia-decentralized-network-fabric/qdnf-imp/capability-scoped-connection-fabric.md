# Capability-scoped connection fabric

Date: 2026-09-10. Status: **CSCP `-00` Internet-Draft plus in-tree supervisor**.
Local loopback bound-UDP, CSCP ConnectRequest codec, and offline receipts are
implemented on `0.0.38`. This is **not** an RFC, **not** IETF datatracker
publication, **not** Internet MASQUE, **not** an admitted noq/QUIC engine,
**not** a public relay, and **not** a deployment claim.

Related: [draft-webcivics-cscp-00.md](../../../../standards/ietf/draft-webcivics-cscp-00.md) (proposed
Internet-Draft), [quic-native-connectivity-research-2026.md](./quic-native-connectivity-research-2026.md)
(32-source report), [internet-peer-connectivity-architecture.md](./internet-peer-connectivity-architecture.md)
(incremental SocialWebNet / WireGuard A–E), [nat-traversal-expert-brief.md](./nat-traversal-expert-brief.md).

## 0. Novel protocols and IETF

Novel protocols **may be defined**. When Qualia defines one, it is written as an
Internet-Draft intended for IETF progression toward RFC **only if** it is a
state-of-the-art, novel, and transformationally better solution for a
human-centric Internet. Shipping a `-00` in this tree does not publish it on
the datatracker and does not make it an RFC.

CSCP is that protocol. The transformational claim is the control plane:
`Connect(peer, purpose, protection_policy, resource_budget)` with
**exclude-then-rank** as a conformance requirement. A faster path that would
disclose an endpoint IP against policy is a protocol error, not an
optimization. CSCP does not define new cryptography and does not replace QUIC.

This document is the architecture note. Normative MUST/MUST NOT text lives in
the Internet-Draft.

## 1. What this is, and what it is not

Applications request:

```text
connect(peer, purpose, protection_policy, resource_budget)
```

The fabric—not the application—then admits who may communicate, locates a
private contact route, reserves bounded intermediary capacity, records what
the transport actually validated, and preserves delivery across outage.

| This fabric does | This fabric does not |
|---|---|
| Exclude prohibited paths **before** scoring performance | Let a faster direct path override “do not disclose this IP” |
| Treat DCUtR, Pkarr and Holepunch as comparators with documented limits | Claim they eliminate relays, conceal publication, or remove infrastructure |
| Use invitation-scoped contact descriptors by default | Publish identity-to-IP in a public DHT for protected persons |
| Separate **relay leases** from **custody leases** | Conflate forwarding bytes with stored-envelope retention |
| Hide `SessionReady` behind verified transport + authority events | Expose `advance(SessionReady)` as a public hatch |
| Keep native QDNF independent of IP | Make WireGuard or QUIC mandatory under every connection |
| Prefer QUIC + MASQUE bound UDP as the **greenfield Internet profile** | Implement RFC 9000 or claim every HTTP/3 server is a bound-UDP proxy |

Decentralization changes **who supplies** reachable machines, bandwidth and
retained data. It does not remove those needs. NAT physics are unchanged.

## 1.1 Ownership

| Owner | Responsibility | Must not imply |
|---|---|---|
| Qualia authority / QSession | Peer identity, grants, purpose, cryptographic profile, operation semantics | A socket, CID, or TLS session grants application authority |
| Connection fabric (this spec) | Admit intents; resolve private contact; exclude then select; leases; evidence; receipts | It can manufacture successful transport events |
| Carrier engines | Packet/path validation, recovery, congestion on **one** admitted carrier | Every peer is authorized for every service |
| Rendezvous | Expiring, authenticated invitations and opaque mailboxes | A public directory of protected persons |
| Core durable store | Sealed operations, idempotency, expiry, retry, receipts | A transport ACK is an application commit |

The incremental WireGuard / ICE / TURN / WSS path remains a **labelled
transition carrier**. The greenfield Internet profile is QUIC-native, IPv6
preferred with prompt IPv4, relay-assisted establishment, then **permitted**
direct upgrades. WireGuard stays useful for VPN / IP-interface compatibility.
It is not mandatory underneath every new connection. Public DHT discovery is
optional and off by default for protected contact.

## 2. Logical records

These are logical records, not frozen Rust layouts and not new ciphers.
In-tree types live in `crates/qualia-core-db/src/net/peer/fabric/`. Large
proofs stay in bounded core-owned references. The 48-byte `NQuin` ABI is not
a certificate container.

### 2.1 Connection intent

Binds intended peer, purpose IRI hash, authority reference, disclosure
constraints, deadline and resource budget. Does **not** prove the peer is
online.

### 2.2 Private contact descriptor

Binds a relationship-scoped contact key, generation, expiry, and either an
opaque mailbox or a permitted locator. Signing an address record does not
conceal it. A valid signature on an old generation is **stale**, not current.
Does **not** prove a locator works or is safe to probe.

### 2.3 Relay lease

Reserves explicitly bounded intermediary capacity: operator, participants,
byte and time limits, generation, cancellation. Signatures make commitments
**attributable**, not physically guaranteed. Does **not** prove the operator
cannot fail or observe metadata.

### 2.4 Custody lease

Separate from a relay lease. Retention, deletion and receipt of stored
encrypted operations. A provider may offer both roles; the semantics must
not be mixed.

### 2.5 Path evidence

Local state produced by a trusted transport integration: path class, network
generation, freshness, validated payload capacity, observed performance,
observer kind (`LocalTransport` vs `RemoteAssertion`). A peer’s signed claim
that a path works remains an assertion. Constructors for `validated = true`
are not public.

### 2.6 Durable operation and receipt

Operation identity, content digest, authority context, expiry, encrypted
payload reference. A receipt states received / validated / durably committed
/ denied / expired. Duplicate identifier with different content fails closed.
Replaying after grant revocation cannot commit.

## 3. Supervisor rule: exclude, then score

```text
admit intent
  → resolve private descriptor (invitation / mailbox; DHT only if policy allows)
    → exclude paths that violate disclosure, crypto, operator, authority or budget
      → among remaining, score validation, freshness, payload, RTT/energy
        → establish relay promptly when allowed
          → upgrade to direct only if still permitted and locally validated
            → else queue sealed operations and report unavailable
```

No weighted score may trade a forbidden IP disclosure for a faster response.
The kernel is a bounded transition function over supplied events, a time
snapshot and caller-owned state. The network is nondeterministic; the kernel’s
response to the same admitted inputs is reproducible. Network-generation
changes, policy changes and timers expire evidence. Cancellation revokes
pending work as well as current paths.

### 3.1 Operating profiles

| Profile | Permitted network activity | Failure |
|---|---|---|
| Direct permitted | Approved discovery and peer address disclosure; IPv6 then prompt IPv4; relays | First qualified path within policy |
| Relay only | Approved proxy/relay access; **no** direct candidate export, probing or automatic upgrade | Alternate approved relay, then queue |
| Isolated / disconnected | Admitted local or deferred bearers only | Sealed durable operations; no unsolicited Internet discovery |

Relay-only still discloses the client’s source address to the access relay.
Two operators are not anonymity.

## 4. Preferred greenfield Internet profile

Qualification targets, not a claim that the tree already speaks them:

1. QUIC transport, IPv6 preferred, prompt IPv4 alternatives.
2. Relay-assisted establishment, then **permitted** direct-path upgrades.
3. Encrypted address discovery and negotiated QUIC traversal extensions,
   version-gated; not assumed universal.
4. MASQUE **bound UDP** for two unreachable peers, with tested HTTP/2/TCP
   capsule fallback. Bound UDP is work in progress—not something every
   HTTP/3 server provides.
5. Separate browser (WebTransport / WSS) and offline-delivery profiles.

Evaluate **noq** as a transport engine behind this fabric. Do not replace
Qualia authority, resolution or storage with iroh’s application model. Iroh’s
default discovery is hosted DNS/Pkarr; Mainline DHT is optional. Pkarr is
discovery, not traversal or private discovery.

In-tree today: a Qualia-owned **bound-UDP topology** on loopback that enforces
leases and the exclude-then-score rule. That proves the fabric. It is not
`CONNECT-UDP`, not HTTP/3, and not a public MASQUE deployment.

## 5. Carriers (interchangeable)

Adding a carrier must not enlarge permitted disclosures.

| Carrier | Role |
|---|---|
| Native QDNF | Independent of IP; not this Internet profile |
| Bound UDP / MASQUE (greenfield) | Two outbound clients, public bindings, end-to-end peer transport |
| QUIC direct (when permitted) | After relayed useful traffic; only with local path validation |
| HTTP/2 capsule fallback | When UDP is blocked; degraded nested recovery; labelled honestly |
| WireGuard + ICE/TURN/WSS | Incremental SocialWebNet / VPN compatibility |
| Browser WebTransport / WSS | Distinct profile; not native UDP NAT probing |
| Offline durable store | Above all carriers; survives restart |

## 6. Qualification gates (not a schedule)

From the research report §8. Do not wait for a speculative traversal RFC to
obtain useful **relay** connectivity. Do not make a young traversal extension
mandatory for every protected session.

| Gate | Deliverable | Evidence before proceeding |
|---|---|---|
| A | Versioned QSession-to-carrier mapping and resource/threat contracts | No duplicated recovery owner; native QDNF and authorization intact |
| B | Authenticated peer transport over real H3 and H2/capsule proxy paths | Two machines, restrictive NATs, genuine certificates |
| C | Same-socket discovery and negotiated traversal | Captures, role tests, payload validation, linkability and privacy-mode negatives |
| D | Family changes, relay failure, expiry, resume, mobility | No unauthorized delivery; bounded recovery; crash/restart |
| E | Dependency review, interop, fuzz, resource and field tests | Reproducible artifacts; independent review of security-sensitive boundaries |

Local fabric tests on this branch satisfy a **kernel and lease** slice of
A and D on loopback. They do not satisfy B, C or E on the public Internet.

## 7. Four falsifiable experiments

The supervisor must preserve authority and disclosure in every case:

1. **Compromised discovery** returns a validly signed but **stale** descriptor.
   No probe of its locator.
2. **Relay lease expires** mid-transfer. Forwarding stops; remainder queues;
   the lease is not silently enlarged.
3. **Network generation changes** while direct probing is forbidden. Direct
   probe count stays zero; work continues on an approved relay or goes offline.
4. **Offline mutation replayed after grant revocation.** Receipt is denied;
   the operation is not committed.

## 8. Outstanding human decisions

Named operators and live URLs; supported proxy capabilities (bound UDP vs
stream relay); key custody and minimum cryptographic profiles; admissible
disclosure per deployment; exact QUIC engine revision (noq evaluation) and
memory budget; browser requirements; availability objectives.

Until those are qualified, the honest result is a specified architecture with
a working local supervisor—not field readiness.

## 9. In-tree map

| Path | Owner |
|---|---|
| `net/peer/fabric/` | CSCP `-00`: intent, contact, leases, evidence, receipts, kernel, selection, wire codec, local bound-UDP |
| `net/peer/connectivity/` | Incremental WG connection-manager contract (policy, ICE planner, durable store) |
| `p2p/connectivity/` | TLS WSS, ICE/TURN local proofs, QSession production path |
| `net/qdnf/` | Native bearer, QSession, authority |

Honesty flags in `net/peer/connectivity/evidence.rs` remain false for public
relay, Internet two-host, browser TURN, QUIC/iroh benchmark, MASQUE Internet
and noq admission.
