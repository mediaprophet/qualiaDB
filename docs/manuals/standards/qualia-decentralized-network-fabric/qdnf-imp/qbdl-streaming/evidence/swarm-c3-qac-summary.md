# C3 summary — QAC complete-entry streaming adapter

Complete authenticated current `.qac` entry via `QacStorageView::open`. Partial ranges refuse. Legacy UA4Q is not re-exported and is not parsed. Dispatch is magic/version bytes, not filename. Native binary only.

Tests were **written, not run**. Do not cargo. `formats/mod.rs` was not edited.

## Files written

- `crates/qualia-core-db/src/bundle/stream/formats/qac.rs` (new)
- `crates/qualia-core-db/src/bundle/stream/formats/qac_tests.rs` (new)
- this summary

Not edited: `formats/mod.rs`, `formats/range.rs`, `audio/qac/**`, `q42/field_*`, Cargo, git, other Wave 1 files.

## Exports

```rust
pub struct QacCompleteView<'a>
impl<'a> QacCompleteView<'a> {
    pub fn admit(range: AuthenticatedRange<'a>, options: QacStorageOptions) -> Result<Self, QacAdmitError>;
    pub const fn range(&self) -> AuthenticatedRange<'a>;
    pub fn storage(&self) -> &QacStorageView<'a>;
}

pub enum QacAdmitError {
    Format(FormatError),
    Family,
    Version,
    Extent,
    Crc,
    IndexDigest,
    PayloadDigest,
    Limit,
    Unsupported,
    Envelope(field_envelope::Error),
}
```

`admit` requires `offset == 0` and `bytes.len() as u64 == selected.logical_length()`, else `Extent`. Then `QacStorageView::open(range.bytes(), options)` only. Never `LegacyQacView`. `admit_signal` was omitted (optional; not required for the first path). `Unsupported` is reserved for that later helper.

## Live `field_envelope::Error` → `QacAdmitError`

Inventory names `Truncated | Magic | Trailing | Flags | Layout | …` are stale. Live enum in `q42/field_envelope/mod.rs`:

| live `field_envelope::Error` | `QacAdmitError` |
| --- | --- |
| `Family` | `Family` |
| `Version` | `Version` |
| `Extent` | `Extent` |
| `Crc` | `Crc` |
| `IndexDigest` | `IndexDigest` |
| `PayloadDigest` | `PayloadDigest` |
| `Limit` | `Limit` |
| `Overflow` | `Envelope(Overflow)` |
| `Header` | `Envelope(Header)` |
| `Directory` | `Envelope(Directory)` |
| `Order` | `Envelope(Order)` |
| `Padding` | `Envelope(Padding)` |
| `Codec` | `Envelope(Codec)` |
| `Metadata` | `Envelope(Metadata)` |
| `Numeric` | `Envelope(Numeric)` |
| `EntryIndex` | `Envelope(EntryIndex)` |

`QacStorageError::Envelope(e)` uses the same table. `Format(FormatError)` is the `From<FormatError>` path; admit itself does not wrap `FormatError` today.

`PayloadDigest` is not produced by `StructuralView::open` (only `verify_chunk`). Admit does not call `verify_chunk`. The success test does, after admit.

Open order that matters for tests: Family, Version, then header CRC, then structure, then index digest. Trailing extra and declared-size mismatch hit `Header` (`u64le(h, 16) != bytes.len()`) before the final `next != len` `Extent` check. There is no `Trailing` arm.

## Tests written (not run)

`formats/qac_tests.rs` (unrouted until integrator adds `mod qac` + `mod qac_tests`):

- `current_complete_entry_admits_qac_v1_candidate` — `qac_candidate-opaque.qac`; `family() == Family::QacV1Candidate`
- `partial_range_is_extent` — short length and `offset != 0`
- `truncated_slice_refuses` — `CURRENT[..100]` → `Extent`; malformed 767-byte truncated fixture → `Envelope(Header)` or `Extent`
- `trailing_extra_byte_refuses` — appended byte; live open → `Envelope(Header)` (no Trailing arm)
- `mutated_version_byte_is_version` — `bytes[8] = 99`, same as `storage.rs`
- `last_digest_byte_corrupted_is_digest_error` — last index-digest byte (`header[87]`) flipped, header CRC retouched, else CRC wins → `IndexDigest`
- `legacy_ua4q_fixture_is_refused` — `audio/qac/fixtures/q4au-v2-contract.qac` (72 B → `Extent` before Family)
- `non_qac_bytes_fail_family_without_filename_sniff` — EMF magic and `0xff` magic, 256 B, no path/extension argument

Fixtures: `q42/field_envelope/fixtures/candidate/qac_candidate-opaque.qac` (768 B, one leaf), `audio/qac/fixtures/q4au-v2-contract.qac`, `q42/field_envelope/fixtures/malformed/qac_candidate-truncated.qac`.

## Integrator routing snippet

```
// formats/mod.rs
mod qac;
pub use qac::{QacAdmitError, QacCompleteView};
#[cfg(test)]
mod qac_tests;
```

Place beside the existing format `mod` / `pub use` / `#[cfg(test)]` pairs. No `Cargo.toml` change. Modules will not compile until those lines are added. Focused selector after routing: `bundle::stream::formats::qac_tests`.

## Risks

- Unrouted: nothing typechecks until `formats/mod.rs` grows `mod qac`.
- Trailing extra is `Envelope(Header)`, not `Extent`, because live open compares header `file_size` before remainder.
- Legacy UA4Q 72-byte fixture is `Extent` (`bytes.get(..128)`), not `Family`. Not reinterpreted.
- Index-digest mutation without CRC retouch is `Crc`. The digest test retouches CRC-32C of the 128-byte header with bytes 124..128 zeroed.
- `PayloadDigest` mapping is dead on the admit path (`open` does not hash payloads). Corrupt last payload byte would still admit; `verify_chunk` is the caller’s next gate.
- `admit_signal` / `Unsupported` unused on the first path. Unknown critical signal semantics are not refused here.
- `Format(FormatError)` unused by admit. Partial ranges are `Extent`, not `Format(Bounds)`.
- Tests rebuild a one-leaf QBDL span via `policy::tests::Fixture` (same pattern as `ten_d_tests` / `m64_tests`). Opaque fixture is 768 B; `SpanLimits.max_encoded_bytes = 16384`.
