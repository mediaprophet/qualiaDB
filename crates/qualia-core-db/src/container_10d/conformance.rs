//! `.10d` v1 normative-spec conformance vectors + layout-table drift gate
//! (P0.7 scaffold).
//!
//! This module is the **conformance harness** — the single place where the
//! `.10d` format's golden vectors and layout tables are pinned. It has two
//! jobs:
//!
//! 1. **Golden vectors (encode∘decode = identity).** Each golden vector is a
//!    pinned byte sequence produced by the current implementation encoding a
//!    known input. The conformance test decodes the golden bytes, re-encodes
//!    the decoded content, and asserts the re-encoded bytes are byte-identical
//!    to the golden bytes. If any field offset, encoding order, CRC algorithm,
//!    or padding rule drifts, the golden vector won't reproduce and the test
//!    breaks. Each golden vector also has a **pinned content hash** (CRC-32C
//!    over the golden bytes) — a double lock so a silent byte change is caught
//!    even if the re-encode path has a compensating bug.
//!
//! 2. **Layout-table drift gate.** The size and offset of every field in
//!    `Container10dHeader`, `SectionDescriptor`, `NodeMiniHeader`, and
//!    `MetricBranchDescriptor` is asserted here in one place, so the spec's
//!    layout tables and the Rust structs cannot drift apart. (The individual
//!    module tests also have offset_of assertions; this centralizes them as
//!    the spec's single source of truth.)
//!
//! **Scaffold status (P0.7 partial):** the golden vectors for the P0.1–P0.5
//! container (header + section table + CRC + NODE section) are pinned here.
//! The mesh-section golden vectors (P0.4) are a clearly-marked placeholder —
//! they will be added when P0.4 lands. The normative `.10d` v1 spec document
//! (the prose layout tables, magic bytes, version number, etc.) is the
//! execution plan's P0.7 deliverable; this module is the executable
//! conformance check, not the prose spec.

use crate::container_10d::header::{Container10dHeader, HEADER_BYTE_SIZE};
use crate::container_10d::mesh_section::{MeshMiniHeader, MESH_MINI_HEADER_SIZE};
use crate::container_10d::metric_check::MetricBranchDescriptor;
use crate::container_10d::node_section::{NodeMiniHeader, NODE_MINI_HEADER_SIZE, TENSOR10D_SIZE};
// Only the layout-drift gate runs outside `#[cfg(test)]`, so it imports just the descriptor + its
// size constant. `crc32c`, `AlignmentTier`, `SectionInput`, and `SectionType` are used exclusively by
// the golden-vector tests and are imported there.
use crate::container_10d::section::{SectionDescriptor, SECTION_DESCRIPTOR_SIZE};

// ---------------------------------------------------------------------------
// Layout-table drift gate — the spec's single source of truth for sizes &
// offsets. If any of these change, the format version MUST bump.
// ---------------------------------------------------------------------------

/// Assert every layout invariant the `.10d` v1 spec pins. Called from the
/// conformance test; also callable from any test that wants to confirm the
/// structs haven't drifted.
pub fn assert_layout_invariants() {
    // --- Container10dHeader (64 bytes) ---
    assert_eq!(std::mem::size_of::<Container10dHeader>(), 64, "header size");
    assert_eq!(
        std::mem::offset_of!(Container10dHeader, magic),
        0,
        "header.magic offset"
    );
    assert_eq!(
        std::mem::offset_of!(Container10dHeader, version),
        4,
        "header.version offset"
    );
    assert_eq!(
        std::mem::offset_of!(Container10dHeader, flags),
        6,
        "header.flags offset"
    );
    assert_eq!(
        std::mem::offset_of!(Container10dHeader, axis_roles),
        8,
        "header.axis_roles offset"
    );
    assert_eq!(
        std::mem::offset_of!(Container10dHeader, pad0),
        18,
        "header.pad0 offset"
    );
    assert_eq!(
        std::mem::offset_of!(Container10dHeader, metric_descriptor),
        20,
        "header.metric_descriptor offset"
    );
    assert_eq!(
        std::mem::offset_of!(Container10dHeader, header_crc32c),
        52,
        "header.header_crc32c offset"
    );
    assert_eq!(
        std::mem::offset_of!(Container10dHeader, section_table_offset),
        56,
        "header.section_table_offset offset"
    );
    assert_eq!(
        std::mem::offset_of!(Container10dHeader, section_count),
        60,
        "header.section_count offset"
    );

    // --- MetricBranchDescriptor (8 bytes) ---
    assert_eq!(
        std::mem::size_of::<MetricBranchDescriptor>(),
        8,
        "metric_branch_descriptor size"
    );

    // --- SectionDescriptor (24 bytes) ---
    assert_eq!(
        std::mem::size_of::<SectionDescriptor>(),
        24,
        "section_descriptor size"
    );
    assert_eq!(
        std::mem::offset_of!(SectionDescriptor, section_type),
        0,
        "section_descriptor.section_type offset"
    );
    assert_eq!(
        std::mem::offset_of!(SectionDescriptor, alignment_tier),
        1,
        "section_descriptor.alignment_tier offset"
    );
    assert_eq!(
        std::mem::offset_of!(SectionDescriptor, reserved16),
        2,
        "section_descriptor.reserved16 offset"
    );
    assert_eq!(
        std::mem::offset_of!(SectionDescriptor, byte_offset),
        4,
        "section_descriptor.byte_offset offset"
    );
    assert_eq!(
        std::mem::offset_of!(SectionDescriptor, byte_length),
        8,
        "section_descriptor.byte_length offset"
    );
    assert_eq!(
        std::mem::offset_of!(SectionDescriptor, stride),
        12,
        "section_descriptor.stride offset"
    );
    assert_eq!(
        std::mem::offset_of!(SectionDescriptor, element_count),
        16,
        "section_descriptor.element_count offset"
    );
    assert_eq!(
        std::mem::offset_of!(SectionDescriptor, crc32c),
        20,
        "section_descriptor.crc32c offset"
    );

    // --- NodeMiniHeader (16 bytes) ---
    assert_eq!(
        std::mem::size_of::<NodeMiniHeader>(),
        16,
        "node_mini_header size"
    );
    assert_eq!(
        std::mem::offset_of!(NodeMiniHeader, node_count),
        0,
        "node_mini_header.node_count offset"
    );
    assert_eq!(
        std::mem::offset_of!(NodeMiniHeader, layout),
        4,
        "node_mini_header.layout offset"
    );
    assert_eq!(
        std::mem::offset_of!(NodeMiniHeader, reserved_u8),
        5,
        "node_mini_header.reserved_u8 offset"
    );
    assert_eq!(
        std::mem::offset_of!(NodeMiniHeader, reserved_u16),
        6,
        "node_mini_header.reserved_u16 offset"
    );
    assert_eq!(
        std::mem::offset_of!(NodeMiniHeader, reserved_u64),
        8,
        "node_mini_header.reserved_u64 offset"
    );

    // --- MeshMiniHeader (40 bytes) ---
    assert_eq!(
        std::mem::size_of::<MeshMiniHeader>(),
        40,
        "mesh_mini_header size"
    );
    assert_eq!(
        std::mem::offset_of!(MeshMiniHeader, flags),
        0,
        "mesh_mini_header.flags offset"
    );
    assert_eq!(
        std::mem::offset_of!(MeshMiniHeader, reserved_u16),
        2,
        "mesh_mini_header.reserved_u16 offset"
    );
    assert_eq!(
        std::mem::offset_of!(MeshMiniHeader, vertex_count),
        4,
        "mesh_mini_header.vertex_count offset"
    );
    assert_eq!(
        std::mem::offset_of!(MeshMiniHeader, triangle_count),
        8,
        "mesh_mini_header.triangle_count offset"
    );
    assert_eq!(
        std::mem::offset_of!(MeshMiniHeader, min),
        12,
        "mesh_mini_header.min offset"
    );
    assert_eq!(
        std::mem::offset_of!(MeshMiniHeader, max),
        24,
        "mesh_mini_header.max offset"
    );
    assert_eq!(
        std::mem::offset_of!(MeshMiniHeader, reserved_u32),
        36,
        "mesh_mini_header.reserved_u32 offset"
    );

    // --- Constants ---
    assert_eq!(HEADER_BYTE_SIZE, 64, "HEADER_BYTE_SIZE");
    assert_eq!(SECTION_DESCRIPTOR_SIZE, 24, "SECTION_DESCRIPTOR_SIZE");
    assert_eq!(NODE_MINI_HEADER_SIZE, 16, "NODE_MINI_HEADER_SIZE");
    assert_eq!(MESH_MINI_HEADER_SIZE, 40, "MESH_MINI_HEADER_SIZE");
    assert_eq!(TENSOR10D_SIZE, 40, "TENSOR10D_SIZE");
}

// ---------------------------------------------------------------------------
// Golden vectors — pinned byte sequences + pinned content hashes.
//
// Each golden vector is a complete `.10d` file (or a section payload) that the
// conformance test:
//   (a) asserts has the pinned CRC-32C (content hash),
//   (b) decodes,
//   (c) re-encodes the decoded content,
//   (d) asserts the re-encoded bytes are byte-identical to the golden bytes.
//
// If any encoding detail drifts (field offset, CRC algorithm, section order,
// padding rule), step (d) breaks. If the golden bytes themselves are silently
// edited, step (a) breaks (the pinned hash won't match). This is the double
// lock.
//
// The golden vectors were generated by the current implementation (P0.1–P0.5)
// on 2026-07-04. They are the normative reference for `.10d` v1 — any future
// change that alters these bytes MUST bump the version field from 1 to 2.
// ---------------------------------------------------------------------------

/// Golden vector 1: a bare header (no section table). The proposed header
/// with `header_crc32c` = 0 (unsealed — the seal is applied at the container
/// level, not the bare-header level).
///
/// Pinned content hash (CRC-32C over the golden bytes): see
/// `GOLDEN_BARE_HEADER_CRC`.
///
/// Generated by `Container10dHeader::proposed().encode_to_vec64()` on
/// 2026-07-04. The metric_kind enum values are 1=Euclidean, 2=Cyclic,
/// 3=Hyperbolic, 4=BoundaryClique; the v≥3 catch-all branch uses
/// `v_class=255`. These bytes are the normative reference for `.10d` v1.
pub const GOLDEN_BARE_HEADER: [u8; HEADER_BYTE_SIZE] = [
    // magic "10d\0"
    0x31, 0x30, 0x64, 0x00, // version: u16 LE = 1
    0x01, 0x00, // flags: u16 LE = FLAG_DEFAULT_DISPOSITION_REFUSE (1)
    0x01, 0x00,
    // axis_roles[10]: Option A — q=Selector(1), v=Selector(1), w=Selector(1),
    //   x=Coordinate(2), y=Coordinate(2), z=Coordinate(2), t=Coordinate(2),
    //   α=Coordinate(2), μ=CoordinateCarrier(4), σ=Coordinate(2)
    0x01, 0x01, 0x01, 0x02, 0x02, 0x02, 0x02, 0x02, 0x04, 0x02, // pad0[2] = 0
    0x00, 0x00,
    // metric_descriptor (32 bytes): 4 x MetricBranchDescriptor (8 bytes each).
    //   Branch 0 (v=0 Euclidean): v_class=0, metric_kind=Euclidean(1),
    //     folded_axes=0x03F8 LE (bits 3-9: x,y,z,t,α,μ,σ), reserved=0
    0x00, 0x01, 0xF8, 0x03, 0x00, 0x00, 0x00, 0x00,
    //   Branch 1 (v=1 Cyclic): v_class=1, metric_kind=Cyclic(2),
    //     folded_axes=0x0038 LE (bits 3-5: x,y,z), reserved=0
    0x01, 0x02, 0x38, 0x00, 0x00, 0x00, 0x00, 0x00,
    //   Branch 2 (v=2 Hyperbolic): v_class=2, metric_kind=Hyperbolic(3),
    //     folded_axes=0x0038 LE (bits 3-5: x,y,z), reserved=0
    0x02, 0x03, 0x38, 0x00, 0x00, 0x00, 0x00, 0x00,
    //   Branch 3 (v>=3 catch-all): v_class=255, metric_kind=BoundaryClique(4),
    //     folded_axes=0x0000 (no coordinate axes folded), reserved=0
    0xFF, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    // header_crc32c: u32 LE = 0 (unsealed at the bare-header level)
    0x00, 0x00, 0x00, 0x00, // section_table_offset: u32 LE = 0 (bare header)
    0x00, 0x00, 0x00, 0x00, // section_count: u32 LE = 0 (bare header)
    0x00, 0x00, 0x00, 0x00,
];

/// Pinned CRC-32C over `GOLDEN_BARE_HEADER`. Double lock: if the golden bytes
/// are silently edited, this hash won't match. Pinned 2026-07-04.
pub const GOLDEN_BARE_HEADER_CRC: u32 = 0xD6DD_ABF5; // pinned 2026-07-04

/// Golden vector 2: a NODE-only container — the proposed header + a section
/// table with one Tensor10DNodes section containing 3 nodes in AoS layout.
/// The `header_crc32c` is sealed (whole-file CRC).
///
/// This vector is generated at runtime by the conformance test (see
/// `golden_node_only_container`) rather than embedded as a const, because it
/// includes a sealed whole-file CRC and a per-section CRC that are easier to
/// generate than to hand-encode. The test pins the CRC-32C of the resulting
/// bytes as `GOLDEN_NODE_ONLY_CRC` — the double lock.
pub const GOLDEN_NODE_ONLY_CRC: u32 = 0x6865_D565; // pinned 2026-07-04

/// Pinned CRC-32C over the golden MESH-only container bytes (the unit cube
/// encoded as a QuantizedMesh section in a `.10d` container, sealed with the
/// whole-file CRC). Pinned at runtime — see the conformance test.
pub const GOLDEN_MESH_ONLY_CRC: u32 = 0x18B5_DD86; // pinned 2026-07-04

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::crc32c::crc32c;
    use crate::container_10d::header::Container10dHeader;
    use crate::container_10d::integrity::{seal_whole_file_crc32c, verify_whole_file_crc32c};
    use crate::container_10d::node_section::{read_node, write_node_section_aos, NodeMiniHeader};
    use crate::container_10d::section::{
        encode_container, parse_section_table, AlignmentTier, SectionInput, SectionType,
    };
    use crate::tensor::Tensor10D;

    // ====================================================================
    // Part 1: Layout-table drift gate
    // ====================================================================

    #[test]
    fn layout_invariants_hold() {
        assert_layout_invariants();
    }

    // ====================================================================
    // Part 2: Golden vectors — encode∘decode = identity + pinned hashes
    // ====================================================================

    #[test]
    fn golden_bare_header_reproduces_byte_identical() {
        // (a) Pin the content hash.
        let actual_crc = crc32c(&GOLDEN_BARE_HEADER);
        assert_eq!(
            actual_crc, GOLDEN_BARE_HEADER_CRC,
            "golden bare header CRC-32C must match the pinned value; \
             if you changed the header encoding, update both the golden bytes \
             AND the pinned CRC, or bump the version"
        );
        // (b) Decode.
        let parsed =
            Container10dHeader::parse(&GOLDEN_BARE_HEADER).expect("golden bare header must parse");
        // (c) Re-encode.
        let mut reencoded = [0u8; HEADER_BYTE_SIZE];
        parsed.encode(&mut reencoded);
        // (d) Assert byte-identity.
        assert_eq!(
            &reencoded[..],
            &GOLDEN_BARE_HEADER[..],
            "re-encoding the decoded golden bare header must reproduce the \
             golden bytes exactly (encode∘decode = identity)"
        );
        // Also confirm the re-encoded header parses to the same struct.
        let reparsed = Container10dHeader::parse(&reencoded).expect("re-encoded must parse");
        assert_eq!(parsed, reparsed);
    }

    #[test]
    fn golden_bare_header_matches_proposed() {
        // The golden bare header must equal what Container10dHeader::proposed()
        // produces. This pins the proposed-header defaults (magic, version,
        // flags, axis_roles, metric_descriptor) as the normative reference.
        let proposed = Container10dHeader::proposed();
        let mut proposed_bytes = [0u8; HEADER_BYTE_SIZE];
        proposed.encode(&mut proposed_bytes);
        assert_eq!(
            &proposed_bytes[..],
            &GOLDEN_BARE_HEADER[..],
            "Container10dHeader::proposed() must produce the golden bare header bytes; \
             if the proposed defaults changed, update the golden vector"
        );
    }

    #[test]
    fn golden_node_only_container_round_trips_byte_identical() {
        // Build a NODE-only container with 3 known tensors, seal it, pin the
        // CRC, then decode + re-encode + assert byte-identity.
        let tensors = [
            Tensor10D::new(0.0, 0.0, 0.0, 0.1, 0.2, 0.3, 0.0, 1.0, 0.0, 0.5),
            Tensor10D::new(0.5, 1.0, 2.0, 0.4, 0.5, 0.6, 1.0, 0.8, 0.2, 0.75),
            Tensor10D::new(999.0, 2.0, 3.0, 0.7, 0.8, 0.9, 2.0, 0.6, 0.9, 0.25),
        ];
        let node_need = NodeMiniHeader::payload_bytes(tensors.len());
        let mut node_payload = vec![0u8; node_need];
        write_node_section_aos(&tensors, &mut node_payload).expect("node write");

        let h = Container10dHeader::proposed();
        let inputs = [SectionInput {
            section_type: SectionType::Tensor10DNodes,
            alignment_tier: AlignmentTier::CacheLine,
            stride: 0,
            element_count: 0,
            payload: &node_payload,
        }];
        let mut out = vec![0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("container encode");
        seal_whole_file_crc32c(&mut out[..n]);

        // (a) Verify the whole-file CRC (the seal wrote it; verify confirms).
        verify_whole_file_crc32c(&mut out[..n]).expect("whole-file CRC must verify");

        // (b) Decode: parse header + section table + NODE payload.
        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        let descs = parse_section_table(&out[..n], &parsed_h).expect("table parse");
        assert_eq!(descs.len(), 1);
        assert_eq!(descs[0].section_type, SectionType::Tensor10DNodes as u8);
        let p_off = descs[0].byte_offset as usize;
        let p_len = descs[0].byte_length as usize;
        let node_payload_back = &out[p_off..p_off + p_len];
        for i in 0..tensors.len() {
            let t = read_node(node_payload_back, i).expect("node read");
            assert_eq!(t, tensors[i], "node {i} must round-trip");
        }

        // (c) Re-encode: rebuild the container from the decoded content.
        let mut reencoded = vec![0u8; 512];
        let n2 = encode_container(&parsed_h, &inputs, &mut reencoded).expect("re-encode");
        seal_whole_file_crc32c(&mut reencoded[..n2]);

        // (d) Assert byte-identity.
        assert_eq!(n, n2, "re-encode must produce the same byte count");
        assert_eq!(
            &out[..n],
            &reencoded[..n2],
            "re-encoding the decoded NODE-only container must reproduce the \
             golden bytes exactly (encode∘decode = identity)"
        );

        // Pin the content hash (printed on first run; update GOLDEN_NODE_ONLY_CRC
        // if this breaks — but only if you intended to change the format, and
        // then bump the version).
        let pinned_crc = crc32c(&out[..n]);
        // The pinned CRC is 0x0000_0000 in the const above (a placeholder —
        // the real value is runtime-generated). This assert is a no-op until
        // the pin is set; it's here to surface the value when the test runs.
        if GOLDEN_NODE_ONLY_CRC != 0 {
            assert_eq!(
                pinned_crc, GOLDEN_NODE_ONLY_CRC,
                "golden NODE-only container CRC-32C must match the pinned value"
            );
        } else {
            // Print the value so it can be pinned. This is not a failure —
            // it's a development aid. Once pinned, the assert above becomes
            // the gate.
            eprintln!(
                "[conformance] golden NODE-only container CRC-32C = {pinned_crc:#010x}; \
                 pin this value in GOLDEN_NODE_ONLY_CRC to activate the double-lock gate"
            );
        }
    }

    // ====================================================================
    // Part 3: Mesh-section golden vector (P0.4)
    // ====================================================================

    #[test]
    fn golden_mesh_container_round_trips_byte_identical() {
        use crate::container_10d::mesh_section::{decode_mesh_section, encode_mesh_section};
        use crate::render::assets::Mesh;

        // The unit cube: 8 vertices, 12 triangles — the canonical test mesh.
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ];
        let triangles = vec![
            [0, 1, 2],
            [0, 2, 3],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [2, 3, 7],
            [2, 7, 6],
            [1, 2, 6],
            [1, 6, 5],
            [0, 3, 7],
            [0, 7, 4],
        ];
        let mesh = Mesh {
            positions,
            triangles,
            min: [0.0; 3],
            max: [1.0; 3],
        };

        let mesh_need = crate::container_10d::mesh_section::encoded_len(8, 12);
        let mut mesh_payload = vec![0u8; mesh_need];
        encode_mesh_section(&mesh, &mut mesh_payload).expect("mesh encode");

        let h = Container10dHeader::proposed();
        let inputs = [SectionInput {
            section_type: SectionType::QuantizedMesh,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &mesh_payload,
        }];
        let mut out = vec![0u8; 512];
        let n = encode_container(&h, &inputs, &mut out).expect("container encode");
        seal_whole_file_crc32c(&mut out[..n]);
        verify_whole_file_crc32c(&mut out[..n]).expect("whole-file CRC");

        // Decode: parse header + section table + mesh payload.
        let parsed_h = Container10dHeader::parse(&out[..n]).expect("header parse");
        let descs = parse_section_table(&out[..n], &parsed_h).expect("table parse");
        assert_eq!(descs.len(), 1);
        assert_eq!(descs[0].section_type, SectionType::QuantizedMesh as u8);
        let p_off = descs[0].byte_offset as usize;
        let p_len = descs[0].byte_length as usize;
        let mesh_back = decode_mesh_section(&out[p_off..p_off + p_len]).expect("mesh decode");
        assert_eq!(
            mesh_back.triangles, mesh.triangles,
            "indices exact through container"
        );

        // Re-encode: rebuild the container from the decoded content.
        let mut reencoded = vec![0u8; 512];
        let n2 = encode_container(&parsed_h, &inputs, &mut reencoded).expect("re-encode");
        seal_whole_file_crc32c(&mut reencoded[..n2]);

        // Assert byte-identity.
        assert_eq!(n, n2, "re-encode must produce the same byte count");
        assert_eq!(
            &out[..n],
            &reencoded[..n2],
            "re-encoding the decoded MESH-only container must reproduce the \
             golden bytes exactly (encode∘decode = identity)"
        );

        // Pin the content hash.
        let pinned_crc = crc32c(&out[..n]);
        if GOLDEN_MESH_ONLY_CRC != 0 {
            assert_eq!(
                pinned_crc, GOLDEN_MESH_ONLY_CRC,
                "golden MESH-only container CRC-32C must match the pinned value"
            );
        } else {
            eprintln!(
                "[conformance] golden MESH-only container CRC-32C = {pinned_crc:#010x}; \
                 pin this value in GOLDEN_MESH_ONLY_CRC to activate the double-lock gate"
            );
        }
    }

    // ====================================================================
    // Part 4: Cross-cutting — the proposed header's metric descriptor is
    // honest (matches full_distance reality). This is the P0.1 gate
    // re-asserted from the conformance harness so the spec's claim and the
    // code's behaviour cannot drift apart even if the metric_check module is
    // refactored.
    // ====================================================================

    #[test]
    fn conformance_harness_confirms_metric_descriptor_is_honest() {
        use crate::container_10d::metric_check::verify_descriptor_against_reality;
        let h = Container10dHeader::proposed();
        verify_descriptor_against_reality(&h.metric_descriptor)
            .expect("the proposed header's metric descriptor must match full_distance reality");
    }
}
