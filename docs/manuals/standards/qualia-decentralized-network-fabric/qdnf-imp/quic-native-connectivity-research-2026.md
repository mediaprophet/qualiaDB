# A QUIC-native alternative to ICE and TURN for Qualia

## Executive decision

For a new Internet connectivity profile, prefer **authenticated relay-assisted QUIC, with encrypted address discovery, transport-integrated direct-path upgrades, and HTTP-based relay access that can fall back to TCP**. Prefer IPv6 among acceptable paths, but never make working IPv4 or relay connectivity wait for exhaustive IPv6 attempts. Preserve an explicitly selected relay-only mode and durable, end-to-end sealed delivery when no permitted live route exists.

This is a credible alternative to using the ICE/STUN/TURN protocols in native Qualia peers. It is not an elimination of discovery, coordination, reachability checks or relay infrastructure. Those functions remain necessary. The opportunity is to give the transport accurate knowledge of its paths, consolidate duplicated machinery, and place privacy and authorization ahead of network activity.

The proposed Qualia-specific contribution is now specified as the
**Capability-Scoped Connectivity Protocol (CSCP)**, Internet-Draft
[draft-webcivics-cscp-00](../../../../standards/ietf/draft-webcivics-cscp-00.md): intent, disclosure,
expiring private contact, admitted relay resources, path evidence and
durable receipts under one bounded supervisor. CSCP is a working document,
not an RFC. Sections 10–11 examine DCUtR, Pkarr and Holepunch and define
this composition. Novel protocols may be defined; they are written as
Internet-Drafts intended for IETF progression only when they are
state-of-the-art and transformationally better for a human-centric Internet.

The most relevant contemporary components are QUIC Address Discovery, the emerging QUIC NAT-traversal work, Multipath QUIC, and MASQUE's **bound UDP** extension. They have different maturity levels; they must not be presented as one finished, universally interoperable standard. The recommendation is a qualification programme around these components, not a claim that the existing Qualia code already implements them.

This changes the greenfield recommendation in the earlier [Internet connectivity proposal](internet-peer-connectivity-architecture.md). That proposal preserves WireGuard and adds traversal around it as an incremental migration. With permission to reconsider the architecture, make QUIC the preferred new Internet transport, retain WireGuard where a VPN/IP interface is actually required, and avoid nesting both by default. Neither proposal changes the requirement that native QDNF remain independent of Internet dependencies.

## 1. Scope and evidence

Standards maturity was assessed on **10 September 2026**. Sources below distinguish published RFCs, Internet-Drafts, implementation reports and empirical research. Vendor deployment statements demonstrate implementation activity, not independently established reliability for Qualia or humanitarian missions. Recommendations, proposed interfaces and acceptance criteria are this report's engineering synthesis, not requirements imposed by the cited standards.

The local baseline remains `0.0.37` at `79231d7d`. The separately reviewed Cursor proposal and implementation are on `origin/0.0.38`, including `dc6462d8` and `7a275039`. Fetching and reviewing those commits did not merge them or qualify their runtime. This report is documentation only; it does not implement a transport, deploy relays or certify security.

The target is native desktop/edge peers, an explicitly different browser profile, and intermittent or hostile access networks. Protection means enforceable disclosure and cryptographic requirements, not a claim of anonymity or guaranteed connectivity. No protocol can deliver live traffic through a total network outage, and endpoint compromise remains outside what transport encryption alone can repair.

## 2. What Gemini gets right—and what needs correcting

### Consolidate the stack, but do not confuse protocols with functions

Removing SDP from a native data-only application is reasonable, but this does not require replacing ICE. The ICE specification deliberately separates its connectivity procedure from the signalling used to exchange information. A compact authenticated native signalling format could coexist with ICE. Thus “no SDP” is not, by itself, a benefit unique to QUIC. [1](https://www.rfc-editor.org/rfc/rfc8445.html)

Likewise, STUN is not inherently an unencrypted-only protocol: its specification includes TLS-over-TCP and DTLS-over-UDP operation. The attractive change is integrated, encrypted address observation using an already relevant transport—not a claim that authenticated discovery was previously impossible. [2](https://www.rfc-editor.org/rfc/rfc8489.html)

### Connection IDs do not open NAT mappings

QUIC connection IDs help route packets to a connection independently of the current endpoint tuple. They are not peer identities or authorization credentials. RFC 9000 explicitly says its path-validation mechanism is not designed as NAT traversal and that effective traversal needs additional synchronization. NATs and firewalls still decide whether packets reach the endpoint. [3](https://www.rfc-editor.org/rfc/rfc9000.html)

Consequently, “both peers blast QUIC packets at all candidates” is not a complete design. It omits authenticated coordination, roles, probe pacing, stale-address handling, amplification limits and a distinction between receiving a probe and validating a usable data path. Moving those responsibilities into a transport implementation is valuable; pretending they disappear is dangerous.

### MASQUE replaces a relay protocol, not the relay function

RFC 9298 CONNECT-UDP asks a proxy to forward UDP to a specified target. An ordinary HTTP server or CDN is not automatically such a proxy. A successful HTTP response establishes the proxy association, not proof that a remote application can be reached. The protocol is not restricted to HTTP/3. [4](https://www.rfc-editor.org/rfc/rfc9298.html)

TURN also supports TLS/TCP between client and server; TCP 443 is not a capability unique to MASQUE. Whether either service works depends on actual network policy and server deployment. [5](https://www.rfc-editor.org/rfc/rfc8656.html)

HTTP/3 uses UDP, which can be blocked while ordinary HTTPS over TCP remains available. Worse for the “virtually impossible to block” claim, a USENIX Security 2025 paper documents deployed, selective SNI-based QUIC censorship. Its observations began in April 2024. Encryption of application packets does not make the protocol or all connection metadata indistinguishable from arbitrary browsing. [6](https://www.usenix.org/conference/usenixsecurity25/presentation/zohaib)

The defensible claim is narrower: HTTP-based proxying can reuse useful authentication, deployment and transport infrastructure. It is neither universally permitted nor censorship-proof.

## 3. The actual 2026 frontier

| Component | Verified maturity | Decision for Qualia |
|---|---|---|
| QUIC transport/TLS and QUIC DATAGRAM | Published RFCs 9000, 9001 and 9221 | Foundation for a new Internet profile |
| HTTP Datagrams/Capsules and CONNECT-UDP | Published RFCs 9297 and 9298 | Standards-based relay-access foundation |
| QUIC Address Discovery, `draft-ietf-quic-address-discovery-01` | WG draft, 15 August 2026 | Negotiate and version-gate; not a universal feature |
| n0 NAT traversal, `draft-bruynooghe-n0-quic-nat-traversal-00` | Individual informational draft, July 2026 | Implementation-informed experimental extension, requiring security review |
| Multipath QUIC, `draft-ietf-quic-multipath-21` | RFC Editor queue, not yet a published RFC at review | Qualify exact negotiated implementation/version |
| Bound UDP, `draft-ietf-masque-connect-udp-listen-16` | RFC Editor queue, August 2026 revision | Preferred MASQUE capability for unreachable peers |
| QUIC-aware proxying, `draft-ietf-masque-quic-proxy-09` | WG Last Call, July 2026 | Optional later optimization, not baseline |

Sources for the draft rows: [7](https://www.ietf.org/archive/id/draft-ietf-quic-address-discovery-01.html), [8](https://datatracker.ietf.org/doc/html/draft-bruynooghe-n0-quic-nat-traversal-00), [9](https://datatracker.ietf.org/doc/draft-ietf-quic-multipath/), [10](https://datatracker.ietf.org/doc/draft-ietf-masque-connect-udp-listen/), [11](https://datatracker.ietf.org/doc/draft-ietf-masque-quic-proxy/).

### Encrypted observation is useful, but still an observation

QUIC Address Discovery introduces an `OBSERVED_ADDRESS` frame for a path's reflexive address. Its security discussion explicitly warns that observers can report incorrect addresses. Authentication identifies the observer; it does not prove the observation is true. [7](https://www.ietf.org/archive/id/draft-ietf-quic-address-discovery-01.html)

Our integration must obtain observations on the actual UDP socket used for direct connectivity. A coordination connection over TCP does not reveal that socket's UDP mapping. Nor should an observation of a relayed path be mistaken for the client's access-network endpoint. Store observer, local socket generation, interface, address family, timestamp and expiry alongside every candidate. Treat agreement between observers as evidence, not proof of universal reachability.

### NAT traversal inside QUIC exists, but is not just base QUIC

The n0 draft starts from a pre-existing relayed peer connection, exchanges candidates, coordinates role-specific probes, and subsequently validates a new path. It describes implementation behaviour rather than a completed normative standard. It also explicitly documents active-CID reuse contrary to RFC 9000's linkability guidance and leaves security considerations incomplete. A successful small probe is not full path validation. [8](https://datatracker.ietf.org/doc/html/draft-bruynooghe-n0-quic-nat-traversal-00)

This is a release gate, not proof that every later implementation retains the documented defect. Inspect and test the exact chosen revision. Do not claim a privacy property from a diagram while the selected transport sends linkable probes.

The implementation lead worth evaluating is **noq**, which n0 describes as a Quinn-derived QUIC implementation powering iroh, with address discovery, NAT traversal and first-class relay/direct paths. Its important architectural contribution is per-path transport awareness rather than opaque underlay switching. These are the maintainers' implementation claims, not this report's independent certification. [12](https://www.iroh.computer/blog/noq-announcement)

### Recent publication is not necessarily recent measurement

Trautwein and colleagues' 2026 DCUtR study reports over 4.4 million attempts across 85,000-plus networks and a conditional hole-punch success rate of approximately 70% ± 7.1%, after relay reservation and discovery succeed. TCP and QUIC results were statistically similar in that experiment. Importantly, the measurement campaign ran **December 2022–January 2023**. This is substantial evidence against magical transport-level success claims, but not a measurement of the September 2026 Internet or of noq/MASQUE. [13](https://arxiv.org/html/2604.12484v1)

Do not compare that conditional statistic directly with a vendor's headline connection rate. Different populations, prerequisites and definitions can make such comparisons meaningless.

## 4. Recommended architecture

### Separate semantic identity from transport implementation

Use a versioned Internet profile with these ownership boundaries:

| Owner | Responsibility | Must not imply |
|---|---|---|
| Qualia authority/session layer | Peer identity, grants, disclosure policy, cryptographic profile, operation semantics | A socket or TLS connection grants application authority |
| Connection supervisor | Admit paths; bound work; select permitted carriers; manage network generations | It can manufacture successful transport events |
| QUIC engine | Peer transport handshake, streams/datagrams, path validation, recovery and congestion behaviour | Every peer is authorized for every service |
| MASQUE adapter | Explicit proxy association, datagram/capsule framing, quotas and cancellation | Proxy TLS is end-to-end peer authentication |
| Rendezvous/resolution | Deliver expiring, authenticated invitations and connection hints | A public directory of protected persons is necessary |
| Core durable store | Sealed operations, idempotency, expiry, retry and receipts | A transport acknowledgement proves application commit |

Use QUIC streams for ordered reliable service flows and negotiated QUIC DATAGRAM for deadline-sensitive unreliable data. RFC 9221 does not retransmit lost DATAGRAM frames, although they remain congestion controlled. [14](https://www.rfc-editor.org/rfc/rfc9221.html)

Refactor QSession's Internet mapping rather than running a second complete packet recovery/congestion protocol over reliable QUIC streams. Preserve QSession's identity, authority, protection and service semantics; specify which functions the QUIC profile supplies. Native non-IP QDNF keeps its own required transport machinery. This is a proposed profile change requiring an architecture decision and conformance updates—not permission to silently alter existing wire semantics.

### Establish useful connectivity before optimizing it

The proposed sequence is:

1. Admit the intended peer, purpose, privacy profile, operator set and resource budget before discovery or probing.
2. Resolve a private invitation or relationship-scoped rendezvous record. Bind transport credentials to the intended peer, not merely the rendezvous provider.
3. Start approved relay access promptly. If policy permits and a plausible direct address is already known, race a bounded direct connection attempt; prefer IPv6 without waiting for it indefinitely.
4. Establish the end-to-end peer QUIC connection and the required Qualia authentication/protection layer. Application delivery remains gated on the service grant.
5. When allowed and negotiated, perform coordinated direct-path discovery and upgrades while the relay carries useful traffic.
6. Use a direct path only after the transport has validated it and the supervisor confirms its policy, generation and payload budget. Retain a bounded standby when warranted.
7. On loss, recover through another permitted path or reconnect and resume durable operations. If none is allowed, report offline/queued rather than silently weakening protection.

This scheduling borrows the principle of racing address families from Happy Eyeballs, not an assumption that its precise timers are optimal for every mission network. Keep delays configurable and measure them under loss, satellite latency and power constraints. [15](https://www.rfc-editor.org/rfc/rfc8305.html)

A direct bootstrap race may create a separate connection from the relayed bootstrap. Deduplicate them at the authenticated logical-session boundary; never merge unrelated QUIC packet-number or key spaces. For an already established connection, use only the migration/multipath behaviour that both engines actually negotiated.

### Give both NATed peers reachable relay endpoints

Plain target-based CONNECT-UDP leaves a missing step when neither endpoint is publicly reachable. The bound-UDP extension provides proxy-owned public addresses and multi-target operation associated with a stable binding; it also defines mechanisms to restrict accepted peer traffic. That is the relevant MASQUE building block, not merely serving HTTP/3 on port 443. [10](https://datatracker.ietf.org/doc/draft-ietf-masque-connect-udp-listen/)

Our proposed two-proxy topology is:

```text
          outbound HTTPS                       outbound HTTPS
Peer A =================> Relay A <---------- Relay B <================= Peer B
                              public UDP between relays

       <---------------- end-to-end peer QUIC ---------------->

Peer A <......... optional validated direct IPv6/IPv4 .........> Peer B
```

Each endpoint obtains a binding at an approved proxy. The peers privately exchange the proxy addresses, arrange peer restrictions, and carry end-to-end QUIC packets through those bindings. Both access legs are outbound; neither client needs a public listener. Relay-to-relay UDP must actually work, including the chosen address-family pairing. A shared operator may offer a more efficient internal circuit, but that is an additional service contract, not a property of arbitrary MASQUE servers.

The adapter must map each relayed transport path unambiguously to its binding and generation. Its socket abstraction must preserve the addressing assumptions of the selected QUIC engine. Do not hide changing relay routes behind one fake stable path while reusing stale path measurements.

This topology still needs maintained servers, public addressing, stable binding state, authentication, capacity, abuse controls and operator accountability. A CDN is eligible only if its supported product exposes and permits the required proxy behaviour. “Runs at the edge” is not a substitute for that capability check.

### A real TCP fallback, not just another UDP service

HTTP Datagrams can use the Capsule Protocol when an unreliable HTTP/3 datagram mapping is unavailable, including over HTTP/2. This gives a standards-based route to TCP-backed proxy access. It does not preserve the loss behaviour of unreliable datagrams. [16](https://www.rfc-editor.org/rfc/rfc9297.html)

For this profile, require and test bound-UDP proxy access over both HTTP/3 and HTTP/2/TLS/TCP. On known UDP blockage, start the latter immediately; otherwise use a short bounded race. An H3-only MASQUE implementation does not meet the fallback requirement. Validate the necessary CONNECT/capsule capabilities explicitly rather than assuming every HTTP/2 intermediary permits them.

Inner peer QUIC over reliable capsules is a degraded compatibility path: TCP head-of-line blocking and nested recovery can hurt latency under loss. Preserve correctness, cap queues and tune deadlines; do not advertise it as equivalent to direct UDP. If that degradation is unacceptable, a separately versioned QSession-over-authenticated-stream profile is a possible later optimization, with reconnection rather than a claim of seamless QUIC path migration.

If bound UDP is unsupported, remain on a separately qualified paired stream relay or another explicitly configured compatibility carrier. Label the protocol honestly. If all allowed relays are blocked, use durable offline delivery; do not rotate into unapproved operators or disclosure modes automatically.

## 5. Security and hostile-environment operation

### Disclosure policy precedes path selection

Define at least three explicit operating profiles:

| Profile | Network activity permitted | Failure behaviour |
|---|---|---|
| Direct permitted | Approved discovery and peer address disclosure; IPv6/IPv4 upgrades; relays | Use the first qualified path within policy |
| Relay only | Approved proxy access; no direct candidate export, direct probing or automatic upgrade | Alternate approved relay, then queue/report unavailable |
| Disconnected/restricted | Only specifically admitted local or deferred bearers | Sealed durable operations; no unsolicited Internet discovery |

In relay-only mode, the remote peer must not receive access-network IP addresses through candidate metadata, logs, debug events or fallback routines. The access proxy still sees its client's source address, and infrastructure can observe timing and volume. Two operators do not automatically establish anonymity, non-collusion or resistance to a global observer. If stronger traffic-analysis resistance is required, specify and independently evaluate a separate privacy bearer rather than naming MASQUE an anonymity system.

Do not publish a stable device identifier, personal DID, contact graph or protected-role label in a public DHT. Use scoped rendezvous capabilities and encrypted relationship records, with bounded retention and revocation handling. Public discovery may serve public infrastructure, not silently become the default for protected persons.

### Authenticate the actual endpoints and bind the layers

QUIC uses TLS for transport security, but peer authentication depends on the chosen credential arrangement. Its early-data mode also has replay implications. [17](https://www.rfc-editor.org/rfc/rfc9001.html)

For Qualia, require a reviewed binding between the peer's transport key, session transcript, intended peer identity, negotiated profile and authorization context. A self-signed certificate accepted without identity binding is not enough. Nor are a CID, FNV/q_hash identifier, proxy credential or relay circuit number substitutes for a cryptographic peer identity.

Keep application payloads end-to-end protected even when browser or proxy connections terminate at infrastructure. If a protection profile requires post-quantum confidentiality, verify the **peer-to-peer** key establishment and stored-envelope protection; a post-quantum outer client-to-proxy tunnel is insufficient. Preserve existing mandatory QSession protection until an audited replacement provides equivalent or stronger properties. Do not invent a new cipher or hybrid construction in the traversal layer.

Disable early data for grants, mutations, acknowledgements with irreversible effects and sensitive discovery in the initial release. Reconnection must not replay an operation into a second authorization context. Proxy-controlled responses must never be able to downgrade the peer's minimum protection profile.

### Treat peers and relays as potential resource adversaries

An authenticated peer can still advertise somebody else's address. Apply destination policy, reject inappropriate loopback/multicast/link-local or internal targets unless explicitly admitted, bound candidate counts and probe bytes, and expire stale network generations. Reachability proof and authorization are separate checks. Relay bindings need allocation quotas, idle expiry, destination restrictions, rate limits and cancellation.

Current dependencies also need scrutiny: iroh's 1.1.0 announcement describes fixes involving a malicious-relay CPU loop, address deserialization panic and misdirected relay datagrams. Use that disclosure to seed regression tests and require a reviewed, patched revision; do not infer safety merely from a stable version number. [18](https://www.iroh.computer/blog/iroh-1-1-0)

## 6. Performance, MTU and bounded resources

The architectural performance hypothesis is that a path-aware transport can avoid duplicated recovery machinery and preserve useful service while better paths are discovered. It is not a promise of universal millisecond setup. Cold relay authentication, rendezvous, peer authentication, loss and propagation delay still contribute. Measure time to the first **authorized application response**, not just HTTP success or a UDP probe.

Nested transports remain real. Elmenhorst and colleagues' ANRW 2025 work studies ACK cascades in multi-hop MASQUE and finds measurable overhead despite practical aggregation reducing the theoretical worst case. Its testbed supports measuring nesting costs, not projecting a universal penalty to this proposed two-proxy layout. [19](https://www.comsys.rwth-aachen.de/publication/2025/2025_elmenhorst_masque-cascading-acks/2025_elmenhorst_masque-cascading-acks.pdf)

Begin with one preferred data path and bounded warm standby. Do not stripe indiscriminately across mobile, satellite and terrestrial links. Admit extra paths only for a stated resilience/throughput benefit and energy budget; preserve each path's own transport state. Multipath support alone does not choose the best application scheduling policy. [9](https://datatracker.ietf.org/doc/draft-ietf-quic-multipath/)

QUIC-aware MASQUE forwarding can avoid some re-encapsulation/re-encryption, but changes the observable transport behaviour and introduces further negotiation. Evaluate it separately after the encapsulated baseline; do not make an experimental optimization necessary for protected connectivity. [11](https://datatracker.ietf.org/doc/draft-ietf-masque-quic-proxy/)

### Packet size is a release gate

An Internet path carrying QUIC must accommodate its minimum UDP payload requirement of 1,200 bytes. [3](https://www.rfc-editor.org/rfc/rfc9000.html) An outer IPv6 MTU of 1,280 does **not automatically prove** that encapsulating an inner 1,200-byte packet will fit: subtract every actual outer header, authentication tag and datagram/context field.

Define the adapter's maximum payload as the minimum of negotiated transport limits and measured path capacity, minus exact encapsulation overhead. Use packetization-layer MTU discovery and black-hole handling rather than relying only on ICMP feedback. [20](https://www.rfc-editor.org/rfc/rfc8899.html)

If that budget is below the required inner packet size, use a qualified reliable-capsule path, another suitable path, or an explicitly specified bounded segmentation/reassembly layer. Do not silently truncate packets or assume UDP proxy fragmentation. Large service objects belong on bounded streams/chunks. The prior WireGuard profile's size calculation is not the QUIC profile's size calculation.

### Respect Qualia's two-tier contract

Do not treat a general-purpose async networking library as zero-heap because its caller API looks small. Its TLS buffers, stream reassembly, timers, task queues, certificate processing and dependency allocations all need accounting.

Keep the Sentinel/ABI boundary caller-buffered and allocation-measured. The complete Sentinel pass remains bounded by 42 MiB, including referenced state; preserve the 48-byte NQuin ABI. A separately admitted Internet adapter must have an explicit resource contract and cannot become an exemption for hot paths. The ordinary cell ceiling is not a substitute for per-pass enforcement.

Admit memory, active handshakes, candidate slots, streams, retransmission/reassembly bytes, relay associations, queued operations and work per scheduling tick. Stop before limits are exceeded, reserve control capacity, and prove cleanup on cancellation, failure and unwind. If noq or another library cannot meet the applicable deployment budget, it does not qualify for that target; a constrained native bearer or separately admitted gateway remains necessary.

## 7. Browser and offline profiles

The browser WebTransport API provides application streams and datagrams to a server endpoint; it is not an API for arbitrary native UDP sockets and transport-level NAT probing. The W3C specification therefore does not establish browser support for this entire native design. [21](https://www.w3.org/TR/webtransport/)

Use WebTransport to an approved gateway where supported, with real TLS-protected WebSocket/HTTPS fallback as separately tested browser carriers. Keep sealed Qualia payloads opaque to the gateway. A browser cannot be assumed to issue arbitrary MASQUE CONNECT requests through ordinary `fetch`, or to participate in the native extension negotiation merely because it supports HTTP/3.

If direct browser-to-browser WebRTC interoperability is a requirement, retain an explicit ICE/TURN compatibility profile. An ICE/TURN-free native architecture does not remove that interoperability obligation.

Offline delivery belongs above all carriers. Persist an operation identifier, content digest, intended authority context, expiry and encrypted content through core storage. On retry or restart, distinguish an identical duplicate from the same identifier carrying different content. A receipt must state whether data was received, validated or durably committed. Test restart, rollback and replay; an in-memory queue is not durable storage. Revalidate authority and expiry before releasing previously queued data.

## 8. Build-versus-adopt decision and migration

Prefer adopting a maintained QUIC engine over writing QUIC, TLS or congestion control from scratch. Evaluate noq first for the integrated traversal/path model, but keep it behind a Qualia-owned boundary. Do not replace Qualia authority, resolution or storage wholesale with iroh's application model. Equally, do not assume that selecting noq automatically supplies an interoperable MASQUE adapter.

Google's upstream QUICHE MASQUE client contains bound-UDP-related implementation code. It is a useful candidate for an independent interoperability target, not proof that a random installed binary supports the exact draft revision or Qualia profile. Freeze and record the tested revision on both sides. [22](https://quiche.googlesource.com/quiche.git/+/refs/heads/main/quiche/quic/masque/masque_client_session.cc)

Structure implementation as directory-backed modules with separate owners for policy, lifecycle, QUIC integration, proxy framing, rendezvous, browser mapping, receipts and test infrastructure. Keep cold configuration and hot execution distinct. Replace the earlier scaffold's synthetic completion paths rather than treating their test counts as protocol qualification: real certificate validation, full-width checked framing, genuine peer-session authentication, actual connectivity checks and durable restart tests are required.

Recommended gates, not estimated completion claims:

| Gate | Deliverable | Evidence required before proceeding |
|---|---|---|
| A: profile decision | Versioned QSession-to-QUIC mapping and resource/threat contracts | No duplicated recovery owner; no weakening of native QDNF or authorization |
| B: relay baseline | Real authenticated peer QUIC over H3 and H2/capsule proxy paths | Two separate machines behind restrictive NATs; genuine certificates and endpoint credentials |
| C: direct upgrade | Same-socket discovery and negotiated traversal | Packet captures, role tests, usable-payload validation, linkability review, privacy-mode negative tests |
| D: resilience | Family changes, relay failure, expiry, resume and mobility | No unauthorized delivery, bounded recovery, real crash/restart evidence |
| E: qualification | Dependency review, interoperability, fuzzing, resource and field tests | Reproducible artifacts and independent review of security-sensitive boundaries |

Do not wait for a speculative new traversal RFC to obtain useful relay connectivity. Conversely, do not make a young traversal extension a mandatory dependency for every protected session. Extension failure should leave a permitted relay path usable, not trigger a protection downgrade.

## 9. Comparative evaluation and acceptance

Use the same machines, workloads, regions and network conditions for the QUIC profile and a genuine mature ICE/TURN baseline. Include the incremental WireGuard profile only once it actually works; loopback scaffolding is not a fair comparator. Neither a new protocol name nor a lower line count establishes superiority.

The matrix must include IPv6-only, IPv4-only, dual stack with broken IPv6, translation environments, endpoint-dependent mappings, restrictive filtering, blocked UDP, permitted HTTP/2 with denied CONNECT, TLS interception, proxy overload, relay-to-relay UDP failure, packet reordering, MTU black holes, mobile rebinding and long outages. Inject malicious candidates, oversized frames, stale bindings, invalid credentials and duplicate identifiers with changed content.

Record distributions, not one average: cold/warm time to authorized response, connection success including failed prerequisites, time to direct upgrade, path-switch disruption, application throughput, relay fraction, bytes per delivered operation, CPU/peak memory, energy, expiry violations and unauthorized/disallowed packets. Separate results by network class. Keep diagnostics opt-in, redacted, time-bounded and distinct from preserved operational evidence.

Initial pass/fail invariants are stronger than an invented percentage target: no forbidden direct probes in relay-only mode; no payload before required peer authorization; no truncated accepted frames; no silent protection downgrade; no allocation or work-budget breach; and no duplicate committed effect after crash/retry. Set numerical availability and latency objectives with deployment owners after establishing a measured baseline and the acceptable operational risk.

The outstanding decisions are concrete: approved operators and domains; supported proxy capabilities; key custody and minimum cryptographic profiles; admissible disclosure per deployment; exact library revisions and memory budgets; browser requirements; and availability objectives. Until those are qualified, the honest result is a strong proposed architecture—not a claim of field readiness.

## 10. Decentralized alternatives: what to borrow and what not to assume

### DCUtR, Identify, AutoNAT and Circuit Relay v2

DCUtR's specification supports both TCP and QUIC direct upgrades. It uses an existing relayed connection for synchronization; if upgrading fails, peers can continue using the relay. A successful upgrade creates a direct connection and prioritizes new streams there; existing long-lived streams need recreation when the old connection closes. It is not intrinsically seamless migration of one QUIC connection. [23](https://raw.githubusercontent.com/libp2p/specs/master/relay/DCUtR.md)

Circuit Relay v2 is not restricted to signalling. It bridges streams into a relayed connection, with optional duration and data limits and explicit reservations. Relays can refuse reservations or shed load. A public peer can offer this role, but public reachability alone does not commit its bandwidth or enable the service. [24](https://github.com/libp2p/specs/blob/master/relay/circuit-v2.md)

Identify communicates endpoint information and an observed address; it is not omniscient NAT-map exchange. AutoNAT v2 tests individual addresses through dial-back. Neither provides an authoritative, permanent statement that all peers can reach the same address. Reachability varies with observer, time, destination and transport. [25](https://raw.githubusercontent.com/libp2p/specs/master/identify/README.md), [26](https://raw.githubusercontent.com/libp2p/specs/master/autonat/autonat-v2.md)

Borrow relay-assisted synchronization, explicit reservations and address-specific evidence. Do not import libp2p as the replacement runtime: that conflicts with this programme's independence requirement. Its protocols remain useful comparators. Also reject the claim that ICE/TURN necessarily dictate one centralized operator; protocol choice and operator topology are different decisions.

### Pkarr and Mainline DHT

Pkarr publishes signed DNS records under Ed25519 public-key addressing, using Mainline DHT or HTTP relay backends. Its own documentation describes ephemeral records, periodic republication and browser HTTP gateways. It decentralizes a naming/publication function; it does not itself perform NAT traversal. [27](https://github.com/pubky/pkarr)

There is also an important correction about iroh: its current address-lookup documentation says DNS/Pkarr publication to a hosted server is enabled by default, while Mainline DHT lookup is opt-in. Signed Pkarr records can describe relay URLs, not just direct IP/port. Using Pkarr does not mean every deployment is using the DHT. [28](https://docs.iroh.computer/concepts/address-lookup)

BEP 44's mutable records use signatures and increasing sequence numbers, and a value larger than 1,000 bytes cannot be assumed to be accepted. [29](https://www.bittorrent.org/beps/bep_0044.html) Design implications: a signature does not ensure freshness or availability; replicated caches cannot guarantee erasure; and larger proof material needs an explicitly authenticated, bounded retrieval mechanism. Do not substitute a short lookup hash for the actual authority proof.

For protected users, publicly publishing a stable key-to-address association creates an avoidable correlation surface. Encryption of record contents can hide fields but does not automatically hide publication, lookup patterns or the publisher's network address. Calling it “serverless” means relying on other participants' infrastructure, bootstrapping and continued availability—not eliminating those dependencies. The target key must still be obtained authentically.

Use public Pkarr/DHT records for deliberately public infrastructure. For private relationships, default to invitation-bootstrapped, scoped rendezvous records, with optional DHT access only when the disclosure profile permits it. A UDP DHT is not the emergency fallback for a network that blocks UDP.

### HyperDHT, Pear and blind peering

Pear's own connection guide states that HyperDHT hole punching fails when both endpoints have randomizing NATs and that HyperDHT does not relay by default. It cites Keet's separate participant-relay implementation as an example. Its HyperDHT reference also exposes bootstrap configuration. This directly contradicts the notion that embedding traversal in a runtime removes every middle-server requirement. [30](https://docs.pears.com/how-to/connect-to-peers/connect-two-peers-by-key-with-hyperdht/), [31](https://docs.pears.com/reference/building-blocks/hyperdht/)

Hypercore replication is a separate function from establishing a path. Pear's blind-peering guide explicitly describes configured, always-on peers that retain and seed encrypted data without the read capability. Useful availability requires somebody to retain the bytes and stay reachable. [32](https://docs.pears.com/how-to/blind-peering/keep-data-available-with-blind-peering/)

Borrow the separation between a contact key, a transport session and encrypted replicated content. For Qualia, relaying and retaining others' data must be explicit admitted roles with consent, quotas, funding/operation responsibilities and cancellation. Do not silently turn protected endpoints into public relays or repositories.

## 11. What Qualia should actually design

### Start with irreducible responsibilities

The design needs six primitives regardless of the chosen ecosystem: authenticate the intended peer; locate a current contact route; establish bidirectional reachability; carry protected traffic; reserve scarce intermediary resources; and preserve delivery semantics across disconnection. No DHT, CID, signed record or relay satisfies all six.

Rather than invent another packet handshake, define a small set of composable control objects in the existing QDNF programme:

| Proposed object | Binds together | Deliberately does not prove |
|---|---|---|
| Connection intent | Intended peer, purpose, authority reference, disclosure constraints, deadline and resource budget | That a peer is online or reachable |
| Contact descriptor | Scoped contact key, generation, expiry, permitted locator or opaque rendezvous reference | That a returned address works or is safe to probe |
| Relay lease | Operator/circuit identity, admitted byte/time limits, expiry, allowed participants and renewal terms | That the operator cannot fail or observe metadata |
| Path evidence | Actual transport event, endpoint pair, network generation, freshness, payload limit and observed performance | Application authority or universal future reachability |
| Delivery receipt | Operation and content identity, authority context, accepted/committed state and replay information | That an unrelated operation or later context is authorized |

These are proposed logical records, not frozen Rust layouts or new cryptographic primitives. Reuse existing authority, lease, QResolve and durable-operation owners. Store large proofs through bounded core-owned references; preserve the NQuin ABI rather than attempting to pack an entire certificate into it.

Path evidence is primarily local state produced by the trusted transport integration. A peer's signed assertion that a path works remains an assertion. Distinguish authenticated remote reports from locally observed validation. Hide constructors for successful states behind verified transport/session events; a public `advance(SessionReady)` escape hatch would defeat the model.

### Select feasible paths before scoring them

The connection supervisor first excludes anything violating disclosure, cryptographic, operator, authority or resource requirements. Only then may it compare latency, reliability, energy and cost among the remaining candidates. No weighted score may trade a forbidden IP disclosure for a faster response.

Implement the policy kernel as a bounded transition function over supplied events, time snapshots and caller-owned state. The external network is nondeterministic; the kernel's response to the same admitted inputs should be reproducible. Expire evidence on network-generation changes, policy changes and timers. Cancellation must revoke pending work as well as current paths.

The application asks to contact an authorized peer, not to dial a provider-specific address. Qualified backends can supply private mailboxes, public infrastructure DHT records, direct IPv6/IPv4, MASQUE or other explicit compatibility carriers. Native QDNF remains another independent bearer. Adding a backend must not enlarge the set of permitted disclosures.

### Decentralize responsibilities, not merely directory storage

Support independently administered rendezvous and relay providers, with admitted capacity and documented failure domains. Replicate expiring encrypted contact descriptors through permitted providers, retaining enough last-known information for temporary discovery failure. A single shared bootstrap, DNS dependency, cloud account or billing suspension can correlate apparently independent failures; record these dependencies explicitly.

For relationship-private contact, begin with an authentically exchanged invitation. It can authorize access to an opaque mailbox or relationship-scoped descriptor rather than publish the user's permanent identity. Bind generations and expiry to a reviewed freshness policy, support key rotation and revoked invitations, and never claim that rotating names defeats traffic analysis. Use established signatures, AEAD and KDF constructions; the exact private-discovery wire format still needs cryptographic review and interoperability vectors.

Relay leases provide a place to express reliable service obligations and resource admission; signatures make commitments attributable, not physically guaranteed. Custody leases for stored encrypted operations are separate from live relay leases. A provider may support both, but receipt and deletion semantics must not be conflated.

### A bounded, falsifiable invention programme

The valuable new work is the integration of semantic authority, disclosure-aware discovery, admitted resources and verifiable local state transitions across carriers. The constituent ideas already exist; establishing publication or patent novelty is outside this review. Success should mean measurable correctness and deployability, not a new acronym.

Before implementing a new wire extension, demonstrate that existing QUIC/MASQUE extension points and an application control stream cannot express the requirement. Then specify the smallest missing operation, its threat model, canonical encoding, version negotiation, failure semantics and bounds. Never make custom cryptography or undocumented engine behaviour the foundation.

Add four decisive experiments to §9: a compromised discovery provider returning valid but stale descriptors; a relay lease expiring during transfer; a network switch while direct probing is forbidden; and an offline mutation replayed after its grant is revoked. The supervisor must preserve authority and disclosure constraints in every case. That is where a Qualia-specific design can improve meaningfully on “dial this public key and hope a path appears.”

## Conclusion

The best alternative is **QUIC-native, policy-controlled connectivity with relay-assisted establishment and opportunistic direct upgrade**. Adopt the mature standards core, negotiate the developing extensions, retain a TCP-accessible relay route, and make protection and durable semantics independent of which network path wins.

The most important improvement over Gemini's blueprint is a capability-scoped connection fabric: a complete separation of what a transport can prove, what an operator must provide, and what Qualia must authorize—supported by bounded execution and measured failure behaviour. Invent this composition where the requirements demand it; reuse and qualify the transport and cryptographic primitives underneath it.

## Sources

1. A. Keränen, C. Holmberg and J. Rosenberg. [Interactive Connectivity Establishment (ICE), RFC 8445](https://www.rfc-editor.org/rfc/rfc8445.html). IETF, July 2018.
2. M. Petit-Huguenin et al. [Session Traversal Utilities for NAT (STUN), RFC 8489](https://www.rfc-editor.org/rfc/rfc8489.html). IETF, February 2020.
3. J. Iyengar and M. Thomson, editors. [QUIC: A UDP-Based Multiplexed and Secure Transport, RFC 9000](https://www.rfc-editor.org/rfc/rfc9000.html). IETF, May 2021; especially §§5.1, 8.2, 9.5 and 14.
4. D. Schinazi. [Proxying UDP in HTTP, RFC 9298](https://www.rfc-editor.org/rfc/rfc9298.html). IETF, August 2022.
5. T. Reddy, A. Johnston, P. Matthews and J. Rosenberg. [TURN, RFC 8656](https://www.rfc-editor.org/rfc/rfc8656.html). IETF, February 2020.
6. Ali Zohaib et al. [Exposing and Circumventing SNI-based QUIC Censorship of the Great Firewall of China](https://www.usenix.org/conference/usenixsecurity25/presentation/zohaib). USENIX Security, August 2025.
7. M. Seemann and C. Huitema. [QUIC Address Discovery, draft-ietf-quic-address-discovery-01](https://www.ietf.org/archive/id/draft-ietf-quic-address-discovery-01.html). IETF working-group draft, 15 August 2026.
8. F. Bruynooghe. [draft-bruynooghe-n0-quic-nat-traversal-00](https://datatracker.ietf.org/doc/html/draft-bruynooghe-n0-quic-nat-traversal-00). Individual informational Internet-Draft, July 2026; especially §§3 and 5.
9. Y. Liu et al. [Managing multiple paths for a QUIC connection, draft-ietf-quic-multipath-21](https://datatracker.ietf.org/doc/draft-ietf-quic-multipath/). March 2026 revision; RFC Editor queue at review.
10. D. Schinazi and A. Singh. [Proxying Bound UDP in HTTP, draft-ietf-masque-connect-udp-listen-16](https://datatracker.ietf.org/doc/draft-ietf-masque-connect-udp-listen/). August 2026 revision; RFC Editor queue at review.
11. T. Pauly, E. Rosenberg and D. Schinazi. [QUIC-Aware Proxying Using HTTP, draft-ietf-masque-quic-proxy-09](https://datatracker.ietf.org/doc/draft-ietf-masque-quic-proxy/). 6 July 2026; WG Last Call at review.
12. n0 team. [noq, noq, who's there?](https://www.iroh.computer/blog/noq-announcement). 19 March 2026. Primary implementation report, not an independent benchmark.
13. D. Trautwein, C. Ihle, M. Schubotz, C. Breitinger and B. Gipp. [Large-Scale Measurement of NAT Traversal for the Decentralized Web: A Case Study of DCUtR in IPFS](https://arxiv.org/html/2604.12484v1). arXiv:2604.12484v1, 14 April 2026. Measurement campaign: December 2022–January 2023.
14. T. Pauly, E. Kinnear and D. Schinazi. [An Unreliable Datagram Extension to QUIC, RFC 9221](https://www.rfc-editor.org/rfc/rfc9221.html). IETF, March 2022.
15. D. Schinazi and T. Pauly. [Happy Eyeballs Version 2: Better Connectivity Using Concurrency, RFC 8305](https://www.rfc-editor.org/rfc/rfc8305.html). IETF, December 2017.
16. D. Schinazi and L. Pardue. [HTTP Datagrams and the Capsule Protocol, RFC 9297](https://www.rfc-editor.org/rfc/rfc9297.html). IETF, August 2022.
17. M. Thomson and S. Turner, editors. [Using TLS to Secure QUIC, RFC 9001](https://www.rfc-editor.org/rfc/rfc9001.html). IETF, May 2021.
18. n0/iroh. [iroh 1.1.0 — Security fixes](https://www.iroh.computer/blog/iroh-1-1-0). September 2026 release announcement.
19. K. Elmenhorst, M. Kühlewind, I. Kunze, C. Sander and K. Wehrle. [Cascades of Nested Acknowledgments in Multi-Hop MASQUE](https://www.comsys.rwth-aachen.de/publication/2025/2025_elmenhorst_masque-cascading-acks/2025_elmenhorst_masque-cascading-acks.pdf). ANRW, 22 July 2025. DOI: 10.1145/3744200.3744773.
20. G. Fairhurst et al. [Packetization Layer Path MTU Discovery for Datagram Transports, RFC 8899](https://www.rfc-editor.org/rfc/rfc8899.html). IETF, September 2020.
21. W3C. [WebTransport](https://www.w3.org/TR/webtransport/). Living specification snapshot consulted 10 September 2026; browser API evidence, not universal implementation support.
22. Google QUICHE contributors. [MASQUE client session source](https://quiche.googlesource.com/quiche.git/+/refs/heads/main/quiche/quic/masque/masque_client_session.cc). Moving upstream source inspected 10 September 2026; a pinned interoperability revision remains a qualification deliverable.
23. libp2p contributors. [Direct Connection Upgrade through Relay specification](https://raw.githubusercontent.com/libp2p/specs/master/relay/DCUtR.md). Active revision r1, 20 November 2021; current source consulted 10 September 2026.
24. libp2p contributors. [Circuit Relay v2 specification](https://github.com/libp2p/specs/blob/master/relay/circuit-v2.md). Active revision r3, 28 February 2023; current source consulted 10 September 2026.
25. libp2p contributors. [Identify specification](https://raw.githubusercontent.com/libp2p/specs/master/identify/README.md). Current source consulted 10 September 2026.
26. libp2p contributors. [AutoNAT v2 specification](https://raw.githubusercontent.com/libp2p/specs/master/autonat/autonat-v2.md). Working draft r2, 15 April 2023; current source consulted 10 September 2026.
27. Pubky/Pkarr contributors. [Pkarr project documentation](https://github.com/pubky/pkarr). Current repository documentation consulted 10 September 2026.
28. n0/iroh. [Address Lookup](https://docs.iroh.computer/concepts/address-lookup). Current documentation consulted 10 September 2026; distinguishes DNS/Pkarr default from optional Mainline DHT.
29. A. Norberg and S. Siloti. [BEP 44: Storing arbitrary data in the DHT](https://www.bittorrent.org/beps/bep_0044.html). Created 19 December 2014; displayed revision last modified 1 February 2017.
30. Holepunch/Pear. [Connect two peers by key with HyperDHT](https://docs.pears.com/how-to/connect-to-peers/connect-two-peers-by-key-with-hyperdht/). Current documentation consulted 10 September 2026.
31. Holepunch/Pear. [HyperDHT reference](https://docs.pears.com/reference/building-blocks/hyperdht/). Documentation for v6.33.2 consulted 10 September 2026.
32. Holepunch/Pear. [Keep data available with blind peering](https://docs.pears.com/how-to/blind-peering/keep-data-available-with-blind-peering/). Current documentation consulted 10 September 2026.
