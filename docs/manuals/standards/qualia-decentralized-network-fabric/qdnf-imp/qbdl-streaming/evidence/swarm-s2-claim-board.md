# Swarm S2 claim board

Integrator-owned. Captured 2026-11-13 after Wave S1 review pass + authority tests 37/37.

Prerequisite: S1 identity bind closed. Do not edit S1 files.

## Live exclusive claims

| Child | Tier | Allowed writes | Forbidden |
|---|---|---|---|
| B remote R1 | Expert grok-4.7 | NEW `crates/qualia-core-db/src/bundle/stream/policy/remote/{mod,recognition,policy,codec,decision,tests}.rs`; `evidence/swarm-s2-remote-r1-summary.md` | `policy/mod.rs`, authority, storage, cargo, git |
| C storage jobs | Senior grok-4.6 | NEW `bundle/stream/runtime/jobs/{mod,ticket,submit,complete,tests}.rs`; NEW `storage/native/job.rs`; optional `storage/native/write.rs`; `evidence/swarm-s2-storage-jobs-summary.md` | `runtime/mod.rs`, `storage/native/mod.rs`, policy, authority, cargo, git |
| Support fixtures | Cheap grok-4.6 | `tools/qbdl-stream-conformance/fixtures/remote-recognition-v1.json`, `storage-job-v1.json`; `evidence/swarm-s2-fixture-notes.md` | `crates/**`, cargo, git |

Integrator retains: `policy/mod.rs`, `runtime/mod.rs`, `storage/native/mod.rs`, Cargo, `task-registry.json`, cargo slot.

Children must not run `cargo test` / `cargo check`. One compile after they stop.

No commit/push. Do not kill Synth MCP (`C:\github\synth\target-studio\debug\synth.exe`).

## Integrator routing after authors stop (do not apply until files exist)

`policy/mod.rs` (B):

```rust
mod remote;
```

Keep remote types crate-private unless the summary names a specific `pub use`. Reuse existing `RECIPIENT_PURPOSE` from `crypto/network/object_stream/types.rs`; do not duplicate it in object_stream.

`runtime/mod.rs` (C):

```rust
mod jobs;
```

`storage/native/mod.rs` (C):

```rust
mod job;
```

Then one `cargo test -p qualia-core-db --lib --locked --offline -j 1` with focused selectors for `bundle::stream::policy::remote` and `bundle::stream::runtime::jobs`. Independent review of B before R2.

## 2026-11-13 resume — live claims after S3.3 / M64

| Child | Tier | Allowed writes | Forbidden |
|---|---|---|---|
| S4 B2 wrap `01a1d40b` | Expert | NEW `bundle/stream/protection/{wrap,wrap_tests}.rs`; EDIT `protection/mod.rs` (mod + re-export only) | policy/remote, authority, storage, cargo, git |
| S2 R2 grant `01a1d411` | Expert | NEW `policy/remote/{issuer,grant,current,lifecycle_tests}.rs`; `evidence/swarm-s2-remote-r2-summary.md` | `remote/mod.rs`, protection/**, cargo, git |
| Cargo slot | Integrator | `evidence/swarm-s3-s4-two-process-m64.log` (`two_os_process` then `m64_v3` then `qbdl::service::`) | overlapping `qualia-core-db` compile |

S3.3 two-process source + independent review: `open_bugs: 0`. M64 v3 source + independent review: `open_bugs: 0` (E0716 test lifetime fixed by integrator). Production `write_leaf` still holds `StorageStage`. WSS/HTTP not started. No commit/push.

## 2026-11-13 resume — live claims after R2 patch + S5 WSS

| Child | Tier | Allowed writes | Forbidden |
|---|---|---|---|
| S5 WSS `01a1d435` | Senior | `p2p/connectivity/wss.rs`, optional `wss_frame.rs`, `evidence/swarm-s5-wss-summary.md` | HTTP module, storage/native/write.rs, cargo, git |
| S2 write_leaf `01a1d439-60cf` | Senior | `storage/native/{write,checkpoint,job}.rs`, optional `write_io.rs`, `evidence/swarm-s2-write-leaf-drop-summary.md` | `native/mod.rs`, `process_crash.rs` protocol names, wss.rs, cargo, git |
| S5 HTTP inventory `01a1d439-e9c7` | Support | `evidence/swarm-s5-http-inventory.md` only | production Rust, cargo, git |
| Cargo slot | Integrator | `evidence/swarm-s4-retry-m64-wrap-r2.log` (m64_v3 → qbdl::service:: → protection::wrap → lifecycle_tests) | overlapping `qualia-core-db` compile |

R2 first review `open_bugs: 2` patched (`join_enrollment` + live `recognize` in recheck). Integrator added `enrolled_controller_cannot_issue_for_unenrolled_source`, `install_independent_source_not_enrolled_denied`, `recognition_rotate_before_recheck_denied`. Independent R2 re-review after cargo green. No commit/push.
