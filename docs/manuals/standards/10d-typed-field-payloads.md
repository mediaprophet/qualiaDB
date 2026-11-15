# `.10d` typed field payload and SDK companion

**Status:** source-grounded draft; not a new canonical standard. Installed Core SDK behavior is described below. Selected native conformance passed on Windows x86_64 little-endian: 9 direct SDK cases within a 22-case aggregate (7 M64, 9 typed-payload and 6 consumer cases), with zero failures. The execution is recorded in [the Round 2 log](../../../../ecopunk/docs/qa/implementation-2026-10-25/batch-6/rounds/round2/native-conformance.log). Existing source and authored fixtures do not by themselves pass FND-03 or any product gate.

## Outer container and registration model

- `.10d` outer header versions 1 and 2 are supported; current writers emit v2. `MID2` is a registered section only in v2. The encoder/parser reject a `ManifoldIdentityV2` section under outer v1; no outer-version bump is required for the registered MID2 field-kind additions below.
- When present, the outer section table permits at most one `FieldSidecar` entry (section type 8) and one `ManifoldIdentityV2` entry (section type 15); the combined-payload path uses both. Canonical encoding sorts distinct section types, rejects duplicate outer types, validates section ranges/alignment/CRC, and retains the existing parser contract.
- `FIELD_KIND_SURFACE_FRAMES` (`0x0200`) and `FIELD_KIND_MORPH_TARGETS` (`0x0201`) are MID2 `TypedFieldRecord.kind` values, not outer `SectionType` values or separate payload formats. Their codec bytes and scientific-field payload ranges can occupy nonoverlapping slices of the single `FieldSidecar` section.

## MID2 manifold manifest (`ManifoldIdentityV2`)

The v2 manifest has a 112-byte header and up to 256 fixed 48-byte field records (12,400 bytes maximum). Header offsets: `MID2` magic `[0..4]`; version 2 `[4..6]`; header size `[6..8]`; total length `[8..12]`; field count `[12..14]`; coordinate count `[14]`; flags `[15]`; stable entity identifier bytes `[16..32]`; profile, domain and coordinate-convention identifiers `[32..56]`; ten `f32` coordinates `[56..96]`; optional external digest/index hint `[96..112]`. Coordinate order is `[q, v, w, x, y, z, t, α, μ, σ]`.

Each 48-byte `TypedFieldRecord` stores `field_id` at `[0..8]`, `kind` `[8..10]`, `flags` `[10..12]`, `unit_id` `[12..20]`, `payload_section_id` `[20..24]`, reserved zero `[24..28]`, `payload_offset` `[28..36]`, `payload_length` `[36..44]`, and reserved zero `[44..48]`. IDs are nonzero, unique and strictly ascending. A populated range names a real outer section and uses checked offset-plus-length arithmetic; empty declarations use a zero section identifier and zero offset/length. `StableEntityId` is an identifier value, not proof of issuer authority. An external digest is not the entity identifier. `unit_id` is distinct from legacy FIEL `unit_hash`.

## Typed payloads within FieldSidecar

| MID2 kind | Existing payload | Binding and reader behavior |
|---|---|---|
| `FIELD_KIND_SURFACE_FRAMES` (`0x0200`) | `NRM1` normals or `FRM1` interleaved normals/tangents. NRM1 uses a 16-byte header plus 4 bytes per vertex; FRM1 uses the 16-byte header plus 4 bytes per normal and/or 8 bytes per tangent per vertex. | Unique kind; `payload_section_id` is FieldSidecar and `unit_id` is zero. The reader checks declared/decoded vertex count. A generic `Vector` field is never guessed to be normals. |
| `FIELD_KIND_MORPH_TARGETS` (`0x0201`) | `MRP1`: 32-byte header and 16-byte quantized delta records; current limits are 256 targets and 1,048,576 deltas. | Unique kind; FieldSidecar, `unit_id=0`. Position deltas use the mesh coordinate convention; normal/tangent deltas are dimensionless. Reader checks vertex count and validates deltas before publishing output. |
| Scientific kinds such as scalar/vector/tensor/spectral/EMF | FIEL v0 descriptors plus raster metadata and `f32` sample arrays where present. Public codecs include `encode_field_section_with_rasters`, `decode_field_section`, and `decode_field_raster_samples`. | Preserve the descriptor's `name_hash`, `unit_hash`, field type/support/representation and sample range. Preserve MID2 `unit_id` separately; do not infer equality, convert units, or discard scientific samples when adding frames/morphs. |

FIEL v0's 12-byte header and 32-byte descriptors coexist with raster encoding APIs. Earlier Rustdoc claiming that v0 contains no sample bytes is stale relative to these APIs and must not be used as capability evidence. Raster grid metadata is 16 bytes (`width`, `height`, `sample_count`, reserved), followed by little-endian `f32` samples. Do not infer equivalence between a legacy FIEL unit hash and a MID2 registry identifier without an explicit qualified mapping.

## Resolution, legacy paths and refusal behavior

`TypedPayloadResolver::new` parses the outer header and canonical section table, then validates the MID2 manifest and ranges before changing caller `out_fields`; caller scratch may change on error. Selection returns borrowed byte ranges. `surface_frames()` and `morph_targets()` require a unique matching typed kind and valid section/unit binding. Generic fields are selected by `field_id`; unknown extension kinds fail closed unless explicitly allowed by the decoder.

When MID2 is absent, the existing readers may recognize a whole-FieldSidecar legacy NRM1/FRM1 or MRP1 payload. When MID2 is present, an absent registered kind means no such payload; the reader does not guess from unrelated field bytes or magic. `read_container_morph_targets_into` checks the declared vertex count, and MRP1 decode validates/preflights its records and output capacity before publishing caller output. Preserve these transactional refusal cases.

Older MID2 readers without the `0x0200`/`0x0201` kind registrations reject those records as unsupported kinds; they must not reinterpret or silently skip them. The outer v2 version and 112/48-byte MID2 layout remain unchanged. HMC/Q42 graph knowledge, resource identifiers, provenance and whole-embodiment coverage remain under the HMC owner and are not replaced by this sidecar map.

## Source references and evidence boundary

- Outer table and version policy: `C:/github/qualiaDB/crates/qualia-core-db/src/container_10d/{header.rs,section.rs,section/table_validation.rs}`.
- MID2 constants, record layout and validation: `C:/github/qualiaDB/crates/qualia-core-db/src/container_10d/manifold_identity_v2.rs`.
- Typed selection and high-level readers: `C:/github/qualiaDB/crates/qualia-core-db/src/container_10d/typed_payloads/{mod.rs,resolver.rs,readers.rs}`.
- Existing codec layouts: `C:/github/qualiaDB/crates/qualia-core-db/src/container_10d/{normal_section.rs,morph_section.rs,field_section.rs}`.
- Authored SDK cases: `C:/github/qualiaDB/crates/qualia-core-db/tests/typed_field_payloads.rs`. The source contains coexistence, legacy fallback, unsupported-kind, count and transactional-output cases; the nine direct cases passed in the recorded native run in [native-conformance.log](../../../../ecopunk/docs/qa/implementation-2026-10-25/batch-6/rounds/round2/native-conformance.log).

## Bounded mesh-frame decode API (Round 3 candidate)

This paragraph specifies the staged Round 3 API; no Round 3 execution pass is claimed here. The NRM1/FRM1 wire layout, codecs, version, accepted oct16 values, MID2 kind registration and legacy/MID2 selection policy are unchanged. Header decoding reads little-endian bytes directly and accepts unaligned codec ranges. It does not require reinterpreting a byte-aligned temporary as an aligned Rust header.

`MeshFrameDecodeLimits` supplies independent maximum vertex count, encoded payload bytes and decoded stream bytes. The existing global `MAX_NORMAL_COUNT` cap remains effective even if the caller requests more. Decoded storage is `count * (12 if normals + 16 if tangents)`, with arithmetic bounded by that global cap on both native and wasm32. The limits cover selected frame payload/output storage, not the whole envelope, mesh, renderer, GPU storage, other HMC resources or allocator bookkeeping.

`inspect_mesh_frames(bytes, limits)` performs full header/version/flags/reserved/count/length/limits validation and validates every tangent direction and handedness before returning `MeshFrameLayout`. It allocates no heap memory. All signed oct16 normal pairs retain their established decoding semantics. A zero-count active stream is valid; zero stream flags remain invalid.

`decode_mesh_frames_into(bytes, limits, out_normals, out_tangents)` uses that complete preflight and checks both active output capacities before changing either output. It writes exactly the active `count` prefix; absent streams and output tails remain unchanged. Present streams require sufficient storage even when the caller only consumes the other stream. Every validation, budget and capacity refusal leaves both caller buffers unchanged. The call performs no allocation; tangents are validated again only as infallible decoded output from the same immutable input. There is no target-specific alternative decoder.

`decode_mesh_frames_with_limits` is an explicitly cold convenience wrapper. Full preflight occurs before any allocation; fallible exact reservation for active streams follows the decoded-byte cap. Allocation refusal returns `AllocationFailed`; no partially constructed output is returned. New limit errors are `PayloadTooLarge` and `DecodedTooLarge`; the existing `CountTooLarge` also enforces the caller vertex cap. The existing `decode_mesh_frames`, `decode_normals`, `decode_tangents` and `read_container_mesh_frames` signatures remain. They route through the canonical decoder under `MeshFrameDecodeLimits::COMPATIBILITY`, whose maxima admit all historically valid frame counts: 4,194,304 vertices, 50,331,664 encoded bytes, and 117,440,512 decoded bytes.

The typed SDK adds `inspect_container_mesh_frames`, `read_container_mesh_frames_into` and `read_container_mesh_frames_with_limits`. They retain the canonical resolver and expected-vertex-count refusal. MID2 absence of SurfaceFrames still returns `None`; a generic Vector declaration never selects frames. Legacy whole-sidecar NRM1/FRM1 remains supported when MID2 is absent. Caller output is unchanged on absence and every refusal, including envelope/manifest failure and late invalid tangents.

The existing render compiler compatibility projection delegates to the bounded SDK with the compatibility policy. Native scene mesh loading and Portal mesh upload select explicit 64 MiB encoded / 256 MiB decoded frame budgets and the global vertex cap. These are per-frame-stream cold allocation budgets, not a claim that the whole existing scene loader is allocation-free or globally budgeted. `tests/frame_decode_sdk.rs` adds focused synthetic-byte conformance and raw allocation-call measurement for successful and refused caller-buffered paths, including typed selection. Native execution and WASM compilation/interop qualification must be recorded separately after integration.

Decomposition: canonical frame admission/decoding now lives in focused `normal_section/frame_decode.rs`; typed frame selection/adapters live in `typed_payloads/frames.rs`. The former allocating decoder bodies were removed from `normal_section.rs` and route to this implementation. Oversized legacy render files receive only thin SDK call changes. Their broader decomposition remains open under the existing SDK ownership programme; no unrelated implementation is added here.