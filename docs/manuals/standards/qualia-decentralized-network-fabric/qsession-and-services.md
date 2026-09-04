# QSession and Native Services

**Status:** Normative design 0.1

## 1. Purpose

QSession provides end-to-end encrypted datagrams, reliable streams, multiplexing, flow control,
loss recovery, congestion control, rekey, and path migration over QRoute. It replaces mandatory
native dependence on UDP, TCP, and TLS while binding transport to persistent DID/resource authority
and QPolicy.

QSession borrows well-tested transport concepts, including monotonically increasing packet numbers,
ACK ranges, loss timers, and per-path congestion state. The initial recovery profile is informed by
[RFC 9002](https://www.rfc-editor.org/rfc/rfc9002.html) but is a QDNF protocol, not QUIC.

## 2. Connection identifiers

| Identifier | Scope |
|---|---|
| session ID | 128-bit random endpoint-local lookup value; changes on new session |
| peer session ID | independently chosen 128-bit value carried after handshake |
| flow ID | 64-bit QFrame demultiplexer derived from both session IDs |
| stream ID | 62-bit value encoding initiator/direction bits |
| packet number | 64-bit monotonic per direction/encryption level |
| operation/message ID | application strong digest or 128-bit random idempotency token |

Session IDs are not stable identifiers and must not enter public aliases or long-term social graphs.

## 3. Handshake modes

| Mode | Responder proof | Requester proof | Use |
|---|---|---|---|
| server-authenticated | required | capability/pseudonym only | public read/service discovery |
| mutual DID | required | required | bilateral relationships, administration |
| pairwise | pairwise controller proof | pairwise controller proof | private social mesh |
| content-only | provider route proof plus content digest | optional | immutable public content |
| resumed | prior resumption secret plus fresh route validation | policy-dependent | low-latency reconnect |

Every mode uses fresh ephemeral X25519. QLink/carrier encryption does not remove this requirement.

## 4. Handshake messages

### 4.1 SessionInit

Contains:

- protocol version, supported crypto suites, and random 256-bit nonce;
- initiator session ID and ephemeral X25519 key;
- persistent target full identifier and digest;
- selected RAR/DNI digests;
- QRoute path transcript digest;
- offered services/application versions;
- requested operation class, sensitivity ceiling, and capability digest;
- requester disclosure mode and optional key/method hint;
- anti-amplification token where required; and
- optional bounded resumption token.

### 4.2 SessionResponse

Contains responder session ID/key/nonce, selected suite/services/limits, target verification method,
RAR/DNI/path confirmation, transcript digest, controller proof, and optional fresh address-validation
token. It is encrypted under handshake keys after the ephemeral exchange where possible.

### 4.3 SessionRequestProof

When mutual authentication or capability proof is required, the requester supplies DID/controller
proof, capability/consent/delegation presentation, exact operation digest, and final transcript
confirmation.

### 4.4 SessionAccept

The responder returns selected service limits and QPolicy outcome: allow, deny, challenge, or
needs-human. Only `allow` activates application delivery. A denial may include a bounded safe reason
but never reveals private policy rules to an unauthenticated party.

## 5. Anti-amplification and replay

Before route/source validation, a responder sends at most three times the received bytes. Stateless
tokens bind QRoute source/path observation, SessionInit digest, time bucket, and a rotating secret.
Tokens expire within 30 seconds and are not transferable to another target/path.

Handshake nonces are single-use. A bounded replay store keys by target/context/requester hint and
nonce digest. Resumption tokens are encrypted, audience-bound, short-lived, one-profile objects; they
do not bypass current RAR, block, revocation, or QPolicy checks.

## 6. Encryption levels and packet spaces

Separate packet-number/key spaces:

- Initial/anti-amplification (not application-confidential; minimum content);
- Handshake;
- Application key phase 0;
- Application key phase 1; and
- optional zero-round-trip data, disabled by default.

Zero-round-trip data is prohibited for state-changing, non-idempotent, capability-changing,
privacy-sensitive, or replay-unsafe operations. An implementation may omit it entirely.

## 7. QSession packet

The QSession payload inside QFrame contains:

| Field | Size/meaning |
|---|---|
| destination session ID | 16 bytes |
| packet number | variable 1/2/4/8 bytes after header protection profile |
| key phase and flags | u8 |
| path ID | u8 |
| payload length | bounded varint |
| encrypted frames | remaining bytes |
| AEAD tag | 16 bytes |

The QFrame flow ID enables early demux; the complete session ID is authenticated inside QSession.
Unknown sessions trigger at most a stateless reset token response within amplification limits.

## 8. Session frame types

| Frame | Purpose |
|---|---|
| `PADDING` | traffic-analysis padding |
| `PING` | ack-eliciting liveness/path probe |
| `ACK` | bounded packet-number ranges and delay |
| `DATAGRAM` | unordered application message |
| `STREAM` | reliable byte range |
| `RESET_STREAM` | terminate one stream with code |
| `STOP_SENDING` | request peer cease stream transmission |
| `MAX_DATA` | connection receive-window update |
| `MAX_STREAM_DATA` | stream receive-window update |
| `MAX_STREAMS` | stream-count update |
| `PATH_CHALLENGE` / `PATH_RESPONSE` | validate migration/multipath candidate |
| `KEY_UPDATE` | coordinate key phase |
| `SERVICE_OPEN` / `SERVICE_CLOSE` | bind stream/datagram channel to full service reference |
| `CAPABILITY_REFRESH` | renew/replace expiring authority without implicit widening |
| `GOAWAY` | drain new work |
| `CONNECTION_CLOSE` | authenticated close and safe reason |

Unknown non-critical frames are skipped by explicit length. Unknown critical frames close the
session with `unsupported_extension`.

## 9. Reliable streams

Stream IDs encode initiator and unidirectional/bidirectional role. Each STREAM frame carries stream
ID, byte offset, length, FIN, and bytes. Requirements:

- out-of-order ranges are buffered within per-stream/connection limits;
- overlapping bytes must be identical or the session closes for protocol violation;
- FIN fixes final size; inconsistent final size is fatal;
- delivery to the application is ordered and exactly once within the live session abstraction;
- application operation IDs still provide end-to-end idempotency across reconnects;
- reset releases buffered memory promptly; and
- no peer can open more streams than explicitly granted.

Default limits are 64 bidirectional and 64 unidirectional streams, 1 MiB connection receive window,
256 KiB per stream, and smaller constrained-profile values. Q42 bulk transfer negotiates larger
windows only within Sentinel and application budgets.

## 10. Datagrams

DATAGRAM carries service channel ID, message ID, expiry delta, acknowledgement policy, and payload.
Datagrams are unordered and may be lost/duplicated at the bearer; replay protection prevents duplicate
delivery within the session. Application-level reliable messages use an explicit ACK/retry profile,
reusing Qualia's chat mesh lessons without pretending to be a byte stream.

Presence uses expiring datagrams and is never retransmitted after expiry. Consent, mutation, and
financial/governance operations use reliable streams plus signed operation IDs.

## 11. Acknowledgements and loss detection

ACK frames contain largest acknowledged packet, bounded acknowledgement delay, and up to eight ranges.
ACK-only packets are not ack-eliciting. A sender records send time and bytes-in-flight for
ack-eliciting packets.

Initial base recovery:

- packet threshold: three newer acknowledged packets;
- time threshold: `9/8 * max(latest_rtt, smoothed_rtt)` with local timer granularity floor;
- probe timeout derived from smoothed RTT, RTT variance, maximum acknowledgement delay, and granularity;
- retransmit semantic frames in new packets with new packet numbers, never replay ciphertext/nonce;
- retain no unbounded sent-packet history; and
- maintain loss/RTT state per path.

Precise pseudocode and constants must be frozen with executable vectors before QDNF 1.0. Constrained
bearers may use larger timer floors and fewer ACK ranges.

## 12. Congestion and pacing

The base general-purpose profile uses a NewReno-like byte congestion window per path:

- initial window bounded by path MTU and conservative packet count;
- slow start until threshold/loss;
- additive increase in congestion avoidance;
- multiplicative decrease on congestion event;
- pacing based on estimated RTT/window where timers allow;
- ACK-only/control traffic remains rate-limited even if not counted as bytes-in-flight; and
- local IPC may negotiate a no-network-congestion profile but still enforces flow/memory control.

Radio/mesh profiles include airtime and energy constraints. QPolicy priority selects queues only after
authorization; it cannot disable congestion safety or starve control/revocation traffic.

## 13. Flow control and backpressure

Connection and stream windows are absolute monotonically increasing limits. Applications receive
bytes only as fast as their bounded inbox can admit them. A peer that stops consuming naturally stops
window updates. QSync, content, chat, and inference channels have separate service quotas so bulk
transfer cannot exhaust control or human-interactive traffic.

## 14. Rekey

Either endpoint may request a new application key phase. The sender begins the new phase only after
derivation; the receiver retains prior keys for a bounded reorder window. One outstanding update is
allowed. Packet numbers remain monotonic across key phases unless the frozen wire profile explicitly
uses separate spaces and proves nonce uniqueness.

Controller/capability refresh is separate from traffic rekey. Rekey does not renew an expired RAR or
grant.

## 15. Path migration

Migration occurs when QRoute selects a new verified DNI/path or a bearer locator changes.

1. Obtain/verify new RAR/DNI if topology identity changed.
2. Send PATH_CHALLENGE on the candidate path.
3. Receive matching PATH_RESPONSE bound to current session/exporter.
4. Start independent RTT/congestion state.
5. Move traffic according to local policy; keep old path briefly if still valid.
6. Retire old route on withdrawal/expiry/failure and erase path-local material.

No packet received from a new locator automatically moves the end-to-end session. WireGuard roaming
in a Transition bearer updates that carrier only; QSession still validates material path changes.

## 16. Multipath extension

The optional multipath feature supports up to three validated paths. Stream scheduling is
deterministic per local policy and avoids reordering unless explicitly enabled. Immutable content
blocks and independent datagrams are natural multipath candidates. Each path has its own congestion,
RTT, validation, and failure state; keys remain session-level with path ID authenticated in nonce/AAD
rules defined by the extension.

## 17. Service opening

SERVICE_OPEN binds a short channel ID to:

- full canonical service IRI and Q42 index;
- protocol version;
- operation class and purpose;
- target resource/context;
- required reliability mode;
- maximum message/stream size;
- sensitivity class;
- capability decision digest; and
- extensions.

The receiver verifies the full IRI before dispatch, so `q_hash` collision cannot select the wrong
service. A service cannot inherit another channel's capability.

## 18. Core services

### 18.1 QResolve

Small request/response datagrams or streams for RAR/Alias lookup. Public queries minimize disclosed
target data. Answers are independently signed/verified.

### 18.2 Chat

Carries signed `RelayEnvelope`-equivalent messages over reliable application messages or streams.
Author DID proof, channel membership, block state, mention limits, attachment digests, and moderation
receipts remain application/QPolicy concerns. Transport ACK is not a read receipt.

### 18.3 Presence

Voluntary, scoped, expiring datagrams. No continuous location by default. Missing presence is unknown,
not offline proof. No delivery receipt is required.

### 18.4 Share

Offer/accept/reject protocol for governed records/content. An offer discloses bounded metadata and
digest; transfer begins only after target capability/consent and sensitivity policy. Acceptance does
not authorize future unrelated access.

### 18.5 QDP/qapp RPC

Typed request/response over streams. Each tool/action has a capability manifest, input/output schema,
deadline, cancellation, and byte budget. Remote text never becomes a command. Qapp isolation remains
enforced at the host boundary.

### 18.6 QSync

Transfers signed CRDT/semantic operations and Q42 blocks. It separates:

- authorization handshake;
- operation publication/pull;
- block/content transfer;
- receipt/acknowledgement; and
- application validation/merge.

The relay/transport is a dumb pipe. Receiver inbox validates version, operation ID, content digest,
signature, replay, context, capability, sensitivity, and merge policy before application.

### 18.7 Content swarm

A manifest stream provides artifact digest, block layout, provider grants, and availability. Blocks
may arrive unordered from multiple providers and are admitted only after digest proof. Mutable data
never uses content equality as permission to merge.

## 19. Group sessions

Base QSession is pairwise. Group messaging uses pairwise delivery or a separately versioned group-key
profile with explicit membership epochs, sender keys, removal/rekey, transcript, and history policy.
Realm group keys are never reused for application groups. Removing a member provides forward
confidentiality after rotation but cannot erase content previously received.

## 20. Session closure

Close codes distinguish normal drain, idle timeout, route expiry, controller/key revocation,
capability expiry/revocation, block, protocol error, resource limit, and administrative shutdown.
Sensitive reason details remain local. GOAWAY stops new services/streams while bounded existing work
finishes. Hard block/non-derogable violation can close immediately.

## 21. Defaults

| Parameter | Default |
|---|---:|
| idle timeout | 120 s interactive, service-specific otherwise |
| keepalive | disabled unless needed for active route/session |
| max concurrent sessions per peer | 8 |
| max streams per direction/type | 64 |
| max ACK ranges | 8 |
| max paths | 3 |
| application packet rekey | 2^30 packets or 24 h |
| handshake timeout | 10 s local, 30 s constrained/relay |
| close drain | 3 probe timeouts, capped by policy |

## 22. Conformance tests

- transcript/key vectors and wrong-DID/wrong-DNI/wrong-path rejection;
- loss, duplication, reordering, delayed ACK, retransmission with fresh packet numbers;
- stream overlap/final-size/reset/flow-control violations;
- datagram replay/expiry and presence non-retransmission;
- congestion response and per-path state;
- path migration and route withdrawal race;
- rekey under packet reorder;
- service hash collision resolved by full IRI;
- QPolicy denial before application delivery;
- block/revocation during active stream;
- content swarm digest failure and malicious provider; and
- zero-allocation packet hot paths plus bounded buffers under adversarial input.
