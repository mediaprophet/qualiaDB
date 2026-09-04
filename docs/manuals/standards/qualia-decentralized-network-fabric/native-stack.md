# QDNF Native Stack Architecture

**Status:** Normative design 0.1

## 1. Native independence

A QDNF-native deployment forms, names, routes, secures, and resolves its own traffic without an IP
configuration or DNS resolver. A disconnected Ethernet switch, a Wi-Fi interface that exposes raw
data frames, BLE mesh, or a radio/serial bearer can carry a complete QDNF network.

The stack is:

| Layer | Name | Replaces native dependency on | Responsibility |
|---|---|---|---|
| Q0 | Bearer adapter | IP socket as lowest common denominator | Send/receive bounded frames on Ethernet, Wi-Fi data, BLE, radio, serial, shared memory, or another medium |
| Q1 | QLink | ARP, NDP trust, DHCP link configuration | Cryptographic neighbor discovery, adjacency, MTU, bearer locator observation, group rendezvous |
| Q2 | QRoute | IP addressing, subnet allocation, BGP | DNI realms, route exchange, forwarding, hop limits, multipath, gateway delegation |
| Q3 | QResolve | DNS and host files | Persistent DID/content/resource to signed DNI route sets and semantic aliases |
| Q4 | QPolicy | Network-location trust, ambient LAN trust | Relationship disclosure, capability, consent, deontic decisions, sensitivity lanes |
| Q5 | QSession | TCP/UDP/TLS as mandatory stack | Encrypted datagrams, reliable ordered streams, multiplexing, congestion/backpressure |
| Q6 | QSync/Application | Server-centric application endpoints | Q42 blocks, signed CRDT ops, chat, presence, sharing, qapps, content swarms |

The functional replacements are explicit:

| Old dependency | QDNF-native replacement |
|---|---|
| ARP / NDP neighbor mapping | QLink beacon, nonce challenge, ephemeral proof, and observed bearer-locator binding |
| DHCP / SLAAC address assignment | Self-created epoch link/node IDs plus signed realm/subnet admission |
| IPv4 / IPv6 host addressing | `network_id / realm_id / node_id / service_id` DNI coordinate |
| ICMP/router discovery | Authenticated QLink status and signed QRoute realm advertisements |
| BGP/default-route authority | Expiring signed inter-realm path vectors with local policy and no compulsory default route |
| DNS / search suffixes | QResolve records and contextual multilingual Alias Assertions |
| TCP / UDP / TLS as mandatory session stack | QSession encrypted datagrams, reliable streams, multiplexing, and rekey |
| Web PKI as universal authority | Target-specific DID/resource controller proof and local trust/capability policy |
| NAT traversal as addressing model | Direct QRoute, explicit realm gateway, or consented relay; no global address scarcity |

## 2. Planes and invariant

```text
Persistent target ──QResolve──> verified DNI candidates
                                      │
Relationship ───────QPolicy───────────┤ disclosure and permission
                                      │
Bearer <──QLink adjacency──> QRoute path ──QSession──> scoped application
```

The following facts are independent:

- a neighbor is physically reachable;
- a transport/session key is possessed;
- a key is authorized by a DID controller;
- a social relationship exists;
- a route may be disclosed in that relationship; and
- a particular read/write/action is authorized.

No layer may collapse those facts into one “trusted peer” boolean.

## 3. Q0 bearer adapters

A bearer adapter supplies:

```text
send(destination_bearer_locator | group_locator, frame_bytes)
receive() -> (observed_source_locator, frame_bytes, link_metadata)
mtu() -> bytes
scope_id() -> stable-during-attachment local value
```

The observed bearer locator—such as an Ethernet MAC—is a forwarding hint, not identity. It is not
signed by the sender because the receiver must bind what it actually observed.

Required initial adapters:

- `raw-ethernet-v1`: QFrames in a project-development EtherType; production requires an assigned
  EtherType or an explicitly administered encapsulation;
- `local-ipc-v1`: shared-memory or local socket carrier for same-device nodes; and
- `udp-transition-v1`: QFrames tunneled over UDP for deployment on the old Internet.

Recommended later adapters are BLE L2CAP, Wi-Fi action/data frames, WebRTC data channels, LoRa-class
constrained frames, and acoustic/serial links. `udp-transition-v1` depends operationally on the old
stack and therefore cannot satisfy Native Independent conformance by itself.

## 4. QLink: cryptographic neighbor discovery

### 4.1 Link identity

On each bearer attachment and privacy epoch, a node generates an ephemeral QLink key pair. Its link
identifier is:

```text
link_id = first_128_bits(SHA-256(
  "qdnf:link:v1" || public_key || bearer_scope || epoch
))
```

The full digest and public key are verified during adjacency; the 128-bit value is only the compact
frame selector. A new bearer, epoch, or key creates a new link ID.

### 4.2 Discovery modes

| Mode | Beacon selector | Use |
|---|---|---|
| `private-pairwise` | rotating HMAC tag from a shared discovery secret | default person-to-person or sensitive relation |
| `closed-group` | group epoch rendezvous tag | cooperative, household, channel, or project |
| `public-service` | public service-class hash and ephemeral link ID | explicitly public infrastructure/service |
| `manual` | scanned/transferred invitation | no beacon required |

Private beacons never carry a stable DID, display name, precise location, or reusable route ID.

### 4.3 Adjacency handshake

```text
Undiscovered
  -> BeaconSeen
  -> ChallengeSent/Received
  -> EphemeralProofVerified
  -> RelationshipDisclosureChecked
  -> LinkKeysDerived
  -> Adjacent
  -> Rekeying
  -> Closed
```

The proof binds both observed link locators, both ephemeral keys, both nonces, bearer scope, epoch,
negotiated MTU, and transcript digest. QLink uses ephemeral Diffie-Hellman plus authenticated proofs.
Private modes also prove knowledge of the rendezvous secret without exposing it.

The neighbor table maps `link_id -> observed locator, keys, MTU, expiry, policy handle`. It has no IP
or hostname fields. Entries expire quickly and never establish application authorization.

### 4.4 Discovery traffic

QLink may use a bounded multicast/group beacon on a shared medium. This is not ARP: it does not ask
who owns a globally meaningful address, accept an unauthenticated address reply, or populate an IP
neighbor cache. Private beacons are unlinkable across epochs, rate-limited, and ignored unless they
match a local rendezvous policy.

## 5. QRoute: DNI-native routing

### 5.1 Topological coordinates

A routable DNI route entry contains:

- `network_id`: 128-bit digest of the network constitution/genesis record;
- `realm_id`: 128-bit self-certifying local routing realm;
- `node_id`: 128-bit epoch-scoped node route identifier;
- `service_id`: 64-bit Q42 service index plus a full service reference in the signed record;
- `route_epoch`, validity, and route proof;
- one or more next-hop/path candidates; and
- a strong digest (`dni_id`) over the complete canonical route entry.

`network_id`, `realm_id`, and `node_id` are not assigned by a registry. Full signed records and
strong digests resolve compact-field collisions.

### 5.2 Realms and DNI subnets

A realm is a local routing scope with a signed constitution defining admission, metric policy,
maximum size, and gateway rules. It may represent a direct mesh, vehicle, home, cooperative, public
venue, clinic, or community resilience network.

Nodes self-create epoch-scoped node IDs. Gateways receive signed Subnet Delegation Records. The
gateway routes for child resources but does not own them and cannot widen their QPolicy grants.
Moving a realm changes its macro path advertisement; internal node IDs may remain stable until the
realm epoch rotates. No DHCP lease or IP prefix is required.

### 5.3 Intra-realm routing

The base profile uses bounded signed link-state advertisements (LSAs):

- origin realm/node ID and authorized routing key;
- adjacent link IDs and locally measured metric class;
- sequence, expiry, capability flags, and signature;
- maximum 32 advertised adjacencies per LSA; and
- flooding only across authenticated QLink adjacencies permitted to carry routing control.

Each router computes deterministic shortest paths from its verified local database. Remote latency
or priority assertions are hints; local observations dominate. Contradictory same-sequence LSAs are
quarantined, not silently merged.

### 5.4 Inter-realm routing

Gateways exchange signed, expiring path vectors:

- destination `network_id/realm_id`;
- ordered realm path, maximum 16 hops;
- gateway DNI and next-hop adjacency;
- policy/sensitivity ceiling and supported service classes;
- sequence, expiry, path digest, and gateway signature.

A gateway rejects its own realm in the path, repeated realms, unauthorized export, excessive hops,
and a widened policy ceiling. Multiple independent paths may coexist. There is no globally privileged
autonomous-system registry or mandatory default route.

### 5.5 Forwarding

QRoute forwards by destination network/realm/node tuple and a locally computed next-hop link ID.
Every packet has a hop limit and flow identifier. Routers validate the adjacent link authenticator,
decrement the hop limit, apply forwarding policy, and emit the packet on the next bearer. End-to-end
QSession encryption prevents routers from reading application content.

Routers do not resolve human names in the forwarding path.

## 6. QResolve

QResolve runs after at least one QRoute adjacency or from local storage. It maps a persistent DID,
DID URL, content digest, or canonical resource IRI to signed DNI route sets. It uses:

1. pinned/current session state;
2. encrypted relationship records;
3. local realm indexes;
4. authorized community directories/introducers;
5. a DHT carried over QRoute; and
6. an explicit Legacy Internet Gateway request.

The DHT cannot bootstrap a disconnected node by itself. Bootstrap comes from local discovery,
manual invitation, previously known relationship peers, or a configured community bearer entry.

## 7. QPolicy

The authorization order is:

```text
block -> route disclosure -> adjacency -> DID/session proof -> capability/consent
      -> sensitivity lane -> deontic decision -> application delivery
```

- Relationships are directional and non-transitive by default.
- Route visibility is separate from data access.
- Sanctuary/selfhood material never enters ordinary sync or auto-merge paths.
- Ambiguous policy produces an interactive decision, not implied permission.
- M-of-N governance uses the bounded suspended transaction mechanism.
- Until delegated-access signatures are truly verified, those grants are rejected at QDNF boundaries.

## 8. QSession

QSession provides the native equivalents of datagrams, reliable streams, and multiplexing without
requiring UDP, TCP, or TLS.

- Handshake binds the persistent target, selected DNI, QLink/QRoute transcript, ephemeral
  key-agreement result, and DID-authorized session key.
- Datagram channels are unordered and optionally acknowledged.
- Reliable streams use byte offsets, selective acknowledgements, bounded retransmission, flow
  control, and pluggable congestion policy.
- Service IDs replace well-known IP ports. The existing chat/presence/share/QDP port meanings map to
  registered QDNF service references.
- Rekeying rotates traffic keys without changing the application resource identifier.
- Route migration rebinds the live session to a newly verified DNI without restarting the semantic
  operation when replay policy permits.

## 9. QSync and content swarms

QSync carries signed operations and content blocks after QPolicy authorization. Immutable content
may be fetched in parallel from a DNI swarm when each block verifies against the strong artifact
digest. Mutable replicas require controller replica authorization and signed operation validation;
matching a target identifier is insufficient.

Offline queues contain signed, expiring operations in bounded encrypted storage. Delivery never
implies application. The recipient repeats replay, signature, capability, sensitivity, and deontic
checks.

## 10. Multilingual discovery

Names live above routing as signed Alias Assertions:

```text
(UTF-8 label, language, script, community context, issuer,
 persistent target, validity, provenance, proof)
```

There is no global alias winner or TLD. Personal, relationship, community, institutional, and legacy
names remain distinguishable. Exact identifiers bypass ranking. Ambiguous or confusable names require
selection. Spatial search is opt-in and coarse by default; requester coordinates are not routing keys.

## 11. Native versus transition conformance

| Class | Lowest carrier | Old-stack dependency |
|---|---|---|
| `QDNF-Native-Independent` | raw Ethernet, BLE, radio, serial, or local IPC | none for QDNF operation |
| `QDNF-Native-Gateway` | native bearer plus LIG | only gateway's legacy side |
| `QDNF-Transition` | UDP, WireGuard, libp2p/TCP, WebRTC | carrier depends on IP and possibly legacy discovery |

A transition node may be fully compliant with QResolve/QPolicy/QSession semantics, but documentation
must disclose that its carrier still depends on the old stack.
