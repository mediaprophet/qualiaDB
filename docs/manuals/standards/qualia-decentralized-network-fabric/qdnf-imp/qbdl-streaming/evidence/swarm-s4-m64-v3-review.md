pass: true
open_bugs: 0

# Wave S4 / QBS-12 — M64 georaster v3 partial admission independent review

Read-only source review. Production Rust was not edited. Cargo was not run. Author summary (`swarm-s4-m64-v3-summary.md`) was not treated as proof. Contract map: `qbs12-m64-v3-partial-map.md`. Unintegrated drafts were compared, not trusted.

Reviewed:

- `crates/qualia-core-db/src/q42/m64_matter/georaster/{range.rs,validation.rs,mod.rs,wire.rs}`
- `crates/qualia-core-db/src/bundle/stream/formats/{m64_v3.rs,m64_v3_tests.rs,mod.rs,m64.rs,range.rs}`
- `crates/qualia-core-db/src/q42/m64_matter/{mod.rs,checked_reader.rs,layout.rs,chunk_directory.rs,strata_record.rs}` (callers / types only)
- `evidence/qbs12-m64-v3-partial-map.md`
- `evidence/swarm-s4-m64-v3-summary.md` (claims only, not proof)
- `evidence/unintegrated-prototypes/m64-v3/` (draft contrast)

## Review

S4 is a separate `M64GeoRasterDirectoryView` over header + GRM3 + the complete authenticated directory, then one in-bounds 64-byte strata record. Codec 0 has no per-chunk BLAKE3 trailer. V3 is not admitted by `M64DirectoryView` and has no `selected_chunk`. Whole-file `validate_container` is factored, not shortened: when `M64Reader::parse` has the complete byte slice it still walks every directory row and every record, including RepeatNearest padding equality. The stream view does not.

Integrator routing is present in the tree (`georaster/mod.rs` `mod range` + `pub(crate) use`; `formats/mod.rs` `mod m64_v3` + `pub use M64GeoRasterDirectoryView` + `mod m64_v3_tests`). The author summary still says those files were not edited. The code is what was reviewed.

## Fail-closed checklist (source, not summary)

1. **V3 is not routed through `M64DirectoryView::selected_chunk`.** Holds.
   - `M64DirectoryView::admit` still requires `version` 1 or 2 (`m64.rs:61-67`). V3 returns `Unsupported` before any row walk.
   - `selected_chunk` (`m64.rs:171-216`) still splits a BLAKE3-full trailer (`BLAKE3_TRAILER_BYTES`) and calls `verify_blake3_trailer`. No v3 branch.
   - New type is `M64GeoRasterDirectoryView` with `selected_sample` only (`m64_v3.rs:100-133`). No chunk accessor.

2. **Partial view does not claim a full-volume receipt or validate unfetched chunk payloads.** Holds.
   - Module/type comments state this is not a full-volume receipt (`m64_v3.rs:1-2,13-14`).
   - `admit` authenticates header (offset 0, 64), GRM3 (offset 64, 128), and the exact directory at byte 192 (`m64_v3.rs:39-70,71-77`). Directory row CRCs and the rigid codec-0 profile are checked; brick payloads are not read (`range.rs:20-50`).
   - `selected_sample` borrows one 64-byte record after the directory row’s declared offset matches (`m64_v3.rs:124-132`). It does not walk sibling records or other chunks.
   - Corrupt-unfetched test dirties chunk 0 reserved bytes with a repaired record CRC; `M64Reader::parse` still returns `GeoRaster(NonRasterFields)`; the view admits the chunk-1 sample (`m64_v3_tests.rs:513-537`). Comment states the missing chunk is not validated.

3. **RepeatNearest padded cells are not reported as validated raster samples.** Holds.
   - `sample_location` refuses `x >= width || y >= height` as `SampleError::Padding` (`range.rs:59-61`), mapped to `FormatError::Geometry` (`m64_v3.rs:143-148`).
   - No padded-cell accessor. `selected_sample` never compares clamp-to-edge neighbours.
   - `m64_v3_repeat_nearest_exposes_in_bounds_samples_not_padded_cells` builds a 65×64 RepeatNearest volume with live `encode`, admits `(64,8)`, refuses `(65,8)` as `Geometry`, and refuses the padded byte range as an in-bounds sample (`Bounds`) (`m64_v3_tests.rs:541-576`). Whole-file `raster_sample(65,8)` is `None`.

4. **No holes-as-zero reconstruction.** Holds.
   - Missing/short/shifted ranges are `Bounds` (`m64_v3.rs:109-131`; tests at `m64_v3_tests.rs:289-318`).
   - Padded RepeatNearest fixtures copy the nearest in-bounds elevation into the edge records (`m64_v3_tests.rs:125-131`); they do not zero-fill. The partial path never synthesizes a sample for an unproved offset.

5. **Whole-file `validate_container` still walks every record when the complete payload is present.** Holds.
   - Diff is a factor, not a skip: `shape_and_header_summary`, `directory_row_profile`, and `non_raster_fields_present` extract the previous checks (`validation.rs:4-57`).
   - `validate_container` still loops `0..shape.chunks` and `0..4096`, checks non-raster bytes, and compares RepeatNearest padded elevations to source cells (`validation.rs:61-97`).
   - `M64Reader::parse_policy` still verifies every record CRC, then calls `georaster::validate_container` on the complete slice (`checked_reader.rs:322-356`). The stream view never calls it.

6. **No new magics, JSON codec, or invented inner BLAKE3 for codec 0.** Holds.
   - GRM3 decode/encode in `wire.rs` is unchanged (git status: not modified). Header magic remains `m64\0` via existing `M64Header::validate` + `shape_and_header_summary` (`layout.rs:129-135`; `validation.rs:12-13`).
   - `directory_row_profile` requires `compression_flags == 0` and zero `bao_root_hash_prefix` (`validation.rs:36-40`). No `verify_blake3_trailer` in `m64_v3.rs` / `range.rs`.
   - No `serde` / JSON in the new modules.

7. **Tests construct real v3 bytes and fail closed on corruption.** Holds.
   - Fixtures write a v3 header with CRC, live `M64GeoRasterMetadata::encode` into bytes 64..192, Hilbert/CRC directory rows, and CRC’d `M64StrataColumnRecord` payloads (`m64_v3_tests.rs:47-134`).
   - Ranges go through `Fixture::new_with_payload`, `admit_header` / `admit_index` / `select_entry`, `TreeBuilder` over `LEAF_BYTES`, and `verify_leaf_span` (`m64_v3_tests.rs:136-249`). Borrowed sample pointer is compared to source (`m64_v3_tests.rs:279-280`).
   - Fail-closed cases: limits, shifted GRM3/directory, header summary mismatch, v2, LRM3 flags, GRM3 bit flip, Hilbert/offset/order/row-CRC, reserved-field and record-CRC sample corruption, foreign entry identity, short/shifted sample, out-of-extent coordinates (`m64_v3_tests.rs:322-510`).

8. **Integrator routing and exports for `formats/m64_v3.rs` imports.** Holds.
   - `georaster/mod.rs:4-9`: `mod range;` and `pub(crate) use range::{sample_location, validate_header_directory, validate_sample_record, SampleError}`.
   - `formats/mod.rs:36-39`: `mod m64_v3; pub use m64_v3::M64GeoRasterDirectoryView; #[cfg(test)] mod m64_v3_tests`.
   - `m64_matter/mod.rs:13,31` already has `pub mod georaster` and `pub use georaster::*`, so `M64GeoRasterMetadata` / `M64RasterSample` / `M64GeoRasterError` resolve. `AuthenticatedRange::same_entry` is the existing `pub(super)` method used by `m64.rs`.

## Also checked

- **Header + GRM3 + complete directory before a selected sample.** `admit` requires same-entry identity, exact offsets/lengths, `version == 3`, `container_flags == 0`, GRM3 `decode`, shape/limits, then `validate_header_directory` over the full directory (`m64_v3.rs:36-87`). `selected_sample` is a method on that view.
- **Corrupt unfetched chunk vs whole-file parse.** Behaviour holds (item 2). The test comment is slightly loose: the dirty record sits at payload offset 320, which is in leaf 0 with the directory, so those bytes are *proved* with the metadata leaf and *not interpreted*. The view still does not validate chunk 0.
- **Identity mismatch across authenticated ranges.** Nested second fixture with a flipped sample byte; `selected_sample` returns `Identity` (`m64_v3_tests.rs:496-510`; `range.rs:57-63`).
- **No warning suppressions / unwrap in new non-test paths.** `range.rs`, `m64_v3.rs`, and the `validation.rs` factor have none. `wire.rs` `try_into().unwrap()` after a 128-byte slice check is pre-existing and unedited.

## Nits (not open_bugs)

- Author summary and the unintegrated-prototype README are stale: they describe unrouted drafts and “mod.rs not edited.” Production has routing, shared validation factoring, RepeatNearest in-bounds-only exposure, and the five named tests.
- `m64_v3_admits_an_unaffected_sample_when_an_unfetched_chunk_is_corrupt` corrupts a record that shares Merkle leaf 0 with the directory. A stricter fixture would dirty a record in a leaf that is not used for header, GRM3, directory, or the selected sample.
- Several directory-corruption cases assert only `is_err()` (Hilbert, offset, swapped rows, row CRC) rather than the `Geometry`/`Integrity` variant.
- `sample_offset` in tests hard-codes `chunks_x = 2`. Fine for these fixtures; not a coordinate helper.
- `validate_sample_record` does not re-check in-bounds `(x,y)`; `selected_sample` is the only caller and runs `sample_location` first.
- `directory_row_profile` uses `payload_start + chunk * 4096 * 64` rather than `checked_add`. `shape()` has already required `file_bytes` to fit in `usize`, so it cannot wrap on this layout; still less explicit than `range.rs`.
- No partial-view nodata sample. Whole-file sentinel handling is unchanged.

## Issues
