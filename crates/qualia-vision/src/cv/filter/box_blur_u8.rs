//! Separable box blur on Gray8 (caller-buffered).

use crate::cv::buffer::GrayView;
use crate::cv::error::CvError;

/// Box blur with odd radius `r` (kernel 2r+1). `out` ≥ w*h.
pub fn box_blur_u8(src: GrayView<'_>, r: u32, out: &mut [u8]) -> Result<(), CvError> {
    let w = src.width as usize;
    let h = src.height as usize;
    if out.len() < w * h {
        return Err(CvError::BufferTooSmall);
    }
    if r == 0 {
        for y in 0..h {
            for x in 0..w {
                out[y * w + x] = src.pixel(x as u32, y as u32);
            }
        }
        return Ok(());
    }
    let k = (2 * r + 1) as i32;
    for y in 0..h {
        for x in 0..w {
            let mut s = 0u32;
            let mut n = 0u32;
            let yi = y as i32;
            let xi = x as i32;
            for dy in -((r as i32))..=(r as i32) {
                let yy = (yi + dy).clamp(0, h as i32 - 1) as u32;
                for dx in -((r as i32))..=(r as i32) {
                    let xx = (xi + dx).clamp(0, w as i32 - 1) as u32;
                    s += src.pixel(xx, yy) as u32;
                    n += 1;
                }
            }
            let _ = k;
            out[y * w + x] = (s / n.max(1)) as u8;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_spreads() {
        let mut img = vec![0u8; 25];
        img[12] = 255;
        let v = GrayView::new(5, 5, 5, &img).unwrap();
        let mut out = [0u8; 25];
        box_blur_u8(v, 1, &mut out).unwrap();
        assert!(out[12] < 255 && out[12] > 0);
    }
}
