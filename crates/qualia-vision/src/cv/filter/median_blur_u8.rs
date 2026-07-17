//! 3x3 median filter Gray8.

use crate::cv::buffer::GrayView;
use crate::cv::error::CvError;

pub fn median_blur_u8(src: GrayView<'_>, out: &mut [u8]) -> Result<(), CvError> {
    let w = src.width as i32;
    let h = src.height as i32;
    if out.len() < (w * h) as usize {
        return Err(CvError::BufferTooSmall);
    }
    for y in 0..h {
        for x in 0..w {
            let mut win = [0u8; 9];
            let mut i = 0usize;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let xx = (x + dx).clamp(0, w - 1) as u32;
                    let yy = (y + dy).clamp(0, h - 1) as u32;
                    win[i] = src.pixel(xx, yy);
                    i += 1;
                }
            }
            win.sort_unstable();
            out[(y * w + x) as usize] = win[4];
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_salt() {
        let img = [0u8, 0, 0, 0, 255, 0, 0, 0, 0];
        let v = GrayView::new(3, 3, 3, &img).unwrap();
        let mut out = [0u8; 9];
        median_blur_u8(v, &mut out).unwrap();
        assert_eq!(out[4], 0);
    }
}
