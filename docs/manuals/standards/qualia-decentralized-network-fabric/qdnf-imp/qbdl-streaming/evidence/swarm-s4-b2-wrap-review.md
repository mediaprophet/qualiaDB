---
pass: true
open_bugs: 0
---

# Wave S4 / QBS-10 B2 — authorized wrap compositor independent review

Read-only source review. Production Rust was not edited. Cargo was not run. Author summary (`swarm-s4-b2-wrap-summary.md`) was not treated as proof. Contract anchors: `qbs10-compositor-contract.md` Stage B2 wrap paragraph; `qbs07-remote-authority-integration.md` B2 wrap/receiver publisher bullet.

Reviewed:

- `crates/qualia-core-db/src/bundle/stream/protection/wrap.rs`
- `crates/qualia-core-db/src/bundle/stream/protection/wrap_tests.rs`
- `crates/qualia-core-db/src/bundle/stream/protection/mod.rs` (routing only)
- `crates/qualia-core-db/src/specialized_libs/cryptographic_library/key_management/managed_object/publication.rs` (`wrap_for`)
- `crates/qualia-core-db/src/crypto/network/object_stream/wrap.rs`
- `crates/qualia-core-db/src/crypto/network/object_stream/types.rs` (`WRAP_BYTES`, `RECIPIENT_PURPOSE`)
- `crates/qualia-core-db/src/bundle/stream/policy/grant.rs`
- `crates/qualia-core-db/src/bundle/stream/policy/grant/scoped.rs`
- `crates/qualia-core-db/src/bundle/stream/policy/types.rs` (`PolicyError`)
- supporting: `crypto/network/authority/pins.rs` (`verify_source`/`recheck`), `crypto/network/cose/verify.rs`, `crypto/network/object_stream/recipient.rs` (`valid_at`), `net/qdnf/authority/owner.rs` (`issue_permit`)

`cargo test` / `cargo check` were not part of this review (integrator slot). Test names are cited as intent only.

## Review

This slice is the crate-private compositor `authorize_and_wrap`. It does not reimplement hybrid wrap. The only `wrap_for(` call site in `qualia-core-db` is `protection/wrap.rs`. Native wire remains the existing 1328-byte `QBDL-PWR-V1` encoder. Provider policy stays inside `wrap_for`; application disclosure is the current `LocalWholeSourceGrant` plus recognized recipient-authority COSE.

Author residuals (PacketProtection/Q delivery, private M decode, durable promotion, `QBDL-PROTECTED-WHOLE-SOURCE-V1`) are documented as open and are not claimed done. They are not this slice's bugs.

## Required attacks (source, not summary)

1. **`wrap_for` is called only after current grant recheck AND recognized recipient `PinAuthority::verify_source` + `recheck`. Provider policy alone is insufficient.** Holds.
   - Sole call site: `publication.wrap_for(record, wipe.out)` at `wrap.rs:82-84`, last statement of the `accept_local_scoped` callback.
   - `LocalWholeSourceGrant::accept_local_scoped` refuses `spent`, sets `spent = true`, then `control.recheck()` before the callback (`grant/scoped.rs:61-76`). That recheck issues the exact owner permit twice (`scoped.rs:29-36`).
   - Callback starts with another `gate.recheck()?` (`wrap.rs:60-61`), then `match_grant` (`wrap.rs:62,92-115`), then `record.valid_at(now)` (`wrap.rs:63-64`).
   - Recipient proof: `pins.verify_source(AuthoritySource::LocalPin(proof.pin), proof.envelope, &binding, now, workspace)` (`wrap.rs:74-80`) then `pins.recheck(&receipt, &auth.0, now)` (`wrap.rs:81`). No `verified: bool`. `verify_source` matches pin key ids/epoch/purpose/audience and `cose::verify_pair` (`pins.rs:124-152`, `cose/verify.rs:21-31`).
   - `wrap_for` still runs `check_now` / `check_locked` (provider policy) (`publication.rs:271,284`). That is additional fencing, not a substitute. The compositor cannot reach `wrap_for` on provider policy alone.
   - `wrap_for` remains `pub(crate)` with a comment that the compositor must check grant + recipient first (`publication.rs:253-255`). Current tree has no other caller.

2. **No raw master export, no second KeyManager, no wrap counter besides `wrap_for`'s `wraps >= 16`.** Holds.
   - `wrap.rs` never reads `workspace.masters`, never calls `metadata_master_row`, never calls `seal_master_raw`. Master bytes stay inside `wrap_for` → `seal_master_raw` (`publication.rs:281`).
   - No `KeyManager::new` in production wrap code. `wrap_for` uses the publication's existing owner plus ephemeral KEM/DH leases (`publication.rs:274-278`). Test fixture KeyManagers in `wrap_tests.rs` are not production path.
   - Publication counter is `wraps: u8`, closed at `wraps >= 16`, incremented once per attempt inside `attempt` (`publication.rs:42,260,273`). `wrap.rs` has no second counter, static, or wrap tally.

3. **Errors zeroize `out` (including unwind). Success keeps the 1328-byte envelope.** Holds.
   - `Wipe` is installed before any work; `keep` starts false; `Drop` zeroizes `out` unless `keep` (`wrap.rs:41-52,52`). `keep = true` is only after `accept_local_scoped` returns `Ok` (`wrap.rs:87-89`).
   - Early `Capacity` (`wrap.rs:53-55`) returns `Err` with `keep` false, so Drop zeroizes the short buffer.
   - `wrap_for` also zeroizes `out[..WRAP_BYTES]` on `Err` (`publication.rs:289-291`). Compositor Wipe covers panic/unwind after a successful copy, including exit-recheck failure after wrap (scoped.rs:77-78): the candidate is destroyed.
   - Success path copies exactly `WRAP_BYTES` (`publication.rs:286-287`) and returns that length (`wrap.rs:89`).

4. **Wrong recipient, wrong context, spent/expired grant, wrong COSE purpose refuse.** Holds.
   - Wrong recipient: `match_grant` compares `f.recipient` to `grant.binding().recipient` (`wrap.rs:99`). Mismatch is `PolicyError::Crypto(Unauthorized)` → `WrapError::Unauthorized` (`wrap.rs:126-127`). `wrap_for` is not reached.
   - Wrong context: `wrap_for` refuses `record.fields().context != c.id()` (`publication.rs:267-269`) → `CryptoError::Unauthorized` → `WrapError::Unauthorized`.
   - Spent grant: `accept_local_scoped` returns `PolicyError::Spent` before the callback (`scoped.rs:61-63`) → `WrapError::Spent` (`wrap.rs:119`).
   - Expired grant: `issue_permit` returns `QdnfError::Expired` when `now_unix >= rec.expires_unix` (`owner.rs:214-216`), mapped `E::Authority` in `recheck`, then `_` in `map_policy` → `WrapError::Unauthorized`. Fixture grant `expires: 90` (`policy/tests.rs:313`). Independent record window: `now >= fields.expires` is `Unauthorized` (`recipient.rs:110-115`).
   - Wrong COSE purpose: compositor Binding purpose is hardcoded `RECIPIENT_PURPOSE` (`wrap.rs:71`). `verify_pair` requires `parsed.binding == *expected` (`cose/verify.rs:29-31`). Pin `scope.matches` is exact purpose+audience (`authority/types.rs:98-100`). Envelope purpose `b"other"` cannot verify.

5. **Magic is `QBDL-PWR-V1`. Native wire is exact-length binary, not JSON.** Holds.
   - `WRAP_BYTES = 1328` (`object_stream/types.rs:17`). `encode_wrap` uses `Writer::new(out, WRAP_BYTES, b"QBDL-PWR-V1\0")` (`object_stream/wrap.rs:111-128`). `Wrap::parse` requires `bytes.len() == WRAP_BYTES` and the same magic (`object_stream/wrap.rs:21-24`).
   - `protection/wrap.rs` has no serde/JSON. Protection module grep for `serde`/`json`/`cache` is empty. Interchange is the caller `out` buffer.

6. **16th wrap can succeed; 17th is `Closed`. Publication wrap counter is the one in `wrap_for`.** Holds.
   - Attempt 16: `wraps == 15`, `wraps >= 16` is false, `wraps += 1` → 16, returns `Ok(WRAP_BYTES)`.
   - Attempt 17: `wraps >= 16` → `E::Closed` before increment (`publication.rs:260-261`) → `PolicyError::Crypto(Closed)` via `From<CryptoError>` → `WrapError::Closed` (`wrap.rs:121-123`).
   - Tests spend a fresh `LocalWholeSourceGrant` per wrap (`wrap_tests.rs:525-547`). That does not create a second wrap cap; the publication object is reused.

7. **`accept_local_scoped` marks `spent = true`: one bounded effect. Acceptable because `wrap_for` still owns the 16-cap.** Holds; not a bug.
   - `spent = true` is immediate (`scoped.rs:64`). One grant authorizes one wrap callback. A single grant cannot wrap 16 times with the existing API.
   - Contract language is one short owner guard per bounded effect (`qbs10-compositor-contract.md` Stage B2; `qbs07` publisher bullet). Each wrap is one effect. No new 16-shot grant API is required for this slice.
   - The 16-cap remains `ManagedObjectPublication.wraps` across those grants.

8. **`map_policy` type-checks: `PolicyError::Authority` carries `QdnfError`, not `CryptoError`.** Holds in current source.
   - `PolicyError { Crypto(CryptoError), Authority(QdnfError), ... }` (`policy/types.rs:19-29`). `impl From<CryptoError> for PolicyError` only (`types.rs:31-35`).
   - `CryptoError` is `pub use QdnfError as CryptoError` (`crypto/network/errors.rs:3`), so `Closed`/`Capacity` exist on both names.
   - `map_policy` matches `PolicyError::Crypto(CryptoError::Closed) | PolicyError::Authority(QdnfError::Closed)` and the Capacity pair (`wrap.rs:117-128`). `wrap_for` errors convert with `PolicyError::from` (`wrap.rs:84`) into the `Crypto(...)` arm.

9. **Audience for recipient-authority is publisher principal 48. Purpose is `RECIPIENT_PURPOSE`.** Holds.
   - Purpose: `RECIPIENT_PURPOSE = b"qbdl-stream-protected-v1/recipient-authority"` (`object_stream/types.rs:5`), Binding field `wrap.rs:71`.
   - Audience: `&record.fields().publisher.0` (`wrap.rs:72`). `StrongDigest` is `[u8; 48]` (`net/qdnf/types.rs:8`).
   - `match_grant` requires `f.publisher == expected.principal` before verify (`wrap.rs:100`). Pin install in tests uses `request.principal.0` as audience (`wrap_tests.rs:233-234,108`).

10. **No public cache of wrap bytes. Success is a candidate envelope, not durable publication or plaintext.** Holds.
    - `authorize_and_wrap` is `pub(crate)` (`wrap.rs:31`). `protection` is `pub(crate)` (`bundle/stream/mod.rs:16`). No store/cache/registry write. `out` is caller scratch.
    - `wrap_for` copies into `out` and does not retain wrap bytes on the publication (`publication.rs:286`). Comment at `wrap.rs:29-30` states candidate, not durable publication or plaintext.
    - Successful test path unwraps through `ManagedRecipientUnwrap` into private decrypt (`wrap_tests.rs:496-517`); that is test-only and does not export the master.

## Residuals (allowed; not this slice's bugs)

- PacketProtection private carrier / delivery of Q.
- Private M decode after unwrap.
- Durable promotion of wrap/candidate artifacts.
- Policy-owned `QBDL-PROTECTED-WHOLE-SOURCE-V1` binding; this compositor reuses plaintext `LocalWholeSourceGrant` as the current whole-source disclosure grant (author residual; not claimed complete).
- `match_grant` does not compare service, descriptor, or KEM public keys as a separate expected-admission table. Context is enforced by `wrap_for`. KEM keys are committed in AuthId (`RecipientRecord::id` = SHA-384 of the 1856-byte A) and COSE is over that AuthId. Full `AuthenticatedRecipient` expected-admission remains Stage B2 policy work, not this wrap child.
- Tests were not executed. Failure to compile or a red test would be an integrator finding, not a source-logic bug demonstrated here.

## Issues

None. `open_bugs: 0`.
