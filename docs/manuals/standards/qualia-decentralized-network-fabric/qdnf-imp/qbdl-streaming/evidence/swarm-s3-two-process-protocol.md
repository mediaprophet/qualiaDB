# Swarm S3.1 two-process spawn / ownership protocol

Support transcription only. Status: notes for later S3.2/S3.3 authors. No cargo, no git, no `crates/**` edits, no opcodes, no magics.

This file is the exclusive S3.1 support write (`swarm-s3-pipeline.md:22`; `swarm-s3-claim-board.md:9`). It copies existing spawn/env/path facts and records ownership constraints from QBS-09 item 3 (`qbs09-native-service-contract.md:100`) and the S3 pipeline. Names that do not appear in inspected source are marked **UNVERIFIED**.

Native interchange stays CBOR / exact-length binary. JSON is not this bearer or service codec (`swarm-s3-pipeline.md:5`; `qbs09-native-service-contract.md:33`).

## 1. What this note is not

- Not a QBDL wire freeze. Exact service magic / profile / variant tags remain root-owned (`qbs09-native-service-contract.md:32–72`). This note does not invent them.
- Not Windows named-pipe code. `CreateNamedPipeW` / `CreateFileW` are later S3.2 (`swarm-s3-pipeline.md:28`). No `CreateNamedPipe` / `CreateFile` / `\\.\pipe` strings exist under `crates/qualia-core-db/src` at transcription.
- Not two-host Ethernet. `harness/two_process.rs:1` and `harness/two_process.rs:25–28` forbid PhysicalNetwork / two-host claims from that module.
- Not in-process `ipc_pair` reuse as OS IPC. `bearer/ipc.rs:1–3` is an in-process `local-ipc-v1` Arc/Mutex queue. Existing IPC loop remains a unit fixture (`qbs09-native-service-contract.md:100`; `qbs09-service-owner-followups.md:19`).
- Not the WSS TCP loopback harness as a `local_stream` bearer. `p2p/connectivity/two_process.rs` is a separate authenticated-WSS process test over `127.0.0.1`.

## 2. Existing child env names (copied)

Two distinct two-process harnesses exist. Do not mix their env namespaces.

### 2.1 QDNF E05.2 Unix-stream process harness

From `crates/qualia-core-db/src/net/qdnf/harness/two_process.rs`:

| Name | Kind | Path:line | Text |
|---|---|---|---|
| `QDNF_TWO_PROCESS_CHILD` | child role flag | `harness/two_process.rs:10` | `const CHILD_ENV: &str = "QDNF_TWO_PROCESS_CHILD";` |
| `QDNF_TWO_PROCESS_SOCK` | Unix socket path | `harness/two_process.rs:12` | `const SOCK_ENV: &str = "QDNF_TWO_PROCESS_SOCK";` behind `#[cfg(unix)]` |

Child detection (`harness/two_process.rs:44–48`): `std::env::var(CHILD_ENV)` is true only when the value is exactly `"1"`.

Parent sets both (`harness/two_process.rs:134–137`):

```
.env(CHILD_ENV, "1")
.env(SOCK_ENV, &sock)
```

`SOCK_ENV` is compiled only on Unix. On this Windows host, `run_two_os_process_ipc` returns `QdnfError::PlatformUnsupported` (`harness/two_process.rs:71–79`). There is no existing Windows pipe env in this module.

### 2.2 WSS relay process harness (spawn-pattern cousin only)

From `crates/qualia-core-db/src/p2p/connectivity/wss.rs:22–24` used by `p2p/connectivity/two_process.rs:13–15`:

| Name | Kind | Path:line | Text |
|---|---|---|---|
| `QDNF_WSS_RELAY_CHILD` | child role flag | `wss.rs:22` | `pub const ENV_CHILD: &str = "QDNF_WSS_RELAY_CHILD";` |
| `QDNF_WSS_ADDR` | TCP socket-addr string | `wss.rs:23` | `pub const ENV_ADDR: &str = "QDNF_WSS_ADDR";` |
| `QDNF_WSS_SECRET` | shared secret | `wss.rs:24` | `pub const ENV_SECRET: &str = "QDNF_WSS_SECRET";` |

Parent bind is `TcpListener::bind("127.0.0.1:0")` (`p2p/connectivity/two_process.rs:48`). Child dials that TCP addr (`p2p/connectivity/two_process.rs:31–36`). This is local TCP, not a named pipe and not a Unix domain socket. `local_stream` must not take this TCP path as a fallback (`swarm-s3-pipeline.md:28`; `qbs09-native-service-contract.md:100`; `qbs09-service-owner-followups.md:19`).

### 2.3 UNVERIFIED later env names

No Windows named-pipe env exists in source. Do not treat the following as allocated:

| Guess | Status | Why guessed |
|---|---|---|
| reuse `QDNF_TWO_PROCESS_SOCK` for a Windows pipe path | **UNVERIFIED** | `SOCK_ENV` is `#[cfg(unix)]` only (`harness/two_process.rs:11–12`). |
| new `QDNF_TWO_PROCESS_PIPE` (or similar) | **UNVERIFIED** | No such constant in inspected crates. |
| reuse `QDNF_WSS_ADDR` for a pipe name | forbidden by this note | WSS addr is a TCP `SocketAddr` string, not a local endpoint path. |

Existing `QDNF_TWO_PROCESS_CHILD` is **not** Unix-gated (`harness/two_process.rs:10`). A later Windows two-process test **may** keep that same child flag and `"1"` value; that reuse is a pattern copy, not a new name.

## 3. Existing spawn recipe (copy this)

Both harnesses spawn **the same test binary** (`std::env::current_exe()`), not a second crate, and re-enter the **same test name** with libtest flags.

E05.2 parent (`harness/two_process.rs:129–141`):

```
let exe = std::env::current_exe() ...
let test_name = std::thread::current().name() ...
Command::new(&exe)
    .args(["--exact", &test_name, "--nocapture", "--test-threads", "1"])
    .env(CHILD_ENV, "1")
    .env(SOCK_ENV, &sock)
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .spawn()
```

WSS parent (`p2p/connectivity/two_process.rs:52–65`) uses the same `Command::new(&exe)` argv (`--exact`, current test name, `--nocapture`, `--test-threads`, `1`) and a child env flag `"1"`. Child branch is `if is_child() { child_main(); return; }` then parent (`p2p/connectivity/two_process.rs:88–94`; same shape at `harness/two_process.rs:204–217` on Unix).

Role split:

1. Parent generates the local endpoint, binds it, then spawns.
2. Child sees the env flag, reads the parent-supplied locator, connects, exchanges, exits.
3. Parent waits on the child status; kill/wait on timeout or malformation (`harness/two_process.rs:151–186`).

Later `local_stream` two-process tests should keep this split. Proposed later files (`swarm-s3-pipeline.md:15`): `local_stream/two_process.rs` **or** `qbdl/service/two_process.rs` plus Windows spawn. Those paths do not exist yet. **UNVERIFIED** which of the two file locations S3.3 will own.

## 4. Why the 8-byte Unix harness is not useful QBDL transfer

Copied facts:

- Module purpose: “two OS-process IPC evidence. Never PhysicalNetwork / two-host.” (`harness/two_process.rs:1`)
- Parent “binds a Unix stream socket, spawns this test binary as a child, and exchanges eight bytes. Evidence class is Process only.” (`harness/two_process.rs:61–62`)
- Fixed 8-byte payloads (`harness/two_process.rs:13–16`, Unix-only): `PARENT_MSG = b"QDNF-E05"` and `CHILD_ACK = b"ACK-E052"`.
- Child writes `PARENT_MSG` then `read_exact` 8 bytes (`harness/two_process.rs:108–113`). Parent `read_exact` 8 bytes then writes `CHILD_ACK` (`harness/two_process.rs:175–182`).
- Evidence helper hard-codes `EvidenceClass::Process` (`harness/two_process.rs:30–42`) and `physical_two_host_from_this_module()` is always `false` (`harness/two_process.rs:25–28`).
- QBS-09 item 3: “harness/two_process.rs exchanges8 bytes over UnixStream only and is process evidence, not a QBDL bearer.” (`qbs09-native-service-contract.md:100`)
- Follow-up: “Unix8-byte process harness is not useful QBDL transfer.” (`qbs09-service-owner-followups.md:19`)
- Qualification: “In-process queue and Unix8-byte process harness do not satisfy QBS09/G01.” (`qbs09-native-service-contract.md:111`)
- Pipeline later bar: “spawn child, transfer a useful framed QBDL record, HTTP/DNS/IP/libp2p disabled.” (`swarm-s3-pipeline.md:30`)

Why those eight bytes cannot stand in for QBDL:

1. **Length.** A useful QBDL record is an exact-length framed service record (proposed prefix 208 bytes plus variant body; LeafData maximum cited 18137 bytes) (`qbs09-native-service-contract.md:33–68`). Eight bytes are a ping, not a Describe/DescriptorPage/Select/Credit/LeafData/Cancel record.
2. **Codec.** The harness writes raw 8-byte arrays. It does not run `qbdl/wire`, carrier assembly, or length-prefixed `local_stream` frames. JSON is also not the bearer (`swarm-s3-pipeline.md:5`).
3. **Framing.** S3.1 frame work is length-prefixed stream framing with partial read/write cursors, WouldBlock, and drain, and “No OS handles” in the frame crate (`swarm-s3-pipeline.md:20`). The harness uses `write_all` / `read_exact` on a UnixStream with no stream-frame cursor.
4. **Platform.** Unix-only. Windows `run_two_os_process_ipc` is `PlatformUnsupported` (`harness/two_process.rs:71–79`). This host’s Windows tests do not prove Linux (`swarm-s3-pipeline.md:29`).
5. **Evidence class.** Process IPC pass is not PhysicalNetwork and is not G01 native qualification (`harness/two_process.rs:189–194`; `qbs09-native-service-contract.md:100,111`).
6. **Magics.** `QDNF-E05` / `ACK-E052` are E05.2 ping bytes. They are **not** a QBDL service magic. Do not reuse them as record opcodes. Root-approved service magic is still unallocated (`qbs09-native-service-contract.md:37,72`).

Those eight bytes remain valid **Process** evidence that two OS processes can exchange a connected local stream. They do not prove a framed QBDL object moved.

## 5. Proposed Windows named-pipe path ownership

S3.2 Windows owner (not this note) is proposed as `net/qdnf/bearer/local_stream/windows.rs` (`swarm-s3-pipeline.md:14,28`; `qbs09-native-service-contract.md:100`).

Copied constraints:

| Constraint | Source |
|---|---|
| `CreateNamedPipeW` / `CreateFileW` | `swarm-s3-pipeline.md:28` |
| restrictive ACL | `swarm-s3-pipeline.md:28` |
| no TCP fallback | `swarm-s3-pipeline.md:28` |
| no IP/DNS fallback | `qbs09-native-service-contract.md:100` |
| no silent TCP/IP fallback | `qbs09-service-owner-followups.md:19` |
| HTTP/DNS/IP/libp2p disabled | `swarm-s3-pipeline.md:30` |
| restrictive local endpoint ownership | `qbs09-native-service-contract.md:100` |
| no arbitrary incoming path | `qbs09-service-owner-followups.md:19` |
| bounded framing / partial I/O cursors / deadlines / drain | `qbs09-native-service-contract.md:100` |
| observed peer identity | `qbs09-native-service-contract.md:100`; PID/OS credential alone is not cryptographic peer authority (`qbs09-service-owner-followups.md:19`) |

### 5.1 Parent owns the local endpoint

Copy the Unix ownership, not the TCP WSS bind.

Unix parent (`harness/two_process.rs:124–127`):

1. Creates a private temp directory (`tempfile::TempDir`).
2. Chooses the socket path itself: `dir.path().join("qdnf-e05.sock")`.
3. Binds `UnixListener` on that path **before** spawn.
4. Passes that path to the child via `SOCK_ENV`.

Windows analogue required by the citations above, with **UNVERIFIED** surface names:

1. Parent creates the named pipe (`CreateNamedPipeW`) with a **restrictive ACL**. The parent is the only binder.
2. Parent generates the pipe name. Child does not invent it and does not accept a peer-supplied bind path.
3. Parent exports the generated name through a child env (existing `QDNF_TWO_PROCESS_CHILD=1` plus an **UNVERIFIED** path env; see §2.3).
4. Child opens that exact name (`CreateFileW`) and connects. Child is a client of the parent-owned endpoint, matching Unix `UnixStream::connect(&path)` (`harness/two_process.rs:92–95`).
5. If create/connect/ACL fails: unsupported / denied / untested — not success (`qbs09-native-service-contract.md:111`).

**UNVERIFIED** pipe-name string. Unix uses filesystem `…/qdnf-e05.sock` under a temp dir. Windows `\\.\pipe\…` form is not in this tree. Do not freeze a pipe prefix here.

### 5.2 No arbitrary incoming path

“No arbitrary incoming path” (`qbs09-service-owner-followups.md:19`) means:

- The child locator is **parent-generated and parent-exported**. It is not taken from the peer’s application bytes, a JSON config blob, DNS, or an advertised URL.
- The later bearer must not bind whatever path a remote asked for.
- Observed locator / peer identity after connect is adapter-observed, then QSession received-Finished — not “the string we were told to open.”
- Extension-manifest `NamedPipe { pipe_name: String }` (`extensions/extension_manifest.rs:9–10`) is a different daemon transport enum. It is **not** the QDNF `local_stream` owner and must not become an incoming-path hole.

### 5.3 No TCP fallback

If the named pipe is unavailable, the Windows local_stream owner returns platform-unsupported / fail-closed. It must not:

- bind `127.0.0.1:0` as the WSS harness does (`p2p/connectivity/two_process.rs:48`);
- dial `QDNF_WSS_ADDR`;
- fall through to HTTP, DNS, IP sockets, or libp2p (`swarm-s3-pipeline.md:30`; `qbs09-native-service-contract.md:100`).

`bearer/ipc.rs` in-process queue is a unit fixture, not an OS fallback (`qbs09-native-service-contract.md:100`).

## 6. Windows vs Unix are DISTINCT evidence classes

Mixing platform passes into one “all tests passed” claim is already forbidden for qualification classes (`harness/qualification.rs:8–16`). `local_stream` adds a platform split on top of `EvidenceClass::Process`.

Copied:

- “Independent Linux/WSL process delivery and Windows process delivery are distinct evidence flags; neither implies physical two-host delivery.” (`qbs09-native-service-contract.md:100`)
- “Unix: `UnixStream`/`UnixListener`, cfg-gated; this host’s Windows tests do not prove Linux.” (`swarm-s3-pipeline.md:29`)
- “Distinguish Windows, independent Linux/WSL process, physical cross-host and unsupported/permission-refused evidence.” (`qbs09-service-owner-followups.md:19`)
- E05.2 helper: `class: EvidenceClass::Process` and never `PhysicalNetwork` (`harness/two_process.rs:30–42,189–194`).
- `physical_two_host_from_this_module()` is `false` (`harness/two_process.rs:25–28`).
- Report Linux AF_PACKET privilege refusal, Windows restrictions, and unavailable carriers as unsupported/untested, not success (`qbs09-native-service-contract.md:111`).

Record later results as **separate flags**, not one Process bit:

| Flag | What it may show | What it must not show |
|---|---|---|
| Windows two-process named-pipe | Two OS processes on Windows over parent-owned pipe + framed QBDL (once S3.2/S3.3 exist) | Linux Unix-domain, WSL, or PhysicalNetwork |
| Independent Linux/WSL two-process Unix-domain | Two OS processes on that Unix host over parent-owned `UnixListener` + framed QBDL | Windows named-pipe or PhysicalNetwork |
| E05.2 Unix 8-byte harness | Process IPC ping only | Useful QBDL transfer or G01 |
| In-process `ipc_pair` | Component / unit fixture (`BearerProfile::LocalIpcV1` already exists; do not edit `registries.rs` — `swarm-s3-pipeline.md:7`) | OS process or two-host |
| PhysicalNetwork / two-host Ethernet | Separate raw-ethernet qualification | Implied by either local_stream process flag |
| Unsupported / permission-refused | Honest skip | Pass |

`BearerProfile::LocalIpcV1` is the in-process profile (`bearer/ipc.rs:76`). A native stream bearer profile allocation, if any, is root-owned. **UNVERIFIED** — this note does not name a new `BearerProfile` variant.

## 7. How a later two-process test should spawn

Do not start until S3.1 frame types exist and S3.2 Windows + Unix owners exist (`swarm-s3-pipeline.md:26–30,15`). S3.3 joins them: spawn child, transfer a **useful framed QBDL record**.

Proposed sequence, copying the existing parent/child split:

1. **Same test binary.** `std::env::current_exe()` (`harness/two_process.rs:129`; `p2p/connectivity/two_process.rs:52`).
2. **Same test re-entry.** `Command` args `["--exact", &test_name, "--nocapture", "--test-threads", "1"]` where `test_name` is `std::thread::current().name()` (`harness/two_process.rs:130–135`).
3. **Env flag.** Child role = existing `QDNF_TWO_PROCESS_CHILD=1` (pattern copy of `harness/two_process.rs:10,45–47,136`). First lines of the test: if child, connect and serve/exchange; else parent path.
4. **Pipe / socket name generation (parent only).**
   - Unix: private temp dir + parent-chosen socket file, then `UnixListener::bind` (`harness/two_process.rs:124–127`). Do not let the child pick the path.
   - Windows: parent `CreateNamedPipeW` with restrictive ACL; parent-generated name only (`swarm-s3-pipeline.md:28`). Exact name format **UNVERIFIED**.
5. **Export locator.** Parent sets the path env after bind, then `spawn`. Child reads that env and connects (`harness/two_process.rs:92,137`). Windows path env name **UNVERIFIED** (§2.3).
6. **Connect, do not listen, in the child.** Unix child is `UnixStream::connect` with retry until deadline (`harness/two_process.rs:92–104`). Windows child is `CreateFileW` on the exported name. Child never binds an extra endpoint and never opens TCP.
7. **Transfer a useful framed QBDL record**, not eight ping bytes (`swarm-s3-pipeline.md:30`). Frame bytes are length-prefixed `local_stream` frames (S3.1) carrying exact-length binary / CBOR service records. JSON is not the bearer (`swarm-s3-pipeline.md:5`).
8. **Disable HTTP/DNS/IP/libp2p** for the test (`swarm-s3-pipeline.md:30`). No WSS TCP fallback.
9. **Finite watchdog**, inherit stdio, kill/wait on expiry as the Unix parent already does (`harness/two_process.rs:143–154,66` sleep analogue is WSS-only; Unix uses accept deadline 15s / IO 5s).
10. **Record the platform flag** from §6. Windows pass does not set Linux; Unix pass does not set Windows; neither sets PhysicalNetwork.
11. **Wire remains exact-length binary / CBOR.** Do not add a JSON control channel beside the pipe. Do not smuggle opcodes through env vars.

S3.1 in-process service over existing `ipc_pair` is a different child and is **not** two-process (`swarm-s3-pipeline.md:21`).

## 8. Wire / codec boundary (no new magics)

Copied:

- “Native interchange is CBOR / exact-length binary. JSON is not this bearer or service codec.” (`swarm-s3-pipeline.md:5`)
- “Caller-output, exact-length big-endian binary codec; no owned unbounded metadata or JSON.” (`qbs09-native-service-contract.md:33`)
- Service prefix table starts with “root-approved service magic” at offset 0..4 (`qbs09-native-service-contract.md:37`). Exact magic/profile/tag allocations need root review **before** codec code (`qbs09-native-service-contract.md:72`).
- Existing E05.2 bytes `QDNF-E05` / `ACK-E052` (`harness/two_process.rs:14–16`) are process-harness pings, not that magic.

This note therefore does **not** specify:

- service magic, schema version, variant tags, profile IDs;
- frame length-prefix width or endianness (S3.1 frame owner);
- named-pipe mode/buffer/ACL numeric constants;
- a Windows pipe path prefix or a new env identifier.

Those remain root / S3.1-frame / S3.2-Windows work.

## 9. Source index

| Path | Role in this note |
|---|---|
| `crates/qualia-core-db/src/net/qdnf/harness/two_process.rs` | Existing `QDNF_TWO_PROCESS_CHILD` / `QDNF_TWO_PROCESS_SOCK`, Unix 8-byte exchange, same-binary spawn, Process-only evidence |
| `crates/qualia-core-db/src/p2p/connectivity/two_process.rs` | Same-binary spawn cousin; TCP WSS — **not** local_stream |
| `crates/qualia-core-db/src/p2p/connectivity/wss.rs:22–24` | `QDNF_WSS_RELAY_CHILD` / `QDNF_WSS_ADDR` / `QDNF_WSS_SECRET` |
| `crates/qualia-core-db/src/net/qdnf/bearer/ipc.rs` | In-process fixture, not OS IPC |
| `crates/qualia-core-db/src/net/qdnf/harness/qualification.rs:11–16` | `EvidenceClass::{Component,Process,PhysicalNetwork,Application}` |
| `docs/.../evidence/qbs09-native-service-contract.md` item 3 (`:100`) | Native OS bearer gap, restrictive local endpoint, distinct Windows vs Linux/WSL flags |
| `docs/.../evidence/qbs09-service-owner-followups.md:19` | No arbitrary incoming path; no silent TCP/IP fallback; 8-byte harness not useful QBDL |
| `docs/.../evidence/swarm-s3-pipeline.md` | CBOR/binary not JSON; Windows `CreateNamedPipeW`/`CreateFileW`; spawn later useful record |
| `docs/.../evidence/swarm-s3-claim-board.md:9` | This file is the only allowed S3c write |
