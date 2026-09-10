# Handoff — CSCP-09.02 HTTP/2 Extended CONNECT + capsules (loopback)

## Result

- Package and child IDs completed or still pending:
  - Child **CSCP-09.02** (loopback TLS HTTP/2 Extended CONNECT, RFC 9297 DATAGRAM capsules, CSCP ConnectRequest/Accept, then QSession): **implemented**, awaiting integrator `pub mod cscp_h2;` / `pub mod h2_capsule;` and independent review.
  - Parent **CSCP-09** remains **incomplete** as an Internet / MASQUE / HTTP/3 claim. This child is loopback TLS HTTP/2 only. Do not tick original 30 QDNF packages.
- Changed files, public behavior and integration source state:
  - `crates/qualia-core-db/src/p2p/connectivity/h2_capsule.rs` (430 lines). `#![cfg(not(target_arch = "wasm32"))]`.
  - `crates/qualia-core-db/src/p2p/connectivity/cscp_h2.rs` (253 lines). `#![cfg(not(target_arch = "wasm32"))]`.
  - This handoff. Forbidden paths (`mod.rs`, `Cargo.toml`, `evidence.rs`, `capsule.rs`, `cscp_wss.rs`, `lib.rs`) **not** edited in the commit.
  - Public API:
    - `cscp_then_qsession_over_local_tls_h2() -> Result<SessionBinding, String>`
    - Carrier: `loopback_tls_h2_capsule`, `loopback_tls_h2_connect`, `CapsuleEndpoint::{send_payload, send_raw_capsule, recv_payload}`, `CAPSULE_PROTOCOL = "capsule"`.
  - Sequence (copy of `cscp_wss.rs`, carrier only changed):
    1. Pin local CA (`wss_tls::mint_ca` / `mint_server`); rustls **ring**; ALPN `h2`; tokio-rustls 0.26.
    2. Server HTTP/2 SETTINGS `SETTINGS_ENABLE_CONNECT_PROTOCOL=1` (`h2::server::Builder::enable_connect_protocol`).
    3. Client waits until `is_extended_connect_protocol_enabled()` then sends RFC 8441 Extended CONNECT with `:protocol` = `capsule` to `https://localhost/` (loopback hostname; no public URL).
    4. Server rejects CONNECT unless `:protocol` is `capsule`.
    5. CSCP ConnectRequest / ConnectAccept as RFC 9297 DATAGRAM capsules via `encode_datagram_capsule` (values ≤ 1024).
    6. QSession `handshake_over_fragments` on the same CONNECT stream as DATAGRAM capsules (`PathClass::Relayed`, `ProtectionPolicy::RELAY_ONLY`, `public_dht` false, no Direct locator TLVs).
    7. `SessionBinding` Active after permit + keys + `admit_application`.
  - Nested recovery labelled degraded / not UDP / not MASQUE Internet in rustdoc. Honesty flags not written.
- Accepted predecessor/interface versions:
  - `cscp_wss.rs` CSCP/QSession sequence.
  - `wss_tls::{mint_ca, mint_server, server_config, client_config}` with ALPN `h2` cloned onto those configs.
  - `fabric/capsule.rs` DATAGRAM encode for CSCP-sized values; unknown-critical type `0x40` via `encode_capsule`.
  - Integrator-owned Cargo deps already in the working tree (not committed here): `h2` 0.4.15, `http` 1, `bytes` 1, `tokio-rustls` 0.26.4; `rustls` 0.23 ring.
- Evidence manifest paths:
  - `/opt/cursor/artifacts/cscp-09-h2-cargo-test.log` — 4 passed with temporary `mod.rs` wiring, then reverted.
- Checks executed with outcomes; checks not executed and why:
  - Temporary `pub mod cscp_h2;` + `pub mod h2_capsule;` in `connectivity/mod.rs` for compile only; **reverted before commit**.
  - `CARGO_TARGET_DIR=/tmp/qdnf-continue-target cargo test -p qualia-core-db --lib --offline p2p::connectivity::cscp_h2 -- --test-threads=1` → **4 passed**, 0 failed, 8481 filtered, 0.21s:
    1. Happy path: ConnectRequest/Accept then QSession `SessionState::Active`; `!public_relay_dialed()`; `!masque_bound_udp_internet_executed()`.
    2. Wrong `:protocol` (`websocket`) rejected.
    3. Unknown-critical capsule type `0x40` rejected (`unknown critical capsule`).
    4. Honesty flags remain false.
  - After revert, the crate filter will match **0 tests** until the integrator adds the mods (same Cargo zero-match behavior as CSCP-09 framing).
  - Not executed: public URL, MASQUE HTTP/3, QUIC, Internet honesty flags, identity:: tests.
- Measured resource bounds, enforcement limitations and error behavior:
  - Capsule value: CSCP uses fabric `MAX_CAPSULE_VALUE` = 1024 via `encode_datagram_capsule`. QSession fragments are MIN_QDNF_MTU (1280); the H2 stream encoder allows DATAGRAM values up to 2048 with the same RFC 9000 varint framing. This is still not UDP/MASQUE.
  - Fail closed: CONNECT without `:protocol=capsule`; truncated leftover DATA; unknown type `>= 0x40`; payload above stream cap.
  - Unknown non-critical (`0x01..=0x3f`) skipped on the stream. No `Vec` on the CSCP/QSession evaluator; channel pumps allocate (cold I/O).
  - Client `h2` 0.4 Builder has no SETTINGS_ENABLE_CONNECT_PROTOCOL setter. The client **requires** the server setting (`is_extended_connect_protocol_enabled`) before CONNECT, which is RFC 8441 §3. Server sends the setting.
- Review findings, fixes and independent reviewer:
  - Author-only. Independent reviewer is after integrator merge (brief). First compile needed `SendRequest::ready()` rebind (consumes `self`) and `unwrap_err` avoided (`CapsuleEndpoint` is not `Debug`).
- Provisional assumptions removed or still blocking acceptance:
  - Integration blocked on `pub mod h2_capsule;` and `pub mod cscp_h2;` in `crates/qualia-core-db/src/p2p/connectivity/mod.rs`.
  - Cargo.toml deps must remain as the integrator added them (h2/http/bytes/tokio-rustls). This commit does not include Cargo.toml.
  - Loopback `https://localhost/` is the TLS/H2 authority, not a public relay URL.

## Integration

- Shared exports/Cargo/profile/registry edits requested from their owner:
  - Integrator: add to `p2p/connectivity/mod.rs`:
    ```
    pub mod cscp_h2;
    pub mod h2_capsule;
    ```
  - Optional: `pub use cscp_h2::cscp_then_qsession_over_local_tls_h2;`
  - Confirm Cargo.toml already has `h2 = "0.4.15"`, `http = "1"`, `bytes = "1"`, `tokio-rustls = "0.26.4"` under native deps.
  - Do **not** set `public_relay_dialed` or `masque_bound_udp_internet_executed` true. Do not invent a relay URL.
- Consumer packages and exact interface migration needed:
  - None. Callers use `cscp_then_qsession_over_local_tls_h2` the same way as `cscp_then_qsession_over_local_tls_wss`.
- Merge/conflict concerns and concurrent changes preserved:
  - New files only. `mod.rs` left untouched in the commit. Concurrent DID-QI / registry / Cargo.toml work in the working tree was not included.
- Rollback/recovery and durable schema compatibility:
  - Delete the two `.rs` files and this handoff. No on-disk schema. No QUIC.
- Outstanding ownership, temporary artifacts and cleanup:
  - `/tmp/qdnf-continue-target` reused per brief. `/tmp/cscp-09-h2-save` scratch copies of the `.rs` files. Integrator owns `mod.rs` / `evidence.rs` / Cargo.toml.
- Next owner, next action and proposed registry transition:
  - Integrator: wire the two mods, rerun
    `CARGO_TARGET_DIR=/tmp/qdnf-continue-target cargo test -p qualia-core-db --lib --offline p2p::connectivity::cscp_h2 -- --test-threads=1`,
    then move CSCP-09.02 to `review`. **Do not** mark parent CSCP-09 complete for MASQUE/Internet HTTP/3.
  - Proposed checked child: CSCP-09.02 loopback TLS HTTP/2 Extended CONNECT + capsules + CSCP/QSession.

Honesty: **local TLS HTTP/2 capsules are not MASQUE bound-UDP Internet and not QUIC.**
