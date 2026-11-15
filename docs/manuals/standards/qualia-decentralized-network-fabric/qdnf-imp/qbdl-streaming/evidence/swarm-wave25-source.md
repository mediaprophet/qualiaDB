# Wave 2.5 source — 2026-11-14, no cargo

EMF/QAC owner is still editing. Streaming did **not** run `qualia-core-db --lib`. Tests below are written, not executed.

## Landed

| Slice | Source | Notes |
|---|---|---|
| D class 8/9 | `qbdl/service/describe.rs` | `send_body_describe` / `send_entry_page_describe`; `answer_describe_pages`; client `select_entry` from wire bytes only |
| D routing | `provider.rs`, `service/mod.rs` | Integrator: `PlainProvider::answer_describe_pages`; `pub use answer_describe_pages`. Source-only `answer_describe` still refuses 8/9 |
| P protected two-process | `qbdl/service/two_process.rs` | Binary `QBDL-PSK1` package (`QDNF_TWO_PROCESS_KEYS`); Initiator child / Responder parent; handshake then `ProtectedSession` useful-leaf. Framing Session fixture kept. `EvidenceClass::Process` |
| C2 crash split | `storage/native/write.rs` + `tests/{process_crash,state,ordering}.rs` | prepare / wait_leaf_write / commit / finish. Crash tests drop `StorageStage` during wait. `WriteStep` names frozen |
| Many-per-family planner | `bundle/stream/plan/tests.rs` | 3 QAC + 3 EMF + 2 TenD compile; two QAC + two EMF schedule delays M64; duplicate **entry id** still refuses |

## Leftovers (named, not papered over)

- Session/ProtectedSession DescriptorPage buffers are `MAX_LEAF_RECORD_BYTES` (18137). Wire `MAX_RECORD_BYTES` is 65776 (prefix + 32 + 65536). Fixture pages fit; a max-size class 8/9 page needs a later buffer lift (stack size).
- `fixtures::write` still calls `write_leaf` with a live stage (convenience).
- Recovery `store_verified` may still run under a live stage.
- `CryptographicLibrary::{get_key,store_key}` still `pub(super)` on the inner manager; two-process uses public `KeyManager`. Optional vis bump documented in `swarm-wave25-p-summary.md`.
- Wave 3 mixed-HMC **binary** fixture waits on real EMF/QAC writers.
- Cargo of Wave 1 C3/C4/C5 + Wave 2 + Wave 2.5 waits on user notify that EMF/QAC is done.

## Agents

- D `01a1d8f8-c6de-70b1-82a5-61e7b17693fa`
- P `01a1d8f8-c6de-70b1-82a5-61fa6567b4ce`
- C2 `01a1d8f8-c6de-70b1-82a5-6203c998c28d`

No commit/push. Synth MCP left running.
