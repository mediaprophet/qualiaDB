//! 3x3 Gaussian blur on Gray8.

use crate::cv::buffer::GrayView;
use crate::cv::error::CvError;

const K: [u32; 9] = [1, 2, 1, 2, 4, 2, 1, 2, 1];

/// Fixed 3×3 Gaussian. `out` ≥ w*h.
pub fn gaussian_blur_u8(src: GrayView<'_>, out: &mut [u8]) -> Result<(), CvError> {
    let w = src.width as i32;
    let h = src.height as i32;
    if out.len() < (w * h) as usize {
        return Err(CvError::BufferTooSmall);
    }
    for y in 0..h {
        for x in 0..w {
            let mut s = 0u32;
            let mut i = 0usize;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let xx = (x + dx).clamp(0, w - 1) as u32;
                    let yy = (y + dy).clamp(0, h - 1) as u32;
                    s += src.pixel(xx, yy) as u32 * K[i];
                    i += 1;
                }
            }
            out[(y * w + x) as usize] = (s / 16) as u8;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smooths_impulse() {
        let mut img = vec![0u8; 9];
        img[4] = 255;
        let v = GrayView::new(3, 3, 3, &img).unwrap();
        let mut out = [0u8; 9];
        gaussian_blur_u8(v, &mut out).unwrap();
        assert_eq!(out[4], 63); // 4*255/16
    }
}
