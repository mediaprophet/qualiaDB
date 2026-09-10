# CSCP implementation programme

**Status:** Wave 0 in-tree; Wave 1 first swarm authorised; Wave 2 blocked on operators / QUIC engine admission  
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
| ContactDescriptor / RelayLease / CustodyLease / PathEvidence / Receipt / Accept / Reject codecs | partial (reject + receipt encode only) | CSCP-01 |
| Private mailbox resolve, stale reject | in-kernel only | CSCP-02 |
| Lease charge/cancel on a live two-peer CSCP exchange | UDP topology only | CSCP-03 |
| PathEvidence: remote `validated=1` fail closed; Accept/Reject | missing | CSCP-04 |
| CSCP control on TLS WSS + QSession | missing | CSCP-05 |
| Receipt persist/recover | RAM only | CSCP-06 |
| QUIC ALPN `cscp/1` | no | CSCP-07 (evaluate quinn/noq; do not write QUIC) |
| MASQUE bound UDP on Internet | no | CSCP-08 blocked on operator |
| HTTP/2 capsule fallback | no | CSCP-09 |
| Browser profile | no | CSCP-10 |
| Independent review | no | CSCP-11 |
| Datatracker submit | no | CSCP-12 human |

“Fully implemented” for CSCP means Wave 1 complete: every message type on the wire, mailbox, leases, evidence rules, Accept/Reject, QSession-bound local TLS WSS, durable receipts. Wave 2 is the Internet profile and stays explicitly incomplete until Gates B–E in the architecture note.

## Wave dispatch

Wave 1 packages are disjoint writes. Workers do not edit `mod.rs`, `Cargo.toml`, honesty flags, or the original 30-package checklists. Integrator merges exports and runs `cargo test -p qualia-core-db --lib --offline net::peer::fabric p2p::connectivity::cscp`.

| ID | Owner files | Depends |
|---|---|---|
| CSCP-00 | done: kernel, connect, select, experiments, local UDP | — |
| CSCP-01 | `net/peer/fabric/wire/` | — |
| CSCP-02 | `net/peer/fabric/mailbox.rs` | CSCP-01 interface |
| CSCP-03 | `net/peer/fabric/lease_protocol.rs` | CSCP-01, lease.rs (read) |
| CSCP-04 | `net/peer/fabric/outcome.rs` | CSCP-01, kernel (read) |
| CSCP-05 | `p2p/connectivity/cscp_wss.rs` | CSCP-01, existing wss_tls/QSession |
| CSCP-06 | `net/peer/fabric/receipt_store.rs` | CSCP-01, durable_store (read) |

Commands:

```text
CARGO_TARGET_DIR=/tmp/qdnf-continue-target
cargo test -p qualia-core-db --lib --offline net::peer::fabric -- --test-threads=1
```

Do not match `identity::`. Do not invent a public relay URL. Do not set Internet honesty flags true.
