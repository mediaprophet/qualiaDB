# C2 leftover close — production write_leaf + recovery/resume copy-out

2026-11-15. Integrator. **Did not run cargo. Did not commit.** Synth MCP was not touched. `WriteStep` names unchanged.

ZIP `.hmc` streaming was on the leftover list. User: ZIP is the legacy document-archive profile; platform unreleased; do not add support. QBDL admission already refuses non-`QBDL` magic. No ZIP module.

## Production `write_leaf`

`write.rs` convenience no longer takes `&mut StorageStage` across wait:

1. `runtime.storage_stage` → `prepare_leaf_write` → drop stage
2. `wait_leaf_write` — no stage
3. new `storage_stage` → `commit_leaf_write` → `recheck` → `finish_leaf_write`

Callers updated: `tests/state.rs`, `tests/leaves.rs`, `tests/allocation.rs`. `fixtures::write` and `process_crash` already used the split path.

## Recovery / resume

- `CopiedLeafMaterial` owns payload/proof/header copies. Drop zeros them.
- `recover_leaf_from`: `RangeRecoveryRead` loan only for the source copy-out; verify/arm on a later short sparse-page stage; wait has no stage. Source `read_exact_at` still requires the loan.
- `reverify_and_rewrite_leaf`: store `session.read_exact_at` into the owned copy with no stage; short stage for verify/arm; wait has no stage.
- Public recover/reverify signatures now take `TransferRuntime` + ticket + scratch, matching `write_leaf`.
- `resume_hint_for_leaf` unchanged (checkpoint bank reads).

## Still later (do not fake)

PhysicalNetwork two-host. Browser/WASM consumer. C6 records and independent reviews after cargo.
