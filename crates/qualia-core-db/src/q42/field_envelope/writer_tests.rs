use super::*;
#[path = "fixture_registry.rs"]
mod fixture_registry;
fn row(payload: &[u8]) -> EntryInput<'_> {
    EntryInput {
        mode: 0x107,
        revision: 1,
        flags: 0,
        stream_id: 0,
        frame_id: 0,
        start_tick: 0,
        time_count: 0,
        spatial_key: 0,
        record_count: payload.len() as u64,
        stride: 1,
        codec: 0,
        descriptor_id: 1,
        payload,
    }
}
#[test]
fn independent_golden_bytes_are_reproduced_exactly() {
    for (stem, ext, family) in [
        ("qac_candidate", "qac", Family::QacV1Candidate),
        ("emf_candidate", "emf", Family::EmfV4Candidate),
    ] {
        let cases = if family == Family::QacV1Candidate {
            &[
                "metadata-only",
                "opaque",
                "aligned4096",
                "two-entries",
                "pcm-signed-zero",
            ][..]
        } else {
            &["metadata-only", "opaque", "aligned4096", "two-entries"][..]
        };
        for name in cases {
            let path = format!("candidate/{stem}-{name}.{ext}");
            let bytes = fixture_registry::load(&path);
            let view = StructuralView::open(bytes, family, Limits::default()).unwrap();
            let entries: Vec<_> = (1..=view.entry_count())
                .map(|i| {
                    let e = view.entry(i).unwrap();
                    EntryInput {
                        mode: e.mode(),
                        revision: e.revision(),
                        flags: e.flags(),
                        stream_id: e.stream_id(),
                        frame_id: e.frame_id(),
                        start_tick: e.start_tick(),
                        time_count: e.time_count(),
                        spatial_key: e.spatial_key(),
                        record_count: e.record_count(),
                        stride: e.stride(),
                        codec: 0,
                        descriptor_id: e.descriptor_id(),
                        payload: view.verify_chunk(i).unwrap(),
                    }
                })
                .collect();
            let input = EnvelopeInput {
                family,
                flags: u32le(bytes, 12),
                metadata: view.metadata_bytes(),
                entries: &entries,
            };
            assert_eq!(input.encoded_len(Limits::default()).unwrap(), bytes.len());
            let mut first = vec![0xa5; bytes.len() + 3];
            let mut second = vec![0x5a; bytes.len()];
            assert_eq!(
                input.encode_into(&mut first, Limits::default()).unwrap(),
                bytes,
                "{path}"
            );
            assert_eq!(
                input.encode_into(&mut second, Limits::default()).unwrap(),
                bytes,
                "{path}"
            );
            assert_eq!(&first[bytes.len()..], &[0xa5; 3]);
        }
    }
}
#[test]
fn empty_envelope_and_selected_alignments() {
    for family in [Family::QacV1Candidate, Family::EmfV4Candidate] {
        for flags in [0, 1] {
            let input = EnvelopeInput {
                family,
                flags,
                metadata: &[],
                entries: &[],
            };
            let len = input.encoded_len(Limits::default()).unwrap();
            assert_eq!(len, if flags == 0 { 128 } else { 4096 });
            let mut bytes = vec![0xff; len];
            input.encode_into(&mut bytes, Limits::default()).unwrap();
            let view = StructuralView::open(&bytes, family, Limits::default()).unwrap();
            assert_eq!(view.entry_count(), 0);
            let rows = [row(&[1, 2, 3])];
            let input = EnvelopeInput {
                entries: &rows,
                ..input
            };
            let mut bytes = vec![0; input.encoded_len(Limits::default()).unwrap()];
            input.encode_into(&mut bytes, Limits::default()).unwrap();
            let view = StructuralView::open(&bytes, family, Limits::default()).unwrap();
            assert_eq!(
                view.entry(1).unwrap().payload_offset(),
                if flags == 0 { 256 } else { 4096 }
            );
            assert_eq!(view.verify_chunk(1).unwrap(), &[1, 2, 3]);
        }
    }
}
fn unchanged_on_error(input: EnvelopeInput<'_>, limits: Limits, expected: Error) {
    let mut output = [0xa5; 512];
    assert_eq!(
        input.encode_into(&mut output, limits).unwrap_err(),
        expected
    );
    assert_eq!(output, [0xa5; 512]);
}
#[test]
fn invalid_rows_overflow_order_and_short_buffer_do_not_publish() {
    let good = row(&[7]);
    for (bad, error) in [
        (EntryInput { codec: 1, ..good }, Error::Codec),
        (EntryInput { flags: 8, ..good }, Error::Directory),
        (
            EntryInput {
                descriptor_id: 0,
                ..good
            },
            Error::Directory,
        ),
        (EntryInput { stride: 0, ..good }, Error::Directory),
        (
            EntryInput {
                record_count: 0,
                ..good
            },
            Error::Directory,
        ),
        (
            EntryInput {
                record_count: 2,
                ..good
            },
            Error::Directory,
        ),
        (
            EntryInput {
                record_count: u64::MAX,
                stride: 2,
                ..good
            },
            Error::Overflow,
        ),
        (
            EntryInput {
                start_tick: 1,
                ..good
            },
            Error::Directory,
        ),
        (
            EntryInput {
                time_count: 1,
                ..good
            },
            Error::Directory,
        ),
        (
            EntryInput {
                spatial_key: 1,
                ..good
            },
            Error::Directory,
        ),
        (
            EntryInput {
                flags: 2,
                time_count: 1,
                ..good
            },
            Error::Directory,
        ),
        (EntryInput { flags: 4, ..good }, Error::Directory),
    ] {
        unchanged_on_error(
            EnvelopeInput {
                family: Family::QacV1Candidate,
                flags: 0,
                metadata: &[],
                entries: &[bad],
            },
            Limits::default(),
            error,
        );
    }
    let input = EnvelopeInput {
        family: Family::QacV1Candidate,
        flags: 0,
        metadata: &[],
        entries: &[good],
    };
    let mut short = [0xa5; 128];
    assert_eq!(
        input
            .encode_into(&mut short, Limits::default())
            .unwrap_err(),
        Error::Extent
    );
    assert_eq!(short, [0xa5; 128]);
    unchanged_on_error(
        EnvelopeInput { flags: 2, ..input },
        Limits::default(),
        Error::Header,
    );
    unchanged_on_error(
        EnvelopeInput {
            entries: &[good, good],
            ..input
        },
        Limits::default(),
        Error::Order,
    );
    let later = EntryInput {
        descriptor_id: 2,
        ..good
    };
    unchanged_on_error(
        EnvelopeInput {
            entries: &[later, good],
            ..input
        },
        Limits::default(),
        Error::Order,
    );
}
#[test]
fn invalid_metadata_and_budgets_do_not_publish() {
    let rows = [row(&[1])];
    let input = EnvelopeInput {
        family: Family::QacV1Candidate,
        flags: 0,
        metadata: &[],
        entries: &rows,
    };
    unchanged_on_error(
        EnvelopeInput {
            metadata: &[0; 15],
            ..input
        },
        Limits::default(),
        Error::Metadata,
    );
    for limits in [
        Limits {
            file_bytes: 0,
            ..Limits::default()
        },
        Limits {
            entries: 0,
            ..Limits::default()
        },
        Limits {
            chunk_bytes: 0,
            ..Limits::default()
        },
        Limits {
            aggregate_decoded_bytes: 0,
            ..Limits::default()
        },
    ] {
        unchanged_on_error(input, limits, Error::Limit);
    }
    let fixture = fixture_registry::load("candidate/qac_candidate-opaque.qac");
    let view = StructuralView::open(fixture, Family::QacV1Candidate, Limits::default()).unwrap();
    let input = EnvelopeInput {
        metadata: view.metadata_bytes(),
        ..input
    };
    for limits in [
        Limits {
            metadata_bytes: 0,
            ..Limits::default()
        },
        Limits {
            blocks: 0,
            ..Limits::default()
        },
        Limits {
            fields_per_block: 0,
            ..Limits::default()
        },
    ] {
        unchanged_on_error(input, limits, Error::Limit);
    }
}
