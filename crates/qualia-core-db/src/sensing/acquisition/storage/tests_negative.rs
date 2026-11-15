use super::tests::*;
use super::*;
use crate::{
    q42::{field_envelope as env, field_semantics as sem},
    sensing::acquisition as a,
};

fn link(seed: u8, n: u64) -> a::ArtifactLink {
    a::ArtifactLink {
        digest: [seed; 32],
        identifier: id(n),
    }
}

fn full_clock() -> a::Clock {
    a::Clock::new(
        id(501),
        9,
        a::Timing {
            scale: 1,
            epoch_seconds: 1_700_000_000,
            epoch_nanoseconds: 250_000_000,
            ticks: a::Rate::new(1_000_000, 1).unwrap(),
            samples: a::Rate::new(1_000, 1).unwrap(),
            start_tick: -25,
        },
    )
    .unwrap()
}

#[test]
fn u64_near_max_preserves_exact_raw_bytes_while_numeric_rejects_precision_loss() {
    let v0 = 1u64 << 53;
    let v1 = v0 + 1;
    let v2 = u64::MAX;
    let mut raw = Vec::with_capacity(24);
    for v in [v0, v1, v2] {
        raw.extend_from_slice(&v.to_le_bytes());
    }
    let orig = a::Original::admit(&raw, raw.len()).unwrap();
    let tv = a::TensorView::admit(
        orig.region(0, 24).unwrap(),
        layout(a::Encoding::U64, 3),
        context(),
        None,
        axes(),
    )
    .unwrap();
    let encoded = encode_test(Input {
        family: Family::EmfV4Candidate,
        view: View::Tensor(tv),
        derivation: None,
    });
    inspect(&encoded, Family::EmfV4Candidate, |d| {
        let View::Tensor(t) = d.view else { panic!("expected tensor") };
        assert_eq!(t.numeric([0; 4]).unwrap(), [v0 as f64, 0.0]);
        assert_eq!(t.numeric([1, 0, 0, 0]), Err(a::Error::Unsupported));
        assert_eq!(t.numeric([2, 0, 0, 0]), Err(a::Error::Unsupported));
        assert_eq!(t.element([1, 0, 0, 0]).unwrap(), &v1.to_le_bytes());
        assert_eq!(t.element([2, 0, 0, 0]).unwrap(), &v2.to_le_bytes());
    });
}

#[test]
fn complex_f64_big_endian_imag_real_with_clock_roundtrips() {
    let mut raw = Vec::new();
    raw.extend_from_slice(&(-3.25f64).to_be_bytes());
    raw.extend_from_slice(&1.5f64.to_be_bytes());
    raw.extend_from_slice(&4.0f64.to_be_bytes());
    raw.extend_from_slice(&(-0.5f64).to_be_bytes());
    let orig = a::Original::admit(&raw, raw.len()).unwrap();
    let tv = a::TensorView::admit(
        orig.region(0, 32).unwrap(),
        a::Layout {
            rank: 1,
            extents: [2, 0, 0, 0],
            strides: [16, 0, 0, 0],
            axes: [a::Axis::Sample; 4],
            encoding: a::Encoding::ComplexF64,
            byte_order: a::ByteOrder::Big,
            components: a::ComponentOrder::ImagReal,
        },
        context(),
        Some(full_clock()),
        a::SignalAxes {
            complex_convention: Some(a::ComplexConvention::RealPlusIImagNegativeExponent),
            phase_origin: Some(link(0x44, 701)),
            ..axes()
        },
    )
    .unwrap();
    let encoded = encode_test(Input {
        family: Family::QacV1Candidate,
        view: View::Tensor(tv),
        derivation: None,
    });
    inspect(&encoded, Family::QacV1Candidate, |d| {
        let View::Tensor(t) = d.view else { panic!("expected tensor") };
        assert_eq!(t.layout().byte_order, a::ByteOrder::Big);
        assert_eq!(t.layout().components, a::ComponentOrder::ImagReal);
        assert_eq!(t.clock(), Some(full_clock()));
        assert_eq!(t.numeric([0, 0, 0, 0]).unwrap(), [1.5, -3.25]);
        assert_eq!(t.numeric([1, 0, 0, 0]).unwrap(), [-0.5, 4.0]);
    });
}

#[test]
fn events_with_links_and_empty_subrange_and_correlation_roundtrip() {
    let original_bytes = [0xEEu8; 8];
    let orig = a::Original::admit(&original_bytes, original_bytes.len()).unwrap();
    let rich_ctx = a::Context {
        geometry: Some(link(0x11, 201)),
        calibration: Some(link(0x22, 202)),
        uncertainty: Some(a::UncertaintyLink {
            artifact: link(0x33, 203),
            target: a::Quantity { kind: id(11), unit: id(12) },
        }),
        ..context()
    };
    let ev = a::EventView::admit(
        orig.region(4, 0).unwrap(),
        4,
        a::ByteOrder::Big,
        a::EventOrdering::AcquisitionOrder,
        full_clock(),
        rich_ctx,
        16,
    )
    .unwrap();
    let rich_axes = a::SignalAxes {
        carrier: a::Carrier::Hertz(2.4e9),
        baseband_hz: Some([-5e6, 5e6]),
        pulse_rate: Some(a::Rate::new(10_000, 3).unwrap()),
        complex_convention: None,
        phase_origin: None,
    };
    let ev_bytes = encode_test(Input {
        family: Family::EmfV4Candidate,
        view: View::Events { view: ev, axes: rich_axes },
        derivation: None,
    });
    inspect(&ev_bytes, Family::EmfV4Candidate, |d| {
        let View::Events { view, axes: out_axes } = d.view else { panic!("expected events") };
        assert_eq!(view.records(), 0);
        assert_eq!(view.context(), rich_ctx);
        assert_eq!(out_axes, rich_axes);
    });

    let mut corr_raw = Vec::new();
    for v in [2.0f64, -1.0, 0.5, 3.0] {
        corr_raw.extend_from_slice(&v.to_le_bytes());
    }
    let cor_orig = a::Original::admit(&corr_raw, corr_raw.len()).unwrap();
    let cor_tv = a::TensorView::admit(
        cor_orig.region(0, 32).unwrap(),
        a::Layout {
            rank: 3,
            extents: [2, 1, 1, 0],
            strides: [16, 16, 16, 0],
            axes: [a::Axis::Lag, a::Axis::LeftChannel, a::Axis::RightChannel, a::Axis::Sample],
            encoding: a::Encoding::ComplexF64,
            byte_order: a::ByteOrder::Little,
            components: a::ComponentOrder::RealImag,
        },
        rich_ctx,
        None,
        a::SignalAxes {
            complex_convention: Some(a::ComplexConvention::RealPlusIImagPositiveExponent),
            phase_origin: Some(link(0x55, 801)),
            ..rich_axes
        },
    )
    .unwrap();
    let cv = a::CorrelationView::admit(
        cor_tv,
        full_clock(),
        -3,
        6,
        a::ProductConvention::ConjugateLeftTimesRight,
        a::Normalization::Reported(link(0x66, 802)),
    )
    .unwrap();
    let cor_bytes = encode_test(Input {
        family: Family::QacV1Candidate,
        view: View::Correlation(cv),
        derivation: None,
    });
    inspect(&cor_bytes, Family::QacV1Candidate, |d| {
        let View::Correlation(c) = d.view else { panic!("expected correlation") };
        assert_eq!(c.product(), a::ProductConvention::ConjugateLeftTimesRight);
        assert_eq!(c.normalization(), a::Normalization::Reported(link(0x66, 802)));
        assert_eq!(c.lag_ticks(0).unwrap(), -3);
        assert_eq!(c.lag_ticks(1).unwrap(), 3);
        assert!(c.lag_seconds(1).is_ok());
    });
}

#[test]
fn multi_input_derivation_and_input_only_entries_roundtrip() {
    let out_bytes = [0x01u8, 0x02, 0x03, 0x04];
    let in_a = b"raw-input-alpha";
    let in_b = b"raw-input-beta-02";
    let o_out = a::Original::admit(&out_bytes, out_bytes.len()).unwrap();
    let inputs = [
        a::Original::admit(in_a, in_a.len()).unwrap(),
        a::Original::admit(in_b, in_b.len()).unwrap(),
    ];
    let losses = [a::Loss::Quantization, a::Loss::Registered(link(0x77, 901))];
    let deriv = a::Derivation::admit(o_out, &inputs, link(0x88, 902), link(0x99, 903), &losses, 16)
        .unwrap();
    let tv = a::TensorView::admit(
        o_out.region(0, 4).unwrap(),
        layout(a::Encoding::U16, 2),
        context(),
        None,
        axes(),
    )
    .unwrap();
    let encoded = encode_test(Input {
        family: Family::EmfV4Candidate,
        view: View::Tensor(tv),
        derivation: Some(deriv),
    });
    inspect(&encoded, Family::EmfV4Candidate, |d| {
        let lin = d.lineage.expect("missing lineage");
        assert_eq!(lin.inputs(), 2);
        assert_eq!(lin.input_entry(0).unwrap(), 2);
        assert_eq!(lin.input_entry(1).unwrap(), 3);
        assert_eq!(d.original(2).unwrap().bytes(), in_a);
        assert_eq!(d.original(3).unwrap().bytes(), in_b);
        assert_eq!(lin.loss(0).unwrap(), a::Loss::Quantization);
        assert_eq!(lin.loss(1).unwrap(), a::Loss::Registered(link(0x77, 901)));
    });
    let structural = env::StructuralView::open(&encoded, Family::EmfV4Candidate, Default::default()).unwrap();
    let mut originals = [None; 8];
    let mut m = [sem::NodeState::default(); 64];
    let mut e = [sem::NodeState::default(); 64];
    let mut scratch = sem::Scratch { metadata: &mut m, entries: &mut e };
    assert_eq!(
        admit(&structural, 2, &mut originals, &mut scratch, Default::default(), Default::default())
            .err(),
        Some(Error::Unsupported)
    );
}

#[test]
fn writer_errors_preserve_output_sentinel_and_reader_checks_budget_before_hash() {
    let raw = [1u8, 0, 2, 0];
    let orig = a::Original::admit(&raw, raw.len()).unwrap();
    let tv = a::TensorView::admit(
        orig.region(0, 4).unwrap(),
        layout(a::Encoding::U16, 2),
        context(),
        None,
        axes(),
    )
    .unwrap();
    let mut meta = [0u8; 16_384];
    let mut staging = vec![0u8; 65_536];
    let mut m = [sem::NodeState::default(); 64];
    let mut e = [sem::NodeState::default(); 64];
    let mut small_out = [0xccu8; 64];
    assert_eq!(
        encode(
            Input { family: Family::QacV1Candidate, view: View::Tensor(tv), derivation: None },
            &mut meta,
            &mut staging,
            &mut sem::Scratch { metadata: &mut m, entries: &mut e },
            &mut small_out,
            Default::default(),
            Default::default(),
            Default::default(),
        ),
        Err(Error::Scratch)
    );
    assert!(small_out.iter().all(|&b| b == 0xcc));

    let valid = encode_test(Input {
        family: Family::QacV1Candidate,
        view: View::Tensor(tv),
        derivation: None,
    });
    assert!(env::StructuralView::open(&valid[..valid.len() - 1], Family::QacV1Candidate, Default::default()).is_err());

    // Corrupt entry 1 payload byte AND enforce tight source_bytes budget: budget MUST reject before hashing.
    let mut bad_chunk = valid.clone();
    let offset = env::StructuralView::open(&valid, Family::QacV1Candidate, Default::default())
        .unwrap()
        .entry(1)
        .unwrap()
        .payload_offset() as usize;
    bad_chunk[offset] ^= 0xff;
    let structural = env::StructuralView::open(&bad_chunk, Family::QacV1Candidate, Default::default()).unwrap();
    let mut originals = [None; 8];
    let mut scratch = sem::Scratch { metadata: &mut m, entries: &mut e };
    assert_eq!(
        admit(
            &structural,
            1,
            &mut originals,
            &mut scratch,
            Default::default(),
            Limits { source_bytes: 1, ..Default::default() },
        )
        .err(),
        Some(Error::Limit)
    );
    assert!(matches!(
        admit(&structural, 1, &mut originals, &mut scratch, Default::default(), Default::default()),
        Err(Error::Envelope(env::Error::PayloadDigest))
    ));
}
