# C2 leftover — drop live stage across destination I/O

File-disjoint. **Did not run cargo. Did not commit.** Synth MCP was not touched. `write.rs` was not edited (convenience `write_leaf` stays a same-stage wrapper). `WriteStep` names unchanged. `process_crash.rs`, `ordering.rs`, `state.rs`, `job.rs`, `tests.rs`, and codec owners were not edited.

Tests were **not** added and **not** run. The `fixtures::write` split is the observable convenience-path change.

## Files written

- `crates/qualia-core-db/src/bundle/stream/storage/native/tests/fixtures.rs`
- `crates/qualia-core-db/src/bundle/stream/storage/native/recovery.rs`
- `crates/qualia-core-db/src/bundle/stream/storage/native/resume.rs`
- `crates/qualia-core-db/src/bundle/stream/storage/native/checkpoint.rs`
- this summary

## What dropped stage

### `fixtures::write`

No longer calls `write_leaf` with a live `StorageStage` across payload/proof/checkpoint I/O. Matches `tests/ordering.rs` `split_hook_write`:

1. `ticket` as before
2. `check_workspace` / `check_leaf` (kept from `write_leaf`; `newer_facade` refuses a rebound workspace)
3. `next = store.next_sequence()?`
4. `attempt = workspace.begin_attempt()`
5. Short `storage_stage` → `prepare_leaf_write` → **drop stage**
6. `wait_leaf_write(armed, &mut |_| Ok(()))?` — no stage
7. New short `storage_stage` on the same ticket → `commit_leaf_write`
8. `stage.recheck` (commit does not)
9. `finish_leaf_write(next, 0, attempt)`
10. `finish(rt, &mut op)` as before

### Recovery / resume destination writes

`store_verified` no longer takes `StorageStageControl` / `LocalEffectControl` and no longer calls `write_verified_bytes`. It only records the uncommitted integrity bit and encodes checkpoint bytes under a short stage (`CandidateBitmapPage` is a stage loan).

`recover_leaf_from` / `reverify_and_rewrite_leaf`:

1. Short callback: recheck, copy payload/proof into already-charged scratch, verify, `UnacceptedMaterial` (recovery only), poison, `store_verified`, `ArmedNativeLeafWrite::from_prepared`, **drop callback**
2. Outside callback: `wait_leaf_write` → `write_verified_bytes` / `session_io`
3. Short callback: one-shot grant + `gates(control, authority, …)` then `stage.recheck` + `finish_leaf_write`

Grant is spent after destination I/O, same as `write_leaf` Phase 3. `UnacceptedMaterial` still zeros unaccepted source proof/payload on every return/unwind from the first recovery callback (including after a successful copy into the armed plan).

## What I/O still holds a loan

| Site | I/O | Why it still runs under a callback |
|---|---|---|
| `recover_leaf_from` source `read_exact_at` | old payload + proof | `RangeRecoveryRead` (`old`) is still a `with_recovery_buffers` loan for the copy-out only. Verify/arm run after that loan drops (follow-on `swarm-c2-write-leaf-stage-drop.md`). |
| `reverify_and_rewrite_leaf` session `read_exact_at` | dest payload + proof | **closed** in the follow-on: session reads go into `CopiedLeafMaterial` with no stage. |
| `resume_hint_for_leaf` | checkpoint bank reads | not a `store_verified` path; unchanged |
| `read_verified_leaf` / `read_material` | session payload + proof | `read.rs` not in this claim |
| `write_leaf` / `write_leaf_with_hook` | same-stage prepare/wait/commit | **closed** in the follow-on: runtime+ticket, short stage, drop, wait, new short stage. |
| `recover_leaf_from` / `reverify_and_rewrite_leaf` **outer** `StorageStage` | exclusive `TransferRuntime` borrow | **closed** in the follow-on: signatures take `TransferRuntime` + ticket + scratch. |

Destination **writes** (`write_verified_bytes` / `session_io`) do not run inside `with_recovery_buffers` / `with_sparse_page_buffers`.

## Tests

None written. None run. No `tests/stage_drop.rs`. No `tests.rs` edit.

## Not done

- Did not run `cargo test` / `cargo check` / `cargo build`.
- Did not commit.
- Did not kill Synth MCP.
