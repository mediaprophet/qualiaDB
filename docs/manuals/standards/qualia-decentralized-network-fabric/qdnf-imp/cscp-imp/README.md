# CSCP implementation programme

**Status:** Wave 1 local control plane tested and fail-closed after independent review; QUIC deferred; local DATAGRAM capsules in-tree; browser profile recorded; CSCP-08/12 blocked  
**Date:** 2026-09-10  
**Normative spec:** [draft-webcivics-cscp-00.md](../draft-webcivics-cscp-00.md)  
**Swarm rules:** reuse [qdnf-imp swarm protocol](../swarm-protocol.md), briefs, handoffs, evidence manifests  
**Branch:** `0.0.38`  
**Integrator:** this session  

CSCP is **not fully implemented**. Wave 0 is the kernel, `ConnectRequest` codec, loopback bound-UDP, and four experiments. That is a control-plane slice, not the complete protocol on interchangeable carriers.

This programme implements CSCP as specified. It does **not** implement QUIC, MASQUE HTTP/3, or new cryptography. It does **not** tick the original 30 QDNF packages. IETF datatracker submission is a human action (CSCP-12).

## Honest gap

| Spec requirement | Wave 0 | Remaining |
|---|---|---|
| Exclude-then-rank | yes | keep tests |
| ConnectRequest TLV | yes | — |
| ContactDescriptor / RelayLease / CustodyLease / PathEvidence / Receipt / Accept / Reject codecs | yes (Wave 1) | keep tests |
| Private mailbox resolve, stale reject | yes (Wave 1) | keep tests |
| Lease charge/cancel on a live two-peer CSCP exchange | yes (local UDP + CSCP lease bytes) | Internet lease still no |
| PathEvidence: remote `validated=1` fail closed; Accept/Reject | yes (Wave 1) | keep tests |
| CSCP control on TLS WSS + QSession | yes (local rustls) | not browser / not Internet |
| Receipt persist/recover | yes (CRC file) | RAM queue still not crash-safe |
| QUIC ALPN `cscp/1` | deferred (CSCP-07) | no engine admitted |
| MASQUE bound UDP on Internet | no | CSCP-08 blocked on operator |
| HTTP/2 capsule fallback | local DATAGRAM framing only | CSCP-09 HTTP/2 CONNECT still open |
| Browser profile | profile note | interop not executed |
| Independent review | yes (accept-with-fixes; F1–F4 applied) | — |
| Datatracker submit | no | CSCP-12 human |

“Fully implemented” for the **local control plane** (Wave 1) is CSCP-01–06: every message type on the wire, mailbox, leases, evidence rules, Accept/Reject, QSession-bound local TLS WSS, durable receipts. That is now in-tree and tested. Wave 2 is the Internet profile and stays explicitly incomplete until Gates B–E in the architecture note.

## Wave dispatch

Wave 1 packages are disjoint writes. Workers do not edit `mod.rs`, `Cargo.toml`, honesty flags, or the original 30-package checklists. Integrator merges exports and runs `cargo test -p qualia-core-db --lib --offline net::peer::fabric p2p::connectivity::cscp`.

| ID | Owner files | Depends |
|---|---|---|
| CSCP-00 | done: kernel, connect, select, experiments, local UDP | — |
| CSCP-01 | done: `net/peer/fabric/wire/` | — |
| CSCP-02 | done: `net/peer/fabric/mailbox.rs` | CSCP-01 |
| CSCP-03 | done: `net/peer/fabric/lease_protocol.rs` | CSCP-01 |
| CSCP-04 | done: `net/peer/fabric/outcome.rs` | CSCP-01 |
| CSCP-05 | done: `p2p/connectivity/cscp_wss.rs` | CSCP-01 |
| CSCP-06 | done: `net/peer/fabric/receipt_store.rs` | CSCP-01 |
| CSCP-07 | `cscp-imp/decisions/CSCP-07-quic-alpn.md` (swarm) | CSCP-05 |
| CSCP-09 | `net/peer/fabric/capsule.rs` (swarm; integrator merges `mod.rs`) | CSCP-01 |
| CSCP-10 | `cscp-imp/decisions/CSCP-10-browser-profile.md` (swarm) | CSCP-05 |
| CSCP-11 | `cscp-imp/reviews/CSCP-11-wave1.md` (swarm) | CSCP-01–06 |

Commands:

```text
CARGO_TARGET_DIR=/tmp/qdnf-continue-target
cargo test -p qualia-core-db --lib --offline net::peer::fabric -- --test-threads=1
```

Do not match `identity::`. Do not invent a public relay URL. Do not set Internet honesty flags true.

Identifier split (not a CSCP wire task): `did:qi:` is the recorded HCAI DID method **name**; QRC stays `did:q42:`; see [decisions/did-qi-git-utxo.md](./decisions/did-qi-git-utxo.md). No resolver until a method spec exists.
