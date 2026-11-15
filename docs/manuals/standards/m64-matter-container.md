# M64 matter container standard

**Revision:** 1, 12 November 2026. **Status:** Internal implementation-aligned standard. Default writer version: 2. Checked readers support versions 1 and 2 and the separately constrained developmental GRM3 version 3. See the [format overview](file-formats-overview.md) for terminology and security boundaries.

## 1. Purpose and identity

M64 stores fixed 64-byte matter records, organized into independently addressed chunks. Point samples, holographic matter, strata, phase junctions and shells are different record modes; a chunk has one mode. This is not the EMF electromagnetic-cell format. Canonical magic is the four bytes `6d 36 34 00` (`m64` followed by NUL). The [E64 compatibility spelling](e64-compatibility-format.md), `65 36 34 00`, is accepted for v1/v2 only. Extension-based guessing MUST NOT replace magic/version checks.

All scalar fields below use little-endian encoding. Directory offsets are absolute file offsets. Fixed-width records do not imply that arbitrary source bytes may safely be cast to native structs: alignment, endianness, range validation and lifetime remain obligations.

## 2. Header: 64 bytes

| Offset | Bytes | Type | Meaning |
|---:|---:|---|---|
| 0 | 4 | bytes | Canonical magic |
| 4 | 4 | u32 | Version |
| 8 | 8 | u64 | Header size: 64 for v1/v2; 192 for GRM3 |
| 16 | 8 | u64 | Exact total file length |
| 24 | 8 | f32[2] | Spatial origin: latitude, longitude |
| 32 | 8 | f32[2] | Elevation datum: base altitude in metres, scale |
| 40 | 8 | u64 | Directory offset |
| 48 | 8 | u64 | Number of 64-byte directory rows |
| 56 | 4 | u32 | Header CRC-32C |
| 60 | 4 | u32 | Flags |

The current header CRC covers **the concatenation of bytes [0,56) and [60,64)**. It is not the CRC of a 64-byte header with a zero checksum. Historical CRCs covering only [0,56) are accepted for compatibility; that omission leaves flags outside their checksum and is a legacy weakness, not the required writer rule.

Recognized flags are bit 4 (`0x10`, 16³ brick layout) and bit 5 (`0x20`, LRM3 local reference frame present). Other bits are rejected by the checked profile. LRM3 starts at `header_size` and occupies 128 bytes before the directory. It does not change the v1/v2 header-size field to 192. The GRM3 extended header has its own rules and requires flags zero.

## 3. Directory: 64 bytes per chunk

| Offset | Bytes | Type | Meaning |
|---:|---:|---|---|
| 0 | 16 | bytes | Chunk UUID |
| 16 | 8 | u64 | Hilbert key |
| 24 | 8 | u64 | Payload offset |
| 32 | 4 | u32 | Stored byte length, including any trailer |
| 36 | 4 | u32 | Record count, excluding the trailer |
| 40 | 4 | u32 | Spatial reference identifier |
| 44 | 2 | u16 | LUT epoch |
| 46 | 1 | u8 | Record mode |
| 47 | 1 | u8 | Codec identifier |
| 48 | 12 | bytes | Defective historical digest prefix; zero in current raw/full profiles |
| 60 | 4 | u32 | CRC-32C of row bytes [0,60) |

Mode identifiers are 0 point, 1 holo, 2 strata, 3 phase junction and 4 shell. A mode identifier is not a schema inference hint: an unsupported mode MUST be rejected before record interpretation.

| Codec | Encoding and acceptance |
|---:|---|
| 0 | Raw records, exactly `64 × count` bytes. With a zero prefix, accepted without cryptographic chunk assurance. A nonzero prefix requires explicit legacy migration admission. |
| 1 | Reserved/declared LZ4 path; the checked reader does not implement it and rejects it. |
| 2 | Defective legacy prefix-verified records, exactly `64 × count`. Migration-only. |
| 3 | Current full BLAKE3 records: exactly `64 × count + 64`; prefix bytes MUST be zero. |

For codec 3, the final 64 bytes contain the **complete 32-byte default BLAKE3 digest over only the record bytes**, followed by 32 zero bytes. All 32 digest bytes and all 32 padding bytes are checked. Record CRC fields, where present, are part of the hashed record bytes. The trailer is excluded from record iteration and GPU upload. No compression or Bao proof tree is implied by codec 3.

## 4. Record modes

Every table below spans exactly 64 bytes. The generic container reader does not certify that encoded values describe physically plausible matter. Applications must validate finite values, domain constraints, referenced vertices/entities and applicable units before simulation.

### 4.1 Point record (mode 0)

| Offset | Bytes | Type | Meaning |
|---:|---:|---|---|
| 0 | 12 | f32[3] | Local x/y offsets in metres; z in metres relative to datum |
| 12 | 4 | f32 | Intensity |
| 16 | 6 | i16[3] | Quantized normal |
| 22 | 1 | u8 | ASPRS LiDAR classification |
| 23 | 1 | u8 | Return information |
| 24 | 8 | u8[8] | Spectral channels |
| 32 | 4 | u32 | Packed density, hardness, hydraulic porosity and flags bytes |
| 36 | 4 | u32 | Timestamp, epoch seconds |
| 40 | 8 | u64 | FNV-based DID reference |
| 48 | 16 | bytes | Reserved |

Normal packing clamps to [-1,1] and scales by 32767; unpacking divides by 32767. A DID hash is a compact semantic identifier, not a cryptographic signature or a replacement for a full digest. Point records have no individual CRC; codec 3 supplies chunk integrity.

### 4.2 Holo record (mode 1)

| Offset | Bytes | Type | Meaning |
|---:|---:|---|---|
| 0 | 6 | binary16[3] | Origin |
| 6 | 1 | u8 | Metric class: 0 Euclidean, 1 cyclic, 2 hyperbolic |
| 7 | 1 | u8 | Semantic class |
| 8 | 2 | i16 | Parent relationship |
| 10 | 4 | binary16[2] | Scale bounds |
| 14 | 2 | i16 | Spin |
| 16 | 6 | binary16[3] | Angular frequency vector |
| 22 | 2 | binary16 | Damping |
| 24 | 2 | binary16 | Mass |
| 26 | 2 | u16 | Phase mask |
| 28 | 4 | u32 | Hamiltonian reference |
| 32 | 12 | i8[12] | Harmonic coefficients |
| 44 | 4 | i8[4] | Tropism |
| 48 | 6 | u8[6] | Spectral coefficients |
| 54 | 2 | u8[2] | Roughness |
| 56 | 8 | u64 | DID reference |

Binary16 fields occupy u16 storage, with half-float interpretation. Holo has no per-record CRC. Semantic labels and compact coefficients require a shared interpretation profile; they do not establish a universal material ontology by themselves.

### 4.3 Strata record (mode 2)

| Offset | Bytes | Type | Meaning |
|---:|---:|---|---|
| 0 | 4 | i32 | Surface elevation, mm |
| 4 | 2 | u16 | Water-table depth, mm |
| 6 | 2 | u16 | Fluid depth, mm |
| 8 | 4 | i16[2] | Horizontal velocity, mm/s |
| 12 | 2 | u16 | Sediment concentration, ppm |
| 14 | 2 | u16 | Roughness code (millimetre convention in this record) |
| 16 | 12 | u16[6] | Layer thicknesses, mm |
| 28 | 6 | u8[6] | Layer material/LUT indices |
| 34 | 6 | u8[6] | Moisture fractions, 0–255 |
| 40 | 6 | u8[6] | Compaction codes |
| 46 | 6 | u8[6] | Artefact codes |
| 52 | 2 | u16 | Shear strength, kPa |
| 54 | 2 | u16 | Repose angle, tenths of a degree |
| 56 | 2 | u16 | Dielectric coefficient × 100 |
| 58 | 2 | u16 | Acoustic speed, m/s |
| 60 | 4 | u32 | CRC-32C over [0,60) |

Do not reinterpret the roughness code as a universally dimensionless Manning coefficient. Material indices and LUT epochs require an agreed bank. The base header defines no automatic embedded LUT section, length or offset. The banked LUT library is a consumer-side representation, not permission to insert unannounced bytes into the container.

### 4.4 Phase junction (mode 3)

| Offset | Bytes | Type | Meaning |
|---:|---:|---|---|
| 0 | 8 | u64 | DID reference |
| 8 | 8 | u64 | Lamport clock |
| 16 | 12 | i32[3] | Anchor, mm |
| 28 | 4 | u32 | Branch identifier |
| 32 | 4 | u32 | Event flags |
| 36 | 4 | f32 | Action value; semantics depend on event profile |
| 40 | 16 | f32[4] | Stress values |
| 56 | 2 | u16 | Radius, mm |
| 58 | 1 | u8 | Material identifier |
| 59 | 1 | u8 | Delay in ticks |
| 60 | 4 | u32 | CRC-32C over [0,60) |

Flags 0–4 designate excavation, collapse, flood, cartoon and speculative events. The action field is not universally seconds: consumers also use quantities such as a volume deficit. Lamport clocks order events; they do not authenticate them or supply physical time.

### 4.5 Shell record (mode 4)

| Offset | Bytes | Type | Meaning |
|---:|---:|---|---|
| 0 | 4 | u32 | Face identifier |
| 4 | 12 | u32[3] | Vertex references |
| 16 | 12 | u32[3] | Edge references |
| 28 | 12 | f32[3] | Normal |
| 40 | 12 | f32[3] | Centroid |
| 52 | 4 | f32 | Crease value |
| 56 | 4 | u32 | Flags |
| 60 | 4 | u32 | CRC-32C over [0,60) |

Reference consistency and topology validity require the shell consumer; CRC success alone establishes neither.

## 5. Spatial layout and local frames

The brick profile uses 16 × 16 × 16 records, 4096 records per brick. Local Morton order interleaves the four bits of x, y and z: x at bit `3i`, y at `3i+1`, z at `3i+2`. Macro brick order uses a Hilbert key. The helpers support orders through 21; their masking/clamping behavior is not a serialized declaration of coordinate range. The base header does not carry a Hilbert order. Producers and consumers must agree on that spatial profile.

A full codec-3 brick occupies 262,208 bytes: 262,144 record bytes plus a 64-byte trailer. A legal small chunk need not be a full brick; applications imposing full occupancy must check that separately.

LRM3 is a 128-byte block:

| Offset | Bytes | Type | Meaning |
|---:|---:|---|---|
| 0 | 4 | bytes | `LRM3` |
| 4 | 2 | u16 | Version 1 |
| 6 | 1 | u8 | Tier |
| 7 | 1 | u8 | Frame: 0 Cartesian, 1 cylindrical, 2 spherical |
| 8 | 8 | u64 | DID reference |
| 16 | 24 | f64[3] | Origin |
| 40 | 16 | f32[4] | Quaternion x, y, z, w |
| 56 | 12 | f32[3] | Scale in micrometres |
| 68 | 12 | i32[3] | Minimum bounds, micrometres |
| 80 | 12 | i32[3] | Maximum bounds, micrometres |
| 92 | 32 | bytes | Reserved |
| 124 | 4 | u32 | CRC-32C over [0,124) |

The Cartesian helper scales by `1e-6`, rotates and translates. Advertising cylindrical or spherical frame codes does not establish equivalent transform support in that helper. Validate quaternion, scale, bounds and frame support at the consuming boundary.

## 6. GRM3 georaster v3

The [GRM3 companion](m64-georaster-v3.md) specifies the 128-byte metadata block at file offsets 64–191. It is developmental, not a promise that released v1/v2 readers accept v3. It uses canonical M64 magic, header size and directory offset 192, flags zero and **raw codec 0 strata chunks**. Codec 3 MUST NOT be substituted without a separately specified GRM3 revision.

Authoritative georeferencing uses f64 latitude/longitude, i32 origin elevation in millimetres and explicit cell size, row direction, cell-center anchor, nodata and padding policy. Header f32 origin fields are summaries. Elevation records are absolute values; do not add the header base a second time. Chunks are 64 × 64 row-major cells (4096 records). Repeat-nearest edge padding is padding, not a new observation. A nodata sentinel is distinct from an absent chunk. Legacy raster emission loses some of this metadata; migration cannot reconstruct missing cell size or precision from the old bytes alone.

## 7. Checked admission and consumer obligations

The checked reader bounds file bytes, directory rows and total records using explicit limits, validates exact declared file length and supported version/header/flags, verifies header and directory CRCs, and uses caller-provided interval scratch to reject payload overlap. Offset arithmetic and record sizes are checked before slicing. Payloads cannot overlap required metadata. Each chunk must have a nonzero record count. Record CRCs are verified for strata, phase junction and shell; point and holo have no such field.

Its verification report distinguishes `full_hash_chunks`, `legacy_prefix_chunks` and records without individual checksums. `records_without_checksum` does **not** mean a codec-3 point chunk lacks integrity: its full chunk digest still applies. A producer claiming the full-hash profile MUST require every relevant chunk to be codec 3 and every verification result to report the expected full-hash count, with no legacy prefix chunks.

Default parsing rejects codec 2 and raw chunks carrying nonzero historical prefixes. `parse_legacy_with_limits` is an explicit migration API, not a fallback after a digest failure. Raw codec 0 with zero prefix remains valid but supplies only the applicable CRC checks. UUID uniqueness, directory sort order, scientific validity, LUT availability and referenced topology require application checks where those properties matter.

## 8. Migration and worked layout

For each admitted historical chunk, obtain the actual record bytes, validate available CRCs and independent provenance, calculate a new full digest, append its 64-byte trailer, select codec 3, zero the 12-byte prefix and rebuild offsets, stored lengths, row CRCs, total length and header CRC. Preserve record values and mode. Revalidate with the default checked reader. A successful prefix comparison does not prove the original bytes were authentic; newly hashing them cannot retroactively provide that provenance.

A minimal illustrative v2 one-record full-hash layout is header [0,64), one directory row [64,128), record [128,192), digest [192,224), zero trailer padding [224,256). The directory records offset 128, stored length 128, count 1, codec 3. Header total length is 256. These positions describe a valid simple layout, not a requirement that every writer place directory before all payloads.

## 9. Implementation authority

The [M64 module](../../../crates/qualia-core-db/src/q42/m64_matter/mod.rs), [layout](../../../crates/qualia-core-db/src/q42/m64_matter/layout.rs), [directory](../../../crates/qualia-core-db/src/q42/m64_matter/chunk_directory.rs), [checked reader](../../../crates/qualia-core-db/src/q42/m64_matter/checked_reader.rs), record modules in the same directory, [LRM3](../../../crates/qualia-core-db/src/q42/m64_matter/lrm3.rs), and [shared full-hash codec](../../../crates/qualia-core-db/src/q42/chunk_integrity/mod.rs) define the inspected implementation. Links are resolved within the repository. See [conformance](file-format-conformance.md) for executed checks and remaining limitations.
