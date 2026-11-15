use super::*;
const V1: &[u8] = include_bytes!("fixtures/q4au-v1-contract.qac");
const V2: &[u8] = include_bytes!("fixtures/q4au-v2-contract.qac");
const V3: &[u8] = include_bytes!("fixtures/q4au-v3-contract.qac");
fn parse(bytes: &[u8]) -> Result<LegacyQacView<'_>, QacReadError> {
    LegacyQacView::parse(bytes, QacReadLimits::default())
}
#[test]
fn independent_legacy_corpus_admission() {
    for (bytes, version) in [(V1, 1), (V2, 2), (V3, 3)] {
        let view = parse(bytes).unwrap();
        assert_eq!(view.info().version, version);
        assert_eq!(view.as_bytes(), bytes);
        assert!(view.spectral_frame(view.info().frames - 1).is_some());
        assert!(view.spectral_frame(view.info().frames).is_none());
    }
    let invalid: &[&[u8]] = &[
        include_bytes!("fixtures/q4au-v1-truncated.qac"),
        include_bytes!("fixtures/q4au-v2-truncated.qac"),
        include_bytes!("fixtures/q4au-v3-truncated.qac"),
        include_bytes!("fixtures/q4au-ascii-spelling.qac"),
        include_bytes!("fixtures/q4au-count-u32-max.qac"),
        include_bytes!("fixtures/q4au-v3-hoa-expansion.qac"),
        include_bytes!("fixtures/q4au-v3-hrir-taps.qac"),
        include_bytes!("fixtures/q4au-v3-hrir-overlap.qac"),
    ];
    for bytes in invalid {
        assert!(parse(bytes).is_err());
    }
}
#[test]
fn rejects_every_truncation_and_trailing_byte() {
    for bytes in [V1, V2, V3] {
        for end in 0..bytes.len() {
            assert!(
                parse(&bytes[..end]).is_err(),
                "version {} length {end}",
                bytes[4]
            );
        }
        let mut extra = bytes.to_vec();
        extra.push(0);
        assert_eq!(parse(&extra).unwrap_err(), QacReadError::Trailing);
    }
}
#[test]
fn all_alignments_read_exact_scalar_values() {
    for bytes in [V1, V2, V3] {
        let expected = parse(bytes).unwrap();
        for offset in 0..8 {
            let mut copy = vec![0; offset];
            copy.extend_from_slice(bytes);
            let view = parse(&copy[offset..]).unwrap();
            assert_eq!(view.info(), expected.info());
            for frame in 0..view.info().frames {
                let a = view.spectral_frame(frame).unwrap();
                let b = expected.spectral_frame(frame).unwrap();
                assert!(a
                    .iter()
                    .zip(b.iter())
                    .all(|(a, b)| a.to_bits() == b.to_bits()));
                if view.info().mel_count > 0 {
                    assert_eq!(
                        view.mel_frame(frame).unwrap().as_bytes(),
                        expected.mel_frame(frame).unwrap().as_bytes()
                    );
                }
                if view.info().mfcc_count > 0 {
                    assert_eq!(
                        view.mfcc_frame(frame).unwrap().as_bytes(),
                        expected.mfcc_frame(frame).unwrap().as_bytes()
                    );
                }
            }
            assert_eq!(view.hrir(0), expected.hrir(0));
        }
    }
}
#[test]
fn scalar_bits_and_limits_preserved() {
    let mut bytes = V1.to_vec();
    let bits = [0u32, 0x80000000, 1, 0x7f7fffff];
    assert_eq!(bytes.len(), 36);
    for (dst, bits) in bytes[20..].chunks_exact_mut(4).zip(bits) {
        dst.copy_from_slice(&bits.to_le_bytes());
    }
    let view = parse(&bytes).unwrap();
    let got: Vec<u32> = (0..view.info().frames)
        .flat_map(|i| view.spectral_frame(i).unwrap().iter().map(f32::to_bits))
        .collect();
    assert_eq!(got, bits);
    assert!(view.spectral_frame(0).unwrap().get(usize::MAX).is_none());
    for limits in [
        QacReadLimits {
            max_bytes: 35,
            ..Default::default()
        },
        QacReadLimits {
            max_frames: 0,
            ..Default::default()
        },
        QacReadLimits {
            max_bins: 0,
            ..Default::default()
        },
    ] {
        assert_eq!(
            LegacyQacView::parse(&bytes, limits).unwrap_err(),
            QacReadError::Limit
        );
    }
}
#[test]
fn nonfinite_samples_and_header_metadata_fail_closed() {
    for pattern in [
        f32::NAN.to_bits(),
        f32::INFINITY.to_bits(),
        f32::NEG_INFINITY.to_bits(),
    ] {
        let mut bytes = V1.to_vec();
        bytes[20..24].copy_from_slice(&pattern.to_le_bytes());
        assert_eq!(parse(&bytes).unwrap_err(), QacReadError::Nonfinite);
    }
    for (offset, value) in [(36, f32::NAN), (40, -1.0), (56, 1.01)] {
        let mut bytes = V3.to_vec();
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        let crc = crate::container_10d::crc32c::crc32c(&bytes[..60]);
        bytes[60..64].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(parse(&bytes).unwrap_err(), QacReadError::Metadata);
    }
    let mut bytes = V3.to_vec();
    bytes[60] ^= 1;
    assert_eq!(parse(&bytes).unwrap_err(), QacReadError::Crc);
}
#[test]
fn hrir_bounds_and_fixed_taps() {
    let view = parse(V3).unwrap();
    assert_eq!(view.info().hrir_directions, 1);
    let hrir = view.hrir(0).unwrap();
    assert_eq!(hrir.left_fir.len(), 64);
    assert_eq!(hrir.right_fir.len(), 64);
    assert!(view.hrir(1).is_none());
    assert!(view.hrir(u32::MAX).is_none());
    assert!(parse(V1).unwrap().hrir(0).is_none());
}
