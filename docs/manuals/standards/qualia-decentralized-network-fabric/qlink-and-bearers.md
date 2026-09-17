# QLink and Bearer Profiles

**Status:** Normative design 0.1

## 1. Purpose

QLink replaces native dependency on ARP/NDP address resolution and DHCP/SLAAC link configuration.
It discovers eligible neighbors, proves control of ephemeral link keys, binds those keys to the
bearer locators actually observed by each endpoint, negotiates MTU/features, and creates a short-
lived encrypted adjacency for QRoute.

QLink does not authenticate a persistent DID/resource by itself and does not authorize application
operations.

## 2. Bearer contract

A bearer adapter implements the following conceptual API:

```rust
trait QdnfBearer {
    fn profile(&self) -> BearerProfile;
    fn scope_id(&self) -> [u8; 32];
    fn mtu(&self) -> u16;
    fn send(&mut self, destination: &BearerLocator, frame: &[u8]) -> Result<(), BearerError>;
    fn send_group(&mut self, group: GroupLocator, frame: &[u8]) -> Result<(), BearerError>;
    fn receive_into(
        &mut self,
        frame_out: &mut [u8],
        metadata_out: &mut ReceiveMetadata,
    ) -> Result<usize, BearerError>;
}
```

The production hot-path API is caller-buffered. `BearerLocator` is a bounded tagged byte array, not
a string. An adapter returns the source it observed; QLink never trusts a source locator copied from
the payload.

### 2.1 Required adapter properties

- stable scope identifier for one attachment, derived without exposing global hardware serials;
- exact MTU and maximum group-frame size;
- unicast and, where supported, bounded group delivery;
- source locator observation;
- interface-up/down and MTU-change notification;
- no automatic DNS/IP fallback; and
- documented privacy, ordering, duplication, corruption-detection, and energy properties.

### 2.2 Resource observations

Where available, bearer metadata supplies scoped airtime, byte/retry counts, and energy observations
or estimates under [Commons and Resource Economics](./commons-and-resource-economics.md). Missing
telemetry is explicit and does not prevent ordinary peering. Observations use bounded counters and
caller-owned records; contract valuation and payment settlement never run inside `send`/`receive`.

## 3. Raw Ethernet profile

`raw-ethernet-v1` carries QFrame directly in Ethernet payloads.

- Development deployments use an explicitly configured experimental/local EtherType.
- Production public deployment requires an assigned EtherType or administered encapsulation.
- Destination MAC is an observed local forwarding locator, not a QDNF identifier.
- Initial public/manual beacons use a configured locally administered multicast group.
- Private discovery beacons may share that group because their rendezvous tag is unlinkable and
  useless to non-members; high-risk deployments may allocate relationship/group-specific filters.
- Ethernet FCS is not a security proof. QLink AEAD is mandatory after adjacency.
- VLAN tags and switch topology are bearer metadata and may influence local policy but do not enter
  persistent identity.
- Bridged loops must be controlled by the physical network or bounded QLink duplicate suppression;
  QRoute still enforces hop limits.

An implementation must be able to operate with the host IP stack unconfigured on the QDNF
interface. Packet capture tests verify that it emits no ARP, NDP, DHCP, DNS, or IP frames.

## 4. Local IPC profile

`local-ipc-v1` connects processes on one device through shared memory, a named local socket, or an
equivalent OS primitive.

- Scope ID includes the boot/session domain and IPC namespace, not the filesystem path.
- OS peer credentials are additional evidence, not a replacement for QLink/QSession proof.
- Filesystem socket names are never interpreted as remote locators or inserted into RARs.
- Permissions restrict access before parsing.
- Frame and queue sizes obey the same limits as other bearers.
- Restart invalidates ephemeral link IDs and requires a fresh adjacency.

## 5. Constrained bearer profile

BLE, low-power radio, acoustic, and serial adapters may have small MTUs and asymmetric energy costs.
They define:

- a bearer-specific locator and group delivery mechanism;
- minimum usable MTU;
- wake/sleep and duty-cycle hints;
- maximum discovery cadence;
- whether link acknowledgement exists;
- corruption detection provided by the medium; and
- a fragmentation profile or a declaration that only compact QLink/QRoute messages are supported.

Constrained nodes may be leaf-only and delegate inter-realm routing to a gateway. The gateway cannot
widen application capabilities or decrypt QSession payloads.

## 6. Transition bearers

`udp-transition-v1`, WireGuard, libp2p, and WebRTC carry QFrames through legacy mechanisms.

- Direct numeric endpoint or previously verified route hints avoid DNS but still depend on IP.
- A domain-based endpoint uses the LIG or host legacy stack and is marked `legacy-dependent`.
- Carrier encryption is retained but does not replace QSession end-to-end authentication.
- Carrier peer IDs/keys are bound into the QSession transcript.
- NAT rebinding/roaming observations remain local and do not become controller-signed RAR fields.
- A Transition bearer never satisfies Native Independent conformance alone.

## 7. Interface lifecycle

```text
Down
  -> Initializing
  -> Scoped
  -> Discovering
  -> Operational
  -> Degraded
  -> Draining
  -> Down
```

On `Initializing`, the adapter obtains randomness, derives a non-global scope ID, measures MTU, and
installs group filters. On `Scoped`, QLink generates an epoch key/link ID. On `Operational`, it may
carry routing/data. MTU reduction, excessive errors, or missing rekey moves to `Degraded`. Shutdown
withdraws local advertisements where possible, stops new sessions, drains bounded traffic, clears
keys, and removes filters.

## 8. Discovery records

### 8.1 DiscoveryBeacon fields

| Field | Size/bound |
|---|---:|
| protocol version | u8 |
| discovery mode | u8 |
| suite bitmap | u32 |
| feature bitmap | u64 |
| rendezvous/service tag | 16 bytes |
| ephemeral link ID | 16 bytes |
| ephemeral X25519 key | 32 bytes |
| bearer scope digest | 16 bytes truncated, verified fully in handshake |
| boot ID | 8 bytes random |
| epoch window | u32 |
| expiry delta | u16 seconds |
| MTU class | u16 |
| optional anti-amplification token | at most 32 bytes |

The beacon contains no wall-clock timestamp requiring globally synchronized clocks. Receivers apply
local arrival time and a narrow accepted epoch window.

### 8.2 Private rendezvous tag

```text
full_tag = HMAC-SHA-256(
  discovery_secret,
  "qdnf:qlink-rendezvous:v1" || bearer_scope || epoch_window || role
)
tag = first_128_bits(full_tag)
```

`discovery_secret` has at least 256 bits of entropy. Passwords, names, DIDs, phone numbers, and email
addresses are invalid discovery secrets. Pairwise secrets differ per relationship; group secrets
rotate when membership changes.

### 8.3 Public discovery

Public beacons use a service-class tag derived from the full canonical service IRI and network/realm
scope. They reveal that a service class is present, not its operator name or stable target. The RAR
is returned only after anti-amplification and QLink establishment.

## 9. QLink handshake

### 9.1 Messages

`LinkChallenge` contains initiator link ID/key, responder link ID/key, initiator random nonce,
observed responder locator digest, bearer scope, selected suite/features, proposed MTU, and optional
cookie.

`LinkProof` contains both link IDs/keys, both nonces, both observed locator digests, selected
suite/features/MTU, transcript digest, key-confirmation tag, and rendezvous proof where applicable.

`LinkConfirm` is an encrypted empty/control frame in the new key phase acknowledging the final
transcript. An adjacency is not `Active` until both directions confirm.

### 9.2 Anti-amplification

Before the source locator is validated, a node sends no more than three times the bytes received from
that locator and retains at most one small challenge slot. Stateless cookies are HMACs over observed
locator, scope, time bucket, offered key digest, and a rotating local secret. They reveal no stable
node secret and expire within 30 seconds.

### 9.3 Simultaneous open

If both nodes initiate, the lexicographically smaller tuple `(link_id, ephemeral_key)` is the logical
initiator for transcript ordering. Both nonces remain included. Duplicate half-open states merge by
the complete key tuple, never by bearer locator alone.

### 9.4 Completion and failure

Handshake state expires after 10 seconds by default. A neighbor gets at most two concurrent half-open
handshakes and an interface at most 64. Signature/PSK failures return a generic close or silence.
Successful QLink proves control of ephemeral keys and, in private modes, relationship-secret
possession. Persistent target verification remains QSession work.

## 10. Neighbor table

Each active entry stores bounded fields:

- link ID and full link-key digest;
- observed bearer locator;
- send/receive key phase and replay window;
- negotiated MTU/features/suite;
- bearer scope and interface index;
- discovery policy handle, not raw relationship secret;
- last authenticated receive and local expiry;
- observed loss/RTT/energy class;
- routing eligibility and quarantine flags; and
- byte/packet counters.

Defaults:

- 256 neighbors per general node;
- 32 per constrained node;
- idle expiry 120 seconds for local dynamic links unless realm policy specifies less;
- keepalive only when a route/session requires it;
- deterministic eviction of expired, quarantined, then least-recently-authenticated entries;
- protected entries for active critical sessions only within a fixed budget.

## 11. Replay window

Each direction uses a 256-packet sliding window per key phase. Packets older than the window or
already marked are dropped before payload dispatch. A valid next key phase has its own window. Link
control sequence and route-object sequence are separate namespaces.

## 12. MTU and fragmentation

The [QPR post-quantum profile](./post-quantum-security.md#6-wire-size-fragmentation-and-denial-of-service)
requires a separately frozen, cookie/relationship-gated handshake chunk mechanism before QLink
authentication. That mechanism admits only bounded handshake flights; it is not general message
fragmentation and confers no identity/application authority. Until implemented and tested, a bearer
whose MTU cannot carry the selected handshake reports unsupported PQ operation, without downgrade.
The rules below continue to apply to ordinary authenticated QLink messages.

QLink negotiates the minimum of both reported MTUs minus bearer/QFrame overhead. The base minimum
QDNF frame payload for general nodes is 512 bytes; constrained profiles may define a lower value.

Fragment fields include message ID, fragment index/count, total length, and full-object digest. Rules:

- only authenticated neighbors may send fragments;
- maximum 16 fragments and 64 KiB total;
- fragment 0 reserves a bounded slot only after quota checks;
- all fragments share route/session context and key phase;
- duplicate fragments do not extend timeout;
- timeout defaults to 2 seconds local / 10 seconds constrained;
- completed object digest is checked before parsing; and
- routing-control protocols should fit one frame whenever possible.

Reliable QSession streams segment at QSession and should not use QLink fragmentation except where a
single control record cannot fit the bearer profile.

## 13. Group delivery

QLink group selectors support discovery and controlled realm dissemination. They are not global
broadcast addresses.

- Discovery group accepts only beacons/challenges within rate limits.
- Realm-control group is available only after membership and uses an epoch group key or per-neighbor
  authenticated flooding.
- Application group delivery is implemented by QSession group semantics, not raw QLink multicast.
- Group keys rotate on membership removal for confidentiality going forward.
- A group message carries an origin digest and duplicate-suppression ID.

## 14. Multiple interfaces

A node may have concurrent Ethernet, radio, IPC, and transition interfaces. Link IDs differ per
interface/epoch. QRoute may use multiple adjacencies but does not merge bearer scopes. QSession path
migration verifies the new DNI/path before moving traffic. Interface priority is local policy based
on authorization, reachability, cost, energy, privacy, and user choice.

## 15. Link error behavior

| Error | Action |
|---|---|
| malformed/oversize beacon | drop before crypto; rate count |
| unknown version/suite | optional generic unsupported response within amplification limit |
| rendezvous mismatch | silent drop |
| bad proof/tag | drop; bounded failure count; possible temporary locator suppression |
| replay | silent drop; local metric |
| MTU violation | authenticated `mtu_error` with supported bound |
| interface down | expire adjacency, notify QRoute/QSession, zero keys |
| locator change | require authenticated proof or new handshake; never accept payload claim alone |
| repeated resource abuse | quarantine locator/link ID without creating global reputation |

## 16. Privacy analysis

Even rotating beacons reveal radio/link activity, timing, approximate population, and frame sizes.
QLink therefore supports beacon jitter, cover groups, optional padding classes, receive-only/manual
mode, and minimum duty cycle. Padding trades privacy for bandwidth/energy and is local policy.

Hardware MAC randomization remains desirable. QLink does not depend on a stable MAC and must survive
locator changes through a fresh binding or verified session migration.

## 17. Conformance tests

A bearer/QLink implementation must demonstrate:

- operation with host IP disabled;
- no ARP, NDP, DHCP, DNS, or IP emission on raw-native interface;
- two-peer discovery and encrypted frame exchange;
- private beacon unlinkability across epochs;
- locator-spoof and transcript-tamper rejection;
- replay, stale epoch, wrong PSK, unknown suite, and amplification limits;
- MTU negotiation, fragmentation quotas, timeout, and digest validation;
- simultaneous open and rekey;
- deterministic neighbor eviction; and
- clean interface shutdown/key erasure.
