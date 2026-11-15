---
pass: false
open_bugs: 1
---

# Wave S5 / QBS-26 — WSS framing repair independent review

Read-only source review. Production Rust was not edited. Cargo was not run.
Synth MCP was not killed. Author summary (`swarm-s5-wss-summary.md`) is claims,
not evidence. Handoff E and HMC §4 asked for fresh client masking, retained
partial I/O cursors, strict framing, actual shutdown, and circuit/generation
binding.

Reviewed:

- `crates/qualia-core-db/src/p2p/connectivity/wss.rs`
- `crates/qualia-core-db/src/p2p/connectivity/wss_frame.rs` (`#[path]` from `wss.rs:17-18`)
- `crates/qualia-core-db/src/p2p/connectivity/wss_bearer.rs`
- `crates/qualia-core-db/src/p2p/connectivity/wss_tls.rs` (residual only)
- `crates/qualia-core-db/src/p2p/connectivity/mod.rs` (must stay unedited)

`mod.rs` still has `pub mod wss;` / `wss_bearer;` / `wss_tls;` only (no
`wss_frame` registry). `WssBearer::profile` is existing
`BearerProfile::TlsWssTransitionV1` (`wss_bearer.rs:55-56`). No JSON/serde in
the WSS files. No second registry.

## Must-confirm

### 1. Fresh client mask — holds

`fresh_client_mask` (`wss_frame.rs:29-33`) fills `[u8; 4]` via
`crate::crypto::network::entropy::fill_os`. That path is OS CSPRNG only
(`entropy.rs:14-22`): `getrandom::fill` failure zeroizes and returns
`CryptoError::EntropyFailure`. Mapped to `QdnfError::EntropyFailure`. No
`[1,2,3,4]` production mask (that literal appears only in tests as a negative
and as a discarded reseal argument).

Idle client seals take a new mask: `seal_datagram` (`wss.rs:151-157`),
`seal_close` (`wss.rs:177-181`), `send_auth` (`wss.rs:337`). Pending datagram
skips `fill_os` (`wss.rs:151-152`). Server path passes `None` (`wss.rs:155-157`,
`wss.rs:701` `client=false`).

### 2. Hot-path I/O / WouldBlock resume — **not closed** (see BUG-1)

`WriteCursor` / `ReadCursor` are `[u8; WIRE_CAP]` (`wss_frame.rs:190-194`,
`332-341`). `flush` uses `sink.write` (`wss_frame.rs:276-291`), `pull` uses
`src.read` (`wss_frame.rs:488-514`). No `write_all` / `read_exact` /
`payload.to_vec()` / `vec![0; len]` on the cursor or `WssBearer` send/recv
path.

`WriteCursor::seal` resumes the same opcode+payload without resealing; a
different payload is `Conflict` and does not rewind `off` (`wss_frame.rs:253-263`).
`would_block_write_resumes_same_masked_payload` (`wss.rs:577-603`) keeps the
original mask after a fake reseal with `[1,2,3,4]`.

`seal_close` bypasses that contract: pending non-close is `abort()` then a new
opcode-8 seal (`wss.rs:169-182`). That is BUG-1.

### 3. Envelope `c/g` — holds

`check_circuit_gen` (`wss.rs:116-128`): wrong circuit `Unauthorized`, any other
generation `StaleGeneration`. `recv_checked` (`wss.rs:206-207`) applies it
before delivering. `WssBearer` send/recv/close use bound `(1, 1)`
(`wss_bearer.rs:46-47,75,85-86,106`). `serve_pair` checks before forward
(`wss.rs:410,417`). QBS record / current-authority bind is out of slice
(documented residual; `WssBearer::new` signature unchanged).

### 4. Close + TCP half-close — holds except BUG-1 interleaving

`seal_close` encodes `FrameType::Close` in the versioned 12-byte envelope and
seals `OP_CLOSE` (`wss.rs:175-182`, `wss_frame.rs:18-19,168`). Pending close
resumes (`wss.rs:170-172`). `close_and_shutdown` then
`TcpStream::shutdown(Shutdown::Write)` (`wss.rs:378-390`).
`WssBearer::shutdown` sends Close and flushes; it does not half-close generic
`S` (`wss_bearer.rs:104-110`). `TlsStream` has no TCP shutdown (documented).

### 5. Strict framing — holds

`header_need` (`wss_frame.rs:87-109`): RSV `Malformed`; FIN=0 `Unsupported`;
opcode other than binary/close `Unsupported`; `len7==127` `Unsupported`.
`parse_header` payload `> MAX_FRAME` `Capacity` (`wss_frame.rs:126-128`).
`encode_ws_frame` never emits 8-byte length (`wss_frame.rs:154-174`). Envelope
v1 `buf[0]==1` is `obsolete-u8-length` → `Downgrade` (`wss.rs:91-92,186-187`).
Covered in `u16_length_keeps_256_and_rejects_oversize` (`wss.rs:508-536`) for
v1, 8-byte length, FIN=0. RSV is code-only (no named test); still refused.

### 6. No JSON / no new profile / no second registry — holds

Binary 12-byte envelope (`wss.rs:27-29,63-84`). Profile stays
`TlsWssTransitionV1`. `mod.rs` unedited by this wave’s split (`#[path]`).

### 7. Required tests exist in source — holds (names only; cargo not run)

In `wss.rs` `cfg(test)`: `fresh_mask_differs_across_client_frames` (538),
`would_block_write_resumes_same_masked_payload` (577),
`would_block_read_resumes_same_frame` (605), `close_frame_then_shutdown` (640),
`stale_generation_or_wrong_circuit_refused` (674), `server_frames_unmasked`
(698). No named test that a *different* pending payload returns `Conflict`.

### 8. Documented residuals — match code, not counted

- HTTP upgrade `write_all`: `wss.rs:234,269`.
- `recv_datagram` stack `[u8; MAX_FRAME]` then `vec![0u8; p.len()]` for existing
  callers: `wss.rs:305-322`.
- QBS record / current-authority bind out of slice; bearer `(1, 1)`.
- `WssBearer<TlsStream>` cannot half-close inner TCP (`wss_tls.rs:99-127`).

`send_datagram` / `close_and_shutdown` still construct a *local* `WriteCursor`
and drop it on return (`wss.rs:291-301,385-389`). Blocking helper boundary;
`WssBearer` retains cursors. Not counted (same class as the Vec residual).

## Open bugs

### BUG-1 — `seal_close` aborts an in-flight datagram (WouldBlock progress lost)

`wss.rs:169-174`:

```text
if write.pending() {
    if write.sealed_opcode() == Some(OP_CLOSE) { return Ok(()); }
    write.abort();
}
```

`WriteCursor::abort` (`wss_frame.rs:298-301`) zeros `len`/`off` but cannot
unwrite bytes already accepted by `flush` (`wss_frame.rs:284`). `WssBearer::send`
returns `WouldBlock` with `off > 0` (`wss_bearer.rs:75-76`). A following
`shutdown` (`wss_bearer.rs:104-108`) reseals opcode-8 onto a truncated WS
header. Item 2 requires a different pending payload to be `Conflict` without
reseal or rewind (`wss_frame.rs:250-263`). Author summary calls abort
intentional; it does not document partial-flush concatenation. Not in the
allowed residual list. Handoff E’s original defect was losing WouldBlock
progress.

Fix (not applied): if `pending && opcode != CLOSE`, return `Conflict` (or
finish the sealed datagram, then close). Never `abort()` after `off > 0` and
then write a new frame.

## HMC §4

Fresh masking, retained cursors, strict framing, Close, and `c/g` checks are
present on the Bearer path. Older fixed-mask / `write_all` / `read_exact` WS
hot path is gone. Do not treat this wave as QBS-ready until BUG-1 is closed:
shutdown after a partial datagram write still loses the sealed frame.
