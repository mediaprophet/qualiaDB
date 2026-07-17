//! Sobel gradient magnitude Gray8.

use crate::specialized_libs::computer_vision::cv::buffer::GrayView;
use crate::specialized_libs::computer_vision::cv::error::CvError;

pub fn sobel_mag_u8(src: GrayView<'_>, out: &mut [u8]) -> Result<(), CvError> {
    let w = src.width as i32;
    let h = src.height as i32;
    if out.len() < (w * h) as usize {
        return Err(CvError::BufferTooSmall);
    }
    for y in 0..h {
        for x in 0..w {
            let mut gx = 0i32;
            let mut gy = 0i32;
            // Sobel kernels
            let k_x = [-1, 0, 1, -2, 0, 2, -1, 0, 1];
            let k_y = [-1, -2, -1, 0, 0, 0, 1, 2, 1];
            let mut i = 0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let xx = (x + dx).clamp(0, w - 1) as u32;
                    let yy = (y + dy).clamp(0, h - 1) as u32;
                    let p = src.pixel(xx, yy) as i32;
                    gx += p * k_x[i];
                    gy += p * k_y[i];
                    i += 1;
                }
            }
            let mag = ((gx * gx + gy * gy) as f32).sqrt().min(255.0) as u8;
            out[(y * w + x) as usize] = mag;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vertical_edge() {
        let mut img = vec![0u8; 16];
        for y in 0..4 {
            for x in 2..4 {
                img[y * 4 + x] = 255;
            }
        }
        let v = GrayView::new(4, 4, 4, &img).unwrap();
        let mut o = [0u8; 16];
        sobel_mag_u8(v, &mut o).unwrap();
        assert!(o[1 * 4 + 1] > 50 || o[1 * 4 + 2] > 50);
    }
}
