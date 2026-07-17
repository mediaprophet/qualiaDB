//! Tiny BRIEF-like descriptor (32 bytes) around keypoints.

use crate::cv::buffer::GrayView;
use crate::cv::error::CvError;
use crate::cv::features::fast_corners_u8::Keypoint;

pub const DESC_LEN: usize = 32;

/// Write `n` descriptors into `out` (n * DESC_LEN bytes).
pub fn brief_desc_u8(
    src: GrayView<'_>,
    kps: &[Keypoint],
    n: usize,
    out: &mut [u8],
) -> Result<(), CvError> {
    let n = n.min(kps.len());
    if out.len() < n * DESC_LEN {
        return Err(CvError::BufferTooSmall);
    }
    // Fixed test pairs (deterministic pseudo-random)
    let mut pairs = [(0i32, 0i32, 0i32, 0i32); 256];
    let mut s = 0xC0FFEE_u32;
    for p in &mut pairs {
        s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        let a = ((s >> 16) as i32 % 9) - 4;
        s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        let b = ((s >> 16) as i32 % 9) - 4;
        s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        let c = ((s >> 16) as i32 % 9) - 4;
        s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        let d = ((s >> 16) as i32 % 9) - 4;
        *p = (a, b, c, d);
    }
    for (i, kp) in kps.iter().take(n).enumerate() {
        let base = i * DESC_LEN;
        out[base..base + DESC_LEN].fill(0);
        for (bi, &(dx1, dy1, dx2, dy2)) in pairs.iter().enumerate() {
            let x1 = (kp.x as i32 + dx1).clamp(0, src.width as i32 - 1) as u32;
            let y1 = (kp.y as i32 + dy1).clamp(0, src.height as i32 - 1) as u32;
            let x2 = (kp.x as i32 + dx2).clamp(0, src.width as i32 - 1) as u32;
            let y2 = (kp.y as i32 + dy2).clamp(0, src.height as i32 - 1) as u32;
            if src.pixel(x1, y1) < src.pixel(x2, y2) {
                out[base + bi / 8] |= 1 << (bi % 8);
            }
        }
    }
    Ok(())
}
