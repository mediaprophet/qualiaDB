
use super::*;
use crate::bundle::writer::BundleWriter;

#[test]
fn section_ranges_cannot_escape_their_bundle_entry() {
    use crate::container_10d::{AlignmentTier, SectionInput, encode_container};
    let header = Container10dHeader::proposed();
    let section = SectionInput {
        section_type: SectionType::QuantizedMesh,
        alignment_tier: AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload: &[1, 2, 3, 4],
    };
    let mut bytes = vec![0u8; 512];
    let len = encode_container(&header, &[section], &mut bytes).unwrap();
    bytes.truncate(len);
    let mut writer = BundleWriter::new();
    writer
        .add_file("mesh.10d", "10d", bytes.clone(), None)
        .unwrap();
    let bundle = writer.build().unwrap();
    let probe = HmcHeaderProbe::parse_header_range(&bundle[..BUNDLE_HEADER_SIZE]).unwrap();
    let footer =
        &bundle[probe.index_offset as usize..(probe.index_offset + probe.index_length) as usize];
    let planner = HmcProgressiveRangePlanner::from_header_and_index(probe, footer).unwrap();
    let ranges = planner
        .plan_10d_interior_section_ranges("mesh.10d", &bytes)
        .unwrap();
    assert_eq!(ranges.len(), 1);
    assert!(ranges[0].offset + ranges[0].length <= planner.entries()[0].offset + len as u64);

    // Keep a valid 10D header, but point the section at the next entry.
    let table = Container10dHeader::parse(&bytes[..HEADER_BYTE_SIZE])
        .unwrap()
        .section_table_offset as usize;
    bytes[table + 4..table + 8].copy_from_slice(&(len as u32).to_le_bytes());
    assert!(
        planner
            .plan_10d_interior_section_ranges("mesh.10d", &bytes)
            .is_err()
    );
}

#[test]
fn section_table_after_header_uses_only_the_selected_entry_prefix() {
    use crate::container_10d::{AlignmentTier, SectionInput, encode_container};
    let header = Container10dHeader::proposed();
    let section = SectionInput {
        section_type: SectionType::QuantizedMesh,
        alignment_tier: AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload: &[1, 2, 3, 4],
    };
    let mut bytes = vec![0u8; 512];
    let len = encode_container(&header, &[section], &mut bytes).unwrap();
    bytes.truncate(len);
    let mut parsed = Container10dHeader::parse(&bytes).unwrap();
    let old_table = parsed.section_table_offset as usize;
    let descriptor = bytes[old_table..old_table + SECTION_DESCRIPTOR_SIZE].to_vec();
    bytes[old_table..old_table + SECTION_DESCRIPTOR_SIZE].fill(0);
    parsed.section_table_offset = 256;
    bytes.resize(256 + SECTION_DESCRIPTOR_SIZE, 0);
    bytes[256..].copy_from_slice(&descriptor);
    bytes[..HEADER_BYTE_SIZE].copy_from_slice(&parsed.encode_to_vec64());
    crate::container_10d::seal_whole_file_crc32c(&mut bytes);
    crate::container_10d::integrity::check_whole_file_crc32c(&bytes).unwrap();
    let sealed_header = Container10dHeader::parse(&bytes).unwrap();
    assert_eq!(
        crate::container_10d::parse_section_table(&bytes, &sealed_header)
            .unwrap()
            .len(),
        1
    );
    let mut writer = BundleWriter::new();
    writer
        .add_file("mesh.10d", "10d", bytes.clone(), None)
        .unwrap();
    let bundle = writer.build().unwrap();
    let probe = HmcHeaderProbe::parse_header_range(&bundle[..BUNDLE_HEADER_SIZE]).unwrap();
    let footer =
        &bundle[probe.index_offset as usize..(probe.index_offset + probe.index_length) as usize];
    let planner = HmcProgressiveRangePlanner::from_header_and_index(probe, footer).unwrap();
    assert_eq!(
        planner
            .plan_10d_interior_section_ranges("mesh.10d", &bytes)
            .unwrap()
            .len(),
        1
    );
    assert!(
        planner
            .plan_10d_interior_section_ranges("mesh.10d", &bytes[..128])
            .is_err()
    );

    // Supplying extra bytes must not make an out-of-entry table admissible.
    let mut overrun = bytes.clone();
    parsed.section_table_offset = bytes.len() as u32 + 1;
    overrun[..HEADER_BYTE_SIZE].copy_from_slice(&parsed.encode_to_vec64());
    overrun.resize(overrun.len() + SECTION_DESCRIPTOR_SIZE + 2, 0);
    assert!(
        planner
            .plan_10d_interior_section_ranges("mesh.10d", &overrun)
            .is_err()
    );
}

#[test]
fn progressive_planner_orders_tier0_before_tier1_and_tier2_and_verifies_ranges() {
    let mut writer = BundleWriter::new();
    writer
        .add_file("mesh_lod0.10d", "10d", vec![0x11; 512], None)
        .unwrap();
    writer
        .add_file("mesh_lod2.10d", "10d", vec![0x22; 128], None)
        .unwrap();
    writer
        .add_file("semantics.q42", "q42", vec![0x33; 96], None)
        .unwrap();
    let bundle_bytes = writer.build().unwrap();

    // Phase 1: probe only the first 64 bytes
    let probe = HmcHeaderProbe::parse_header_range(&bundle_bytes[..BUNDLE_HEADER_SIZE]).unwrap();
    assert_eq!(probe.entry_count, 3);
    let (idx_off, idx_len) = probe.index_footer_range();

    // Phase 2: parse only the CBOR index footer range
    let footer_slice = &bundle_bytes[idx_off as usize..(idx_off + idx_len) as usize];
    let planner = HmcProgressiveRangePlanner::from_header_and_index(probe, footer_slice).unwrap();
    let schedule = planner.plan_entry_ranges();
    assert_eq!(schedule.len(), 3);

    // Tier 0 (.q42) is scheduled first, then Tier 1 (lod2), then Tier 2 (lod0)
    assert_eq!(schedule[0].entry_key, "semantics.q42");
    assert_eq!(
        schedule[0].tier,
        StreamPriorityTier::Tier0SemanticAndProcedural
    );
    assert_eq!(schedule[1].entry_key, "mesh_lod2.10d");
    assert_eq!(schedule[1].tier, StreamPriorityTier::Tier1MidLodMeshes);
    assert_eq!(schedule[2].entry_key, "mesh_lod0.10d");
    assert_eq!(schedule[2].tier, StreamPriorityTier::Tier2HighPolyAndMip0);

    // Verify arrived slices pass SHA-256 check and tampered slices fail closed
    for req in &schedule {
        let start = req.offset as usize;
        let end = start + req.length as usize;
        verify_arrived_range(req, &bundle_bytes[start..end]).unwrap();
    }
    let mut corrupted = bundle_bytes
        [schedule[0].offset as usize..(schedule[0].offset + schedule[0].length) as usize]
        .to_vec();
    corrupted[0] ^= 0xFF;
    assert!(verify_arrived_range(&schedule[0], &corrupted).is_err());
}

#[test]
fn forged_public_probes_and_changed_remote_extent_cannot_bypass_admission() {
    let mut writer = BundleWriter::new();
    writer.add_file("a", "10d", vec![1, 2, 3], None).unwrap();
    let bytes = writer.build().unwrap();
    let probe = HmcHeaderProbe::parse_header_range(&bytes[..64]).unwrap();
    let footer = &bytes[probe.index_offset as usize..];
    assert!(HmcProgressiveRangePlanner::from_header_and_index(probe, footer).is_ok());
    for which in 0..6 {
        let mut bad = probe;
        match which {
            0 => bad.version = 2,
            1 => bad.flags = 2,
            2 => bad.entry_count += 1,
            3 => bad.index_offset += 1,
            4 => bad.index_length -= 1,
            _ => bad.total_length += 1,
        }
        assert!(HmcProgressiveRangePlanner::from_header_and_index(bad, footer).is_err());
    }
    assert!(
        HmcProgressiveRangePlanner::from_raw_header_and_index_with_limits(
            &bytes[..64],
            footer,
            bytes.len() as u64 + 1,
            ExtentLimit::default(),
            IndexReadLimits::default()
        )
        .is_err()
    );
    let mut bad_header = bytes[..64].to_vec();
    bad_header[40] = 1;
    assert!(
        HmcProgressiveRangePlanner::from_raw_header_and_index_with_limits(
            &bad_header,
            footer,
            bytes.len() as u64,
            ExtentLimit::default(),
            IndexReadLimits::default()
        )
        .is_err()
    );
    // Only the compatibility prefix probe permits more than the 64 header bytes.
    assert!(HmcHeaderProbe::parse_header_range(&bytes).is_ok());
    assert!(
        HmcProgressiveRangePlanner::from_raw_header_and_index_with_limits(
            &bytes,
            footer,
            bytes.len() as u64,
            ExtentLimit::default(),
            IndexReadLimits::default()
        )
        .is_err()
    );
}

#[test]
fn no_digest_means_no_verified_arrival_even_for_empty_bytes() {
    let mut request = HmcRangeRequest {
        entry_key: "manual".into(),
        entry_kind: "10d".into(),
        tier: StreamPriorityTier::Tier1MidLodMeshes,
        offset: 64,
        length: 0,
        expected_sha256: None,
        expected_crc32c: None,
        section_type_raw: None,
    };
    assert!(verify_arrived_range(&request, &[]).is_err());
    request.expected_sha256 = Some(Sha256::digest([]).into());
    verify_arrived_range(&request, &[]).unwrap();
    request.expected_sha256 = None;
    request.expected_crc32c = Some(crc32c(&[]));
    verify_arrived_range(&request, &[]).unwrap();
    request.length = 1;
    assert!(verify_arrived_range(&request, &[]).is_err());
}

#[test]
fn remote_structure_does_not_claim_missing_payload_hash_completion() {
    let mut writer = BundleWriter::new();
    writer.add_file("a", "10d", vec![1, 2, 3], None).unwrap();
    let bytes = writer.build().unwrap();
    let probe = HmcHeaderProbe::parse_header_range(&bytes[..64]).unwrap();
    let footer = &bytes[probe.index_offset as usize..];
    let planner = HmcProgressiveRangePlanner::from_header_and_index(probe, footer).unwrap();
    let request = &planner.plan_entry_ranges()[0];
    assert!(verify_arrived_range(request, &[4, 5, 6]).is_err());
    assert!(verify_arrived_range(request, &[1, 2, 3]).is_ok());
}
