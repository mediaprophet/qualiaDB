---
pass: true
open_bugs: 0
---

# Wave S2 residual / HMC R05 — independent review

Read-only. Cargo not run. Production Rust not edited. Author summary not treated as proof. `process_crash.rs` protocol names were not changed in this review.

HMC R05 (`implementation-review-hmc-multiplexing-2026-11-13.md:72-78`) claimed `write.rs:88-126` still called `store_verified` inside a live `StorageStage` buffer scope, and that `ArmedNativeWrite::wait_blocking` only invoking a closure was not a real durable job. **Current source supersedes that gap on the production `write_leaf` path.** Occupancy `ArmedNativeWrite` still exists as scaffolding; `write_leaf_with_hook` does not use it for I/O.

## Production drop (HMC R05 closed for `write_leaf`)

`write_leaf` is a thin crash-protocol wrapper (`write.rs:36-58`) into `write_leaf_with_hook` (`write.rs:68-110`).

Phase 1 `prepare_leaf_write` (`write.rs:116-178`) runs `stage.with_sparse_page_buffers` (`write.rs:129-177`), copies payload/proof/header/bitmap into owned `ArmedNativeLeafWrite`, sets `StoreState::Poisoned` (`write.rs:154`), encodes checkpoint bytes, and **returns**. The stage callback is over before wait.

Phase 2:

```
write.rs:86-89
    let session = &mut self.session;
    armed.wait_blocking(hook, &mut |op, bytes| session_io(session, op, bytes))?;
```

That is `ArmedNativeLeafWrite::wait_blocking` (`job.rs:129-145`), not occupancy `ArmedNativeWrite::wait_blocking` (`job.rs:30-34`). Signature has no `StorageStage`, `LocalWholeSourceGrant`, `CurrentAuthorityGuard`, or `RuntimeParents`. It calls `write_verified_bytes` (`checkpoint.rs:23-63`) then `session_io` (`checkpoint.rs:65-76`) → `write_exact_at` / `sync_file`. No `gates()`.

Phase 3 `commit_leaf_write` (`write.rs:183-230`) is the first `grant.accept_local_scoped` on this path (one-shot spent after I/O). Inner `gates` + `verify_leaf_view` + `page.record_integrity`, then outer `stage.recheck` (`write.rs:100`) **before** `attempt.commit()` / `sequence = next` / `StoreState::Active` (`write.rs:101-103`). `write.rs` has no `store_verified`.

## WriteStep order (unchanged)

Enum `write.rs:17-32` matches `write_verified_bytes` `checkpoint.rs:37-61`:

`PayloadWrite`, `PayloadWritten`, `ProofWrite`, `ProofWritten`, `PayloadSync`, `PayloadSynced`, `ProofSync`, `ProofSynced`, `BitmapWrite`, `BitmapWritten`, `HeaderWrite`, `HeaderWritten`, `CheckpointSync`, `CheckpointSynced`, `FinalCheck`.

Crash child still uses `write_leaf_with_hook` (`process_crash.rs:56-77`). Protocol names untouched: `CHILD_TEST` (`process_crash.rs:13-14`), `QUALIA_QBS_CRASH_TEST_ROOT` / `POINT` (`process_crash.rs:15-16`), `READY` (`process_crash.rs:17`), `crash_writer_child` (`process_crash.rs:31`).

## Failed Phase 3 / wait fail-closed

Wait or `commit_leaf_write` `?` returns before `attempt.commit` / sequence bump / `Active`. Phase 1 already set `Poisoned`. Uncommitted `WorkspacePageAttempt` drop invalidates the page (`sparse_workspace/types.rs:119-124`). Ordering/state tests still assert no integrity mark + `Poisoned` on hook/policy/timeout/unwind (`ordering.rs:120-121`, `ordering.rs:182-183`, `state.rs:67-68`, `state.rs:107-108`). Disk bytes from Phase 2 stay unverified (crash reopen still re-verifies; `process_crash.rs:188`).

## Tests (exist; names as required)

In `storage/native/job.rs`:

- `production_write_wait_site_takes_no_stage_grant_or_parent_mutex` (`job.rs:356`) — drops `CurrentAuthorityGuard` + `RuntimeParents::try_with_runtime`, then `ArmedNativeLeafWrite::wait_blocking`; `ledger.try_lock()` and `CurrentAuthorityGuard::try_lock` succeed at `PayloadWrite` (`job.rs:420-430`).
- `write_leaf_drops_stage_before_payload_write_hook` (`job.rs:450`) — wait API, not the crash wrapper; exclusive `rt.cancel` at `PayloadWrite`; first hook is `PayloadWrite`.
- `write_drops_stage_before_simulated_blocking_hook` (`job.rs:319`) — occupancy wait, kept.

No second production `ReservationLedger`. No JSON in `native/`. Tests construct one fixture ledger (`job.rs:255`, `job.rs:362`).

## Residuals (not open_bugs)

1. Crash wrapper still *takes* `&mut StorageStage` and `&mut LocalWholeSourceGrant` (`write.rs:68-78`) for Phase 1/3. Caller borrow remains live across wait; wait-site types do not.
2. `store_verified` (`checkpoint.rs:81-120`) still takes `StorageStageControl` / `LocalEffectControl`, writes via `write_verified_bytes`, then `gates()` after `FinalCheck`. Callers are recovery/resume only: `recover_leaf_from` (`recovery.rs:105-120` inside `with_recovery_buffers`) and `reverify_and_rewrite_leaf` (`resume.rs:178-193` inside `with_sparse_page_buffers`). Not on live `write_leaf`.
3. `ArmedNativeWrite` occupancy wait (`job.rs:30-34`) remains a closure-only scalar. Not a substitute; production I/O is `ArmedNativeLeafWrite`.
4. `write_leaf_drops_stage_before_payload_write_hook` uses `for_wait_site_test` dummy I/O (`job.rs:472-485`), not `NativeSparseStore::write_leaf`. Production wiring is `write.rs:86-89`.
5. Phase 1 `page.record_integrity` (`write.rs:155`) mutates the uncommitted attempt so checkpoint *bytes* include the hint. Durable in-memory publication remains `attempt.commit()` after Phase 3. `arm_leaf_write` still spends the grant at occupancy-arm time (`job.rs:206-228`); that API is not the `write_leaf` I/O path.

No leftover `store_verified` I/O-inside-gates on live production `write_leaf`.
