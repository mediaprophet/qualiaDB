# Wave 2 source (2026-11-14) — no cargo

Implemented in the shared QualiaDB tree. EMF/QAC files untouched. No cargo; EMF/QAC compile still owns default `target/` until that pause lifts.

## C1-D Describe bootstrap

- Additive `ServiceIo::recv_describe` / `recv_descriptor`. `recv_control` stays 80-byte Open/Grant/Credit and now rejects Describe.
- NEW `qbdl/service/describe.rs`: `DescribeClient::bind(CandidateIdentity, policy_generation)` without `SelectedPlainEntry`.
- Class 1 Describe → class 6 64-byte HMC header page; class 7 → index slice from live `admit_header`.
- `PlainProvider::answer_describe` rechecks `LocalEffectControl` before recv and before send.
- Identity mutation and stale policy generation refuse. Kind 13+ remains wire `Unsupported`. ZERO descriptor/context refuse bind.
- Tests live in `describe.rs` (`#[cfg(test)]`).

## C1-W remote wrap

- `authorize_and_wrap_remote` in `protection/wrap.rs`.
- Rechecks live `RemoteDisclosureGrant::recheck(..., MetadataRelease)`.
- `match_remote_grant` requires record source/length/policy/publisher/purpose/operation to match `grant.recognized()`.
- Foreign source (trusted pin, right purpose/audience, different source) → `WrapError::Unauthorized` and zeroizes `out`.
- `RemoteDisclosureGrant::recognized()` accessor. `policy::remote` is `pub(crate)`.
- Tests: `protection/wrap_remote_tests.rs` (`not(target_arch = "wasm32")`). Unwrap clock is `SystemTime::now()`; recipient `expires = 4_000_000_000`.

## C1-P ProtectedSession useful-leaf

- `transfer_one_leaf` is generic over `ServiceIo` (Session and ProtectedSession).
- In-process useful-leaf over handshake `with_pair` in `protected_session_tests.rs`.
- `two_process.rs` stays framing `Session` + caller transcript (`EvidenceClass::Process`). Not QBS-09 PhysicalNetwork.

## C2 residual

Left as-is. `write_leaf_with_hook` still takes `&mut StorageStage`. `process_crash.rs` WriteStep names stay frozen. Not a small disjoint edit.

## Next

One locked offline `qualia-core-db --lib` after EMF/QAC pause, filters covering `qbdl::service::`, `describe`, `wrap`, `protected_session_tests`. Then Wave 1 recargo (C3/C4/C5) if that source is still routed.
