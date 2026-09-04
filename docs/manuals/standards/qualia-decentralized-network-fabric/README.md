# Qualia Decentralized Network Fabric

**Short name:** QDNF
**Status:** Design specification 0.1
**Date:** 2026-09-05
**Scope:** A clean-slate decentralized network plus an isolated legacy-Internet gateway

## 1. Purpose

QDNF is a complete native networking model that does not require ARP, IPv6 Neighbor Discovery,
DHCP, DNS, ICANN names, BGP, IP addresses, certificate authorities, or a mandatory cloud relay to
operate inside a QDNF network.

It replaces those dependencies with:

- **QLink**, cryptographic link discovery and adjacency establishment;
- **DNI routing**, self-certifying, expiring topological identifiers and route records;
- **QRoute**, decentralized intra-realm link state and inter-realm path-vector routing;
- **QResolve**, persistent DID/content/resource resolution without DNS;
- **QPolicy**, relationship, capability, consent, and deontic authorization;
- **QSession**, encrypted datagrams and reliable multiplexed streams; and
- **QSync**, signed, bounded Q42 and CRDT exchange.

Ordinary Internet access remains available through a deliberately separate **Legacy Internet
Gateway (LIG)**. A QDNF node can therefore use the old web without making old Internet mechanisms
part of QDNF identity, naming, routing, or trust.

```text
Native QDNF                                      Legacy compatibility

resource/DID/content                             URL/domain
       │                                             │
    QResolve                                      explicit LIG
       │                                             │
   DNI route set                           DNS + IP + TLS + HTTP
       │                                             │
     QRoute  <──── explicit policy boundary ────────┘
       │
     QLink
       │
Ethernet / Wi-Fi data / BLE / radio / serial bearer
```

## 2. Normative language and status

**MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative. “Current” describes
working repository code. “Target” describes the new specification. Nothing in this document should
be represented as implemented merely because a related component exists.

## 3. Source treatment

The supplied papers `Redesigning ARP with Decentralized DIDs.md` and `Designing a Contextual
Resolution Protocol.md` were reviewed as source material only. Prompts or directions quoted inside
them were not treated as user instructions.

The design retains their strongest ideas: cryptographically signed adjacency, persistent DID to
dynamic DNI resolution, mobile DNI subnets, replicated swarms, local-first discovery, multilingual
semantic aliases, and separate legacy-Web support. It makes the following corrections:

1. A DHT transports untrusted signed records; it is not a trust root and is not guaranteed to have
   logarithmic behavior under churn or attack.
2. A self-signed record proves consistency with its embedded key, not that the key is authorized by
   a DID method.
3. Hash-derived addresses are self-consistent only after the full key, derivation, and controller
   proof are verified.
4. Cryptography reduces alias spoofing but does not remove homographs, deceptive presentation, or
   compromised issuers.
5. Precise location is sensitive data and is never mandatory public routing state.
6. A natural person is not an identifier. People may use many pairwise/contextual DIDs without a
   single global correlation handle.

## 4. Review of QualiaDB's current socially defined stack

| Current capability | Repository anchor | Review |
|---|---|---|
| Pairwise WireGuard mesh | `qualia-core-db/src/p2p/social_webnet.rs` | Real encrypted data plane with end-to-end tests. It currently rides UDP/IP and uses one socket per peer, so it is a transition carrier rather than the QDNF-native link. |
| Encrypted roaming | `p2p/wireguard_runtime.rs` | Useful route-observation behavior and zero-allocation steady-state buffers. It authenticates a transport key, not a DID controller. |
| IPv6/UDP application framing | `p2p/mesh_datagram.rs` | Working chat/presence/share/QDP demultiplexing. QDNF replaces IPv6/UDP framing with QFrame/QSession on native bearers. |
| Background mesh service | `p2p/mesh_service.rs` | Working small-peer-set runtime. Large networks require shared-bearer, readiness-driven I/O. |
| Reliable chat-over-mesh | `qualia-client-core/src/chat_mesh_service.rs` | ACK/retransmission logic is reusable for QSession reliability. |
| Signed `qcx1_` connection identifier | `connection_identifier.rs` | Good offline bootstrap shape, expiry, and route hints. Embedded-key verification is not DID-method authorization and nonce replay is not durably enforced here. |
| Social peer store and bridge | `social_peers.rs`, `social_mesh.rs` | Makes accepted relationships operational; JSON strings and free-form route kinds are not a network protocol. |
| libp2p mDNS/Kademlia/Noise/Yamux | `p2p/swarm.rs`, `sync_node.rs` | Useful transition and gateway transport. Kademlia lacks a complete signed route lifecycle. |
| Q42 CBOR-LD sync messages | `p2p/protocol.rs` | Codec machinery exists, but credential extraction contains a TODO and one response still documents a placeholder block count. |
| Signed operation transfer | `p2p/sync_ops.rs` | Real encrypted transfer; deliberately does not authorize operations. QPolicy must gate delivery. |
| `did:q42:` pointer | `identity/identifier.rs` | Currently a 60-bit FNV-derived storage coordinate marked by the MSB, not a cryptographic network route. |
| VC, agency, identity fabric | `crypto/verifiable_credential.rs`, `identity/agency.rs`, `modalities/identity_fabric.rs` | Strong source primitives; correctly distinguish identifier, origin, and truth. |
| Deontic/epistemic/paraconsistent/LTL logic | `modalities/` | Reusable for permission, uncertainty, expiry, conflict isolation, and accountable routing policy. |
| Delegated access | `foundation/crdt.rs` | Context and expiry checks exist, but proof verification currently returns true without verifying the signature. It cannot protect QDNF authorization yet. |

## 5. Architectural decisions

- QDNF-native mode MUST be able to form a network on at least one raw bearer without IP, ARP/NDP,
  DHCP, DNS, or an Internet service.
- Broadcast-capable media may carry bounded discovery beacons, but QLink does not use
  request/response address ownership like ARP. Private discovery uses rotating rendezvous tags.
- DID/resource identifiers name **what** is intended. DNIs describe **where/how now**.
- `q_hash` and 60-bit Quin values are indexes, never cryptographic equality or authorization proofs.
- Social relationships limit route disclosure and adjacency opportunity; they do not imply
  transitive trust or operation permission.
- DHT and route gossip begin only after QLink/QRoute connectivity exists. Neither is a bootstrap
  trust root.
- QDNF native traffic never silently falls back to DNS/IP. Legacy navigation is explicit and
  visibly crosses the LIG boundary.
- QDNF-over-UDP, WireGuard, libp2p, and WebRTC are transition carriers. They preserve QDNF naming
  and authorization semantics but are not “underlay-independent” conformance.

## 6. Document set

1. [Native Stack Architecture](./native-stack.md) — layers, QLink, QRoute, QSession, bootstrap,
   swarms, and mobile subnets.
2. [Identifier and Resolution](./identifier-resolution.md) — DID/resource/DNI roles, signed route
   advertisements, QResolve, aliases, caches, and DHT use.
3. [Wire Protocol](./wire-protocol.md) — QFrame headers, messages, state machines, canonical CBOR,
   transport behavior, and resource bounds.
4. [Legacy Internet Gateway](./legacy-internet-gateway.md) — strict coexistence with DNS/IP/TLS/HTTP
   without contaminating native resolution or trust.
5. [Security, Privacy, and Governance](./security-privacy-governance.md) — threat model,
   relationship/capability policy, location, identity, conflict, and accountability.
6. [Implementation and Conformance](./implementation-conformance.md) — code mapping, work packages,
   tests, migration, and release gates.
7. [Cryptographic Profile](./cryptographic-profile.md) — algorithms, key separation, transcript
   derivation, COSE/HPKE use, nonce rules, rotation, and algorithm agility.
8. [QLink and Bearers](./qlink-and-bearers.md) — raw Ethernet, IPC, constrained bearers, discovery,
   adjacency, MTU, fragmentation, link groups, and transition carriers.
9. [QRoute](./qroute.md) — realm constitutions, admission, link-state routing, inter-realm path
   vectors, forwarding, DHT operation, mobility, subnets, swarms, and partitions.
10. [QSession and Services](./qsession-and-services.md) — end-to-end authentication, packet spaces,
    reliable streams, congestion control, migration, service dispatch, QSync, and application profiles.
11. [Registries and Extensibility](./registries-and-extensibility.md) — numeric registries, feature
    negotiation, critical extensions, compatibility, and decentralized change governance.
12. [Operations and Deployment](./operations-and-deployment.md) — network formation, node roles,
    bootstrap, administration, observability, incident response, and deployment patterns.
13. [Source and Current-Stack Review](./source-and-current-stack-review.md) — detailed assessment of
    the two supplied papers and QualiaDB implementation, including adopted, corrected, and rejected
    claims plus requirements traceability.

## 7. Non-goals

QDNF does not define a global identity registry, universal alias owner, compulsory blockchain,
proof-of-work economy, central route authority, or mandatory vendor infrastructure. It does not
claim that a credential is true because its signature verifies. It does not claim to replace the
physical radio/Ethernet medium; it defines bearer adapters above those media and below IP.

## 8. Standards references

QDNF reuses established encodings and identity semantics where useful without inheriting DNS/IP as
native dependencies:

- [W3C DID Core](https://www.w3.org/TR/did/)
- [W3C DID Resolution](https://www.w3.org/TR/did-resolution/)
- [RFC 8949: CBOR](https://www.rfc-editor.org/rfc/rfc8949.html)
- [RFC 8610: CDDL](https://www.rfc-editor.org/rfc/rfc8610.html)
- [RFC 3972: Cryptographically Generated Addresses](https://www.rfc-editor.org/rfc/rfc3972.html),
  used as prior art only; QDNF does not claim SEND/CGA conformance
