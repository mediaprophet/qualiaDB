//! One-step sparse Lucas–Kanade optical flow (translation per point).

use crate::cv::buffer::GrayView;
use crate::cv::error::CvError;
use crate::cv::features::fast_corners_u8::Keypoint;

/// Estimate flow from `prev` to `next` for each keypoint. Writes dx,dy into `flow` as pairs.
pub fn lucas_kanade_step(
    prev: GrayView<'_>,
    next: GrayView<'_>,
    kps: &[Keypoint],
    n: usize,
    flow_xy: &mut [f32],
) -> Result<usize, CvError> {
    if prev.width != next.width || prev.height != next.height {
        return Err(CvError::DimensionMismatch);
    }
    let n = n.min(kps.len()).min(flow_xy.len() / 2);
    let w = prev.width as i32;
    let h = prev.height as i32;
    for i in 0..n {
        let x = kps[i].x as i32;
        let y = kps[i].y as i32;
        // Spatial gradients + temporal over 3x3
        let mut ix2 = 0.0f32;
        let mut iy2 = 0.0f32;
        let mut ixiy = 0.0f32;
        let mut ixt = 0.0f32;
        let mut iyt = 0.0f32;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let x0 = (x + dx).clamp(1, w - 2);
                let y0 = (y + dy).clamp(1, h - 2);
                let ix = (prev.pixel((x0 + 1) as u32, y0 as u32) as f32
                    - prev.pixel((x0 - 1) as u32, y0 as u32) as f32)
                    * 0.5;
                let iy = (prev.pixel(x0 as u32, (y0 + 1) as u32) as f32
                    - prev.pixel(x0 as u32, (y0 - 1) as u32) as f32)
                    * 0.5;
                let it = next.pixel(x0 as u32, y0 as u32) as f32
                    - prev.pixel(x0 as u32, y0 as u32) as f32;
                ix2 += ix * ix;
                iy2 += iy * iy;
                ixiy += ix * iy;
                ixt += ix * it;
                iyt += iy * it;
            }
        }
        let det = ix2 * iy2 - ixiy * ixiy;
        let (dx, dy) = if det.abs() < 1e-3 {
            (0.0, 0.0)
        } else {
            ((iy2 * (-ixt) - ixiy * (-iyt)) / det, (ix2 * (-iyt) - ixiy * (-ixt)) / det)
        };
        flow_xy[i * 2] = dx;
        flow_xy[i * 2 + 1] = dy;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runs_identity() {
        let img = [10u8; 25];
        let v = GrayView::new(5, 5, 5, &img).unwrap();
        let k = [Keypoint {
            x: 2,
            y: 2,
            score: 1,
        }];
        let mut f = [0.0f32; 2];
        let n = lucas_kanade_step(v, v, &k, 1, &mut f).unwrap();
        assert_eq!(n, 1);
    }
}
