//! sRGB8 → CIE L\*a\*b\* (D65 illuminant), per-pixel, caller-buffered.
//!
//! Output is interleaved f32 `[L, a, b, …]` with L∈[0,100] roughly, a/b signed.

use super::HistoError;

/// D65 white point (CIE 1931 2°).
const XN: f32 = 0.95047;
const YN: f32 = 1.0;
const ZN: f32 = 1.08883;

const DELTA: f32 = 6.0 / 29.0;
const DELTA_CUBE: f32 = DELTA * DELTA * DELTA; // (6/29)³

#[inline]
fn srgb_u8_to_linear(c: u8) -> f32 {
    let v = c as f32 / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

#[inline]
fn lab_f(t: f32) -> f32 {
    if t > DELTA_CUBE {
        t.cbrt()
    } else {
        t / (3.0 * DELTA * DELTA) + 4.0 / 29.0
    }
}

/// Convert one sRGB8 triple to CIE Lab (D65).
#[inline]
pub fn rgb_u8_to_lab_f32(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let rl = srgb_u8_to_linear(r);
    let gl = srgb_u8_to_linear(g);
    let bl = srgb_u8_to_linear(b);
    // sRGB → XYZ (D65)
    let x = 0.412_456_4 * rl + 0.357_576_1 * gl + 0.180_437_5 * bl;
    let y = 0.212_672_9 * rl + 0.715_152_2 * gl + 0.072_175_0 * bl;
    let z = 0.019_333_9 * rl + 0.119_192_0 * gl + 0.950_304_1 * bl;
    let fx = lab_f(x / XN);
    let fy = lab_f(y / YN);
    let fz = lab_f(z / ZN);
    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let bb = 200.0 * (fy - fz);
    (l, a, bb)
}

/// Convert packed RGB8 to interleaved Lab f32. `out` length ≥ `rgb.len()`.
pub fn rgb_to_lab(rgb: &[u8], out: &mut [f32]) -> Result<(), HistoError> {
    if rgb.is_empty() {
        return Err(HistoError::EmptyInput);
    }
    if rgb.len() % 3 != 0 {
        return Err(HistoError::InvalidParameter);
    }
    if out.len() < rgb.len() {
        return Err(HistoError::BufferTooSmall);
    }
    let n = rgb.len() / 3;
    for i in 0..n {
        let base = i * 3;
        let (l, a, b) = rgb_u8_to_lab_f32(rgb[base], rgb[base + 1], rgb[base + 2]);
        out[base] = l;
        out[base + 1] = a;
        out[base + 2] = b;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computer_vision::bio::histopathology::lab_to_rgb;

    #[test]
    fn white_high_l() {
        let rgb = [255u8, 255, 255];
        let mut lab = [0f32; 3];
        rgb_to_lab(&rgb, &mut lab).unwrap();
        assert!((lab[0] - 100.0).abs() < 0.5, "L={}", lab[0]);
        assert!(lab[1].abs() < 1.0);
        assert!(lab[2].abs() < 1.0);
    }

    #[test]
    fn black_low_l() {
        let rgb = [0u8, 0, 0];
        let mut lab = [0f32; 3];
        rgb_to_lab(&rgb, &mut lab).unwrap();
        assert!(lab[0] < 1.0, "L={}", lab[0]);
    }

    #[test]
    fn roundtrip_lab_approx() {
        // Mid greys and a few chromatic samples.
        let samples: &[[u8; 3]] = &[
            [128, 128, 128],
            [200, 50, 50],
            [40, 180, 60],
            [30, 60, 200],
            [255, 255, 0],
            [0, 255, 255],
        ];
        let mut lab = [0f32; 3];
        let mut back = [0u8; 3];
        for s in samples {
            rgb_to_lab(s, &mut lab).unwrap();
            lab_to_rgb(&lab, &mut back).unwrap();
            for c in 0..3 {
                let d = (s[c] as i16 - back[c] as i16).unsigned_abs();
                assert!(
                    d <= 3,
                    "channel {c}: in={} out={} lab={:?}",
                    s[c],
                    back[c],
                    lab
                );
            }
        }
    }
}
