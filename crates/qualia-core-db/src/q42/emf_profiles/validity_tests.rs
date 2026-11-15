use super::*;
fn masked_document(mode: u16, payload: &[u8], mask: &[u8], valid: u64) -> Vec<u8> {
    let mut m = metadata(mode, false);
    replace(&mut m, 5, 15, n64(15, 2, valid));
    replace(&mut m, 5, 16, n32(16, 9, 2));
    let mut ext = metadata(0, false)[3].clone();
    ext.id = 7;
    ext.fields
        .retain(|f| u16::from_le_bytes(f[..2].try_into().unwrap()) != 4);
    m.push(ext);
    replace(&mut m, 7, 1, n32(1, 1, 0x106));
    replace(&mut m, 7, 6, s(6, VALIDITY_PROFILE));
    m.push(Meta {
        id: 8,
        schema: 3,
        fields: vec![
            n32(3, 8, 3),
            s(7, "validity"),
            s(8, "1"),
            n32(9, 1, 1),
            s(10, "byte"),
            n32(11, 1, 7),
            s(12, VALIDITY_MODEL_URI),
            n32(18, 8, 7),
        ],
    });
    let target = row(mode, payload, 2, false);
    let mask_row = EntryInput {
        mode: 0x106,
        revision: 1,
        flags: 0,
        stream_id: 0,
        frame_id: 0,
        start_tick: 0,
        time_count: 0,
        spatial_key: 0,
        record_count: mask.len() as u64,
        stride: 1,
        codec: 0,
        descriptor_id: 8,
        payload: mask,
    };
    encode(&m, &[target, mask_row])
}
#[test]
fn authoritative_byte_masks_hide_invalid_rows_without_rewriting_sources() {
    let mut payload = field_payload(0);
    payload.extend_from_slice(&[0; 128]);
    let encoded = masked_document(0, &payload, &[1, 0], 1);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let typed = admit(&view).unwrap();
    assert!(typed.field(0).unwrap().is_some());
    assert!(typed.field(1).unwrap().is_none());
    assert_eq!(view.verify_chunk(1).unwrap(), payload);
    assert_eq!(view.verify_chunk(2).unwrap(), &[1, 0]);
    let mut bad = payload;
    put(&mut bad, 128, 1.0);
    let encoded = masked_document(0, &bad, &[1, 0], 1);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    assert_eq!(admit(&view).unwrap_err(), Error::Reserved);
    assert_eq!(view.verify_chunk(1).unwrap(), bad);
}
#[test]
fn shared_mask_count_values_and_digest_remain_authoritative() {
    let mut payload = field_payload(0);
    payload.extend_from_slice(&[0; 128]);
    for (mask, count) in [(&[1, 2][..], 1), (&[1][..], 1), (&[1, 0][..], 2)] {
        let encoded = masked_document(0, &payload, mask, count);
        let view =
            StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
        assert!(matches!(
            admit(&view),
            Err(Error::Shared(semantics::Error::Validity))
        ));
    }
    let mut encoded = masked_document(0, &payload, &[1, 0], 1);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let offset = view.entry(2).unwrap().payload_offset() as usize;
    encoded[offset] ^= 1;
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    assert_eq!(
        admit(&view).unwrap_err(),
        Error::Shared(semantics::Error::PayloadIntegrity)
    );
}

#[test]
fn invalid_ray_and_spectrum_rows_are_absent_but_keep_reference_structure() {
    for mode in [1, 3] {
        let mut payload = if mode == 1 {
            ray_payload()
        } else {
            spectrum_payload(0)
        };
        let mut invalid = vec![0; 128];
        if mode == 3 {
            invalid[36..40].copy_from_slice(&6u32.to_le_bytes());
        }
        payload.extend_from_slice(&invalid);
        let encoded = masked_document(mode, &payload, &[1, 0], 1);
        let view =
            StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
        let typed = admit(&view).unwrap();
        if mode == 1 {
            assert!(typed.ray(1).unwrap().is_none());
        } else {
            assert!(typed.spectrum(1).unwrap().is_none());
            let mut bad = payload;
            bad[164..168].copy_from_slice(&99u32.to_le_bytes());
            let encoded = masked_document(mode, &bad, &[1, 0], 1);
            let view =
                StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default())
                    .unwrap();
            assert_eq!(admit(&view).unwrap_err(), Error::Descriptor);
        }
    }
}
