use super::*;
use crate::medical::dicom_dataset::{ParseLimits, Part10Dataset, WorkBudget};
const RLE16: &[u8] = include_bytes!("fixtures/rle-i12-two-frames-bot.dcm");
const FRAME16: &[u8] = include_bytes!("fixtures/frame-i12-first.rle");
fn dataset(b: &[u8]) -> Part10Dataset<'_> {
    Part10Dataset::parse(b, ParseLimits::default()).unwrap()
}
fn unchanged_error(
    bytes: &[u8],
    limits: RleLimits,
    out_len: usize,
    scratch_len: usize,
) -> RleError {
    let ds = dataset(bytes);
    let mut out = vec![0x93; out_len];
    let mut scratch = vec![0x57; scratch_len];
    let e = decode_rle_lossless_into(
        &ds,
        &mut out,
        &mut scratch,
        limits,
        &mut WorkBudget::new(10000),
    )
    .err()
    .unwrap();
    assert!(out.iter().all(|b| *b == 0x93));
    assert!(scratch.iter().all(|b| *b == 0x57));
    e
}
fn frame_offset(bytes: &[u8]) -> usize {
    bytes.windows(64).position(|w| w == &FRAME16[..64]).unwrap()
}
#[test]
fn hand_built_msb_planes_decode_exactly_and_match_native_samples() {
    let native = dataset(include_bytes!("fixtures/native-i12-two-frames.dcm"));
    let n = NativePixelView::from_dataset(&native, 1024, &mut WorkBudget::new(10000)).unwrap();
    for bytes in [
        RLE16,
        include_bytes!("fixtures/rle-i12-two-frames-empty-bot.dcm").as_slice(),
    ] {
        let ds = dataset(bytes);
        let mut out = [0xaa; 24];
        let mut scratch = [0xbb; 8];
        let p = decode_rle_lossless_into(
            &ds,
            &mut out,
            &mut scratch,
            RleLimits::default(),
            &mut WorkBudget::new(10000),
        )
        .unwrap();
        assert_eq!(p.stored_sample_bytes(), n.stored_sample_bytes());
        for f in 0..2 {
            for row in 0..2 {
                for col in 0..2 {
                    assert_eq!(
                        p.sample(f, row, col).unwrap(),
                        n.sample(f, row, col).unwrap()
                    );
                }
            }
        }
        assert_eq!(p.source_blake3(), ds.source_blake3());
        assert_eq!(
            p.sample(0, 0, 0).unwrap(),
            super::super::StoredSample::Signed(-2048)
        );
        assert_eq!(
            p.sample(0, 0, 1).unwrap(),
            super::super::StoredSample::Signed(-1)
        );
        assert_eq!(ds.original_bytes(), bytes);
        drop(p);
        assert_eq!(&out[16..], &[0xaa; 8]);
        assert_eq!(&scratch[4..], &[0xbb; 4]);
    }
}
#[test]
fn noop_replicate_and_literals_keep_monochrome1_stored_values() {
    let ds = dataset(include_bytes!("fixtures/rle-u8-noop-repeat.dcm"));
    let mut out = [0; 8];
    let mut scratch = [0; 8];
    let p = decode_rle_lossless_into(
        &ds,
        &mut out,
        &mut scratch,
        RleLimits::default(),
        &mut WorkBudget::new(10000),
    )
    .unwrap();
    assert_eq!(p.photometric(), "MONOCHROME1");
    assert_eq!(p.stored_sample_bytes(), [7, 7, 7, 7, 1, 2, 3, 4]);
    let native = dataset(include_bytes!("fixtures/native-u8.dcm"));
    let n = NativePixelView::from_dataset(&native, 1024, &mut WorkBudget::new(10000)).unwrap();
    assert_eq!(p.stored_sample_bytes(), n.stored_sample_bytes());
}
#[test]
fn buffers_bytes_frames_work_and_query_budget_are_admitted_before_writes() {
    assert_eq!(
        unchanged_error(RLE16, RleLimits::default(), 15, 4),
        RleError::OutputTooSmall
    );
    assert_eq!(
        unchanged_error(RLE16, RleLimits::default(), 16, 3),
        RleError::ScratchTooSmall
    );
    for limits in [
        RleLimits {
            max_decoded_bytes: 15,
            ..Default::default()
        },
        RleLimits {
            max_scratch_bytes: 3,
            ..Default::default()
        },
        RleLimits {
            max_compressed_bytes: 151,
            ..Default::default()
        },
        RleLimits {
            max_frames: 1,
            ..Default::default()
        },
        RleLimits {
            max_work: 0,
            ..Default::default()
        },
    ] {
        unchanged_error(RLE16, limits, 16, 4);
    }
    let ds = dataset(RLE16);
    for steps in (0..2000).step_by(43) {
        let mut out = [0x93; 16];
        let mut scratch = [0x57; 4];
        if decode_rle_lossless_into(
            &ds,
            &mut out,
            &mut scratch,
            RleLimits::default(),
            &mut WorkBudget::new(steps),
        )
        .is_err()
        {
            assert_eq!(out, [0x93; 16]);
            assert_eq!(scratch, [0x57; 4]);
        }
    }
}
#[test]
fn invalid_segment_counts_offsets_unused_words_and_padding_fail_before_decode() {
    let at = frame_offset(RLE16);
    for (offset, value) in [
        (0, 0),
        (0, 3),
        (0, 16),
        (4, 63),
        (4, 66),
        (8, 64),
        (8, 65),
        (8, 9999),
        (12, 4),
    ] {
        let mut b = RLE16.to_vec();
        b[at + offset..at + offset + 4].copy_from_slice(&(value as u32).to_le_bytes());
        unchanged_error(&b, RleLimits::default(), 16, 4);
    }
    let second = at + 76 + 8;
    let mut b = RLE16.to_vec();
    b[second + 69] = 3;
    assert_eq!(
        unchanged_error(&b, RleLimits::default(), 16, 4),
        RleError::TrailingSegmentData
    );
    for len in 0..64 {
        assert!(frame::validate(&FRAME16[..len], 2, 2, 2).is_err());
    }
}
#[test]
fn strict_packbits_rejects_truncation_and_cross_row_or_excess_runs() {
    assert_eq!(
        packbits::walk(&[1, 7], 1, 2, None),
        Err(RleError::TruncatedRun)
    );
    assert_eq!(
        packbits::walk(&[255], 1, 2, None),
        Err(RleError::TruncatedRun)
    );
    assert_eq!(
        packbits::walk(&[2, 1, 2, 3, 0], 2, 2, None),
        Err(RleError::RunCrossesRow)
    );
    assert_eq!(
        packbits::walk(&[254, 7], 1, 2, None),
        Err(RleError::RunCrossesRow)
    );
    assert_eq!(
        packbits::walk(&[0, 7, 0, 9], 1, 1, None),
        Err(RleError::TrailingSegmentData)
    );
    assert_eq!(
        packbits::walk(&[128, 128], 1, 1, None),
        Err(RleError::TruncatedRun)
    );
    let mut out = [0; 128];
    assert!(packbits::walk(&[129, 9], 1, 128, Some(&mut out)).is_ok());
    assert_eq!(out, [9; 128]);
    let mut literal = vec![127];
    literal.extend(0..128);
    literal.push(0);
    assert!(packbits::walk(&literal, 1, 128, Some(&mut out)).is_ok());
    assert_eq!(&out[..4], &[0, 1, 2, 3]);
}
#[test]
fn basic_offset_table_frame_count_high_bit_and_pixel_representation_are_verified() {
    let at = frame_offset(RLE16);
    let bot = at - 16;
    let mut b = RLE16.to_vec();
    b[bot + 4..bot + 8].copy_from_slice(&83u32.to_le_bytes());
    assert_eq!(
        unchanged_error(&b, RleLimits::default(), 16, 4),
        RleError::BasicOffsetTable
    );
    for (tag, value) in [
        (0x00280102, 15u16),
        (0x00280103, 2),
        (0x00280002, 3),
        (0x00280100, 32),
    ] {
        let mut b = include_bytes!("fixtures/rle-i12-two-frames-empty-bot.dcm").to_vec();
        let pattern = [
            (tag >> 16) as u8,
            (tag >> 24) as u8,
            tag as u8,
            (tag >> 8) as u8,
            b'U',
            b'S',
            2,
            0,
        ];
        let at = b.windows(8).position(|x| x == pattern).unwrap() + 8;
        b[at..at + 2].copy_from_slice(&value.to_le_bytes());
        unchanged_error(&b, RleLimits::default(), 64, 4);
    }
    let mut b = include_bytes!("fixtures/rle-i12-two-frames-empty-bot.dcm").to_vec();
    let at = b
        .windows(8)
        .position(|v| v == [0x28, 0, 8, 0, b'I', b'S', 2, 0])
        .unwrap()
        + 8;
    b[at] = b'1';
    assert_eq!(
        unchanged_error(&b, RleLimits::default(), 16, 4),
        RleError::FragmentCount
    );
}
#[test]
fn lossless_view_keeps_source_bound_quantitative_frame_semantics() {
    let ds = dataset(RLE16);
    let frames = crate::medical::dicom_frames::FrameSet::extract(&ds, Default::default()).unwrap();
    let mut out = [0; 16];
    let mut scratch = [0; 4];
    let p = decode_rle_lossless_into(
        &ds,
        &mut out,
        &mut scratch,
        RleLimits::default(),
        &mut WorkBudget::new(10000),
    )
    .unwrap();
    assert_eq!(
        frames.frames[0]
            .quantitative_sample(&p, 0, 0, false)
            .unwrap(),
        Some(-5096.0)
    );
    assert_eq!(
        frames.frames[1]
            .quantitative_sample(&p, 1, 0, false)
            .unwrap(),
        Some(3094.0)
    );
    let profile = super::super::transfer_syntax_profile(ds.transfer_syntax_uid());
    assert!(profile.encapsulated_decode);
    assert!(profile.preserve_original);
}

#[test]
fn signed8_and_unsigned16_extremes_match_independent_native_values() {
    for (rle, native, expected) in [
        (
            include_bytes!("fixtures/rle-i8.dcm").as_slice(),
            include_bytes!("fixtures/native-i8.dcm").as_slice(),
            [-128.0, -1.0, 0.0, 127.0],
        ),
        (
            include_bytes!("fixtures/rle-u16.dcm").as_slice(),
            include_bytes!("fixtures/native-u16.dcm").as_slice(),
            [0.0, 65535.0, 32768.0, 1.0],
        ),
    ] {
        let ds = dataset(rle);
        let source = dataset(native);
        let n = NativePixelView::from_dataset(&source, 1024, &mut WorkBudget::new(10000)).unwrap();
        let mut out = [0; 8];
        let mut scratch = [0; 4];
        let p = decode_rle_lossless_into(
            &ds,
            &mut out,
            &mut scratch,
            RleLimits::default(),
            &mut WorkBudget::new(10000),
        )
        .unwrap();
        assert_eq!(p.stored_sample_bytes(), n.stored_sample_bytes());
        for c in 0..4 {
            assert_eq!(p.sample(0, 0, c).unwrap().as_f64(), expected[c as usize]);
        }
    }
}
