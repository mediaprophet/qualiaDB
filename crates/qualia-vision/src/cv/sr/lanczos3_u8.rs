//! Lanczos-3 upscale RGB packed u8 by integer scale 2|3|4.

use crate::cv::buffer::RgbView;
use crate::cv::error::CvError;

const A: f32 = 3.0;
const PI: f32 = core::f32::consts::PI;

/// Lanczos-3 upsample RGB8. `out` ≥ `w*scale * h*scale * 3`. Scale ∈ {2,3,4}.
pub fn lanczos3_u8(src: RgbView<'_>, scale: u8, out: &mut [u8]) -> Result<(), CvError> {
    if scale < 2 || scale > 4 {
        return Err(CvError::InvalidParameter);
    }
    let w = src.width;
    let h = src.height;
    if w == 0 || h == 0 {
        return Err(CvError::EmptyInput);
    }
    let out_w = w.checked_mul(scale as u32).ok_or(CvError::InvalidParameter)?;
    let out_h = h.checked_mul(scale as u32).ok_or(CvError::InvalidParameter)?;
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
    // Support radius a=3 → taps from floor(x)-a+1 .. floor(x)+a
    let support = A as i32;

    for oy in 0..out_h {
        let sy = (oy as f32 + 0.5) / s - 0.5;
        let y_center = sy;
        for ox in 0..out_w {
            let sx = (ox as f32 + 0.5) / s - 0.5;

            let mut r_acc = 0.0f32;
            let mut g_acc = 0.0f32;
            let mut b_acc = 0.0f32;
            let mut w_acc = 0.0f32;

            let y0 = y_center.floor() as i32 - support + 1;
            let y1 = y_center.floor() as i32 + support;
            let x0 = sx.floor() as i32 - support + 1;
            let x1 = sx.floor() as i32 + support;

            for j in y0..=y1 {
                let wy = lanczos_weight(y_center - j as f32);
                if wy == 0.0 {
                    continue;
                }
                let yy = clamp_i(j, 0, hi - 1) as u32;
                for i in x0..=x1 {
                    let wx = lanczos_weight(sx - i as f32);
                    if wx == 0.0 {
                        continue;
                    }
                    let weight = wx * wy;
                    let xx = clamp_i(i, 0, wi - 1) as u32;
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

/// Lanczos kernel L(x) with window a=3.
#[inline]
fn lanczos_weight(x: f32) -> f32 {
    let ax = x.abs();
    if ax < 1e-6 {
        1.0
    } else if ax >= A {
        0.0
    } else {
        let pi_x = PI * x;
        let pi_x_a = pi_x / A;
        (A * pi_x.sin() * pi_x_a.sin()) / (pi_x * pi_x)
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
            200u8, 100, 50, 10, 20, 30, //
            40, 50, 60, 70, 80, 90,
        ];
        let v = RgbView::new(2, 2, 6, &img).unwrap();
        let mut out = [0u8; 4 * 4 * 3];
        lanczos3_u8(v, 2, &mut out).unwrap();
        assert!(out.iter().any(|&c| c > 0));
        // Near top-left, should resemble first pixel
        assert!(out[0] > 100);
    }

    #[test]
    fn scale_four_dims() {
        let img = [255u8, 128, 64];
        let v = RgbView::new(1, 1, 3, &img).unwrap();
        let mut out = [0u8; 4 * 4 * 3];
        lanczos3_u8(v, 4, &mut out).unwrap();
        // Single-pixel source → uniform-ish upsample of that colour
        assert_eq!(out[0], 255);
        assert_eq!(out[1], 128);
        assert_eq!(out[2], 64);
        assert_eq!(out[4 * 4 * 3 - 3], 255);
    }

    #[test]
    fn buffer_too_small() {
        let img = [1u8, 2, 3];
        let v = RgbView::new(1, 1, 3, &img).unwrap();
        let mut out = [0u8; 4];
        assert_eq!(lanczos3_u8(v, 2, &mut out), Err(CvError::BufferTooSmall));
    }
}
