//! CIE L\*a\*b\* (D65) → sRGB8, per-pixel, caller-buffered.
//!
//! Input interleaved f32 `[L, a, b, …]`; output packed RGB8 same length.

use super::HistoError;

const XN: f32 = 0.95047;
const YN: f32 = 1.0;
const ZN: f32 = 1.08883;

const DELTA: f32 = 6.0 / 29.0;

#[inline]
fn lab_f_inv(t: f32) -> f32 {
    if t > DELTA {
        t * t * t
    } else {
        3.0 * DELTA * DELTA * (t - 4.0 / 29.0)
    }
}

#[inline]
fn linear_to_srgb_u8(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Convert one Lab triple to sRGB8 (D65).
#[inline]
pub fn lab_f32_to_rgb_u8(l: f32, a: f32, b: f32) -> (u8, u8, u8) {
    let fy = (l + 16.0) / 116.0;
    let fx = a / 500.0 + fy;
    let fz = fy - b / 200.0;
    let x = XN * lab_f_inv(fx);
    let y = YN * lab_f_inv(fy);
    let z = ZN * lab_f_inv(fz);
    // XYZ → linear sRGB
    let rl = 3.240_454_2 * x - 1.537_138_5 * y - 0.498_531_4 * z;
    let gl = -0.969_266_0 * x + 1.876_010_8 * y + 0.041_556_0 * z;
    let bl = 0.055_643_4 * x - 0.204_025_9 * y + 1.057_225_2 * z;
    (
        linear_to_srgb_u8(rl),
        linear_to_srgb_u8(gl),
        linear_to_srgb_u8(bl),
    )
}

/// Convert interleaved Lab f32 to packed RGB8. `out` length ≥ `lab.len()`.
pub fn lab_to_rgb(lab: &[f32], out: &mut [u8]) -> Result<(), HistoError> {
    if lab.is_empty() {
        return Err(HistoError::EmptyInput);
    }
    if lab.len() % 3 != 0 {
        return Err(HistoError::InvalidParameter);
    }
    if out.len() < lab.len() {
        return Err(HistoError::BufferTooSmall);
    }
    let n = lab.len() / 3;
    for i in 0..n {
        let base = i * 3;
        let (r, g, b) = lab_f32_to_rgb_u8(lab[base], lab[base + 1], lab[base + 2]);
        out[base] = r;
        out[base + 1] = g;
        out[base + 2] = b;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computer_vision::bio::histopathology::rgb_to_lab;

    #[test]
    fn lab_100_is_white() {
        let lab = [100.0f32, 0.0, 0.0];
        let mut rgb = [0u8; 3];
        lab_to_rgb(&lab, &mut rgb).unwrap();
        assert!(rgb[0] >= 250 && rgb[1] >= 250 && rgb[2] >= 250);
    }

    #[test]
    fn roundtrip_from_lab() {
        let rgb_in = [90u8, 140, 200];
        let mut lab = [0f32; 3];
        let mut rgb_out = [0u8; 3];
        rgb_to_lab(&rgb_in, &mut lab).unwrap();
        lab_to_rgb(&lab, &mut rgb_out).unwrap();
        for c in 0..3 {
            let d = (rgb_in[c] as i16 - rgb_out[c] as i16).unsigned_abs();
            assert!(d <= 3, "c={c} {} vs {}", rgb_in[c], rgb_out[c]);
        }
    }
}
