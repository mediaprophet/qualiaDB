# Wave S5 / QBS-26 — WSS incremental framing repair

Repair of the existing RFC 6455 adapter: fresh per-frame client masks, bounded
retained read/write cursors, envelope `c/g` checks, and a real Close + TCP
half-close. Native interchange is the existing versioned 12-byte binary envelope
inside WS binary/close frames. Not JSON. Not CBOR maps. QBS record / current
authority bind is **not** in this slice.

WSS remains `BearerProfile::TlsWssTransitionV1`. TLS is not PQ publisher
authority.

## Files written

- `crates/qualia-core-db/src/p2p/connectivity/wss.rs` — envelope, handshake,
  `send_datagram` / `recv_datagram` / auth, `c/g` check, Close+half-close,
  tests. `#[path = "wss_frame.rs"]` so `connectivity/mod.rs` is untouched.
- `crates/qualia-core-db/src/p2p/connectivity/wss_frame.rs` — NEW. RFC 6455
  encode/decode, `WriteCursor` / `ReadCursor`, `fill_os` mask, WouldBlock
  resume. Split because framing + cursors would push `wss.rs` over ~500
  implementation lines.
- `crates/qualia-core-db/src/p2p/connectivity/wss_bearer.rs` — retained
  cursors, bound circuit/generation `(1, 1)`, `c/g` refuse, Close on
  `shutdown`, WouldBlock no longer mapped to `Closed`.
- this summary

Not edited: `connectivity/mod.rs`, `registries.rs`, `wss_tls.rs`, `cscp_wss.rs`,
`establish.rs`, `protection/**`, `policy/remote/**`, HTTP/LIG, Cargo.

## Defects closed

1. **Fresh client mask.** `send_datagram` / `send_auth` / `seal_datagram` /
   `seal_close` call `crate::crypto::network::entropy::fill_os` for a new 4-byte
   mask on each idle client seal. Entropy failure is `QdnfError::EntropyFailure`
   (fail closed). Server frames stay unmasked (`MASK=0`).
2. **Retained cursors.** `write_all` / `read_exact` and `to_vec` / `vec![0; len]`
   on the WS hot path are gone. `WriteCursor` seals once into `[u8; WIRE_CAP]`.
   WouldBlock keeps those exact bytes (including the mask). Resume with the same
   opcode+payload is not a reseal; a different payload is `Conflict`.
   `ReadCursor` assembles header then body; WouldBlock keeps `filled`.
3. **Envelope `c/g`.** `check_circuit_gen` / `recv_checked`: wrong circuit →
   `Unauthorized`; other generation → `StaleGeneration`. `WssBearer` send/recv/
   close use the bound `(1, 1)`. `serve_pair` refuses mismatched `c/g` before
   forward.
4. **Shutdown.** Opcode-8 Close carrying `FrameType::Close` + `c/g`. WouldBlock
   retries the same sealed close. `close_and_shutdown` then `TcpStream::shutdown(Write)`.
   `WssBearer::shutdown` sends that Close and flushes; it does not half-close a
   generic `S` (see residuals).
5. **Kept refusals.** FIN=0 (fragmented), RSV, opcode other than binary/close,
   8-byte length (127), payload `> MAX_FRAME`, obsolete envelope v1
   (`obsolete-u8-length` / `Downgrade`).

`loopback_ws_datagram`, `accept_key_rfc6455_sample`, and
`u16_length_keeps_256_and_rejects_oversize` remain.

## Test names (`wss.rs` `cfg(test)`)

- `accept_key_rfc6455_sample`
- `loopback_ws_datagram`
- `u16_length_keeps_256_and_rejects_oversize` (also 8-byte len + FIN=0)
- `fresh_mask_differs_across_client_frames`
- `would_block_write_resumes_same_masked_payload`
- `would_block_read_resumes_same_frame`
- `close_frame_then_shutdown`
- `stale_generation_or_wrong_circuit_refused`
- `server_frames_unmasked`

## API refused to invent

- Second bearer registry / new `BearerProfile` / `registries.rs`
- JSON or CBOR-map envelope
- QBS record bind, `CurrentAuthorityGuard`, credits (requires `stream/policy`)
- Changing `WssBearer::new` to take circuit/generation (would break
  `establish.rs` / `cscp_wss.rs`, outside write set). Bound `(1, 1)` internally.
- `HalfClose` trait on generic `S` (would break `WssBearer<TlsStream>`)
- Ping/pong auto-reply
- RFC 6455 2-byte status close payload (Close carries the versioned envelope)
- HTTP/LIG adapter (next serial slice)

`recv_datagram` still returns `Vec<u8>` for `two_process` / `cscp_wss`. The
frame is assembled on a stack `[u8; MAX_FRAME]`; the `Vec` is only the existing
call-boundary copy. HTTP upgrade still uses `write_all` / `read`.

## Residuals (still open)

- HTTP/LIG (next serial slice after this).
- QBS record bind with original budgets / current authority.
- `WssBearer<TlsStream>` inner TCP half-close (`wss_tls.rs` not in write set).
  Close frame is sent; `close_and_shutdown` half-closes plain `TcpStream`.
- Browser Origin / compression / 16 KiB QBS records / PQ-from-TLS claim:
  not introduced. Profile stays transition.

## What was not run

- `cargo test` / `cargo check` / `cargo build` (integrator owns the serial slot)
- git commit / push / deploy
- `synth.exe` was not killed

Formatted `wss.rs`, `wss_frame.rs`, and `wss_bearer.rs` with
`rustfmt --edition 2021`.
