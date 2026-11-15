use super::*;
use crate::audio::qac::signal::{admit, Rate, Sample, SignalView};
const PCM: &[u8] = include_bytes!("../fixtures/pcm-f32-special.le");
fn document(samples: PcmSamples<'_>) -> PcmDocument<'_> {
    PcmDocument {
        samples,
        timing: Timing {
            scale: 2,
            epoch_seconds: -5,
            epoch_nanoseconds: 123,
            ticks: Rate::new(96_000, 1).unwrap(),
            samples: Rate::new(48_000_000, 1001).unwrap(),
            start_tick: -3,
        },
        stream_id: 7,
        channels: 1,
        channel_order: "mic-A",
        purpose: "original-acquisition",
        quantity: PcmQuantity::DigitalSample,
        relation: SignalRelation::DigitalUnrelatedToPressure,
        units_per_stored_sample: 1.0,
        stored_offset: 0.0,
        provenance: Provenance {
            role: ProvenanceRole::Measured,
            algorithm: None,
            algorithm_version: None,
            instrument_uri: Some("urn:instrument:fixture"),
            lineage: None,
        },
        original: None,
    }
}
fn encode_doc(document: PcmDocument<'_>) -> Vec<u8> {
    let mut metadata = [0; 4096];
    let mut payload = [0; 8192];
    let mut file = [0; 16384];
    let mut output = vec![0xa5; 16400];
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    let n = document
        .encode_into(
            WriteBuffers {
                metadata: &mut metadata,
                payload: &mut payload,
                file: &mut file,
            },
            &mut sem::Scratch {
                metadata: &mut m,
                entries: &mut e,
            },
            &mut output,
            WriteOptions::default(),
        )
        .unwrap();
    assert!(output[n..].iter().all(|x| *x == 0xa5));
    output.truncate(n);
    output
}
fn read<'a>(view: &env::StructuralView<'a>, index: u32) -> SignalView<'a> {
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    admit(
        view,
        index,
        &mut sem::Scratch {
            metadata: &mut m,
            entries: &mut e,
        },
        sem::Limits::default(),
        Limits::default(),
    )
    .unwrap()
}
#[test]
fn native_f32_and_unaligned_encoded_originals_have_exact_expected_bits() {
    let native = [
        f32::from_bits(0),
        f32::from_bits(0x80000000),
        f32::from_bits(1),
        1.0,
        -1.0,
        f32::MAX,
    ];
    let bytes = encode_doc(document(PcmSamples::F32(&native)));
    let view =
        env::StructuralView::open(&bytes, env::Family::QacV1Candidate, env::Limits::default())
            .unwrap();
    let signal = read(&view, 1);
    assert_eq!(signal.as_bytes(), PCM);
    assert_eq!(
        signal.timing(),
        Some(document(PcmSamples::F32(&native)).timing)
    );
    assert_eq!(signal.stream_id(), 7);
    assert_eq!(signal.channel_order(), "mic-A");
    assert_eq!(signal.frame_id(), 0);
    assert_eq!(signal.purpose(), "original-acquisition");
    for (i, bits) in [0u32, 0x80000000, 1, 0x3f800000, 0xbf800000, 0x7f7fffff]
        .into_iter()
        .enumerate()
    {
        let Sample::F32(value) = signal.sample(i as u64, 0, 0).unwrap() else {
            panic!()
        };
        assert_eq!(value.to_bits(), bits)
    }
    let descriptor = view.blocks().find(|b| b.id() == 5).unwrap();
    assert!(sem::field(descriptor, 2).is_none());
    assert!(sem::field(descriptor, 6).is_none());
    for shift in 0..8 {
        let mut original = vec![0; shift];
        original.extend_from_slice(PCM);
        let before = original.clone();
        let other = encode_doc(document(PcmSamples::EncodedLe {
            scalar: Scalar::F32,
            bytes: &original[shift..],
        }));
        assert_eq!(bytes, other);
        assert_eq!(original, before);
    }
}
#[test]
fn f64_and_all_integer_precisions_roundtrip_without_conversion() {
    let f64bits = [
        0u64,
        0x8000000000000000,
        1,
        0x3ff0000000000000,
        0x7fefffffffffffff,
    ];
    let doubles = f64bits.map(f64::from_bits);
    let bytes = encode_doc(document(PcmSamples::F64(&doubles)));
    let view =
        env::StructuralView::open(&bytes, env::Family::QacV1Candidate, env::Limits::default())
            .unwrap();
    let signal = read(&view, 1);
    for (i, bits) in f64bits.into_iter().enumerate() {
        let Sample::F64(value) = signal.sample(i as u64, 0, 0).unwrap() else {
            panic!()
        };
        assert_eq!(value.to_bits(), bits)
    }
    let a = [i16::MIN, i16::MAX];
    let b = [-8_388_608, 8_388_607, 0, -1];
    let c = [i32::MIN, i32::MAX];
    for (samples, expected) in [
        (
            PcmSamples::I16(&a),
            vec![Sample::I16(i16::MIN), Sample::I16(i16::MAX)],
        ),
        (
            PcmSamples::I24(&b),
            vec![
                Sample::I24(-8_388_608),
                Sample::I24(8_388_607),
                Sample::I24(0),
                Sample::I24(-1),
            ],
        ),
        (
            PcmSamples::I32(&c),
            vec![Sample::I32(i32::MIN), Sample::I32(i32::MAX)],
        ),
    ] {
        let mut doc = document(samples);
        doc.quantity = PcmQuantity::RawSampleCount;
        doc.relation = SignalRelation::RawCounts;
        let bytes = encode_doc(doc);
        let view =
            env::StructuralView::open(&bytes, env::Family::QacV1Candidate, env::Limits::default())
                .unwrap();
        let signal = read(&view, 1);
        for (i, value) in expected.into_iter().enumerate() {
            assert_eq!(signal.sample(i as u64, 0, 0).unwrap(), value)
        }
        if signal.scalar() == Scalar::I24 {
            assert_eq!(
                signal.as_bytes(),
                include_bytes!("../fixtures/pcm-i24-extrema.le")
            );
        }
    }
}
#[test]
fn pressure_and_provenance_are_explicit_and_original_entry_is_bound_exactly() {
    let original = b"actual original opaque acquisition\0\xff";
    let samples = [6.0f32, 8.0];
    let mut doc = document(PcmSamples::F32(&samples));
    doc.quantity = PcmQuantity::Pressure;
    doc.relation = SignalRelation::PhysicalBaseUnits;
    doc.provenance = Provenance {
        role: ProvenanceRole::Derived,
        algorithm: Some("urn:algorithm:supplied-linear-calibration"),
        algorithm_version: Some("fixture1"),
        instrument_uri: None,
        lineage: Some("supplied reported physical values; traceability unknown"),
    };
    doc.original = Some(original);
    let original_before = original.to_vec();
    let bytes = encode_doc(doc);
    assert_eq!(original.as_slice(), original_before.as_slice());
    let view =
        env::StructuralView::open(&bytes, env::Family::QacV1Candidate, env::Limits::default())
            .unwrap();
    assert_eq!(view.entry_count(), 2);
    assert_eq!(read(&view, 1).as_bytes(), original);
    let signal = read(&view, 2);
    assert_eq!(signal.units(), "Pa");
    assert_eq!(signal.quantity(), "pressure");
    assert_eq!(signal.sample(0, 0, 0).unwrap(), Sample::F32(6.0));
    let prov = view.blocks().find(|b| b.id() == 2).unwrap();
    assert_eq!(sem::field(prov, 1).unwrap().value(), &2u32.to_le_bytes());
    assert_eq!(
        sem::field(prov, 2).unwrap().value(),
        blake3::hash(original).as_bytes()
    );
    let descriptor = view.blocks().find(|b| b.id() == 5).unwrap();
    assert_eq!(
        sem::field(descriptor, 17).unwrap().value(),
        &1u32.to_le_bytes()
    );
    assert!(sem::field(descriptor, 6).is_none());
}
fn reject(doc: PcmDocument<'_>, options: WriteOptions, capacities: (usize, usize, usize, usize)) {
    let mut metadata = [0xcc; 4096];
    let mut payload = [0xcc; 8192];
    let mut file = [0xcc; 16384];
    let mut output = [0xa5; 16400];
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    assert!(doc
        .encode_into(
            WriteBuffers {
                metadata: &mut metadata[..capacities.0],
                payload: &mut payload[..capacities.1],
                file: &mut file[..capacities.2]
            },
            &mut sem::Scratch {
                metadata: &mut m,
                entries: &mut e
            },
            &mut output[..capacities.3],
            options
        )
        .is_err());
    assert!(output.iter().all(|b| *b == 0xa5));
}
#[test]
fn all_malformed_input_and_buffer_failures_preserve_final_output() {
    let good = document(PcmSamples::EncodedLe {
        scalar: Scalar::F32,
        bytes: PCM,
    });
    let full = (4096, 8192, 16384, 16400);
    for capacities in [
        (1, 8192, 16384, 16400),
        (4096, 1, 16384, 16400),
        (4096, 8192, 1, 16400),
        (4096, 8192, 16384, 1),
    ] {
        reject(good, WriteOptions::default(), capacities)
    }
    let mut bad = good;
    bad.quantity = PcmQuantity::Pressure;
    reject(bad, WriteOptions::default(), full);
    bad = good;
    bad.relation = SignalRelation::RawCounts;
    bad.quantity = PcmQuantity::RawSampleCount;
    reject(bad, WriteOptions::default(), full);
    bad = good;
    bad.channels = 4;
    reject(bad, WriteOptions::default(), full);
    bad = good;
    bad.purpose = "invalid\0purpose";
    reject(bad, WriteOptions::default(), full);
    bad = good;
    bad.timing.samples = Rate {
        numerator: 0,
        denominator: 0,
    };
    reject(bad, WriteOptions::default(), full);
    bad = good;
    bad.original = Some(&[]);
    reject(bad, WriteOptions::default(), full);
    bad = good;
    bad.channels = 2;
    bad.channel_order = "same,same";
    reject(bad, WriteOptions::default(), full);
    for samples in [
        PcmSamples::F32(&[f32::NAN]),
        PcmSamples::F64(&[f64::INFINITY]),
        PcmSamples::I24(&[8_388_608]),
        PcmSamples::EncodedLe {
            scalar: Scalar::F32,
            bytes: &[0, 0, 128, 127],
        },
    ] {
        reject(document(samples), WriteOptions::default(), full)
    }
}
#[test]
fn input_budgets_precede_work_and_graph_scratch_is_explicit() {
    let good = document(PcmSamples::EncodedLe {
        scalar: Scalar::F32,
        bytes: PCM,
    });
    let full = (4096, 8192, 16384, 16400);
    for signal in [
        Limits {
            samples: 0,
            ..Default::default()
        },
        Limits {
            payload_bytes: 0,
            ..Default::default()
        },
        Limits {
            entries: 0,
            ..Default::default()
        },
        Limits {
            work: 0,
            ..Default::default()
        },
    ] {
        reject(
            good,
            WriteOptions {
                signal,
                ..Default::default()
            },
            full,
        )
    }
    let mut metadata = [0; 4096];
    let mut payload = [0; 8192];
    let mut file = [0; 16384];
    let mut output = [0xa5; 16400];
    assert!(good
        .encode_into(
            WriteBuffers {
                metadata: &mut metadata,
                payload: &mut payload,
                file: &mut file
            },
            &mut sem::Scratch {
                metadata: &mut [],
                entries: &mut []
            },
            &mut output,
            WriteOptions::default()
        )
        .is_err());
    assert!(output.iter().all(|b| *b == 0xa5));
}
