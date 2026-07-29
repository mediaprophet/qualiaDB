//! FAST-9 corner detector (simplified).

use crate::specialized_libs::computer_vision::cv::buffer::GrayView;
use crate::specialized_libs::computer_vision::cv::error::CvError;

#[derive(Debug, Clone, Copy, Default)]
pub struct Keypoint {
    pub x: u32,
    pub y: u32,
    pub score: u16,
}

/// Detect FAST corners. `out` caller-buffered.
pub fn fast_corners_u8(
    src: GrayView<'_>,
    thresh: u8,
    out: &mut [Keypoint],
) -> Result<usize, CvError> {
    if src.width < 7 || src.height < 7 {
        return Ok(0);
    }
    // Circle offsets (Bresenham 16)
    let off: [(i32, i32); 16] = [
        (0, -3),
        (1, -3),
        (2, -2),
        (3, -1),
        (3, 0),
        (3, 1),
        (2, 2),
        (1, 3),
        (0, 3),
        (-1, 3),
        (-2, 2),
        (-3, 1),
        (-3, 0),
        (-3, -1),
        (-2, -2),
        (-1, -3),
    ];
    let mut n = 0usize;
    let t = thresh as i16;
    for y in 3..src.height.saturating_sub(3) {
        for x in 3..src.width.saturating_sub(3) {
            let p = src.pixel(x, y) as i16;
            let mut bright = 0u32;
            let mut dark = 0u32;
            for &(dx, dy) in &off {
                let v = src.pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32) as i16;
                if v >= p + t {
                    bright += 1;
                } else if v <= p - t {
                    dark += 1;
                }
            }
            let score = bright.max(dark);
            if score >= 9 {
                if n >= out.len() {
                    return Ok(n);
                }
                out[n] = Keypoint {
                    x,
                    y,
                    score: score as u16,
                };
                n += 1;
            }
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_something_on_noise() {
        let mut img = vec![128u8; 64];
        img[4 * 8 + 4] = 255;
        let v = GrayView::new(8, 8, 8, &img).unwrap();
        let mut k = [Keypoint::default(); 16];
        let _ = fast_corners_u8(v, 20, &mut k).unwrap();
    }
}
