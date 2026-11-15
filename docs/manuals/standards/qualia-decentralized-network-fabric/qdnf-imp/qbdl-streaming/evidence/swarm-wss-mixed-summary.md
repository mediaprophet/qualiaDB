# Swarm Wave 3 — WSS mixed-HMC parity (HMC review §6 item 8)

File-disjoint. **Did not cargo. Did not rustc. Did not commit.** Exclusive writes: `bundle/stream/http/wss_mixed.rs`, this summary. Integrator has **not** routed `mod wss_mixed;` in `http/mod.rs` (adds that line after this child stops).

## Files written

| Path | Role |
|---|---|
| `crates/qualia-core-db/src/bundle/stream/http/wss_mixed.rs` | Loopback `WssBearer<TcpStream>` pairing plus `#[cfg(all(test, not(target_arch = "wasm32")))]` tests (four mixed QAC/EMF/Q42) |
| this summary | |

Not edited: `http/mod.rs`, `http/{range,status,client,provider,tests,mixed}.rs`, `p2p/connectivity/wss.rs`, `wss_bearer.rs`, `wss_tls.rs`, `cscp_wss.rs`, `qbdl/service/**`, `formats/**`, `mixed/**`, codec-owner trees, Cargo.toml/lock, `registries.rs`.

No `BearerProfile` 8+. Profile stays `TlsWssTransitionV1`. JSON is not an application object. Kind 1–12 frozen.

## WssBearer pairing (loopback)

EvidenceClass::**Component**. Never Process. Never PhysicalNetwork. Same host.

1. `TcpListener::bind("127.0.0.1:0")` then `TcpStream::connect` / `accept`.
2. Wrap each side in `crate::p2p::connectivity::wss_bearer::WssBearer::new(stream, local, remote, scope, client)`.
3. Locators / `ScopeEpoch` match `p2p/connectivity/establish.rs` and `cscp_wss.rs` (`dest_a=0x01`, `dest_b=0x02`, `scope=1, epoch=1`).
4. `Session::attach` already takes `Bearer`. Client peer is the server locator; provider peer is the client locator. `Session.profile()` must be `TlsWssTransitionV1`.

No TLS handshake in this slice (raw TCP + WSS frame codec). TLS/Web PKI is not PQ publisher authority. `loopback_tls_wss` stays in `wss_tls.rs` (not edited).

## Mux vs HTTP

`HttpMux` / `serve_scheduled` are HTTP-only (`http/mixed.rs`). WSS uses **native mux over WssBearer sessions**:

- `MuxTransfer::compile` / `next` / `acknowledge` / `push_control`
- `transfer_scheduled` from `qbdl::service` (already routed)
- `PlainProvider` / `PlainClient` / `BoundEntry`
- Session framing is QBDL records (Kind 1–12). A WSS datagram is not a second application codec.

`transfer_scheduled` owns both `ServiceIo` ends and ping-pongs Open/Grant/Credit/RequestLeaves/LeafData in one thread. Same-thread loopback TCP does not need a second host process. A provider-loop thread would still be Component (same host) if duplex blocking required it; this slice did not spawn one.

Leaf payloads are intercepted after Session unframing (`LeafCapture: ServiceIo`) so bodies can be compared to `select_entry` slices. That wrapper is not a codec.

## Multi-entry HMC

Local `PlainPublisher` + `BundleWriter` (same pattern as mux `Scene::publish` / HTTP mixed). Does not depend on F `default_mixed_entries` tempfile.

Family tags `Qac` / `Emf` only after `select_entry` **and** admit:

- QAC: `QacStreamView::admit` (complete-entry) or `QacStorageView::open` on the whole entry (`logical_length == bytes.len()`).
- EMF: `EmfStreamView::admit` (envelope v4). Not `EmfDirectoryView`.
- Q42: whole-entry `AuthenticatedRange` + `Q42AuthenticatedRangeSource::new`. HTTP Range parity stays in `http/mixed.rs`.
- Failed admit → omit from `present[]`, never transfer as that family.

| File | Role |
|---|---|
| `qac_candidate-opaque.qac` | live QAC WSS object |
| `qac_candidate-pcm-signed-zero.qac` | live QAC WSS object |
| `emf_candidate-opaque.emf` | live EMF WSS object |
| `emf_candidate-aligned4096.emf` | live EMF WSS object |
| `write_unified_volume` `.q42` | live Q42 whole-entry leaf (not HTTP Range) |

Two QAC + two EMF are **distinct objects**. HMC rule: many entries per family. `include_bytes` path from `http/wss_mixed.rs` is `../../../q42/field_envelope/fixtures/candidate/...`.

Grant closures wrap service `Error` as `Ok((|| -> Result<(), Error> { ... })())` is not required here: tests call `transfer_scheduled` and `match` on `Result`. No `Debug` on `MuxTransfer` / `BoundEntry` / `MuxReport`.

## Tests written **not run**

`#[cfg(all(test, not(target_arch = "wasm32")))]`. Comment at top: `EvidenceClass::Component`. `std::net` gated. Tests do not compile until the integrator adds `mod wss_mixed;`.

| Test | What it asserts |
|---|---|
| `wss_two_qac_and_two_emf_match_native_bytes` | Four objects over WssBearer `Session`; each LeafData body equals `select_entry` slice; each admits as `QacStreamView` / `EmfStreamView`; four distinct ids/roots; Control first if enqueued |
| `wss_delays_m64_while_ready_qac_emf_continue` | QAC + EMF + M64; `delayed_deferred >= 1`; M64 only after live families |
| `wss_q42_entry_bytes_match_native` | Real `write_unified_volume` Q42 in a mixed HMC; whole-entry WSS leaf equals native / `select_entry` |
| `wss_incomplete_mandatory_is_not_a_complete_answer` | Missing mandatory Q42 calib; `Error::Admission`; payload buffer unchanged |

Did not copy Open/Credit/RequestLeaves encoding. Did not invent Kind 13+. Did not mint BearerProfile 8+.

## Integrator

```rust
// bundle/stream/http/mod.rs — after this child stops
mod wss_mixed;
```

Do not `pub use` from `http/mod.rs`. Tests live inside `wss_mixed.rs`; there is no `wss_mixed_tests.rs`.
