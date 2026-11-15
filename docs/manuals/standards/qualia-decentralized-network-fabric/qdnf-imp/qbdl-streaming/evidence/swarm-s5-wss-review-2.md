---
pass: true
open_bugs: 0
---

# Wave S5 / QBS-26 — WSS framing re-review (BUG-1 patch)

Read-only. Production Rust was not edited. Cargo was not run. No commit/push.
Previous review `swarm-s5-wss-review.md` was `pass: false` `open_bugs: 1`
because `seal_close` aborted a pending datagram and resealed opcode-8.
Author summary is claims, not evidence.

Current source read:

- `crates/qualia-core-db/src/p2p/connectivity/wss.rs`
- `crates/qualia-core-db/src/p2p/connectivity/wss_frame.rs`
- `crates/qualia-core-db/src/p2p/connectivity/wss_bearer.rs`
- `wss_tls.rs` residual only; `mod.rs:18-20` still `pub mod wss;` /
  `wss_bearer;` / `wss_tls;` (no `wss_frame` registry)

## BUG-1 closed

`seal_close` (`wss.rs:163-183`) has no `abort()` on the production path.
Pending close resumes; pending non-close is `Conflict` without reseal:

```text
if write.pending() {
    if write.sealed_opcode() == Some(OP_CLOSE) { return Ok(()); }
    return Err(QdnfError::Conflict);
}
```

(`wss.rs:169-174`). Idle path still encodes `FrameType::Close` and seals
`OP_CLOSE` with a fresh client mask (`wss.rs:175-182`). `WriteCursor::abort`
remains only after a completed `flush` (`wss_frame.rs:293-301`) and on
`ReadCursor` parse failure — not in `seal_close`.

`pending_datagram_then_seal_close_is_conflict` (`wss.rs:641-659`): partial
flush, then `seal_close` is `Conflict`; `sealed_bytes()` and `accepted()`
unchanged; `sealed_opcode()` stays `OP_BINARY`.

Pending close still resumes: `close_frame_then_shutdown` (`wss.rs:662-693`)
WouldBlock-flushes Close, calls `seal_close` again, and asserts the sealed
bytes are identical (`wss.rs:673-675`).

`WssBearer::shutdown` (`wss_bearer.rs:104-109`) uses this `seal_close`. A
pending datagram is `Conflict`, not an opcode-8 reseal onto a truncated header.

## Prior must-confirms still hold

1. **Fresh client mask.** `fresh_client_mask` (`wss_frame.rs:29-33`) via
   `fill_os`. Idle `seal_datagram` (`wss.rs:151-157`) / `seal_close`
   (`wss.rs:177-181`). Pending datagram skips `fill_os` (`wss.rs:151-152`).
   Server `None`.
2. **WouldBlock resume.** `WriteCursor::seal` same opcode+payload resumes;
   different payload is `Conflict` without rewind (`wss_frame.rs:250-263`).
   `would_block_write_resumes_same_masked_payload` (`wss.rs:577-603`) keeps
   the original mask after a fake `[1,2,3,4]` reseal.
   `would_block_read_resumes_same_frame` (`wss.rs:605-638`).
3. **Envelope c/g.** `check_circuit_gen` (`wss.rs:116-128`).
   `stale_generation_or_wrong_circuit_refused` (`wss.rs:695-717`). Bearer
   bound `(1, 1)` (`wss_bearer.rs:46-47,75,85-86,106`).
4. **Close + TCP half-close.** `close_and_shutdown` (`wss.rs:378-390`) then
   `TcpStream::shutdown(Write)`. Bearer sends Close; generic `S` is not
   half-closed. `TlsStream` residual unchanged.
5. **Strict framing / no JSON / `TlsWssTransitionV1` / no second registry.**
   Unchanged from the first review.

## Required tests present (names; cargo not run)

- `would_block_write_resumes_same_masked_payload` (`wss.rs:577`)
- `would_block_read_resumes_same_frame` (`wss.rs:605`)
- `pending_datagram_then_seal_close_is_conflict` (`wss.rs:641`)
- `close_frame_then_shutdown` (`wss.rs:662`)
- `stale_generation_or_wrong_circuit_refused` (`wss.rs:695`)
- `server_frames_unmasked` (`wss.rs:720`)
- `fresh_mask_differs_across_client_frames` (`wss.rs:538`)

## Residuals (not counted)

Same as first review: HTTP upgrade `write_all`; `recv_datagram` Vec copy;
QBS record / current-authority out of slice; `WssBearer<TlsStream>` no inner
TCP half-close; helper `send_datagram` / `close_and_shutdown` use a local
`WriteCursor`.

`WssBearer::shutdown` sets `closing = true` before `seal_close`
(`wss_bearer.rs:105-106`). A premature shutdown after `send` WouldBlock
returns `Conflict` and later `send` is `Closed`. Resume contract remains
retry `send` with the same payload; the cursor is not aborted. Not counted.

## Verdict

BUG-1 is gone in current source. Cursor/`seal_close` WouldBlock contract
holds. Cargo still not run in this review.
