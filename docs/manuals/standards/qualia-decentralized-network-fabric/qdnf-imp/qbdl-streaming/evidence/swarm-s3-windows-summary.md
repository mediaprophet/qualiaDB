# Wave S3.2 / Windows — named-pipe StreamBytes

Parent-owned `CreateNamedPipeW` with a restrictive current-user DACL. Parent generates `\\.\pipe\qbdl-…`. Child `CreateFileW` on that exact name. Length-prefixed framing via existing `FramedStream::connect`. No TCP/IP/DNS/libp2p fallback. PID/OS credential is adapter-observed locator, not cryptographic peer authority.

Evidence class: Windows process. Not Linux, not two-host, not two-process spawn (S3.3).

Profile is existing `BearerProfile::LocalIpcV1`. `registries.rs` was not edited.

## Files written

- `crates/qualia-core-db/src/net/qdnf/bearer/local_stream/windows.rs`
- this summary

`local_stream/mod.rs`, `frame.rs`, `tests.rs`, `unix.rs`, `qbdl/**`, `bearer/mod.rs`, `Cargo.toml`, and `registries.rs` were **not** edited.

## Requested integrator patch

`crates/qualia-core-db/src/net/qdnf/bearer/local_stream/mod.rs` (after `mod frame;`):

```rust
#[cfg(windows)]
mod windows;
```

`windows` crate in `Cargo.toml` is `cfg(all(windows, target_arch = "x86_64"))`. If a non-x86_64 Windows target must compile this crate, use `#[cfg(all(windows, target_arch = "x86_64"))] mod windows;` instead. Do not add `Win32_System_Pipes`.

Optional re-exports after routing:

```rust
#[cfg(windows)]
pub use windows::{
    child_connect, framed_connect, framed_named_pipe_pair, named_pipe_pair, NamedPipeListener,
    NamedPipeStream, PipeName, PIPE_NAME_PREFIX,
};
```

Tests in `windows.rs` will not compile until `mod windows` is routed.

## Test names

`windows.rs` (`#[cfg(all(test, windows))]`):

- `parent_generates_qbdl_pipe_name`
- `child_rejects_arbitrary_incoming_path`
- `tcp_dns_ip_paths_are_denied`
- `in_process_pair_exchanges_length_prefixed_frame`
- `framed_stream_reuses_local_ipc_profile`
- `observed_locator_from_adapter_not_payload`
- `idle_read_maps_error_no_data_to_would_block`
- `broken_pipe_maps_to_closed`
- `drain_after_shutdown_is_closed`

In-process pair is two handles on the same pipe. Two-process spawn is S3.3.

## ACL / path facts

- Parent `NamedPipeListener::bind` generates the name. There is no bind-path argument.
- Generated form: `\\.\pipe\qbdl-{pid:08x}-{tid:08x}-{seq:016x}` (hex + `-` only after the prefix).
- `PipeName::from_exported` refuses anything not that prefix plus hex/`-`. `\\.\pipe\not-ours`, filesystem paths, `127.0.0.1:0`, DNS, HTTP URLs → `Denied`. Empty suffix after prefix → `Malformed`.
- Child `CreateFileW` opens that exact exported name. `nMaxInstances = 1`, `FILE_FLAG_FIRST_PIPE_INSTANCE`, `PIPE_REJECT_REMOTE_CLIENTS`, `bInheritHandle = FALSE`.
- Restrictive DACL: current-process user SID only, `FILE_GENERIC_READ | FILE_GENERIC_WRITE`. Owner = that SID. No Everyone, no NULL DACL. ACL setup failure fails closed (does not create a default-SD pipe).
- `PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT`. No overlapped I/O (no pending-I/O pin).
- `ERROR_NO_DATA` / `ERROR_PIPE_LISTENING` / `ERROR_PIPE_BUSY` → `QdnfError::WouldBlock`. `ERROR_BROKEN_PIPE` / `ERROR_PIPE_NOT_CONNECTED` → `Closed`. `Ok(0)` on read remains orderly EOF for `ReadCursor`.
- Observed locator: 4-byte LE PID + 1-byte role (0 parent, 1 child) from `GetCurrentProcessId` / `GetNamedPipeClientProcessId` / `GetNamedPipeServerProcessId`. Never from framed bytes. PID is not cryptographic authority.
- `CreateNamedPipeW` / `ConnectNamedPipe` / `GetNamedPipe*ProcessId` / `SetNamedPipeHandleState` / `WaitNamedPipeW` are `kernel32` FFI. `Win32_System_Pipes` is not a crate feature; `Cargo.toml` was not edited. Reused features only: `Win32_Foundation`, `Win32_Storage_FileSystem`, `Win32_Security`, `Win32_System_IO`, `Win32_System_Threading`.

## Stop notes

- Did not edit `local_stream/mod.rs`, `frame.rs`, `tests.rs`, `unix.rs`, `qbdl/**`, `bearer/mod.rs`, `registries.rs`, or `Cargo.toml`.
- Did not add TCP/IP/DNS/libp2p fallback.
- Did not spawn a second process (S3.3).
- Did not run `cargo test` / `cargo check` / `cargo build`.
- No git commit/push. Synth MCP was not touched.

Formatted `windows.rs` with `rustfmt --edition 2021`.
