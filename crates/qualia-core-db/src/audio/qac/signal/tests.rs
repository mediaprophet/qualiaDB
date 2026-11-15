use super::*;
use crate::q42::{
    field_envelope::{self as env, EntryInput, EnvelopeInput, Family, StructuralView},
    field_semantics as sem,
};
use super::test_support::*;
const PCM: &[u8] = include_bytes!("fixtures/pcm-f32-special.le");
const COMPLEX: &[u8] = include_bytes!("fixtures/stft-n4-complex.le");
fn read<'a>(bytes: &'a [u8], selected: u32) -> Result<SignalView<'a>, Error> {
    let view = StructuralView::open(bytes, Family::QacV1Candidate, env::Limits::default())?;
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    admit(
        &view,
        selected,
        &mut sem::Scratch {
            metadata: &mut m,
            entries: &mut e,
        },
        sem::Limits::default(),
        Limits::default(),
    )
}
#[test]
fn f32_expected_bits_exact_clocks_and_all_alignments() {
    let bytes = build(PCM, Config::default(), |_| {});
    for shift in 0..8 {
        let mut unaligned = vec![0; shift];
        unaligned.extend_from_slice(&bytes);
        let view = read(&unaligned[shift..], 1).unwrap();
        assert_eq!(view.as_bytes(), PCM);
        for (i, bits) in [0u32, 0x80000000, 1, 0x3f800000, 0xbf800000, 0x7f7fffff]
            .into_iter()
            .enumerate()
        {
            let Sample::F32(v) = view.sample(i as u64, 0, 0).unwrap() else {
                panic!()
            };
            assert_eq!(v.to_bits(), bits);
        }
        assert_eq!(
            view.timing().unwrap().samples,
            Rate::new(48_000_000, 1001).unwrap()
        );
        assert_eq!(
            view.time_at(5).unwrap(),
            ExactTime {
                numerator: 1001,
                denominator: 9_600_000
            }
        );
        assert!(view.sample(u64::MAX, 0, 0).is_err());
        assert!(view.sample(0, u32::MAX, 0).is_err());
        assert!(view.sample(0, 0, u32::MAX).is_err());
    }
}
#[test]
fn integer_precision_and_f64_bits_survive() {
    let i24 = include_bytes!("fixtures/pcm-i24-extrema.le");
    let bytes = build(
        i24,
        Config {
            scalar: 4,
            ..Default::default()
        },
        |_| {},
    );
    let view = read(&bytes, 1).unwrap();
    for (i, n) in [-8388608, 8388607, 0, -1].into_iter().enumerate() {
        assert_eq!(view.sample(i as u64, 0, 0).unwrap(), Sample::I24(n));
    }
    for (scalar, bytes, expected) in [
        (
            3,
            vec![0, 128, 255, 127],
            vec![Sample::I16(i16::MIN), Sample::I16(i16::MAX)],
        ),
        (
            5,
            vec![0, 0, 0, 128, 255, 255, 255, 127],
            vec![Sample::I32(i32::MIN), Sample::I32(i32::MAX)],
        ),
    ] {
        let object = build(
            &bytes,
            Config {
                scalar,
                ..Default::default()
            },
            |_| {},
        );
        let view = read(&object, 1).unwrap();
        for (i, x) in expected.into_iter().enumerate() {
            assert_eq!(view.sample(i as u64, 0, 0).unwrap(), x)
        }
    }
    let expected = [
        0u64,
        0x8000000000000000,
        1,
        0x3ff0000000000000,
        0x7fefffffffffffff,
    ];
    let payload: Vec<u8> = expected.iter().flat_map(|b| b.to_le_bytes()).collect();
    let object = build(
        &payload,
        Config {
            scalar: 2,
            ..Default::default()
        },
        |_| {},
    );
    let view = read(&object, 1).unwrap();
    for (i, bits) in expected.into_iter().enumerate() {
        let Sample::F64(v) = view.sample(i as u64, 0, 0).unwrap() else {
            panic!()
        };
        assert_eq!(v.to_bits(), bits)
    }
}
#[test]
fn complex_stft_expected_bins_and_frame_cadence() {
    let bytes = build(
        COMPLEX,
        Config {
            spectral: true,
            scalar: 6,
            ..Default::default()
        },
        |_| {},
    );
    let view = read(&bytes, 1).unwrap();
    for (k, n) in [0f32, 2., 0., 2.].into_iter().enumerate() {
        let Sample::ComplexF32 { real, imag } = view.sample(0, 0, k as u32).unwrap() else {
            panic!()
        };
        assert_eq!(real.to_bits(), n.to_bits());
        assert_eq!(imag.to_bits(), 0)
    }
    assert_eq!(view.timing().unwrap().samples, Rate::new(2, 1).unwrap());
    assert_eq!(
        view.transform().unwrap().input_rate,
        Rate::new(4, 1).unwrap()
    );
    let magnitude = [0f32, 2., 0., 2.]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<_>>();
    assert!(read(
        &build(
            &magnitude,
            Config {
                spectral: true,
                ..Default::default()
            },
            |_| {}
        ),
        1
    )
    .is_ok());
}
#[test]
fn exact_rational_time_separates_epoch_ticks_and_sample_clock() {
    let t = Timing {
        scale: 2,
        epoch_seconds: 0,
        epoch_nanoseconds: 0,
        ticks: Rate::new(96_000, 1).unwrap(),
        samples: Rate::new(48_000_000, 1001).unwrap(),
        start_tick: 0,
    };
    assert_eq!(
        t.time_at(1001).unwrap(),
        ExactTime {
            numerator: 1_002_001,
            denominator: 48_000_000
        }
    );
    let t = Timing {
        epoch_seconds: -2,
        epoch_nanoseconds: 125_000_000,
        start_tick: -24_000,
        ..t
    };
    assert_eq!(
        t.time_at(0).unwrap(),
        ExactTime {
            numerator: -17,
            denominator: 8
        }
    );
    assert!(Rate::new(0, 1).is_err());
    assert!(Rate::new(2, 2).is_err());
    assert!(Rate {
        numerator: 0,
        denominator: 0
    }
    .divided_by(1)
    .is_err());
    assert!(Rate::new(1, 1).unwrap().divided_by(0).is_err());
    let bytes = build(
        PCM,
        Config {
            start: -3,
            ..Default::default()
        },
        |bs| {
            field(bs, 1, 2).bytes = (-3i64).to_le_bytes().to_vec();
            field(bs, 1, 3).bytes = 1u32.to_le_bytes().to_vec();
        },
    );
    let timing = read(&bytes, 1).unwrap().timing().unwrap();
    assert_eq!(timing.start_tick, -3);
    assert_eq!(timing.epoch_seconds, -3);
    assert_eq!(timing.epoch_nanoseconds, 1);
}
#[test]
fn rejects_invalid_shape_scalar_unit_clock_reference_and_critical_field() {
    let bad: Vec<Box<dyn FnOnce(&mut Vec<B>)>> = vec![
        Box::new(|bs| field(bs, 3, 14).bytes[..8].copy_from_slice(&999u64.to_le_bytes())),
        Box::new(|bs| field(bs, 6, 11).bytes = 2u32.to_le_bytes().to_vec()),
        Box::new(|bs| field(bs, 6, 8).bytes = b"Pa".to_vec()),
        Box::new(|bs| field(bs, 1, 5).bytes = 0u64.to_le_bytes().to_vec()),
        Box::new(|bs| field(bs, 6, 1).bytes = 99u32.to_le_bytes().to_vec()),
        Box::new(|bs| {
            bs.iter_mut()
                .find(|b| b.id == 4)
                .unwrap()
                .fields
                .push(strf(6, "unknown-critical"))
        }),
        Box::new(|bs| field(bs, 6, 11).tag = 11),
        Box::new(|bs| field(bs, 6, 9).critical = 0),
        Box::new(|bs| field(bs, 4, 4).bytes = "authority\u{0085}command".as_bytes().to_vec()),
    ];
    for change in bad {
        let bytes = build(PCM, Config::default(), |bs| {
            change(bs);
            for b in bs {
                for f in &mut b.fields {
                    let width = match f.tag {
                        1 | 4 | 8 | 9 | 11 => 4,
                        2 | 3 | 5 => 8,
                        _ => 1,
                    };
                    f.count = (f.bytes.len() / width) as u32;
                }
            }
        });
        assert!(
            StructuralView::open(&bytes, Family::QacV1Candidate, env::Limits::default()).is_ok()
        );
        assert!(read(&bytes, 1).is_err());
    }
    let payload = [0u8; 16];
    let bytes = build(
        &payload,
        Config {
            channels: 2,
            ..Default::default()
        },
        |bs| field(bs, 4, 1).bytes = b"same,same".to_vec(),
    );
    assert!(read(&bytes, 1).is_err());
}
#[test]
fn unsupported_profile_pairs_do_not_block_structural_archival() {
    for (id, key, value) in [
        (5, 1, "CQT-complex"),
        (5, 6, "unqualified-normalization"),
        (6, 7, "spectral_density"),
        (4, 1, "implicit-stereo"),
    ] {
        let bytes = build(
            COMPLEX,
            Config {
                spectral: true,
                scalar: 6,
                ..Default::default()
            },
            |bs| {
                let f = field(bs, id, key);
                f.bytes = value.as_bytes().to_vec();
                f.count = f.bytes.len() as u32;
            },
        );
        assert!(
            StructuralView::open(&bytes, Family::QacV1Candidate, env::Limits::default()).is_ok()
        );
        assert!(read(&bytes, 1).is_err());
    }
    let bytes = build(
        COMPLEX,
        Config {
            spectral: true,
            scalar: 6,
            start: 1,
            ..Default::default()
        },
        |_| {},
    );
    assert!(read(&bytes, 1).is_err());
}
#[test]
fn pressure_unknown_traceability_retained_and_encoding_not_applied() {
    let bytes = build(PCM, Config::default(), |bs| {
        *field(bs, 6, 7) = strf(7, "pressure");
        *field(bs, 6, 8) = strf(8, "Pa");
        *field(bs, 4, 5) = strf(5, "physical-base-units");
    });
    let view = read(&bytes, 1).unwrap();
    assert_eq!(view.units(), "Pa");
    assert_eq!(view.as_bytes(), PCM);
    let bad = build(PCM, Config::default(), |bs| {
        field(bs, 4, 2).bytes = 2f64.to_le_bytes().to_vec()
    });
    let view = read(&bad, 1).unwrap();
    assert_eq!(view.units_per_stored_sample(), 2.0);
    let Sample::F32(v) = view.sample(3, 0, 0).unwrap() else {
        panic!()
    };
    assert_eq!(v, 1.0); // raw access never applies encoding/calibration twice
}
#[test]
fn rejects_nonfinite_payload_and_admission_budgets() {
    for bits in [0x7fc00000u32, 0x7f800000, 0xff800000] {
        let mut p = PCM.to_vec();
        p[..4].copy_from_slice(&bits.to_le_bytes());
        assert!(matches!(
            read(&build(&p, Config::default(), |_| {}), 1),
            Err(Error::Nonfinite)
        ));
    }
    let bytes = build(PCM, Config::default(), |_| {});
    let v = StructuralView::open(&bytes, Family::QacV1Candidate, env::Limits::default()).unwrap();
    let mut m = [sem::NodeState::default(); 16];
    let mut e = m;
    for limits in [
        Limits {
            samples: 0,
            ..Default::default()
        },
        Limits {
            payload_bytes: 0,
            ..Default::default()
        },
        Limits {
            work: 0,
            ..Default::default()
        },
    ] {
        assert!(admit(
            &v,
            1,
            &mut sem::Scratch {
                metadata: &mut m,
                entries: &mut e
            },
            sem::Limits::default(),
            limits
        )
        .is_err());
    }
    assert!(admit(
        &v,
        1,
        &mut sem::Scratch {
            metadata: &mut [],
            entries: &mut []
        },
        sem::Limits::default(),
        Limits::default()
    )
    .is_err());
    let at = v.entry(1).unwrap().payload_offset() as usize;
    let mut corrupt = bytes.clone();
    corrupt[at] ^= 1;
    assert!(matches!(
        read(&corrupt, 1),
        Err(Error::Envelope(env::Error::PayloadDigest))
    ));
}
fn reencode(bytes: &[u8], final_out: &mut [u8]) -> Result<usize, Error> {
    let view = StructuralView::open(bytes, Family::QacV1Candidate, env::Limits::default())?;
    let mut entries = Vec::new();
    for i in 1..=view.entry_count() {
        let e = view.entry(i)?;
        entries.push(EntryInput {
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
            payload: view.verify_chunk(i)?,
        });
    }
    let input = EnvelopeInput {
        family: Family::QacV1Candidate,
        flags: 0,
        metadata: view.metadata_bytes(),
        entries: &entries,
    };
    let mut staging = vec![0xcc; bytes.len()];
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    encode(
        input,
        env::Limits::default(),
        sem::Limits::default(),
        Limits::default(),
        &mut sem::Scratch {
            metadata: &mut m,
            entries: &mut e,
        },
        &mut staging,
        final_out,
    )
}
#[test]
fn buffered_encode_preserves_originals_and_final_output_on_semantic_error() {
    let bytes = build(
        PCM,
        Config {
            source: true,
            ..Default::default()
        },
        |_| {},
    );
    assert_eq!(read(&bytes, 1).unwrap().as_bytes(), ORIGINAL);
    assert_eq!(read(&bytes, 2).unwrap().as_bytes(), PCM);
    let mut out = vec![0xa5; bytes.len() + 16];
    let n = reencode(&bytes, &mut out).unwrap();
    assert_eq!(&out[..n], &bytes);
    assert_eq!(&out[n..], &[0xa5; 16]);
    let bad = build(PCM, Config::default(), |bs| {
        *field(bs, 6, 8) = strf(8, "Pa")
    });
    let original = bad.clone();
    let mut out = vec![0xa5; bad.len()];
    assert!(reencode(&bad, &mut out).is_err());
    assert!(out.iter().all(|b| *b == 0xa5));
    assert_eq!(bad, original);
    let mut short = vec![0xa5; bytes.len() - 1];
    assert!(reencode(&bytes, &mut short).is_err());
    assert!(short.iter().all(|b| *b == 0xa5));
}
#[test]
fn rate_divided_by_reduces_before_denominator_multiplication() {
    let rate = Rate::new(1_000_000_007 * 999_983, 999_999_999_989).unwrap();
    let divided = rate.divided_by(1_000_000_007).unwrap();
    assert_eq!(
        divided,
        Rate {
            numerator: 999_983,
            denominator: 999_999_999_989
        }
    );
}
