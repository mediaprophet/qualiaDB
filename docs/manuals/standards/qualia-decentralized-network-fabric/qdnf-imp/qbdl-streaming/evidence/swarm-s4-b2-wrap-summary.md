# Wave S4 / QBS-10 B2 — local authorized metadata-master wrap

Application compositor for recipient wrap of an existing metadata master. `ManagedObjectPublication::wrap_for` is called, not reimplemented. Provider policy is not disclosure authority. Native interchange is the existing 1328-byte `QBDL-PWR-V1` envelope. No JSON.

## Files written

- `crates/qualia-core-db/src/bundle/stream/protection/wrap.rs`
- `crates/qualia-core-db/src/bundle/stream/protection/wrap_tests.rs`
- `crates/qualia-core-db/src/bundle/stream/protection/mod.rs` (routing only: `mod wrap;`, `#[cfg(test)] mod wrap_tests;`, crate-private re-export)
- this summary

Not edited: `managed_object/publication.rs`, `object_stream/{recipient,wrap}.rs`, `packet_protection.rs`, WSS/HTTP, registries, Cargo, signing_profile.

## Entry point

`authorize_and_wrap(publication, grant, pins, clock, record, proof, workspace, out) -> Result<usize, WrapError>`

Order:

1. Zeroize `out` on every error/unwind.
2. Recheck current `LocalWholeSourceGrant` via `accept_local_scoped` (pins + clock) immediately before wrap.
3. Bind recipient record A to the grant (recipient, publisher/principal, purpose, operation, instrument, scope) and to the authenticated closure (source digest/length, policy, consumer, closure digest). KEM public keys are in A and therefore in AuthId.
4. Verify recognized issuer paired COSE with existing `PinAuthority::verify_source` over AuthId, purpose `qbdl-stream-protected-v1/recipient-authority`, audience = `publisher_principal[48]`. Recheck the receipt. No `verified: bool`. No second verifier. No signing_profile/registry edit.
5. Call `publication.wrap_for(record, out)`. Wrap attempt counting and the 16-cap stay in `wrap_for`.
6. Success is a candidate envelope, not durable publication and not plaintext disclosure.

`WrapError` is crate-private in `wrap.rs` (`Unauthorized`, `Closed`, `Spent`, `Capacity`). `ProtectionError` was not extended: `error.rs` is outside this slice's write set. Spent/expired/missing grant and unrecognized proof map to `Spent` / `Unauthorized`. The 17th wrap maps to `Closed` from `wrap_for`.

## Test names

- `grant_missing_spent_expired_leave_no_master_bytes` — spent grant and expired grant refuse; `out` is all-zero (no master bytes).
- `wrong_recipient_or_context_refuses_and_erases` — grant recipient mismatch, publication context mismatch, and wrong COSE purpose refuse and erase `out`.
- `authorized_wrap_is_exact_envelope_then_private_handle` — 1328-byte `QBDL-PWR-V1` envelope; unwrap through `ManagedRecipientUnwrap` into the existing private decrypt-only handle; metadata leaf opens without exporting the master.
- `sixteenth_wrap_succeeds_seventeenth_is_closed` — 16th wrap succeeds; 17th is `Closed` and erases `out`. One wrap counter, in `wrap_for`.

## Residuals (still open)

- PacketProtection private carrier / delivery of Q.
- Private M decode compositor (authenticated metadata after unwrap).
- Durable promotion of wrap/candidate artifacts.
- Policy-owned protected-source binding (`QBDL-PROTECTED-WHOLE-SOURCE-V1`); this slice reuses the existing plaintext `LocalWholeSourceGrant` as the current whole-source disclosure grant.
- HTTP/WSS, public cache of wraps, raw-key bypass — not introduced.

## What was not run

- `cargo test` / `cargo check` / `cargo build` (integrator owns the serial compile slot)
- git commit / push
- no synth MCP kill

Formatted `wrap.rs`, `wrap_tests.rs`, and `mod.rs` with `rustfmt --edition 2021`.
