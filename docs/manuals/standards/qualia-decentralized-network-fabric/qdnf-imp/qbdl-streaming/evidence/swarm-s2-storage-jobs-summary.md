# Wave S2 / critical path C — storage jobs outside owner locks

Bounded submitted storage jobs with ticket, reserved bytes, cancel, and exactly-once completion. Native adapter arms a write under a short StorageStage / grant recheck, then drops those borrows before the OS wait. Original transfer charges stay on the existing ReservationLedger until actual I/O completion.

## Files written

- `crates/qualia-core-db/src/bundle/stream/runtime/jobs/mod.rs`
- `crates/qualia-core-db/src/bundle/stream/runtime/jobs/ticket.rs`
- `crates/qualia-core-db/src/bundle/stream/runtime/jobs/submit.rs`
- `crates/qualia-core-db/src/bundle/stream/runtime/jobs/complete.rs`
- `crates/qualia-core-db/src/bundle/stream/runtime/jobs/tests.rs`
- `crates/qualia-core-db/src/bundle/stream/storage/native/job.rs`
- this summary

`storage/native/write.rs` was **not** edited. Crash-hook barriers (`WriteStep`, `write_leaf_with_hook`) and `process_crash.rs` child protocol are unchanged.

## Key decisions

- `StorageJobs` is occupancy only. It binds the runtime owner id and the original `CompletionKey`. It does not construct a second `ReservationLedger`, lease table, or quota.
- Submit creates a short `StorageStage`, copies payload into already-charged scratch when it fits, records reserved bytes, then drops the stage before returning.
- `StorageJobTicket` is non-Copy, generation-fenced, and cannot complete without the matching live `TransferTicket`.
- Cancel marks job `Cancelling` and calls the original `TransferRuntime::cancel`. Buffers stay occupied. `complete_io` is exactly-once and is the only refund.
- Late complete after cancel returns `FinishOutcome::Cancelled`. `may_publish_verified` is false, so verified bits must not be set.
- Worker panic/error: charges stay; `observe_worker_failure` cancels without releasing the slot; complete is still required.
- Native `arm_write_from_stage` / `NativeSparseStore::arm_leaf_write` recheck then return `ArmedNativeWrite`. `wait_blocking` takes no `StorageStage`, grant, `CurrentAuthorityGuard`, or `RuntimeParents`. Comment at the wait site records that `Mutex<AuthorityOwner>` / `RuntimeParents` are not held.

## Requested integrator patch

`crates/qualia-core-db/src/bundle/stream/runtime/mod.rs` (after `mod storage_stage;`):

```rust
mod jobs;
pub use jobs::{StorageJobTicket, StorageJobs, MAX_STORAGE_JOBS};
```

`crates/qualia-core-db/src/bundle/stream/storage/native/mod.rs` (after `mod write;`):

```rust
mod job;
pub use job::{arm_write_from_stage, ArmedNativeWrite};
```

`NativeSparseStore::arm_leaf_write` is an inherent impl in `job.rs`; routing `mod job` is enough. No `Cargo.toml` change.

## Test names

`runtime/jobs/tests.rs`:

- `submit_slow_complete_keeps_live_transfers_charged_until_complete`
- `cancel_then_complete_is_cancelled_without_double_refund_or_early_reuse`
- `worker_panic_or_error_conserves_charges_and_does_not_free_slot`
- `job_ticket_from_owner_a_cannot_complete_on_owner_b`
- `wait_site_does_not_hold_runtime_parents_or_authority_mutex`

`storage/native/job.rs`:

- `write_drops_stage_before_simulated_blocking_hook`

Existing `process_crash.rs` tests are reused, not replaced.

## Remaining `write.rs` risks

- `write_leaf` / `write_leaf_with_hook` still hold `&mut StorageStage` and `&mut LocalWholeSourceGrant` (therefore `TransferRuntime` and `AuthorityOwner`) across `write_exact_at` / `sync_file`.
- `store_verified` still takes `StorageStageControl` and rechecks under those borrows at every barrier.
- Production callers of `write_leaf` are not migrated to `arm_leaf_write` + `wait_blocking`. Connecting the armed wait to actual payload/proof/checkpoint I/O without re-holding the stage is follow-up work and must not weaken crash barriers.
- `write.rs` was left unchanged so the process-crash child protocol (`CHILD_TEST`, `QUALIA_QBS_CRASH_TEST_ROOT` / `POINT`, `READY` marker, `WriteStep` names) stays intact.

## Stop notes

- Did not edit `runtime/mod.rs`, `storage/native/mod.rs`, `write.rs`, `process_crash.rs`, authority, policy, or `Cargo.toml`.
- Did not invent a second reservation ledger.
- Did not run `cargo test` / `cargo check` / `cargo build`.
- No git commit/push. Synth MCP was not touched.

Formatted only the six new Rust files with `rustfmt --edition 2021`.
