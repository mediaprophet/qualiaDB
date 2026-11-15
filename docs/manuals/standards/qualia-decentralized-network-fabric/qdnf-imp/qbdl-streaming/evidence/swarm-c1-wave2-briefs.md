# Wave 2 briefs — C1 residuals (queued until Wave 1 authors stop)

Do not start while C3/C4/C5 hold NEW-file claims. Integrator routes Wave 1 `mod.rs` and runs one cargo first.

## C1-D — Describe / descriptor bootstrap (expert)

**Gap:** R01 acceptance needs actual Describe/descriptor/index bootstrap. The two-process and in-process clients still reconstruct selected entry/trust locally.

**Writes (proposed):** NEW `qbdl/service/describe.rs` only. Integrator adds `mod describe;` and a thin `PlainProvider::answer_describe` / `PlainClient::recv_descriptor` if needed.

**Behavior:** client sends `Kind::Describe`; provider answers `DescriptorPage` from the live admitted catalog (identity, generation, entry count, policy generation). Client must not already own the complete source fixture. Malformed/stale generation refuse. No JSON.

**Forbidden:** inventing Kind 13+; changing wire PREFIX_BYTES; holding guards across WouldBlock.

**Acceptance:** client without pre-built `SelectedPlainEntry` can Select/Open after Describe pages; identity mutation of Describe/DescriptorPage refuses.

## C1-P — Protected two-process (expert)

**Gap:** `two_process.rs` still uses framing `Session` + caller `TRANSCRIPT`. Not a protected QSession. Not QBS-09 complete even after this slice (physical-network evidence remains separate).

**Writes:** `qbdl/service/two_process.rs` only, unless a one-line vis bump is requested of integrator.

**Behavior:** parent/child run the existing handshake pair (or the established `ProtectedChannel` owner already used by `protected_session_tests`) then `ProtectedSession::attach`. Same useful-leaf transfer. HTTP/DNS/IP/libp2p remain disabled.

**Acceptance:** `EvidenceClass::Process` still; wrong-transcript AAD refuses; do not claim PhysicalNetwork.

## C1-W — Remote grant wrap binding (expert)

**Gap:** `authorize_and_wrap` takes `LocalWholeSourceGrant`. A locally trusted paired pin with the right purpose/audience but no rights over this source must refuse.

**Writes:** exact claim under `bundle/stream/protection/` after Wave 1. Do not rewrite `policy/remote/**` unless integrator grants.

**Acceptance:** wrap under live `RemoteDisclosureGrant::recheck`; foreign source pin Unauthorized; clock/expires follow the wrap 4/4 conventions (`ml_private`, unwrap `SystemTime::now()` for long-lived recipient fields).

## C2 residual (later)

Crash wrapper still takes `&mut StorageStage`. Separate claim on `storage/native/write.rs` only.
