//! RGB8 → Gray8 (BT.601 integer).

use crate::specialized_libs::computer_vision::cv::buffer::RgbView;
use crate::specialized_libs::computer_vision::cv::error::CvError;

/// Convert packed RGB8 to Gray8. `out` length ≥ width*height.
pub fn rgb_to_gray_u8(src: RgbView<'_>, out: &mut [u8]) -> Result<(), CvError> {
    let n = (src.width * src.height) as usize;
    if out.len() < n {
        return Err(CvError::BufferTooSmall);
    }
    let mut o = 0usize;
    for y in 0..src.height {
        for x in 0..src.width {
            let (r, g, b) = src.pixel(x, y);
            // (77*R + 150*G + 29*B) / 256
            out[o] = ((77u32 * r as u32 + 150 * g as u32 + 29 * b as u32) / 256) as u8;
            o += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_is_bright() {
        let rgb = [255u8, 255, 255, 0, 0, 0];
        let v = RgbView::new(2, 1, 6, &rgb).unwrap();
        let mut g = [0u8; 2];
        rgb_to_gray_u8(v, &mut g).unwrap();
        assert!(g[0] > 250);
        assert!(g[1] < 5);
    }
}
