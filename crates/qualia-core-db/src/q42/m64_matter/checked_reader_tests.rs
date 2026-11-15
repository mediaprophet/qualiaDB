use super::super::has_m64_magic;
use super::*;

#[test]
fn legacy_prefix_requires_opt_in_and_detects_tampering() {
    let payload = [0x42u8; 64];
    let digest = blake3::hash(&payload);
    let mut bao_prefix = [0u8; 12];
    bao_prefix.copy_from_slice(&digest.as_bytes()[..12]);

    let mut header = M64Header::default();
    header.total_file_size = 192; // 64B header + 64B dir + 64B payload
    header.chunk_count = 1;
    header.chunk_table_offset = 64;
    header.update_crc();

    let mut entry = M64ChunkDirectoryEntry {
        chunk_uuid: [0; 16],
        spatial_hilbert_index: 0,
        byte_offset: 128,
        byte_length: 64,
        record_count: 1,
        srid_geodetic_code: 0,
        lut_bank_epoch_id: 0,
        chunk_record_mode: M64_MODE_POINT,
        compression_flags: 2, // Bao verified
        bao_root_hash_prefix: bao_prefix,
        entry_crc32c: 0,
    };
    entry.update_crc();

    let mut buffer = vec![0u8; 192];
    buffer[..64].copy_from_slice(bytemuck::bytes_of(&header));
    buffer[64..128].copy_from_slice(bytemuck::bytes_of(&entry));
    buffer[128..192].copy_from_slice(&payload);

    let limits = M64ReadLimits {
        max_file_bytes: 1024,
        max_chunks: 16,
        max_records: 16,
    };
    let mut scratch = [M64PayloadInterval::default(); 16];

    assert!(matches!(M64Reader::parse(&buffer, limits, &mut scratch),
        Err(M64ReadError::LegacyIntegrityRequiresOptIn { .. })));
    let reader = M64Reader::parse_legacy_with_limits(&buffer, limits, &mut scratch).unwrap();
    assert_eq!(reader.verification().legacy_prefix_chunks, 1);
    assert_eq!(reader.verification().full_hash_chunks, 0);

    // Tamper with chunk payload byte
    buffer[130] ^= 0xFF;
    let tampered_result = M64Reader::parse_legacy_with_limits(&buffer, limits, &mut scratch);
    assert_eq!(
        tampered_result.err(),
        Some(M64ReadError::BaoHashMismatch { chunk_index: 0 }),
        "Expected BaoHashMismatch on tampered payload"
    );
}

fn one_record_container(magic: [u8; 4], mode: u8, payload: &[u8; 64]) -> Vec<u8> {
    let mut header = M64Header::default();
    header.magic = magic;
    header.total_file_size = 192;
    header.chunk_count = 1;
    header.chunk_table_offset = 64;
    header.update_crc();
    let mut entry = M64ChunkDirectoryEntry {
        chunk_uuid: [0; 16],
        spatial_hilbert_index: 0,
        byte_offset: 128,
        byte_length: 64,
        record_count: 1,
        srid_geodetic_code: 0,
        lut_bank_epoch_id: 0,
        chunk_record_mode: mode,
        compression_flags: 0,
        bao_root_hash_prefix: [0u8; 12],
        entry_crc32c: 0,
    };
    entry.update_crc();
    let mut buffer = vec![0u8; 192];
    buffer[..64].copy_from_slice(bytemuck::bytes_of(&header));
    buffer[64..128].copy_from_slice(bytemuck::bytes_of(&entry));
    buffer[128..192].copy_from_slice(payload);
    buffer
}

#[test]
fn admits_e64_magic_and_shell_mode_with_crc() {
    let mut rec = M64ShellRecord::default();
    rec.face_id = 3;
    rec.update_crc();
    let buffer = one_record_container(E64_MAGIC, M64_MODE_SHELL, bytemuck::cast_ref(&rec));
    let limits = M64ReadLimits {
        max_file_bytes: 1024,
        max_chunks: 4,
        max_records: 4,
    };
    let mut scratch = [M64PayloadInterval::default(); 4];
    let reader = M64Reader::parse(&buffer, limits, &mut scratch).expect("e64 shell");
    assert_eq!(reader.header().magic, E64_MAGIC);
    assert_eq!(reader.verification().record_crc_checked, 1);
    assert_eq!(
        reader.chunk(0).unwrap().directory_entry().chunk_record_mode,
        M64_MODE_SHELL
    );
}

#[test]
fn admits_holo_without_per_record_crc() {
    let rec = M64HoloRecord::default();
    let buffer = one_record_container(M64_MAGIC, M64_MODE_HOLO, bytemuck::cast_ref(&rec));
    let limits = M64ReadLimits {
        max_file_bytes: 1024,
        max_chunks: 4,
        max_records: 4,
    };
    let mut scratch = [M64PayloadInterval::default(); 4];
    let reader = M64Reader::parse(&buffer, limits, &mut scratch).expect("holo");
    assert_eq!(reader.verification().records_without_checksum, 1);
    assert_eq!(reader.verification().record_crc_checked, 0);
    assert_eq!(
        reader.chunk(0).unwrap().directory_entry().chunk_record_mode,
        M64_MODE_HOLO
    );
}

fn parse_one(buffer: &[u8]) -> Result<M64Reader<'_>, M64ReadError> {
    let limits = M64ReadLimits {
        max_file_bytes: 1024,
        max_chunks: 4,
        max_records: 4,
    };
    let mut scratch = [M64PayloadInterval::default(); 4];
    M64Reader::parse(buffer, limits, &mut scratch)
}

#[test]
fn admits_point_without_per_record_crc() {
    let rec = M64PointRecord::default();
    let buffer = one_record_container(M64_MAGIC, M64_MODE_POINT, bytemuck::cast_ref(&rec));
    let reader = parse_one(&buffer).expect("point");
    assert_eq!(reader.verification().records_without_checksum, 1);
    assert_eq!(reader.verification().record_crc_checked, 0);
    assert_eq!(
        reader.chunk(0).unwrap().directory_entry().chunk_record_mode,
        M64_MODE_POINT
    );
}

#[test]
fn shell_m64_and_e64_alias_writer_round_trip() {
    let mut rec = M64ShellRecord::default();
    rec.face_id = 11;
    rec.vertex_indices = [1, 2, 3];
    rec.edge_indices = [4, 5, 6];
    rec.crease = 0.25;
    rec.update_crc();
    let payload: &[u8; 64] = bytemuck::cast_ref(&rec);
    for magic in [M64_MAGIC, E64_MAGIC] {
        let buffer = one_record_container(magic, M64_MODE_SHELL, payload);
        assert!(has_m64_magic(&buffer));
        let reader = parse_one(&buffer).expect("shell round-trip");
        assert_eq!(reader.header().magic, magic);
        assert!(has_m64_magic(&reader.header().magic));
        assert_eq!(reader.verification().record_crc_checked, 1);
        assert_eq!(reader.verification().records_without_checksum, 0);
        let chunk = reader.chunk(0).unwrap();
        assert_eq!(chunk.directory_entry().chunk_record_mode, M64_MODE_SHELL);
        assert_eq!(chunk.record_bytes(0).unwrap(), payload.as_slice());
        let decoded: M64ShellRecord =
            bytemuck::pod_read_unaligned(chunk.record_bytes(0).unwrap());
        assert_eq!(decoded, rec);
        assert!(decoded.validate().is_ok());
    }
}

#[test]
fn rejects_shell_record_crc_mismatch() {
    let mut rec = M64ShellRecord::default();
    rec.update_crc();
    let mut buffer = one_record_container(M64_MAGIC, M64_MODE_SHELL, bytemuck::cast_ref(&rec));
    buffer[128] ^= 1;
    assert!(matches!(
        parse_one(&buffer),
        Err(M64ReadError::Record {
            chunk: 0,
            record: 0,
            error: M64Error::ChecksumMismatch { .. },
        })
    ));
}

#[test]
fn strata_terrain_still_uses_record_crc() {
    let mut rec = M64StrataColumnRecord::default();
    rec.update_crc();
    let buffer = one_record_container(M64_MAGIC, M64_MODE_STRATA, bytemuck::cast_ref(&rec));
    let reader = parse_one(&buffer).expect("strata");
    assert_eq!(reader.verification().record_crc_checked, 1);
    assert_eq!(reader.verification().records_without_checksum, 0);
    assert_eq!(
        reader.chunk(0).unwrap().directory_entry().chunk_record_mode,
        M64_MODE_STRATA
    );
}

#[test]
fn rejects_unknown_record_mode() {
    let buffer = one_record_container(M64_MAGIC, 0x05, &[0u8; 64]);
    assert_eq!(
        parse_one(&buffer).err(),
        Some(M64ReadError::UnsupportedMode {
            chunk: 0,
            mode: 0x05
        })
    );
}

fn lrm3_container(lrm3: &M64LocalMetricHeader) -> Vec<u8> {
    let rec = M64HoloRecord::default();
    let mut header = M64Header::default();
    header.total_file_size = 320;
    header.chunk_count = 1;
    header.chunk_table_offset = 192;
    header.container_flags = M64_FLAG_LOCAL_METRIC_LRM3 as u32;
    header.update_crc();
    let mut entry = M64ChunkDirectoryEntry {
        chunk_uuid: [0; 16],
        spatial_hilbert_index: 0,
        byte_offset: 256,
        byte_length: 64,
        record_count: 1,
        srid_geodetic_code: 0,
        lut_bank_epoch_id: 0,
        chunk_record_mode: M64_MODE_HOLO,
        compression_flags: 0,
        bao_root_hash_prefix: [0u8; 12],
        entry_crc32c: 0,
    };
    entry.update_crc();
    let mut buffer = vec![0u8; 320];
    buffer[..64].copy_from_slice(bytemuck::bytes_of(&header));
    buffer[64..192].copy_from_slice(bytemuck::bytes_of(lrm3));
    buffer[192..256].copy_from_slice(bytemuck::bytes_of(&entry));
    buffer[256..320].copy_from_slice(bytemuck::bytes_of(&rec));
    buffer
}

#[test]
fn admits_lrm3_gap_when_flag_set_and_crc_valid() {
    let mut lrm3 = M64LocalMetricHeader::default();
    lrm3.anchor_did_hash = 0xAABB_CCDD_EEFF_0011;
    lrm3.anisotropic_scale_um = [2.0, 3.0, 4.0];
    lrm3.update_crc();
    let buffer = lrm3_container(&lrm3);
    let reader = parse_one(&buffer).expect("lrm3 container");
    let parsed = reader.lrm3().expect("authenticated LRM3");
    assert_eq!(parsed.anchor_did_hash, 0xAABB_CCDD_EEFF_0011);
    assert_eq!(parsed.anisotropic_scale_um, [2.0, 3.0, 4.0]);
    assert!(parsed.validate().is_ok());
}

#[test]
fn rejects_lrm3_flag_when_block_missing() {
    let rec = M64HoloRecord::default();
    let buffer = one_record_container(M64_MAGIC, M64_MODE_HOLO, bytemuck::cast_ref(&rec));
    let mut header: M64Header = bytemuck::pod_read_unaligned(&buffer[..64]);
    header.container_flags = M64_FLAG_LOCAL_METRIC_LRM3 as u32;
    header.update_crc();
    let mut tampered = buffer;
    tampered[..64].copy_from_slice(bytemuck::bytes_of(&header));
    assert_eq!(parse_one(&tampered).err(), Some(M64ReadError::Lrm3Missing));
}

#[test]
fn rejects_lrm3_gap_crc_mismatch() {
    let mut lrm3 = M64LocalMetricHeader::default();
    lrm3.update_crc();
    let mut buffer = lrm3_container(&lrm3);
    buffer[70] ^= 1;
    assert!(matches!(
        parse_one(&buffer),
        Err(M64ReadError::Lrm3(M64Error::ChecksumMismatch { .. }))
    ));
}

#[test]
fn rejects_payload_overlapping_authenticated_lrm3_gap() {
    let mut lrm3 = M64LocalMetricHeader::default();
    lrm3.update_crc();
    let mut buffer = lrm3_container(&lrm3);
    // Directory at 192..256; LRM3 occupies [64, 192). Alias the second LRM3
    // cache line (128..192) so the range does not also hit the directory.
    let mut entry: M64ChunkDirectoryEntry =
        bytemuck::pod_read_unaligned(&buffer[192..256]);
    entry.byte_offset = 128;
    entry.update_crc();
    buffer[192..256].copy_from_slice(bytemuck::bytes_of(&entry));
    assert_eq!(
        parse_one(&buffer).err(),
        Some(M64ReadError::MetadataOverlap { chunk: 0 })
    );
}
