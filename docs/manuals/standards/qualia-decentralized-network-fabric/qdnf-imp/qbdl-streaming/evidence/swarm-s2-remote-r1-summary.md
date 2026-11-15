# Wave S2 / child B — remote authority Stage R1

Purpose-scoped recognition table, whole-source disclosure policy artifact, RecipientRecord reuse (1856-byte payload), separately versioned decision-ticket codec, and private nonconstructible `DisclosureDecision`.

No crypto primitives, no PinAuthority signing, no `AuthorityOwner::install_grant`, no B2 wrap, no R2 issuer/grant/current files. `policy/mod.rs` was not edited.

## Files written (new only)

- `crates/qualia-core-db/src/bundle/stream/policy/remote/mod.rs`
- `crates/qualia-core-db/src/bundle/stream/policy/remote/recognition.rs`
- `crates/qualia-core-db/src/bundle/stream/policy/remote/policy.rs`
- `crates/qualia-core-db/src/bundle/stream/policy/remote/codec.rs`
- `crates/qualia-core-db/src/bundle/stream/policy/remote/decision.rs`
- `crates/qualia-core-db/src/bundle/stream/policy/remote/tests.rs`
- this summary

Formatted with `rustfmt --edition 2021` on the six Rust files only.

## Integrator routing (`policy/mod.rs`)

Mandatory (tests will not compile until this exists):

```rust
mod remote;
```

Recommended crate-visible re-exports (keep the module private; do not make remote a public crate API yet):

```rust
pub(crate) use remote::{
    AllowedOps, ClosureBudget, ControllerRecognition, DecisionTicketFields, DisclosureAction,
    DisclosureDecision, DisclosurePolicyFields, IndependentAdmission, RecognitionAdmin,
    RecognitionEnrollment, RecognitionHandle, RecognizedController, SourceCoverage,
    DECISION_TICKET_BYTES, DISCLOSURE_DECISION_PURPOSE, MAX_DECISION_LEASE_SECS,
    MAX_DISCLOSURE_POLICY_BYTES, MAX_RECOGNIZED, MAX_TICKET_PACKAGE_BYTES,
    REMOTE_AUTHORIZATION_BYTES, bind_policy_to_closure, check_recipient_against_independent,
    decode_decision_ticket, decode_disclosure_policy, encode_decision_ticket_into,
    encode_disclosure_policy_into, refuse_all_subjects_consented_flag, refuse_bool_authority,
    refuse_consent_ledger_as_pq_source_auth, refuse_delegation_chain, refuse_policy_outcome,
    refuse_signed_sensing_scope_as_whole_source, refuse_verified_label_as_issuer,
    refuse_wire_role_install, ticket_package_len,
};
```

Do **not** re-export `RecipientRecord` / `RECIPIENT_PURPOSE` from `policy/mod.rs`. Those stay owned by `crypto/network/object_stream/types.rs`. Remote reuses them in place (`RECIPIENT_BYTES == 1856`). Decision purpose is defined only in `remote/`:

`DISCLOSURE_DECISION_PURPOSE = b"qbdl-stream-protected-v1/disclosure-decision"`

## R1 contracts implemented

- Recognition: capacity 8, no implicit eviction, default deny, no source wildcard, no transitive delegation. Stores PinHandle + principal + source SHA384/length + policy digest + publisher + purpose + ops + expiry + max decision lifetime. No keys/signatures/vault.
- Install requires `RecognitionAdmin` from `ControllerRecognition::new_local_admin()` (instance-bound, analogous to `LocalBootstrap`). `refuse_wire_role_install` always denies a wire role flag.
- Policy artifact `QBDL-WSD-POL-V1` counted in the existing 1 MiB / 16 node / 256 edge budget. Whole coverage only (header | original index | gaps | every entry). One issuer-attested whole-source label plus <=16 sorted closure labels. Rule/graph are mandatory digest refs, not copied `governed/program.rs` instruction semantics. No Poet adapter.
- Decision ticket `QBDL-PDD-V1` is 256 bytes, separately versioned. Binds `A.id()`, transcript digest, request ID, challenge[32], controller generation, policy digest, action, nbf/exp, consumed decision ID. Ticket+envelope bound 8192. Lease ceiling 30s downward-only.
- `DisclosureDecision` has private fields and only a `pub(super) fn bind` mint. `PolicyOutcome::Allow` and `bool` are refused. No `From`/`Default`/`pub fn new`.
- Rejected as remote authority: `VerifiedLabel` as issuer, `ConsentLedger` as PQ source auth, `SignedSensingScope` as whole-source.

## Test names

In `remote/tests.rs` (submodule of `remote/mod.rs`):

- `unknown_principal_is_denied`
- `source_wildcard_and_empty_source_are_denied`
- `eighth_enrollment_ok_ninth_is_capacity`
- `rotation_replaces_recognition_and_invalidates_old_decision_ids`
- `wrong_purpose_publisher_or_expired_recognition_is_denied`
- `policy_artifact_missing_gap_header_or_index_coverage_is_denied`
- `decision_ticket_codec_roundtrip_and_truncated_trailing_wrong_domain_refuse`
- `disclosure_decision_has_no_public_constructor`
- `recognition_table_holds_no_key_material`
- `wire_role_flag_cannot_install`
- `verified_label_consent_ledger_and_sensing_scope_are_not_remote_authority`

Focused selector after routing: `bundle::stream::policy::remote`

## What R2 must own

NEW `bundle/stream/policy/remote/{issuer,grant,current,lifecycle_tests}.rs` plus a narrowly claimed `net/qdnf/authority/current_guard` child for install/retire with exact owner affinity.

R2 is the first stage that may:

- Sign `RecipientRecord` and the decision ticket through existing PinAuthority / central managed signing
- Evaluate current consent/policy under a short current guard
- Consume challenges once, with two independently owned controller/verifier states
- Call `AuthorityOwner::install_grant_with_ceiling` and `CurrentAuthorityGuard` install/retire
- Recheck live recognition + pin receipts immediately before wrap/send/consumer (do not trust a copied `RecognizedController` snapshot across rotation)
- Treat `DisclosureDecision::bind` as post-verification mint only, never as a substitute for paired COSE

Do not start those files in this R1 claim.

## Risks

- `policy/mod.rs` is unrouted until the integrator adds `mod remote;`. This worker did not compile.
- `DisclosureDecision::bind` is schema/recognition binding only. It is not paired verification.
- `RecognizedController` is `Copy` metadata. Rotation invalidates table handles and decision-id floors; R2 must re-query the live table and PinAuthority.
- Rule/graph refs are digests. Extracting a neutral instruction helper from `governed/program.rs` was not required; R1 stopped rather than copying inference instruction semantics.
- `RecognitionAdmin` uses `fill_os` like `LocalBootstrap`. Tests also install real PinAuthority pins to obtain `PinHandle` values (handles are otherwise unconstructible outside `authority/`).

## What was not run

- `cargo test` / `cargo check` / `cargo build` (integrator owns the serial compile slot)
- git commit / push
- no synth MCP kill
