use super::*;

#[test]
fn test_m64_chunk_directory_entry_size_and_alignment() {
    assert_eq!(
        std::mem::size_of::<M64ChunkDirectoryEntry>(),
        64,
        "M64ChunkDirectoryEntry must be exactly 64 bytes"
    );
    assert_eq!(
        std::mem::align_of::<M64ChunkDirectoryEntry>(),
        64,
        "M64ChunkDirectoryEntry must have 64-byte alignment"
    );
}

#[test]
fn test_m64_header_size_and_alignment() {
    assert_eq!(
        std::mem::size_of::<M64Header>(),
        64,
        "M64Header must be exactly 64 bytes"
    );
    assert_eq!(
        std::mem::align_of::<M64Header>(),
        64,
        "M64Header must have 64-byte alignment"
    );
}

#[test]
fn test_m64_chunk_directory_crc_roundtrip() {
    let mut entry = M64ChunkDirectoryEntry {
        chunk_uuid: [1; 16],
        spatial_hilbert_index: 0x0123_4567_89AB_CDEF,
        byte_offset: 1024,
        byte_length: 4096,
        record_count: 64,
        srid_geodetic_code: 4326,
        lut_bank_epoch_id: 1,
        chunk_record_mode: M64_MODE_STRATA,
        compression_flags: 0,
        bao_root_hash_prefix: [2; 12],
        entry_crc32c: 0,
    };

    entry.update_crc();
    assert_ne!(entry.entry_crc32c, 0);
    assert!(entry.validate().is_ok());

    // Single bit flip should fail validation
    entry.record_count ^= 1;
    assert!(matches!(entry.validate(), Err(M64Error::ChecksumMismatch { .. })));
}

#[test]
fn test_m64_header_crc_roundtrip() {
    let mut header = M64Header::default();
    header.total_file_size = 65536;
    header.chunk_count = 16;
    header.spatial_origin = [-35.78, 148.29]; // Talbingo, NSW coordinates
    header.elevation_datum = [350.0, 1.0];

    header.update_crc();
    assert_ne!(header.header_crc32c, 0);
    assert!(header.validate().is_ok());

    // Single bit flip should fail validation
    header.chunk_count ^= 1;
    assert!(matches!(header.validate(), Err(M64Error::ChecksumMismatch { .. })));
}

#[test]
fn test_m64_strata_record_size_and_alignment() {
    assert_eq!(
        std::mem::size_of::<M64StrataColumnRecord>(),
        64,
        "M64StrataColumnRecord must be exactly 64 bytes"
    );
    assert_eq!(
        std::mem::align_of::<M64StrataColumnRecord>(),
        64,
        "M64StrataColumnRecord must have 64-byte alignment"
    );
}

#[test]
fn test_m64_strata_record_crc_and_navigation() {
    let mut record = M64StrataColumnRecord::default();
    record.surface_elevation_mm = 520_000; // 520.0 m
    // layer_thickness_mm default is [400, 800, 1600, 1200, 2000, 10000]
    // Soil sum (first 5 layers): 400 + 800 + 1600 + 1200 + 2000 = 6000 mm (6.0 m)
    assert_eq!(record.total_soil_depth_mm(), 6000);
    assert_eq!(record.bedrock_elevation_mm(), 514_000);

    // Layer navigation
    assert_eq!(record.layer_at_depth_mm(200), 0); // 0..400: Topsoil
    assert_eq!(record.layer_at_depth_mm(400), 1); // 400..1200: Clay
    assert_eq!(record.layer_at_depth_mm(1199), 1);
    assert_eq!(record.layer_at_depth_mm(1200), 2); // 1200..2800: Gravel
    assert_eq!(record.layer_at_depth_mm(2799), 2);
    assert_eq!(record.layer_at_depth_mm(2800), 3); // 2800..4000: Clasts
    assert_eq!(record.layer_at_depth_mm(4000), 4); // 4000..6000: Basalt
    assert_eq!(record.layer_at_depth_mm(6000), 5); // 6000+: Bedrock

    // CRC validation
    record.update_crc();
    assert_ne!(record.record_crc32c, 0);
    assert!(record.validate().is_ok());

    // Bit flip detection
    record.surface_elevation_mm ^= 1;
    assert!(matches!(record.validate(), Err(M64Error::ChecksumMismatch { .. })));
}

#[test]
fn test_m64_phase_juncture_size_and_alignment() {
    assert_eq!(
        std::mem::size_of::<M64PhaseJunctureRecord>(),
        64,
        "M64PhaseJunctureRecord must be exactly 64 bytes"
    );
    assert_eq!(
        std::mem::align_of::<M64PhaseJunctureRecord>(),
        64,
        "M64PhaseJunctureRecord must have 64-byte alignment"
    );
}

#[test]
fn test_m64_phase_juncture_crc_and_reinflation() {
    let mut record = M64PhaseJunctureRecord::default();
    record.anchor_pos_mm = [100_000, 200_000, 50_000];
    record.excavation_radius_mm = 500;
    record.excavated_lut_material = 3;
    record.provenance_did_hash = 0x9876_5432_10FE_DCBA;

    // Verify ground truth state
    assert!(record.is_ground_truth());

    // Set cartoon wafer deformation deficit
    record.set_cartoon_wafer_deficit(0.125, 48);
    assert_eq!(record.volume_deficit_m3(), 0.125);
    assert_eq!(record.re_inflation_delay_ticks, 48);
    assert_ne!(record.boundary_flags & FLAG_CARTOON_WAFER, 0);

    // CRC validation
    record.update_crc();
    assert_ne!(record.record_crc32c, 0);
    assert!(record.validate().is_ok());

    // Bit flip detection
    record.anchor_pos_mm[0] ^= 1;
    assert!(matches!(record.validate(), Err(M64Error::ChecksumMismatch { .. })));
}

#[test]
fn test_constituent_lut_entry_size_and_alignment() {
    assert_eq!(
        std::mem::size_of::<Q42ConstituentLutEntry>(),
        64,
        "Q42ConstituentLutEntry must be exactly 64 bytes"
    );
    assert_eq!(
        std::mem::align_of::<Q42ConstituentLutEntry>(),
        64,
        "Q42ConstituentLutEntry must have 64-byte alignment"
    );
}

#[test]
fn test_constituent_lut_entry_crc_validation() {
    let mut entry = Q42ConstituentLutEntry::default();
    entry.refractive_index_x100 = 154; // Quartz
    entry.bulk_density_kg_m3 = 2650;
    entry.provenance_did_hash = 0x0123_4567_89AB_CDEF;

    entry.update_crc();
    assert_ne!(entry.entry_crc32c, 0);
    assert!(entry.validate().is_ok());

    entry.bulk_density_kg_m3 ^= 1;
    assert!(matches!(entry.validate(), Err(M64Error::ChecksumMismatch { .. })));
}

#[test]
fn test_banked_lut_size_and_paging_isolation() {
    let mut lut = BankedConstituentLut::new_with_terrestrial_defaults();
    let gpu_bytes = lut.as_gpu_bytes();
    assert_eq!(gpu_bytes.len(), 16384, "Total LUT size must be exactly 16 KB (16,384 bytes)");

    // Bank 0 slot 1 should be Topsoil
    assert_eq!(lut.entry(1).refractive_index_x100, 145);
    // Bank 0 slot 3 should be Quartz
    assert_eq!(lut.entry(3).refractive_index_x100, 154);

    // Prepare Bank 1 paged recipes (96 entries)
    let mut paged = [Q42ConstituentLutEntry::default(); BANK_1_PAGED_COUNT];
    for (i, entry) in paged.iter_mut().enumerate() {
        entry.refractive_index_x100 = (200 + i) as u16;
        entry.update_crc();
    }

    // Page Bank 1 with epoch 42
    assert!(lut.page_bank_1(42, &paged).is_ok());
    assert_eq!(lut.active_bank_1_epoch_id, 42);
    assert!(lut.is_dirty);

    // Verify Bank 1 entries updated
    assert_eq!(lut.entry(128).refractive_index_x100, 200);
    assert_eq!(lut.entry(128 + 95).refractive_index_x100, 295);

    // Verify Bank 0 was untouched (isolation preserved)
    assert_eq!(lut.entry(1).refractive_index_x100, 145);
    assert_eq!(lut.entry(3).refractive_index_x100, 154);
}
