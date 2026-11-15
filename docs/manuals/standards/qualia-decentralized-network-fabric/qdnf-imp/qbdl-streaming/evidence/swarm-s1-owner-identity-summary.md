# Wave S1 / critical path A — AuthorityOwner identity

Bind QDNF `AuthorityOwner` to an actual owner identity (`CanonicalIncarnation`), not mutex pointer affinity alone.

## Files changed

- `crates/qualia-core-db/src/net/qdnf/authority/owner.rs`
- `crates/qualia-core-db/src/net/qdnf/authority/permit.rs`
- `crates/qualia-core-db/src/net/qdnf/authority/current_guard.rs`
- `crates/qualia-core-db/src/net/qdnf/authority/current_guard/tests.rs`
- `crates/qualia-core-db/src/net/qdnf/authority/owner_tests.rs`
- this summary

No `owner/identity.rs` and no `authority/mod.rs` edit. Identity helpers live as a small `impl AuthorityOwner` in `owner.rs` (`bind_incarnation` / `existing_incarnation` / `require_identity`). `owner.rs` stays under ~500 lines excluding tests.

## Key decisions

- Reuse `crate::net::peer::runtime::incarnation::CanonicalIncarnation` in place. Do not export a new identity type.
- `AuthorityOwner::new()` stays `const`; identity starts `UNASSIGNED`. No `Copy`/`Clone` on `AuthorityOwner`. Handles/permits stay `Copy` because `CanonicalIncarnation` is `Copy`.
- Bind lazily in `install_grant_with_ceiling` only after every fallible admission check (`free_slot`, generation bump, credential/contact constructors). Failed admission never assigns. Later successful installs reuse the bound identity (`bind()` is stable).
- Private `incarnation` on `AuthorityOwner`, `MutationHandle`, and `ExecutionPermit`. `record()` / `record_mut()` reject unassigned and foreign identity with `QdnfError::StaleGeneration` (same reconstructed-parent code as runtime identity tests).
- `MutationHandle::matches_permit` and `ExecutionPermit::issue` reject unassigned / mismatched identity (`StaleGeneration`). `issue` stays `pub(crate)`; identity is a crate-private argument, not a public constructor.
- `CurrentAuthorityGuard::check_affinity` keeps mutex `ptr::eq` (`Unauthorized` on mismatch) **and** requires the guarded owner identity to be assigned (`StaleGeneration` if unassigned). `check_handle` still goes through `issue_permit` → `record()`, so a reconstructed owner at the same mutex address fails even when slot/generation/principal collide.
- Moved original owner value remains valid; restore-after-replace works.

## Test names added

`owner_tests.rs`:

- `replaced_same_mutex_equal_records_refuse_then_restored_original_works`
- `foreign_equal_generation_permit_is_rejected`
- `moved_original_owner_preserves_handle_and_permit`
- `failed_admission_does_not_assign_identity`

`current_guard/tests.rs`:

- `replaced_same_mutex_equal_records_refuse_check_handle_then_restore`

Existing `pure_checks_have_zero_raw_allocations_with_positive_control` is unchanged (identity checks are `Copy` integers; positive-control `vec` remains).

## Compile-time callers updated

`ExecutionPermit::issue` gained a crate-private `CanonicalIncarnation` argument. Both callers are in the write set:

- `owner.rs` (`AuthorityOwner::issue_permit`)
- `permit.rs` tests (`expired_permit_is_rejected`, via `CanonicalIncarnation::fresh()`)

`MutationHandle` struct literals: only `owner.rs` `install_grant_with_ceiling`.

No caller outside the write set needed a signature update. Public `install_grant` / `issue_permit` / `validate_handle` / `revoke` / `current_grant` signatures are unchanged.

## What was not run

- `cargo test` / `cargo check` / `cargo build` (integrator owns the serial compile slot)
- git commit / push
- no synth MCP kill

Formatted only the five edited Rust files with `rustfmt --edition 2021`.
