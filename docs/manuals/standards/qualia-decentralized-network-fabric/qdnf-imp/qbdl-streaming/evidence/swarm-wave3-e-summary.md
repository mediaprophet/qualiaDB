# Wave 3-E summary — EMF streaming support

Authenticated current `.emf` via `StructuralView::open(..., Family::EmfV4Candidate, limits)`. Dispatch is magic/version bytes, not filename. Partial ranges refuse. Unknown critical scientific interpretation is `OpaqueArchival` (`ready_for_plan == false`), never a GPU display cell / sRGB. Native binary only. No JSON. No Kind 13+. No BearerProfile 8+.

Tests were **written, not run**. Do not cargo. `formats/mod.rs` was not edited.

## Files written

- `crates/qualia-core-db/src/bundle/stream/formats/emf_stream.rs` (new)
- `crates/qualia-core-db/src/bundle/stream/formats/emf_stream_tests.rs` (new; `#[path]` from `emf_stream.rs`)
- this summary

Not edited: `formats/mod.rs`, `formats/emf.rs`, `formats/emf_v3.rs`, `q42/field_*`, `q42/emf_container/**`, `q42/emf_profiles/**`, `render/gpu/emf_pipeline.rs`, portal EMF, etalon-render, `audio/qac/**`, Cargo, git, mux, qac, qac_stream, registries.

## Exports

```rust
pub enum EmfProfile {
    EnvelopeV4,         // Family::EmfV4Candidate complete-entry
    DirectoryV2,        // reserved; EmfDirectoryView split-range brick
    InterpretationV3,   // reserved; EmfV3DirectoryView split-range brick
    OpaqueArchival,     // structural v4 + unknown-critical interpretation
}

pub struct EmfStreamView<'a>
impl<'a> EmfStreamView<'a> {
    pub fn admit(range: AuthenticatedRange<'a>, limits: Limits) -> Result<Self, EmfAdmitError>;
    pub const fn profile(&self) -> EmfProfile;
    pub const fn ready_for_plan(&self) -> bool; // false when OpaqueArchival
    pub const fn range(&self) -> AuthenticatedRange<'a>;
    pub fn structure(&self) -> &StructuralView<'a>;
    pub fn select_region(&self, chunk_or_entry_index: usize) -> Result<&'a [u8], EmfAdmitError>;
    pub fn declared_interval(&self, chunk_or_entry_index: usize) -> Option<(i64, u64)>;
}

pub enum EmfAdmitError {
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

pub const fn emf_present_id(entry_id: u32, view: &EmfStreamView<'_>) -> Option<u32>;
```

`admit` requires `offset == 0` and `bytes.len() as u64 == selected.logical_length()`, else `Extent`. Then peeks magic/version. `Q42EMF64` + version 4 → `StructuralView::open(..., EmfV4Candidate)`. No `EmfStorageView`.

`select_region` is zero-based streaming index, mapped to 1-based `StructuralView::verify_chunk`. Selected complete chunk only, not a whole-volume receipt.

`declared_interval` returns `Entry::start_tick` / `time_count` only when the timed flag (`flags & 2`) is set. Otherwise `None`. No invented clock.

## Profile dispatch

| bytes | result |
| --- | --- |
| magic ≠ `Q42EMF64` | `Family` (no filename sniff) |
| version 4 | `StructuralView::open`; then EnvelopeV4 or OpaqueArchival |
| version 1 / 2 / 3 (`EMF_LEGACY_VERSION` / `EMF_CONTAINER_VERSION` / `EMF_EXTENDED_VERSION`) | `Unsupported` — **not** `DirectoryV2` / `InterpretationV3`, not a v4 open |
| other version | `Version` |
| unknown-critical schema/field/type after v4 open | `OpaqueArchival`, `ready_for_plan == false` |

Unknown-critical uses public `Block`/`Field` + `ProfileRegistry::rules` (schemas 103, 105..=107, 132..=137) and published shared schemas `1..=5`. Does not copy envelope/directory parsers. Does not call `emf_pipeline` or portal `DisplayEmfCell`.

## v2 / v3 brick coverage

**Not admitted from this API.** `EmfDirectoryView::admit` / `EmfV3DirectoryView::admit` need split header/directory/(v3 tail) `AuthenticatedRange`s. `AuthenticatedRange` has no public subrange constructor; this slice is not allowed to edit `range.rs` / `emf.rs` / `emf_v3.rs`. No in-tree v3 tail was reused; none was fabricated.

`EmfProfile::DirectoryV2` and `InterpretationV3` are reserved for an integrator who can pass split authenticated ranges into the existing brick views. Recognized brick versions refuse `Unsupported` here so a version-byte flip cannot silently become EnvelopeV4 or a fake DirectoryV2.

No live v2/v3 fixture was admitted. Candidate fixtures under `q42/field_envelope/fixtures/candidate/emf_candidate-*.emf` are envelope **v4**.

## Live `field_envelope::Error` → `EmfAdmitError`

Same table as C3 QAC. Distinct arms: `Family`, `Version`, `Extent`, `Crc`, `IndexDigest`, `PayloadDigest`, `Limit`. Remainder (`Header`, `Directory`, `EntryIndex`, …) is `Envelope(other)`. Not collapsed into `FormatError::Integrity`.

Open order that matters for tests: Family (peek), brick version → Unsupported, other version → Version, then `StructuralView` (CRC, structure, index digest). Partial `AuthenticatedRange` is `Extent` before open. `OPAQUE[..100]` is a 100-byte whole payload → open `Extent` (header needs 128). Trailing extra would be `Envelope(Header)` (header `file_size`); not a dedicated test.

`PayloadDigest` is not produced by `open` (only `verify_chunk` / `select_region`). Index-digest mutation without CRC retouch is `Crc`.

## Fixtures used

All under `crates/qualia-core-db/src/q42/field_envelope/fixtures/`:

| file | use |
| --- | --- |
| `candidate/emf_candidate-opaque.emf` (1088 B) | EnvelopeV4, ready_for_plan, select_region, version/digest mutations |
| `candidate/emf_candidate-aligned4096.emf` (8192 B, one leaf) | second admit; distinct region bytes vs opaque |
| `candidate/emf_candidate-two-entries.emf` (1280 B) | two inner `select_region` results, identities differ |
| `malformed/emf_candidate-truncated.emf` | truncated whole payload refuses |
| `malformed/emf_candidate-unknown-critical-schema.emf` | OpaqueArchival, not ready_for_plan |
| `malformed/emf_candidate-payload.emf` | open succeeds; `select_region` → `PayloadDigest` |

`candidate/emf_candidate-metadata-only.emf` not used (0-entry structural file; not required).

## Tests written (not run)

`formats/emf_stream_tests.rs` (unrouted until integrator adds `mod emf_stream`; tests ride `#[path]`):

1. `v4_candidate_admits_envelope_profile` — opaque → `EnvelopeV4`, `ready_for_plan`, `family() == EmfV4Candidate`, `emf_present_id` Some
2. `two_emf_entries_are_distinct_regions` — opaque + aligned4096 as two admits; two-entries inner chunks differ
3. `version_byte_mismatch_is_unsupported_not_v2_fallback` — version 2/3 → `Unsupported`; 99 → `Version`
4. `truncated_or_partial_range_refuses` — short length, `offset != 0`, `OPAQUE[..100]`, truncated fixture
5. `last_digest_or_verify_chunk_refuses_integrity` — last index-digest byte (`header[87]`) + CRC retouch → `IndexDigest`; payload fixture `select_region` → `PayloadDigest`
6. `select_region_returns_authenticated_chunk` — matches `verify_chunk(1)`; missing index → `Envelope(EntryIndex)`
7. `opaque_archival_is_not_ready_for_plan` — unknown-critical-schema → `OpaqueArchival`, present id None; verified bytes still selectable (opaque preservation, not a display cell)
8. `non_emf_bytes_fail_family_without_filename_sniff` — QAC magic and `0xff` magic

Match on `Result`. No Debug derive on `EmfStreamView`. Tests may allocate (`Fixture`, `Vec`). Hot admit: no extra heap (stack walk of public metadata views).

## Integrator routing snippet

```
// formats/mod.rs — tests are path-declared from emf_stream.rs; do not add mod emf_stream_tests
mod emf_stream;
pub use emf_stream::{emf_present_id, EmfAdmitError, EmfProfile, EmfStreamView};
```

Place beside the existing format `mod` / `pub use` pairs. No `Cargo.toml` change. Modules will not compile until `mod emf_stream` is added. Focused selector after routing: `bundle::stream::formats::emf_stream::emf_stream_tests`.

## Risks

- Unrouted: nothing typechecks until `formats/mod.rs` grows `mod emf_stream`.
- v2/v3 brick not exercised. Callers that already split ranges should keep using `EmfDirectoryView` / `EmfV3DirectoryView` until a split-range admit is added here.
- EnvelopeV4 is structural complete-entry, not `ProfileReader` scientific admission. Unsupported EMF modes stay EnvelopeV4 unless the metadata is unknown-critical.
- `select_region` on OpaqueArchival still returns verified stored bytes (R07 opaque preservation). Planner must use `ready_for_plan` / `emf_present_id`, not chunk presence, as the measurement gate.
- Index-digest mutation without CRC retouch is `Crc`. The digest test retouches CRC-32C of the 128-byte header with bytes 124..128 zeroed.
- `Format(FormatError)` unused by admit. Partial ranges are `Extent`, not `Format(Bounds)`.
- Tests rebuild a one-leaf QBDL span via `policy::tests::Fixture` (same helper as `qac_tests.rs`). Largest fixture used is 8192 B; `SpanLimits.max_encoded_bytes = 16384`.
