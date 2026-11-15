# Wave S3.3 — two-process framed QBDL

Two OS processes exchange a useful framed QBDL record: Open → Grant ack → Credit → RequestLeaves → LeafData for one selected entry and the tail leaf (19 bytes). Parent owns the Windows named pipe (`CreateNamedPipeW` / `CreateFileW`). HTTP/DNS/IP/libp2p/WSS TCP are not used. In-process `ipc_pair` is not a fallback. The E05.2 8-byte Unix ping is not the payload.

Evidence class is `Process`. `physical_two_host_from_this_module()` is `false`. Windows two-process is a distinct flag from Linux/WSL Unix-domain. Neither is PhysicalNetwork. This Windows host does not execute the `#[cfg(unix)]` spawn; those tests are compile-gated only.

Native interchange stays length-prefixed binary / existing `qbdl/wire`. JSON is not a control channel. No new magics, BearerProfile, or service IRI.

## Files written / edited

New:

- `crates/qualia-core-db/src/net/qdnf/bearer/local_stream/two_process.rs` — OS spawn, parent bind, child `from_exported` / connect, 15s accept / 5s IO watchdogs
- `crates/qualia-core-db/src/net/peer/replication/qbdl/service/two_process.rs` — QBDL protocol (parent provider, child client), independent fixture reconstruction
- this summary

Edited:

- `crates/qualia-core-db/src/net/peer/replication/qbdl/service/session.rs` — `Session<'a, B: Bearer = IpcEndpoint>` + WouldBlock retry
- `crates/qualia-core-db/src/net/peer/replication/qbdl/service/client.rs` — methods take `Session<'_, B: Bearer>`
- `crates/qualia-core-db/src/net/peer/replication/qbdl/service/provider.rs` — same
- `crates/qualia-core-db/src/net/peer/replication/qbdl/service/mod.rs` — generic `transfer_one_leaf` + `#[cfg(test)] mod two_process;`

Not edited (integrator-owned or forbidden): `local_stream/mod.rs`, `local_stream/{windows,unix,frame,tests}.rs`, `bearer/mod.rs`, `qbdl/mod.rs`, `qbdl/{wire,carrier,provider}/**`, `registries.rs`, `bearer/ipc.rs`, `Cargo.toml`, `Cargo.lock`, git, task-registry.

`service/mod.rs` already routes `two_process`. Existing `service/tests.rs` five tests are unchanged and still use `Session<'_>` (= `IpcEndpoint` default).

## Integrator patch (`local_stream/mod.rs`)

After the existing `windows` / `unix` modules:

```rust
#[cfg(test)]
mod two_process;
```

And public Windows / Unix types so `qbdl/service/two_process.rs` can name them:

```rust
#[cfg(all(windows, target_arch = "x86_64"))]
pub use windows::{
    child_connect, framed_connect, framed_named_pipe_pair, named_pipe_pair, NamedPipeListener,
    NamedPipeStream, PipeName, PIPE_NAME_PREFIX,
};

#[cfg(unix)]
pub use unix::{UnixIpcListener, UnixStreamBytes, SOCKET_FILE};
```

`FramedStream` is already public from `local_stream/mod.rs`. Cargo remains integrator-owned. This child did not run `cargo`.

## Session genericization

```rust
pub struct Session<'a, B: Bearer = IpcEndpoint> { endpoint: B, ... }
impl<'a, B: Bearer> Session<'a, B> {
    pub fn attach(endpoint: B, ...) -> Result<Self, Error>
}
```

Existing in-process tests keep compiling as `Session<'_>`. Named-pipe `FramedStream<NamedPipeStream>` and Unix `FramedStream<UnixStreamBytes>` also implement `Bearer`. Wire/carrier were not rewritten.

## WouldBlock policy

`windows.rs` uses `PIPE_NOWAIT`. `FramedStream::send` is retry-safe for the **same** payload (`WriteCursor::seal` resumes; a different payload is `Conflict`). `recv` returns `WouldBlock` while assembling (`ReadCursor` keeps the buffer).

Previously `session.rs` mapped every `endpoint.send` / `recv` error through `fail_close`, which killed named-pipe transfer.

Now:

- `QdnfError::WouldBlock` → retry the **same** sealed frame / recv into the **same** buffer
- finite IO deadline 5s, poll sleep 1ms
- do **not** reseal, rewind, or `fail_close` on WouldBlock
- deadline expiry → `fail_close(Error::Expired)`
- any other send/recv error → `fail_close` as before
- accept loop (parent): 15s watchdog, 5ms poll; kill/wait child on timeout
- `ipc_pair` only WouldBlocks on an empty/full queue; in-process tests stay sequential ping-pong and do not hit the retry path

No busy-spin without a deadline.

## Two-process protocol

Cannot call `transfer_one_leaf` across processes (single-threaded ping-pong). Parent and child run concurrently:

1. Child first line: if `QDNF_TWO_PROCESS_CHILD` is exactly `"1"`, run client path and return (no recursive spawn). `AtomicBool SPAWNED` as in E05.2.
2. Parent = provider. Reconstructs `payload()` (`0xA1` / `0xDEADBEEF` / `0xB2` / `0x0DD0FEED`, 3 leaves) independently. Holds `LocalWholeSourceGrant`. Binds first. Does not send first. After accept: `answer_open` then `answer_leaf` (WouldBlock retries live inside `Session`).
3. Child = client. Same fixture reconstructed independently (no source bytes via env/JSON). `PipeName::from_exported` then `child_connect` (never bind, never TCP). `send_open` → `recv_grant_send_credit_request` → `recv_leaf` for the tail leaf (19 bytes). Unrelated `0xA1` / `DEADBEEF` / `0DD0FEED` bytes stay unrequested.
4. Parent generates `\\.\pipe\qbdl-{pid}-{tid}-{seq}`. Env: `QDNF_TWO_PROCESS_CHILD=1` and `QDNF_TWO_PROCESS_PIPE=<PipeName::as_text()>`. Unix: `QDNF_TWO_PROCESS_SOCK` + `UnixIpcListener`.
5. Spawn: `std::env::current_exe()` args `["--exact", test_name, "--nocapture", "--test-threads", "1"]`. Inherit stdio.
6. Both wrap with `framed_connect` / `FramedStream::connect`, MTU 1280, `BearerProfile::LocalIpcV1`.
7. Transcript `StrongDigest([9; 48])` is a test constant in both processes, not a secret over env.
8. Bind/ACL/connect failure → `PlatformUnsupported` / `Denied` / test fail. Never `ipc_pair` or `127.0.0.1`.

Carrier frames remain 88+1024=1112 ≤ MTU 1280 ≤ `FRAME_CAP` 8192. PID/OS credential is adapter-observed locator, not cryptographic peer authority.

## Test names

`qbdl/service/two_process.rs` (the QBS-09 item 3 gate):

- `two_os_process_transfers_useful_leaf` — `#[cfg(all(windows, target_arch = "x86_64"))]` and `#[cfg(unix)]` (same name, mutually exclusive)
- `child_refuses_tcp_and_foreign_pipe_name` — `from_exported("127.0.0.1:0")` and `\\.\pipe\not-ours` → `Denied` (unit, no spawn). Unix: `UnixStreamBytes::connect` of those paths → `Denied`
- `wire_is_length_prefixed_binary_not_json` — first four bytes `u32be` length, not `{`

`local_stream/two_process.rs` (OS spawn owner; not the useful QBDL record):

- `two_os_process_exchanges_length_prefixed_frame` — two OS processes, framed binary payload, profile `LocalIpcV1`, evidence Process
- `child_refuses_tcp_and_foreign_pipe_name`
- `wire_is_length_prefixed_binary_not_json`

Existing `service/tests.rs` (must keep passing semantically):

- `useful_leaf_transfers_while_unrelated_payload_stays_unrequested`
- `malformed_prefix_refuses_and_fail_closes`
- `cancel_fail_closes_without_leaf_disclosure`
- `close_fail_closes_session`
- `mixed_private_protected_wraps_are_unsupported`

## Residuals

- Cargo / git not run. Integrator routes `local_stream/mod.rs` pub-use and runs cargo.
- Unix two-process is compile-gated; this Windows host does not produce Linux/WSL evidence.
- Windows pass does not set Linux; Unix pass does not set Windows; neither sets PhysicalNetwork.
- Production `write_leaf` still holds StorageStage across OS wait.
- Remote R2 signing / current grant not started; local-admin profile still `LocalWholeSourceGrant`.
- QBS10 consumed common channel / one `PacketProtection` not started (frames here are plaintext `ServiceRecordHeader` + chunk over the local stream).
- Describe/DescriptorPage/Select/Resume/Outcome not in this transfer.
- Session genericization did not force a wire/carrier rewrite.
