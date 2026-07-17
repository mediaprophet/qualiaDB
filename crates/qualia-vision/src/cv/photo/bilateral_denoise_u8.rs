//! Simplified bilateral denoise Gray8 (small window).

use crate::cv::buffer::GrayView;
use crate::cv::error::CvError;

pub fn bilateral_denoise_u8(src: GrayView<'_>, out: &mut [u8]) -> Result<(), CvError> {
    let w = src.width as i32;
    let h = src.height as i32;
    if out.len() < (w * h) as usize {
        return Err(CvError::BufferTooSmall);
    }
    let sigma_s2 = 4.0f32;
    let sigma_r2 = 400.0f32;
    for y in 0..h {
        for x in 0..w {
            let p0 = src.pixel(x as u32, y as u32) as f32;
            let mut num = 0.0f32;
            let mut den = 0.0f32;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let xx = (x + dx).clamp(0, w - 1) as u32;
                    let yy = (y + dy).clamp(0, h - 1) as u32;
                    let p = src.pixel(xx, yy) as f32;
                    let ds = (dx * dx + dy * dy) as f32;
                    let dr = (p - p0) * (p - p0);
                    let wgt = (-ds / (2.0 * sigma_s2) - dr / (2.0 * sigma_r2)).exp();
                    num += wgt * p;
                    den += wgt;
                }
            }
            out[(y * w + x) as usize] = (num / den.max(1e-6)) as u8;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_flat() {
        let img = [100u8; 25];
        let v = GrayView::new(5, 5, 5, &img).unwrap();
        let mut o = [0u8; 25];
        bilateral_denoise_u8(v, &mut o).unwrap();
        assert!((o[12] as i16 - 100).abs() < 3);
    }
}
