# Wave S3.1 / child A — local_stream frame codec

Length-prefixed framing over a reliable ordered **byte stream**. Partial I/O cursors. WouldBlock advances the cursor only on accepted bytes. Drain/shutdown. No OS handles. Named-pipe (`windows.rs`) and UDS (`unix.rs`) are later backends.

Native interchange is exact-length binary (`u32be length || payload`). Not JSON. Not CBOR maps. Handshake datagram reassembly stays in `bearer/fragment.rs` and was not copied.

Profile is existing `BearerProfile::LocalIpcV1`. `registries.rs` was not edited.

## Files written

- `crates/qualia-core-db/src/net/qdnf/bearer/local_stream/mod.rs`
- `crates/qualia-core-db/src/net/qdnf/bearer/local_stream/frame.rs`
- `crates/qualia-core-db/src/net/qdnf/bearer/local_stream/tests.rs`
- this summary

`windows.rs`, `unix.rs`, `qbdl/service/**`, `bearer/mod.rs`, and Cargo were **not** edited.

## Key decisions

- Wire: `u32be payload_len || payload`. Payload cap `FRAME_CAP = 8192` (same as `ipc.rs`). Wire cap `WIRE_CAP = 8196`. Length is payload bytes, not including the prefix.
- Truncated prefix/body → `QdnfError::Truncated`. Trailing bytes on exact decode → `Malformed`. Declared length or payload above `FRAME_CAP` → `Capacity`.
- `WriteCursor` seals once into a fixed `[u8; WIRE_CAP]`. WouldBlock retains those exact bytes. `off` advances only on `Ok(n > 0)`. A different payload while pending is `Conflict` and does not reseal or rewind.
- `ReadCursor` assembles prefix then body. Oversize length poisons the cursor (does not treat payload bytes as a new prefix). Short `out` after a complete frame is `Capacity` and does not drop it. Idle EOF is `Closed`; EOF mid-frame is `Truncated`.
- `StreamBytes` is the byte-pipe trait later OS adapters implement. `FramedStream<T: StreamBytes>` implements `Bearer`: `send` is atomic from the caller’s view (full frame or WouldBlock with retained sealed write); `RecvMeta.observed_source` is the constructor peer locator, never a payload field.
- Drain: `begin_drain` Ready → Draining; new send is Closed; recv still delivers complete frames. `shutdown` aborts both cursors, closes lifecycle, then `StreamBytes::shutdown`. Drain after shutdown is Closed.

## Exports for `windows.rs` / `unix.rs`

```rust
use super::{
    decode_stream_frame, encode_stream_frame, next_stream_frame, FramedStream, ReadCursor,
    StreamBytes, WriteCursor, FRAME_CAP, LEN_SIZE, WIRE_CAP,
};
use super::super::contract::{Bearer, RecvMeta};
```

Implement `StreamBytes` on the OS handle (`write` / `read` / `shutdown`). Map `ERROR_NO_DATA` / `EAGAIN` to `WouldBlock`, broken pipe / EOF to `Closed`, `Ok(0)` on read to orderly EOF. Then `FramedStream::connect(handle, local, peer, scope, mtu)`. Observed locators come from the adapter (pipe/UDS peer credentials), never from framed bytes. No IP/DNS/TCP fallback. Do not reuse `bearer/fragment.rs`.

`FramedStream::write_cursor` / `read_cursor` / `inner_mut` / `begin_drain` / `drain_write` are the flush and drain sites.

## Requested integrator patch

`crates/qualia-core-db/src/net/qdnf/bearer/mod.rs` (after `pub mod ipc;`):

```rust
pub mod local_stream;
```

Recommended re-exports (optional):

```rust
pub use local_stream::{
    decode_stream_frame, encode_stream_frame, FramedStream, ReadCursor, StreamBytes, WriteCursor,
    FRAME_CAP as STREAM_FRAME_CAP, WIRE_CAP,
};
```

No `Cargo.toml` change. Tests will not compile until `mod local_stream` is routed.

Focused selector after routing: `net::qdnf::bearer::local_stream`

## Test names

`local_stream/tests.rs` (submodule of `local_stream/mod.rs`):

- `split_writes_retain_sealed_pending_across_partial_accept`
- `split_reads_assemble_one_length_prefixed_frame`
- `would_block_then_resume_does_not_reseal_or_drop_cursor`
- `oversize_payload_and_declared_length_are_capacity`
- `drain_after_shutdown_is_closed`
- `encode_is_exact_length_binary_not_json`
- `truncated_and_trailing_refuse_without_payload_view`
- `short_recv_buffer_does_not_drop_completed_frame`
- `framed_stream_reuses_local_ipc_profile_and_observed_locator`

## Stop notes

- Did not edit `bearer/mod.rs`, `registries.rs`, `windows.rs`, `unix.rs`, `qbdl/**`, or `Cargo.toml`.
- Did not copy handshake fragment reassembly.
- Did not run `cargo test` / `cargo check` / `cargo build`.
- No git commit/push. Synth MCP was not touched.

Formatted only the three new Rust files with `rustfmt --edition 2021`.
