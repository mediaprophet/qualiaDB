---
pass: false
open_bugs: 2
---

# Swarm S2 / Stage R2 — remote issuance/current grant independent review

Read-only source review. Production Rust was not edited. Cargo was not run. Author summary (`swarm-s2-remote-r2-summary.md`) was not treated as proof. Contracts: `qbs07-remote-authority-integration.md` Stage R2, `swarm-s2-remote-r1-summary.md`, `swarm-s2-remote-r1-review.md`.

Reviewed:

- NEW `crates/qualia-core-db/src/bundle/stream/policy/remote/{issuer,grant,current,lifecycle_tests}.rs`
- Integrator routing `remote/mod.rs` and `policy/mod.rs`
- Frozen R1 `recognition.rs`, `policy.rs`, `codec.rs`, `decision.rs`, `tests.rs`
- Supporting owners: `net/qdnf/authority/{current_guard,owner}.rs`, `crypto/network/authority/pins.rs`, `crypto/network/object_stream/{types,recipient}.rs`, `specialized_libs/cryptographic_library/library/paired_signing.rs`

## Verdict

Stage R2 does real paired issuance of unchanged 1856-byte `RecipientRecord` plus a separately versioned 256-byte `QBDL-PDD-V1` ticket, verifier `LocalPin` verify, `install_grant_with_ceiling` on the caller-supplied owner, and a current-guard recheck/revoke/retire/drop path. Several required attacks hold in source.

Two source bugs fail the fail-closed composition R1 assigned to R2:

1. A / `IndependentAdmission` are never joined to the enrolled `RecognizedController` source/policy/publisher/purpose/operation.
2. `RemoteDisclosureGrant::recheck` trusts a copied `RecognizedController`; `current_at` is expiry-only and there is no live `ControllerRecognition` query.

`open_bugs: 0` is refused. Tests exist for most named cases but were not executed here.

## Frozen R1 / routing

Entire `policy/remote/` is untracked (`git status` `??`), so freeze versus a committed R1 tree cannot be proven. Inspection: `recognition.rs`, `policy.rs`, `codec.rs`, `decision.rs`, `tests.rs` still contain only R1 schema/reuse. `decision.rs:64` still has `#[allow(dead_code)]` on `bind`, which R2 now calls from `grant.rs` — evidence those R1 files were not rewritten.

`policy/mod.rs:5` now has the mandatory `mod remote;`. `remote/mod.rs:8-10,32-33` adds `mod issuer; mod grant; mod current;` and `#[cfg(test)] mod lifecycle_tests;`. Recommended crate-visible `pub(crate) use remote::{...}` in `policy/mod.rs` is still absent; the child stays private. That is allowed.

`pub use issuer::issue_disclosure` (`remote/mod.rs:25-27`) is not cfg-gated, while `issue_disclosure` itself is `#[cfg(not(target_arch = "wasm32"))]` (`issuer.rs:83-84`). Native is the R2 test target; wasm32 compile of this re-export is a residual integrator hazard, not one of the 14 attacks.

## Bugs

### Bug 1 — A is not checked against the enrolled source/policy (attack 6)

`issue_disclosure` snapshots the live row, then only checks A against caller-supplied `IndependentAdmission`:

```102:123:crates/qualia-core-db/src/bundle/stream/policy/remote/issuer.rs
    let recognized = recognition.snapshot(request.handle)?;
    recognized.current_at(now)?;
    // ...
    check_recipient_against_independent(&record, &request.independent)?;
```

Neither `issuer.rs` nor `grant.rs` reads `recognized.source()`, `source_length()`, `publisher()`, `purpose()`, or `operation()`. Ticket policy is taken from the enrollment (`issuer.rs:133`) while A.policy may differ. Install repeats the same split: A vs `independent` (`grant.rs:185`), ticket vs snapshot via `DisclosureDecision::bind` (`grant.rs:245`, `decision.rs:72-79` — policy/generation/floor/ops/lease/expiry only), then mixes `recognized.principal()` with `independent` recipient/instrument/scope/purpose/operation into `AuthorityBinding` (`grant.rs:253-260`).

`ControllerRecognition::recognize` (`recognition.rs:283-319`) is the live exact-source join. R2 never calls it. `lifecycle_tests.rs` `a_field_substitution_denied` only tampers A while leaving `independent()` at the original enrollment values, so it does not exercise this hole.

Consequence: a controller enrolled for source X / policy Y can sign an A for source Z. If the verifier passes an `IndependentAdmission` that matches that A, install succeeds and the current grant is minted for Z’s recipient/purpose/operation under Y’s controller principal. Tampering A while holding independent facts fixed is denied (`decision.rs:91-114` plus `A.id()` in the ticket). Fail-closed whole-source enrollment is not.

### Bug 2 — recheck trusts a copied `RecognizedController` (attack 14)

R1 residual, assigned to R2: do not trust a copied `RecognizedController` across rotation; re-query the live table and PinAuthority.

`RemoteDisclosureGrant` stores `recognized: RecognizedController` (`grant.rs:150`) from `InstallRequest.recognized` (`grant.rs:278`). `InstallRequest` has no `ControllerRecognition`. Recheck ends with:

```47:48:crates/qualia-core-db/src/bundle/stream/policy/remote/current.rs
        self.recognized.current_at(now)?;
        Ok(())
```

`RecognizedController::current_at` is expiry-only (`recognition.rs:131-137`). It does not consult table generation, `decision_id_floor`, pin handle identity, or `recognize()`. Pin receipts are rechecked live (`current.rs:34-38`) and catch `PinAuthority::rotate_pin` / revoke. `ControllerRecognition::rotate` (`recognition.rs:236-279`) replaces the table pin/generation without touching PinAuthority; an old snapshot plus a still-live old pin passes recheck.

`rotate_pin_before_install_denied` only rotates the verifier pin before install. There is no recognition-rotate-before-recheck test.

## Attacks (source, not summary)

1. **Controller and verifier independently owned.** Holds. `world()` (`lifecycle_tests.rs:172-239`) constructs distinct `CryptographicLibrary`, `PinAuthority`, `ControllerRecognition`, and `Mutex<AuthorityOwner>` instances. Positive path signs with `controller_lib` / `controller_pins` and installs with `verifier_pins` / `w.owner`. `controller_owner` is a separate mutex (`lifecycle_tests.rs:397`). Same public keys on both pin tables is recognition of one controller, not a shared owner. Not a helper that pokes a grant into existence.

2. **A is 1856-byte RecipientRecord; ticket is 256-byte QBDL-PDD-V1. No JSON. No Delivery/RecordHeader smuggling.** Holds. `RECIPIENT_BYTES = 1856` (`object_stream/types.rs:16`), parse exact length (`recipient.rs:67-70`). Ticket magic `QBDL-PDD-V1\0`, `DECISION_TICKET_BYTES = 256` (`codec.rs:5-7`). No serde/json in R2 files. Ticket is a separate buffer; A is not extended. `REMOTE_AUTHORIZATION_BYTES = RECIPIENT_BYTES` (`remote/mod.rs:51`).

3. **Signing is `CryptographicLibrary::begin_paired_signing` + `cose::sign_pair`. No second KeyManager. No raw master export.** Holds. `sign_with_managed` (`issuer.rs:261-289`) builds `PairedSigningRequest` and calls those two APIs. Capability is documented as non-exporting (`paired_signing.rs:54-56`; `ManagedPairedSigning` fields private). Existing library KeyManager is reused, not a second store. Failed envelopes/ticket are `fill(0)` (`issuer.rs:253-257`); sign scratch is `Zeroizing`. Binding purposes are `RECIPIENT_PURPOSE` and `DISCLOSURE_DECISION_PURPOSE`. Post-sign `verify_source` uses `AuthoritySource::LocalPin` only (`issuer.rs:198-214`). `SignedControllerChain` is `Unsupported` in `pins.rs:134` and the positive test probes that.

4. **`check_recipient_against_independent` never checks A against A.** Holds at the helper (`decision.rs:91-114`): compares `record.fields()` to a separate `IndependentAdmission`. Production callers pass `request.independent`, not `record.fields()`. The enrollment-join failure is Bug 1, not this helper comparing A to itself.

5. **Arbitrary self-signer denied. Right key wrong role/purpose denied.** Holds. `verify_source` requires `expected.key_ids == pin.ids` and `pin.scope.matches(purpose, audience)` (`pins.rs:138-142`). `expected_binding` refuses purpose/audience/descriptor mismatch before verify (`grant.rs:291-305`). Self-signer keys do not match the enrolled pin. Re-signing A under `DISCLOSURE_DECISION_PURPOSE` is `Unauthorized`. Same keys on the two purpose pins still cannot satisfy the wrong purpose.

6. **A field substitution (D/Ctx/source/length/operation/recipient) denied.** Fails closed only against caller-supplied `IndependentAdmission` and against unsigned `A.id()` mismatch. Does **not** hold against the enrolled row (Bug 1). Naive A-only tamper is denied by `check_recipient_against_independent` and by envelope `descriptor_digest != rec_id`. KEM/epoch are not in `IndependentAdmission`; unsigned substitution still changes `A.id()` (`recipient.rs:104-106`) and fails `expected_binding`. Co-substituted independent facts plus a valid controller signature for the wrong source are accepted.

7. **Challenge consume-once; nonce replay and cross-session ticket denied. No serialized-ticket resume.** Holds. `VerifierChallengeSlot` is capacity-1 (`grant.rs:24-40`). `issue` refuses a second Outstanding (`grant.rs:60-62`). `consume` Empty → `Unauthorized`, Consumed → `Replay`, and mismatches of owner pointer / transcript / request_id / challenge → `Replay`, incarnation change → `StaleGeneration` (`grant.rs:81-106`). Consume is after paired verify+bind (`grant.rs:245-251`); failed consume does not mark Consumed. No `Clone`/`Serialize`. Fields private; `issue` always `fill_os`s a new challenge. Restart needs a new slot. `fresh_challenge` / codec refuse all-zero (`issuer.rs:70-76`, `codec.rs:59-61`).

8. **Expired ticket denied. Rotate pin before install denied. Revoke before recheck denied.** Holds for those three paths. `ticket.valid_at` uses `now >= expires` (`codec.rs:76-81`); install `now = TICKET_EXP` is expired. `rotate_pin` changes generation; `current_pin`/`record` refuse the old handle (`pins.rs:81-107,177-195`). `revoke_current` calls `guard.revoke_handle` then sets `self.revoked` (`current.rs:53-65`); recheck returns `Revoked` (`current.rs:20-22`) and `check_handle` → `issue_permit` would also refuse a revoked record (`owner.rs:208-210`). Recognition rotate after install is Bug 2, not this pin/revoke trio.

9. **CurrentAuthorityGuard affinity: foreign guard refuses. Shared mutex session+app check does not deadlock (one guard).** Holds in source. Recheck requires `ptr::eq(self.owner, owner)` then `guard.check_affinity(owner)` then `check_handle` (`current.rs:23-29`). `check_affinity` is pointer equality plus assigned incarnation (`current_guard.rs:22-29`). Recheck does not `try_lock`. Session+app on one guard is the `shared_mutex_guard_no_deadlock` shape; a second `CurrentAuthorityGuard::try_lock` is `WouldBlock`. No dedicated foreign-guard test (unexecuted anyway).

10. **Eight-slot exhaustion and revoke/retire reuse.** Holds. `MAX_RECORDS = 8` (`owner.rs:15`). `install_grant_with_ceiling` uses `free_slot` (`owner.rs:123-152`). `close_now` revokes under a short guard then `retire_revoked` after drop (`current.rs:88-95`). `retire_revoked` writes `Record::EMPTY` without recycling `next_generation` (`owner/retirement.rs:8-16`).

11. **Drop without lock does not mutate; capacity stays occupied until explicit close.** Holds. `Drop` returns if `closed`; otherwise `CurrentAuthorityGuard::try_lock`; on failure it does nothing (`current.rs:98-112`). No unlocked `AuthorityOwner` mutation.

12. **`PolicyOutcome::Allow` and bool are not grants. `RemoteDisclosureGrant` has no public constructor.** Holds. No `From`/`Default`/`pub fn new` on `RemoteDisclosureGrant` or `DisclosureDecision`. Fields `pub(super)`; `MutationHandle` / `DisclosureDecision` fields are private so even `remote` siblings cannot forge a grant without `install_remote_disclosure`. `refuse_policy_outcome` / `refuse_bool_authority` always `Reference` (`remote/mod.rs:83-92`). `check_handle` discards `issue_permit` (`current_guard.rs:40-42`); no `ExecutionPermit` getter.

13. **`install_grant_with_ceiling` uses the verifier owner. Principal ≠ recipient.** Holds. Install uses `request.owner` (`grant.rs:265-271`). Adapter refuses `binding.principal == binding.recipient` (`grant.rs:261-263`); owner repeats that (`owner.rs:146-148`). `ceiling == 0` refused (`grant.rs:180-182`). `ContactState::Active`. TOCTOU between dropping `CurrentAuthorityGuard::try_lock` and `Mutex::try_lock` (`grant.rs:315-325`) is the documented missing current_guard child, not a false done claim.

14. **Recheck calls `guard.check_affinity` + `check_handle`, `pins.recheck`, decision expiry, action match, `recognized.current_at`.** Call list is present (`current.rs:26-47`). The live-recognition property fails (Bug 2). Pin receipts are live; recognition is not.

## Tests (present, not executed)

`lifecycle_tests.rs` names match the summary. Cargo was not run. Distinguish:

- Source-holding attacks 1–5, 7–13 have corresponding tests; results unverified.
- Attack 6 test does not cover enrollment-disagreeing A (Bug 1).
- Attack 9 has shared-mutex coverage, not a foreign-guard case.
- Attack 14 test coverage is pin-rotate-before-install and revoke-before-recheck, not recognition-rotate-before-recheck.

## Residuals (not this slice’s bugs)

- B2 `wrap_for` / private M / `PacketProtection` / `ChannelIo::with_current` / received-content: not claimed.
- `CurrentAuthorityGuard` still has no `install_grant_with_ceiling` / `retire_revoked`; install TOCTOU window is disclosed.
- Pin expiry is via `current_pin` / `recheck`, not a pin-expiry field on the ticket.
- Recheck does not re-parse A; A validity after install is an R3 wrap concern.
- `IndependentAdmission` still omits KEM/epoch; unsigned tamper is covered by `A.id()`.
- Wasm re-export of cfg-gated `issue_disclosure` (`remote/mod.rs:25-27`).
- Graph/VM policy execution, restriction-bit mapping, label join: R3.

## Author claims vs source

- Paired managed signing of A + ticket, `LocalPin` only, consume-once challenge, principal ≠ recipient, no public grant constructor, drop-without-lock occupancy: hold.
- “Recheck live recognition … do not trust a copied `RecognizedController` snapshot”: does not hold (Bug 2). Author listed `recognized.current_at` as if it were that check.
- “Check every A field against independently selected admission/session/source state”: holds vs `IndependentAdmission`, not vs the enrollment the adapter already holds (Bug 1).
- wrap_for / private M / PacketProtection: correctly not claimed.
