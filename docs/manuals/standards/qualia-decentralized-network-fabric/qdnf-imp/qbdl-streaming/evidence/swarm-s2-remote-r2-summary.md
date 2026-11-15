# Wave S2 / Stage R2 — remote issuance, current grant, pre-effect recheck

Actual controller-side paired issuance of unchanged `RecipientRecord` A plus a separately versioned 256-byte decision ticket; verifier-side `PinAuthority::verify_source` + `DisclosureDecision::bind` + `AuthorityOwner::install_grant_with_ceiling`; `RemoteDisclosureGrant::recheck` immediately before wrap/send/plaintext (those callers are not in this stage).

R1 schema is frozen and was not edited. No JSON codecs. Native interchange remains CBOR/exact-length binary.

## Files written (new only)

- `crates/qualia-core-db/src/bundle/stream/policy/remote/issuer.rs`
- `crates/qualia-core-db/src/bundle/stream/policy/remote/grant.rs`
- `crates/qualia-core-db/src/bundle/stream/policy/remote/current.rs`
- `crates/qualia-core-db/src/bundle/stream/policy/remote/lifecycle_tests.rs`
- this summary

Formatted with `rustfmt --edition 2021` on the four Rust files only.

## Integrator routing (`remote/mod.rs`)

Mandatory (R2 will not compile until these exist). Do **not** otherwise edit R1 files:

```rust
mod issuer;
mod grant;
mod current;
#[cfg(test)]
mod lifecycle_tests;
```

Recommended crate-visible re-exports (keep the module private):

```rust
pub use issuer::{
    fresh_challenge, issue_disclosure, IssueBuffers, IssueRequest, IssuedLengths, ManagedSigners,
};
pub use grant::{
    install_remote_disclosure, InstallRequest, RemoteDisclosureGrant, VerifierChallengeSlot,
};
```

`recheck` / `revoke_current` / `retire_current` / `close_now` are methods on `RemoteDisclosureGrant` (defined in `current.rs`).

## Types

| Type | Role |
|---|---|
| `ManagedSigners` | Existing `CryptographicLibrary` + `SigningControl` + `PairedSigningRequest` identity. Not a second KeyManager. |
| `IssueRequest` / `IssueBuffers` / `IssuedLengths` | Controller issuance of A (1856) and ticket (256) with paired envelopes. |
| `VerifierChallengeSlot` | Capacity-1 outstanding challenge bound to transcript, request id, owner mutex pointer and incarnation. Consume once. No serialized-ticket resume. |
| `InstallRequest` | Verifier inputs. Distinct `Mutex<AuthorityOwner>` / `PinAuthority` / `CryptographicLibrary` from the controller. |
| `RemoteDisclosureGrant` | Nonconstructible current grant. Private `MutationHandle`, `AuthorityBinding`, pin receipts, decision. No public constructor. No `ExecutionPermit` escape. |

Issuance: `RecognitionAdmin` + `RecognitionHandle` against **this** `ControllerRecognition`; `snapshot` + `current_at`; `consume_decision_id`; `check_recipient_against_independent` (never A against A); ticket binds `A.id()`, transcript, request, `fresh_challenge`/`fill_os`, controller generation, policy digest, action, nbf/exp (`<= max_decision_lifetime` and `<= MAX_DECISION_LEASE_SECS` and `<= recognized.expires_unix()`), consumed decision id. Both A and ticket signed with `CryptographicLibrary::begin_paired_signing` + `cose::sign_pair`. Binding purposes: `RECIPIENT_PURPOSE` and `DISCLOSURE_DECISION_PURPOSE`. Audience is exact caller bytes. Failed issue zeros envelope scratch (`Zeroizing`); consumed decision ids are not reused and do not mint `DisclosureDecision`.

Install: `AuthoritySource::LocalPin` only (`SignedControllerChain` → `Unsupported`). Recheck receipt before install. `decode_decision_ticket` + `DisclosureDecision::bind`. Challenge consume. `AuthorityBinding` from `recognized.principal()` + `IndependentAdmission` fields (`operation` as `OperationId`). `ContactState::Active`. Finite caller ceiling. Principal ≠ recipient.

Recheck: `guard.check_affinity` then `check_handle` with retained handle + binding; `pins.recheck` on live receipts; `now < decision.expires()`; action match; `recognized.current_at`. Foreign guard / revoked / rotated / expired refuse.

Cleanup: `revoke_current(&mut CurrentAuthorityGuard)` then `retire_current` after the guard is dropped. `close_now` does both when the mutex is free. `Drop` `try_lock`s; if it cannot, the slot stays occupied (no unlocked mutation).

## Test names (`lifecycle_tests.rs`)

- `issue_and_verify_installs_grant_on_independent_verifier`
- `arbitrary_self_signer_denied`
- `right_key_wrong_role_denied`
- `a_field_substitution_denied`
- `nonce_replay_denied`
- `cross_session_ticket_denied`
- `expired_ticket_denied`
- `rotate_pin_before_install_denied`
- `revoke_before_recheck_denied`
- `shared_mutex_guard_no_deadlock`
- `eight_slot_exhaustion_and_revoke_retire_reuse`
- `drop_without_lock_does_not_leak_by_mutating`
- `policy_outcome_allow_is_not_a_grant`
- `bool_is_not_a_grant`

Focused selector after routing: `bundle::stream::policy::remote::lifecycle_tests`

Two pins are installed on each `PinAuthority` (recipient-authority and disclosure-decision). R2 signing uses `CryptographicLibrary` managed keys, not dummy `keygen_from_seed`. Controller and verifier use distinct libraries, pin tables, recognition tables, and `Mutex<AuthorityOwner>`.

## Still R3 / B2 (not this claim)

- `protection/` wrap child: `ManagedObjectPublication::wrap_for(A, out)` under the remote-grant current guard; `encode_delivery`
- Private M decoder / metadata AEAD / complete mapping before consumer access
- `PacketProtection` / `ChannelIo::with_current` delivery and send_pending app recheck
- `key_management/managed_object/received_content`
- Graph/VM policy execution (`ConstraintInstruction::lower`, `evaluate_inference_guard`), label join / `admit_egress`, restriction-bit mapping

## API gaps refused (not invented)

1. **`CurrentAuthorityGuard` has no `install_grant_with_ceiling` / `retire_revoked`.** Fields are private and there is no `DerefMut`. This worker did not edit `net/qdnf/authority/**`. Install proves the mutex is free with `CurrentAuthorityGuard::try_lock`, drops the guard, then `Mutex::try_lock` + `AuthorityOwner::install_grant_with_ceiling` (TOCTOU window). Revoke runs under the guard; retire runs only after the guard is dropped (`revoke_current` / `retire_current`). A current_guard child for install/retire remains an integrator/authority-owner claim.
2. **`PinAuthority` does not expose pin expiry.** Ticket expiry is bounded by `recognized.expires_unix()` and the 30s lease. Live pin expiry is enforced by post-sign `verify_source` / `recheck` (`current_pin`).
3. **`PolicyOutcome::Allow` and `bool` are not grants.** Refused via existing R1 functions; no `From`/`new` on `RemoteDisclosureGrant` or `DisclosureDecision`.
4. No second KeyManager, budget ledger, Poet adapter, `SignedControllerChain`, JSON, or raw master export.

## What was not run

- `cargo test` / `cargo check` / `cargo build` (integrator owns the serial compile slot)
- git commit / push
- no synth MCP kill
