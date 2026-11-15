---
pass: true
open_bugs: 0
---

# Wave S3.3 — two-process QBDL transfer independent review

Read-only source review of Wave S3.3 two-OS-process framed QBDL. Production Rust was not edited. Cargo was not run. Author summary (`swarm-s3-two-process-summary.md`) was not treated as proof.

Reviewed:

- `crates/qualia-core-db/src/net/qdnf/bearer/local_stream/{mod.rs,two_process.rs,windows.rs,unix.rs,frame.rs}`
- `crates/qualia-core-db/src/net/peer/replication/qbdl/service/{mod.rs,session.rs,client.rs,provider.rs,two_process.rs,tests.rs}`
- `evidence/swarm-s3-two-process-protocol.md`
- `evidence/swarm-s3-two-process-summary.md`
- `evidence/qbs09-native-service-contract.md` item 3 and qualification section
- `evidence/swarm-s3-pipeline.md`

## Review

S3.3 joins the S3.1 in-process service and S3.2 named-pipe / UDS adapters into a same-binary two-OS-process transfer. Parent owns `CreateNamedPipeW` / `UnixIpcListener::bind`. Child connects to the exported name only. After accept, parent is provider and recvs first; child is client and sends Open first. Wire is existing QBS1 208-byte prefix plus length-prefixed `local_stream` frames. Payload is the same useful tail leaf as `service/tests.rs` (19 bytes of a 3-leaf plaintext entry). `Fixture::new_with_payload` is reconstructed independently in each process from a deterministic `payload()`; source bytes are not passed through env. Bind/accept failure panics; there is no `ipc_pair` / TCP / `127.0.0.1` / DNS / HTTP / WSS / libp2p fallback.

`Session<'a, B: Bearer = IpcEndpoint>` keeps the in-process default. `service/tests.rs` still constructs `ipc_pair` and `Session::attach` without a second type argument. Named-pipe `FramedStream<NamedPipeStream>` and Unix `FramedStream<UnixStreamBytes>` also implement `Bearer`. `WouldBlock` on send/recv retries the same sealed frame / recv buffer with a 5s deadline and 1ms poll; it does not `fail_close` until expiry.

Evidence class is `EvidenceClass::Process`. `physical_two_host_from_this_module()` is `false`. Windows and Unix spawn tests are mutually exclusive `cfg`s. `BearerProfile::LocalIpcV1` is reused; `registries.rs` is not in this wave’s files.

## Attacks (source, not summary)

1. **Two-process test does not fall back to `ipc_pair`, TCP, `127.0.0.1`, DNS, HTTP, WSS, or libp2p.** Holds.
   - `service/two_process.rs` does not import `ipc_pair`. Bind/accept `Err` panics with an explicit “no ipc_pair/TCP fallback” message (`two_process.rs:435-437`, `461-463`). No `TcpStream` / `TcpListener` / `std::net::` connect / libp2p / HTTP client in `service/{two_process,session,client,provider}.rs` or `local_stream/{two_process,windows,frame}.rs`. Unix `std::net::Shutdown::Both` is socket shutdown, not a TCP dial (`unix.rs:130`).
   - Child parse refuses `127.0.0.1:0` and `\\.\pipe\not-ours` as `Denied` (`service/two_process.rs:335-344`, `412-420`; `local_stream/two_process.rs:220-229`, `304-312`; `windows.rs:101-111`; `unix.rs:198-215`).
   - In-process `ipc_pair` remains only in `service/tests.rs` (the S3.1 unit fixture) and is not consulted on OS failure.

2. **JSON is not the bearer or control channel; source bytes do not travel through env.** Holds.
   - Stream wire is `u32be len || payload` (`frame.rs:2-3,20-36`). Service records go through existing `wire::encode` / `Header` (`session.rs:130`). Magic remains `QBS1` (`wire/prefix.rs:72,97`). IRI remains `b"q42:QBDLStream/1"`. Open/Grant bodies are 64-byte `Limits` plus ordinal/page, not maps.
   - Env is only `QDNF_TWO_PROCESS_CHILD=1` plus parent-generated `QDNF_TWO_PROCESS_PIPE` / `QDNF_TWO_PROCESS_SOCK` (`service/two_process.rs:35-39,173-176,291-293,356-358`). `payload()` is rebuilt in both processes (`two_process.rs:72-81,203-204,228-229`). Transcript is `StrongDigest([9; 48])` in both (`two_process.rs:41`). `Fixture::new_with_payload` is deterministic: `keygen_from_seed(&[71; 32])`, `SigningKey::from_bytes(&[72; 32])`, nonce `[1; 32]`, issuer `[31; 48]` (`policy/tests.rs:55-179`); `BundleWriter` is “deterministic bundles for a given input” (`bundle/writer.rs:19`).
   - Grep of the scoped service/local_stream files finds no `serde_json` / `serde::`. Unit tests assert the first stream byte is not `{` (`service/two_process.rs:488-497`; `local_stream/two_process.rs:402-411`).

3. **WouldBlock on named-pipe/UDS retries the same sealed frame / recv buffer with a deadline; it does not fail-close.** Holds.
   - `windows.rs` maps `ERROR_NO_DATA` / `ERROR_PIPE_LISTENING` / `ERROR_PIPE_BUSY` and `WriteFile` `written == 0` to `WouldBlock` (`windows.rs:248-249,571-573`). `unix.rs` maps `EAGAIN` / `EWOULDBLOCK` / `EINTR` to `WouldBlock` (`unix.rs:224-227`).
   - `WriteCursor::seal` resumes the same payload while pending; a different payload is `Conflict` and does not reseal or rewind `off` (`frame.rs:109-123`). `flush` / `ReadCursor::fill` return `WouldBlock` without dropping the cursor (`frame.rs:140,268`).
   - `Session::send_frame` retries `endpoint.send` of the **same** `frame` slice until Ok / non-WouldBlock / 5s `Expired` (`session.rs:159-174`). `recv_frame` retries `endpoint.recv` into the **same** `frame` buffer (`session.rs:277-291`). WouldBlock is not passed through `fail_close`. Deadline expiry is `Expired` (recv fail-closes; send’s caller `send_message` fail-closes). Poll is 1ms, not a bare spin.
   - `local_stream/two_process.rs` has the same 5s send/recv deadline loop (`two_process.rs:99-136`). Parent accept is 15s / 5ms (`service/two_process.rs:43-45,296-318`).

4. **Session genericization keeps in-process `Session<'_>` default (`IpcEndpoint`).** Holds.
   - `pub struct Session<'a, B: Bearer = IpcEndpoint>` (`session.rs:42`). `IpcEndpoint` is `pub` from `bearer/mod.rs:19`. `attach` takes `endpoint: B` (`session.rs:56-62`).
   - `service/tests.rs` is still the five S3.1 tests over `ipc_pair` and unannotated `Session::attach` (`tests.rs:134-156` and the other four). `transfer_one_leaf<PB: Bearer, CB: Bearer>` infers `IpcEndpoint` there (`service/mod.rs:159-163`). No `Session<'_>` site requires an explicit second type parameter.

5. **Parent owns the endpoint; child does not bind; incoming path is not arbitrary.** Holds.
   - Windows: `NamedPipeListener::bind()` takes no path (`windows.rs:154-158`); `generate_name` writes `\\.\pipe\qbdl-{pid}-{tid}-{seq}` (`windows.rs:330-358`). Child is `PipeName::from_exported` then `CreateFileW` (`windows.rs:99-111,293-296`; `service/two_process.rs:329-332`). `from_exported` requires that prefix and hex/`-` only.
   - Unix: `UnixIpcListener::bind()` creates a 0700 temp dir and binds `qdnf-local.sock` (`unix.rs:152-166`). Child is `UnixStreamBytes::connect` of the exported path (`unix.rs:60-65`; `service/two_process.rs:394-409`). `connect` does not bind. `reject_non_local_path` refuses empty, relative, abstract NUL, `://`, and `host:port` with no `/` (`unix.rs:198-215`).
   - Accept `SocketAddr` is discarded (`unix.rs:174-176`). Child never calls `CreateNamedPipeW` / `UnixListener::bind`.

6. **Useful leaf is transferred; not an 8-byte ping; unrelated payload stays unrequested.** Holds.
   - `service/two_process.rs` rebuilds the same 2×`LEAF_BYTES`+19 fixture as `tests.rs` (`two_process.rs:72-81`; `tests.rs:21-29`). Client requests `tail = leaf_count - 1` (`two_process.rs:212-218`). Asserts `n == 19`, bytes equal that tail slice, and absence of leaf-0 `0xA1` / `DEADBEEF` and leaf-1 `0DD0FEED` (`two_process.rs:193-198,219-223,262-266`). Parent checks `served.leaf == 2`, `length == 19` (`two_process.rs:270-272`).
   - E05.2 `QDNF-E05` / `ACK-E052` are not used. `local_stream/two_process.rs` is a separate length-prefixed ping (`\xA1\xB2\xC3\xD4` / `\x11\x22`) and is not the QBS-09 item 3 gate.

7. **Evidence class is Process, not PhysicalNetwork; Windows pass is not treated as Linux.** Holds.
   - `process_stream_evidence` hard-codes `EvidenceClass::Process` (`service/two_process.rs:59-69`; `local_stream/two_process.rs:54-64`). Tests assert `class == Process` and `class != PhysicalNetwork`, and `!physical_two_host_qualified()` (`service/two_process.rs:274-277,427-428`; `local_stream/two_process.rs:138-144`).
   - Spawn tests are `#[cfg(all(windows, target_arch = "x86_64"))]` xor `#[cfg(unix)]` (`service/two_process.rs:424-474`). `unix.rs` `windows_process_evidence()` is `false` (`unix.rs:36-38`). This Windows host cannot execute the Unix spawn; those tests are compile-gated only.

8. **No new magics; no new `BearerProfile`.** Holds.
   - Profile asserts are `BearerProfile::LocalIpcV1` plus `native_independent()` (`service/two_process.rs:201-211,227,248`; `local_stream/mod.rs:95,162`). Registry enum is still `LocalIpcV1 = 1` … `NymMixnetTransitionV1 = 7` (`registries.rs:155-167`). Service prefix is existing `QBS1` / `q42:QBDLStream/1`. Stream framing has no magic, only a length prefix (`frame.rs:10-18`).

9. **No dual-recv deadlock. Child/client sends Open first; parent/provider recvs first after accept.** Holds.
   - After `parent_bind_spawn_accept` / `child_connect_framed`, parent calls `run_provider` → `answer_open` (`recv_control` of `Kind::Open`) then `answer_leaf` (`service/two_process.rs:226-256,439-447`). Child calls `run_client` → `send_open` then `recv_grant_send_credit_request` then `recv_leaf` (`two_process.rs:200-218,429-433`).
   - Sequence is Open → Grant → Credit → RequestLeaves → LeafData. Each wait has a sender. `transfer_one_leaf` is correctly not used across processes (it is single-threaded ping-pong).
   - `local_stream/two_process.rs` ping is the opposite byte order (parent send, child recv) and is also not a dual-recv; it is not the QBDL Open path.

10. **Child spawn is the same binary with `--exact` current test name; child cannot recurse into parent spawn.** Holds.
    - `test_command` is `std::env::current_exe()` plus `["--exact", &test_name, "--nocapture", "--test-threads", "1"]` where `test_name` is `std::thread::current().name()` (`service/two_process.rs:166-177`; `local_stream/two_process.rs:71-82`). Matches E05.2 (`harness/two_process.rs:129-140`).
    - Test entry: `if is_child()` (`env == "1"`) runs client/connect and `return`s before `parent_bind_spawn_accept` (`service/two_process.rs:51-53,429-433`). `SPAWNED` is stored only on the parent spawn path (`two_process.rs:295`).

## Residuals (allowed; not S3.3 bugs)

- Cargo was not run here. This is source review, not G01 execution evidence.
- QBS09 qualification extras (induced partial read/write, frame corruption/replay, disconnect/reconnect missing-ID resume, revoke mid-transfer) are not in the two-process test. In-process malformed/cancel/close tests remain in `service/tests.rs`. Item 3’s G01 bar is not closed by this wave.
- QBS10 consumed common channel / one `PacketProtection` is not present. Frames are plaintext `ServiceRecordHeader` (88) + ≤1024 chunk over the local stream. Summary says so.
- Production `write_leaf` still holds `StorageStage` across OS wait. Service never calls storage.
- Describe / DescriptorPage / Select / Resume / Outcome / remote Share are unimplemented, as the first-profile contract allows.
- Authority recheck is at `answer_open` / `answer_leaf` entry plus `accept_local_scoped` entry/exit, not immediately before every `send_message`. The two-process clock is `|| 20` (constant), so the test cannot expire mid-wait.

## Nits

- Protocol §6 asked for distinct Windows vs Linux/WSL two-process **flags**, not one Process bit. The implementation records `EvidenceClass::Process` on both cfgs and relies on mutually exclusive `#[cfg]` plus `!PhysicalNetwork` asserts. That does not treat a Windows pass as Linux, but it is not the separate flag table the protocol note described. `unix.rs::linux_wsl_two_process_evidence()` stays `false` (that module’s unit tests still do not spawn).
- `is_child()` is preceded by two `physical_two_host_*` asserts in the QBDL spawn test. Those do not spawn; the recursive-parent gate still holds.
- `child_refuses_tcp_and_foreign_pipe_name` and `wire_is_length_prefixed_binary_not_json` exist in both `local_stream/two_process.rs` and `service/two_process.rs`. They do not spawn, so `--exact` cannot recurse. Duplicate names are a harness smell only.
- `Session` recv scratch is `FRAME_BYTES = 88+1024 = 1112` while `FramedStream` MTU is 1280. An oversized stream frame would `Capacity` fail-close without dropping the cursor. Honest peers only send carrier chunks ≤1112. Pre-existing S3.1 geometry; not a 19-byte-leaf failure.
- Unix child reads `SOCK_ENV` via `std::env::var` (UTF-8). Parent tempfile paths are UTF-8 in practice; `var_os` would be stricter for non-UTF-8.
- Windows `peer_pid` still falls back to `GetCurrentProcessId` if the named-pipe PID query fails (`windows.rs:522-533`). Session dest/observed-source use each side’s own adapter `framed.peer()`, so a wrong PID does not break send/recv. It remains not cryptographic peer authority.
- `local_stream/two_process.rs` ping is 4+2 bytes. That is the OS-spawn owner’s frame test, not useful QBDL. Callers must not cite it as the item 3 gate.

## Issues
