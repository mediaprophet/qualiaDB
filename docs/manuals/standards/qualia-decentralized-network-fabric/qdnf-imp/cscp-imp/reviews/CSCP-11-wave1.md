# CSCP-11 — Independent review of Wave 1 (local control plane)

**Reviewer:** CSCP-11 (did not author Wave 1). Not approved from the author summary in `cscp-imp/progress-log.md`.  
**Date:** 2026-09-10  
**Branch:** `0.0.38`  
**HEAD read:** `4cb9f9b1aeb8c733785324550126164ac7fececf`  
**Normative spec:** `docs/standards/ietf/draft-webcivics-cscp-00.md`  
**Scope:** CSCP-01–06 local completeness only. Wave 2 (QUIC ALPN, MASQUE Internet, HTTP/2 capsule, browser, datatracker) is out of scope for accept.  
**Concurrent dirty tree (not this review, preserved):** untracked `fabric/capsule.rs` (CSCP-09) and CSCP-10 markdown.

## Verdict

**accept-with-fixes** for Wave 1 *local* completeness.

The local control plane exists and the disclosure invariant holds on the exercised honest path: faster Direct cannot override ApprovedRelaysOnly; unknown critical TLVs fail closed on all eight message types; decoded PathEvidence is never local validation; relay and custody are distinct types and message codes; grant-revoked receipt replay is Denied; the mailbox is a 16-slot invitation store, not a DHT; local WSS is rustls, not plain TCP, and does not assign Internet honesty flags. Those claims were re-read in source and re-run as tests in this session.

The codec and a few APIs still fail *open* on malformed or caller-lying inputs that a second implementation or a hostile peer can send. That is not Internet scope. It is still a Wave 1 defect in “every message type on the wire.” Integrator applies the fixes below. This reviewer does not edit `.rs` and does not tick workstream boxes.

A clean **accept** would require the required-TLV and lease-cap clamps (F1, F2) at minimum. **Reject** would require a live path where Direct beats relay-only, PathEvidence decode yields local validation, receipts Commit after revoke, mailbox probes Direct under relay-only, or WSS sets Internet flags. Those were not found.

## Independent checks (this session)

```text
CARGO_TARGET_DIR=/tmp/qdnf-continue-target
cargo test -p qualia-core-db --lib --offline net::peer::fabric -- --test-threads=1
# 34 passed, 0 failed, 0 ignored  (rustc 1.98.1, Linux x86_64)

cargo test -p qualia-core-db --lib --offline p2p::connectivity::cscp_wss -- --test-threads=1
# 1 passed, 0 failed  (0.26s)
```

Author-claimed “34 fabric + 1 WSS” reproduces. That measures the in-tree tests. It does not measure missing negative cases listed under Defects.

## Review items (brief CSCP-11)

### 1. Exclude-then-rank — **hold** (with residual)

`select::exclude_then_rank` drops prohibited classes *before* scoring. Test `faster_direct_cannot_beat_relay_only_policy` keeps only Relayed when DirectV6 RTT is 1 ms and Relayed is 50 ms.

`carrier::prohibited`: ApprovedRelaysOnly / QualifiedMultiHop forbid DirectV6/V4; Isolated forbids every live class.

`kernel::step(PathValidated { DirectV6 })` under `ProtectionPolicy::RELAY_ONLY` returns `Exclude` and does not enter `PathLive` (`relay_only_excludes_validated_direct`). `Descriptor { direct_locator: true }` under relay-only returns `Exclude { DirectV6 }` without incrementing `direct_probe_count`.

**Residual:** the live kernel never calls `exclude_then_rank`. On a resolved mailbox descriptor it always `Probe { Relayed }` if Relayed is allowed. Ranking is a library function plus unit test, not the connect state machine. DirectPermitted therefore does not actually prefer a faster validated Direct on the live path. Disclosure is still fail-closed; ordinary ranking is incomplete, not inverted.

### 2. Unknown critical TLV — **hold**

`wire::walk_tlvs`: tag bit 7 set and `!known(tag & 0x7f)` → `CscpError::UnknownCritical`.

Covered for MsgType 1 in `connect_msg::unknown_critical_tlv_is_rejected` and for types 2–8 in `records::unknown_critical_rejected_on_every_message_type`. Unknown non-critical tags are ignored.

### 3. PathEvidence demotion — **hold**

`records::decode_evidence`: `observer == 2 && validated != 0` → `Malformed`. Successful decode always returns `PathEvidence::remote_assertion(...)` and discards wire `validated` / payload / RTT (`let _ = (max_payload, rtt_ms, at)` aside from `at` as observation time). Tests: `observer2_validated1_is_malformed`, `decoded_evidence_is_never_locally_validated`.

`PathEvidence::validated()` requires *both* the private `validated` bit and `ObserverKind::LocalTransport`. `observer` is `pub` but flipping it to LocalTransport cannot promote a remote row because `validated` stays private and false. `from_witness` is the only local-true constructor; `TransportWitness::from_local` is `pub(crate)`.

### 4. Relay lease ≠ custody lease — **hold** (with F2)

Distinct structs (`lease.rs`). Distinct MsgType 3 vs 4. `apply_relay_lease` only `decode_relay_lease`. `custody_cannot_forward` only `decode_custody`. `RelayLease::charge` does not exist on `CustodyLease`. `apply_relay_lease` forces `export_observations = false` unless disclosure is `DirectPermitted` (covers ApprovedRelaysOnly, QualifiedMultiHop, Isolated). `encode_forced_relay_only` clears the bit even if the in-memory lease said 1.

`BoundUdpRelay::forward_once` charges a `RelayLease` and stops on expiry/cap (`lease_expiry_stops_forward`). Two local UDP peers exchange under a CSCP-encoded lease (`two_local_peers_exchange_under_cscp_lease`).

### 5. Transport ACK is not Committed; revoke replay Denied — **hold** (with residual)

`BoundUdpRelay` success does not mint an `OpReceipt`. Receipts are a separate queue (`Fabric::enqueue` / `receipt_store`).

`OpReceipt::replay`: grant dead or generation mismatch → `Denied`; different digest → `Denied`. Experiments: `revoked_grant_replay`. Durable: recover then `replay(..., grant_live=false)` → `Denied` (`persist_recover_and_truncated_fail_closed`). Truncated `CSCPRC1` file fails closed.

**Residual:** matching `op_id`+digest under a *live* grant returns `Committed` from `Queued` with no Received/Validated step. That is the in-tree “happy replay” gate, not a relay ACK. Spec status 2/3 are unused. Do not describe RAM `Fabric.receipts` as crash-safe; only the CRC file path is durable, and README already says the RAM queue is not.

### 6. Mailbox is not a DHT; Direct not probed under relay-only — **hold** (with residual)

`PrivateMailbox`: 16 `Option<ContactDescriptor>` slots, linear `find`, no network, no Pkarr. `ProtectionPolicy::RELAY_ONLY.public_dht` is `false` (asserted in the mailbox test).

`resolve`: Direct + `prohibited(DirectV6)` → `PolicyDenied`. `ingest_wire` of a Direct descriptor under ApprovedRelaysOnly returns `PolicyDenied`. Kernel stale+direct event does not increment `direct_probe_count`. No code in `mailbox.rs` dials `locator`.

**Residual:** `publish` still *stores* Direct locators under relay-only; `encode_current` still emits the Direct TLV blob (the mailbox test does this). Spec §8: relay-only MUST NOT export access-network addresses through candidate TLVs. Storage is not a probe; re-encoding the locator is an export of the 16-byte field if it holds an IP.

### 7. `cscp_wss.rs` honesty + rustls — **hold** (with residual)

`cscp_then_qsession_over_local_tls_wss` uses `loopback_tls_wss` (`wss_tls.rs`): `rustls::StreamOwned<ClientConnection|ServerConnection, TcpStream>`, pinned local CA, not a raw TCP CSCP path. Control bytes are ConnectRequest then ConnectAccept (Relayed). No Direct locator TLV on that stream. Then `handshake_over_fragments` + `SessionBinding::from_permit`.

Internet flags in `net/peer/connectivity/evidence.rs` are `pub const fn` returning `false` (`public_relay_dialed`, `internet_two_host_handshake_executed`, `masque_bound_udp_internet_executed`, `noq_transport_admitted`, `quic_iroh_benchmark_executed`). This file cannot set them. The WSS test asserts only the first and the MASQUE flag.

**Residual:** Accept is produced after an in-process `TransportWitness::from_local(PathClass::Relayed, ...)` with no relay hop. The WSS carrier did not validate Relayed. Honest class for this fixture is closer to `BrowserGateway` / loopback WSS. The test proves CSCP TLVs on local rustls WSS + QSession, not a Relayed path.

### 8. Zero-heap — **hold** on named hot paths; cold persist allocates

| Path | Alloc |
|---|---|
| `wire/` encode/decode | stack `[u8; N]`, no `Vec`/`String`/`Box` |
| `kernel.rs`, `select.rs`, `mailbox.rs`, `lease_protocol.rs`, `outcome.rs` | Copy structs / fixed arrays |
| `receipt_store.rs` | **`Vec::new()`** for file body and read-back — allowed cold persist; say so |
| `cscp_wss.rs` | `Result<..., String>` on the demo/error path only |

`capsule.rs` is CSCP-09 concurrent work and was not in HEAD; not scored as Wave 1.

### 9. Security (TLV duplicate, 1024 cap, replay, stale) — **partial**

| Control | Status |
|---|---|
| Duplicate TLV | `seen[tag & 0x7f]` for tags 0–15 only. Known CSCP tags are 1–6, so duplicates of specified fields fail closed. Tags ≥ 16 skip the bitmap. |
| Body cap 1024 | `parse_header` rejects `blen > MAX_BODY`. **`finish` / `write_tlv` do not cap encode.** Local encoders use small stacks so honest encode stays under 1024. |
| Replay after revoke | Denied (item 5). |
| Stale generation | `mailbox::publish` / `resolve` compare `generation < last_gen`. `ContactDescriptor::is_stale` is correct. **`KernelEvent::Descriptor.stale` is a caller boolean.** `kernel::step` does not recompute against `last_contact_generation`. A caller that `note_contact`s gen 5 then steps `stale: false` with an old descriptor admits it. |
| Required critical TLVs present | **Fail open.** See F1. |
| `Flags` / `Reserved` | Spec: other flag bits MUST be zero in v1. `parse_header` ignores bytes 6–7. |
| `public_dht` | Decoded as sent. Relay-only / clinical ConnectRequest with `public_dht=1` is accepted. Constants default false; wire does not force 0. |

## Defects (integrator fixes)

Each item is a failing scenario against current functions. This reviewer does not patch them.

### F1 — Required critical TLVs are optional at decode (must fix)

**Where:** `wire/records.rs` `decode_contact`, `decode_relay_lease`, `decode_custody`, `decode_evidence`, `decode_receipt`, `decode_accept`, `decode_reject`; `wire/connect_msg.rs` `decode_connect_request`; `wire/mod.rs` `walk_tlvs` + match arms with `if val.len() == N`.

**Scenario:** 10-byte header, `Body Length = 0`, valid magic/version/MsgType. `walk_tlvs` returns `Ok` immediately.

- `decode_accept` → `Ok((PathClass::Offline, 0))`. A peer can Accept without tags 1–2.
- `decode_contact` → mailbox key `[0;32]`, generation 0. (`publish` later rejects zero key; decode itself does not.)
- `decode_connect_request` with all fields except `TAG_PEER` present and a future deadline → peer stays `[0;32]`, intent is admitted. Critical tag 1 with length 31 is also ignored (`TAG_PEER if val.len() == 32` falls through to `_ => {}`).

Spec §4 lists those tags as critical. Unknown-critical is implemented; *missing* and *wrong-length* critical values are not. No test constructs an empty body or a short peer TLV.

**Fix:** after `walk_tlvs`, require every critical tag for that MsgType with the specified length; otherwise `Malformed`. Length mismatch on a present critical tag must not be ignored.

### F2 — `remaining_bytes` may exceed `max_bytes` (must fix)

**Where:** `records::decode_relay_lease` tag 4; `lease_protocol::apply_relay_lease`; `RelayLease::charge`.

**Scenario:** craft MsgType 3 with `max_bytes = 1`, `remaining_bytes = 1000`. `apply_relay_lease` returns the struct unchanged (aside from export bit). `charge(1000, now)` succeeds because `charge` uses `remaining_bytes`, not `max_bytes`.

Spec §4.4: implementations MUST NOT enlarge remaining_bytes to finish a transfer. Honest `RelayLease::grant` sets remaining = max. Adversarial bytes enlarge the cap at decode.

**Fix:** on decode/apply, `remaining_bytes = min(remaining_bytes, max_bytes)` or `Malformed` if remaining > max.

### F3 — `SessionReady` / `admit_session` do not re-check disclosure (should fix)

**Where:** `session::SessionReady::try_new`; `connect::admit_session`.

**Scenario:** RELAY_ONLY kernel reaches `SessionLive` via Relayed `PathValidated`. Caller then passes `PathEvidence::from_witness(DirectV6)` with the live generation. `try_new` checks grant, `path.validated()`, generation, and `SessionLive` only — not `prohibited(disclosure, path.class)` and not `kernel.selected`. Session can be bound to Direct evidence after a Relayed admit.

`outcome::encode_outcome` *does* reject prohibited / unvalidated rows. The session hatch is weaker than Accept.

**Fix:** `try_new` must fail if the path class is prohibited for the admitted intent, and should require `Some(path.class) == kernel.selected`.

### F4 — v1 Flags/Reserved not fail-closed (should fix)

**Where:** `wire::parse_header`.

**Scenario:** valid ConnectRequest with `Flags = 0x02` or `Reserved != 0` decodes. Spec §4: other flag bits MUST be zero in v1.

**Fix:** non-zero flags or reserved → `Malformed`.

## Residual risks (not blocking Wave 1 local accept-with-fixes)

- Live kernel does not rank candidates (`exclude_then_rank` unused). Ordinary DirectPermitted never selects Direct on Descriptor/network-switch; it probes Relayed. Conservative for disclosure, incomplete for §5 ranking.
- Mailbox retains and can re-encode Direct locators under relay-only (F6 residual in item 6).
- `KernelEvent::Descriptor.stale` / `expired` trusted; mailbox is the real generation gate only if callers use it.
- `known_lease` on custody accepts tags 5–6 (relay-only fields). Critical tag 5 on MsgType 4 is ignored, not `UnknownCritical`.
- Duplicate bitmap ignores tag numbers ≥ 16.
- Encode path does not enforce `MAX_BODY`.
- WSS fixture mints Relayed local evidence without a relay (item 7).
- `decode_connect_request` does not force `public_dht = 0` for ApprovedRelaysOnly / Isolated / Clinical.
- `independent_protocol_review_executed()` remains `false` in `evidence.rs`. This markdown is the review; that const must stay false until the integrator decides otherwise. This assignment forbids editing it.
- RAM receipt queue is not crash-safe. CRC file is.
- Wave 2 remains incomplete by design: no QUIC `cscp/1`, no Internet MASQUE, no browser origin, no datatracker.

## What is complete enough locally (do not over-claim)

Eight CSCP v1 message types round-trip. Isolate queues without a session. Loopback bound-UDP forwards under a live RelayLease and stops when the lease is dead. ConnectAccept encoding refuses remote assertions and Direct under relay-only when `encode_outcome` is the gate. Durable receipts recover; truncation fails closed; revoke replay is Denied. Local rustls WSS carries ConnectRequest/Accept then QSession. `public_dht` defaults false. Internet honesty flags stay false.

That is a local control-plane slice, matching draft §10, not an Internet profile.

## Recommended integrator actions

1. Apply F1 and F2; add tests: empty body on every MsgType; short critical peer TLV; `remaining > max` lease.
2. Apply F3 (and F4 if cheap).
3. Re-run `cargo test -p qualia-core-db --lib --offline net::peer::fabric p2p::connectivity::cscp_wss -- --test-threads=1`.
4. Do not tick CSCP-11 or Wave 2 boxes from this file. Do not set Internet honesty flags. Do not treat this review as CSCP-12.

After F1+F2 (and ideally F3) land, a follow-up independent pass can upgrade to **accept** for Wave 1 local completeness. Until then the codec is not fail-closed against malformed CSCP bytes.
