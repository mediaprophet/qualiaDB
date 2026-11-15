use super::*;
#[test]
fn validity_mask_counts_and_integrity() {
    let mut target = block(
        2,
        3,
        vec![
            n32(3, 8, 1),
            string(7, "opaque-original"),
            string(8, "1"),
            string(12, "urn:test:opaque"),
            n64(15, 2, 2),
            n32(16, 9, 2),
        ],
    );
    let mask = block(
        3,
        3,
        vec![
            n32(3, 8, 1),
            string(7, "validity"),
            string(8, "1"),
            string(12, "urn:test:validity"),
        ],
    );
    let p = block(1, 4, vec![n32(1, 1, 4)]);
    let target_row = EntryInput {
        flags: 0,
        frame_id: 0,
        spatial_key: 0,
        descriptor_id: 2,
        record_count: 3,
        payload: b"a\0b",
        ..row()
    };
    let mask_row = EntryInput {
        mode: 0x106,
        descriptor_id: 3,
        payload: &[1, 0, 1],
        ..target_row
    };
    assert_eq!(
        result(
            &[p.clone(), target.clone(), mask.clone()],
            &[target_row, mask_row]
        ),
        Ok(())
    );
    target
        .fields
        .iter_mut()
        .find(|f| f.key == 15)
        .unwrap()
        .value = 1u64.to_le_bytes().to_vec();
    assert_eq!(
        result(
            &[p.clone(), target.clone(), mask.clone()],
            &[target_row, mask_row]
        ),
        Err(Error::Validity)
    );
    target
        .fields
        .iter_mut()
        .find(|f| f.key == 15)
        .unwrap()
        .value = 2u64.to_le_bytes().to_vec();
    let bad = EntryInput {
        payload: &[1, 2, 0],
        ..mask_row
    };
    assert_eq!(
        result(
            &[p.clone(), target.clone(), mask.clone()],
            &[target_row, bad]
        ),
        Err(Error::Validity)
    );
    let mut encoded = bytes(&[p, target, mask], &[target_row, mask_row]);
    let v =
        StructuralView::open(&encoded, Family::QacV1Candidate, StorageLimits::default()).unwrap();
    let offset = v.entry(2).unwrap().payload_offset() as usize;
    encoded[offset] ^= 1;
    assert_eq!(
        result_with(&encoded, &SharedOnly, Limits::default()),
        Err(Error::PayloadIntegrity)
    );
}
