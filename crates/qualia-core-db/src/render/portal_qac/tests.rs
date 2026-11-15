use super::*;
use crate::audio::qac::signal::{decode_sample, ExactTime, Sample, Scalar};
use crate::q42::field_envelope::{Family, Limits as StorageLimits, StructuralView};
#[path = "../../audio/qac/signal/test_support.rs"]
mod support;
use support::{build, Config};
fn open(bytes: Vec<u8>, selected: u32) -> RetainedQac {
    RetainedQac::open(bytes.into_boxed_slice(), selected, Limits::default()).unwrap()
}
#[test]
fn pcm_retains_source_bits_channels_and_exact_time_without_copy() {
    let values = [-0.0f64, f64::from_bits(1), 1.0 + 2f64.powi(-40), f64::MAX];
    let payload: Vec<_> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    let bytes = build(
        &payload,
        Config {
            scalar: 2,
            channels: 2,
            start: -3,
            ..Default::default()
        },
        |b| {
            support::field(b, 1, 2).bytes = (-5i64).to_le_bytes().to_vec();
            support::field(b, 1, 3).bytes = 123u32.to_le_bytes().to_vec();
            support::field(b, 4, 2).bytes = 2.5f64.to_le_bytes().to_vec();
            support::field(b, 4, 3).bytes = (-4.0f64).to_le_bytes().to_vec();
        },
    );
    let expected = bytes.clone();
    let owned = bytes.into_boxed_slice();
    let ptr = owned.as_ptr();
    let retained = RetainedQac::open(owned, 1, Limits::default()).unwrap();
    assert_eq!(retained.source_bytes().as_ptr(), ptr);
    assert_eq!(retained.source_bytes(), expected);
    assert_eq!(retained.selected_payload_bytes(), payload);
    let meta = retained.metadata();
    assert_eq!(meta.epistemic_role, "measured");
    assert_eq!(&*meta.channel_order, "channel-0,channel-1");
    assert_eq!(&*meta.relation, "digital-unrelated-to-pressure");
    assert_eq!(&*meta.quantity, "digital_sample");
    assert_eq!(&*meta.units, "1");
    assert_eq!(meta.units_per_stored_sample, 2.5);
    assert_eq!(meta.stored_offset, -4.0);
    assert_eq!(meta.timing.epoch_seconds, -5);
    assert_eq!(meta.timing.epoch_nanoseconds, 123);
    let window = retained.pcm_window(0, 2, 1).unwrap();
    for (i, expected) in [values[1], values[3]].into_iter().enumerate() {
        let Sample::F64(v) = window.sample(i as u64).unwrap() else {
            panic!()
        };
        assert_eq!(v.to_bits(), expected.to_bits());
    }
    let first = retained.pcm_window(0, 2, 0).unwrap();
    let Sample::F64(v) = first.sample(0).unwrap() else {
        panic!()
    };
    assert_eq!(v.to_bits(), (-0.0f64).to_bits());
    assert_eq!(window.original_interleaved_bytes(), payload);
    assert_eq!(window.time_at(1).unwrap(), meta.timing.time_at(1).unwrap());
    // Independent rational oracle: -5 +123/1e9 -3*1001/48000000 +1001/48000000.
    assert_eq!(
        window.time_at(1).unwrap(),
        ExactTime {
            numerator: -3750031189,
            denominator: 750000000
        }
    );
    assert!(retained.stft_window(0, 1, 0).is_err());
}
#[test]
fn pcm_integer_precision_and_pressure_are_not_normalized() {
    for (scalar, payload, expected) in [
        (3, i16::MIN.to_le_bytes().to_vec(), Sample::I16(i16::MIN)),
        (4, vec![0, 0, 128], Sample::I24(-8388608)),
        (5, i32::MAX.to_le_bytes().to_vec(), Sample::I32(i32::MAX)),
    ] {
        let retained = open(
            build(
                &payload,
                Config {
                    scalar,
                    ..Default::default()
                },
                |_| {},
            ),
            1,
        );
        assert_eq!(
            retained.pcm_window(0, 1, 0).unwrap().sample(0).unwrap(),
            expected
        );
        assert_eq!(&*retained.metadata().relation, "raw-counts");
    }
    let payload = 8f32.to_le_bytes();
    let bytes = build(&payload, Config::default(), |b| {
        support::field(b, 6, 7).bytes = b"pressure".to_vec();
        support::field(b, 6, 8).bytes = b"Pa".to_vec();
        support::field(b, 4, 5).bytes = b"physical-base-units".to_vec();
    });
    let retained = open(bytes, 1);
    assert_eq!(&*retained.metadata().units, "Pa");
    assert_eq!(
        retained.pcm_window(0, 1, 0).unwrap().sample(0).unwrap(),
        Sample::F32(8.0)
    );
}
#[test]
fn stft_original_complex_bins_and_exact_offsets_survive() {
    let payload = include_bytes!("../../audio/qac/signal/fixtures/stft-n4-complex.le");
    let retained = open(
        build(
            payload,
            Config {
                scalar: 6,
                spectral: true,
                ..Default::default()
            },
            |_| {},
        ),
        1,
    );
    let window = retained.stft_window(0, 1, 0).unwrap();
    assert_eq!(window.original_interleaved_bytes(), payload);
    assert_eq!(window.bins(), 4);
    for (bin, expected) in [
        (0, (0.0, 0.0)),
        (1, (2.0, 0.0)),
        (2, (0.0, 0.0)),
        (3, (2.0, 0.0)),
    ] {
        assert_eq!(
            window.sample(0, bin).unwrap(),
            Sample::ComplexF32 {
                real: expected.0,
                imag: expected.1
            }
        );
    }
    for (bin, hz) in [(0, 0), (1, 1), (2, -2), (3, -1)] {
        let f = window.frequency_offset(bin).unwrap();
        assert_eq!((f.numerator, f.denominator), (hz, 1));
    }
    assert!(window.frequency_offset(4).is_err());
    assert!(retained.pcm_window(0, 1, 0).is_err());
    assert!(retained.metadata().window_coefficients_le.is_none());
}
#[test]
fn hann_stft_retains_authored_window_and_source_entry_origin_basis() {
    use crate::audio::qac::{
        metadata::{OriginBasis, WindowKind},
        signal::{
            stft::{StftDocument, StftQuantity, StftWriteBuffers, StftWriteOptions},
            writer::{PcmSamples, Provenance, ProvenanceRole, SignalRelation},
            Rate, Timing,
        },
    };
    use crate::q42::field_semantics as sem;
    let pcm = [0.5f32, -0.5, 0.25, -0.25, 0.75, -0.75];
    let doc = StftDocument {
        samples: PcmSamples::F32(&pcm),
        timing: Timing {
            scale: 2,
            epoch_seconds: 10,
            epoch_nanoseconds: 0,
            ticks: Rate::new(96_000, 1).unwrap(),
            samples: Rate::new(48_000, 1).unwrap(),
            start_tick: 0,
        },
        stream_id: 3,
        channels: 1,
        channel_order: "mic-A",
        fft_size: 4,
        hop: 2,
        pcm_purpose: "original-acquisition",
        stft_purpose: "hann-complex-stft",
        quantity: StftQuantity::DigitalSample,
        relation: SignalRelation::DigitalUnrelatedToPressure,
        pcm_provenance: Provenance {
            role: ProvenanceRole::Measured,
            algorithm: None,
            algorithm_version: None,
            instrument_uri: Some("urn:instrument:portal-test"),
            lineage: None,
        },
        stft_provenance: Provenance {
            role: ProvenanceRole::Derived,
            algorithm: Some("urn:etalon:signal:hann-stft-f32"),
            algorithm_version: Some("1"),
            instrument_uri: None,
            lineage: Some("symmetric-hann-radix2-no-padding"),
        },
        original: None,
    };
    let mut mono = [0.0f32; 16];
    let mut fft = [0.0f32; 32];
    let mut win = [0.0f32; 16];
    let mut spectra = [0u8; 4096];
    let mut metadata = [0u8; 8192];
    let mut payload = [0u8; 4096];
    let mut file = [0u8; 16384];
    let mut out = vec![0u8; 16384];
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    let n = doc
        .encode_into(
            StftWriteBuffers {
                mono: &mut mono,
                fft: &mut fft,
                window: &mut win,
                spectra: &mut spectra,
                metadata: &mut metadata,
                payload: &mut payload,
                file: &mut file,
            },
            &mut sem::Scratch {
                metadata: &mut m,
                entries: &mut e,
            },
            &mut out,
            StftWriteOptions::default(),
        )
        .unwrap();
    let retained = open(out[..n].to_vec(), doc.stft_entry_index());
    let meta = retained.metadata();
    assert_eq!(meta.epistemic_role, "derived");
    let tx = meta.transform.unwrap();
    assert_eq!(tx.window_kind(), WindowKind::HannSymmetricF32);
    assert_eq!(tx.origin_basis(), OriginBasis::SourceEntryFirstSample);
    let coeffs = meta.window_coefficients_le.as_deref().unwrap();
    assert_eq!(coeffs.len(), 16);
    for (i, bytes) in coeffs.chunks_exact(4).enumerate() {
        assert_eq!(bytes, &win[i].to_le_bytes());
    }
}
#[test]
fn bounds_selection_and_whole_document_integrity_fail_closed() {
    let bytes = build(
        &[1f32.to_le_bytes(), 2f32.to_le_bytes()].concat(),
        Config {
            source: true,
            ..Default::default()
        },
        |_| {},
    );
    assert!(RetainedQac::open(bytes.clone().into_boxed_slice(), 0, Limits::default()).is_err());
    assert!(RetainedQac::open(bytes.clone().into_boxed_slice(), 1, Limits::default()).is_err()); // opaque entry
    let retained = open(bytes.clone(), 2);
    assert!(retained.pcm_window(0, 3, 0).is_err());
    assert!(retained.pcm_window(0, 1, 1).is_err());
    assert!(retained.pcm_window(u64::MAX, 1, 0).is_err());
    let empty = retained.pcm_window(2, 0, 0).unwrap();
    assert_eq!(empty.records(), 0);
    assert!(empty.sample(0).is_err());
    assert!(empty.time_at(0).is_err());
    let mut corrupted = bytes.clone();
    let view =
        StructuralView::open(&bytes, Family::QacV1Candidate, StorageLimits::default()).unwrap();
    corrupted[view.entry(1).unwrap().payload_offset() as usize] ^= 1;
    assert!(RetainedQac::open(corrupted.into_boxed_slice(), 2, Limits::default()).is_err()); // even unselected opaque source
    for limits in [
        Limits {
            memory_bytes: 0,
            ..Default::default()
        },
        Limits {
            signal: crate::audio::qac::signal::Limits {
                work: 0,
                ..Default::default()
            },
            ..Default::default()
        },
        Limits {
            shared: crate::q42::field_semantics::Limits {
                work: 0,
                ..Default::default()
            },
            ..Default::default()
        },
    ] {
        assert!(RetainedQac::open(bytes.clone().into_boxed_slice(), 2, limits).is_err());
    }
    let limited = RetainedQac::open(
        bytes.into_boxed_slice(),
        2,
        Limits {
            window_records: 1,
            window_samples: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(limited.pcm_window(0, 2, 0).is_err());
}
#[test]
fn checked_scalar_decoder_rejects_wrong_width_nonfinite_and_preserves_bits() {
    assert!(decode_sample(Scalar::U8, &[1, 2]).is_err());
    assert!(decode_sample(Scalar::I16, &[0]).is_err());
    assert!(decode_sample(Scalar::F32, &f32::NAN.to_le_bytes()).is_err());
    assert!(decode_sample(Scalar::ComplexF32, &[0, 0, 0, 0, 0, 0, 128, 127]).is_err());
    let b = (-0.0f32).to_le_bytes();
    let mut unaligned = vec![99];
    unaligned.extend_from_slice(&b);
    let Sample::F32(v) = decode_sample(Scalar::F32, &unaligned[1..]).unwrap() else {
        panic!()
    };
    assert_eq!(v.to_bits(), (-0.0f32).to_bits());
    assert_eq!(decode_sample(Scalar::U8, &[255]).unwrap(), Sample::U8(255));
}
