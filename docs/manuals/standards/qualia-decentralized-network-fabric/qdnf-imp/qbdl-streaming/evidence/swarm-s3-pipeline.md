# Wave S3 multi-agent pipeline

QBS-09 is too large for one author. One-process expert `01a1d3a9-72e4-7c31-94f3-b9bb480768ba` was stopped before it wrote (read-only so far). File-disjoint children; integrator owns parent `mod.rs` and cargo.

Native interchange is CBOR / exact-length binary. JSON is not this bearer or service codec.

Existing reuse (do not rewrite): `bearer/ipc.rs` in-process fixture, `qbdl/wire`, `qbdl/carrier`, `qbdl/provider/outboard`, `LocalWholeSourceGrant`, S1 `CurrentAuthorityGuard`. `BearerProfile::LocalIpcV1` already exists; do not edit `registries.rs`.

## Stages

| Stage | Children (≤3 live) | Writes | Depends |
|---|---|---|---|
| S3.1 | Frame (senior) ∥ In-process service (expert) ∥ Spawn-protocol notes (support) | `bearer/local_stream/{mod,frame,tests}.rs`; `qbdl/service/{mod,session,provider,client,tests}.rs`; evidence notes only | S2 closed |
| S3.2 | Windows named pipe (senior) ∥ Unix domain cfg (senior) ∥ Frame+service reviewer | `local_stream/windows.rs`; `local_stream/unix.rs` | S3.1 frame types exist |
| S3.3 | Two-process join (expert) | `local_stream/two_process.rs` or `qbdl/service/two_process.rs` + Windows spawn | S3.2 Windows + S3.1 service |
| S3.4 | Independent networking reviewer | evidence review only | S3.3 tests |

## S3.1 exclusive claims

- **Frame:** NEW `net/qdnf/bearer/local_stream/{mod.rs,frame.rs,tests.rs}` only. Length-prefixed stream framing, partial read/write cursors, WouldBlock, drain. No OS handles. Do not copy `bearer/fragment.rs` handshake logic.
- **Service:** NEW `net/peer/replication/qbdl/service/{mod.rs,session.rs,provider.rs,client.rs,tests.rs}` only. Join existing wire+carrier+outboard over **existing `ipc_pair`**. Useful partial object in-process. Not two-process. Not JSON.
- **Support:** `evidence/swarm-s3-two-process-protocol.md` only. Env/path/ownership notes. Copy `harness/two_process.rs` patterns. Mark Windows vs Unix as distinct evidence classes.

Integrator routes `bearer/mod.rs` and `qbdl/mod.rs` after authors stop.

## Later (do not start until S3.1 frame exists)

- Windows: `CreateNamedPipeW` / `CreateFileW`, restrictive ACL, no TCP fallback.
- Unix: `UnixStream`/`UnixListener`, cfg-gated; this host’s Windows tests do not prove Linux.
- Two-process: spawn child, transfer a useful framed QBDL record, HTTP/DNS/IP/libp2p disabled.

## Residuals already documented

- Production `write_leaf` still holds StorageStage across OS wait.
- Remote R2 signing/current grant not started; S3 local-admin profile uses `LocalWholeSourceGrant`.
