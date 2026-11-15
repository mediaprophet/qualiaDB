open_bugs: 0
pass: true

# Wave S3.2 Windows named-pipe StreamBytes — independent review

Read-only source review. Production Rust was not edited. Cargo was not run. Author summary (`swarm-s3-windows-summary.md`) was not treated as proof. Protocol notes: `swarm-s3-two-process-protocol.md`. `Win32_System_Pipes` is not a crate feature.

Reviewed:

- `crates/qualia-core-db/src/net/qdnf/bearer/local_stream/windows.rs`
- `crates/qualia-core-db/src/net/qdnf/bearer/local_stream/{mod.rs,frame.rs}`
- `crates/qualia-core-db/Cargo.toml` windows features
- `windows` 0.62.2 HANDLE / pipe / flag types (cargo registry, for ABI only)
- `evidence/swarm-s3-windows-summary.md` (claims only, not proof)
- `evidence/swarm-s3-two-process-protocol.md`
- `evidence/swarm-s3-pipeline.md` S3.2/S3.3 split

## Review

S3.2 is a parent-owned `CreateNamedPipeW` / child `CreateFileW` byte pipe implementing `StreamBytes`, wrapped by existing `FramedStream::connect`. Pipe name is generated inside `bind()`; there is no bind-path argument. Child parse (`PipeName::from_exported`) refuses anything that is not `\\.\pipe\qbdl-` plus hex/`-`. Profile is existing `BearerProfile::LocalIpcV1`. Locators are PID+role from the adapter. `named_pipe_pair` is two handles in one process. Two-process spawn is not in this tree and is not claimed.

`local_stream/mod.rs` already routes `#[cfg(all(windows, target_arch = "x86_64"))] mod windows;` (integrator, not the untrusted summary). `Win32_System_Pipes` is absent from `Cargo.toml`; pipe APIs are a `kernel32` `extern "system"` block whose argument/return layout matches the 0.62.2 `Win32::System::Pipes` link signatures.

## Attacks (source, not summary)

1. **Parent generates the pipe name; no bind-path argument; child cannot open an arbitrary path.** Holds.
   - `NamedPipeListener::bind` takes no path (`windows.rs:150-154`). `generate_name` is private (`windows.rs:325-333`) and writes `\\.\pipe\qbdl-{pid:08x}-{tid:08x}-{seq:016x}` (`windows.rs:335-354`; prefix `windows.rs:42`).
   - `PipeName::from_ascii` is private (`windows.rs:123-135`). The only public constructor is `from_exported` (`windows.rs:96-111`), which requires `PIPE_NAME_PREFIX`, non-empty rest, and `ascii_hexdigit` or `-` only. `\\.\pipe\not-ours`, `\\.\pipe\qbdl-../x`, `C:\Temp\qdnf.sock` → `Denied`; empty suffix → `Malformed` (tests `windows.rs:604-622`).
   - Child I/O is `CreateFileW` on that `PipeName` (`windows.rs:288-292`, `windows.rs:391-427`). There is no child `CreateNamedPipeW` and no bind of a caller-supplied path.

2. **TCP / DNS / IP / HTTP / filesystem denied.** Holds.
   - `from_exported` is prefix+charset only. `127.0.0.1:0`, `localhost:443`, `http://127.0.0.1/`, `/tmp/qdnf.sock` → `Denied` (`windows.rs:625-642`).
   - Grep of `windows.rs`: no `TcpStream`, `TcpListener`, `UdpSocket`, `std::net`, HTTP client, DNS, or libp2p. Failures map to `Denied` / `Incomplete` / `PlatformUnsupported` (`windows.rs:557-571`), not a socket fallback.
   - Matches protocol “no silent TCP/IP fallback” (`swarm-s3-two-process-protocol.md` §5.3). WSS `127.0.0.1:0` is a different harness and is not imported here.

3. **Restrictive DACL (current user), `nMaxInstances=1`, `PIPE_REJECT_REMOTE_CLIENTS`, non-inheritable.** Holds.
   - `create_parent_pipe` calls `with_restrictive_sa` (`windows.rs:368-388`). Token user SID is copied; DACL is one `AddAccessAllowedAce` of `FILE_GENERIC_READ | FILE_GENERIC_WRITE` for that SID; owner is that SID (`windows.rs:440-506`). ACL setup errors return without `CreateNamedPipeW` (fail closed, no default-SD pipe).
   - `nMaxInstances` is literal `1` (`windows.rs:375`). Open mode is `PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE` (`windows.rs:373`).
   - `PIPE_REJECT_REMOTE_CLIENTS` is hardcoded `8`, matching `windows` 0.62.2 `NAMED_PIPE_MODE(8)` (not WDK `FILE_PIPE_REJECT_REMOTE_CLIENTS = 2`). Combined mode is byte + `PIPE_NOWAIT` + reject-remote (`windows.rs:46-51`).
   - `bInheritHandle: false.into()` (`windows.rs:500-504`). Child `CreateFileW` passes `None` for security attributes (`windows.rs:395-403`).

4. **`ERROR_NO_DATA` → `WouldBlock`; broken pipe → `Closed`; `FramedStream` wrapping; `LocalIpcV1`.** Holds.
   - `map_win32`: `ERROR_NO_DATA` (232) / `ERROR_PIPE_LISTENING` (536) / `ERROR_PIPE_BUSY` (231) → `WouldBlock`; `ERROR_BROKEN_PIPE` (109) / `ERROR_PIPE_NOT_CONNECTED` (233) / `ERROR_INVALID_HANDLE` → `Closed` (`windows.rs:557-564`). `WriteFile` success with `written == 0` is also `WouldBlock` (`windows.rs:243-244`).
   - `win32_code` unwraps `HRESULT_FROM_WIN32` (`0x8007xxxx` → low 16 bits) so crate `ReadFile`/`WriteFile` `Err` values still hit those Win32 codes (`windows.rs:544-555`).
   - `StreamBytes::read` `Ok(0)` stays orderly EOF (`windows.rs:263-272`; `mod.rs:29`). `ReadCursor` turns idle `Ok(0)` into `Closed` (`frame.rs:253-260`). Raw-read test allows `Closed` or `Ok(0)`; framed `recv` after peer shutdown asserts `Closed` (`windows.rs:717-733`).
   - `framed_connect` / `framed_named_pipe_pair` call `FramedStream::connect` (`windows.rs:302-323`; `mod.rs:47-71`). `Bearer::profile` and `capabilities` are `BearerProfile::LocalIpcV1` (`mod.rs:74-82`, `mod.rs:143-145`). Tests assert that plus `native_independent()` (`windows.rs:667-677`). `registries.rs` is not in this wave’s files.

5. **Locators from the adapter, never from payload.** Holds.
   - `from_connected` builds 5-byte locators: LE PID + role (`windows.rs:211-223`, `windows.rs:530-535`). Parent peer PID from `GetNamedPipeClientProcessId`; child peer PID from `GetNamedPipeServerProcessId` (`windows.rs:508-519`).
   - `framed_connect` copies `io.local_locator()` / `io.peer_locator()` into `FramedStream` (`windows.rs:302-311`). `recv` sets `RecvMeta.observed_source` to `self.peer` (`mod.rs:174-181`). `peer()` comment: never from a payload field (`mod.rs:98-101`).
   - Test sends `b"not-loc!"` and asserts `meta.observed_source` equals adapter peer and is not the payload; send to self locator is `Unauthorized` (`windows.rs:679-703`). PID/OS credential is not cryptographic (module docs `windows.rs:4-6`).

6. **kernel32 FFI vs windows 0.62 HANDLE types — flag ABI mismatches.** No mismatch found.
   - `Cargo.toml:304-320` windows 0.62.2 features include `Win32_Foundation`, `Win32_Storage_FileSystem`, `Win32_Security`, `Win32_System_IO`, `Win32_System_Threading`. **`Win32_System_Pipes` is not enabled.** Pipe create/connect/state/wait therefore cannot use the crate wrappers; the `kernel32` block (`windows.rs:62-84`) is the intended workaround.
   - 0.62.2 `HANDLE` is `#[repr(transparent)] struct HANDLE(*mut c_void)` with `is_invalid` true for both `-1` and null. Custom `CreateNamedPipeW` returns that `HANDLE`; failure is `INVALID_HANDLE_VALUE` (`windows.rs:382-384`). ABI matches the crate’s own `CreateNamedPipeW` link (`windows-0.62.2` `System/Pipes/mod.rs`: `PCWSTR`, `FILE_FLAGS_AND_ATTRIBUTES`, `NAMED_PIPE_MODE`, `u32`×4, `*const SECURITY_ATTRIBUTES` → `HANDLE`).
   - Custom uses `u32` / `*const u16` / `i32` where the crate uses transparent newtypes `FILE_FLAGS_AND_ATTRIBUTES(u32)`, `NAMED_PIPE_MODE(u32)`, `PCWSTR(*const u16)`, `BOOL(i32)`. Same System V / Microsoft x64 layout. `ConnectNamedPipe` `*mut c_void` vs `*mut OVERLAPPED` is pointer-sized; the call passes null (`windows.rs:168`).
   - Flag values: `PIPE_ACCESS_DUPLEX = 3`, `FILE_FLAG_FIRST_PIPE_INSTANCE = 524288`, OR extracted via `.0` (`windows.rs:373`). `PIPE_NOWAIT = 1`, `PIPE_REJECT_REMOTE_CLIENTS = 8`. Not the WDK `FILE_PIPE_REJECT_REMOTE_CLIENTS = 2` (that would collide with `PIPE_READMODE_MESSAGE`).
   - `SetNamedPipeHandleState` takes `*const u32` vs crate `*const NAMED_PIPE_MODE`; `NAMED_PIPE_MODE` is transparent `u32`. Child sets `PIPE_READMODE_BYTE | PIPE_NOWAIT` only (`windows.rs:406-409`).

7. **In-process pair is not two-process evidence.** Holds. Not claimed as S3.3.
   - `named_pipe_pair` is bind → same-process `CreateFileW` → `accept` (`windows.rs:294-300`). Comment: “In-process pair”. No `Command::new`, `current_exe`, or `QDNF_TWO_PROCESS_*` under `local_stream/`.
   - No `local_stream/two_process.rs`. Unix module’s `linux_wsl_two_process_evidence()` is a separate cfg-gated file and returns `false`.
   - Summary (`swarm-s3-windows-summary.md:5,53,71`) and protocol (`swarm-s3-two-process-protocol.md` §7) place spawn in S3.3. Author does not claim two-OS-process or useful QBDL transfer.

## Residuals (allowed; not S3.2 bugs)

- Two-process spawn (S3.3) is not present: no env export of the pipe name, no same-binary child, no framed QBDL record across processes. Residual only; fail bar was “fail if claimed.”
- `mod windows` is routed (`mod.rs:8-9`) but there is no `pub use windows::{...}`. Tests live inside `windows.rs` so they compile without the optional re-export. S3.3 will need a public surface.
- `TOKEN_USER` / ACL / SID copies sit in `[u8; N]` stack arrays (`windows.rs:445,469,479`). Type alignment is 1; `InitializeAcl` wants DWORD alignment. x64 Windows stack is 16-byte aligned in practice; this is not a claimed-bar failure.
- `peer_pid` falls back to `GetCurrentProcessId` if the named-pipe PID query fails (`windows.rs:515-518`). Correct for this in-process pair; S3.3 must not treat that fallback as peer authority.
- DACL construction is current-user-only in source. Tests do not impersonate another user. Source review, not a cross-user run.
- Author summary said `mod.rs` was not edited. Current `mod.rs` has the windows (and unix) cfg modules. Summary is stale on routing; the code is what was reviewed.
- Cargo was not run here (instruction). In-process tests are source-present, not execution evidence.

## Issues
