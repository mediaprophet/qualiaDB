//! Bilinear upscale RGB packed u8 by integer scale 2|3|4.

use crate::specialized_libs::computer_vision::cv::buffer::RgbView;
use crate::specialized_libs::computer_vision::cv::error::CvError;

/// Bilinear upsample RGB8. `out` ≥ `w*scale * h*scale * 3`. Scale ∈ {2,3,4}.
pub fn bilinear_u8(src: RgbView<'_>, scale: u8, out: &mut [u8]) -> Result<(), CvError> {
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
    let wm1 = (w.saturating_sub(1)) as f32;
    let hm1 = (h.saturating_sub(1)) as f32;

    for oy in 0..out_h {
        let sy = (oy as f32 + 0.5) / s - 0.5;
        let sy = sy.clamp(0.0, hm1);
        let y0 = sy.floor() as u32;
        let y1 = (y0 + 1).min(h - 1);
        let fy = sy - y0 as f32;
        for ox in 0..out_w {
            let sx = (ox as f32 + 0.5) / s - 0.5;
            let sx = sx.clamp(0.0, wm1);
            let x0 = sx.floor() as u32;
            let x1 = (x0 + 1).min(w - 1);
            let fx = sx - x0 as f32;

            let (r00, g00, b00) = src.pixel(x0, y0);
            let (r10, g10, b10) = src.pixel(x1, y0);
            let (r01, g01, b01) = src.pixel(x0, y1);
            let (r11, g11, b11) = src.pixel(x1, y1);

            let r = lerp2(r00, r10, r01, r11, fx, fy);
            let g = lerp2(g00, g10, g01, g11, fx, fy);
            let b = lerp2(b00, b10, b01, b11, fx, fy);

            let doff = ((oy * out_w + ox) * 3) as usize;
            out[doff] = r;
            out[doff + 1] = g;
            out[doff + 2] = b;
        }
    }
    Ok(())
}

#[inline]
fn lerp2(c00: u8, c10: u8, c01: u8, c11: u8, fx: f32, fy: f32) -> u8 {
    let v0 = c00 as f32 + (c10 as f32 - c00 as f32) * fx;
    let v1 = c01 as f32 + (c11 as f32 - c01 as f32) * fx;
    let v = v0 + (v1 - v0) * fy;
    v.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_by_two_to_four_preserves_corners() {
        // Checker: TL white, TR black, BL black, BR white
        let img = [
            255u8, 255, 255, 0, 0, 0, //
            0, 0, 0, 255, 255, 255,
        ];
        let v = RgbView::new(2, 2, 6, &img).unwrap();
        let mut out = [0u8; 4 * 4 * 3];
        bilinear_u8(v, 2, &mut out).unwrap();
        // Top-left output region should stay near white
        assert!(out[0] > 200);
        // Bottom-right pixel near white
        let br = (15 * 3) as usize;
        assert!(out[br] > 200);
    }

    #[test]
    fn rejects_bad_scale() {
        let img = [1u8, 2, 3];
        let v = RgbView::new(1, 1, 3, &img).unwrap();
        let mut out = [0u8; 12];
        assert_eq!(bilinear_u8(v, 5, &mut out), Err(CvError::InvalidParameter));
    }

    #[test]
    fn buffer_too_small() {
        let img = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let v = RgbView::new(2, 2, 6, &img).unwrap();
        let mut out = [0u8; 10];
        assert_eq!(bilinear_u8(v, 2, &mut out), Err(CvError::BufferTooSmall));
    }
}
