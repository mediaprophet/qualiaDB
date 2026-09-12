# CSCP-07 — QUIC engine and ALPN `cscp/1`

- Decision ID, date, status and owner: **CSCP-07** / 2026-09-10 / **accepted: defer** / swarm-cscp-07
- Affected parent/child tasks and source/profile baseline: CSCP-07.01 (engine choice), CSCP-07.02 (ALPN binding), CSCP-07.03 (`noq_transport_admitted()`). Source: branch `0.0.38` at `b96304919f68d408cf067fab814fd4f01fc54255`. Toolchain observed: rustc 1.98.1, cargo 1.98.1. Profile: Qualia CSCP control plane on local TLS WSS + loopback bound-UDP; greenfield Internet QUIC profile remains unspecified in code.
- Semantic requirement or measured implementation problem: `draft-webcivics-cscp-00` §6 binds CSCP-over-QUIC to one bidirectional client-initiated stream with ALPN `cscp/1` **after** the Qualia peer session. That binding needs an admitted QUIC engine. Live source has no Qualia-owned QUIC engine, no ALPN glue, and no measured loopback QUIC admission.

## Options, including reuse of existing core primitives

### (1) Admit quinn

Add `quinn` as a first-party CSCP carrier, negotiate ALPN `cscp/1` on a local QUIC connection, and keep QSession admission in front of application delivery.

Reuse considered: workspace `Cargo.lock` already contains `quinn` 0.11.11 / `quinn-proto` 0.11.16 / `quinn-udp` 0.5.15. Those crates are **not** a Qualia QUIC engine. They are pulled only as transitive dependencies of:

- `libp2p-quic` 0.13.1 (libp2p optional compat stack)
- `reqwest` 0.12.28 and `reqwest` 0.13.4 (HTTP client HTTP/3 path)

`crates/qualia-core-db/Cargo.toml` declares `libp2p` 0.56.0 as optional under feature `libp2p-compat` with features `tcp, dns, noise, yamux, request-response, kad, mdns, macros, tokio` — **not** `quic`. Native reqwest is used as JSON/HTTP, not as CSCP peer transport. Grep of every workspace `Cargo.toml` for `quinn`, `noq`, and `iroh` returned **no matches**. Grep of crate `.rs` files for `quinn::`, `use quinn`, and `extern crate quinn` returned **no matches**. Grep of crate `.rs`/`.toml` for `cscp/1` and `alpn` returned **no matches**.

Rejected for this pass: admitting quinn would add a QUIC dependency and ALPN glue, which CSCP-07 forbids. Transitive lockfile presence is not an in-tree engine. Research (`quic-native-connectivity-research-2026.md` §8) prefers evaluating **noq** first for integrated traversal/path awareness; vanilla quinn does not supply MASQUE bound-UDP or n0-style NAT traversal by itself.

### (2) Evaluate / admit noq

Adopt n0’s Quinn-derived `noq` (iroh transport) behind a Qualia-owned boundary, as recommended for later qualification in the research note and `capability-scoped-connection-fabric.md` §4.

Live source: no `noq` or `iroh` package in any `Cargo.toml` or `Cargo.lock`. Honesty flags remain closed:

```73:76:crates/qualia-core-db/src/net/peer/connectivity/evidence.rs
/// noq has not been admitted as the Internet QUIC engine.
pub const fn noq_transport_admitted() -> bool {
    false
}
```

```58:61:crates/qualia-core-db/src/net/peer/connectivity/evidence.rs
/// QUIC/iroh alternative has been recorded, not benchmarked here.
pub const fn quic_iroh_benchmark_executed() -> bool {
    false
}
```

The unit test in the same file asserts `assert!(!noq_transport_admitted())` and `assert!(!quic_iroh_benchmark_executed())`. The CSCP draft §10 and the fabric architecture note both state the tree is **not** an admitted noq engine. Outstanding human decisions still include exact QUIC engine revision and memory budget.

Rejected for this pass: evaluation may continue as documentation; **admission** would require Cargo.toml + honesty-flag edits owned by the integrator, a reviewed revision, a resource contract under the 42 MiB Sentinel / two-tier heap rules, and a measured local loopback admission. None of those exist. This brief forbids adding noq/iroh and forbids setting `noq_transport_admitted()` true.

### (3) Defer

Keep CSCP on the existing local carriers (kernel, TLV codec, loopback bound-UDP, TLS WSS + QSession). Reserve ALPN `cscp/1` in the draft only. Do not implement QUIC, do not add ALPN glue, do not change honesty flags.

This is the required default when no QUIC crate is present as a first-party engine and no measured local loopback QUIC admission is in-tree.

## Evidence and quantitative tradeoffs

| Check | Result |
|---|---|
| `rg -i 'quinn\|noq\|iroh' --glob '**/Cargo.toml'` | no matches |
| First-party QUIC engine in `qualia-core-db` | absent |
| `noq` / `iroh` in any `Cargo.lock` | absent |
| `quinn` in `Cargo.lock` | present as transitive only (`libp2p-quic`, `reqwest`) |
| `use quinn` / `quinn::` in `crates/**/*.rs` | no matches |
| ALPN `cscp/1` in crate source | no matches |
| Measured local loopback QUIC admission | not in-tree |
| `noq_transport_admitted()` | `false` (quoted above) |
| `quic_iroh_benchmark_executed()` | `false` |
| `capability_fabric_local_executed()` | `true` (loopback fabric, not QUIC) |
| `masque_bound_udp_internet_executed()` | `false` |

No latency, handshake, or memory numbers are claimed for a QUIC path that does not exist. Lockfile crate versions are inventory, not a benchmark.

Raw capture: `/opt/cursor/artifacts/cscp-07-quic-alpn-source-evidence.log` (2026-09-10T07:20:20Z, HEAD `b96304919f68d408cf067fab814fd4f01fc54255`).

## Decision, accepted limits and rejected assumptions

**Decision: defer (option 3).**

Accepted limits:

- CSCP-07.01: no quinn admission; no noq admission; no iroh application-model substitution.
- CSCP-07.02: ALPN identification sequence `cscp/1` is **reserved** in `draft-webcivics-cscp-00` §6 (QUIC transport binding) and §9 (IANA request). It is **not implemented**. No ALPN glue is added in this assignment.
- CSCP-07.03: `noq_transport_admitted()` remains `false`. This assignment does not change `evidence.rs`.
- Existing local CSCP (Wave 1 control plane on TLS WSS + loopback bound-UDP) is unchanged and is not a QUIC profile.
- CSCP-08 (Internet MASQUE bound-UDP) stays blocked on an operator URL; this deferral does not unblock it.

Rejected assumptions:

- Transitive `quinn` via reqwest or libp2p-quic is not an admitted CSCP QUIC engine.
- Draft reservation of ALPN `cscp/1` is not a wire implementation and does not create an IANA entry (draft §9).
- Local fabric tests (`capability_fabric_local_executed() == true`) do not satisfy research Gates B–E or a QUIC loopback admission.
- Selecting noq in a research note is not in-tree admission.

## Single-purpose library/file ownership

- This record: `qdnf-imp/cscp-imp/decisions/CSCP-07-quic-alpn.md` (swarm-cscp-07).
- Honesty flags: `crates/qualia-core-db/src/net/peer/connectivity/evidence.rs` (integrator only).
- Cargo / features: crate `Cargo.toml` files (integrator only).
- Future QUIC adapter, if later admitted: a Qualia-owned directory-backed module under `net/peer/` with a separate owner from authority, QSession, mailbox, and durable receipts. Do not replace those with iroh’s application model.

## Memory/work/storage and failure/recovery consequences

No new allocator, handshake budget, or stream reassembly is introduced. A later admission must still publish an explicit Internet-adapter resource contract (memory, active handshakes, candidate slots, retransmission bytes) and fail closed before the 42 MiB Sentinel pass is exceeded. QUIC TLS buffers are Tier-2/cold adapter state, not a hot-path zero-heap exemption.

Failure mode of deferral: CSCP-over-QUIC remains unspecified in code. Callers keep using admitted local carriers. Offline receipts and exclude-then-rank continue to apply. There is no silent fallback onto reqwest HTTP/3 or libp2p-quic.

## Authority, privacy, crypto and compatibility consequences

QUIC TLS is not Qualia peer authentication. Draft §6 requires the Qualia peer session **before** CSCP-over-QUIC; draft §7 forbids treating a self-signed transport certificate as a CSCP peer. Deferral preserves that split. No new cipher, hybrid KEM, or early-data profile is introduced. Relay-only disclosure rules are unchanged because no QUIC candidate export path is added.

Compatibility: native QDNF remains independent of this Internet profile. The incremental WireGuard / ICE / TURN / WSS path remains the labelled transition carrier.

## Affected consumers, dependency changes and migration

- CSCP-08: still blocked; depends on an admitted carrier **and** a public operator URL. This decision supplies neither.
- CSCP-09 (HTTP/2 capsules) and CSCP-10 (browser profile) do not require a QUIC engine for their local notes.
- No Cargo.toml, feature, or `mod.rs` change is requested.

## Independent domain reviewers and objections resolved

This is a documentation decision from live source. Independent review of Wave 1 remains CSCP-11 and does not include QUIC admission. No reviewer objection is recorded against deferral; the brief itself prefers defer when no QUIC crate is present.

## Verification and reconsideration triggers

Verified this session:

- Workspace `Cargo.toml` files contain no `quinn` / `noq` / `iroh` direct dependencies.
- `noq_transport_admitted()` is `false` in `evidence.rs` (quoted).
- ALPN `cscp/1` exists only in the draft, not in crate source.

Reconsider admission only when **all** of the following are true:

1. Integrator-owned Cargo change names a reviewed engine revision (prefer noq behind a Qualia boundary; quinn only if noq cannot meet the resource contract).
2. A measured **local loopback** QUIC handshake exists, with Qualia session still required after ALPN `cscp/1`.
3. Resource, linkability, and early-data reviews are recorded; `noq_transport_admitted()` is flipped only after that review.
4. `quic_iroh_benchmark_executed()` remains a separate measurement gate and is not implied by compile success.

Until then, ALPN `cscp/1` stays a draft reservation.
