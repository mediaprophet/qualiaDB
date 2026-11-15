# Swarm S3 claim board

Integrator-owned. S3.1 and S3.2 closed in source. S3.3 live.

| Child | Tier | Status | Allowed writes | Forbidden |
|---|---|---|---|---|
| S3a frame | Senior | closed | `local_stream/{mod,frame,tests}.rs` | parent routing, OS backends |
| S3b service | Expert | closed 5/5 | `qbdl/service/{mod,session,provider,client,tests}.rs` | wire/carrier/provider, bearer |
| S3c protocol | Support | closed | `evidence/swarm-s3-two-process-protocol.md` | crates |
| S3.2 Windows | Senior | closed (bind works; name-count assertion patched) | `local_stream/windows.rs` | frame.rs, unix.rs |
| S3.2 Unix | Senior | closed cfg | `local_stream/unix.rs` | windows.rs |
| S3.3 two-process | Expert | claimed | NEW `local_stream/two_process.rs`; NEW `qbdl/service/two_process.rs`; EDIT `qbdl/service/{session,client,provider,mod}.rs` for generic Bearer + WouldBlock retry; `evidence/swarm-s3-two-process-summary.md` | `bearer/mod.rs`, `local_stream/mod.rs`, `qbdl/mod.rs`, windows.rs, unix.rs, wire/carrier, registries, cargo, git |

Integrator retains: `bearer/mod.rs`, `local_stream/mod.rs` routing, `qbdl/mod.rs`, Cargo, cargo slot.

No commit/push. Do not kill Synth MCP.
