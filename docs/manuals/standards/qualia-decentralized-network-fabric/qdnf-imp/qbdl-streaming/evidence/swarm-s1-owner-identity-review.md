open_bugs: 0
pass: true

# Swarm S1 — AuthorityOwner identity independent review

Read-only source review of the reconstructed-owner attack: replacing `AuthorityOwner` inside the same `Mutex` with matching records (same slot / generation / principal) must not let old handles or permits issue, revoke, validate, retire, or `check_handle`. Cargo was not run. Author summary was not treated as proof.

## Review

The live owner identity is `CanonicalIncarnation`, a crate-private process-local nonce. Handle and permit operations compare that value, not mutex address and not grant generation alone. Reconstructing equal records in the same mutex yields `QdnfError::StaleGeneration`. Moved original values keep the same nonce and remain valid. No identity-defeating bug remains in the reviewed files.

Checklist against the requested attack (source, not summary):

1. **`const new` stays `UNASSIGNED`.** `AuthorityOwner::new` is `const` and stores `CanonicalIncarnation::UNASSIGNED` (`owner.rs:89-94`). `Default` delegates to `new`. `CanonicalIncarnation::assigned` is `self.0 != 0` (`incarnation.rs:8-11`).
2. **Bind only after fallible admission; failed admission does not bind.** Zero digests, self-contact, zero ceiling, expiry, inactive contact, `free_slot`, candidate `next_generation.next()`, `VerifiedCredential::new`, and `AuthorisedContact::new` all return before `bind_incarnation` (`owner.rs:131-164`). After bind, only Copy struct writes remain. `failed_admission_does_not_assign_identity` asserts unassigned after those refusals, then one successful install binds once and later installs reuse it (`owner_tests.rs:277-313`).
3. **`next_generation.next()` before bind is not a stored bump.** `Generation::next` is `const fn next(self)` and does not mutate (`types.rs:145-150`). The owner field is published only at `self.next_generation = generation` after bind and record write (`owner.rs:186-187`). If bind fails, stored generation, slots, and identity stay unchanged. That is the existing compute-then-publish pattern, not a generation skip and not an identity bypass.
4. **Moved original owner remains valid.** `AuthorityOwner` is not `Copy`/`Clone`. Move copies the private nonce. `moved_original_owner_preserves_handle_and_permit` still validates, issues, and matches (`owner_tests.rs:260-274`). Restore-after-replace uses the same move (`owner_tests.rs:223-231`).
5. **Foreign equal-generation permit is rejected.** Two owners with colliding slot/generation/principal get distinct incarnations. Cross `matches_permit` / `matches_handle` is `StaleGeneration`; same-owner pairs succeed (`owner_tests.rs:235-257`). `matches_permit` requires both incarnations assigned and equal, then generation and principal (`owner.rs:63-76`).
6. **`check_affinity` is mutex `ptr::eq` and assigned identity; `check_handle` still uses `record()`.** Foreign mutex is `Unauthorized`. Unassigned occupant is `StaleGeneration` (`current_guard.rs:22-29`). `check_handle` then calls `issue_permit` → `record()` → `require_identity` (`current_guard.rs:31-42`, `owner.rs:207,268-269,322-326`). Pointer affinity alone cannot accept a reconstructed occupant.
7. **`ExecutionPermit::issue` is crate-private; identity is not a public constructor.** Fields are private (`permit.rs:13-24`). `issue` is `pub(crate)` and rejects unassigned incarnation (`permit.rs:28-56`). `incarnation()` is `pub(crate)`. No public `new`/`Default`. Outside this crate a permit cannot be minted with a chosen identity.
8. **`CanonicalIncarnation` is not exported as a new public authority type.** It remains `pub(crate)` in `net/peer/runtime/incarnation.rs`. `runtime/mod.rs` does not re-export it. `authority/mod.rs` still exports only `AuthorityOwner`, `MutationHandle`, `ExecutionPermit`, and related grant types. S1 added no public identity type.
9. **`retire_revoked` still goes through `record_mut` identity check.** First statement is `self.record_mut(handle)?` (`owner/retirement.rs:9`). `record_mut` calls `require_identity` (`owner.rs:287-288`). Same-owner slot reuse after retire fails on generation (`retirement_tests.rs:40-45`). Reconstructed occupant fails identity on `revoke` (`owner_tests.rs:219-221`).
10. **Tests actually replace mutex contents.** `std::mem::replace` on `&mut *mutex.lock()` with a second bound owner, then old-handle `issue_permit` / `validate_handle` / `current_grant` / `revoke` / `check_handle` (`owner_tests.rs:191-232`, `current_guard/tests.rs:75-95`). `owner_tests` asserts slot/generation/principal collide and incarnation differs (`owner_tests.rs:205-208`). Failure code is `StaleGeneration`, not mutex `Unauthorized`, so the proof is identity, not pointer inequality.

`require_identity` rejects unassigned even when both sides are `UNASSIGNED` (`owner.rs:322-326`). `issue_permit` stamps `self.existing_incarnation()` only after `record()` has required handle identity (`owner.rs:242`). Copied `ExecutionPermit` remains a documented historical snapshot (`owner/retirement.rs:6-8`); live effects that re-issue from the concrete owner refuse the reconstructed occupant.

## Issues
