use super::*;
use alloc::vec::Vec;

fn header(kvs: u64, tensors: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    bytes.extend_from_slice(&3u32.to_le_bytes());
    bytes.extend_from_slice(&tensors.to_le_bytes());
    bytes.extend_from_slice(&kvs.to_le_bytes());
    bytes
}
fn string(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value);
}
fn limits() -> ParseLimits {
    ParseLimits {
        max_metadata_bytes: 4096,
        max_nesting_depth: 32,
        max_work: 1000,
        versions: VersionPolicy::V2V3,
    }
}

#[test]
fn preserves_raw_tensor_dimensions_names_offsets_and_full_hash() {
    let mut bytes = header(0, 1);
    string(&mut bytes, b"hello");
    bytes.extend_from_slice(&5u32.to_le_bytes());
    for dimension in [2u64, 3, 4, 5, u64::MAX] {
        bytes.extend_from_slice(&dimension.to_le_bytes());
    }
    bytes.extend_from_slice(&999u32.to_le_bytes());
    bytes.extend_from_slice(&u64::MAX.to_le_bytes());
    let view = MetadataView::parse(&bytes, limits()).unwrap();
    let tensor = view.tensors().next().unwrap().unwrap();
    assert_eq!(tensor.rank, 5);
    assert_eq!(tensor.dimension(4), Some(u64::MAX));
    assert_eq!(tensor.dimension(5), None);
    assert_eq!(tensor.dimension(usize::MAX), None);
    assert_eq!(tensor.ggml_type, 999);
    assert_eq!(tensor.offset, u64::MAX);
    assert_eq!(tensor.name_hash(), 0xa430_d846_80aa_bd0b);
    assert_eq!(view.metadata_bytes().as_ptr(), bytes.as_ptr());
    assert_eq!(view.metadata_end(), bytes.len());
}

#[test]
fn partial_cursor_failure_is_sticky_and_strict_view_rejects() {
    let mut bytes = header(2, 0);
    string(&mut bytes, b"ok");
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.push(7);
    let mut cursor = KvCursor::new(&bytes, limits()).unwrap();
    assert_eq!(cursor.next().unwrap().unwrap().encoded_value, &[7]);
    let error = cursor.next().unwrap().unwrap_err();
    assert_eq!(error.kind, ParseErrorKind::Truncated);
    assert_eq!(cursor.failure(), Some(error));
    assert!(!cursor.is_complete());
    assert!(cursor.next().is_none());
    assert!(cursor.next().is_none());
    assert!(MetadataView::parse(&bytes, limits()).is_err());
}

#[test]
fn nested_arrays_obey_explicit_depth_and_cumulative_work() {
    for depth in [1, 32, 33] {
        let mut bytes = header(1, 0);
        string(&mut bytes, b"");
        bytes.extend_from_slice(&9u32.to_le_bytes());
        for index in 0..depth {
            bytes.extend_from_slice(&(if index == depth - 1 { 0u32 } else { 9u32 }).to_le_bytes());
            bytes.extend_from_slice(&(if index == depth - 1 { 0u64 } else { 1u64 }).to_le_bytes());
        }
        let mut policy = limits();
        policy.max_work = depth as u64 + 1;
        let result = MetadataView::parse(&bytes, policy);
        if depth <= 32 {
            assert!(result.is_ok());
        } else {
            assert_eq!(result.unwrap_err().kind, ParseErrorKind::DepthLimit);
        }
        policy.max_nesting_depth = 0;
        assert_eq!(
            MetadataView::parse(&bytes, policy).unwrap_err().kind,
            ParseErrorKind::DepthLimit
        );
        policy.max_nesting_depth = 33;
        assert_eq!(
            MetadataView::parse(&bytes, policy).unwrap_err().kind,
            ParseErrorKind::InvalidLimits
        );
    }
}

#[test]
fn work_budget_crosses_kv_and_tensor_boundary() {
    let mut bytes = header(1, 1);
    string(&mut bytes, b"key");
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.push(1);
    string(&mut bytes, b"");
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&1u64.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&0u64.to_le_bytes());
    let mut policy = limits();
    policy.max_work = 4;
    assert!(MetadataView::parse(&bytes, policy).is_ok());
    policy.max_work = 3;
    assert_eq!(
        MetadataView::parse(&bytes, policy).unwrap_err().kind,
        ParseErrorKind::WorkLimit
    );
    policy = limits();
    policy.max_metadata_bytes = bytes.len() - 1;
    assert_eq!(
        MetadataView::parse(&bytes, policy).unwrap_err().kind,
        ParseErrorKind::MetadataLimit
    );
}

#[test]
fn version_alignment_and_absent_payload_are_explicit() {
    let mut bytes = header(0, 0);
    bytes[4..8].copy_from_slice(&99u32.to_le_bytes());
    assert_eq!(
        MetadataView::parse(&bytes, limits()).unwrap_err().kind,
        ParseErrorKind::UnsupportedVersion(99)
    );
    let mut policy = limits();
    policy.versions = VersionPolicy::AtLeast2;
    policy.max_work = 0;
    let view = MetadataView::parse(&bytes, policy).unwrap();
    assert_eq!(view.tensor_data_start(32).unwrap(), 32);
    assert_eq!(view.tensor_data_start(64).unwrap(), 64);
    assert_eq!(
        view.tensor_data_start(0).unwrap_err().kind,
        ParseErrorKind::InvalidAlignment
    );
    assert_eq!(
        view.tensor_data_start(3).unwrap_err().kind,
        ParseErrorKind::InvalidAlignment
    );
    assert!(view.kvs().is_complete());
}

#[test]
fn unknown_empty_array_and_huge_lengths_fail_without_panics() {
    let mut bytes = header(1, 0);
    string(&mut bytes, b"");
    bytes.extend_from_slice(&9u32.to_le_bytes());
    bytes.extend_from_slice(&99u32.to_le_bytes());
    bytes.extend_from_slice(&0u64.to_le_bytes());
    assert_eq!(
        MetadataView::parse(&bytes, limits()).unwrap_err().kind,
        ParseErrorKind::UnknownValueType(99)
    );
    bytes = header(1, 0);
    bytes.extend_from_slice(&u64::MAX.to_le_bytes());
    assert_eq!(
        MetadataView::parse(&bytes, limits()).unwrap_err().kind,
        ParseErrorKind::ArithmeticOverflow
    );
}
