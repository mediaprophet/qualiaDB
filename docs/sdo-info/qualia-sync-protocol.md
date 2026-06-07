# Qualia Sync Protocol Draft

**Status:** Internal draft  
**Date:** 2026-06-08  
**Purpose:** Define the current implementation reality and intended
standardization path for the Qualia sync protocol within the Qualia Protocol
Ecosystem.

## 1. Role

The Qualia sync protocol is the peer-to-peer graph synchronization layer that
sits between:

- the `q42` storage and artifact family
- identifier-bearing peer coordination
- higher-level Webizen governance and agency logic

Its job is not to define human identity, vault meaning, or qapp launch
surfaces. Its job is to define how peers exchange enough information to decide
whether sync should proceed and, if so, how graph or block-oriented state moves
between them.

## 2. Current Implementation Reality

The codebase currently implements a concrete sync path with the following
elements:

1. libp2p transport stack over TCP + Noise + Yamux
2. mDNS peer discovery
3. Kademlia behavior attached to the swarm
4. libp2p request-response protocol using a fixed stream protocol id
5. CBOR-framed payload encoding with a 4-byte big-endian length prefix
6. a two-message request family: `Handshake` and `Sync`

Important distinction:

- the current codec implementation uses generic CBOR serialization through
  `ciborium`
- the broader architectural direction in this ecosystem is for semantically
  rich exchange payloads to converge on CBOR-LD profiles where appropriate
- this draft therefore distinguishes current wire framing from intended semantic
  payload conventions

Relevant implementation anchors:

- [crates/qualia-core-db/src/p2p/swarm.rs](/C:/Projects/qualiaDB/crates/qualia-core-db/src/p2p/swarm.rs:1)
- [crates/qualia-core-db/src/p2p/protocol.rs](/C:/Projects/qualiaDB/crates/qualia-core-db/src/p2p/protocol.rs:1)
- [crates/qualia-core-db/src/p2p/routing.rs](/C:/Projects/qualiaDB/crates/qualia-core-db/src/p2p/routing.rs:1)
- [crates/qualia-core-db/src/daemon.rs](/C:/Projects/qualiaDB/crates/qualia-core-db/src/daemon.rs:1020)

## 3. Protocol Identifier

The currently implemented stream protocol id is:

```text
/qualia/crdt-sync/1.0.0
```

This should be treated as the current protocol identifier for the internal
draft, even if the final externalized name changes later.

## 4. Current Network Stack

The currently implemented daemon path uses:

- TCP transport
- Noise security
- Yamux multiplexing
- libp2p request-response

It also attaches:

- mDNS for local discovery
- Kademlia behavior for DHT-oriented participation

This is important because some broader repo prose talks about WebRTC mesh sync,
Automerge-style gossip, or other future-facing coordination layers. The
implemented sync path documented here is narrower and more concrete.

## 5. Current Framing

Each request and response is framed as:

```text
4-byte big-endian length
+ CBOR-encoded payload
```

The current codec in `p2p/protocol.rs` serializes both requests and responses
with `ciborium`.

This means the current wire contract is not raw Quins on the stream. It is
length-prefixed CBOR messages whose fields may themselves contain flattened
binary Quin material.

For this draft, that should be read carefully:

- **current implementation reality**: CBOR framing and CBOR message bodies
- **intended semantic direction**: CBOR-LD-aligned payload profiles where the
  exchange is carrying linked-data meaning rather than merely local binary
  buffers

## 6. Current Request Grammar

The currently implemented request grammar is:

```text
QualiaRequest =
  Handshake { compressed_vcs: bytes }
  | Sync { hop_count: u8, gatekeeper_token: optional string, target_shapes: string[] }
```

The currently implemented response grammar is:

```text
QualiaResponse =
  HandshakeAck { success: bool }
  | SyncAck { success: bool, message: string, blocks_sent: u64 }
```

## 7. Handshake Message

### Purpose

`Handshake` is the trust-establishment step that allows a receiving peer to
decide whether the requesting peer is authorized to participate in the current
route.

### Current Payload Shape

The payload is a flattened byte buffer:

```text
compressed_vcs = repeated (48-byte Quin + 64-byte Ed25519 signature)
```

The current daemon logic iterates over this buffer in 112-byte slices.

### Current Validation Logic

The daemon currently:

1. iterates through the flattened credential entries
2. reinterprets the first 48 bytes of each slice as a protocol-local Quin
3. reads the next 64 bytes as an Ed25519 signature
4. checks for a trusted-group relationship through the routing table
5. approves or rejects the route

If no qualifying trusted-group proof is found:

- the handshake is rejected
- the peer is disconnected

If a qualifying proof is found:

- the handshake is approved
- trust is upgraded for that peer session

## 8. Sync Message

### Purpose

`Sync` is the selective synchronization request.

It is currently intended to express:

- trust horizon
- whether a gatekeeper challenge token is present
- which graph shapes are being requested

### Current Fields

- `hop_count`
- `gatekeeper_token`
- `target_shapes`

### Current Authorization Logic

The daemon currently applies three visible rules:

1. `hop_count > 2` is rejected under the current two-hop trust horizon
2. a present `gatekeeper_token` authorizes the request
3. without a gatekeeper token, `foaf:Person` in `target_shapes` is currently
   treated as enough to authorize the request

This yields one of two current responses:

- `SyncAck { success: true, message: "Sync Approved", blocks_sent: 42 }`
- `SyncAck { success: false, message: "RequiresGatekeeperChallenge", blocks_sent: 0 }`

The value `42` is a current implementation placeholder rather than a fully
specified transfer accounting rule.

## 9. Relationship To Shapes and Qapps

The current developer docs already tie qapp shape requirements to the sync
layer.

Specifically:

- `qapp.json` `required_shapes`
- map to sync `target_shapes`
- and are intended to constrain which graph data the daemon grants access to

Relevant anchor:

- [docs/manuals/developing-qapps.md](/C:/Projects/qualiaDB/docs/manuals/developing-qapps.md:107)

This means the sync protocol is not only transport. It is also one of the
places where graph-shape scoping is enforced.

## 10. Relationship To CRDT Semantics

The broader architecture describes this layer as CRDT-oriented sync, and the
repo contains a deterministic last-write-wins resolver in `crdt.rs`.

Current local CRDT semantics include:

- Lamport-clock comparison
- deterministic object-value tie-breaking
- delegated-access verification helpers

Relevant anchor:

- [crates/qualia-core-db/src/crdt.rs](/C:/Projects/qualiaDB/crates/qualia-core-db/src/crdt.rs:1)

However, the current network protocol draft does not yet transmit a full CRDT
operation grammar. It only exposes enough handshake and sync request structure
to gate or approve a sync path.

So the current sync protocol should be described as:

- CRDT-aware
- shape-scoped
- trust-gated

But not yet as a complete replicated-operation protocol.

## 11. Relationship To Adjacent Exchange Surfaces

The repo contains several adjacent exchange surfaces that should not be
collapsed into this protocol draft.

They are related, but distinct:

- daemon chat relay over loopback HTTP
- WebTorrent `.c.q42` web-seeding
- qapp loopback serving
- future WebRTC or GUN-oriented coordination language elsewhere in the repo

This draft is specifically about the libp2p sync path currently implemented in
the daemon.

It is also adjacent to a lower-stack enrichment direction that is not yet
implemented as part of this sync grammar:

- WireGuard or similar encrypted tunnel profiles
- TLS certificate SAN carriage for CBOR-LD-linked identifier material
- IPv6-layer semantic enrichment of identifier and route metadata

Those ideas belong in the broader network architecture, but should not be
misstated as already-frozen parts of the current libp2p sync wire format.

## 12. Conformance Targets

The internal draft should define at least these conformance targets.

### Sync Codec

Must:

- use the current stream protocol id
- frame messages with a 4-byte big-endian length prefix
- encode the currently implemented payloads as CBOR
- decode both requests and responses deterministically

Should:

- clearly declare when a payload profile is merely CBOR-framed binary transport
  versus when it is a CBOR-LD semantic payload

### Sync Peer

Must:

- support `Handshake`
- support `Sync`
- emit `HandshakeAck`
- emit `SyncAck`

### Handshake Gate

Must:

- reject malformed or unauthorized credential sequences
- reject unauthorized group routes
- approve authorized group routes

### Sync Gate

Must:

- enforce the current hop-count rule
- process gatekeeper-token presence
- process shape-based request scoping

## 13. Contradictions To Resolve Before Externalization

The current repo story is not yet clean enough for external standardization.

Key contradictions and unfinished areas:

1. The architecture prose often emphasizes WebRTC mesh behavior, while the
   implemented protocol here is libp2p over TCP.
2. The protocol name says `crdt-sync`, but the wire grammar does not yet define
   a complete CRDT operation set.
3. The handshake field name `compressed_vcs` implies compressed verifiable
   credentials, but the actual structure is a flattened binary buffer of Quin
   and signature slices.
4. The request types use heap-backed `Vec` and `String`, which does not align
   with the zero-heap mandate used elsewhere in hot execution paths.
5. `SyncAck.blocks_sent` is not yet tied to a stable block-transfer grammar.
6. The trust logic still includes obvious placeholder conditions such as
   `foaf:Person` authorization and fixed `42` block counts.
7. The code currently uses CBOR serialization, while broader architecture prose
   often speaks in CBOR-LD terms without yet freezing the exact payload
   profile boundary.

## 14. Proposed Direction

This draft recommends the following sequence:

1. Freeze the current handshake and sync message grammar as an internal v0.
2. Separate trust-establishment semantics from actual block or delta transfer
   semantics.
3. Define whether the next normative payload layer is:
   - block-oriented `.q42` / `.c.q42` transfer
   - CRDT operation transfer
   - or a mixed model with explicit message types for each
4. Replace ambiguous names like `compressed_vcs` with field names that match
   the actual payload shape.
5. Decide whether libp2p/TCP remains the primary transport profile or whether
   WebRTC becomes a second formally specified profile.
6. Define where CBOR ends and CBOR-LD begins in the protocol stack:
   - plain CBOR framing for transport envelopes
   - CBOR-LD for semantic payload profiles
   - or CBOR-LD throughout once the linked-data profile is frozen

## 15. Open Questions

1. Should `Handshake` remain binary-buffer based, or should it become a typed
   credential list structure?
2. Should `Sync` request graph shapes only, or should it also include explicit
   block ranges, content hashes, or artifact ids?
3. Should gatekeeper tokens be opaque bearer material, typed capability
   envelopes, or references to separate `.qchk` material?
4. What is the canonical relationship between this sync protocol and WebTorrent
   delivery of `.c.q42` artifacts?
5. Should the current libp2p/TCP path and a future WebRTC path be separate
   transport profiles under one protocol, or different protocols entirely?
6. Should lower-network profiles such as WireGuard, TLS SAN binding, or IPv6
   semantic enrichment carry CBOR-LD-linked identifier material, and if so, at
   which layer should that be specified?

## 16. Immediate Next Steps

1. Freeze the v0 message grammar in prose with examples.
2. Decide whether `compressed_vcs` is renamed before wider adoption.
3. Define a real transfer grammar for `blocks_sent`.
4. Decide whether CRDT operations, artifact blocks, and trust credentials live
   in one protocol or in clearly separated message families.
5. Write down the intended CBOR-LD payload profile boundary so current CBOR
   framing is not mistaken for the full semantic serialization model.
