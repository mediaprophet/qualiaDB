# CSCP implementation progress

## 2026-09-10 — Programme opened; Wave 0 already on 0.0.38

- Step: principal asked whether CSCP is fully implemented and to plan a swarm. Status: **not fully implemented; programme created; Wave 1 launched**.
- Built: this directory (README, workstream, registry). Wave 0 remains the recorded kernel/UDP/ConnectRequest slice.
- Measured: Wave 0 was 24 fabric tests at `8a3f4ff9`. Wave 1 not yet measured.
- Human input needed: CSCP-12 datatracker submit; CSCP-08 operator URL.
- Next: CSCP-01 codec freeze, then parallel CSCP-02/03/04/05/06.

## 2026-09-10 — Wave 1 implemented and tested (CSCP-01–06)

- Step: CSCP-01–06. Status: **done** (local control plane). Wave 2 not done.
- Built: `fabric/wire/` (all eight CSCP v1 message types), `mailbox.rs`, `lease_protocol.rs`, `outcome.rs`, `receipt_store.rs`, `p2p/connectivity/cscp_wss.rs`. Split ConnectRequest-only `wire.rs` into a directory. Unknown critical TLVs fail closed on every type. Decoded PathEvidence is always a remote assertion. Relay-only forces `export_observations=0`. Receipts persist as `CSCPRC1\0` + CRC-32C; truncation fails closed; replay after recover + grant revoke is Denied. Local rustls WSS exchanges ConnectRequest/Accept then QSession `handshake_over_fragments` + `SessionBinding::from_permit`.
- Measured (Linux x86_64, rustc 1.98.1, `CARGO_TARGET_DIR=/tmp/qdnf-continue-target`, `--offline`, `--test-threads=1`):
  - `cargo test -p qualia-core-db --lib --offline net::peer::fabric` → **34 passed**, 0 failed.
  - `cargo test -p qualia-core-db --lib --offline p2p::connectivity::cscp_wss` → **1 passed**, 0 failed (0.26s).
  - Honesty flags remain false: `public_relay_dialed`, `internet_two_host_handshake_executed`, `masque_bound_udp_internet_executed`, `noq_transport_admitted`, `quic_iroh_benchmark_executed`.
- Not claimed: QUIC ALPN `cscp/1`, MASQUE Internet, HTTP/2 capsule, browser WebTransport, IETF publication, RFC.
- Human input needed: CSCP-12 datatracker submit; CSCP-08 named operator / live URL. none this Wave 1 code step otherwise.
- Next: swarm Wave 2 first set — CSCP-07 (decision record), CSCP-09 (local capsule framing), CSCP-10 (browser profile note), CSCP-11 (independent review). CSCP-08 and CSCP-12 stay blocked.

## 2026-09-10 — Wave 2 swarm dispatched

- Step: first remaining set claimed for parallel workers. Status: **in progress**.
- Built: claims in `task-registry.json`; briefs under `briefs/`. Workers must not edit `mod.rs`, `Cargo.toml`, honesty flags, or the original 30-package checklists.
- Measured: not yet; workers report evidence in their handoffs.
- Human input needed: still CSCP-08 URL and CSCP-12 submit.
- Next: integrate worker exports; tick CSCP-07/09/10/11 only after evidence.

## 2026-09-10 — Wave 2 first swarm integrated; CSCP-11 fixes applied

- Step: swarm CSCP-07/09/10/11 + integrator F1–F4. Status: **Wave 1 codec fail-closed; QUIC deferred; capsule framing local-only; browser profile recorded; CSCP-08/12 blocked**.
- Built:
  - CSCP-07: defer quinn/noq. ALPN `cscp/1` not implemented. `noq_transport_admitted()` remains false.
  - CSCP-09.01: `fabric/capsule.rs` RFC 9297-style DATAGRAM capsules. Integrator added `pub mod capsule`. Not HTTP/2 CONNECT, not MASQUE Internet.
  - CSCP-10: browser profile note. Local rustls WSS is not WebTransport interop.
  - CSCP-11: accept-with-fixes. Integrator applied F1 (required critical TLVs), F2 (`remaining_bytes > max_bytes` malformed), F3 (SessionReady re-checks disclosure + selected class), F4 (non-zero Flags/Reserved malformed). Mailbox `encode_current` refuses Direct under relay-only disclosure.
- Measured: `net::peer::fabric` **46 passed**, 0 failed; `p2p::connectivity::cscp_wss` **1 passed**. Honesty flags unchanged.
- Human input needed: CSCP-12 datatracker; CSCP-08 operator URL. none this step otherwise.
- Next: do not claim HTTP/2 CONNECT, QUIC, MASQUE Internet, or RFC.

## 2026-09-10 — CSCP-12 author recorded; CSCP-08 still not a URL

- Step: principal named the I-D author and asked whether GitHub Pages / CloudFront / a firewalled home PC / IPFS / chains can stand in for a public relay. Status: **author filled in; not submitted; no public relay**.
- Built: Authors' Addresses in `draft-webcivics-cscp-00.md` now Timothy Charles Holborn <timothy.holborn@gmail.com>. Individual draft; WG not chosen.
- Measured: not applicable (document identity, not a network trial).
- Human input needed: datatracker post (his login + confirmation mail); WG later via DISPATCH if he wants a home. CSCP-08 still needs a process that accepts inbound Internet sessions (WSS/443 or MASQUE), not a static CDN and not a ledger.
- Next: do not set Internet honesty flags. Pages/WASM can be a *client* origin later; they cannot be Gate B.


