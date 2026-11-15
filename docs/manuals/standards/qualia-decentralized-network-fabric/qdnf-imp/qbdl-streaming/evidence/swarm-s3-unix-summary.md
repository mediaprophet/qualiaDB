# Wave S3.2 / Unix-domain backend

`UnixListener` / `UnixStream` adapter for existing length-prefixed `FramedStream`. Parent generates a private temp directory and binds `qdnf-local.sock` there (0700 dir, 0600 socket). Child connects to that exported path. No caller bind path. No IP/DNS/TCP/libp2p fallback. Profile is existing `BearerProfile::LocalIpcV1`.

This host is Windows. The module is compile-gated (`#![cfg(unix)]`). Windows tests were not run and do not prove Unix. Independent Linux/WSL two-process delivery is a distinct evidence class and is **not** claimed by the same-process unit tests in this file. E05.2 eight-byte ping is not reused.

## Files written

- `crates/qualia-core-db/src/net/qdnf/bearer/local_stream/unix.rs`
- this summary

`local_stream/mod.rs`, `frame.rs`, `windows.rs`, `qbdl/**`, `Cargo.toml`, and git were **not** edited. `cargo` was not run.

## Requested integrator patch

`crates/qualia-core-db/src/net/qdnf/bearer/local_stream/mod.rs` (after `mod frame;`):

```rust
#[cfg(unix)]
mod unix;
```

Recommended re-exports (optional):

```rust
#[cfg(unix)]
pub use unix::{
    linux_wsl_two_process_evidence, physical_two_host_from_this_module, profile,
    silent_ip_fallback, windows_process_evidence, UnixIpcListener, UnixStreamBytes, SOCKET_FILE,
};
```

No `Cargo.toml` change. The module will not compile on Unix until `#[cfg(unix)] mod unix;` is routed. On this Windows host the gated file is present but not compiled.

Focused selector after routing (Unix only): `net::qdnf::bearer::local_stream::unix`

## Key decisions

- `UnixIpcListener::bind()` takes **no path**. It creates `tempfile::TempDir`, chmod 0700, binds `SOCKET_FILE` (`qdnf-local.sock`), chmod 0600, sets the listener non-blocking. Child is `UnixStreamBytes::connect(&parent.path())` only. Incoming `accept` `SocketAddr` is discarded and is not an observed locator.
- `connect` refuses empty, relative, abstract (`NUL`), URL (`://`), and TCP-looking (`host:port` with no `/`) paths as `Denied` / `Incomplete`. There is no `TcpListener` / `TcpStream` / `127.0.0.1` fallback.
- `UnixStreamBytes` implements `StreamBytes`. `EAGAIN` / `EWOULDBLOCK` / `EINTR` / `ErrorKind::WouldBlock` map to `QdnfError::WouldBlock`. Broken pipe / reset / `Ok(0)` on read is orderly EOF for the cursor (`Closed` mid-idle is applied by `ReadCursor`). Shutdown is idempotent.
- `FramedStream::connect` wraps the handle. Profile is `BearerProfile::LocalIpcV1` (same as in-process `ipc.rs`). `registries.rs` was not edited. Same frame cursors (`WriteCursor` / `ReadCursor`); handshake `bearer/fragment.rs` was not copied.
- Adapter locators: Linux `SO_PEERCRED` (pid, uid); macOS `LOCAL_PEERPID`. PID/uid is **not** cryptographic peer authority. `into_framed_with` lets a caller supply adapter-side locators; they still must not come from framed bytes.
- `UnixStreamBytes::pair()` is same-process only. `linux_wsl_two_process_evidence() == false`, `windows_process_evidence() == false`, `physical_two_host_from_this_module() == false`, `silent_ip_fallback() == false`.
- Two-process spawn (`QDNF_TWO_PROCESS_CHILD` / `QDNF_TWO_PROCESS_SOCK`) remains S3.3. This file does not spawn, does not exchange `QDNF-E05` / `ACK-E052`, and does not invent a Windows pipe env.

## Test names

`local_stream/unix.rs` (`#[cfg(all(test, unix))]` submodule). These compile and run only on Unix. They were **not** executed on this Windows host.

- `parent_generated_path_child_connect_framed_roundtrip`
- `eagain_would_block_then_resume_retains_write_cursor`
- `connect_rejects_tcp_url_and_abstract_paths`
- `bind_owns_private_temp_socket_not_incoming_path`
- `shutdown_then_io_is_closed`
- `framed_profile_is_local_ipc_v1`
- `no_ip_dns_tcp_fallback`
- `peer_locator_is_adapter_observed`

## Stop notes

- Did not edit `local_stream/mod.rs`, `frame.rs`, `windows.rs`, `bearer/mod.rs`, `registries.rs`, `qbdl/**`, or `Cargo.toml`.
- Did not run `cargo test` / `cargo check` / `cargo build`.
- Did not claim Windows tests prove Unix. Did not claim same-process UDS proves Linux/WSL two-process or PhysicalNetwork.
- No git commit/push. Synth MCP was not touched.

Formatted `unix.rs` with `rustfmt --edition 2021`.
