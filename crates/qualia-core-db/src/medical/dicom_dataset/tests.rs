use super::*;
const DEFINED: &[u8] = include_bytes!("fixtures/enhanced-defined.dcm");
const UNDEFINED: &[u8] = include_bytes!("fixtures/enhanced-undefined.dcm");
#[test]
fn nested_defined_and_undefined_items_preserve_values_and_order() {
    for bytes in [DEFINED, UNDEFINED] {
        let ds = Part10Dataset::parse(bytes, ParseLimits::default()).unwrap();
        assert_eq!(ds.original_bytes(), bytes);
        assert_eq!(ds.source_blake3(), *blake3::hash(bytes).as_bytes());
        let mut w = WorkBudget::new(10000);
        let per = ds.find(None, 0x52009230, &mut w).unwrap().unwrap();
        let items: Vec<_> = ds.sequence_items(per, &mut w).unwrap().collect();
        assert_eq!(items.len(), 2);
        for (i, item) in items.iter().enumerate() {
            let pos = ds.find(Some(*item), 0x00209113, &mut w).unwrap().unwrap();
            let child = ds.sequence_items(pos, &mut w).unwrap().next().unwrap();
            let raw = ds
                .expected_value(Some(child), 0x00200032, *b"DS", &mut w)
                .unwrap()
                .unwrap();
            assert_eq!(
                std::str::from_utf8(raw).unwrap().trim(),
                if i == 0 { "0\\0\\0" } else { "0\\0\\5" }
            );
        }
        assert_eq!(
            ds.text(None, 0x00111010, *b"LO", &mut w).unwrap(),
            Some("KEEP ORIGINAL")
        );
    }
}
#[test]
fn explicit_byte_index_depth_element_item_and_work_limits_fail_closed() {
    let cases = [
        (
            ParseLimits {
                max_bytes: DEFINED.len() - 1,
                ..Default::default()
            },
            DatasetError::ByteLimit,
        ),
        (
            ParseLimits {
                max_index_bytes: 0,
                ..Default::default()
            },
            DatasetError::IndexLimit,
        ),
        (
            ParseLimits {
                max_depth: 1,
                ..Default::default()
            },
            DatasetError::DepthLimit,
        ),
        (
            ParseLimits {
                max_elements: 0,
                ..Default::default()
            },
            DatasetError::ElementLimit,
        ),
        (
            ParseLimits {
                max_items: 0,
                ..Default::default()
            },
            DatasetError::ItemLimit,
        ),
        (
            ParseLimits {
                max_work: 0,
                ..Default::default()
            },
            DatasetError::WorkLimit,
        ),
    ];
    for (limit, error) in cases {
        assert!(matches!(Part10Dataset::parse(DEFINED,limit),Err(e) if e==error));
    }
    let ds = Part10Dataset::parse(DEFINED, ParseLimits::default()).unwrap();
    assert_eq!(
        ds.find(None, 0x52009230, &mut WorkBudget::new(0))
            .unwrap_err(),
        DatasetError::WorkLimit
    );
}
#[test]
fn malformed_delimiters_lengths_and_truncation_are_rejected() {
    let needle = [0xfe, 0xff, 0x0d, 0xe0, 0, 0, 0, 0];
    let at = UNDEFINED.windows(8).position(|b| b == needle).unwrap();
    let mut bad = UNDEFINED.to_vec();
    bad[at + 2] = 0xdd;
    assert!(matches!(
        Part10Dataset::parse(&bad, ParseLimits::default()),
        Err(DatasetError::MalformedDelimiter)
    ));
    let mut bad = UNDEFINED.to_vec();
    bad[at + 4] = 2;
    assert!(matches!(
        Part10Dataset::parse(&bad, ParseLimits::default()),
        Err(DatasetError::MalformedDelimiter)
    ));
    assert!(Part10Dataset::parse(&DEFINED[..DEFINED.len() - 1], ParseLimits::default()).is_err());
    let at = DEFINED
        .windows(6)
        .position(|b| b == [0, 0x52, 0x29, 0x92, b'S', b'Q'])
        .unwrap();
    let mut bad = DEFINED.to_vec();
    let len = u32::from_le_bytes(bad[at + 8..at + 12].try_into().unwrap());
    bad[at + 8..at + 12].copy_from_slice(&(len - 2).to_le_bytes());
    assert!(Part10Dataset::parse(&bad, ParseLimits::default()).is_err());
}
#[test]
fn archive_identity_is_of_whole_independent_source_and_non_image_is_preserved() {
    let bytes = include_bytes!("fixtures/non-image.dcm");
    let ds = Part10Dataset::parse(bytes, ParseLimits::default()).unwrap();
    let tmp = tempfile::TempDir::new().unwrap();
    let input = tmp.path().join("synthetic.dcm");
    std::fs::write(&input, bytes).unwrap();
    let a = crate::medical::dicom_source::archive_part10_source(
        tmp.path(),
        &input,
        crate::medical::dicom_source::DicomSourceBudget {
            max_source_bytes: bytes.len(),
        },
    )
    .unwrap();
    assert_eq!(ds.source_blake3(), a.blake3);
    assert_eq!(std::fs::read(a.path).unwrap(), bytes);
    assert_eq!(ds.original_bytes(), bytes);
}
#[test]
fn encapsulated_fragments_are_indexed_as_opaque_not_dataset_elements() {
    let bytes = include_bytes!("fixtures/encapsulated-opaque.dcm");
    let ds = Part10Dataset::parse(bytes, ParseLimits::default()).unwrap();
    let mut w = WorkBudget::new(10000);
    let id = ds.find(None, 0x7fe00010, &mut w).unwrap().unwrap();
    assert_eq!(
        ds.element(id).unwrap().kind,
        ElementKind::EncapsulatedPixels
    );
    let items: Vec<_> = ds.sequence_items(id, &mut w).unwrap().collect();
    assert_eq!(items.len(), 2);
    assert!(ds.item_bytes(items[0]).unwrap().is_empty());
    assert_eq!(
        ds.item_bytes(items[1]).unwrap(),
        [255, 216, 1, 2, 3, 4, 255, 217]
    );
}

#[test]
fn duplicate_dataset_identity_is_not_silently_replaced() {
    let mut bytes = DEFINED.to_vec();
    let pattern = [0x20, 0, 0x0e, 0, b'U', b'I'];
    let at = bytes.windows(6).position(|b| b == pattern).unwrap();
    bytes[at + 2] = 0x0d;
    assert!(matches!(
        Part10Dataset::parse(&bytes, ParseLimits::default()),
        Err(DatasetError::Duplicate(0x0020000d))
    ));
}
