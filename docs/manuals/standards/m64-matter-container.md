# M64 Matter-64 format and checked SDK companion



**Status:** source-grounded draft; not a new canonical standard. Installed Core/geospatial APIs are the current subject. Selected native conformance passed on Windows x86_64 little-endian: 7 direct SDK cases within a 22-case aggregate (7 M64, 9 typed-payload and 6 consumer cases), with zero failures. The execution is recorded in [the Round 2 log](../../../../ecopunk/docs/qa/implementation-2026-10-25/batch-6/rounds/round2/native-conformance.log). This companion describes the checked reader's actual structural scope and the geospatial writer's current raw output; it does not claim authentication, scientific validity, Bao verification, streaming, or a passed qualification gate.



## Wire family



- Canonical Matter-64 magic is `m64\0`; `e64\0` is the legacy magic alias for early `.e64` prototypes. Both retain the same M64 wire interpretation and header API (`E64Header` remains an exact alias of `M64Header`). This alias is specific to the M64 contract.

- `M64_VERSION` is 2; `M64Reader::parse` accepts versions 1 and 2. The current geospatial raster writer still emits version 1. Do not infer the emitted version from the current-version constant.

- The wire is little-endian. The 64-byte header is followed by 64-byte directory entries and 64-byte records. Directory and payload starts must be 64-byte aligned.



## Header and directory



The 64-byte `M64Header` contains, in order: magic `[0..4]`; version `[4..8]`; header size `[8..16]`; total file size `[16..24]`; latitude/longitude origin as `f32` `[24..32]`; base elevation metres and vertical scale as `f32` `[32..40]`; chunk-directory offset `[40..48]`; chunk count `[48..56]`; header CRC-32C `[56..60]`; and container flags `[60..64]`. The header CRC covers bytes `[0..56]`.



Each 64-byte `M64ChunkDirectoryEntry` carries chunk identifier bytes, Hilbert key, payload offset/length, record count, SRID/geodetic code, LUT epoch, record mode, codec flag, Bao-root prefix, and CRC-32C over bytes `[0..60]`. CRCs detect accidental corruption; they do not authenticate an issuer, content, or Bao proof.



## Checked reader contract



`M64Reader::parse(bytes, M64ReadLimits, scratch)` requires explicit `max_file_bytes`, `max_chunks`, and `max_records` limits; there is no unlimited default. Caller-owned `M64PayloadInterval` scratch must have one entry per chunk. Parsing is little-endian only, uses checked arithmetic, and rejects unsupported flags/modes/codecs, size mismatch, malformed or out-of-range directory entries, misalignment, metadata overlap, payload overlap, and record-count/byte-length mismatch. Only raw codec flag 0 is admitted. Metadata and limits are checked before record CRC scanning; overlap sorting is iterative.



The reader accepts an unaligned input base by copying POD metadata with unaligned reads. `M64Chunk::payload()` and `record_bytes()` return borrowed bytes; they do not promise an aligned typed slice. Caller scratch may change on refusal. The view is immutable and zero-copy only for payload bytes.



| Record mode | Record CRC behavior |

|---|---|

| `M64_MODE_POINT` (`0x00`) | No embedded record CRC in the current layout; counted but not certified by `M64Verification`. |

| `M64_MODE_HOLO` (`0x01`) | No embedded record CRC in the current layout; counted but not certified by `M64Verification`. |

| `M64_MODE_STRATA` (`0x02`) | Each fixed 64-byte record validates CRC-32C over bytes `[0..60]`. |

| `M64_MODE_PHASE_JUNCTURE` (`0x03`) | Each fixed 64-byte record validates CRC-32C over bytes `[0..60]`. |



`M64Verification` reports counts, including records without checksums. A successful parse proves only the structural checks and listed checksums. It does not validate domain/scientific meaning, issuer authority, signatures, or a Bao tree.



## Raw geospatial writer



`qualia_geospatial::compile_raster_to_m64` delegates to `compile_raster_to_m64_with_codec` with `UncompressedMmap`. Other codec variants return `UnsupportedCompression`; no compressed or Bao output is produced. The writer requires a little-endian host, nonzero dimensions, exactly `width × height` `i32` elevation samples in millimetres, and a caller buffer large enough for the computed result. Arithmetic overflow and short buffers return errors.



The current output uses `m64\0`, version 1, record mode `0x02`, and codec flag 0. It partitions into 64×64-cell chunks, writes row-major chunks with Hilbert keys, and repeats the nearest input edge sample to fill partial edge chunks. Each output strata record stores `surface_elevation_mm` and has a record CRC. The frame SRID is retained; geographic origin and elevation datum values are narrowed to `f32`. Although `ElevationRaster` has `cell_size_mm` and `GeodeticFrame` has a vertical-datum declaration, this writer does not serialize them as a separate horizontal cell resolution or vertical-datum identifier. `ElevationRaster` has no separate nodata mask, so this API does not establish nodata semantics. These limits and conversions are part of the writer contract and must be represented honestly by callers.



## Source references



- Core layout and aliases: `C:/github/qualiaDB/crates/qualia-core-db/src/q42/m64_matter/layout.rs`.

- Checked reader and structural checks: `C:/github/qualiaDB/crates/qualia-core-db/src/q42/m64_matter/checked_reader/{mod.rs,validation.rs,intervals.rs}`.

- Header, directory and record layouts: `C:/github/qualiaDB/crates/qualia-core-db/src/q42/m64_matter/{chunk_directory.rs,point_record.rs,holo_record.rs,strata_record.rs,phase_juncture.rs}`.

- Raw writer: `C:/github/qualiaDB/crates/qualia-geospatial/src/geotiff.rs` (`compile_raster_to_m64`, `compile_raster_to_m64_with_codec`).

- Direct SDK fixture source: `C:/github/qualiaDB/crates/qualia-core-db/tests/m64_container_sdk.rs`; its seven direct cases passed in the recorded native run; this does not qualify every producer/consumer or target.



## Subsequent developmental georaster profile

An explicit opt-in M64 version-3 GRM3 georaster profile is described in
[its separate developmental contract](m64-georaster-v3.md). The current default
`M64_VERSION` remains 2 and the legacy raster writer still emits its unchanged
version-1 bytes. The newer checked reader additionally admits the complete v3
profile under its separate structural/metadata checks. Earlier readers refuse
v3; the original Round 2 execution above qualified only its recorded v1/v2 scope.
The new profile retains f64 origin, integer source elevations, cell spacing,
datum/epoch declarations, source extent and explicit padding/nodata policy.
It does not perform a CRS/datum transformation or validate identifier authority.
See the source-pinned Round 3 results for execution status; this addition alone
does not qualify the profile or establish production readiness.
