# Wave 2.5-C2 — crash-wrapper stage split

File-disjoint. **Did not run cargo. Did not commit/push.** Synth MCP was not touched. `job.rs`, `tests.rs`, `tests/fixtures.rs`, `runtime/**`, and EMF/QAC owner files were not edited. `WriteStep` variant names and discriminant order are unchanged. `process_crash.rs` `point()` still maps `payload-written` / `payload-synced` / `proof-synced` / `header-written` / `checkpoint-synced`. Env vars, `CHILD_TEST`, and `QBS_CRASH_READY` are unchanged.

Tests were written, **not run**.

## Files written

- `crates/qualia-core-db/src/bundle/stream/storage/native/write.rs`
- `crates/qualia-core-db/src/bundle/stream/storage/native/tests/process_crash.rs`
- `crates/qualia-core-db/src/bundle/stream/storage/native/tests/state.rs`
- `crates/qualia-core-db/src/bundle/stream/storage/native/tests/ordering.rs`
- this summary

## New signatures (`write.rs`)

```rust
pub(super) fn next_sequence(&self) -> Result<u64, E>  // sequence.checked_add(1)

pub(super) fn prepare_leaf_write(
    &mut self,
    stage: &mut StorageStage<'_, '_>,
    workspace: &mut SparsePageWorkspace,
    leaf: u64,
    nodes: &mut [StrongDigest],
    next: u64,
    clock: &impl Fn() -> u64,
    policy: &impl Fn() -> Generation,
) -> Result<ArmedNativeLeafWrite, E>

pub(super) fn wait_leaf_write(
    &mut self,
    armed: ArmedNativeLeafWrite,
    hook: &mut impl FnMut(WriteStep) -> Result<(), E>,
) -> Result<(), E>  // session_io; no stage/grant in the signature

pub(super) fn commit_leaf_write(
    &mut self,
    stage: &mut StorageStage<'_, '_>,
    grant: &mut LocalWholeSourceGrant<'_, '_>,
    pins: &PinAuthority,
    workspace: &mut SparsePageWorkspace,
    leaf: u64,
    nodes: &mut [StrongDigest],
    clock: &impl Fn() -> u64,
    policy: &impl Fn() -> Generation,
) -> Result<(), E>

pub(super) fn finish_leaf_write(
    &mut self,
    next: u64,
    leaf: u64,
    attempt: WorkspacePageAttempt<'_>,
) -> Result<LeafBarrierObservation, E>
```

`finish_leaf_write` is the success-path publication: `attempt.commit()`, `sequence = next`, `StoreState::Active`, observation. Callers `stage.recheck` immediately before it (same order as the old wrapper).

`write_leaf` still takes `&mut StorageStage` (fixtures::write uses it) and still calls `write_leaf_with_hook` on that same stage object. `write_leaf_with_hook` is now a thin same-stage wrapper over prepare / wait / commit / finish. Crash tests do not call it.

## Crash path (R05 residual)

Replace `write_leaf_with_hook(&mut stage, …)` held across OS I/O with:

1. `next = store.next_sequence()`
2. `WorkspacePageAttempt` on the caller workspace (drop still invalidates marks)
3. Short `storage_stage` → `prepare_leaf_write` → **drop stage**
4. `wait_leaf_write(armed, hook)` — no stage in the frame; hook still parks at the named `WriteStep`
5. New `storage_stage` on the **same ticket** → `commit_leaf_write` → `stage.recheck` → `finish_leaf_write`

Crash-at-barrier tests park during wait and never reach finish — same as now.

## Tests moved off `write_leaf_with_hook`

- `process_crash::crash_writer_child` — parks in `wait_leaf_write`; still writes `QBS_CRASH_READY`
- `state::current_policy_change_or_timeout_after_real_io_clears_marks_and_poison` — HeaderWrite mutates clock/policy; **commit** fails; Poisoned; no integrity mark
- `state::unwind_after_real_sync_clears_armed_page_and_keeps_store_poisoned` — panic at CheckpointSynced still poisons
- `ordering::actual_writes_and_barriers_precede_checkpoint_and_duplicate_uses_same_charge` — same WriteStep event vector; duplicate-charge still uses `fixtures::write`
- `ordering::every_stopped_real_protocol_boundary_poison_clears_and_new_facade_cannot_reuse_marks`
- `ordering::outer_authority_final_check_failure_cannot_commit_page_attempt`

## Leftover (stage still in the convenience signature)

- `fixtures::write` still calls `write_leaf` with a live `StorageStage` for the whole convenience call (not edited; not a crash path)
- `state::malformed_input_refuses_before_file_growth_and_does_not_poison` still uses `write_leaf`
- `process_crash` recovery reopen still uses `recover_leaf_from` under a live stage (out of scope)

## `storage_stage` twice on one ticket

Source read of `TransferRuntime::storage_stage` / `check_storage_stage`: there is no “stage already issued” flag. The exclusive `&mut TransferRuntime` borrow ends when `StorageStage` is dropped. A second `storage_stage` on the same `TransferTicket` is allowed while the slot stays `InFlight`.

No runtime API change is required. Second-stage construction uses the original fixture inputs `(now=20, Generation(1))`, not the mutated HeaderWrite clock/policy, so those tests still fail inside `commit_leaf_write` rather than at stage create.

If a later runtime change forbade a second stage on one ticket, the integrator would need a reopen that re-borrows an InFlight slot after the previous `StorageStage` drop (same payload/proof/scratch, no second credit). Do not edit `runtime/**` in this wave.
