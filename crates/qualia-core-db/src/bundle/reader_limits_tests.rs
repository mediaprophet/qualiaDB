//! Authored source regressions; execution is deferred until the coding freeze.
use super::*;
use super::super::writer::BundleWriter;

fn sample() -> Vec<u8> {
    let mut w = BundleWriter::new();
    w.add_file("source.q42", "q42", vec![1, 2, 3], Some(vec![0, 255, 42])).unwrap();
    w.add_file("model.10d", "10d", vec![4, 5, 6], None).unwrap();
    w.build().unwrap()
}
fn replace_index(bytes: &mut Vec<u8>, index: &[u8], count: u32) {
    let offset = read_u64(bytes, OFF_INDEX_OFFSET) as usize;
    bytes.truncate(offset);
    bytes.extend_from_slice(index);
    bytes[8..12].copy_from_slice(&count.to_le_bytes());
    bytes[OFF_INDEX_LENGTH..OFF_INDEX_LENGTH + 8].copy_from_slice(&(index.len() as u64).to_le_bytes());
    let len = bytes.len() as u64;
    bytes[OFF_TOTAL_LENGTH..OFF_TOTAL_LENGTH + 8].copy_from_slice(&len.to_le_bytes());
    let crc = whole_file_crc(bytes);
    bytes[OFF_CRC32C..OFF_CRC32C + 4].copy_from_slice(&crc.to_le_bytes());
}
#[test]
fn bounded_roundtrip_preserves_opaque_metadata_and_payloads() {
    let bytes = sample();
    let r = BundleReader::parse_with_limits(&bytes, BundleReadLimits::default()).unwrap();
    assert_eq!(r.get("source.q42"), Some(&[1, 2, 3][..]));
    assert_eq!(r.entry("source.q42").unwrap().meta.as_deref(), Some(&[0, 255, 42][..]));
}
#[test]
fn each_caller_limit_refuses_before_returning_a_reader() {
    let bytes = sample();
    for which in 0..7 {
        let mut limits = BundleReadLimits::default();
        match which { 0 => limits.file_bytes = bytes.len() - 1,
            1 => limits.index_bytes = 1, 2 => limits.entries = 1,
            3 => limits.key_bytes = 1, 4 => limits.kind_bytes = 1,
            5 => limits.meta_bytes = 2, _ => limits.decoded_index_bytes = 1 }
        assert!(BundleReader::parse_with_limits(&bytes, limits).is_err(), "limit {which}");
    }
}
#[test]
fn huge_declared_containers_strings_and_nested_unknown_fields_refuse() {
    // No allocation from declared CBOR lengths: each input remains tiny.
    let malicious: &[&[u8]] = &[
        &[0x9b, 255, 255, 255, 255, 255, 255, 255, 255],
        &[0x81, 0xa5, 0x63, b'k', b'e', b'y', 0x7b, 255, 255, 255, 255, 255, 255, 255, 255],
        &[0x81, 0xa6, 0x67, b'u', b'n', b'k', b'n', b'o', b'w', b'n', 0x9f, 0x9f, 0x9f],
        &[0x9f, 0xff],
    ];
    for index in malicious {
        let mut bytes = sample();
        replace_index(&mut bytes, index, 1);
        assert!(BundleReader::parse_with_limits(&bytes, BundleReadLimits::default()).is_err());
    }
}
#[test]
fn strict_profile_rejects_trailing_index_values_and_header_count_mismatch() {
    let mut bytes = sample();
    let offset = read_u64(&bytes, OFF_INDEX_OFFSET) as usize;
    let mut index = bytes[offset..].to_vec();
    index.push(0xf6);
    replace_index(&mut bytes, &index, 2);
    assert!(BundleReader::parse_with_limits(&bytes, BundleReadLimits::default()).is_err());
    let mut bytes = sample();
    bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
    let crc = whole_file_crc(&bytes);
    bytes[OFF_CRC32C..OFF_CRC32C + 4].copy_from_slice(&crc.to_le_bytes());
    assert!(BundleReader::parse_with_limits(&bytes, BundleReadLimits::default()).is_err());
}
#[test]
fn valid_crc_cannot_conceal_a_changed_payload_digest() {
    let mut bytes = sample();
    let offset = BundleReader::parse(&bytes).unwrap().entry("source.q42").unwrap().offset as usize;
    bytes[offset] ^= 1;
    let crc = whole_file_crc(&bytes);
    bytes[OFF_CRC32C..OFF_CRC32C + 4].copy_from_slice(&crc.to_le_bytes());
    assert!(BundleReader::parse(&bytes).is_ok()); // Legacy API stays unchanged.
    assert!(BundleReader::parse_with_limits(&bytes, BundleReadLimits::default()).is_err());
}

#[test]
fn duplicate_keys_and_overlapping_payloads_refuse_before_payload_hashing() {
    for duplicate in [true, false] {
        let mut bytes = sample();
        let mut entries = BundleReader::parse(&bytes).unwrap().entries().to_vec();
        entries[1] = entries[0].clone();
        if !duplicate { entries[1].key = "another.q42".into(); }
        // Both hashes genuinely match the same retained payload. Integrity
        // alone must not admit duplicate/overlapping directory records.
        let mut index = Vec::new();
        ciborium::into_writer(&entries, &mut index).unwrap();
        replace_index(&mut bytes, &index, 2);
        assert!(BundleReader::parse_with_limits(&bytes, BundleReadLimits::default()).is_err());
    }
}
