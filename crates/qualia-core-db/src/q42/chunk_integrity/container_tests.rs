use super::*;
use crate::q42::{emf_container::*, m64_matter::*};

fn m64_full(magic: [u8; 4], mode: u8) -> Vec<u8> {
    let mut records = [0x42; 64];
    if mode == M64_MODE_SHELL {
        let mut shell = M64ShellRecord::default();
        shell.update_crc();
        records.copy_from_slice(bytemuck::bytes_of(&shell));
    }
    let mut header = M64Header::default();
    header.magic = magic;
    header.total_file_size = 256;
    header.chunk_count = 1;
    header.update_crc();
    let mut entry = M64ChunkDirectoryEntry {
        chunk_uuid: [0; 16], spatial_hilbert_index: 0, byte_offset: 128,
        byte_length: 128, record_count: 1, srid_geodetic_code: 0,
        lut_bank_epoch_id: 0, chunk_record_mode: mode,
        compression_flags: M64_CODEC_BLAKE3_FULL, bao_root_hash_prefix: [0; 12],
        entry_crc32c: 0,
    };
    entry.update_crc();
    let mut bytes = vec![0; 256];
    bytes[..64].copy_from_slice(bytemuck::bytes_of(&header));
    bytes[64..128].copy_from_slice(bytemuck::bytes_of(&entry));
    bytes[128..192].copy_from_slice(&records);
    bytes[192..].copy_from_slice(&blake3_trailer(&records));
    bytes
}
fn m64_parse(bytes: &[u8]) -> Result<M64Verification, M64ReadError> {
    let limits = M64ReadLimits { max_file_bytes: 1024, max_chunks: 8, max_records: 8 };
    let mut scratch = [M64PayloadInterval::default(); 8];
    let reader = M64Reader::parse(bytes, limits, &mut scratch)?;
    assert_eq!(reader.chunk(0).unwrap().payload().len(), 64);
    assert!(reader.chunk(0).unwrap().record_bytes(1).is_none());
    Ok(reader.verification())
}
#[test]
fn m64_and_e64_full_digest_and_crc_records_coexist() {
    for magic in [M64_MAGIC, E64_MAGIC] {
        for mode in [M64_MODE_POINT, M64_MODE_HOLO, M64_MODE_SHELL] {
            let v = m64_parse(&m64_full(magic, mode)).unwrap();
            assert_eq!(v.full_hash_chunks, 1);
            assert_eq!(v.legacy_prefix_chunks, 0);
            assert_eq!(v.total_records, 1);
            assert_eq!(v.record_crc_checked, if mode == M64_MODE_SHELL { 1 } else { 0 });
        }
    }
}
#[test]
fn m64_checks_entire_digest_including_previously_discarded_tail() {
    let original = m64_full(M64_MAGIC, M64_MODE_POINT);
    for i in 0..64 {
        let mut bytes = original.clone();
        bytes[192 + i] ^= 1;
        assert!(matches!(m64_parse(&bytes), Err(M64ReadError::FullHashMismatch { .. })), "trailer byte {i}");
    }
    let mut bytes = original;
    bytes[128] ^= 1;
    assert!(matches!(m64_parse(&bytes), Err(M64ReadError::FullHashMismatch { .. })));
}
#[test]
fn m64_refuses_length_downgrade_and_truncation() {
    let mut bytes = m64_full(M64_MAGIC, M64_MODE_POINT);
    let mut entry: M64ChunkDirectoryEntry = bytemuck::pod_read_unaligned(&bytes[64..128]);
    entry.compression_flags = 0;
    entry.update_crc();
    bytes[64..128].copy_from_slice(bytemuck::bytes_of(&entry));
    assert!(matches!(m64_parse(&bytes), Err(M64ReadError::RecordLength { .. })));
    assert!(m64_parse(&bytes[..224]).is_err());
}
fn emf_full() -> Vec<u8> {
    let mut writer = EmfContainerWriter::new([0.; 3], 1., [1; 3], 0.);
    writer.add_brick_chunk(0, &[EmfCellRecord64::default()], 0, 1, 0).unwrap();
    writer.to_bytes().unwrap()
}
#[test]
fn emf_checks_entire_digest_and_never_exposes_trailer_as_cell() {
    let original = emf_full();
    let reader = EmfContainerReader::parse(&original).unwrap();
    assert_eq!(reader.header().version, 2);
    assert_eq!(reader.chunk_payload(0).unwrap().len(), 64);
    assert!(reader.cell(0, 1).is_none());
    assert_eq!(original, emf_full());
    for i in 0..64 {
        let mut bytes = original.clone();
        bytes[192 + i] ^= 1;
        assert!(matches!(EmfContainerReader::parse(&bytes), Err(EmfError::FullHashMismatch { .. })), "trailer byte {i}");
    }
}
#[test]
fn emf_legacy_is_explicit_and_can_be_resealed() {
    let mut bytes = emf_full();
    bytes.truncate(192);
    let mut header: EmfContainerHeader = bytemuck::pod_read_unaligned(&bytes[..64]);
    header.version = EMF_LEGACY_VERSION;
    header.update_crc();
    bytes[..64].copy_from_slice(bytemuck::bytes_of(&header));
    let mut entry: EmfChunkDirectoryEntry = bytemuck::pod_read_unaligned(&bytes[64..128]);
    entry.flags = 2;
    entry.byte_length = 64;
    entry.bao_hash_prefix.copy_from_slice(&blake3::hash(&bytes[128..]).as_bytes()[..16]);
    entry.update_crc();
    bytes[64..128].copy_from_slice(bytemuck::bytes_of(&entry));
    assert!(matches!(EmfContainerReader::parse(&bytes), Err(EmfError::LegacyIntegrityRequiresOptIn)));
    let legacy = EmfContainerReader::parse_legacy_with_limits(&bytes, EmfReadLimits::conservative()).unwrap();
    let mut writer = EmfContainerWriter::new([0.; 3], 1., [1; 3], 0.);
    writer.add_brick_chunk(0, &[legacy.cell(0, 0).unwrap()], 0, 1, 0).unwrap();
    let upgraded = writer.to_bytes().unwrap();
    assert_eq!(upgraded, emf_full());
    assert!(EmfContainerReader::parse(&upgraded).is_ok());
}
#[test]
fn emf_refuses_missing_or_mixed_integrity_flags() {
    for flags in [0, 2, 6] {
        let mut bytes = emf_full();
        let mut entry: EmfChunkDirectoryEntry = bytemuck::pod_read_unaligned(&bytes[64..128]);
        entry.flags = flags;
        entry.update_crc();
        bytes[64..128].copy_from_slice(bytemuck::bytes_of(&entry));
        assert!(matches!(EmfContainerReader::parse(&bytes), Err(EmfError::InvalidIntegrityEncoding { .. })));
    }
}
