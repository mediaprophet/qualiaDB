# EMF electromagnetic field container standard

**Revision:** 1 — 12 November 2026. **Current container:** v2, magic `Q42EMF64`, extension `.emf`. **Status:** Internal implementation-aligned standard, including full BLAKE3 repair. See [family conventions](file-formats-overview.md) and [conformance evidence](file-format-conformance.md).

## 1. Scope and model

EMF stores electromagnetic/wave-optical cell values in spatial bricks, with spectral and discrete time selectors. It is neither Windows Enhanced Metafile nor E64. The durable field lives in EMF; a sibling `.10d` carries geometry and field declarations, and `.m64` may carry occupancy/matter. Display RGBA or acoustic conversion is a derived view, not replacement scientific storage.

All numeric fields are little-endian. Header, directory rows and cells are exactly 64 bytes. Current POD readers explicitly refuse non-little-endian hosts. They copy fixed metadata into aligned values and borrow payload bytes; arbitrary caller memory must not be assumed 64-byte aligned.

Canonical layout is header, directory, then chunks in stable ascending Hilbert-key order. Chunk positions and lengths are taken from the directory. Each v2 chunk consists of `cell_count * 64` record bytes followed by the 64-byte full-digest trailer. No compression or proof tree is encoded by v2.

## 2. Header: 64 bytes

| Offset | Size/type | Field and meaning |
|---:|---|---|
| 0 | 8 bytes | ASCII `Q42EMF64` |
| 8 | u16 | Version: 2 for new output |
| 10 | u16 | Container flags |
| 12 | u32 | Number of directory rows |
| 16 | u64 | File-relative directory offset |
| 24 | f64 | Reference epoch in seconds |
| 32 | u32[3] | Grid dimensions in bricks, X/Y/Z |
| 44 | f32 | Voxel spacing in metres |
| 48 | f32[3] | Origin in metres |
| 60 | u32 | CRC-32C of header bytes `[0,60)` |

Header flags: bit 0 declares 8×8×8 bricks; bit 1 indicates polarization data; bit 2 indicates spectral bands. Bit 3 was the defective historical `BAO_VERIFIED` claim and MUST be zero in v2. Conforming new writers MUST clear undefined bits. Current structural admission rejects the legacy bit in v2 but does not comprehensively validate all application flag combinations or scientific values.

The timestamp reference alone does not encode a time-step duration, calendar/time-scale convention or temporal interpolation. Applications needing those semantics must bind them in associated metadata. Positive finite spacing, finite origin/epoch and appropriate dimensions are application-level requirements, not guarantees of structural parsing.

## 3. Directory row: 64 bytes

| Offset | Size/type | Field |
|---:|---|---|
| 0 | u64 | Spatial Hilbert key |
| 8 | u64 | Chunk start, file-relative |
| 16 | u32 | Stored length including v2 trailer |
| 20 | u32 | Cell count, excluding trailer |
| 24 | u16 | Spectral band mask |
| 26 | u16 | Chunk flags |
| 28 | u32 | Time-slice index |
| 32 | 16 bytes | Historical `bao_hash_prefix`; MUST be all zero in v2 |
| 48 | 12 bytes | Reserved; MUST be zero in v2 |
| 60 | u32 | CRC-32C of row bytes `[0,60)` |

Row flag bit 0 is unallocated by this v2 profile; no compressed codec is implemented by this reader/writer path. Conforming v2 output MUST leave it clear. Bit 1 (`0x0002`) is the defective legacy prefix flag and MUST be clear. Bit 2 (`0x0004`) is the mandatory full-BLAKE3 trailer flag. Conforming writers MUST clear all other bits. The current writer preserves other caller-supplied chunk flag bits, so callers must restrict flags to this profile. The reader explicitly checks bit 2, rejects bit 1 and nonzero old-prefix/reserved bytes; its permissiveness toward other bits does not define additional codecs.

Band-mask bit meanings beyond the stored 16-bit selector are not a complete spectral ontology. A producer/consumer profile must agree on band definitions. Hilbert key alone does not identify a unique time slice: use spatial key together with `time_slice_index` and any profile-defined spectral organization.

## 4. Cell: 64 bytes

All sixteen lanes are f32; offsets below are within one cell.

| Offset | Field | Meaning/unit |
|---:|---|---|
| 0, 4, 8 | `e_field_xyz` | Electric field vector, V/m |
| 12, 16, 20 | `b_field_xyz` | Magnetic flux density vector, tesla |
| 24 | `poynting_flux` | Stored flux/intensity scalar, W/m² |
| 28 | `wavelength_nm` | Wavelength, nanometres |
| 32 | `refractive_n` | Real refractive index |
| 36 | `extinction_k` | Extinction coefficient |
| 40 | `coherence_phase` | Coherence phase, radians |
| 44, 48, 52 | `stokes_q/u/v` | Polarization components |
| 56 | `spectral_sigma` | Authored spectral-coordinate scalar |
| 60 | `confidence_alpha` | Confidence scalar |

There is no additional independently stored Stokes-I lane. The polarization flag does not create one. A consumer must not infer physical consistency between E, B, stored flux and polarization solely from checksum success. Likewise, NaNs, infinities, negative wavelengths or implausible confidence require domain validation before scientific use.

The current `effective_sigma` helper uses nonzero authored sigma first; otherwise a positive wavelength maps through `clamp((wavelength_nm - 400) / 300, 0, 1)`, and absent/nonpositive wavelength yields zero. This helper is a conversion convention, not a serialization change or proof of a complete spectrum. Rendering and sonification may derive outputs from the same cells; output approximations and calibration remain consumer responsibilities.

## 5. Full BLAKE3 trailer and integrity

For `N = cell_count`, let `R = file[offset : offset + 64*N]`. The stored chunk length MUST be `64*N + 64`.

| Chunk-relative position | Meaning |
|---|---|
| `[0,64*N)` | Exact cell bytes |
| `[64*N,64*N+32)` | All 32 bytes of default unkeyed `blake3::hash(R)` |
| `[64*N+32,64*N+64)` | 32 zero bytes |

Verify all 32 digest bytes and every reserved trailer byte. Hash no directory/header/trailer bytes as part of `R`; never hash decoded/reformatted floats instead of the original cell bytes. The old 16-byte slot stays zero. The trailer is not a 513th cell, and `chunk_payload`/`cell` expose only records.

CRC checks cover metadata, while the digest covers exact cell bytes. These checks detect corruption relative to their stored values, but an attacker who can rewrite both data and checks can recompute them. Publisher authenticity, rights and PQ streaming commitments belong to an independently trusted outer mechanism. No Bao outboard tree or range proof exists in this encoding.

## 6. Spatial and dense-volume behavior

A full brick contains 8³ = 512 cells: 32,768 record bytes and 64 trailer bytes. Local order uses Morton interleaving of three bits per axis: X at `3*i`, Y at `3*i+1`, Z at `3*i+2`. Macro-brick positions use 3D Hilbert keys.

The current file does not serialize Hilbert order. The dense reader tries orders `[2,3,1,4,5,6,7,8]`, seeking keys that round-trip inside declared dimensions with unique `(x,y,z,time)` placements. This inference is a compatibility limitation. Do not standardize an unstored order as if it were explicit. Applications should use an agreed bounded grid convention and reject ambiguity where it affects correctness.

Dense output has dimensions `grid_dims_bricks * 8`, with `nt = max(time_slice_index)+1`, ordered `[t][z][y][x]`. It refuses more than 1,048,576 decoded cells across all times. Missing cells receive `EmfCellRecord64::default()`, including refractive index 1 and confidence 1; they are not all-zero bytes or independently measured observations. Consumers needing missing-data semantics must retain occupancy/coverage separately.

Structural writer/reader paths allow cell counts other than 512. A full-brick scientific profile MUST enforce 512 itself. Stable key sorting does not impose a temporal order among equal spatial keys, and structural parsing does not establish sortedness or unique spatial/time occupancy; dense conversion adds its own placement checks.

## 7. Admission and resource limits

`EmfContainerReader::parse` uses a 16 MiB file ceiling and 4096-row ceiling. `parse_with_limits` accepts explicit alternatives. Before exposing cells, the current parser checks header/row CRC, supported version, directory alignment/bounds, checked offset/count arithmetic, 64-byte chunk alignment, payload bounds, exclusion of metadata, pairwise nonoverlap, exact count/length relation and v2 integrity encoding/digest.

These are full-buffer structural checks; the API does not supply authenticated partial network admission. Pairwise overlap checks are bounded by the caller's chunk limit. Dense allocation is a separate operation with its own cap. A narrow valid file can still request an excessively large dense grid and must be refused there.

Writers MUST initialize reserved bytes/padding, sort deterministically, derive all lengths with checked arithmetic, compute complete trailers before metadata CRCs, and return the actual written extent. The current writer's preparation sorts an owned collection; it is not a blanket zero-heap authoring guarantee.

## 8. Legacy migration

Version 1 used raw `N*64` chunks and a defective 16-byte BLAKE3 prefix. Default v2 readers reject it with `LegacyIntegrityRequiresOptIn`. `parse_legacy_with_limits` is migration-only: it reproduces the old check when the legacy flag/prefix requires it; absent old checks do not become full integrity. The historical error name `BaoHashMismatch` remains API compatibility text, not evidence of Bao verification.

Migration procedure: admit under explicit limits, retain record bytes and semantics, validate intended scientific profile, rebuild with the v2 writer, recompute full hashes/metadata CRCs, and reopen with default parsing. Rebuild enclosing HMC digests and external references. Preserve source provenance and record that old data was only legacy-verified; a new digest cannot retroactively prove its original authenticity.

A version-2 hash failure MUST NOT trigger a version-1 fallback. Neither changing the version byte nor padding the prefix to 32 bytes is migration. Existing v1-only binaries must be rebuilt/requalified before consuming v2.

## 9. Layout examples and conformance

A one-record structural test file has header `[0,64)`, one row `[64,128)`, cell `[128,192)`, digest `[192,224)` and zero trailer tail `[224,256)`. Total: 256 bytes; row count 1, cell count 1, stored chunk length 128. It is a valid structural test, not a full 8³ brick.

A canonical single full brick has the same header/row positions, records `[128,32896)`, trailer `[32896,32960)` and total length 32,960. An empty writer output contains the header and no chunk trailer.

Conformance cases include complete round-trip, deterministic repeated write, every digest/tail byte mutation, record corruption, mixed/missing flags, nonzero old prefix, count overflow, overlap, exact trailer exclusion from cell access, explicit v1 admission/reseal, dense caps and time-placement ambiguity. See the [test/evidence register](file-format-conformance.md).

## 10. Implementation references

- [Header, cells and constants](../../../crates/qualia-core-db/src/q42/emf_container/header.rs)
- [Directory and Morton helpers](../../../crates/qualia-core-db/src/q42/emf_container/chunk_directory.rs)
- [Writer](../../../crates/qualia-core-db/src/q42/emf_container/writer.rs), [checked reader](../../../crates/qualia-core-db/src/q42/emf_container/reader.rs)
- [Shared full-digest trailer](../../../crates/qualia-core-db/src/q42/chunk_integrity/mod.rs)
- [10D typed field boundary](10d-typed-field-payloads.md), [QDNF proposal boundary](file-formats-overview.md#qdnf-streaming-boundary-and-advisor-attachment)


## Opt-in v3 interpretation extension

The repaired v2 field format above remains the default. See the [EMF v3 interpretation and timed PCM addendum](emf-v3-extension.md) for explicit colour declarations, exact physical timebase, and original PCM-f32LE tracks, preserving the same 64-byte physical cells and full v2 chunk integrity.
