# `.10d` QuantizedMesh numeric payload companion

**Scope:** the numeric payload used by `SectionType::QuantizedMesh` in the current Core implementation. This companion records the implemented mini-header and coordinate decode contract. It does not replace the Q42 coordinate standard or claim to replace the historical `docs/plans/native-computational-geometry.md`, which is not present in this checkout.

The [Q42 10D tensor standard](q42-10d-tensor-standard.md) defines tensor coordinates and their meanings. This document covers only the QuantizedMesh section payload's numeric frame and the enclosing structural checks needed to interpret it. The container header and section-table envelope remain governed by the existing implementation.

## Payload layout

The container section descriptor identifies this payload as `QuantizedMesh` (section type `1`). A payload begins with a fixed 40-byte `#[repr(C)]` `MeshMiniHeader`, followed by quantized vertex coordinates, then triangle indices. The current implementation's fields and offsets are:

| Offset | Bytes | Field | Implemented meaning |
|---:|---:|---|---|
| 0 | 2 | `flags: u16` | Bit 0 (`0x0001`) means triangle indices are `u16`; zero means `u32`. Other bits are rejected. |
| 2 | 2 | `reserved_u16: u16` | Must be zero. |
| 4 | 4 | `vertex_count: u32` | Number of vertices; decoder limit is 4,194,304. |
| 8 | 4 | `triangle_count: u32` | Number of triangles; decoder limit is 4,194,304. |
| 12 | 12 | `min: [f32; 3]` | Inclusive per-axis lower endpoint of the dequantization frame. |
| 24 | 12 | `max: [f32; 3]` | Inclusive per-axis upper endpoint of the dequantization frame. |
| 36 | 4 | `reserved_u32: u32` | Must be zero. |

The vertex array has `vertex_count` entries, each three `u16` coordinates (6 bytes per vertex). The triangle array has three indices per triangle: 6 bytes per triangle when flag bit 0 is set, otherwise 12 bytes. The section length must cover the 40-byte header and both arrays for their declared counts. In the current implementation the mini-header is read/written as a POD byte representation; coordinate and index values are explicitly converted to little-endian bytes. This records the current implementation and does not claim cross-endian qualification.

## Quantized coordinate decode

For each axis, the stored bounds must both be finite and ordered (`min <= max`). A non-finite endpoint or `min > max` is rejected by `MeshSectionError::InvalidBounds { axis }`; the axis index is zero-based. Endpoint rules take precedence over the zero-span branch: `q == 0` returns `min` bit-for-bit and `q == 65535` returns `max` bit-for-bit, including when the numeric endpoints compare equal but have different signed-zero bits. For an interior `q`, if `min == max`, the decoder returns the stored `min`.

Let `q` be one stored unsigned 16-bit coordinate and `M = 65535`. The decoder uses the exact stored endpoint values for the endpoints:

- `q == 0` returns the stored `min` value bit-for-bit.
- `q == 65535` returns the stored `max` value bit-for-bit.
- Otherwise, if the span is nonzero, the reference is `min + (q / 65535) * (max - min)`, evaluated in `f64` using the stored `f32` endpoints, then rounded once to `f32`.

This avoids computing the span in `f32`, which can overflow for finite endpoints or round in a way that moves a decoded endpoint outside its stored frame. The resulting coordinates are expected to be finite, monotonic as `q` increases, and within the inclusive declared bounds. Regression coverage includes the observed endpoint case, asymmetric and negative intervals, zero-span axes, and large finite decoder frames. Those tests do not qualify extreme-frame encoding.

## Validation layers

The section-table parser checks the outer descriptor structure (type, alignment, reserved bits, offsets/lengths, stride/count consistency, overlap and bounds) and verifies the descriptor's CRC-32C over the complete section payload. The mesh parser then validates its own 40-byte header, reserved fields, flags, counts, finite ordered bounds and required payload length. CRC integrity and mesh structural/numeric validity are separate gates: a correct CRC does not make a malformed mesh header valid, and a mesh parser call alone does not verify the enclosing section CRC.

## Compatibility and limits

The correction changes decoder arithmetic and rejects non-finite or reversed bounds that were previously accepted. For affected interior coordinates, decoded `f32` values may differ from the former `f32` interpolation by an ULP; `q=0` and `q=65535` now preserve the exact stored endpoint bits. The serialized mini-header layout, encoder, quantizer and output bytes are unchanged. No format/container version was bumped for this decode correction.

The encoder still computes its bounding-box extent and quantization arithmetic in `f32`. This document does not claim that wide-span encoding is fixed or qualified. Whole-format conformance, every producer/consumer, GPU behavior, cross-endian behavior and CI enforcement are outside this bounded numeric companion.

## Evidence

- [Public decoder regression test](../../../crates/qualia-core-db/tests/mesh_section_bounds.rs) contains exactly four regression tests for the measured endpoint case, asymmetric/negative/zero-span frames, large finite decoder frames, and malformed bounds/header errors. The current [mesh-section decoder source](../../../crates/qualia-core-db/src/container_10d/mesh_section.rs) performs per-axis finite ordered-bound validation and the endpoint/interior decode above. Root ran these four tests through the Poet native conformance test target and its Core dependency closure: 4 passed, 0 failed. This is not qualification of the full Core suite or all target platforms.
- [Round 1 numeric probe](../../../../ecopunk/docs/qa/implementation-2026-10-25/batch-6/rounds/round1/producer/decoded-bounds-probe.json) records the producer's observed one-ULP endpoint overshoot; it is not an independent measurement.
- [Independent codec source review](../../../../ecopunk/docs/qa/implementation-2026-10-25/batch-6/rounds/round1/review/mesh-codec-review.md) reviewed the final patch, not its runtime output or visual quality.
- [Current Core codec test log](../../../../ecopunk/docs/qa/implementation-2026-10-25/batch-6/rounds/round1/codec-tests.log) is the execution evidence; consult it for the result rather than inferring success from the test source or review.
- [Proposed `.10d` evolution roadmap](../../../../ecopunk/docs/planning/tech/10d-container-evolution.md) describes staged ways to improve the format and its native tooling; it is a proposal, not a released version or current qualification.

This implementation companion does not establish successful native relink/reopen/render, a PNG, human acceptance, high-grade model completion, or IP-03 C1–C8 acceptance.

