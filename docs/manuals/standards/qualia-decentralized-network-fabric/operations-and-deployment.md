# QDNF Operations and Deployment

**Status:** Normative design 0.1

## 1. Purpose

This specification describes how a QDNF network is formed, operated, upgraded, repaired, and
observed. It is intentionally explicit about the difference between a native QDNF deployment and a
transition deployment. A native deployment can form and provide local services without ARP, NDP,
DHCP, SLAAC, IP, ICMP, BGP, DNS, TCP, UDP, TLS, a certificate authority, or a cloud rendezvous
service. A transition deployment may use those systems as a carrier while preserving QDNF identity
and policy semantics.

The Legacy Internet Gateway is a separate role and trust boundary. Its failure, compromise, or
absence must not prevent native peers from discovering one another, resolving native resources, or
using native services within reachable QDNF topology.

## 2. Operational principles

Operators and implementations must preserve these invariants:

- physical reachability is not identity;
- a relationship is not blanket trust;
- discovery does not imply admission;
- route availability does not imply authorization;
- cryptographic validity does not imply social truth;
- native resolution never silently falls through to DNS;
- a legacy gateway cannot mint native identity or rewrite native policy;
- all mutable control objects have an issuer, scope, sequence, and bounded lifetime or explicit
  monotonic revocation behavior;
- administrative operations are signed, attributable, and subject to least privilege;
- loss of wall-clock time reduces privilege instead of widening it; and
- networks continue locally through partitions whenever policy permits.

## 3. Node roles

A process or device may implement more than one role, but permissions are independent.

| Role | Function | Authority it does not automatically receive |
|---|---|---|
| endpoint | runs user services and QSessions | routing, admission, gateway, or registry authority |
| link router | forwards within a realm | identity issuance or inter-realm export |
| realm gateway | exchanges authorized realm paths | permission to inspect QSession plaintext |
| rendezvous helper | relays bounded introductions | power to authenticate the introduced peer |
| resolution cache | stores verified RARs/resources | power to alter or vouch for cached records |
| bootstrap introducer | supplies initial signed artifacts | permanent administrative control |
| constitution custodian | participates in realm/network changes | unilateral change unless constitution says so |
| recovery guardian | participates in key/account recovery | routine content or session access |
| time witness | signs bounded time evidence | global clock authority |
| relay | forwards opaque authenticated traffic | service authorization or decryption |
| observability collector | receives minimized metrics/receipts | private payload or credential access |
| legacy gateway | proxies explicitly requested Internet operations | native resolver/root or transparent fallback |
| archive | retains authorized signed history/content | live signing or routing authority |

Role assignment is expressed in signed capabilities and constitution records. Running a particular
binary, listening on a bearer, or possessing a route does not grant the role.

## 4. Deployment classes

### 4.1 Native standalone

All QFrames travel over native bearers such as raw Ethernet, IPC, direct L2CAP, serial, or a
radio-frame adapter. There is no Internet requirement. Installation media contains the software,
base protocol registry, trust anchors chosen by the participants, and optional invitation packages.

### 4.2 Native with optional legacy gateway

Native operation is complete locally. One or more LIG nodes expose explicit legacy services. The
gateway may itself use IP, DNS, TLS, and HTTP externally; native endpoints do not need those stacks
to communicate with it over QDNF.

### 4.3 Transition mesh

QFrames use UDP, WireGuard, libp2p, or WebRTC as a carrier between sites or during migration. The
deployment depends operationally on the carrier for those paths and must report that dependency.
Native identifiers, QResolve proofs, QPolicy decisions, and QSession authentication remain in force.

### 4.4 Hybrid

At least one path is native and at least one path is transition. Routing policy can prefer native
paths and can prohibit transition carriers for restricted or classified traffic.

## 5. Installation artifacts

A reproducible installation set contains:

- exact QDNF software artifact digests;
- the initial protocol registry bundle/digest;
- supported native and transition bearer profiles;
- supported cryptographic suites;
- a local hard-limit manifest;
- an empty or explicitly initialized local policy store;
- schema/test-vector versions;
- operator recovery instructions; and
- no mandatory vendor or public-network credential.

Default installation creates a fresh node key set from an operating-system cryptographic random
source. Images must not ship cloned private keys. Device imaging occurs before key generation or
runs a mandatory first-boot rekey.

## 6. Node initialization

The first boot state machine is:

```text
Uninitialized
  -> EntropyVerified
  -> DeviceKeysCreated
  -> LocalDIDBound
  -> BearersEnumerated
  -> LocalPolicyLoaded
  -> IsolatedReady
  -> [InvitationImported | OpenNetworkCandidate]
  -> Admitted
  -> Operational
```

At `IsolatedReady`, local IPC services can operate and the node can display/import invitation
material. It has not joined a network. It advertises no stable DID in unauthenticated beacons.

Initialization records public key references, encrypted secret-key handles, software measurements
where available, boot counter, registry digest, and configuration digest. Secret keys remain in the
strongest available hardware or OS key store; export requires explicit recovery policy.

## 7. Creating a new network

Network creation is an explicit local ceremony, not a claim to a globally unique name.

### 7.1 Inputs

Creators select:

- a network constitution document;
- initial custodian DIDs and decision threshold;
- accepted DID and resource verification profiles;
- realm creation/admission rules;
- cryptographic suites and deprecation policy;
- maximum control-plane and data-plane resource budgets;
- sensitivity/export rules;
- human review and appeal paths;
- recovery and emergency isolation rules;
- optional display alias, which is not an identifier; and
- whether any open discovery or public services are permitted.

### 7.2 Identifier derivation

The network identifier is derived from the immutable constitution genesis digest and creator
commitment as defined by QRoute. Each creator verifies the rendered constitution and identifier on
an independent display where feasible. Signatures form the genesis ratification object.

### 7.3 Genesis package

The content-addressed genesis package contains:

- constitution and human-readable summary;
- network ID derivation inputs;
- initial custodian verification methods;
- initial registry/profile digests;
- initial realm definitions;
- resource ceilings;
- recovery and amendment procedures;
- ratification signatures; and
- test evidence that the package parses within bounds.

It may be copied by QR code, removable media, direct cable, local QLink exchange, or a legacy
download. The retrieval channel is not trusted; the verified digest and signatures are.

## 8. Creating a realm

A realm is a bounded routing and policy domain within a network. Its constitution names parent
network, realm derivation nonce, administrators, admitted node classes, route export rules, metrics,
service policy, and change process. The realm ID is derived from the signed definition.

At least one admitted router can originate a link-state advertisement, but small two-node realms do
not require a dedicated router. A realm can operate completely disconnected from every other realm.

## 9. Invitation package

Private networks use a single-use or limited-use invitation package:

```cddl
invitation = {
  0: 1,
  1: bstr .size 32,             ; network identifier
  2: bstr .size 32,             ; realm identifier
  3: bstr .size 32,             ; constitution/genesis digest
  4: bstr .size 32,             ; invitation ID
  5: bstr,                      ; rendezvous secret or sealed contribution
  6: [1*8 bstr],                ; bounded bootstrap RAR/resource set
  7: uint,                      ; granted onboarding operation class
  8: uint,                      ; issue time/evidence class
  9: uint,                      ; expiry or monotonic use bound
  10: uint,                     ; maximum uses
  11: tstr .size (1..512),      ; inviter verification method
  12: bstr                      ; signed envelope
}
```

The package does not contain the invitee's eventual long-lived private key. The invitee generates
its own keys and proves possession during admission. Invitation secrets are encrypted at rest,
displayed only with explicit action, and destroyed or marked used after acceptance according to
policy.

## 10. Native bootstrap without ARP, DHCP, IP, or DNS

On a shared raw-Ethernet bearer, the node performs:

1. open the interface by local OS device identity;
2. install the QDNF EtherType receive filter;
3. derive an epoch-scoped ephemeral link ID;
4. emit bounded QLink beacons to the QDNF discovery destination MAC or bearer broadcast primitive;
5. match public service tags or private rendezvous tags locally;
6. complete QLink challenge/proof with an observed-link transcript;
7. validate network/realm invitation or open-admission evidence;
8. obtain signed constitution and route objects from the peer;
9. originate/receive link-state advertisements;
10. compute routes and QResolve sources; and
11. open QSession only after persistent target verification and QPolicy negotiation.

No step asks for a MAC address through ARP, obtains an IP lease, performs router solicitation,
queries a DNS server, or contacts an Internet bootstrap host.

On point-to-point bearers, steps 3–5 use the already observed adjacent endpoint. On IPC, the adapter
also binds the OS peer principal into the QLink transcript.

## 11. Admission

Admission is a stateful policy transaction:

```text
Candidate -> LinkVerified -> InvitationVerified -> IdentityPresented
          -> RequirementsEvaluated -> [HumanReview]
          -> Admitted | Deferred | Denied
```

Evaluation verifies invitation use/expiry, key possession, accepted DID method, requested roles,
device posture where required, guardian/consent rules, and conflicts or blocks. An admission result
grants precise capabilities and route/service scopes. It is not a universal trust assertion.

The candidate receives the decision and public/minimized reason. Private safety information and
other members' credentials are not disclosed. Human-review paths expose the exact requested powers
and consequences in comprehensible language.

## 12. Open-network admission

An open network may allow unauthenticated discovery and a bounded join request. It still requires:

- proof of return path before amplification;
- rate limits before expensive verification;
- a fresh node key;
- explicit acceptance of the network constitution;
- limited initial capabilities;
- separation of guest and administrative route scopes; and
- an abuse/blocking mechanism.

Proof-of-work is not mandatory and must not become a wealth/energy gate. A realm may use proof of
rate limit, invitation, local physical presence, social attestation, or human moderation.

## 13. Daily start and convergence

After restart a node:

1. verifies local configuration and registry digests;
2. increments persistent boot identity/sequence state;
3. marks cached routes tentative;
4. starts native bearers before optional transition carriers;
5. re-establishes QLink adjacencies;
6. validates fresh LSAs/RPAs and withdrawals;
7. recomputes deterministic routes;
8. refreshes expiring RARs and policy objects;
9. resumes only replay-safe sessions/operations; and
10. advertises services after their local authorization state is ready.

Stale cached data can help select a peer but cannot bypass current signature, expiry, revocation, or
policy checks. A reboot must not reset replay protection into accepting old control messages.

## 14. Time operation

QDNF distinguishes:

- monotonic local duration;
- signed wall-clock evidence;
- boot/sequence order;
- Lamport order for semantic operations; and
- time confidence.

Native networking does not depend on public NTP. A node may learn time from a hardware clock,
multiple authorized time witnesses, a paired device, or an explicitly configured legacy service.
Time evidence records source, uncertainty, observation monotonic time, and signature.

If wall-clock confidence is insufficient:

- short local durations continue using monotonic time;
- expired data remains expired;
- not-yet-valid privileged grants do not activate;
- long-lived cached authorization becomes `challenge` or `needs_human`;
- key rotation can use signed monotonic succession; and
- administrators receive a bounded clock-confidence alert.

Resource accounting uses monotonic durations with clock/boot identity; Lamport sequence is never
billable time. Quotes name their wall-clock confidence requirement. A clock correction or restart
must not add work duration, renew a spend grant, or repeat a payment. See the
[energy/time accounting rules](./commons-and-resource-economics.md#3-energy-and-time-units).

## 15. Routing operations

Routers maintain bounded adjacency, LSA, RPA, forwarding, and replay tables. Operators configure
policy through signed realm profiles, not ad hoc unsigned route injection.

Routine tasks include:

- inspecting adjacency identity and bearer type;
- confirming LSA/RPA age and verification status;
- comparing selected routes with deterministic policy reasons;
- identifying partitions and flapping links;
- rotating routing keys;
- draining a router before maintenance;
- withdrawing a failed/exported path; and
- verifying that sensitivity ceilings never widen along a route.

A route advertisement is rejected if its origin is unauthorized, sequence regresses, path loops,
signature fails, lifetime exceeds policy, or resource limits are exceeded.

## 16. Service publication

A service operator creates a signed resource or DID service binding, service version, capability
requirements, sensitivity class, and one or more RARs. Publication can be distributed by QSync,
QResolve providers, local peers, removable media, or a legacy mirror.

Before advertising, the node verifies that:

- the service process is healthy and bound to the named local principal;
- the controller key is authorized for service publication;
- route profiles are actually supported;
- no private bearer locator is exposed beyond policy;
- service resource ceilings are configured; and
- revocation/withdrawal can be issued if the service stops.

Stopping a service first withdraws or shortens its RAR, then drains sessions, then releases local
resources. Abrupt failure is bounded by advertisement expiry.

## 17. QResolve operations

Resolution providers index independently verifiable records. They do not become authoritative merely
by caching them. Operators monitor:

- record verification failures;
- stale/expired ratio;
- query amplification and rate limits;
- privacy class of requested identifiers;
- negative-cache lifetime;
- DHT bucket health where used; and
- consistency between advertised route profiles and routing availability.

A native miss returns `not_found` or `temporarily_unreachable`. It never triggers DNS. A user or
application must explicitly invoke the LIG for an Internet name.

## 18. Legacy Internet Gateway deployment

The LIG is installed as a distinct service principal, process boundary, and ideally host/network
segment. It has:

- an explicit native service URI;
- narrowly scoped `qdnf:op:bridge-legacy` capabilities;
- separate native and legacy caches;
- a legacy-side DNS/IP/TLS stack;
- egress allow/deny and content-size policy;
- origin evidence capture/minimization rules;
- no network/realm administration key; and
- a visible audit and disable control.

Native applications send a structured legacy request naming scheme, host, port policy, method,
purpose, expected content constraints, and whether redirects are allowed. The gateway performs the
legacy operation, returns payload plus bounded provenance, and labels the result `legacy-origin`.

Gateway DNS responses never populate QResolve. Web PKI authenticates the Internet endpoint only; it
does not establish a native DID binding unless a separate, verified signed binding exists.

## 19. Multiple gateways

A realm can operate multiple independent LIGs. Selection considers operator identity, egress policy,
privacy jurisdiction, supported legacy functions, cost, and availability. It is not transparent load
balancing: the selected gateway identity and policy are included in user consent and the QSession
transcript.

Gateways must not share request histories unless separately authorized. A failed gateway may be
replaced for reachability failures; a policy denial is not retried through another gateway to evade
policy.

## 20. Remote-site transition deployment

Where raw native reachability is unavailable, sites can exchange QFrames over an explicit transition
carrier. Deployment records:

- carrier type and endpoint acquisition dependency;
- whether DNS is required to find the carrier endpoint;
- carrier credentials and rotation owner;
- maximum encapsulation MTU;
- privacy exposure of outer metadata;
- availability assumptions; and
- which QDNF sensitivity classes may use the path.

QLink still authenticates the QDNF adjacency, QRoute still controls scope, and QSession still
authenticates the final target. Carrier security is defense in depth, not a substitute.

## 21. Rural, disaster, and off-grid deployment

A resilient kit can contain small routers, direct-radio/serial adapters, power storage, printed or QR
genesis digests, and signed invitation packages. Formation proceeds without Internet or public time.

Operational policy should favor:

- store-and-forward QSync with explicit expiry;
- energy-aware route metrics;
- small advertisements and longer stable lifetimes;
- delay-tolerant service profiles;
- local time witnesses and monotonic sequencing;
- removable-media transfer with full signature/content verification;
- human-readable device and realm labels that do not act as security identifiers; and
- safe reconciliation after partitions.

Delayed links must not be misclassified as malicious solely for latency. Replay and revocation rules
still apply when bundles arrive much later.

## 22. Mobile personal subnet

A phone, vehicle, or wearable cluster can form a mobile realm/subnet. The current gateway advertises
a summarized route without exposing every attached device. On movement:

1. establish a new QLink and validate the new realm path;
2. send QSession path challenge/response;
3. enforce whether the session's sensitivity allows the new path;
4. move traffic only after validation;
5. retain the old path briefly where policy and resources allow; and
6. withdraw the prior route without rotating persistent identity.

Loss of one gateway does not renumber services. Node and service identity remain cryptographic,
while bearer/link identifiers rotate independently.

## 23. Community mesh deployment

A community mesh should distribute rather than centralize operational powers:

- multiple constitution custodians with threshold changes;
- multiple routers and optional gateways;
- independent resolution caches;
- participant-controlled identity/recovery choices;
- transparent route/export policies;
- accessible blocking, safety, and appeal processes;
- bounded observability without message surveillance; and
- offline copies of genesis, registry, recovery, and software artifacts.

Social graph edges cannot automatically grant routing administration. Vulnerable-user and bilateral
micro-commons paths apply paraconsistent isolation and consent rules rather than collapsing disputed
claims into a global reputation score.

## 24. Data center and campus deployment

QDNF can run beside conventional Ethernet/IP on the same physical switches using its assigned or
experimental EtherType during development. Operations must verify switch behavior for unknown
EtherTypes, frame size, VLAN filtering, multicast/broadcast containment, and storm control.

Native QDNF topology does not use VLAN numbers as identity. A VLAN or physical segment may constrain
the bearer, while QLink/QRoute still perform authenticated admission and forwarding. Administrative
automation uses signed capabilities and deterministic configuration artifacts.

## 25. Key rotation

Routine rotation uses overlapping validity:

1. create the replacement key in an approved store;
2. publish a signed succession statement from the current authorized key/threshold;
3. update DID/resource/realm records;
4. wait for bounded distribution evidence or policy threshold;
5. originate fresh route/service records using the new key;
6. stop issuing with the old key;
7. drain or rekey active sessions; and
8. revoke/archive the old key according to policy.

Link and session ephemeral keys rotate independently and more often. Compromise rotation follows the
incident process and may skip overlap.

## 26. Software and protocol upgrades

An upgrade artifact is content-addressed, signed by authorized release keys, reproducibly described,
and verified before execution. Operators stage it through:

```text
Available -> Verified -> CompatibilityChecked -> Staged
          -> Canary -> RealmCohort -> General -> OldVersionRetired
```

Compatibility checks cover QFrame, protocol features, registry digest, cryptographic suites, state
migration, and resource ceilings. Mixed-version periods are time-bounded. Unsupported critical
features do not downgrade; nodes continue the old safe profile or refuse the operation.

Rollback is permitted only to a non-revoked artifact compatible with current persistent state and
security policy. A signed emergency disablement can prohibit a vulnerable old version.

## 27. Configuration management

Configuration is divided into:

- local device policy;
- signed network constitution;
- signed realm profile;
- role/capability grants;
- service resource configuration;
- transition-carrier secrets/endpoints; and
- LIG legacy-side policy.

Precedence is fail-closed: local hard safety limits and explicit user blocks cannot be widened by a
remote profile. Conflicts are surfaced with provenance. Effective configuration has a deterministic
digest and can be reconstructed from versioned source objects without logging secrets.

## 28. Observability model

QDNF provides useful operations evidence while minimizing identity and social-graph exposure.

### 28.1 Metrics

Recommended aggregate metrics include:

- bearer up/down and MTU class;
- authenticated adjacency count by bearer class;
- handshake success/failure by coarse stable reason;
- LSA/RPA accepted/rejected/expired counts;
- route convergence time and partition state;
- QResolve result class and verification failure count;
- QSession RTT/loss/congestion aggregates;
- resource-budget rejection count;
- policy outcomes by coarse class;
- clock confidence; and
- LIG requests by destination category only where consented.

Metrics must not use stable peer DID, private alias, destination, or relationship context as a
default label. High-cardinality identity labels are disabled.

### 28.2 Logs

Logs are structured, bounded, access-controlled, and retention-limited. They record event ID,
monotonic time, optional confidence-bounded wall time, component, coarse peer/session pseudonym,
outcome, and source-object digest. They exclude secret keys, rendezvous secrets, plaintext
credentials, chat/content payloads, and exact private locators.

### 28.3 Receipts

Security-sensitive administrative and policy actions may create signed receipts. A receipt names the
actor capability, target digest, action, outcome, policy digest, and time evidence. It does not copy
all evaluated private evidence. Receipt disclosure follows its own capability policy.

Economic receipts retain energy/time evidence states, accepted contract and semantic-bundle digests,
funding allocation, and contribution/settlement status. Keep `reserved`, `pending`, `unknown`,
`settled`, `disputed`, and `refunded` distinct in operator views. Never interpret a dispatch `Sent`
status or successful HTTP submission as final settlement without adapter-specific verification.

## 29. Health checks

Health is layered so a carrier outage is not confused with identity or policy failure:

| Layer | Healthy evidence |
|---|---|
| Q0 bearer | local interface operational and frames transmitted/observed |
| QLink | authenticated adjacency and replay window progressing |
| QRoute | current verified route and bounded convergence |
| QResolve | records verified with acceptable freshness/provenance |
| QPolicy | policy inputs current; evaluation within budget |
| QSession | handshake/data/ACK progression within selected profile |
| service | service-specific signed or authenticated probe |
| LIG | explicit legacy request succeeds; does not affect native status |

A system can therefore report “native network healthy; Internet gateway unavailable” rather than
“network down.”

## 30. Backups and recovery

Backup categories are kept separate:

- public genesis/constitution/registry artifacts;
- signed routing, resource, and governance history;
- encrypted application state and CRDT operations;
- local configuration;
- key-recovery material; and
- audit receipts.

Private keys are not placed in ordinary backups unless the chosen recovery design explicitly uses
encrypted export with threshold control. Backups are content-verified, encrypted, retention-bounded,
and regularly restored in an isolated test environment.

Recovery rebuilds public state first, restores keys or completes the guardian process, verifies
revocation/succession history, then republishes fresh RARs/routes. Restoring a filesystem snapshot
must not roll sequence numbers or replay windows backward; persistent monotonic state is advanced.

## 31. Lost device and identity recovery

Upon a lost or compromised device:

1. locally block the affected keys/identifiers immediately;
2. use independent channels to reach recovery guardians/custodians;
3. publish signed revocation or recovery initiation;
4. withdraw route and service advertisements;
5. terminate or reauthorize sessions;
6. rotate affected relationship/rendezvous secrets;
7. issue replacement capability grants with least privilege;
8. synchronize recovery evidence across reachable partitions; and
9. reconcile late partitions without resurrecting revoked authority.

Recovery does not erase the former DID's history or silently transfer every relationship. Each
capability issuer may require re-consent.

## 32. Incident response

Incident classes include key compromise, malicious route injection, resolver poisoning, replay or
resource exhaustion, abusive peer behavior, vulnerable software/suite, LIG compromise, privacy leak,
and governance-key loss.

The response lifecycle is:

```text
Detect -> Bound -> PreserveMinimalEvidence -> Isolate -> Revoke/Withdraw
       -> Restore -> Verify -> Communicate -> Review -> Amend
```

Containment actions are precisely scoped: close an adjacency, deny a key, withdraw a route, disable
an extension, pause a service, or isolate a gateway. An incident in the LIG should not shut down
native QLink/QRoute unless shared host compromise makes that necessary.

Human-impact incidents include a clear notice, contest/appeal mechanism where safe, and preservation
of contradictory claims in isolated contexts rather than destructive overwrite.

## 33. Denial-of-service operations

Nodes enforce staged cost:

1. frame length/magic/version checks;
2. bearer and source rate limiting;
3. cookie/return-path proof;
4. bounded deterministic parsing;
5. cheap replay/expiry lookup;
6. signature/key validation;
7. policy evaluation; and
8. allocation of session/service state.

Unauthenticated senders receive no large response. Per-source, per-adjacency, per-realm, and global
budgets prevent one peer from exhausting the 42 MB Sentinel or process limits. Overload sheds
optional discovery and diagnostics before established safety-critical services, subject to explicit
realm policy.

## 34. Resource planning

Each deployment documents limits for:

- interfaces and bearer queues;
- neighbors per interface;
- LSAs/RPAs and computed routes;
- DHT buckets/providers;
- simultaneous handshakes;
- sessions, streams, and retransmission bytes;
- fragment/reassembly slots;
- verified resource records and negative cache;
- QPolicy evaluation frames;
- QSync pending operations;
- observability buffers; and
- LIG concurrent requests/response bytes.

Hot-path buffers are caller-owned/fixed and allocation-free under the QualiaDB Tier 1 contract.
Cold construction may use bounded workspace arenas and must fail before exceeding configured or 42
MB execution-pass ceilings.

For governed services also budget energy, elapsed/device time, battery reserve, economic negotiation,
semantic-bundle expansion, accounting storage, monetary spend, and unsettled exposure. Document
meter/model scope and unknown telemetry. Concurrent sessions share atomic reservations; reaching
a limit pauses new billable work while preserving a small closure/reconciliation allowance.

## 35. Administrative access

Administration is a QDNF service protected by a dedicated capability, not an ambient shell implied
by network location. High-impact actions can require M-of-N signatures and a human-readable preview.

Remote administration should support:

- read-only status distinct from mutation;
- exact target and action digest;
- short-lived session-bound capabilities;
- no wildcard grant by default;
- signed result receipt;
- emergency local physical denial; and
- recovery when remote administration is unavailable.

Legacy SSH/HTTPS management, if retained during transition, belongs to the legacy/host management
plane and cannot be the sole way to operate a native network.

## 36. Partition behavior and reconciliation

During partition, realms continue with their last verified constitution and locally available policy.
Operations that require absent signers remain suspended rather than lowering thresholds. Records are
ordered by Lamport clocks and CRDT rules where applicable; authorization and governance changes still
require their original proofs.

Upon reconnection:

1. authenticate the new adjacency/path;
2. exchange summaries and revocations first;
3. synchronize constitutions/policies/registry deprecations;
4. reconcile routes and resource records;
5. exchange application operations under QPolicy;
6. isolate contradictions for paraconsistent review; and
7. release suspended transactions only after required signatures exist.

Last-writer-wins must not resolve key ownership, consent, constitutional authority, or human-safety
conflicts merely by clock magnitude.

## 37. Decommissioning

### 37.1 Node

Withdraw routes/services, drain sessions, revoke role capabilities, synchronize final operations,
export authorized records, destroy local secrets, and retain only policy-required receipts.

### 37.2 Realm

A constitution-authorized closure record defines route withdrawal, service/data disposition,
participant notification, appeal/recovery window, and archival custodians. Other realms stop
accepting paths after verified closure or expiry.

### 37.3 Legacy gateway

Remove `bridge-legacy` capabilities, withdraw its service RAR, drain requests, clear DNS/HTTP/TLS
caches according to policy, destroy credentials, and verify that native clients return an explicit
gateway-unavailable result rather than falling back elsewhere.

## 38. Usability requirements

User interfaces distinguish:

- person/service display name from cryptographic identifier;
- direct, relayed, transition, and legacy-origin paths;
- reachable from authorized;
- signature valid from claim accepted;
- permanent block from temporary mute or route failure;
- native-not-found from Internet-name request; and
- automated allow/deny from a decision requiring human review.

Before a sensitive connection, the interface explains the target, requested action, information to
be disclosed, path/gateway class, expiry, and revocation mechanism. Fingerprint-only workflows are
an expert fallback; QR/contact verification and relationship context reduce human comparison errors.

## 39. Conformance reporting

Every deployed node emits a local, optionally signed manifest containing:

- implementation/version/artifact digest;
- QFrame and protocol versions;
- active registry/profile digests;
- native bearers actually available;
- transition bearers actually available and their dependencies;
- cryptographic suites and disabled algorithms;
- supported services/extensions;
- hard resource limits;
- time sources/confidence policy classes;
- enabled node roles; and
- LIG presence as a separate component.

The manifest contains no secret, private locator, or member list. A build with only UDP/WireGuard
transport says `native_bearers: []` and cannot claim native-underlay independence.

## 40. Deployment acceptance tests

A native release is not accepted until the following succeed with external IP connectivity disabled
and no DNS/DHCP service available:

1. two fresh nodes initialize with unique keys;
2. a network/realm is created or invitation imported from local media;
3. peers discover and authenticate over a native bearer;
4. routes converge without IP addresses;
5. a DID/resource is resolved from signed native records;
6. a QSession opens after persistent key and capability verification;
7. chat/QSync/content service data transfers;
8. restart preserves replay/sequence safety;
9. link failure selects another verified native path where available;
10. partitioned operations reconcile deterministically;
11. a revoked key/route/resource remains rejected after restart;
12. native resolution miss returns without DNS traffic;
13. LIG absence leaves native services healthy; and
14. resource-exhaustion tests fail closed within configured limits.

Hybrid/transition acceptance additionally verifies explicit dependency labeling, nested
authentication, outer-metadata exposure documentation, path sensitivity policy, and continued native
operation when transition endpoints disappear.

## 41. Legacy gateway acceptance tests

The LIG must demonstrate:

- no automatic invocation from QResolve;
- DNS packets originate only from the gateway's legacy side;
- separate cache namespaces and provenance labels;
- Web PKI result not treated as DID authorization;
- capability denial not retried through another gateway;
- bounded redirects, response bytes, decompression, and execution behavior;
- SSRF/private-address and local-service protections;
- sanitized response metadata;
- gateway compromise simulation does not grant realm administration; and
- disabling all gateways produces a clear legacy-unavailable result while native tests continue.

## 42. Minimum operational runbooks

Production deployments maintain tested, offline-accessible runbooks for:

- first network/realm creation;
- invite issuance, expiry, and revocation;
- router/gateway addition and removal;
- key rotation and lost-device recovery;
- software/registry upgrade and rollback;
- clock-confidence loss;
- partition and delayed reconciliation;
- route poisoning or resolver attack;
- resource-exhaustion attack;
- privacy incident and evidence minimization;
- LIG compromise/isolation; and
- complete realm/network decommissioning.

Runbooks name required roles and thresholds, but never embed live secrets. Each is exercised on a
schedule chosen by the realm's risk profile.

## 43. Example: two-person native network

1. Alice creates a network and one realm on an offline laptop.
2. The laptop generates a constitution package and single-use invitation QR.
3. Bob's device generates its own keys and scans the package.
4. Both devices connect to the same Ethernet segment with no IP configuration.
5. Private QLink tags match; challenge/proof authenticates the invitation context.
6. Bob reviews the constitution and requests chat/share capabilities.
7. Alice approves; both nodes record the signed bilateral agreement.
8. QRoute installs the direct native path and QResolve indexes signed service RARs.
9. QSession verifies each persistent DID method and opens chat.
10. Unplugging the Internet router has no effect because it was never a dependency.

If Alice later enables a LIG, Bob sees a distinct “Internet via Alice's gateway” service and decides
whether to grant request metadata to it.

## 44. Example: three-site hybrid network

Each site has a native Ethernet realm. Sites A and B use directly configured QDNF-over-WireGuard;
site C receives delayed bundles over a radio-frame bearer. Realm gateways exchange signed RPAs.
Restricted records may use A↔B only; public synchronization can use all sites. QSession persists
service identity across path migration. If Internet connectivity fails, each site continues locally,
A↔B transition stops, and radio reconciliation continues. No native name is sent to DNS.

## 45. Readiness checklist

A deployment is ready only when operators can answer yes to all applicable questions:

- Can the native portion start from power-off without Internet, DHCP, or DNS?
- Are identities verified independently from bearer locators?
- Are network and realm constitutions available and human-reviewable?
- Are administrative, routing, relay, recovery, and gateway roles separately authorized?
- Are native and transition paths visibly distinguishable?
- Are resource ceilings enforced before expensive work?
- Are time uncertainty and partition behavior fail-closed?
- Can a participant block, revoke, recover, and appeal where policy allows?
- Are private identifiers and social-graph details absent from default telemetry?
- Can the network operate when every LIG is disabled?
- Have restore, key compromise, route attack, and gateway compromise exercises succeeded?
- Does the conformance manifest state real capabilities rather than planned ones?

### 45.1 Contract and commons service readiness

For services using the optional profiles, operators also demonstrate:

- the accepted CBOR-LD context/ontology/table/shapes/rules bundle is locally available by digest;
- quotes expose resource, monetary, fee, and unsettled-exposure caps and the party funding them;
- a donated/community-funded interaction works without any external payment adapter;
- exhausted subsidy or battery allowance pauses new work according to published capacity rules;
- receipt, dispute, cancellation, and pending-payment recovery survives a restart;
- Q42 compaction preserves signed source records, pinned contracts, and outstanding instructions;
- cache keys separate private requester scopes and the LIG, with expiry checked after cache hits;
- energy readings/estimates/unknowns and elapsed/device/human time remain distinguishable; and
- an adapter outage or ambiguous submission reconciles the original instruction without a second debit.

## 46. Operational non-goals

QDNF operations do not require a universal network operator, compulsory blockchain, public alias
registry, global reputation score, always-online time authority, mandatory cloud control plane, or
automatic Internet fallback. Local autonomy does not mean unverifiable authority: every delegated
power remains explicit, scoped, attributable, revocable where the governing agreement permits, and
bounded by participant rights and local safety controls.
