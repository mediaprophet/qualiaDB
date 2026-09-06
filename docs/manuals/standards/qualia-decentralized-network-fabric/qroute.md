# QRoute Routing Specification

**Status:** Normative design 0.1

## 1. Purpose

QRoute replaces native dependence on IP addressing, IP subnets, router advertisements, default
gateways, and BGP. It routes QFrames across authenticated QLink adjacencies using decentralized,
self-created routing realms and signed topological state.

QRoute does not resolve human names, authorize application access, or expose plaintext QSession
payloads to routers.

## 2. Routing identifiers

### 2.1 Network

A QDNF network is a set of realms that accept a common Network Constitution. Its identity is:

```text
network_digest = SHA-256("qdnf:network:v1" || deterministic_cbor(constitution))
network_id     = first_128_bits(network_digest)
```

The full digest is retained and verified when networks interconnect. A network does not need global
registration. Two networks with incompatible constitutions remain separate or exchange through an
explicit inter-network gateway agreement.

### 2.2 Realm

A realm is a bounded local routing domain:

```text
realm_digest = SHA-256("qdnf:realm:v1" || network_digest || realm_genesis_record)
realm_id     = first_128_bits(realm_digest)
```

Typical realms are an ad hoc local mesh, home, vehicle, organization, event, clinic, neighborhood,
or resilience hub. A large organization uses multiple realms rather than one unbounded link-state
database.

### 2.3 Node

```text
node_digest = SHA-256(
  "qdnf:route-node:v1" || router_public_key || realm_digest || route_epoch
)
node_id = first_128_bits(node_digest)
```

The full digest and membership proof disambiguate compact collisions. A node may use different node
IDs in different realms to reduce correlation.

## 3. Network Constitution

The signed Network Constitution defines:

- version, network name as non-authoritative display metadata, and strong network digest;
- accepted cryptographic suites and protocol versions;
- maximum realm/path/record sizes;
- realm-creation and interconnection policy;
- constitution update and emergency procedures;
- signer set and threshold;
- non-derogable security/privacy minima;
- expiry/review interval; and
- predecessor digest for amendments.

An open ad hoc network may use a self-signed genesis constitution with no claim of external trust. A
community network may require M-of-N signers. Constitution membership establishes routing-protocol
compatibility and governance only; it does not grant application data access.

## 4. Realm Genesis and admission

The Realm Genesis Record defines:

- network digest and realm nonce;
- creator routing key and optional administrative signer set;
- admission mode: `open`, `invitation`, `relationship`, `credential`, or `threshold`;
- routing/control export policy;
- size limits, metric profile, and gateway eligibility;
- public/private discovery mode;
- route epoch duration;
- optional geographic description as private/non-authoritative metadata; and
- issue/expiry/predecessor/proof.

A Realm Membership Credential binds a route-node key to realm, role, allowed control messages,
export ceiling, epoch, validity, and revocation reference. Open admission still requires proof of
node-key possession and resource quotas; it does not imply application trust.

## 5. Router roles

| Role | Responsibilities |
|---|---|
| leaf | originates/receives local service traffic; does not forward |
| member router | forwards within realm and participates in signed LSAs |
| realm gateway | exchanges inter-realm paths under export policy |
| border translator | terminates QRoute at a Legacy Internet Gateway; never transparent |
| directory/DHT node | stores bounded signed resolution records over QRoute |
| constrained proxy | represents sleeping/low-power reachability without owning child capability |

Roles are explicit in membership credentials. A leaf cannot become a gateway by setting a flag in an
advertisement.

## 6. Link metrics

The initial metric is a tuple evaluated locally:

```text
(policy_class, reachability, hop_cost, latency_class,
 loss_class, bandwidth_class, energy_cost, monetary_cost, privacy_cost)
```

- `policy_class` is an eligibility gate, not a weighted number.
- `reachability` comes from authenticated recent QLink observation.
- Other fields use coarse integer classes to avoid floats and false precision.
- Remote metrics are assertions. Local measurements and administrator policy dominate.
- Humanitarian priority is applied only after authorized classification and proportionality review.
- A deterministic lexicographic tuple and route digest break ties.

No peer can buy or self-assert universal priority through a single scalar reputation/price.

### 6.1 Resource and tariff scope

Energy classes derive from scoped joule observations or explicit estimates; latency remains a
duration and is distinct from airtime or device service time. Every metric profile defines the
work basis (for example per delivered KiB), observation interval, evidence state, normalization,
and path aggregation. Unknown energy is not zero-cost. Realm-normalized values cannot be compared
across realms without an agreed mapping; incompatible metrics cannot improve a path's preference.

Monetary hints identify the asset/issuer and billing basis. They are not quotes, spend permissions,
or evidence of settled payment. Discovery need not expose a private tariff or payer. Detailed
terms are negotiated through [CBOR-LD contracts](./ontological-contracts.md) under the
[commons economics profile](./commons-and-resource-economics.md).

Paid transit uses a preauthorized, bounded allowance installed by cold QPolicy processing. Route
migration or provider substitution must fit the accepted funding and aggregate spend/resource
limits, otherwise billable work pauses for a new agreement. Forwarding uses verified handles and
counters; it never calls a wallet or price oracle per packet. Payment cannot defeat congestion,
privacy, or control/revocation traffic protections.

## 7. Link-State Advertisement

An LSA contains:

- network/realm/full origin node digest and compact node ID;
- boot ID and monotonically increasing sequence;
- issue time, maximum age, and route epoch;
- up to 32 neighbor entries;
- origin role and capability bitmap;
- predecessor LSA digest;
- signer membership credential digest; and
- COSE_Sign1 proof.

Each neighbor entry contains neighbor node/link ID, bidirectional-state flag, locally observed metric
classes, bearer class, MTU class, and adjacency expiry. Private relationship identifiers are not
included.

### 7.1 Bidirectional confirmation

A link is usable for transit only when both endpoints advertise a compatible adjacency or complete a
direct signed adjacency confirmation. A one-sided claim may support diagnostic reachability but not
forwarding.

### 7.2 Flooding

Routers flood a newly verified LSA once per digest to eligible QLink adjacencies. They do not flood:

- older sequence from the same origin/boot ID;
- expired or future-invalid records;
- records violating realm limits/policy;
- exact duplicates;
- unverifiable membership or signature; or
- quarantined same-sequence conflicts as active routes.

Flooding uses jitter and per-origin rate caps. A digest cache suppresses duplicates. Summaries allow
neighbors to request missing LSAs after partition healing without reflooding the complete database.

### 7.3 Restart

A router restart generates a new random boot ID and begins sequence at zero. A signed restart marker
links the new boot to the prior node/membership record when available. Peers reject a new boot that
attempts to resurrect expired/revoked membership.

## 8. Shortest-path computation

Each router builds a verified realm graph and runs deterministic bounded Dijkstra/SPF:

1. remove expired, quarantined, policy-ineligible, or unconfirmed links;
2. compare policy class first;
3. accumulate bounded integer hop/quality costs with saturating arithmetic;
4. retain up to three equal/near-equal next hops when multipath is permitted;
5. break exact ties by ordered full node/path digests; and
6. emit immutable forwarding-table generation into caller-owned buffers.

Route computation is cold/Tier-2 and may use a budgeted workspace. Packet forwarding is Tier-1 and
reads a fixed published generation. A new table swaps atomically after complete validation; partial
computation never changes live forwarding.

## 9. Inter-realm path advertisements

A Realm Path Advertisement (RPA) contains:

- origin and destination network/realm digests;
- advertising gateway DNI and membership/delegation proof;
- ordered realm path, maximum 16;
- next-hop adjacency and previous RPA digest;
- reachable service classes or signed realm-set manifest digest;
- sensitivity and policy ceiling;
- coarse cost tuple;
- sequence, issue time, expiry, route epoch; and
- per-hop attestation chain or compact aggregate profile when standardized.

Each gateway appends its realm only after validating the received path, confirms export/import policy,
reduces (never widens) sensitivity/policy ceiling, updates metric, and signs the new canonical RPA.

Reject if:

- own realm or any realm repeats;
- path exceeds 16;
- network/realm full digests do not match compact IDs;
- predecessor or signature chain is invalid;
- gateway lacks export authority;
- ceiling is widened;
- expiry exceeds the upstream advertisement; or
- the destination is withdrawn/quarantined.

There is no mandatory default route. A realm may advertise an explicit legacy-gateway service route,
but native unresolved targets never follow it implicitly.

## 10. Route selection between realms

Default local preference:

1. allowed import/export and sensitivity policy;
2. native path over legacy/transition gateway;
3. authenticated relationship/community path where requested;
4. fewer realm hops;
5. local measured availability/latency/loss;
6. lower energy/monetary/privacy cost per user policy;
7. more recent valid sequence;
8. deterministic path digest.

Administrators may change policy order but cannot disable loop, signature, expiry, block, or
non-derogable checks.

## 11. Forwarding header

The QRoute authenticated QFrame extension contains:

| Field | Size |
|---|---:|
| destination network ID | 16 bytes |
| destination realm ID | 16 bytes |
| destination node ID | 16 bytes |
| source realm ID | 16 bytes |
| route generation | u32 |
| traffic class | u8 |
| path flags | u8 |
| reserved | u16 |
| packet/message digest prefix | 8 bytes |

The full route/session records already bind compact IDs to full digests. A router looks up
`(network_id, realm_id, node_id)` in its immutable forwarding generation, selects an active QLink next
hop, decrements base-header hop limit, updates hop-local authentication, and transmits. It never
changes end-to-end QSession ciphertext.

## 12. Forwarding failures

Authenticated QRoute errors include `no_route`, `realm_unreachable`, `node_unknown`, `hop_limit`,
`mtu`, `policy_denied`, `congested`, and `route_generation_stale`. An error binds the triggering
packet digest prefix and returning path. It is rate-limited and never quotes application plaintext.

Unauthenticated sources receive no detailed topology error. QSession decides whether to retry,
migrate, or report failure.

## 13. Multipath

- At most three active next hops per destination in the base profile.
- Packets from one reliable stream use one path until migration unless the multipath extension is
  negotiated.
- Unordered content blocks may use independent verified paths/providers.
- Congestion and RTT state are maintained per path.
- A path is validated by challenge/response before receiving more than anti-amplification traffic.
- A failed path does not weaken session identity or capability state.

## 14. Mobility

Node movement may change link ID, node ID, realm, and RAR while its persistent DID/resource remains
stable.

### 14.1 Within a realm

The node forms a new QLink, advertises adjacency, withdraws/stales the old adjacency, and may keep the
same epoch node ID. QSession validates the new path and migrates.

### 14.2 Between realms

The node joins the new realm, creates a new realm/node DNI, publishes a higher-sequence RAR, and
optionally retains both old/new routes during a bounded handover. Route selection never treats the old
route as authoritative after withdrawal/expiry.

### 14.3 Mobile subnet

A gateway moves a whole child realm by updating its macro RPA/RAR. Internal node IDs remain stable
within the current realm epoch. The Subnet Delegation Record limits which realm/services the gateway
may advertise and does not grant child data access.

## 15. Sleeping and intermittent nodes

A node may publish a signed reachability schedule or designate an authorized mailbox/proxy. The proxy
advertises custody, not live adjacency or application authority. Queued items are end-to-end encrypted,
bounded, expiring, deduplicated, and reauthorized on delivery. Failure to appear does not revoke the
node's persistent identifier.

## 16. Partitions and healing

Realms continue locally during partition using unexpired known state. They do not invent remote
reachability. On reconnection:

1. exchange database summaries;
2. request missing newer records within quotas;
3. validate signatures/sequence/expiry before activation;
4. isolate same-sequence conflicts;
5. recompute route table deterministically; and
6. allow QSession migration only after new path validation.

Wall-clock uncertainty and disconnected sequence histories are surfaced. Security state such as
revocation is fail-closed according to sensitivity policy.

## 17. QRoute DHT

The resolution DHT is an application of QRoute, not its foundation.

- DHT node ID is the full SHA-256 digest of an authorized DHT service key.
- Separate overlays may exist per public network, realm, community, or relationship group.
- Kademlia distance determines storage/routing only, never trust.
- Stored values are signed RARs, Alias Assertions, provider pointers, or digests with strict TTL/size.
- The store caps records per target/publisher and rejects structurally invalid/expired data before
  signature work.
- Resolvers query diverse routes/providers for high-value operations.
- Private overlays encrypt keys/values to the group context.
- Provider disappearance is availability information, not revocation or nonexistence.

## 18. Swarm routing

A resource swarm exposes multiple independently verified provider DNIs. QRoute gets packets to each
provider; QResolve/QSync verifies replica authorization and content/operation integrity.

- Immutable blocks may be scheduled across providers by digest.
- Mutable state uses signed operations and application merge policy.
- Providers cannot alter target identity by advertising a faster path.
- Failure/poor performance affects only local route observation, not a global reputation score.

## 19. Traffic classes

Base traffic classes are control, interactive, bulk, background, and locally verified humanitarian.
Control traffic has small strict quotas and cannot starve application traffic. Humanitarian priority
requires authenticated classification and local deontic/proportionality policy. Remote flags alone are
ignored. Rate fairness is enforced per adjacency/realm policy.

## 20. Routing table bounds

Default general-node bounds:

- 256 active neighbors;
- 4,096 realm nodes/LSAs;
- 8,192 intra-realm forwarding entries;
- 4,096 inter-realm destinations;
- three next hops per destination;
- 16-hop realm paths;
- two active LSA boot epochs per origin during restart transition;
- 42 MiB maximum for any construction pass; and
- fail-closed admission before allocation beyond configured arena.

Constrained profiles lower these values and may operate as leaves.

## 21. Conformance scenarios

1. Three raw-Ethernet nodes route A-C through B with IP disabled.
2. A forged one-sided adjacency never becomes transit-capable.
3. Link loss triggers deterministic alternate path without session identity change.
4. Stale, repeated-realm, widened-ceiling, wrong-signer, and same-sequence-conflict RPAs fail.
5. Two disconnected realms reconnect and converge without erasing quarantined conflict.
6. A vehicle child realm moves gateway paths while internal services remain addressable.
7. No default route sends native lookup strings to a LIG/DNS service.
8. DHT poisoning cannot bypass RAR/controller verification.
9. Resource limits hold under LSA flood and many bogus paths.
10. Equivalent verified inputs produce byte-identical forwarding generations.
11. Unknown or incompatible energy/tariff metrics never become fictitious zero-cost routes.
12. Migration cannot create unaccepted transit charges or reset aggregate resource/spend allowances.
