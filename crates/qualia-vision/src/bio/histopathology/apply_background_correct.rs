//! Divide / normalize packed RGB by a background illumination estimate.

use super::background_intensity_sample::RgbBg;
use super::HistoError;

/// Correct RGB illumination: `out = clamp(rgb * 255 / bg, 0..255)` per channel.
///
/// Models flat-field style correction against a measured slide background.
/// `out` length ≥ `rgb.len()`. Channels with near-zero `bg` are left unchanged.
pub fn apply_background_correct(
    rgb: &[u8],
    bg: RgbBg,
    out: &mut [u8],
) -> Result<(), HistoError> {
    if rgb.is_empty() {
        return Err(HistoError::EmptyInput);
    }
    if rgb.len() % 3 != 0 {
        return Err(HistoError::InvalidParameter);
    }
    if out.len() < rgb.len() {
        return Err(HistoError::BufferTooSmall);
    }
    let scale = [
        if bg.r > 1e-3 { 255.0 / bg.r } else { 1.0 },
        if bg.g > 1e-3 { 255.0 / bg.g } else { 1.0 },
        if bg.b > 1e-3 { 255.0 / bg.b } else { 1.0 },
    ];
    let n = rgb.len() / 3;
    for i in 0..n {
        let base = i * 3;
        for c in 0..3 {
            let v = rgb[base + c] as f32 * scale[c];
            out[base + c] = v.round().clamp(0.0, 255.0) as u8;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_gray_bg_to_white() {
        // Uniform mid-gray frame; bg is that gray → should map to ~255.
        let rgb = [128u8, 128, 128, 64, 64, 64];
        let bg = RgbBg {
            r: 128.0,
            g: 128.0,
            b: 128.0,
            n_samples: 1,
        };
        let mut out = [0u8; 6];
        apply_background_correct(&rgb, bg, &mut out).unwrap();
        assert_eq!(out[0], 255);
        assert_eq!(out[1], 255);
        assert_eq!(out[2], 255);
        // 64 * 255/128 = 127.5 → 128
        assert!((out[3] as i16 - 128).unsigned_abs() <= 1);
    }

    #[test]
    fn tinted_bg_balances_channels() {
        let rgb = [100u8, 50, 25];
        let bg = RgbBg {
            r: 200.0,
            g: 100.0,
            b: 50.0,
            n_samples: 1,
        };
        let mut out = [0u8; 3];
        apply_background_correct(&rgb, bg, &mut out).unwrap();
        // Each channel → 100 * 255/200 = 127.5
        assert!((out[0] as i16 - 128).unsigned_abs() <= 1);
        assert!((out[1] as i16 - 128).unsigned_abs() <= 1);
        assert!((out[2] as i16 - 128).unsigned_abs() <= 1);
    }
}
