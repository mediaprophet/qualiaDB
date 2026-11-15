# Wave S2 residual — production `write_leaf` drops stage/grant before OS I/O

`write_leaf` / `write_leaf_with_hook` keep the crash-protocol signatures. Internally they no longer hold `StorageStageControl` or `LocalEffectControl` across `write_exact_at` / `sync_file`. `WriteStep` names and order are unchanged. `process_crash.rs` was not edited.

Did not run cargo. No commit/push. Synth MCP was not touched. `native/mod.rs` was not edited.

## Files written

- `crates/qualia-core-db/src/bundle/stream/storage/native/write.rs`
- `crates/qualia-core-db/src/bundle/stream/storage/native/checkpoint.rs`
- `crates/qualia-core-db/src/bundle/stream/storage/native/job.rs`
- this summary

No `write_io.rs` (write.rs stayed under ~400 implementation lines). No `write_stage_tests.rs` (tests live in `job.rs`).

## Phase split

**Phase 1 — `NativeSparseStore::prepare_leaf_write`.** Short `StorageStage::with_sparse_page_buffers`: identity/page/scratch checks, `verify_leaf_view`, copy proof/header into already-charged scratch and payload into the armed buffer, `StoreState::Poisoned` before any file effect. Checkpoint *bytes* are materialized here (`record_integrity` on the uncommitted `WorkspacePageAttempt` page, then `encode_checkpoint`) so Phase 2 can write them without a live `CandidateBitmapPage`. The stage callback returns. The one-shot `LocalWholeSourceGrant` is **not** spent here; `accept_local_scoped` is one-shot and must run after I/O so `outer_authority_final_check_failure_cannot_commit_page_attempt` still fails closed.

**Phase 2 — wait site `ArmedNativeLeafWrite::wait_blocking`.** Blocking payload/proof/checkpoint writes and syncs. No `StorageStage`, `LocalWholeSourceGrant`, `CurrentAuthorityGuard`, or `RuntimeParents` in the signature. Does not call `gates()`. `WriteStep` order is identical:

`PayloadWrite`, `PayloadWritten`, `ProofWrite`, `ProofWritten`, `PayloadSync`, `PayloadSynced`, `ProofSync`, `ProofSynced`, `BitmapWrite`, `BitmapWritten`, `HeaderWrite`, `HeaderWritten`, `CheckpointSync`, `CheckpointSynced`, `FinalCheck`.

**Phase 3 — `NativeSparseStore::commit_leaf_write`.** Short grant (`accept_local_scoped`) plus `StorageStage`. Recheck clock/policy/identity. `page.record_integrity` (in-memory publication; already marked from Phase 1 encoding, or `AlreadyMarked`). Then outer `stage.recheck`, `attempt.commit()`, `sequence++`, `StoreState::Active`.

If Phase 3 recheck fails or wait errors: payload/proof/checkpoint bytes stay unverified, bitmap bits are not published (`WorkspacePageAttempt` drop invalidates the page), `Poisoned` is preserved. Resume re-verifies. Late complete after cancel cannot set verified bits.

## Wait-site signature

Occupancy primitive (unchanged):

```rust
impl ArmedNativeWrite {
    pub fn wait_blocking<R>(self, hook: impl FnOnce() -> R) -> R
}
```

Production I/O wait site:

```rust
impl ArmedNativeLeafWrite {
    pub(crate) fn wait_blocking(
        self,
        hook: &mut impl FnMut(WriteStep) -> Result<(), E>,
        io: &mut impl FnMut(LeafIoOp, &[u8]) -> Result<(), E>,
    ) -> Result<(), E>
}
```

`write_leaf_with_hook` calls that wait site with `session_io` closures over `RangeSession`. I/O extraction is `write_verified_bytes` in `checkpoint.rs` (no `StorageStageControl` / `LocalEffectControl`). `store_verified` keeps its recovery/resume signature and now delegates I/O to `write_verified_bytes`, then `gates()` once after `FinalCheck`.

Crash callers still pass `&mut stage`; the function uses it only in Phase 1 and Phase 3. Production callers should prefer `arm_leaf_write` (occupancy) / prepare + `ArmedNativeLeafWrite::wait_blocking` + Phase 3 commit so the wait-site frame does not even mention `StorageStage`.

## Bitmap / checkpoint split

In-memory bitmap publication cannot be done without a live page (hence a short Phase 3 `StorageStage`). Checkpoint **bytes** (header + bitmap payload including the would-be integrity hint) are written in Phase 2. In-memory marks become durable only at `attempt.commit()` after Phase 3 recheck. Stated because `CandidateBitmapPage` is only available inside `with_sparse_page_buffers`.

## Test names

In `storage/native/job.rs`:

- `production_write_wait_site_takes_no_stage_grant_or_parent_mutex` — arms under `CurrentAuthorityGuard` + `RuntimeParents::try_with_runtime`, drops those scopes, then `ArmedNativeLeafWrite::wait_blocking`; `ledger.try_lock()` and `CurrentAuthorityGuard::try_lock` succeed during `PayloadWrite`.
- `write_leaf_drops_stage_before_payload_write_hook` — goes through the new wait API (not the crash-hook wrapper). At `PayloadWrite` the original runtime is used exclusively (`live_transfers` / `cancel`). First hook is `PayloadWrite`.
- `write_drops_stage_before_simulated_blocking_hook` — kept, same meaning.

## Integrator routing

None required for a new module. Tests are `#[cfg(test)]` in `job.rs`. Optional later:

```rust
pub use job::ArmedNativeLeafWrite;
```

`ArmedNativeLeafWrite` is `pub(crate)` today. `mod job` is already routed.

Do not rename `WriteStep` variants, `CHILD_TEST`, `QUALIA_QBS_CRASH_TEST_ROOT` / `POINT`, the `READY` marker, or `crash_writer_child`.

## Remaining risks

- `write_leaf_with_hook` still *takes* `&mut StorageStage` and `&mut LocalWholeSourceGrant` (crash child). Inner callbacks are dropped before OS wait; exclusive runtime use during wait cannot be proven from that wrapper, which is why the new test uses `wait_blocking`.
- Recovery/resume still call `store_verified` under a live stage callback (those files were not in this claim). I/O no longer takes stage/grant controls, but the caller still holds them.
- `arm_leaf_write` still spends the grant at occupancy-arm time. `write_leaf_with_hook` defers the one-shot grant to Phase 3. A production `arm_verified` + wait + **fresh** grant commit path is not wired beyond `write_leaf`.
- Phase 1 `record_integrity` mutates the uncommitted attempt page so checkpoint bytes include the hint bit. Failed Phase 3 still invalidates; a mid-wait `has_integrity_mark` observer would see the bit earlier than the old ProofSynced-gated point.
- Phase 2 does not call `gates()`. Policy/clock/cancel changes during wait are observed at Phase 3. Existing hook tests that mutate policy at `HeaderWrite` still fail closed; they no longer abort the header write itself.
- Did not run `cargo test` / `cargo check` / `cargo build`.

Formatted only the three Rust files with `rustfmt --edition 2021`.
