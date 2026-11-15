use super::*;

#[test]
fn odd_dimensions_area_filter_all_source_texels_and_floor_dimensions() {
    let source = [
        0, 0, 0, 0, 30, 30, 30, 30, 60, 60, 60, 60, 90, 90, 90, 90, 120, 120, 120, 120, 150, 150,
        150, 150, 180, 180, 180, 180, 210, 210, 210, 210, 240, 240, 240, 240,
    ];
    let mut out = [0u8; 2 * 4 * 4];
    assert_eq!(
        downsample_rgba8_level_into(&source, 3, 3, CpuTextureMipSemantic::LinearData, &mut out),
        Ok((1, 1))
    );
    // Floor-halved 3x3 is 1x1 and area filtering covers the full source extent.
    assert_eq!(&out[..4], &[120, 120, 120, 120]);

    let source = [42u8; 5 * 2 * 4];
    let mut out = [0u8; 2 * 1 * 4];
    assert_eq!(
        downsample_rgba8_level_into(&source, 5, 2, CpuTextureMipSemantic::LinearData, &mut out),
        Ok((2, 1))
    );
    assert_eq!(out, [42; 8]);
}

#[test]
fn insufficient_output_is_unchanged() {
    let source = [255u8; 4 * 4 * 4];
    let mut out = [0xA5; 15];
    assert_eq!(
        downsample_rgba8_level_into(&source, 4, 4, CpuTextureMipSemantic::Color, &mut out),
        Err(CpuTextureMipError::OutputTooSmall {
            needed: 16,
            available: 15
        })
    );
    assert_eq!(out, [0xA5; 15]);
}

#[test]
fn color_filter_averages_in_linear_light_and_preserves_linear_alpha() {
    let source = [
        0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 64, 255, 255, 255, 192,
    ];
    let mut color = [0u8; 4];
    let mut linear = [0u8; 4];
    downsample_rgba8_level_into(&source, 2, 2, CpuTextureMipSemantic::Color, &mut color).unwrap();
    downsample_rgba8_level_into(
        &source,
        2,
        2,
        CpuTextureMipSemantic::LinearData,
        &mut linear,
    )
    .unwrap();
    assert!(color[0] > linear[0]);
    assert_eq!(color[0], color[1]);
    assert_eq!(color[0], color[2]);
    assert_eq!(color[3], 128);
    assert_eq!(linear[3], 128);
    assert_eq!(linear[0], 128);
}

#[test]
fn normal_filter_renormalizes_rgb_and_averages_alpha() {
    let source = [
        255, 128, 128, 0, 128, 255, 128, 64, 0, 128, 128, 128, 128, 0, 255, 255,
    ];
    let mut out = [0u8; 4];
    downsample_rgba8_level_into(&source, 2, 2, CpuTextureMipSemantic::Normal, &mut out).unwrap();
    let n = [
        f32::from(out[0]) / 127.5 - 1.0,
        f32::from(out[1]) / 127.5 - 1.0,
        f32::from(out[2]) / 127.5 - 1.0,
    ];
    let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    assert!((length - 1.0).abs() < 0.015);
    assert_eq!(out[3], 112);
}

#[test]
fn alpha_mask_is_explicitly_rejected_without_coverage_correction() {
    let source = [0u8; 16];
    let mut out = [0x6D; 4];
    assert_eq!(
        downsample_rgba8_level_into(
            &source,
            2,
            2,
            CpuTextureMipSemantic::AlphaMask { cutoff: 0.5 },
            &mut out
        ),
        Err(CpuTextureMipError::AlphaCoverageCorrectionUnavailable)
    );
    assert_eq!(out, [0x6D; 4]);
    assert_eq!(
        downsample_rgba8_level_into(
            &source,
            2,
            2,
            CpuTextureMipSemantic::AlphaMask { cutoff: f32::NAN },
            &mut out
        ),
        Err(CpuTextureMipError::InvalidAlphaCutoff)
    );
}

#[test]
fn alpha_mask_preserves_mixed_authored_coverage_after_quantization() {
    let mut authored = [0u8; 4 * 4 * 4];
    let mut source = [0u8; 4 * 4 * 4];
    for y in 0..4 {
        for x in 0..2 {
            authored[(y * 4 + x) * 4 + 3] = 255;
        }
    }
    // The four 2x2 source blocks have raw alpha coverage 1.0, 0.75, 0.25, and 0.0.
    // The authored base has 50% coverage, so the output can preserve two of four pixels.
    for y in 0..4 {
        for x in 0..4 {
            let covered = match (x < 2, y < 2) {
                (true, true) => true,
                (false, true) => !(x == 3 && y == 1),
                (true, false) => x == 0 && y == 2,
                (false, false) => false,
            };
            source[(y * 4 + x) * 4 + 3] = if covered { 255 } else { 0 };
        }
    }
    let mut out = [0u8; 2 * 2 * 4];
    assert_eq!(
        downsample_alpha_mask_level_into(&authored, 4, 4, &source, 4, 4, 0.5, &mut out),
        Ok((2, 2))
    );
    let covered = out
        .chunks_exact(4)
        .filter(|pixel| f32::from(pixel[3]) / 255.0 >= 0.5)
        .count();
    assert_eq!(covered, 2);
}

#[test]
fn alpha_mask_opaque_and_transparent_extremes_are_preserved() {
    let mut opaque = [0u8; 4 * 4 * 4];
    for pixel in opaque.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[255, 128, 0, 255]);
    }
    let mut out = [0u8; 2 * 2 * 4];
    downsample_alpha_mask_level_into(&opaque, 4, 4, &opaque, 4, 4, 0.5, &mut out).unwrap();
    assert!(out.chunks_exact(4).all(|pixel| pixel[3] >= 128));
    assert!(out.chunks_exact(4).all(|pixel| pixel[0] == 255));

    let transparent = [0u8; 4 * 4 * 4];
    downsample_alpha_mask_level_into(&transparent, 4, 4, &transparent, 4, 4, 0.5, &mut out)
        .unwrap();
    assert!(out.chunks_exact(4).all(|pixel| pixel[3] == 0));
}

#[test]
fn alpha_mask_selects_the_nearest_attainable_coverage_count() {
    let mut authored = [0u8; 4 * 4 * 4];
    for pixel in authored.chunks_exact_mut(4).take(6) {
        pixel[3] = 255;
    }
    // The two output pixels have equal raw alpha, so quantization can cover zero or two pixels.
    // The authored target is 0.75 covered pixels; zero is the closer attainable count.
    let source = [255u8; 4 * 2 * 4];
    let mut out = [0u8; 2 * 1 * 4];
    assert_eq!(
        downsample_alpha_mask_level_into(&authored, 4, 4, &source, 4, 2, 0.5, &mut out),
        Ok((2, 1))
    );
    let covered = out
        .chunks_exact(4)
        .filter(|pixel| f32::from(pixel[3]) / 255.0 >= 0.5)
        .count();
    assert_eq!(covered, 0);
    let target = 6.0f32 / 16.0 * 2.0;
    assert!((covered as f32 - target).abs() <= (2.0 - target).abs());
}

#[test]
fn alpha_mask_odd_dimensions_are_deterministic_and_cover_full_extent() {
    let mut authored = [0u8; 5 * 3 * 4];
    let mut source = [0u8; 5 * 3 * 4];
    for (index, pixel) in authored.chunks_exact_mut(4).enumerate() {
        pixel[3] = if index % 3 == 0 { 255 } else { 0 };
    }
    for (index, pixel) in source.chunks_exact_mut(4).enumerate() {
        pixel[0] = (index * 11) as u8;
        pixel[1] = (index * 7) as u8;
        pixel[2] = (index * 3) as u8;
        pixel[3] = if index % 2 == 0 { 255 } else { 0 };
    }
    let mut first = [0u8; 2 * 1 * 4];
    let mut second = [0u8; 2 * 1 * 4];
    assert_eq!(
        downsample_alpha_mask_level_into(&authored, 5, 3, &source, 5, 3, 0.4, &mut first),
        Ok((2, 1))
    );
    assert_eq!(
        downsample_alpha_mask_level_into(&authored, 5, 3, &source, 5, 3, 0.4, &mut second),
        Ok((2, 1))
    );
    assert_eq!(first, second);
}

#[test]
fn alpha_mask_validation_errors_leave_destination_unchanged() {
    let source = [255u8; 4 * 4 * 4];
    let mut out = [0xA7; 15];
    assert_eq!(
        downsample_alpha_mask_level_into(&source, 4, 4, &source, 4, 4, 0.5, &mut out),
        Err(CpuTextureMipError::OutputTooSmall {
            needed: 16,
            available: 15
        })
    );
    assert_eq!(out, [0xA7; 15]);
    assert_eq!(
        downsample_alpha_mask_level_into(&source, 4, 4, &source[..8], 4, 4, 0.5, &mut out),
        Err(CpuTextureMipError::SourceLength {
            expected: 64,
            actual: 8
        })
    );
    assert_eq!(out, [0xA7; 15]);
    assert_eq!(
        downsample_alpha_mask_level_into(&source, 4, 4, &source, 4, 4, f32::NAN, &mut out),
        Err(CpuTextureMipError::InvalidAlphaCutoff)
    );
    assert_eq!(out, [0xA7; 15]);
}
