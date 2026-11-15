# QBS-12 M64 v3 partial-range source map

Read-only map of the current developmental M64 georaster v3 implementation and the QBDL range adapter. No full-volume result should be inferred from a selected range.

## Current source boundary

- `crates/qualia-core-db/src/q42/m64_matter/checked_reader.rs:140,166,354,372,380`: `M64Reader::parse_policy` admits versions 1/2/3, verifies header CRC/size, decodes the 128-byte GRM3 block at bytes 64..192 for v3, and applies common directory/record bounds. It then verifies all records and, for v3, calls `georaster::validate_container` over the complete byte slice. This is the whole-file admission path; `M64Reader::georaster` exposes decoded metadata and `raster_sample` maps in-bounds coordinates to a chunk/record.
- `crates/qualia-core-db/src/q42/m64_matter/georaster/wire.rs:39`, `georaster/mod.rs:73,118`: `M64GeoRasterMetadata::decode` checks GRM3 magic/schema/CRC/reserved/policy fields, reconstructs metadata, calls `shape`, and validates declared record/chunk counts. `shape` bounds dimensions/chunk extents and computes exact whole-file geometry; `georaster_chunk_hilbert` defines row-major chunk keys.
- `crates/qualia-core-db/src/q42/m64_matter/georaster/validation.rs:6`: `validate_container` checks v3 header summary, exact total length, every directory row’s mode/codec/count/length/offset/SRID/Hilbert key, then every record’s non-raster bytes and RepeatNearest edge padding against source cells. It is currently whole-file-only and crate-private.
- `crates/qualia-core-db/src/bundle/stream/formats/m64.rs:39,61,171`: `M64DirectoryView::admit` deliberately accepts only versions 1/2 and requires BLAKE3-full rows plus optional authenticated LRM3. `selected_chunk` verifies the selected chunk’s 64-byte BLAKE3 trailer and validates typed records. V3 is outside this view because its georaster profile uses codec 0 and no per-chunk trailer.
- `crates/qualia-core-db/src/bundle/stream/formats/m64_tests.rs:160,246,326`: current authenticated-range tests cover v1/v2/E64, full chunk digests, record CRC, LRM3, legacy prefix refusal, bounds and budgets. They do not exercise GRM3/v3.

## Minimal reusable seam

Keep the GRM3 decoder and `shape` as-is. Refactor `georaster::validate_container` into reusable internal checks rather than copying its row/profile rules into the stream adapter:

1. A metadata/directory closure validator taking `(logical_length, header, meta, exact authenticated directory bytes)` should reuse the existing header-summary and row loop. It can validate all row CRCs (through the existing generic `row.validate()` path or an explicit checked call), exact row order/offsets, row-major Hilbert keys, and shape consistency without reading any brick payload. Require the entire directory range before admitting v3 interpretation.
2. A selected-record validator should take `(meta, chunk_index, exact selected 4096 × 64 record bytes)` and reuse the per-record non-raster-field check. The selected range must be exactly the directory row’s declared payload and have an outer QBDL proof; codec 0 means no inner M64 digest is available for this profile.
3. Preserve padding semantics carefully. `validate_container` compares RepeatNearest padded edge records with their in-bounds source cells, which may reside in another chunk. A partial adapter can either accept authenticated caller-supplied dependency ranges for those source cells and validate them, or expose only in-bounds samples and explicitly leave padded-record equality unclaimed. It must not silently call the selected chunk a fully validated raster tile if these cross-chunk edge checks were skipped.

Then add a separate `M64GeoRasterDirectoryView` (or explicit v3 branch/type) that authenticates header + GRM3 + complete directory, checks logical length/metadata budgets and v3-only flags, and borrows selected record bytes only after the exact row’s selected range is verified. Do not route v3 through `M64DirectoryView::selected_chunk`: that method’s full-trailer contract is incompatible with codec 0. Avoid whole-file checks requiring every raster payload; no arbitrary holes-as-zero reconstruction.

## Focused tests for a later implementation

- Build a small real v3 georaster with more than one chunk; prove header, GRM3 block, complete directory, and one selected chunk through actual QBDL selection/span verification. Compare selected borrowed record pointer with source bytes.
- Assert metadata/row corruption, mismatched shape/header summary, bad row order/key/offset, malformed selected-record reserved bytes, short/shifted payload range, identity mismatch, and limits fail closed. Preserve v1/v2 tests.
- Corrupt an unfetched chunk while creating a fresh outer entry root. The v3 partial view should admit an unaffected selected chunk only if its own validation passes; `M64Reader::parse` must still reject the malformed whole file. State plainly this does not validate the missing chunk.
- For `RepeatNearest`, test selected in-bounds records separately from padded-edge records. If padding comparison is exposed, provide and verify the required neighboring source cells; otherwise do not report padded cells as validated raster samples.

No implementation or build was performed for this map.
