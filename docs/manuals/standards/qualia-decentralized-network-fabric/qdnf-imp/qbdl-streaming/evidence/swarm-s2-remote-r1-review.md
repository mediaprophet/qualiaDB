open_bugs: 0
pass: true

# Swarm S2 / child B — remote authority Stage R1 independent review

Read-only source review of Wave S2 B remote-auth Stage R1. Production Rust was not edited. Cargo was not run. Author summary (`swarm-s2-remote-r1-summary.md`) was not treated as proof. Stage R1 contract is `qbs07-remote-authority-integration.md` (schema/reuse only).

Reviewed:

- `crates/qualia-core-db/src/bundle/stream/policy/remote/{mod,recognition,policy,codec,decision,tests}.rs`
- `crates/qualia-core-db/src/bundle/stream/policy/mod.rs`
- supporting types: `policy/types.rs`, `crypto/network/errors.rs`, `crypto/network/authority/types.rs`, `crypto/network/object_stream/{types,recipient}.rs`, `net/qdnf/types.rs`

## Review

Stage R1 is schema/reuse: purpose-scoped recognition, whole-source disclosure policy artifact, RecipientRecord reuse (1856), separately versioned decision-ticket codec, private nonconstructible `DisclosureDecision`. No paired signing, grant install, or B2 wrap is present, and R1 does not claim those.

`policy/mod.rs` does not declare `mod remote` and does not re-export any remote type. `bundle/stream/mod.rs` keeps `pub mod policy` but the remote child is unrouted. That is not a public crate API. Integrator may add private `mod remote;`.

## Attacks (source, not summary)

1. **Recognition: capacity 8, no eviction, default deny, no source wildcard, no keys in the table.** Holds.
   - Capacity is `MAX_RECOGNIZED = 8` (`recognition.rs:10`). `install` takes the first `None` slot and returns `PolicyError::Capacity` when full (`recognition.rs:204-208`). No row is overwritten except explicit `rotate` of that handle. Expired rows still occupy slots.
   - `recognize` default-denies: empty/mismatch is `PolicyError::Reference` (`recognition.rs:298-308`). Match is exact `principal`/`source`/`source_length`/`policy`, then exact `publisher`/`purpose`/`operation`, then `allowed_ops.contains(action)`, then expiry.
   - Zero source, zero length, zero principal, and zero policy are refused at recognize (`recognition.rs:295-297`) and at enroll (`recognition.rs:387-395`). There is no glob, `ANY`, or all-sources path. Duplicate `source+policy` is `CryptoError::Conflict` (`recognition.rs:199-202`).
   - Table records store `PinHandle` plus digests/metadata (`recognition.rs:141-157`). `PinHandle` is instance/slot/generation only (`authority/types.rs:15-19`). No ML-DSA, ML-KEM, Ed25519, or secret arrays. Tests mint pins only to obtain an otherwise-unconstructible handle.

2. **Install requires instance-bound `RecognitionAdmin`; wire role flag cannot install.** Holds.
   - `new_local_admin` fills a 16-byte instance from `fill_os` and shares it with `RecognitionAdmin` (`recognition.rs:168-181`). `install`/`rotate`/`consume_decision_id` call `admin()` which requires `admin.instance == self.instance` else `Unauthorized` (`recognition.rs:184-188,197,243,326`).
   - `RecognitionAdmin` has a private field and no public constructor, `From<u8>`, or wire decoder. Cross-table admin cannot install.
   - `refuse_wire_role_install` always returns `PolicyError::Crypto(Unauthorized)` (`remote/mod.rs:42-44`). No `install` overload takes a role flag.

3. **Policy artifact requires whole coverage (header/index/gaps/entries). Missing any bit denied.** Holds.
   - `SourceCoverage::WHOLE = HEADER|INDEX|GAPS|ENTRIES` (`policy.rs:21-25`). `admit` requires exact equality (`policy.rs:39-45`), so missing or extra bits are `PolicyError::Graph`.
   - Encode runs `fields_valid` → `coverage.admit()` (`policy.rs:140,257-258`). Decode admits the coverage byte before constructing the view (`policy.rs:179`). Trailing/truncated policy is exact-length `Encoding` (`policy.rs:196-198`).

4. **Decision ticket is separate from RecipientRecord (1856). Truncated/trailing/wrong-domain refuse. Lease <= 30s.** Holds.
   - Ticket is `DECISION_TICKET_BYTES = 256`, magic `QBDL-PDD-V1\0` (`codec.rs:5-7`). RecipientRecord remains `RECIPIENT_BYTES = 1856`, magic `QBDL-PRA-V1\0`, purpose `qbdl-stream-protected-v1/recipient-authority`. Decision purpose is only `DISCLOSURE_DECISION_PURPOSE` in `remote/mod.rs:37`. Delivery/RecordHeader are untouched.
   - Decode requires exact 256 bytes (`codec.rs:121-126`), matching magic (`codec.rs:127-128`), version 1, flags 0, reserved 7+8 zero (`codec.rs:154-166`). Shorter, longer, or corrupted domain is `Encoding`.
   - `expires - not_before` must be `<= MAX_DECISION_LEASE_SECS` (30) (`codec.rs:66-72`, `remote/mod.rs:38`). Enrollment `max_decision_lifetime` is the same ceiling (`recognition.rs:399-401`). `ticket_package_len` caps ticket+envelope at 8192, matching `MAX_PRIVATE_PACKAGE`.

5. **`DisclosureDecision` has no public constructor; `PolicyOutcome::Allow` and bool are not authority.** Holds.
   - Fields are private (`decision.rs:29-36`). Only mint is `pub(super) fn bind` (`decision.rs:65-88`). No `pub fn new`, `Default`, or `From`. Type is not `Copy`/`Clone`.
   - `refuse_policy_outcome` / `DisclosureDecision::refuse_policy_outcome` ignore the enum and return `Reference` (`remote/mod.rs:72-75`). `refuse_bool_authority` likewise (`remote/mod.rs:78-80`). No conversion into `DisclosureDecision`.

6. **`VerifiedLabel` / `ConsentLedger` / `SignedSensingScope` refused as remote authority.** Holds.
   - `refuse_verified_label_as_issuer` always `Reference` (`remote/mod.rs:57-59`). `refuse_consent_ledger_as_pq_source_auth` always `Unsupported` (`remote/mod.rs:62-64`). `refuse_signed_sensing_scope_as_whole_source` always `Unsupported` (`remote/mod.rs:67-69`). Delegation chains are `Unsupported` (`remote/mod.rs:47-49`). All-subjects-consented bool is `Reference` (`remote/mod.rs:52-54`). No recognize/bind/decode path accepts those types.

7. **Rotation invalidates old handles and decision ids.** Holds on the live table.
   - `rotate` requires a new `PinHandle` (`recognition.rs:252-254`), advances generation (`recognition.rs:255`), and sets `decision_id_floor`/`next_decision_id` to `old.next_decision_id` (`recognition.rs:268-269`). Old handle fails `record()` with `StaleGeneration` (`recognition.rs:346-348`).
   - `bind` refuses `ticket.decision_id < recognized.decision_id_floor()` and mismatched controller generation (`decision.rs:72-79`). Previously consumed ids are below the new floor.

8. **No PinAuthority signing, no `AuthorityOwner::install_grant`, no B2 wrap, no R2 files.** Holds.
   - Remote directory is only `{mod,recognition,policy,codec,decision,tests}.rs`. No `issuer.rs`, `grant.rs`, `current.rs`, or `lifecycle_tests.rs`.
   - Production remote code does not call `verify_source`, `sign_*`, `install_grant`, `install_grant_with_ceiling`, `wrap_for`, or `CurrentAuthorityGuard`. Tests construct `PinAuthority` solely to obtain `PinHandle` values.

9. **RecipientRecord fields checked against `IndependentAdmission`, not against A itself.** Holds for the R1 helper.
   - `check_recipient_against_independent` compares `record.fields()` to a caller-supplied `expected` (`decision.rs:91-114`). It never copies A into expected and never compares a field to itself.
   - Identity fields covered: descriptor, context, recipient, publisher, service, purpose, instrument, scope, operation, closure, policy, consumer, source, source_length. Ticket `recipient_record_id` is `A.id()` = SHA-384 of the full 1856-byte record (`codec.rs:40,104`), so KEM/epoch/validity bytes are committed at ticket bind even though they are not duplicated in `IndependentAdmission`.

10. **Compile hazards: `Generation::next()` is `Result<_, QdnfError>`; `PolicyError::Crypto` takes `CryptoError`.** No mismatch in the R1 sources.
    - `Generation::next` returns `Result<Self, QdnfError>` (`net/qdnf/types.rs:145-150`).
    - `PolicyError::Crypto(CryptoError)` and `PolicyError::Authority(QdnfError)` (`policy/types.rs:27-28`). `CryptoError` is `pub use QdnfError as CryptoError` (`crypto/network/errors.rs:3`), so both variants type-check against `QdnfError`.
    - `install`/`rotate` use `self.next_generation.next().map_err(E::Authority)` (`recognition.rs:209,255`). `fill_os` correctly uses `map_err(E::Crypto)` (`recognition.rs:170`). No `map_err(E::Crypto)` on `Generation::next`.

## Residuals (allowed; not R1 bugs)

- Integrator has not yet added `mod remote;` in `policy/mod.rs`. Files are unrouted until that private module line exists. They must not be `pub mod remote` or crate-public `pub use`.
- `DisclosureDecision::bind` is schema/recognition binding on a `Copy` `RecognizedController` snapshot. It does not re-query the live table. R2 must recheck live recognition and pin receipts before wrap/send/consumer.
- `IndependentAdmission` omits KEM public keys, key epoch, and A validity. Live comparison of those against session/recipient/clock is R2/R3. Ticket hash already covers the 1856-byte encoding.
- No explicit retire/uninstall. Exhausted eight source/policy rows stay occupied (no implicit eviction). R2 lifecycle owns retire.
- R2 signing, current grant, `AuthorityOwner::install_grant_with_ceiling`, and B2 wrap are absent. R1 does not claim them.

## Issues
