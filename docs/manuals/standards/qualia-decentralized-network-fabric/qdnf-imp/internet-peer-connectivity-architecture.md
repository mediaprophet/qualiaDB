# QDNF Internet peer connectivity — architecture decision proposal

Date: 2026-09-10. Status: in-tree implementation on `0.0.38`; no deployment certification.

Related notes: [nat-traversal-expert-brief.md](./nat-traversal-expert-brief.md),
[internet-two-host.md](./internet-two-host.md).
A–E decisions in §2 are the recorded answers. Real relay URLs and operators remain
deployment inputs. The connection manager, ICE checklist, TURN codec, local
authenticated WSS, invitations, durable queues, consent/path-bind, and NAT64
contract are in-tree. They are not an Internet trial.

Reviewed local baseline: `0.0.37`, `79231d7d98bee8bce1b02e93f3253d198582d3d2`.
Reviewed selected remote sources: `cursor/qdnf-enhancement-e00-e01-cb60`,
`16ed59e7fe514f39d39f3e9473ae3649a743055f`, including Cursor's brief introduced in `38112131`.

## 1. Decision

Build **one policy-controlled connection manager, with IPv6-preferred direct paths, promptly
available outbound relays, and durable delivery across disconnection**. Retain WireGuard as the
SocialWebNet datagram carrier. Refactor the carrier boundary so that the same tunnel can use direct
UDP, a standard TURN allocation, or an outbound WSS relay. Keep QSession as the authenticated
application/authority boundary and keep the native QDNF bearer independent of IP dependencies.

For an ordinary permitted Internet connection, establish an approved relay concurrently with
bounded direct-path discovery. Use the first authenticated, policy-compatible path. Upgrade to
direct IPv6 when it performs well; accept IPv4 promptly when it is the working path. Keep a bounded
relay standby when the availability policy and energy budget require it. Connection success must
not wait for exhaustive NAT classification or a sequence of failed direct attempts.

For hostile-environment use, **filter by disclosure and protection requirements before probing**.
If direct IP disclosure is forbidden, never gather/export direct candidates to peers, run public
STUN, or attempt a direct upgrade. Encryption and reliable connectivity do not establish anonymity.
Where the selected protection profile cannot be met, queue sealed data or report unavailability.

This is a focused refactor of connection ownership and Internet adapters. Rewriting cryptographic
primitives, the graph engine, or the entire transport stack would add risk without solving the
immediate reachability problem. QSession's congestion/recovery implementation still needs independent
qualification; retaining it is not a claim that it already matches a mature Internet transport.

The approach resembles the relay-assisted establishment used by [Tailscale](https://tailscale.com/docs/reference/connection-types)
and [iroh](https://docs.iroh.computer/concepts/relays). These are useful architectural comparators,
not dependencies or evidence of Qualia's success rate.

## 2. Disposition of Cursor's brief

The brief is on the remote branch: [source at the reviewed revision](https://github.com/mediaprophet/qualiaDB/blob/16ed59e7fe514f39d39f3e9473ae3649a743055f/docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/nat-traversal-expert-brief.md).

| Question | Decision |
|---|---|
| A: WireGuard + checks + outbound relay? | Confirm, with concurrent establishment. ICE is the procedure that discovers/checks direct and TURN candidates; it is not a second transport attempted after WireGuard. WSS circuits are separately scheduled carrier candidates, not automatically standard ICE candidates. |
| B: Operator and first URL? | Recommend mission/commons-controlled infrastructure with independently administered backup capacity. No operator or live endpoint has been supplied or verified, so there is no honest URL to name yet. Initial service: authenticated WSS on TCP 443. Actual deployment inventory must supply host, certificate identity, administrator and quotas before the Internet test. |
| C: Initial signalling? | First two-host test: privately exchanged, expiring connection invitations with independently verified key fingerprints. Production: invitation-bootstrapped, end-to-end protected HTTPS rendezvous/mailboxes, integrated with QSR. |
| D: TURN for browsers? | Required for a qualified WebRTC fallback. Use mission-operated TURN with short-lived credentials. Actual TURN URIs remain a deployment input. A WSS relay is not a TURN server, even when the same organisation operates both. |
| E: libp2p circuit relay? | Exclude from the replacement's default dependency graph. Existing compatibility applications can migrate through explicit adapters. |

Technical corrections to carry into implementation:

- WireGuard roaming follows authenticated traffic; it does not create general inbound reachability.
- Cursor's two STUN results demonstrate different observed mappings to different destinations.
  They do not alone distinguish all address/port-dependent mapping and filtering behaviours, nor
  prove that every possible direct technique fails. Treat the observation as a reason to prefer
  relay establishment; retain only bounded, useful direct checks. Mapping and filtering are
  separate properties in [RFC 4787](https://www.rfc-editor.org/rfc/rfc4787.html).
- WSS/443 is a broad compatibility fallback, not a guarantee. Captive portals, explicit proxies,
  blocked Upgrade requests, TLS interception and destination filtering can prevent it.
- TURN is usable by native UDP applications; it does not require WebRTC or DTLS. Client-to-relay
  TCP/TLS support does not make the other leg TCP or provide end-to-end application delivery.
  [TURN specification](https://www.rfc-editor.org/rfc/rfc8656).
- The IPv6-only inner-packet rule belongs to this SocialWebNet profile. Boringtun itself exposes
  both IPv4 and IPv6 tunnel results; the inspected caller rejects IPv4. An inner ULA is not a
  globally reachable outer IPv6 address.
- A self-signed invitation proves control of its signing key, not that the signer is the intended
  human or an authorised mission member. Pin the key through a trusted introduction/issuer.
- Ordinary phrases must not generate production WireGuard private keys. Generate independent
  device keys with the OS CSPRNG. A pairing secret is a separate high-entropy, expiring credential.

## 3. Evidence and reuse boundaries

| Source inspected | What it establishes | Design consequence |
|---|---|---|
| Local `qualia-peer/src/lib.rs`, `net/peer/host/mod.rs` | IPC facade; baseline directly constructs Allow and copies application payloads | Do not deploy this local facade as a protected Internet endpoint. Reconcile against the remote implementation first. |
| Remote `qdnf/crypto/finished.rs`, `schedule.rs`, `peer/host/driver.rs` | Secret-based HMAC confirmation, transcript-bound derivation, permit/key-based protected activation are present in source | Preserve and review the newer work; old findings must not be presented as uncorrected on this branch. A complete protocol audit remains outstanding. |
| `p2p/wireguard_runtime.rs` | Real UDP socket and boringtun state machine; receive uses `data.to_vec()` | Preserve the cryptographic engine; separate socket ownership and add caller-buffered receive/events. Existing comments do not prove zero allocations. |
| Remote `p2p/social_qdnf.rs`, `mesh_datagram.rs` | QFrames carried inside IPv6/UDP, QDNF port 6423, chat port 6420; loopback test code | Retain framing compatibility, add checked lengths, negotiated MTU and a shared demultiplexer. `recv` must not consume/discard another service's packets. |
| Remote `p2p/outbound_relay.rs` | Fixed queues behind `Arc<Mutex<_>>`; test shuttles WireGuard bytes within one process | Useful fixture only. No real outbound dial, TLS, admission, public server or two-NAT emulation is proved by this fixture. |
| Remote Internet test note | Reports external STUN observations; explicitly says two-host test unexecuted | Preserve that evidence distinction. Reading the note is not repeating the measurement. |
| Local `qualia-client-core/src/connection_identifier.rs` | Signed `qcx1_` with clear-text name/DID/hints and delimiter-based signing serialization | Use only in a trusted private bootstrap; define an unambiguous versioned encoding and minimise exposed fields for production. |

The remote branch also adds path, storage, Ethernet and cell modules. This review sampled the
connectivity boundaries; it did not certify its roughly 223 changed networking/p2p files. Integrate
reviewed changes using the existing programme, not a blind wholesale replacement of local files.

## 4. Layering and delivery ownership

```text
Application intent: target, purpose, label, deadline, delivery class
                          |
             QPR / Webizen authority and resource admission
                          |
          QSession authenticated service + QSync durable operations
                          |
                   QFrame bearer interface
                   /                     \
       Native QLink/QRoute           SocialWebNet adapter
       Ethernet/local/radio          inner IPv6/UDP :6423
                                            |
                                  WireGuard tunnel engine
                                            |
                                  connection/path manager
                           /                |              \
                    direct UDP         TURN datagrams     WSS circuit
                    IPv6 / IPv4        UDP / TCP / TLS     outbound :443
```

The connection manager owns reachability, interface generations, candidate checks and path
selection. The tunnel engine owns WireGuard keys, counters and encryption. QSession owns
application peer proof, negotiated protection and service admission; a relay login cannot grant
those. QSync/core transaction owners retain operation identities and durable effects across all
carrier changes. No route or ciphertext receipt means the recipient applied an operation.

For profiles requiring post-quantum confidentiality, preserve the reviewed hybrid QSession key
establishment and authenticated encryption on every carrier. An ML-DSA signature alone does not
provide confidentiality, and classical WireGuard/TLS cannot substitute for that requirement.
Bind target, both roles/keys, protocol version, carrier identity, protection policy and negotiated
limits into the reviewed session transcript. Require fresh confirmation and current authority on
reconnect; prohibit replay-unsafe zero-RTT application release. Crypto agility is versioned and
authenticated, never a silent fallback. Independent protocol/key-lifecycle review remains a gate.

All Internet paths above are labelled transition. They do not replace native-independent Ethernet
acceptance. Public Internet routing still requires the host IP stack even if QDNF's own names,
routes and authority are independent of DNS/IP addressing.

### Transport/recovery contract

Represent carrier properties explicitly: datagram boundaries, maximum payload, end-to-end versus
hop-local reliability, backpressure, ordering, expiry and observed source provenance.

- Direct WG/UDP and WG/TURN provide no reliable stream service. QSession must supply authenticated
  ACKs, loss recovery, pacing and congestion control; WireGuard does not supply these.
- WSS gives an ordered reliable client-to-relay stream. Relay queue rejection, reconnect and the
  destination leg can still lose a forwarded datagram. Preserve end-to-end QSession delivery
  accounting; never disable it merely because `ordered == true`. Bound queues and use relay
  backpressure. Do not add another bespoke hop-level retransmission protocol over TCP.
- Bulk and interactive traffic require separate bounded scheduling queues. Where WSS is used,
  permit a small number of separately admitted circuits to limit bulk head-of-line interference.
  Actual real-time media may be unavailable when only a congested stream relay survives.
- If a later QUIC carrier supplies reliable peer-to-peer streams, map QSession service streams to
  them through a versioned carrier-owned-recovery profile. Do not layer two competing reliable
  stream/retransmission engines over the same stream. Durable application receipts still apply.

QUIC is a strong comparator: [RFC 9000](https://www.rfc-editor.org/rfc/rfc9000) and
[RFC 9002](https://datatracker.ietf.org/doc/html/rfc9002) define transport and recovery. It does not
itself discover peers or make UDP pass a firewall. Multipath QUIC was in the RFC Editor queue as
`draft-ietf-quic-multipath-21` at review time, not an assumed universally deployed capability.
[Current document status](https://datatracker.ietf.org/doc/draft-ietf-quic-multipath/).

## 5. Connection establishment and migration

### 5.1 Bootstrap without publishing a relationship graph

Issue a private, short-lived invitation binding the expected peer key, protocol version, allowed
relay/service descriptors, expiry, nonce and protection policy. Retain `qcx1_` compatibility at the
import boundary. A successor encoding must use canonical length-delimited fields, reject duplicate
fields and unknown critical extensions, and impose decoded-size bounds. Never concatenate arbitrary
strings separated by unescaped delimiters for new signed messages.

An invitation points to a bounded set of reachable rendezvous services; this solves the bootstrap
cycle before QSR is reachable. Known peers then exchange encrypted candidate updates and current
QSR route hints. Rendezvous stores opaque, expiring records addressed by relationship-scoped random
capabilities. Authentication, quotas and response minimisation prevent an unrestricted lookup oracle.
Do not publish stable person keys, device addresses or mission membership into a public DHT.

Candidate updates bind both peer identities/relationship, connection attempt, interface generation,
sequence, expiry and required profile. Authenticate them inside an established protected channel;
for first contact use the reviewed invitation-based encrypted exchange. Use standard cryptographic
constructions and the existing crypto owner; do not invent an ad hoc encryption format. Public
relays see connection metadata even when the record body is encrypted.

Bootstrap bundles can carry multiple signed relay names, numeric address hints and verification
keys for DNS outages. Numeric hints must retain certificate/key verification and an expiry/rotation
plan. No invalid-certificate bypass. Carrier-dependent DNS/NAT64 support remains explicit.

### 5.2 Direct candidates and IPv6

Use full ICE for the standard UDP/TURN adapter: [RFC 8445](https://www.rfc-editor.org/info/rfc8445/),
[STUN RFC 8489](https://www.rfc-editor.org/rfc/rfc8489.html), and
[Trickle ICE RFC 8838](https://www.rfc-editor.org/rfc/rfc8838.html). Include candidate-pair
checks, controlling/controlled role conflict handling, peer-reflexive candidates, nomination,
restart generations and consent freshness. A STUN Binding parser alone is not ICE.

1. Admit the operation and disclosure policy before opening discovery sockets.
2. Enumerate usable interfaces within a fixed bound. Gather IPv6 and IPv4 host candidates,
   same-socket STUN observations and approved relay allocations concurrently where allowed.
3. Interleave address families and interfaces; give working global IPv6 a small initial preference.
   Start IPv4 checks promptly instead of exhausting IPv6 candidates. Follow
   [RFC 8421's dual-stack guidance](https://www.rfc-editor.org/rfc/rfc8421).
4. A global IPv6 address is only a candidate: stateful inbound firewalls still require checks.
   Link-local addresses require the correct local interface scope and never become Internet hints.
   Avoid stable hardware-derived IPv6 interface identifiers where the platform supports alternatives.
5. Treat observed NAT behaviour as an expiring per-interface/path observation. Do not hard-code
   `relay_required_for_address_dependent() == true` as a universal connectivity theorem.
6. Keep probing and tunnel traffic on the same admitted socket/mapping. Use a validated bounded
   STUN/TURN/WireGuard demultiplexer with source and transaction checks, not socket recreation.
7. Permit PCP/administered port mappings only on trusted managed networks under explicit policy;
   never depend on them for operation or silently enable UPnP on a hostile network.

Native IPv6-only and IPv4-only peers may have no direct common address family. A dual-stack relay
bridges their carrier connectivity. Support platform NAT64/464XLAT where available. Do not assume
`64:ff9b::/96`: discover the applicable prefix through supported OS mechanisms, including
[PREF64](https://www.rfc-editor.org/rfc/rfc8781.html) or
[RFC 7050](https://www.rfc-editor.org/rfc/rfc7050.html). Numeric IPv4 bootstrap hints alone are
insufficient on some IPv6-only networks.

### 5.3 One state owner

| State | Valid transition / observable result |
|---|---|
| Admitted | Resolve only approved introduction/relay descriptors and reserve resources. |
| Establishing | Schedule permitted direct checks and relay circuits concurrently; no application release. |
| CarrierReady | Bidirectional carrier authenticated; still require QSession target/profile/authority proof. |
| SessionReady | First compatible authenticated session path may carry authorised operations. |
| Upgrading | Validate a better candidate while the working path continues; require current generation and policy. |
| Degraded | Move to a validated standby or reconnect; preserve operation IDs and report interruption. |
| Deferred | No compatible live route; persist only authorised sealed jobs within quota and expiry. |
| Closed | Cancel pending work, invalidate callbacks, release leases and erase ephemeral secrets. |

Every asynchronous completion includes attempt, slot and generation. Late success cannot activate
a reused slot. Authenticate a fresh challenge on a proposed route before changing the send path.
An old authenticated WireGuard packet is not, by itself, proof that its newly observed source is a
safe current bidirectional route. For relayed traffic, preserve the circuit's provenance instead of
mistaking the relay IP for a direct peer candidate.

Use policy feasibility first, then measured latency/loss, availability, energy and cost. Among
equivalent permitted paths, prefer direct IPv6. Use hysteresis and a minimum dwell to avoid route
flapping. A network change invalidates affected candidate observations; it does not automatically
change application identity, grant or recipient. ICE consent-to-send is separate from human consent
and application authority. [Consent freshness](https://www.rfc-editor.org/rfc/rfc7675.html).

Start with one selected carrier and one optional warm standby. Do not spray each WireGuard packet
over multiple paths: duplicate/reordered traffic can interact with anti-replay and roaming. Critical
operation hedging, if enabled later, uses separately validated paths and durable operation
deduplication with explicit byte budgets. Seamless bandwidth aggregation is a separate qualification.

## 6. Relays, browsers and hostile-network profiles

### Relay service

First milestone: a real WSS binary-datagram relay on TCP 443, with verified TLS and authenticated
outbound clients. Production also needs standard TURN over UDP for efficient relay traffic and
TURN over TCP/TLS for supported restricted networks. These share an operator/admission service,
not a fictitious common wire protocol. Standard TURN allocations need their configured public
relay port range to be reachable, not just the listener port.

WSS must be actual HTTP Upgrade through the intended proxy path. TURN/TLS on port 443 is not HTTPS.
Use separate listeners/IPs or an explicitly tested protocol-aware front end; a conventional HTTP
reverse proxy cannot be assumed to multiplex TURN. Honour configured enterprise proxies through
supported CONNECT behaviour; do not disable TLS checks or collect portal credentials automatically.

Prefer an existing reviewed relay implementation where its framing and admission fit. If using a
small Qualia WSS envelope, version and fuzz it: bounded message length, opaque circuit ID, circuit
generation, datagram type and bytes, explicit close/backpressure. Reject oversized frames before
allocation, disable WebSocket compression, authenticate before queue admission, and expire stale
circuits. It is DERP-shaped, not DERP-compatible unless that protocol is actually implemented.

Clients prove control of admitted device credentials; knowing another peer's public key cannot
register its relay address. Issue short-lived, audience-scoped circuit capabilities with packet,
byte, connection and duration quotas. Keep root issuing keys off field devices. Limit pre-auth
state and per-destination probes, reject arbitrary forwarding destinations, and rate-limit reconnects.
WireGuard key material remains endpoint-only; a relay does not terminate the WG tunnel.

Operate at least two approved failure domains before mission qualification, with distinct upstreams
and administrative access where practicable. Preload mutually reachable fallback relay sets and
replicate only the required rendezvous availability state. Two healthy but disjoint home relays do
not connect peers automatically: the peers must select a common allowed relay or use authenticated,
explicitly configured inter-relay forwarding. Do not infer independence from different hostnames.

Provide reserved control/urgent capacity, fair queues and an incident/rotation runbook. Commons
funding pays real bandwidth and operations costs; humanitarian exemptions must not create an open
unauthenticated relay. No public development relay or single provider control plane is a mission
availability guarantee.

### Browser interoperability

Browser profile: WebRTC DataChannel with ICE and configured TURN, or WSS where WebRTC cannot work.
Native browsers cannot simply reuse the native boringtun socket API. A TURN service cannot convert
browser DTLS/SCTP packets into WireGuard packets.

Define a separate QFrame/QSession browser bearer. A native peer can expose a browser-compatible
endpoint, or an approved gateway can terminate the browser carrier while forwarding end-to-end
QSession ciphertext. The gateway must never gain QSession payload keys or grant authority. For WSS
browser forwarding, reviewed QSession encryption is mandatory because outer TLS terminates at the
relay. Bind WebRTC fingerprints/carrier identities to the protected application handshake.
[WebRTC specification](https://www.w3.org/TR/webrtc/).

Use `iceTransportPolicy: relay` where IP disclosure to peers is prohibited, and verify the actual
browser's traffic rather than assuming the setting alone meets a full threat model. Self-host or
pin delivered application assets: compromised browser JavaScript can exfiltrate plaintext before
transport encryption. This endpoint risk is part of profile qualification.

### Protection selection

Reuse P0–P4 from [the sensitive-operations blueprint](sensitive-operations-blueprint.md), with
independent connectivity controls rather than a new conflicting profile ladder:

| Connectivity control | Meaning / remaining exposure |
|---|---|
| Direct permitted | Counterpart learns an IP locator; access provider observes traffic. |
| Approved relays only | No direct probes, candidate disclosure, LAN broadcast or public STUN; relay operator still sees endpoint metadata. |
| Qualified multi-hop privacy | Independently operated hops plus a reviewed metadata-protection construction; two ordinary forwarding relays alone do not establish anonymity. |
| Intermittent/isolated operation | Local authorised routes and sealed store-and-forward; no automatic external discovery. |

For stronger privacy, integrate an established, separately qualified onion/circumvention transport
where available; do not invent onion cryptography or claim generic TLS traffic is unclassifiable.
[Tor's circumvention mechanisms](https://support.torproject.org/tor-browser/circumvention/) address
blocking but do not guarantee connectivity against every censor. Tor/other high-latency paths require
different real-time expectations. Cover traffic, batching and padding need measured costs and
correlation limits; direct fallback is forbidden when it would violate the chosen privacy requirement.

Peace-infrastructure deployments need independent uplinks where feasible, offline onboarding,
device-loss recovery, minimal location-bearing logs/notifications and prepared alternate bearers.
Legal/personnel status is private authorisation evidence, not a public discovery label or a technical
network protection. This design does not determine legal protected-person status or imply UN approval.
Internet software cannot restore a disconnected or jammed physical link.

## 7. Resource, packet and offline contracts

### Bounded ownership

Initial test configuration: 8 concurrent path attempts per peer, at most 3 admitted active paths
(matching the existing QSession ceiling), ordinarily 1 selected + 1 standby; 32 retained local and
32 remote candidates, with at most 64 candidate pairs after diversity-aware pruning. Return an
explicit capacity result when a profile needs more. Do not enumerate an unbounded Cartesian product.

Proposed starting timers, to be tuned in the test matrix: IPv4 eligible within 250 ms of the first
IPv6 attempt; approved relay eligible immediately; first direct-check round bounded to 3 seconds
for ordinary links; initial overall setup deadline 10 seconds. Satellite profiles require longer
deadlines. Respect ICE's own pacing/retransmission rules; these are scheduler objectives, not new
STUN timer values. Retry with capped jitter and event-driven reprobes. No permanent per-contact
polling loop for offline peers; charge standby/keepalive energy separately.

Use caller-owned slabs, packet leases and bounded event rings. State tables, socket queues,
crypto state, TLS/WebSocket libraries, buffering and error paths all require measured accounting.
An in-process mutex hub or a preallocated Vec is not proof of the Tier-1 contract. External adapter
libraries cannot be declared cold merely because setup allocates; their per-packet paths must be
measured. Isolate compatibility workers for fault containment where useful, but isolation does not
make allocation conformant. An adapter that fails the required resource profile stays unqualified.

Maintain separate complete-pass 42 MiB and host/cell memory accounts under the existing
[memory design](../core-memory-and-parallel-networking.md). Reducing active peers/windows is valid;
changing the 48-byte NQuin ABI or hiding scratch beyond the pass ceiling is not. Reserve control,
rekey and cancellation capacity before accepting bulk transfer. Memory/work exhaustion must yield
backpressure or a typed failure before allocation, not an OOM or a false send success.

### MTU correctness

Negotiate the path payload using every encapsulation layer and actual datagram limits. For direct
outer IPv6/UDP with path MTU 1280, ordinary WG data has 32 bytes of fixed overhead plus padding;
inner IPv6/UDP consumes another 48 bytes. A conservative QFrame budget is:

```text
floor((outer_path_MTU - 40 - 8 - 32) / 16) * 16 - 48
1280-byte outer IPv6 path => at most 1152 bytes of QFrame
```

QSession/QFrame headers reduce application payload further. Additional TURN/other carrier overhead
is accounted separately. The branch's 1280-byte QFrame assumption works only on sufficiently large
outer paths; it cannot be presumed safe on a minimum-MTU IPv6 underlay. Define a negotiated
transition fragmentation/chunk profile or reject paths below the supported QDNF minimum. Never
silently truncate, rely on arbitrary IP fragmentation, or assume the relay fixture's 2048-byte
slots fit all traffic. Test MTU reduction and black holes. Preserve bounded PQ handshake chunking.

### Disconnection is a delivery mode

Separate the low-latency relay from durable custody. Relay forwarding queues are short-lived and
do not promise persistence. Store authorised encrypted messages in existing Q42/core storage with
recipient/label binding, operation ID, expiry, size limit, chunk digest and durable acknowledgement.
Resume from verified chunks after reconnection; deduplicate durable effects atomically.

Expose distinct states: QueuedLocally, AcceptedByCustodian, ReceivedByEndpoint, AppliedDurably,
Expired and Unavailable. Never call a relay socket write Delivered. Revalidate grant freshness and
recipient keys before plaintext release; offline nodes cannot know fresh revocations. Long-term
sealed messages need their own reviewed envelope/key lifecycle, not a saved live-session key.

Reuse the branch's custody/DTN work subject to integration review. If interoperability with external
delay-tolerant networks is required, add a deliberate [BPv7](https://www.rfc-editor.org/rfc/rfc9171.html)
and [BPSec](https://www.rfc-editor.org/rfc/rfc9172.html) adapter. QDNF custody is not automatically
BPv7-compatible. Keep radio/BLE/satellite feasibility, bandwidth and operating constraints explicit.

## 8. Refactor and delivery sequence

Retain `qualia-peer` as the public facade and core as graph/authority/storage owner. Proposed new
implementation directories remain focused and directory-backed; reuse remote owners where present:

| Boundary | Work |
|---|---|
| `net/peer/connectivity/` | Socket-independent policy planner, bounded candidate/path tables, generations, event/timer state machine. |
| `p2p/connectivity/` | Native interface/socket driver; UDP demux; ICE/STUN/TURN adapter; proxy/WSS adapter; no second authority engine. |
| `p2p/wireguard_runtime/` | Extract socket-free tunnel driver, caller-buffered packet API, timers and authenticated source observations. |
| `p2p/social_qdnf/` | Shared port demux, checked framing/MTU, QFrame adapter; consume every packet once and dispatch to its actual service. |
| `net/qdnf/session/` | Integrate reviewed peer/profile/channel binding, recovery ownership, path validation and existing crypto owners. |
| `qualia-client-core` introduction adapter | Private invitations, versioned encoding, user verification and relay inventory import. |
| Browser adapter / relay executable | Platform-specific carriers and operation, isolated from native-independent core dependencies. |

Keep new implementation files below 500 lines and separate tests/protocol fixtures from lifecycle
drivers. The following packages extend the E00–E21 programme; they are proposed, not completed:

1. **Connectivity contract and evidence reconciliation** (E00/E04/E09/E20): reconcile Cursor branch,
   freeze peer/path/carrier API and fail-closed states. Identify which crypto, storage and memory
   requirements are actually integrated. No security claim based on source names or constant flags.
2. **Socket-free WG and real relay slice** (E03/E05/E20): caller-owned buffers, real bounded WSS
   client/server, TLS/authentication, two separate processes; same fixture bytes and negative tests.
   Initial Internet run uses privately exchanged invitations. Endpoint/operator inventory is the
   only external prerequisite for that run, not for implementing the client/server locally.
3. **IPv6/IPv4 checks and standard TURN** (E05/E09): full ICE adapter, same-socket demux, relay race,
   NAT64, MTU and generation-safe migration. Freeze draft wire details before interoperability tests.
4. **Private rendezvous and relay resilience** (E07/E16/E18): expiring encrypted records, common
   fallback relay selection, independent failures, quotas, credential rotation and no public defaults.
5. **Protected QPR integration and durable reconnect** (E01/E02/E06/E11/E13): actual remote peer
   proof, protected payload round-trip, policy revocation, crash-safe receipts and operation replay.
6. **Browser, hostile profiles and qualification** (E14/E16/E21): real TURN/WSS interoperability,
   no-direct-disclosure captures, proxy/censor cases, offline operation and deployment evidence.

Implementation selection: retain boringtun; reuse a maintained ICE/TURN implementation behind the
bounded adapter if it passes capacity, platform, security and allocation gates. Use coturn or another
qualified standard TURN server instead of writing one. Evaluate mature TLS/WebSocket libraries for
the actual target profiles. Pin reviewed versions and scan advisories before implementation.

Benchmark a separate iroh/QUIC transition adapter as the principal alternative if WG + QSession
recovery/roaming cannot meet the acceptance matrix. [iroh 1.1.0](https://www.iroh.computer/blog/iroh-1-1-0)
was released September 1 with security fixes, including relay-driven CPU exhaustion; use at least a
version containing those fixes, with a fresh advisory review. Its presence is not an audit substitute.
Private discovery, relay-only behaviour, credentials, memory and endpoint identity rotation need
explicit tests; do not rely on defaults. [Quinn](https://docs.rs/quinn/latest/quinn/) is a transport
component alternative, not a ready-made rendezvous/NAT solution.

Replace a carrier when matched measurements justify it. Do not change peer identity, QSession
authority semantics or the graph/storage API to adopt a networking library. Maintain native
independent conformance as a separate build and test path. This avoids a permanent fork of the
whole application architecture while leaving a concrete route to a broader transport rewrite.

## 9. Acceptance matrix and remaining deployment inputs

| Test | Required evidence |
|---|---|
| Working global IPv6; IPv4 also present | Direct authenticated IPv6 selected when comparable; packet capture shows actual outer family. |
| Advertised but black-holed IPv6 | IPv4/relay scheduled promptly; no family-wide timeout before fallback. |
| IPv4 NAT, double NAT, CGNAT, destination-dependent egress pool | Bounded checks, real endpoint-to-endpoint exchange through direct or relay path; no success from STUN alone. |
| Two separate restricted networks; inbound UDP denied | Both endpoints initiate outbound relay connections; protected bidirectional exchange actually received. |
| One peer v6-only, other v4-only; NAT64 variants | Dual-stack relay succeeds; prefix discovery and numeric-bootstrap limitations tested. |
| UDP blocked; WSS allowed | WSS carries WG ciphertext; QSession proof and data/receipt tests pass. |
| Upgrade blocked; explicit proxy; captive portal; TLS interception | Correct supported alternate or typed unavailable; certificate validation intact; no false success. |
| Relay A dies, clients originally use different relays | Reach common permitted B or qualified inter-relay route; retain operation IDs; show reconnection delay. |
| Wi-Fi to cellular, suspend/resume, address rotation | Stale completions rejected; no unauthorised route activation or duplicate durable effect. |
| Continuous unrelated chat while QDNF receives | Bounded per-tick work; shared demux loses neither service's packets. |
| Path MTU 1280; encapsulation overhead; mid-session MTU decrease | Valid packet sizes or negotiated chunks; no indefinite black-hole retransmission. |
| Malicious candidate list, replayed invitation, fake relay, forged ACK | No arbitrary scan/reflection, impersonation, state advance or payload release. |
| Relay-only profile | Captures show no direct candidate probes, public STUN, mDNS or unintended DNS/relay destinations. |
| Browser behind TURN; native endpoint/gateway | Real DTLS/SCTP-to-bearer integration; gateway cannot decrypt QSession payload. |
| Loss, reordering, duplication, high RTT and relay head-of-line stalls | Congestion fairness and bounded queues; measured recovery; explicit loss of real-time capability where applicable. |
| Flood, queue exhaustion, oversize frames, malformed authenticated peers | CPU/work/byte limits enforce; control service survives; memory returns after cancellation/error. |
| Offline custody, crash at each commit, grant revoked before release | Durable recovery and explicit expiry; no duplicate effect or stale-authority disclosure. |
| P3/P4 routing/cover unavailable | ProfileUnavailable or sealed defer; no automatic weaker path. |

Measure time to first authenticated application byte and durable receipt separately; report p50,
p95, p99, failures, direct-path fraction by network class, interruption during failover, bytes/energy
per useful byte and peak memory/CPU. Proposed lab objectives: within 3 seconds when a permitted
healthy relay is already reachable on a low-RTT network, and within 5 seconds to recover via a warm
standby after detected path failure. Record cold bootstrap and detection time separately. These are
targets for tests, not current measurements or global availability promises.

Run deterministic state-machine/fuzz/allocation tests, real socket process tests, controlled NAT
emulation, then independent physical-network tests on Windows/Linux/mobile/browser targets. Compare
against a relay-assisted reference and QUIC on the same loss/RTT/traffic budget. Gate release on
actual received bytes, peer proof, policy enforcement and durable effects; environment-unset no-op
tests and functions returning `false`/`true` are not Internet evidence.

Deployment inventory still needed: responsible operator(s), verified WSS URLs/certificates, TURN
URIs and relay port ranges, identity/credential issuing authority, intended regions/failure domains,
first two test endpoints and the selected mission protection policy. This is operational information
that cannot be inferred from the repository. No server was purchased, deployed, advertised or
contacted for a live peer test during this design review.
