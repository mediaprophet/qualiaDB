use super::helpers::{assert_both_families, base_single, encode_envelope, entry_timed, sample_metadata};
use crate::q42::field_envelope::{
    BlockInput, EntryInput, Error, Family, FieldInput, FieldValue, Limits, MetadataInput,
    StructuralView,
};

#[test]
fn golden_all_emf_v4_modes_0100_to_0107_round_trip() {
    let meta = sample_metadata(None, None);
    let strides = [128usize, 128, 128, 128, 256, 128, 128, 128];
    for (idx, &stride) in strides.iter().enumerate() {
        let mode = 0x0100u16 + idx as u16;
        let payload = vec![(idx as u8).wrapping_add(17); stride];
        let entry = [EntryInput {
            mode,
            revision: 1,
            flags: 3,
            stream_id: 0,
            frame_id: 0,
            start_tick: -100,
            time_count: 1,
            spatial_key: 0,
            record_count: 1,
            stride: stride as u32,
            codec: 0,
            descriptor_id: 4,
            payload: &payload,
        }];
        let bytes = encode_envelope(Family::EmfV4Candidate, 0, &meta, &entry);
        let view =
            StructuralView::open(&bytes, Family::EmfV4Candidate, Limits::default()).unwrap();
        assert_eq!(view.entry(1).unwrap().mode(), mode);
        assert_eq!(view.verify_chunk(1).unwrap(), payload.as_slice());
    }
}

#[test]
fn golden_all_qac_v1_modes_0100_to_0107_continuum_and_aux() {
    let meta = sample_metadata(None, None);
    let mut modes = vec![0x0000u16, 0x0001];
    modes.extend(0x0100u16..=0x0107);
    for (idx, &mode) in modes.iter().enumerate() {
        let stride = if mode < 0x0100 { 64usize } else { 16usize };
        let payload = vec![(idx as u8).wrapping_add(31); stride];
        let entry = [EntryInput {
            mode,
            revision: 1,
            flags: 3,
            stream_id: 0,
            frame_id: 0,
            start_tick: 0,
            time_count: 1,
            spatial_key: 0,
            record_count: 1,
            stride: stride as u32,
            codec: 0,
            descriptor_id: 4,
            payload: &payload,
        }];
        let bytes = encode_envelope(Family::QacV1Candidate, 0, &meta, &entry);
        let view =
            StructuralView::open(&bytes, Family::QacV1Candidate, Limits::default()).unwrap();
        assert_eq!(view.entry(1).unwrap().mode(), mode);
        assert_eq!(view.verify_chunk(1).unwrap(), payload.as_slice());
    }
}

#[test]
fn golden_4096_alignment_and_metadata_only_and_entry_index_bounds() {
    assert_both_families(|family| {
        let meta = sample_metadata(None, None);
        let entry = [entry_timed(0, -1, 0x0104, 4, b"ALIGNED")];
        let bytes = encode_envelope(family, 1, &meta, &entry);
        assert_eq!(bytes.len() % 4096, 0);
        let view = StructuralView::open(&bytes, family, Limits::default()).unwrap();
        assert_eq!(view.entry_count(), 1);
        assert_eq!(view.entry(0).unwrap_err(), Error::EntryIndex);
        assert_eq!(view.entry(2).unwrap_err(), Error::EntryIndex);
        assert_eq!(view.verify_chunk(0).unwrap_err(), Error::EntryIndex);
        assert_eq!(view.verify_chunk(2).unwrap_err(), Error::EntryIndex);

        let meta_only = encode_envelope(family, 0, &meta, &[]);
        let empty_view = StructuralView::open(&meta_only, family, Limits::default()).unwrap();
        assert_eq!(empty_view.entry_count(), 0);
        assert_eq!(empty_view.entry(1).unwrap_err(), Error::EntryIndex);
    });
}

#[test]
fn golden_five_tuple_sort_tie_breakers_and_negative_ticks() {
    assert_both_families(|family| {
        let meta = sample_metadata(None, None);
        let entries = [
            EntryInput {
                mode: 0x0104,
                revision: 1,
                flags: 3,
                stream_id: 0,
                frame_id: 0,
                start_tick: -10,
                time_count: 1,
                spatial_key: 0,
                record_count: 1,
                stride: 1,
                codec: 0,
                descriptor_id: 4,
                payload: b"a",
            },
            EntryInput {
                mode: 0x0104,
                revision: 1,
                flags: 3,
                stream_id: 0,
                frame_id: 0,
                start_tick: -1,
                time_count: 1,
                spatial_key: 0,
                record_count: 1,
                stride: 1,
                codec: 0,
                descriptor_id: 4,
                payload: b"b",
            },
            EntryInput {
                mode: 0x0104,
                revision: 1,
                flags: 7,
                stream_id: 0,
                frame_id: 2,
                start_tick: -1,
                time_count: 1,
                spatial_key: 9,
                record_count: 1,
                stride: 1,
                codec: 0,
                descriptor_id: 4,
                payload: b"c",
            },
            EntryInput {
                mode: 0x0105,
                revision: 1,
                flags: 7,
                stream_id: 0,
                frame_id: 2,
                start_tick: -1,
                time_count: 1,
                spatial_key: 9,
                record_count: 1,
                stride: 1,
                codec: 0,
                descriptor_id: 4,
                payload: b"d",
            },
            EntryInput {
                mode: 0x0105,
                revision: 1,
                flags: 7,
                stream_id: 0,
                frame_id: 2,
                start_tick: -1,
                time_count: 1,
                spatial_key: 9,
                record_count: 1,
                stride: 1,
                codec: 0,
                descriptor_id: 5,
                payload: b"e",
            },
            EntryInput {
                mode: 0x0104,
                revision: 1,
                flags: 3,
                stream_id: 1,
                frame_id: 0,
                start_tick: i64::MIN,
                time_count: 1,
                spatial_key: 0,
                record_count: 1,
                stride: 1,
                codec: 0,
                descriptor_id: 4,
                payload: b"f",
            },
        ];
        let bytes = encode_envelope(family, 0, &meta, &entries);
        let view = StructuralView::open(&bytes, family, Limits::default()).unwrap();
        assert_eq!(view.entry_count(), 6);
        assert_eq!(view.entry(1).unwrap().start_tick(), -10);
        assert_eq!(view.entry(6).unwrap().start_tick(), i64::MIN);
        for i in 1..=6 {
            view.verify_chunk(i).unwrap();
        }
    });
}

#[test]
fn golden_negative_zero_subnormal_and_non_ascii_utf8_preserved() {
    let f32_bits = [-0.0f32, f32::from_bits(1)];
    let f64_bits = [-0.0f64, f64::from_bits(1)];
    let fields = [
        FieldInput {
            key: 1,
            flags: 0,
            value: FieldValue::F32(&f32_bits),
        },
        FieldInput {
            key: 2,
            flags: 0,
            value: FieldValue::F64(&f64_bits),
        },
        FieldInput {
            key: 3,
            flags: 0,
            value: FieldValue::Utf8("café-Ω"),
        },
    ];
    let blocks = [BlockInput {
        id: 1,
        schema_type: 65000,
        revision: 1,
        flags: 0,
        fields: &fields,
    }];
    let m = MetadataInput { blocks: &blocks };
    let limits = Limits::default();
    let mut meta = vec![0u8; m.encoded_len(limits).unwrap()];
    m.encode_into(&mut meta, limits).unwrap();

    assert_both_families(|family| {
        let neg_zero_payload = (-0.0f32).to_le_bytes();
        let entry = [EntryInput {
            mode: 0x0100,
            revision: 1,
            flags: 1,
            stream_id: 0,
            frame_id: 0,
            start_tick: 0,
            time_count: 0,
            spatial_key: 0,
            record_count: 1,
            stride: 4,
            codec: 0,
            descriptor_id: 1,
            payload: &neg_zero_payload,
        }];
        let bytes = encode_envelope(family, 0, &meta, &entry);
        let view = StructuralView::open(&bytes, family, limits).unwrap();
        let mut iter = view.blocks().next().unwrap().fields();
        let f1 = iter.next().unwrap();
        let f2 = iter.next().unwrap();
        let f3 = iter.next().unwrap();
        assert_eq!(&f1.value()[..4], &0x8000_0000u32.to_le_bytes());
        assert_eq!(&f2.value()[..8], &0x8000_0000_0000_0000u64.to_le_bytes());
        assert_eq!(std::str::from_utf8(f3.value()).unwrap(), "café-Ω");
        assert_eq!(view.verify_chunk(1).unwrap(), &neg_zero_payload);
    });
}

#[test]
fn rejects_archival_q42ac064_magic_at_structural_and_storage_boundaries() {
    use crate::audio::qac::QacStorageView;

    let mut qac = base_single(Family::QacV1Candidate);
    qac[..8].copy_from_slice(b"Q42AC064");
    assert_eq!(
        StructuralView::open(&qac, Family::QacV1Candidate, Limits::default()).unwrap_err(),
        Error::Family
    );
    assert!(QacStorageView::open(&qac, crate::audio::qac::QacStorageOptions { envelope_limits: Limits::default() }).is_err());

    let mut q42 = base_single(Family::QacV1Candidate);
    q42[..8].copy_from_slice(b"Q42AC064");
    assert_eq!(
        StructuralView::open(&q42, Family::QacV1Candidate, Limits::default()).unwrap_err(),
        Error::Family
    );
}
