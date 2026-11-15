use super::*;
fn clock() -> FieldClock {
    FieldClock {
        scale: 0,
        epoch_seconds: 0,
        epoch_nanoseconds: 0,
        ticks_per_second_num: 1000,
        ticks_per_second_den: 1,
    }
}
fn frame(grid: Option<GridSpec>) -> FieldFrame {
    FieldFrame {
        origin_m: [1e12, -2e12, 3e12],
        rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
        grid,
    }
}
fn provenance() -> FieldProvenance<'static> {
    FieldProvenance {
        role: 1,
        algorithm: Some("explicit-field-source"),
        algorithm_version: Some("1"),
        lineage: Some("f64 vectors retained without display-sigma inference"),
    }
}
fn sample() -> FieldSample {
    FieldSample {
        electric_v_per_m: [-0.0, -f64::MAX, 1e-200],
        magnetic_tesla: [2e-300, -3e30, f64::MIN_POSITIVE],
        position_m: [123.5, -0.0, 9.0],
        tick: 0,
        carrier_hz: Some(3e20),
        stored_poynting_w_per_m2: Some([1.0, 2.0, 3.0]),
        confidence: Some(0.75),
    }
}
#[test]
fn source_sample_bits_are_packed_without_downcast_or_invented_physics() {
    let values = [sample()];
    let mut bytes = [0xa5; 131];
    let prefix = encode_field_samples(&values, &mut bytes, Limits::default()).unwrap();
    assert_eq!(prefix.len(), 128);
    assert_eq!(&prefix[..8], &(-0.0f64).to_le_bytes());
    assert_eq!(&prefix[8..16], &(-f64::MAX).to_le_bytes());
    assert_eq!(f64_at(prefix, 80), 3e20);
    assert_eq!(&bytes[128..], &[0xa5; 3]);
    for bad in [
        FieldSample {
            electric_v_per_m: [f64::NAN, 0.0, 0.0],
            ..sample()
        },
        FieldSample {
            carrier_hz: Some(0.0),
            ..sample()
        },
        FieldSample {
            confidence: Some(1.1),
            ..sample()
        },
    ] {
        let mut out = [0xa5; 128];
        let mut late = [0xa5; 256];
        assert!(encode_field_samples(&[sample(), bad], &mut late, Limits::default()).is_err());
        assert_eq!(late, [0xa5; 256]);
        assert!(encode_field_samples(&[bad], &mut out, Limits::default()).is_err());
        assert_eq!(out, [0xa5; 128]);
    }
}
#[test]
fn point_writer_deterministic_roundtrip_preserves_full_source_payload_and_metadata() {
    let mut payload = [0; 128];
    encode_field_samples(&[sample()], &mut payload, Limits::default()).unwrap();
    let chunks = [FieldChunkInput {
        stream_id: 0,
        start_tick: 0,
        time_count: 0,
        spatial_key: None,
        payload: &payload,
    }];
    let document = FieldDocument {
        clock: clock(),
        frame: frame(None),
        provenance: provenance(),
        temporal_step_ticks: None,
        chunks: &chunks,
        align_payloads_to_4096: false,
    };
    let limits = FieldWriteLimits::default();
    let mut metadata = vec![0; field_metadata_len(&document, limits).unwrap()];
    let mut entries = [field_entry_slot(); 1];
    let prepared = prepare_field_document(&document, &mut metadata, &mut entries, limits).unwrap();
    let size = prepared.encoded_len().unwrap();
    let mut staging = vec![0; size];
    let mut out = vec![0xa5; size + 3];
    let mut nodes = [semantics::NodeState::default(); 5];
    let mut entry_nodes = [semantics::NodeState::default(); 1];
    let mut scratch = Scratch {
        metadata: &mut nodes,
        entries: &mut entry_nodes,
    };
    let first = prepared
        .encode_into(&mut staging, &mut out, &mut scratch)
        .unwrap()
        .to_vec();
    let mut second = vec![0x5a; size];
    assert_eq!(
        prepared
            .encode_into(&mut staging, &mut second, &mut scratch)
            .unwrap(),
        first
    );
    assert_eq!(&out[size..], &[0xa5; 3]);
    let view =
        StructuralView::open(&first, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let typed = admit(&view).unwrap();
    let record = typed.field(0).unwrap().unwrap();
    assert_eq!(record.electric_v_per_m()[1], -f64::MAX);
    assert_eq!(record.magnetic_tesla(), sample().magnetic_tesla);
    assert_eq!(record.position_m(), sample().position_m);
    assert_eq!(record.carrier_hz(), Some(3e20));
    assert_eq!(typed.verified_stored_bytes(), payload);
    assert_eq!(typed.profile(), FIELD_PROFILE);
}
#[test]
fn grid_writer_supports_sparse_keys_all_orders_and_4096_alignment() {
    for indexing in [
        GridIndexing::Linear,
        GridIndexing::Morton { order: 2 },
        GridIndexing::Hilbert { order: 2 },
    ] {
        let spec = GridSpec {
            dimensions: [5, 3, 2],
            brick_dimensions: [2, 2, 2],
            spacing_m: [0.5, 2.0, 3.0],
            indexing,
        };
        let layout = spec.validate().unwrap();
        let mut keys = [
            layout.encode_key([0, 0, 0]).unwrap(),
            layout.encode_key([2, 1, 0]).unwrap(),
        ];
        keys.sort();
        let payloads: Vec<Vec<u8>> = keys
            .iter()
            .map(|&key| super::spatial_tests::grid_payload(spec, key, 1))
            .collect();
        let chunks: Vec<_> = keys
            .iter()
            .zip(&payloads)
            .map(|(&key, payload)| FieldChunkInput {
                stream_id: 0,
                start_tick: 0,
                time_count: 0,
                spatial_key: Some(key),
                payload,
            })
            .collect();
        let document = FieldDocument {
            clock: clock(),
            frame: frame(Some(spec)),
            provenance: provenance(),
            temporal_step_ticks: None,
            chunks: &chunks,
            align_payloads_to_4096: true,
        };
        let limits = FieldWriteLimits::default();
        let mut metadata = vec![0; field_metadata_len(&document, limits).unwrap()];
        let mut slots = [field_entry_slot(); 2];
        let prepared =
            prepare_field_document(&document, &mut metadata, &mut slots, limits).unwrap();
        let size = prepared.encoded_len().unwrap();
        let mut staging = vec![0; size];
        let mut out = vec![0; size];
        let mut nodes = [semantics::NodeState::default(); 5];
        let mut entries = [semantics::NodeState::default(); 2];
        prepared
            .encode_into(
                &mut staging,
                &mut out,
                &mut Scratch {
                    metadata: &mut nodes,
                    entries: &mut entries,
                },
            )
            .unwrap();
        let view =
            StructuralView::open(&out, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
        assert_eq!(view.entry_count(), 2);
        for index in 1..=2 {
            assert_eq!(view.entry(index).unwrap().payload_offset() % 4096, 0);
            let typed = super::super::admit_chunk(
                &view,
                index,
                &mut Scratch {
                    metadata: &mut nodes,
                    entries: &mut entries,
                },
                Limits::default(),
            )
            .unwrap();
            assert_eq!(typed.spatial_key(), Some(keys[index as usize - 1]));
            assert_eq!(typed.verified_stored_bytes(), payloads[index as usize - 1]);
        }
    }
}
#[test]
fn publish_failure_never_changes_any_final_output_byte() {
    for case in 0..5 {
        let mut payload = field_payload(0);
        if case == 0 {
            put(&mut payload, 0, f64::NAN);
        }
        let chunks = [FieldChunkInput {
            stream_id: 0,
            start_tick: 0,
            time_count: 0,
            spatial_key: None,
            payload: &payload,
        }];
        let document = FieldDocument {
            clock: clock(),
            frame: if case == 1 {
                FieldFrame {
                    rotation_xyzw: [0.0; 4],
                    ..frame(None)
                }
            } else {
                frame(None)
            },
            provenance: provenance(),
            temporal_step_ticks: None,
            chunks: &chunks,
            align_payloads_to_4096: false,
        };
        let limits = FieldWriteLimits::default();
        let mut metadata = vec![0; field_metadata_len(&document, limits).unwrap()];
        let mut slots = [field_entry_slot(); 1];
        let prepared =
            prepare_field_document(&document, &mut metadata, &mut slots, limits).unwrap();
        let size = prepared.encoded_len().unwrap();
        let mut staging = vec![0; if case == 4 { size - 1 } else { size }];
        let mut out = vec![0xa5; if case == 2 { size - 1 } else { size }];
        let mut nodes = [semantics::NodeState::default(); 5];
        let mut entries = [semantics::NodeState::default(); 1];
        let mut scratch = Scratch {
            metadata: if case == 3 { &mut [] } else { &mut nodes },
            entries: &mut entries,
        };
        assert!(prepared
            .encode_into(&mut staging, &mut out, &mut scratch)
            .is_err());
        assert!(out.iter().all(|&b| b == 0xa5));
    }
}

#[test]
fn temporal_clipped_grid_writer_preserves_exact_tick_groups() {
    let spec = GridSpec {
        dimensions: [5, 3, 2],
        brick_dimensions: [2, 2, 2],
        spacing_m: [0.5, 2.0, 3.0],
        indexing: GridIndexing::Hilbert { order: 2 },
    };
    let key = spec.validate().unwrap().encode_key([2, 1, 0]).unwrap();
    let payload = super::spatial_tests::grid_payload(spec, key, 2);
    let chunks = [FieldChunkInput {
        stream_id: 1,
        start_tick: -2,
        time_count: 2,
        spatial_key: Some(key),
        payload: &payload,
    }];
    let document = FieldDocument {
        clock: clock(),
        frame: frame(Some(spec)),
        provenance: provenance(),
        temporal_step_ticks: Some(2),
        chunks: &chunks,
        align_payloads_to_4096: false,
    };
    let limits = FieldWriteLimits::default();
    let mut metadata = vec![0; field_metadata_len(&document, limits).unwrap()];
    let mut slots = [field_entry_slot(); 1];
    let prepared = prepare_field_document(&document, &mut metadata, &mut slots, limits).unwrap();
    let size = prepared.encoded_len().unwrap();
    let mut stage = vec![0; size];
    let mut out = vec![0; size];
    let mut nodes = [semantics::NodeState::default(); 5];
    let mut entries = [semantics::NodeState::default(); 1];
    let mut scratch = Scratch {
        metadata: &mut nodes,
        entries: &mut entries,
    };
    prepared
        .encode_into(&mut stage, &mut out, &mut scratch)
        .unwrap();
    let view = StructuralView::open(&out, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let chunk = admit(&view).unwrap();
    assert_eq!(
        (0..4)
            .map(|i| chunk.field(i).unwrap().unwrap().tick())
            .collect::<Vec<_>>(),
        vec![-2, -2, 0, 0]
    );
    assert_eq!(chunk.temporal_step_ticks(), Some(2));
    assert_eq!(chunk.verified_stored_bytes(), payload);
}
