# E64 compatibility format standard

**Revision:** 1, 12 November 2026. **Status:** Implementation-aligned compatibility profile. Read with the [M64 standard](m64-matter-container.md) and [format overview](file-formats-overview.md).

## Identity and scope

E64 is the historical `e64` plus NUL magic spelling of the M64 matter container. It is **not** an alias for EMF, a second electromagnetic-cell encoding, or an independent 64-byte field schema. Its first four bytes are `65 36 34 00`. The checked matter reader accepts this spelling for versions 1 and 2. Canonical new M64 authoring uses `6d 36 34 00`. Developmental GRM3 version 3 requires canonical M64 magic and does not admit E64.

## Complete wire mapping

Every structural rule and record table in the [M64 standard](m64-matter-container.md) applies unchanged to E64 v1/v2 except the four magic bytes:

| Component | Size and interpretation |
|---|---|
| Header | 64 bytes; u32 version at 4, u64 header size at 8, exact total length at 16, directory offset/count at 40/48, CRC at 56, flags at 60 |
| Header CRC | CRC-32C over concatenated [0,56) and [60,64); includes the E64 magic bytes |
| Optional LRM3 | 128 bytes at header-size offset when flag 0x20 is present |
| Directory | 64 bytes per row; mode at 46, codec at 47, obsolete prefix at 48, CRC at 60 |
| Records | 64 bytes each; modes 0 point, 1 holo, 2 strata, 3 phase junction, 4 shell |
| Spatial profile | Optional 16³ bricks under flag 0x10; local Morton and macro Hilbert conventions as specified for M64 |
| Full integrity | Codec 3, full 32-byte BLAKE3 digest plus 32 zero bytes after the records |

Numeric fields are little-endian. Embedded f32/f64/binary16 encodings, georeferencing, reserved fields, LUT resolution, record CRC coverage and reader limits are exactly those of M64. There is no E64-only compression, signature or trailer placement.

## Current integrity profile

An E64 chunk using codec 3 has stored length `64 × record_count + 64`. Its old 12-byte prefix MUST be all zero. Its trailer contains BLAKE3 over precisely the record region, followed by 32 zero bytes; the complete digest and padding are verified. The trailer is not included in record count or exposed as a matter record.

Codec 0 with zero prefix is still an admitted raw profile, without full cryptographic chunk integrity. Codec 1 is unsupported by the checked reader. Codec 2, and codec 0 with nonzero prefixes, describe defective historical encodings admitted only by the explicit legacy migration API. The truncated hashes were bugs: their presence does not establish a useful standardized security level, a Bao proof or a current full-hash conformance claim.

E64 spelling does not force use of the legacy parser: an E64 v1/v2 codec-3 container is accepted by the normal checked reader. Conversely, changing the extension or magic does not repair old chunks.

## Migration

1. Identify the actual magic, version, header, mode and codec; apply explicit resource limits before allocation.
2. Admit legacy prefixes only through the migration API, retaining their reduced provenance assurance.
3. Validate actual records and any independent trusted commitments, append full digest trailers, select codec 3 and zero the obsolete prefix.
4. Recalculate lengths, offsets and directory CRCs. If canonicalizing the spelling to M64, replace the magic and regenerate the header CRC; all other semantic record content may remain unchanged.
5. Validate the rewritten object with the current default checked reader. Regenerate enclosing bundle entry digests and external signatures when applicable.

Do not zero-extend the old 12 bytes, preserve them as a claimed full digest, or silently fall back to legacy admission after current verification fails. A separate migration step must supply any information absent from old georaster files; renaming E64 to M64 does not create GRM3 metadata.

## Example and compatibility tests

A one-record v2 full-hash object can use header [0,64), directory [64,128), record [128,192), digest [192,224) and zero padding [224,256). The directory stores count 1, stored length 128 and codec 3; the header stores total length 256. It differs from the corresponding M64 object in magic and therefore header CRC, not in record or trailer interpretation.

Tests should accept current E64 v1/v2 full-hash objects, reject E64 GRM3, reject unsupported codecs and default legacy admission, detect mutations in the last digest byte and reserved trailer padding, and validate round-trip canonicalization. Executed evidence and limitations are recorded in [conformance](file-format-conformance.md).

## Source ownership

E64 shares the [M64 layout](../../../crates/qualia-core-db/src/q42/m64_matter/layout.rs), [checked reader](../../../crates/qualia-core-db/src/q42/m64_matter/checked_reader.rs) and [shared chunk integrity implementation](../../../crates/qualia-core-db/src/q42/chunk_integrity/mod.rs). There is no separate E64 implementation implied by this compatibility standard.
