# C3 inventory — QAC streaming adapter (no parser copy)

Coordinate with the EMF/QAC codec owner. Do not start until C1 is in source. Do not copy `audio::qac` parsers into `bundle/stream/formats`. Do not edit `audio/qac/**` or `q42/field_*`.

Cutover (codec owner, 2026-11-14): canonical current `.qac` is the v1 envelope (`b"qac\0\0\0\0\0"`, version `1`, `Family::QacV1Candidate`). Legacy UA4Q (`55 41 34 51`, v1..=3) remains on disk as `audio/qac/legacy.rs` and is **not** re-exported from `audio/qac/mod.rs`. Do not ask them to re-export `LegacyQacView` until after C0/C1, and only if a historical-leaf path is still required.

## Current routing

`bundle/stream/formats/mod.rs` exports EMF v2/v3, M64 v1/v2/v3, 10D, Q42 ranges. No QAC module.

## Canonical complete-entry admit (use this)

```rust
crate::audio::qac::QacStorageView::open(
    bytes: &'a [u8],
    options: crate::audio::qac::QacStorageOptions,
) -> Result<QacStorageView<'a>, crate::audio::qac::QacStorageError>
```

- `view.structure()` → `&crate::q42::field_envelope::StructuralView<'a>`
- `view.structure().verify_chunk(index)` — 1-based entry per-chunk BLAKE3
- Header CRC-32C; `file_bytes == bytes.len() as u64`
- Partial/truncated/trailing slices refuse: `Truncated | Layout | Trailing | Crc`

Typed signal (modes `0x100` PCM, `0x101` spectral/STFT, `0x104` preserved original):

```rust
crate::audio::qac::signal::admit(view, selected, scratch, shared_limits, limits)
    -> Result<SignalView<'a>, crate::audio::qac::signal::Error>
```

`QacStorageError::Envelope(field_envelope::Error)`:
`Limit | Truncated | Magic | Version | Flags | Layout | Alignment | Overlap | Crc | Order | DuplicateId | Reference | Compression | Digest | Trailing`

Signal errors also include `Unsupported | Shape | Scalar | Unit | Clock | Overflow | Nonfinite | Index | Limit | Profile`.

Outer QBDL leaf proof remains transfer integrity. Dispatch on magic/version bytes, not filename. Renaming `.qac` is not transcoding.

## First adapter (complete-entry fallback)

NEW `formats/qac.rs`:

- `QacCompleteView<'a>` holds an `AuthenticatedRange<'a>` covering the **entire** selected entry.
- `admit` calls `QacStorageView::open(range.bytes(), options)`. Map envelope errors into `FormatError` without collapsing Magic/Version/Crc/Digest/Truncated.
- Partial ranges return `FormatError`.
- Optional second step: `signal::admit` for scientific use; unsupported critical semantics refuse.

## Explicitly later

- Re-export of `LegacyQacView::parse` only if a historical UA4Q leaf must still stream (codec owner will re-export on request after C0/C1).
- Candidate shared-envelope timed QAC alongside EMF.
- Automatic `.qac` key rewrite.

## Mixed-HMC note (C4)

A valid current `.qac` complete entry is one of the six required families in the HMC review §6 scenario. Adapter success is not multiplex completion.
