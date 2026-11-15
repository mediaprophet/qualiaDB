use super::*;

fn fixture(version: u16, bins: u32, frames: u32, mel: u32, mfcc: u32) -> Vec<u32> {
    let header = AudioSpectralSidecarHeader {
        magic: SPECTRAL_SIDECAR_MAGIC,
        version,
        _pad: SIDECAR_KIND_CQT,
        bin_count: bins,
        frame_count: frames,
        sample_rate: 48_000,
    };
    let plane0_end = 20 + bins as usize * frames as usize * 4;
    let len = if version == 2 {
        v2_sidecar_size(frames as usize, bins as usize, mel as usize, mfcc as usize)
    } else {
        plane0_end
    };
    let mut words = vec![0u32; len / 4];
    let bytes: &mut [u8] = bytemuck::cast_slice_mut(&mut words);
    bytes[..20].copy_from_slice(bytemuck::bytes_of(&header));
    for (i, chunk) in bytes[20..plane0_end].chunks_exact_mut(4).enumerate() {
        chunk.copy_from_slice(&(i as f32 + 1.0).to_ne_bytes());
    }
    if version == 2 {
        let sub = SpectralV2SubHeader {
            magic: SPECTRAL_V2_SUBHEADER_MAGIC,
            n_mel: mel,
            n_mfcc: mfcc,
        };
        bytes[plane0_end..plane0_end + 12].copy_from_slice(bytemuck::bytes_of(&sub));
        for (i, chunk) in bytes[plane0_end + 12..].chunks_exact_mut(4).enumerate() {
            chunk.copy_from_slice(&(i as f32 + 101.0).to_ne_bytes());
        }
    }
    words
}

#[test]
fn nonzero_and_last_frames_preserve_all_plane_values() {
    for version in [1, 2] {
        let words = fixture(version, 3, 3, 2, 1);
        let bytes = bytemuck::cast_slice(&words);
        assert_eq!(sidecar_frame_view(bytes, 1), Some(&[4.0, 5.0, 6.0][..]));
        assert_eq!(sidecar_frame_view(bytes, 2), Some(&[7.0, 8.0, 9.0][..]));
        assert!(sidecar_frame_view(bytes, 3).is_none());
        assert!(sidecar_frame_view(bytes, u32::MAX).is_none());
        if version == 2 {
            assert_eq!(sidecar_mel_frame_view(bytes, 1), Some(&[103.0, 104.0][..]));
            assert_eq!(sidecar_mel_frame_view(bytes, 2), Some(&[105.0, 106.0][..]));
            assert_eq!(sidecar_mfcc_frame_view(bytes, 1), Some(&[108.0][..]));
            assert_eq!(sidecar_mfcc_frame_view(bytes, 2), Some(&[109.0][..]));
        } else {
            assert!(sidecar_mel_frame_view(bytes, 0).is_none());
            assert!(sidecar_mfcc_frame_view(bytes, 0).is_none());
        }
        assert!(sidecar_mel_frame_view(bytes, 3).is_none());
        assert!(sidecar_mfcc_frame_view(bytes, u32::MAX).is_none());
    }
}

#[test]
fn every_plane0_truncation_fails_closed_for_even_first_frame() {
    let words = fixture(1, 3, 3, 0, 0);
    let bytes: &[u8] = bytemuck::cast_slice(&words);
    for len in 0..bytes.len() {
        assert!(
            sidecar_frame_view(&bytes[..len], 0).is_none(),
            "length {len}"
        );
    }
    let mut preview = [9.0; SPECTRAL_PREVIEW_BINS];
    assert!(!copy_sidecar_frame_to_preview_bins(
        &bytes[..bytes.len() - 1],
        0,
        &mut preview
    ));
    assert_eq!(preview, [9.0; SPECTRAL_PREVIEW_BINS]);
    assert!(copy_sidecar_frame_to_preview_bins(bytes, 1, &mut preview));
    assert_eq!(&preview[..3], &[4.0, 5.0, 6.0]);
    assert!(preview[3..].iter().all(|v| *v == 0.0));
}

#[test]
fn all_feature_truncations_fail_but_subheader_parser_remains_header_only() {
    let words = fixture(2, 3, 3, 2, 1);
    let bytes: &[u8] = bytemuck::cast_slice(&words);
    for len in 0..bytes.len() {
        assert!(
            sidecar_mel_frame_view(&bytes[..len], 0).is_none(),
            "mel length {len}"
        );
        assert!(
            sidecar_mfcc_frame_view(&bytes[..len], 0).is_none(),
            "mfcc length {len}"
        );
    }
    let plane0_end = 20 + 3 * 3 * 4;
    assert!(parse_v2_subheader(&bytes[..plane0_end + 12]).is_some());
    assert!(parse_v2_subheader(&bytes[..plane0_end + 11]).is_none());
    // v1-compatible access only requires the complete original plane.
    assert_eq!(
        sidecar_frame_view(&bytes[..plane0_end], 1),
        Some(&[4.0, 5.0, 6.0][..])
    );
}

#[test]
fn malformed_feature_subheaders_and_huge_extents_are_rejected() {
    for (magic, mel, mfcc) in [
        (0, 2, 1),
        (SPECTRAL_V2_SUBHEADER_MAGIC, 0, 1),
        (SPECTRAL_V2_SUBHEADER_MAGIC, 2, 0),
        (SPECTRAL_V2_SUBHEADER_MAGIC, u32::MAX, u32::MAX),
    ] {
        let mut words = fixture(2, 3, 3, 2, 1);
        let bytes: &mut [u8] = bytemuck::cast_slice_mut(&mut words);
        let sub = SpectralV2SubHeader {
            magic,
            n_mel: mel,
            n_mfcc: mfcc,
        };
        bytes[56..68].copy_from_slice(bytemuck::bytes_of(&sub));
        assert!(sidecar_mel_frame_view(bytes, 0).is_none());
        assert!(sidecar_mfcc_frame_view(bytes, 0).is_none());
    }
    for version in [1, 2] {
        let header = AudioSpectralSidecarHeader {
            version,
            bin_count: u32::MAX,
            frame_count: u32::MAX,
            ..AudioSpectralSidecarHeader::empty()
        };
        let bytes = bytemuck::bytes_of(&header);
        assert!(sidecar_frame_view(bytes, u32::MAX - 1).is_none());
        assert!(sidecar_frame_view(bytes, 0).is_none());
        assert!(parse_v2_subheader(bytes).is_none());
        assert!(sidecar_mel_frame_view(bytes, u32::MAX - 1).is_none());
        assert!(sidecar_mfcc_frame_view(bytes, u32::MAX - 1).is_none());
    }
}

#[test]
fn unaligned_headers_parse_but_borrowed_float_views_fail_closed() {
    let source = fixture(2, 3, 3, 2, 1);
    let bytes: &[u8] = bytemuck::cast_slice(&source);
    let mut shifted = vec![0u32; source.len() + 1];
    let shifted_bytes: &mut [u8] = bytemuck::cast_slice_mut(&mut shifted);
    shifted_bytes[1..1 + bytes.len()].copy_from_slice(bytes);
    let unaligned = &shifted_bytes[1..1 + bytes.len()];
    assert_ne!(unaligned.as_ptr() as usize % std::mem::align_of::<f32>(), 0);
    assert!(parse_sidecar_header(unaligned).is_some());
    assert!(parse_v2_subheader(unaligned).is_some());
    for frame in [0, 1, 2] {
        assert!(sidecar_frame_view(unaligned, frame).is_none());
        assert!(sidecar_mel_frame_view(unaligned, frame).is_none());
        assert!(sidecar_mfcc_frame_view(unaligned, frame).is_none());
    }
}

#[test]
fn checked_size_rejects_each_overflow_stage_without_wrapping() {
    assert_eq!(checked_v2_sidecar_size(3, 3, 2, 1), Some(104));
    assert_eq!(v2_sidecar_size(3, 3, 2, 1), 104);
    for (frames, bins, mel, mfcc) in [
        (usize::MAX, 2, 0, 0),
        (usize::MAX / 4, 1, 0, 0),
        (2, 1, usize::MAX, 0),
        (2, 1, 0, usize::MAX),
        (1, usize::MAX / 8, usize::MAX / 8, 0),
    ] {
        assert!(checked_v2_sidecar_size(frames, bins, mel, mfcc).is_none());
        assert_eq!(v2_sidecar_size(frames, bins, mel, mfcc), usize::MAX);
    }
}
