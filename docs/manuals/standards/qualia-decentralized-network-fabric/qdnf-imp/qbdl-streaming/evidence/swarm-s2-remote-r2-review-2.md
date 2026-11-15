---
pass: true
open_bugs: 0
---

# Swarm S2 / Stage R2 — remote issuance/current grant re-review

Read-only source review. Production Rust was not edited. Cargo was not run here (assignment states wrap 4/4 and lifecycle_tests 22/22, including `recognition_rotate_before_recheck_denied` and `install_independent_source_not_enrolled_denied`). Author summaries were not treated as proof. Contracts: `qbs07-remote-authority-integration.md` Stage R2, `swarm-s2-remote-r1-review.md`, prior `swarm-s2-remote-r2-review.md` (`pass: false`, `open_bugs: 2`).

Reviewed:

- `crates/qualia-core-db/src/bundle/stream/policy/remote/{mod,issuer,grant,current,lifecycle_tests,recognition,policy,codec,decision,tests}.rs`
- Integrator routing `policy/mod.rs`
- Supporting owners: `net/qdnf/authority/{current_guard,owner}.rs`, `crypto/network/authority/pins.rs`, `crypto/network/object_stream/{types,recipient}.rs`, `specialized_libs/cryptographic_library/library/paired_signing.rs`

## Verdict

Stage R2 still does real paired issuance of unchanged 1856-byte `RecipientRecord` plus a separately versioned 256-byte `QBDL-PDD-V1` ticket, verifier `LocalPin` verify, `install_grant_with_ceiling` on the caller-supplied owner, and a current-guard recheck/revoke/retire/drop path.

The two fail-open bugs from `swarm-s2-remote-r2-review.md` are closed in source:

1. `join_enrollment` now calls live `ControllerRecognition::recognize` and compares pin/generation/source/source_length/policy to the handle snapshot. Both `issue_disclosure` and `install_remote_disclosure` call it after `check_recipient_against_independent`.
2. `RemoteDisclosureGrant::recheck` now takes `&ControllerRecognition`, calls `recognize(...)`, and requires live pin/generation to match the stored snapshot or `StaleGeneration`.

No remaining source bug fails fail-closed composition of this slice. Attacks 1–14 hold.

## Frozen R1 / routing

R1 schema files remain schema/reuse. `recognition.rs`, `policy.rs`, `codec.rs`, `decision.rs`, `tests.rs` were not rewritten into issuance/grant. `decision.rs:64` still has `#[allow(dead_code)]` on `bind`, which R2 calls from `grant.rs`.

`policy/mod.rs:6` has private `mod remote;`. No crate-public `pub use` of remote types from `policy/mod.rs`. Allowed.

`remote/mod.rs:8-10,32-33` routes `mod issuer; mod grant; mod current;` and `#[cfg(test)] mod lifecycle_tests;`.

`pub use issuer::issue_disclosure` is now cfg-gated (`remote/mod.rs:25-27`) together with `IssueBuffers` / `IssueRequest` / `IssuedLengths` / `ManagedSigners`. `fresh_challenge` stays ungated (`remote/mod.rs:25`); it does not depend on `CryptographicLibrary`. Prior wasm re-export residual is closed.

## Bugs

None open.

### Prior Bug 1 — closed (attack 6)

`join_enrollment` (`issuer.rs:77-104`) looks up the live row with snapshot principal plus independent source/length/policy/publisher/purpose/operation, then refuses unless live pin, controller generation, source, source_length, and policy equal the snapshot.

Issuer (`issuer.rs:160-167`) runs `check_recipient_against_independent` then `join_enrollment` before `consume_decision_id` / sign. Install (`grant.rs:187-199`) does the same after parsing A and the ticket, using `ticket.action`. The grant stores `recognized: live` (`grant.rs:288`), not the unverified caller copy.

A controller enrolled for source X / policy Y cannot sign an A for source Z: independent+A may agree with each other, but `recognize` misses the Z row or live.source disagrees with the handle snapshot (`issuer.rs:95-102`). Co-substituted independent facts plus a valid signature for the wrong enrolled source fail the same compare. Ticket policy remains `recognized.policy()` (`issuer.rs:177`); after join that equals independent.policy and A.policy.

`lifecycle_tests.rs` `enrolled_controller_cannot_issue_for_unenrolled_source` and `install_independent_source_not_enrolled_denied` exercise the enrollment disagreeing path. Naive A-only tamper remains denied by `check_recipient_against_independent`.

Publisher/purpose/operation are not re-compared to the snapshot after `recognize`. That is not a hole: `recognize` (`recognition.rs:298-311`) already requires exact publisher/purpose/operation on the unique source+policy row (`recognition.rs:199-202`), and pin/generation catch rotation of that row.

### Prior Bug 2 — closed (attack 14)

`recheck` (`current.rs:11-20,48-63`) takes live `&ControllerRecognition` and calls `recognize` with the stored snapshot’s principal/source/length/policy/publisher/purpose/operation plus `expected_action`. Pin or controller generation mismatch is `StaleGeneration`. `RecognizedController::current_at` is no longer the sole recognition check (it remains expiry-only at `recognition.rs:131-137`).

`ControllerRecognition::rotate` still replaces table pin/generation without touching `PinAuthority` (`recognition.rs:236-279`). Recheck now fails that case: pin receipts can still be live, then `recognize` returns the new pin and the compare at `current.rs:59-62` fires.

`recognition_rotate_before_recheck_denied` installs an extra pin with `audience: b"reader-rotated"` (`lifecycle_tests.rs:737-758`) so `PinAuthority::install_pin` does not `Conflict` on principal+`RECIPIENT_PURPOSE`+`b"reader"` (`pins.rs:56-61`). It then `rotate`s recognition only (`lifecycle_tests.rs:760-767`) and expects `StaleGeneration` (`lifecycle_tests.rs:769-779`). Old pin receipts remain valid; this is recognition rotate, not pin Conflict.

## Attacks (source, not summary)

1. **Controller and verifier independently owned.** Holds. `world()` (`lifecycle_tests.rs:172-239`) constructs distinct `CryptographicLibrary`, `PinAuthority`, `ControllerRecognition`, and `Mutex<AuthorityOwner>` instances. Positive path signs with `controller_lib` / `controller_pins` and installs with `verifier_pins` / `w.owner`. `controller_owner` is a separate mutex (`lifecycle_tests.rs:238,397-398`). Same public keys on both pin tables is recognition of one controller, not a shared owner.

2. **A is 1856-byte RecipientRecord; ticket is 256-byte QBDL-PDD-V1. No JSON. No Delivery/RecordHeader smuggling.** Holds. `RECIPIENT_BYTES = 1856` (`object_stream/types.rs:16`); parse is exact length (`recipient.rs:67-70`). Ticket magic `QBDL-PDD-V1\0`, `DECISION_TICKET_BYTES = 256` (`codec.rs:5-7`). No serde/json in R2 files. Ticket is a separate buffer; A is not extended. `REMOTE_AUTHORIZATION_BYTES = RECIPIENT_BYTES` (`remote/mod.rs:51`).

3. **Signing is `CryptographicLibrary::begin_paired_signing` + `cose::sign_pair`. No second KeyManager. No raw master export.** Holds. `sign_with_managed` (`issuer.rs:305-334`) builds `PairedSigningRequest` and calls those two APIs. Capability cannot export keys (`paired_signing.rs:54-56`; `ManagedPairedSigning` fields private at `managed_signing.rs:37-44`). Existing library KeyManager is reused. Failed envelopes/ticket are `fill(0)` (`issuer.rs:297-301`); sign scratch is `Zeroizing`. Binding purposes are `RECIPIENT_PURPOSE` and `DISCLOSURE_DECISION_PURPOSE`. Post-sign `verify_source` uses `AuthoritySource::LocalPin` only (`issuer.rs:242-258`). `SignedControllerChain` is `Unsupported` (`pins.rs:134`) and the positive test probes that.

4. **`check_recipient_against_independent` never checks A against A.** Holds at the helper (`decision.rs:91-114`): compares `record.fields()` to a separate `IndependentAdmission`. Production callers pass `request.independent`, not `record.fields()`. Enrollment join is now a second, distinct check (`issuer.rs:77-104`).

5. **Arbitrary self-signer denied. Right key wrong role/purpose denied.** Holds. `verify_source` requires `expected.key_ids == pin.ids` and `pin.scope.matches(purpose, audience)` (`pins.rs:138-142`). `expected_binding` refuses purpose/audience/descriptor mismatch before verify (`grant.rs:300-314`). Self-signer keys do not match the enrolled pin. Re-signing A under `DISCLOSURE_DECISION_PURPOSE` is `Unauthorized`. Same keys on the two purpose pins still cannot satisfy the wrong purpose.

6. **A field substitution (descriptor/context/source/length/operation/recipient) denied against both IndependentAdmission and the enrolled row.** Holds. A-only tamper fails `check_recipient_against_independent` and envelope `descriptor_digest != rec_id` (`grant.rs:201-217`, `recipient.rs:104-106`). Co-substituted independent + A for an unenrolled or different enrolled source fails `join_enrollment` on both issue (`issuer.rs:161-167`) and install (`grant.rs:193-199`). KEM/epoch are not in `IndependentAdmission`; unsigned substitution still changes `A.id()` and fails `expected_binding` plus ticket `recipient_record_id`.

7. **Challenge consume-once; nonce replay and cross-session ticket denied. No serialized-ticket resume.** Holds. `VerifierChallengeSlot` is capacity-1 (`grant.rs:24-40`). `issue` refuses a second Outstanding (`grant.rs:60-62`). `consume` Empty → `Unauthorized`, Consumed → `Replay`, mismatches of owner pointer / transcript / request_id / challenge → `Replay`, incarnation change → `StaleGeneration` (`grant.rs:81-106`). Consume is after paired verify+bind (`grant.rs:254-260`); failed consume does not mark Consumed. No `Clone`/`Serialize`. Fields private; `issue` always `fill_os`s a new challenge. Restart needs a new slot. `fresh_challenge` / codec refuse all-zero (`issuer.rs:107-113`, `codec.rs:59-61`).

8. **Expired ticket denied. Rotate pin before install denied. Revoke before recheck denied.** Holds. `ticket.valid_at` uses `now >= expires` (`codec.rs:76-81`); install `now = TICKET_EXP` is expired. `rotate_pin` changes generation; `current_pin`/`record` refuse the old handle (`pins.rs:81-107,177-195`). `revoke_current` calls `guard.revoke_handle` then sets `self.revoked` (`current.rs:69-81`); recheck returns `Revoked` (`current.rs:21-22`) and `check_handle` → `issue_permit` would also refuse a revoked record (`owner.rs:208-210`). Recognition rotate after install is now attack 14, not an open hole.

9. **CurrentAuthorityGuard affinity: foreign guard refuses. Shared mutex session+app check does not deadlock (one guard).** Holds in source. Recheck requires `ptr::eq(self.owner, owner)` then `guard.check_affinity(owner)` then `check_handle` (`current.rs:24-29`). `check_affinity` is pointer equality plus assigned incarnation (`current_guard.rs:22-29`). Recheck does not `try_lock`. Session+app on one guard is the `shared_mutex_guard_no_deadlock` shape; a second `CurrentAuthorityGuard::try_lock` is `WouldBlock`. No dedicated foreign-guard test.

10. **Eight-slot exhaustion and revoke/retire reuse.** Holds. `MAX_RECORDS = 8` (`owner.rs:15`). `install_grant_with_ceiling` uses `free_slot` (`owner.rs:123-152`). `close_now` revokes under a short guard then `retire_revoked` after drop (`current.rs:105-111`). `retire_revoked` writes `Record::EMPTY` without recycling `next_generation` (`owner/retirement.rs:8-16`).

11. **Drop without lock does not mutate; capacity stays occupied until explicit close.** Holds. `Drop` returns if `closed`; otherwise `CurrentAuthorityGuard::try_lock`; on failure it does nothing (`current.rs:114-128`). No unlocked `AuthorityOwner` mutation.

12. **`PolicyOutcome::Allow` and bool are not grants. `RemoteDisclosureGrant` has no public constructor.** Holds. No `From`/`Default`/`pub fn new` on `RemoteDisclosureGrant` or `DisclosureDecision`. Grant fields `pub(super)`; `MutationHandle` / `DisclosureDecision` fields are private so even `remote` siblings cannot forge a grant without `install_remote_disclosure` plus `bind`. `refuse_policy_outcome` / `refuse_bool_authority` always `Reference` (`remote/mod.rs:83-92`). `check_handle` discards `issue_permit` (`current_guard.rs:40-42`); no `ExecutionPermit` getter.

13. **`install_grant_with_ceiling` uses the verifier owner. Principal ≠ recipient.** Holds. Install uses `request.owner` (`grant.rs:274-280`). Adapter refuses `binding.principal == binding.recipient` (`grant.rs:270-272`); owner repeats that (`owner.rs:146-148`). `ceiling == 0` refused (`grant.rs:182-184`). `ContactState::Active`. TOCTOU between dropping `CurrentAuthorityGuard::try_lock` and `Mutex::try_lock` (`grant.rs:324-330`) is the documented missing current_guard child, not a false done claim.

14. **Recheck calls `guard.check_affinity` + `check_handle`, `pins.recheck`, decision expiry, action match, AND live `recognition.recognize` (not only snapshot `current_at`). Recognition rotate after install must fail recheck.** Holds. Call list is `current.rs:24-63`. Live pin/generation must match the stored snapshot. `recognition_rotate_before_recheck_denied` proves the rotate-without-`rotate_pin` case.

Two-lifetime `InstallRequest<'owner, 'tmp>` (`grant.rs:129-143`, `177-181`): the grant retains only `'owner` (`owner` mutex). Pins, recognition, and wire buffers are `'tmp` and are not stored. Install does not hold `CurrentAuthorityGuard` across tmp verification; the guard is dropped before `Mutex::try_lock` (`grant.rs:327-330`). A foreign recognition table at install is refused: `join_enrollment` compares live pin/generation to the snapshot, and `verify_source` uses `LocalPin(request.recognized.pin())` against `request.pins`. Controller snapshot + verifier pins (or the reverse) fail pin instance/generation. Recheck’s `&ControllerRecognition` is host-supplied like `&PinAuthority`; pin+generation must still match the stored live snapshot, so a controller table is refused.

## Tests (present in source)

`lifecycle_tests.rs` (17 `#[test]` functions; native-only `cfg`):

- `issue_and_verify_installs_grant_on_independent_verifier`
- `arbitrary_self_signer_denied`
- `right_key_wrong_role_denied`
- `a_field_substitution_denied`
- `nonce_replay_denied`
- `cross_session_ticket_denied`
- `expired_ticket_denied`
- `enrolled_controller_cannot_issue_for_unenrolled_source` (prior Bug 1, issue)
- `install_independent_source_not_enrolled_denied` (prior Bug 1, install)
- `recognition_rotate_before_recheck_denied` (prior Bug 2; extra pin audience `reader-rotated`)
- `rotate_pin_before_install_denied`
- `revoke_before_recheck_denied`
- `shared_mutex_guard_no_deadlock`
- `eight_slot_exhaustion_and_revoke_retire_reuse`
- `drop_without_lock_does_not_leak_by_mutating`
- `policy_outcome_allow_is_not_a_grant`
- `bool_is_not_a_grant`

R1 `tests.rs` still has the eleven schema tests. This reviewer did not execute cargo. Assignment states wrap 4/4 and lifecycle_tests 22/22 including the two named closures; source of `lifecycle_tests.rs` contains the 17 names above.

Attack 9 has shared-mutex coverage, not a foreign-guard case. Attack 6 now has both A-only tamper and enrollment-disagreeing independent+A tests.

## Residuals (not this slice’s bugs)

- B2 `wrap_for` / private M / `PacketProtection` / `ChannelIo::with_current` / received-content: not claimed.
- `CurrentAuthorityGuard` still has no `install_grant_with_ceiling` / `retire_revoked`; install TOCTOU window is disclosed.
- Pin expiry is via `current_pin` / `recheck`, not a pin-expiry field on the ticket.
- Recheck does not re-parse A; A validity after install is an R3 wrap concern.
- `IndependentAdmission` still omits KEM/epoch; unsigned tamper is covered by `A.id()`.
- Recheck’s recognition table is a caller argument (forced by `'tmp`). A second host-built table that re-enrolls the same pin at the same generation would satisfy pin/generation; that is local-admin equivalent to not rotating, not a remote fail-open. `AuthorityReceipt` does not expose `PinHandle`.
- Graph/VM policy execution, restriction-bit mapping, label join: R3.

## Author claims vs source

- `issuer.rs` `join_enrollment` calls live `recognize` and compares pin/generation/source/source_length/policy: holds (`issuer.rs:77-104`).
- `grant.rs` `install_remote_disclosure` also calls `join_enrollment` after parsing A: holds (`grant.rs:193-199`).
- `InstallRequest` is two-lifetime `'owner, 'tmp` and carries `recognition: &'tmp ControllerRecognition`: holds (`grant.rs:129-132`). Grant does not retain tmp borrows or extend the owner lock across them.
- `current.rs` `recheck` takes `&ControllerRecognition` and requires live pin/generation match or `StaleGeneration`: holds (`current.rs:11-16,48-63`).
- `recognition_rotate_before_recheck_denied` extra pin uses `audience: b"reader-rotated"` and still proves recognition rotate (receipts stay live; pin/generation compare fails): holds (`lifecycle_tests.rs:737-779`).
- `remote/mod.rs` wasm-gates `issue_disclosure` re-export: holds (`remote/mod.rs:26-27`).
- Paired managed signing of A + ticket, `LocalPin` only, consume-once challenge, principal ≠ recipient, no public grant constructor, drop-without-lock occupancy: hold.
- wrap_for / private M / PacketProtection: correctly not claimed.
