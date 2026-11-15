# C3 brief — QAC complete-entry streaming adapter

Senior implementer. File-disjoint. **Do not cargo.** Integrator routes `formats/mod.rs` after you stop.

## Objective

Admit a **complete** authenticated `.qac` entry through the existing format-view pattern. Dispatch on magic/version bytes, not filename. Reuse `QacStorageView::open`. Do not copy parsers. Partial ranges refuse.

## Read first

- `docs/.../qbdl-streaming/evidence/swarm-c3-qac-adapter-inventory.md`
- `crates/qualia-core-db/src/audio/qac/storage.rs` (live `QacStorageView::open`, `QacStorageError`)
- `crates/qualia-core-db/src/audio/qac/mod.rs` (no `LegacyQacView` re-export)
- `crates/qualia-core-db/src/q42/field_envelope/mod.rs` live `Error` enum
- `crates/qualia-core-db/src/audio/qac/signal/` admit API
- Template: `bundle/stream/formats/ten_d.rs` and `range.rs` (`AuthenticatedRange`, `FormatError`)

Inventory error names (`Truncated|Magic|…`) may be stale. **Map the live `field_envelope::Error` variants** without collapsing Family/Version/Crc/IndexDigest/PayloadDigest/Extent into one arm.

## Allowed writes

- NEW `crates/qualia-core-db/src/bundle/stream/formats/qac.rs`
- NEW `crates/qualia-core-db/src/bundle/stream/formats/qac_tests.rs`
- `docs/.../qbdl-streaming/evidence/swarm-c3-qac-summary.md`

Do **not** edit `formats/mod.rs` (integrator). Do not add `mod qac`.

## Design

```rust
pub struct QacCompleteView<'a> { /* AuthenticatedRange covering the entire selected entry */ }

pub enum QacAdmitError {
    Format(FormatError),
    Family,   // magic / Family mismatch
    Version,
    Extent,   // truncated / not whole entry
    Crc,
    IndexDigest,
    PayloadDigest,
    Limit,
    Unsupported,
    Envelope(crate::q42::field_envelope::Error), // only if a live variant has no distinct arm
}

impl<'a> QacCompleteView<'a> {
    pub fn admit(range: AuthenticatedRange<'a>, options: QacStorageOptions) -> Result<Self, QacAdmitError>;
    pub fn storage(&self) -> &QacStorageView<'a>;
}
```

Rules:

- `range.offset() == 0` and `range.bytes().len() as u64 == range.selected().logical_length()`; else `Extent`.
- `QacStorageView::open(range.bytes(), options)` — never `LegacyQacView`.
- Optional helper `admit_signal(...)` calling `audio::qac::signal::admit`; unsupported critical semantics refuse (`Unsupported`). Not required for the first green path.
- Do not rewrite HMC entry keys. Renaming `.qac` is not transcoding.
- Do not edit `audio/qac/**` or `q42/field_*`.

Fixture bytes: `crates/qualia-core-db/src/q42/field_envelope/fixtures/candidate/qac_candidate-opaque.qac` (current). Legacy `audio/qac/fixtures/q4au-v2-contract.qac` must **refuse**.

## Tests (in `qac_tests.rs`, `#[cfg(test)]`)

- current complete entry admits; `structure().family() == Family::QacV1Candidate`
- partial range (offset≠0 or short length) → `Extent`
- truncated slice → refuse
- trailing extra byte → refuse (if live envelope refuses Trailing/Extent)
- version byte mutated → `Version` (see `storage.rs` `invalid_current_version_is_not_reinterpreted`)
- last digest byte corrupted → digest error, not silent admit
- legacy UA4Q fixture → refuse (not reinterpreted)
- filename `.qac` on non-qac bytes → Family/open fail; do not sniff extension

## Stop

Write the summary (files, exports, mapping table live-Error→QacAdmitError, tests written **not run**, risks). Do not cargo. Do not commit. Do not spawn children.
