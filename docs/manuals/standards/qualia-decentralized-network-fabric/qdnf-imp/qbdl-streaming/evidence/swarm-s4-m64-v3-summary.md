# Wave S4 / QBS-12 — M64 georaster v3 partial admission

Header + GRM3 + complete authenticated directory, then one selected in-bounds 64-byte record. This is not a full-volume receipt. Codec 0 has no per-chunk BLAKE3 trailer. V3 is not routed through `M64DirectoryView::selected_chunk`.

Native interchange stays exact-length binary. GRM3 decode/`shape` were not changed. Whole-file `validate_container` still walks every record when the complete byte slice is present; the stream view does not.

## Files written

- `crates/qualia-core-db/src/q42/m64_matter/georaster/range.rs` (new)
- `crates/qualia-core-db/src/q42/m64_matter/georaster/validation.rs` (factored header-summary / row-profile / non-raster helpers only)
- `crates/qualia-core-db/src/bundle/stream/formats/m64_v3.rs` (new)
- `crates/qualia-core-db/src/bundle/stream/formats/m64_v3_tests.rs` (new)
- this summary

Not edited: `georaster/mod.rs`, `formats/mod.rs`, `formats/m64.rs`, `formats/m64_tests.rs`, `checked_reader.rs`, `wire.rs`, Cargo, git.

## Semantics

- `validate_header_directory(logical_length, header, meta, directory)` reuses the whole-file header-summary and directory-row profile checks, then adds v3-only version/flags, `total_file_size == logical_length`, exact directory length, and per-row CRC. No brick payload is read.
- `sample_location(shape, x, y) -> (chunk, record)` refuses out-of-extent coordinates as `SampleError::Padding`.
- `validate_sample_record(meta, chunk_index, raw)` checks one 64-byte strata record: CRC and non-raster reserved bytes. No inner M64 digest.
- `M64GeoRasterDirectoryView::admit` requires same-entry header (offset 0, 64 B), GRM3 (offset 64, 128 B), and the complete directory at byte 192. Version must be 3 and `container_flags` must be 0. Budgets: entry bytes, row count, record count, metadata bytes, `max_chunk_bytes >= 64`.
- `selected_sample(x, y, range)` borrows the 64-byte record only after the directory row’s declared payload offset matches. No padded-cell accessor and no selected-chunk view.

## Padding residual

RepeatNearest clamp-to-edge sources under this tiling live in the same edge chunk as the padded cells (`x.min(width-1)`, `y.min(height-1)`). The partial view still **does not** treat padded coordinates as validated raster samples and **does not** claim padded-record equality. Neighbor-cell authentication is not exposed. Whole-file `validate_container` continues to compare padded edge records against source cells when the complete payload is present.

This does not reconstruct holes as zero.

## Test names

`formats/m64_v3_tests.rs` (unrouted until integrator adds `mod m64_v3_tests`):

- `m64_v3_reads_one_proved_in_bounds_sample_without_a_chunk_view`
- `m64_v3_metadata_directory_and_sample_fail_closed`
- `m64_v3_rejects_a_range_from_another_selected_entry`
- `m64_v3_admits_an_unaffected_sample_when_an_unfetched_chunk_is_corrupt`
- `m64_v3_repeat_nearest_exposes_in_bounds_samples_not_padded_cells`

The corrupt-unfetched-chunk test builds a fresh outer entry root over a payload whose chunk 0 reserved bytes are dirty. The v3 view admits a proved sample from chunk 1. `M64Reader::parse` still rejects the whole file. That admission does not validate the missing chunk.

v1/v2 tests in `m64_tests.rs` were not touched. Fixtures use live `M64GeoRasterMetadata::encode`; no second GRM3 encoder.

## Integrator routing snippet

```
// georaster/mod.rs
mod range;
pub(crate) use range::{sample_location, validate_header_directory, validate_sample_record, SampleError};

// formats/mod.rs
mod m64_v3;
pub use m64_v3::M64GeoRasterDirectoryView;
#[cfg(test)]
mod m64_v3_tests;
```

Suggested placement: `mod range;` beside `mod validation;` in `georaster/mod.rs`. In `formats/mod.rs`, beside the existing `mod m64` / `mod m64_tests` pair. `sample_location` returns `(usize, usize)` (chunk, record); `SampleLocation` is not a public type, so the re-export list above is sufficient.

No `Cargo.toml` change. The new modules will not compile until those two `mod` lines are added. Focused selector after routing: `bundle::stream::formats::m64_v3_tests`.
