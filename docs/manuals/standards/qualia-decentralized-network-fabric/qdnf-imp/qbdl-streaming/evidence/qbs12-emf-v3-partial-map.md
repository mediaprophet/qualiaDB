# QBS-12 EMF v3 partial-read source map

Read-only source review, 2026-11-13. This records current codec boundaries and a minimal partial-read seam; it does not claim that such an adapter exists.

## Canonical EMF v3 path

- `src/q42/emf_container/reader.rs:71-75` exposes `parse_with_extension_limits`, but takes the complete file as one `&[u8]`. `parse_policy` (`:75-212`) validates the whole header and directory, every row CRC and extent, and every chunk full-hash trailer before publishing `EmfContainerReader`. It also parses/validates the v3 extension and field-directory closure. `parse_legacy_with_limits` is explicitly migration-only and must not be used by a new v3 path.
- Version 3 and `EMF_CHUNK_FLAG_BLAKE3_FULL` are declared in `src/q42/emf_container/header.rs:15-18`. A v3 chunk has `cell_count * 64` record bytes followed by `BLAKE3_TRAILER_BYTES`, which is **64 bytes** (`src/q42/chunk_integrity/mod.rs:7-18`). The trailer carries the full 32-byte BLAKE3 digest in its defined 64-byte framing. Never shorten it to 32 bytes.
- `EmfChunkDirectoryEntry::validate` (`src/q42/emf_container/chunk_directory.rs:79-90`) checks the CRC-32C over row bytes 0..60. It does not on its own validate v3 flags, expected payload size, extent/non-overlap, or digest trailer.
- `EmfContainerReader::chunk_payload` (`reader.rs:243-249`) returns only record bytes, not the trailer. `cell` is safe after the complete-file parse has verified all chunk trailers. A partial view must verify the selected chunk’s entire records plus 64-byte trailer before exposing records/cells.

## Metadata and interpretation closure

- `extension/codec.rs:123` has `parse_extension`, currently `pub(super)` and requiring the complete file slice. It validates the 64-byte `Q42EMFX3` footer (schema/reserved/footer CRC), explicit file/metadata/audio/track limits, absolute field-end and aligned JSON/PCM extents, and exact total file length. The extension commitment is a complete 64-byte BLAKE3 trailer over JSON, alignment padding, and PCM. It then parses bounded JSON, checks schema and field interpretation, validates each track’s channel/frame/byte arithmetic, contiguous aligned PCM offsets and zero padding, full 32-byte per-track BLAKE3, timing, finite samples and normalized-amplitude limits, and exact PCM exhaustion.
- `validate_field_directory` (`extension/codec.rs:165-185`, also `pub(super)`) checks v3 field/header consistency, fixed directory offset 64, canonical contiguous chunk placement through `field_end`, full-hash flag encoding, cell-count bounds, sorted Hilbert keys, unique `(Hilbert,time)` coordinates, Hilbert/grid round-trip, and timebase slice arithmetic. It assumes the reader already validated row CRCs, payload extents/overlap, expected record-plus-trailer lengths, and every actual chunk trailer. Calling it alone does not replace those checks.
- Existing v3 coverage is in `src/q42/emf_container/extension/tests.rs`: round-trip of v2 cells plus exact PCM bits/time/interpretation; declared Hilbert order and time selection; all 64 extension-trailer-byte mutations and PCM tampering; malformed metadata, overlap, nonfinite samples, padding and explicit limits; large epoch and field-boundary checks.

## Smallest partial adapter seam

Use actual selected-entry `AuthenticatedRange` bytes for the 64-byte header, the entire bounded `chunk_count * 64` directory, the complete v3 extension tail (metadata, padding, PCM, 64-byte extension trailer and footer), and only the requested complete chunk including its 64-byte trailer. Do not fetch unrelated brick payloads. The complete directory and extension are needed to validate global ordering, occupancy uniqueness, and interpretation/time closure; the selected digest covers the full chunk, so a record-only partial chunk is insufficient.

Minimal source changes before implementation:

1. Factor/expose a cold extension parser that accepts `field_end`, the exact contiguous extension-tail bytes, and `EmfExtensionLimits`. The current parser’s absolute offsets and whole-file `bytes.len()` requirement prevent directly passing only the tail; do not fabricate zero-filled holes and call it a whole-file parse.
2. Factor the existing v3 directory checks so the partial path can pass the validated header, exact complete directory bytes, absolute directory offset, total source length, and parsed extension metadata. Reuse `EmfChunkDirectoryEntry::validate` for each row and retain the reader’s checked extent, overlap, exact `cell_count * 64 + 64`, full-hash flag/reserved-field and full trailer checks. `validate_field_directory` currently assumes those previous checks and a whole-file byte slice.
3. For an individual requested row, validate the row CRC and exact identity/extent against the admitted directory, fetch all records plus the complete 64-byte trailer, and call `verify_blake3_trailer` before exposing the records. This establishes selected-brick integrity only, not validation of unrelated brick contents or dense-volume completeness.

## `.10d` typed-field boundary

- `FIELD_KIND_ELECTROMAGNETIC = 0x0100` is a MID2 kind (`src/container_10d/manifold_identity_v2.rs:49`); a `TypedFieldRecord` refers to a stable `SectionType` and byte range. `decode_container_manifold_identity` (`manifold_identity_container.rs:137`) and `TypedPayloadResolver::new` (`typed_payloads/resolver.rs:47`) consume a complete `.10d` byte slice and full section table. The resolver has no EMF-specific accessor; its unique-kind helpers cover surface frames and morph targets.
- FIEL v0 `inspect_field_section` (`field_section/checked.rs:55`) validates generic FIEL declarations and contiguous Scalar/Region/Grid float samples; it is not the EMF v3 or PCM codec. MID2 unit IDs and FIEL unit hashes are distinct fields; no equivalence should be inferred. Neither MID2’s electromagnetic kind nor FIEL by itself encodes a qualified sibling `.emf` locator.

Partial-reader tests should prove exact range requests for header, complete directory, complete extension tail, and one complete selected chunk; refuse missing/truncated extension or chunk trailer bytes, invalid row CRC/flags/extents, bad Hilbert/time metadata, and v1 legacy input; and show unrelated chunk payloads are not fetched. Existing full-reader v3 tests remain the oracle for exact metadata/PCM semantics.
