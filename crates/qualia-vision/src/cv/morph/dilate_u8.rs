//! Gray dilate 3x3 max.

use crate::cv::buffer::GrayView;
use crate::cv::error::CvError;

pub fn dilate_u8(src: GrayView<'_>, out: &mut [u8]) -> Result<(), CvError> {
    let w = src.width as i32;
    let h = src.height as i32;
    if out.len() < (w * h) as usize {
        return Err(CvError::BufferTooSmall);
    }
    for y in 0..h {
        for x in 0..w {
            let mut m = 0u8;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let xx = (x + dx).clamp(0, w - 1) as u32;
                    let yy = (y + dy).clamp(0, h - 1) as u32;
                    m = m.max(src.pixel(xx, yy));
                }
            }
            out[(y * w + x) as usize] = m;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grows_bright() {
        let img = [0u8; 9];
        let mut img = img;
        img[4] = 255;
        let v = GrayView::new(3, 3, 3, &img).unwrap();
        let mut o = [0u8; 9];
        dilate_u8(v, &mut o).unwrap();
        assert_eq!(o[1], 255);
    }
}
