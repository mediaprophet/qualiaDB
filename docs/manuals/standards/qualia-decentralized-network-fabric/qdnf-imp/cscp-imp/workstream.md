# CSCP workstream checklists

Integrator ticks a box only after recorded evidence. Do not tick because a stub compiles.

## CSCP-00 — Wave 0 slice (recorded 2026-09-10)

- [x] CSCP-00.01 Kernel admit / exclude Direct under relay-only
- [x] CSCP-00.02 `exclude_then_rank` faster Direct cannot beat relay-only
- [x] CSCP-00.03 ConnectRequest encode/decode + unknown critical TLV reject
- [x] CSCP-00.04 Loopback bound-UDP with lease charge
- [x] CSCP-00.05 Four experiments (stale descriptor, lease expiry, network switch, revoked replay)
- [x] CSCP-00.06 `SessionReady` not a public hatch
- [x] CSCP-00.07 Honesty: no public MASQUE, no noq, no Internet two-host

## CSCP-01 — Complete CSCP v1 TLV codec

- [x] CSCP-01.01 Split `wire.rs` into `wire/` below 500 lines per file
- [x] CSCP-01.02 ContactDescriptor encode/decode; stale is a receiver policy, not a missing field
- [x] CSCP-01.03 RelayLease encode/decode; `export_observations` must be 0 for relay-only callers
- [x] CSCP-01.04 CustodyLease encode/decode; must not be usable as forwarding budget
- [x] CSCP-01.05 PathEvidence encode/decode; `observer=2 && validated=1` rejected
- [x] CSCP-01.06 Decoded PathEvidence is never locally validated (demote to assertion)
- [x] CSCP-01.07 Receipt encode/decode round-trip; Committed not implied by relay
- [x] CSCP-01.08 ConnectAccept / ConnectReject encode/decode
- [x] CSCP-01.09 Unknown critical TLV rejected on every message type
- [x] CSCP-01.10 Tests: `cargo test -p qualia-core-db --lib --offline net::peer::fabric::wire`

## CSCP-02 — Private mailbox

- [x] CSCP-02.01 Bounded mailbox store (fixed slots, no DHT)
- [x] CSCP-02.02 Publish descriptor; resolve by contact key
- [x] CSCP-02.03 Stale generation rejected; expired rejected
- [x] CSCP-02.04 Direct locator in mailbox not probed when disclosure forbids
- [x] CSCP-02.05 `public_dht` false remains default

## CSCP-03 — Lease protocol

- [x] CSCP-03.01 Apply RelayLease from CSCP bytes; charge; cancel
- [x] CSCP-03.02 Mid-transfer expiry stops forward; cap not enlarged
- [x] CSCP-03.03 Relay-only forces `export_observations=0` even if the bytes say 1
- [x] CSCP-03.04 CustodyLease cannot satisfy a forwarding charge
- [x] CSCP-03.05 Two local peers exchange a lease then data under it

## CSCP-04 — Accept / Reject / evidence

- [x] CSCP-04.01 ConnectAccept only after locally validated permitted path
- [x] CSCP-04.02 ConnectReject on stale, expired, revoked, isolated, policy
- [x] CSCP-04.03 Remote assertion cannot become selected path
- [x] CSCP-04.04 Kernel + codec integration test

## CSCP-05 — TLS WSS + QSession carrier

- [x] CSCP-05.01 Two loopback TLS WSS peers exchange ConnectRequest/Accept
- [x] CSCP-05.02 QSession `handshake_over_fragments` then `SessionBinding::from_permit`
- [x] CSCP-05.03 Relay-only: no direct locator TLVs on that control stream
- [x] CSCP-05.04 Plain TCP fixture is not this path
- [x] CSCP-05.05 Do not set `public_relay_dialed` or Internet flags

## CSCP-06 — Durable receipts

- [x] CSCP-06.01 Persist CSCP receipts to CRC file (reuse durable_store pattern or sibling file)
- [x] CSCP-06.02 Recover after drop; truncated fails closed
- [x] CSCP-06.03 Replay after recover + grant revoke still Denied
- [x] CSCP-06.04 RAM queue is not claimed crash-safe

## CSCP-07 — QUIC ALPN (evaluate, do not write QUIC)

- [ ] CSCP-07.01 Decision record: quinn vs noq vs defer
- [ ] CSCP-07.02 If admitted: ALPN `cscp/1` on loopback only; Qualia session still required
- [ ] CSCP-07.03 `noq_transport_admitted()` stays false until an admitted revision is reviewed

## CSCP-08–12 — Internet / review / submit

- [ ] CSCP-08 MASQUE bound-UDP Internet (blocked: operator URL)
- [ ] CSCP-09 HTTP/2 capsule fallback
- [ ] CSCP-10 Browser WebTransport/WSS profile
- [ ] CSCP-11 Independent review of CSCP-01–06
- [ ] CSCP-12 Human: submit `-00` to IETF datatracker
