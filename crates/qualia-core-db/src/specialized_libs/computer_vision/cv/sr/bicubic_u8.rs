//! Bicubic (Keys a=-0.5) upscale RGB packed u8 by integer scale 2|3|4.

use crate::specialized_libs::computer_vision::cv::buffer::RgbView;
use crate::specialized_libs::computer_vision::cv::error::CvError;

/// Keys cubic parameter (Catmull-Rom-class).
const A: f32 = -0.5;

/// Bicubic upsample RGB8. `out` ≥ `w*scale * h*scale * 3`. Scale ∈ {2,3,4}.
pub fn bicubic_u8(src: RgbView<'_>, scale: u8, out: &mut [u8]) -> Result<(), CvError> {
    if scale < 2 || scale > 4 {
        return Err(CvError::InvalidParameter);
    }
    let w = src.width;
    let h = src.height;
    if w == 0 || h == 0 {
        return Err(CvError::EmptyInput);
    }
    let out_w = w
        .checked_mul(scale as u32)
        .ok_or(CvError::InvalidParameter)?;
    let out_h = h
        .checked_mul(scale as u32)
        .ok_or(CvError::InvalidParameter)?;
    let need = (out_w as usize)
        .checked_mul(out_h as usize)
        .and_then(|n| n.checked_mul(3))
        .ok_or(CvError::InvalidParameter)?;
    if out.len() < need {
        return Err(CvError::BufferTooSmall);
    }

    let s = scale as f32;
    let wi = w as i32;
    let hi = h as i32;

    for oy in 0..out_h {
        let sy = (oy as f32 + 0.5) / s - 0.5;
        let y_base = sy.floor() as i32;
        let fy = sy - y_base as f32;
        for ox in 0..out_w {
            let sx = (ox as f32 + 0.5) / s - 0.5;
            let x_base = sx.floor() as i32;
            let fx = sx - x_base as f32;

            let mut r_acc = 0.0f32;
            let mut g_acc = 0.0f32;
            let mut b_acc = 0.0f32;
            let mut w_acc = 0.0f32;

            for j in -1..=2 {
                let wy = cubic_weight(fy - j as f32);
                let yy = clamp_i(y_base + j, 0, hi - 1) as u32;
                for i in -1..=2 {
                    let wx = cubic_weight(fx - i as f32);
                    let weight = wx * wy;
                    let xx = clamp_i(x_base + i, 0, wi - 1) as u32;
                    let (r, g, b) = src.pixel(xx, yy);
                    r_acc += r as f32 * weight;
                    g_acc += g as f32 * weight;
                    b_acc += b as f32 * weight;
                    w_acc += weight;
                }
            }

            let inv = if w_acc.abs() > 1e-6 { 1.0 / w_acc } else { 0.0 };
            let doff = ((oy * out_w + ox) * 3) as usize;
            out[doff] = (r_acc * inv).round().clamp(0.0, 255.0) as u8;
            out[doff + 1] = (g_acc * inv).round().clamp(0.0, 255.0) as u8;
            out[doff + 2] = (b_acc * inv).round().clamp(0.0, 255.0) as u8;
        }
    }
    Ok(())
}

/// Keys cubic kernel weight for distance `t` (sample offset from fractional coord).
#[inline]
fn cubic_weight(t: f32) -> f32 {
    let t = t.abs();
    if t <= 1.0 {
        (A + 2.0) * t * t * t - (A + 3.0) * t * t + 1.0
    } else if t < 2.0 {
        A * t * t * t - 5.0 * A * t * t + 8.0 * A * t - 4.0 * A
    } else {
        0.0
    }
}

#[inline]
fn clamp_i(v: i32, lo: i32, hi: i32) -> i32 {
    v.max(lo).min(hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_by_two_to_four_runs() {
        let img = [
            255u8, 0, 0, 0, 255, 0, //
            0, 0, 255, 128, 128, 128,
        ];
        let v = RgbView::new(2, 2, 6, &img).unwrap();
        let mut out = [0u8; 4 * 4 * 3];
        bicubic_u8(v, 2, &mut out).unwrap();
        // Top-left should remain predominantly red
        assert!(out[0] > out[1] && out[0] > out[2]);
        // Output filled (not all zero after real samples)
        assert!(out.iter().any(|&c| c > 0));
    }

    #[test]
    fn scale_three_dims() {
        let img = [10u8, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
        let v = RgbView::new(2, 2, 6, &img).unwrap();
        let mut out = [0u8; 6 * 6 * 3];
        bicubic_u8(v, 3, &mut out).unwrap();
        // Corner-ish sample near first pixel
        assert!(out[0] > 0);
    }

    #[test]
    fn rejects_scale_one() {
        let img = [1u8, 2, 3];
        let v = RgbView::new(1, 1, 3, &img).unwrap();
        let mut out = [0u8; 3];
        assert_eq!(bicubic_u8(v, 1, &mut out), Err(CvError::InvalidParameter));
    }
}
