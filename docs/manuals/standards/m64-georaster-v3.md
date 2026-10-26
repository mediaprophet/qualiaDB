# GRM3 developmental M64 georaster profile — revision 1

Status: proposed implementation contract approved for staged source development by
the integration supervisor on 2026-10-25. Not a released standard or scientific
qualification. FND-02/FND-04 remain open. Existing v1/v2 and `M64_VERSION=2`
defaults and legacy raster output are unchanged. V3 is an explicit opt-in;
earlier readers reject it. Core owns wire admission; geospatial owns conversion.

V3 requires canonical `m64\0`, version 3, flags zero, `header_size=192`, and
`chunk_table_offset=192`. The original 64-byte header has the same fields and CRC;
its f32 origin/base elevation are only summaries, which must equal narrowing of
the authoritative GRM3 values. The block at absolute bytes 64..192 is little-endian:

| Block bytes | Field |
|---|---|
| 0..4 | magic GRM3 |
| 4..6 / 6..8 | u16 schema revision 1 / u16 block size 128 |
| 8..16 / 16..24 | u64 source width / height |
| 24..32 / 32..40 | f64 first-cell-center latitude / longitude in degrees |
| 40..44 / 44..48 | i32 origin elevation mm / u32 cell size mm |
| 48..52 / 52..56 / 56..58 | original f32 epoch bits / u32 SRID / u16 vertical datum declaration |
| 58 / 59 / 60 | row direction 0 north or 1 south / anchor 0 first-cell-center / basis 0 local east-north |
| 61 / 62 / 63 | edge 0 reject partial or 1 repeat nearest / nodata 0 none or 1 i32 sentinel / reserved zero |
| 64..68 / 68..72 | i32 sentinel (zero when none) / u32 chunk grid size 64 |
| 72..80 / 80..88 / 88..96 | u64 emitted records / chunks / payload bytes |
| 96..124 / 124..128 | reserved zero / CRC-32C of block bytes 0..124 |

The first sample center is at the declared geographic anchor. Each
`surface_elevation_mm` is the original absolute integer elevation in the declared
vertical datum, including any nodata sentinel; the origin elevation is independent
frame metadata. Neither writer nor recovery subtracts or adds that base. The grid
declares local X east, Y north, equal spacing in mm; columns increase east and
rows increase in the selected north/south direction. No datum transformation,
geodesic solver, map projection, geographic footprint or physical accuracy is
established. Identifier numbers are retained declarations, not authority proofs.
Zero/unknown identifiers may be retained; no authority is inferred from a code.

Dimensions are nonzero, cell size nonzero, origin finite/in latitude/longitude
ranges, epoch finite; chunk axes cannot exceed the 16-bit Hilbert grid. Records
are 4096 per raw strata chunk, directory and payload row-major, contiguous after
the metadata/directory. Directory SRID agrees with metadata. All checksums,
summary values, declared counts and exact lengths must agree; unsupported flags,
reserved bytes, codecs, modes, anchor/basis and policy values are refused.

Only elevation is populated; other strata bytes are zero and do not assert
measured/physical material properties. Nodata is a retained optional i32 sentinel,
not a new validity mask. Sentinel occurrence remains distinguishable from valid
elevation; no interpolation occurs. Outside source width/height is padding,
never additional source observations. Repeat-nearest padding elevations must
equal the closest source edge sample; reject-partial requires full chunks.
The checked source accessor returns out-of-extent separately from nodata.

Caller supplies output, chunk-overlap scratch and explicit file/chunk/record
limits. Writer preflights size, metadata and cancellation before any write;
the bounded final write is non-interruptible and cannot return a partial-error
result. Decode likewise checks cancellation and capacity before writing.
Cancellation arriving after final admission may complete successfully.

Checksums establish accidental-corruption checks only. No signatures, identifier authority,
rights, source provenance adoption, human acceptance, absolute scientific datum
accuracy, complete HMC, Poet workflow, WASM or benchmark qualification is claimed.
Source snapshots and manifests accompany the implementation; tests are authored
for the supervisor's serial verification, not executed by the staging worker.
