# Wave 2.5-C2 — crash-wrapper stage split (senior)

File-disjoint. **Do not cargo. Do not commit.** Exclusive writes:

- `crates/qualia-core-db/src/bundle/stream/storage/native/write.rs`
- `crates/qualia-core-db/src/bundle/stream/storage/native/tests/process_crash.rs`
- `.../tests/state.rs`
- `.../tests/ordering.rs`
- evidence `swarm-wave25-c2-summary.md`

Do **not** edit `job.rs`, `tests.rs`, `tests/fixtures.rs`. `WriteStep` **variant names and discriminant order stay frozen** (`process_crash.rs` `point()` maps `"payload-written"` etc.).

## Gap

Production `write_leaf` already splits internally: Phase 1 `prepare_leaf_write` (short stage) / Phase 2 `ArmedNativeLeafWrite::wait_blocking` (no stage) / Phase 3 `commit_leaf_write` (short grant+stage).

`write_leaf_with_hook` still takes `&mut StorageStage` for the whole call, so crash tests in `process_crash.rs` / `state.rs` / `ordering.rs` **hold stage across OS I/O**. R05 residual.

`job.rs` `arm_leaf_write` is the occupancy helper. `ArmedNativeLeafWrite` (payload-owning plan) lives in `job.rs` and is already returned by `prepare_leaf_write`.

## Change in `write.rs`

1. Make `prepare_leaf_write` and `commit_leaf_write` `pub(super)`.
2. Add `pub(super) fn wait_leaf_write(&mut self, armed: ArmedNativeLeafWrite, hook: &mut impl FnMut(WriteStep) -> Result<(), E>)` that does the existing session_io wait **with no stage/grant in the signature**.
3. Keep `write_leaf` as a convenience that still takes `stage` (fixtures::write uses it). Internally it may still call prepare/wait/commit on that same stage object; the crash path must not.
4. Keep `write_leaf_with_hook` if other in-module code needs it, or rewrite it as a thin wrapper. Crash tests must **stop calling it**.
5. After successful wait+commit, the current `write_leaf_with_hook` still sets `sequence`, `StoreState::Active`, `attempt.commit()`, `stage.recheck`. Expose that as `pub(super) fn finish_leaf_write(...)` or document that callers of the split path must do the same sequence/state/recheck that `write_leaf_with_hook` does today. Crash-at-barrier tests park **during wait** and never reach finish — same as now.

`prepare_leaf_write` currently takes `next: u64` (already `sequence+1`). Crash tests must compute `next` the same way (`store` sequence field is private — if you cannot read it, add `pub(super) fn next_sequence(&self) -> Result<u64, E>` as `sequence.checked_add(1)`). Do not change StoreState machine besides existing Poisoned-on-prepare.

## Crash tests

Replace:

```
let mut stage = rt.storage_stage(...)?;
store.write_leaf_with_hook(&mut stage, grant, ..., hook)
```

with:

```
let next = store.next_sequence()?; // or equivalent
let armed = {
    let mut stage = rt.storage_stage(...)?;
    store.prepare_leaf_write(&mut stage, workspace, leaf, nodes, next, clock, policy)?
}; // stage dropped before wait
store.wait_leaf_write(armed, hook)?;
{
    let mut stage = rt.storage_stage(...)?;
    store.commit_leaf_write(&mut stage, grant, pins, workspace, leaf, nodes, clock, policy)?;
    stage.recheck(clock(), policy())?;
    store.finish_leaf_write(next)?; // sequence/Active/attempt — match current success path
}
```

`process_crash.rs`: hook still parks at the named `WriteStep`. Child still writes `QBS_CRASH_READY`. Do not rename env vars, `CHILD_TEST` path, or barrier strings.

`state.rs`: policy/clock mutation at `HeaderWrite` must still make the **commit** fail; store ends Poisoned; no integrity mark. Panic at `CheckpointSynced` still poisons.

`ordering.rs`: event vector must still be the same WriteStep sequence. Duplicate-charge test behaviour stays.

`storage_stage` may be created twice on the same ticket (prepare + commit). If the runtime forbids that, keep one ticket and document the exact API you need from integrator — do not edit `runtime/**`.

## Stop

Write `docs/.../qbdl-streaming/evidence/swarm-wave25-c2-summary.md`: new signatures, which tests moved off `write_leaf_with_hook`, leftover (`fixtures::write` still uses `write_leaf` with stage). Tests written **not run**. Do not cargo. Do not commit. Do not touch EMF/QAC owner files.
