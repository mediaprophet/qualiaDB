//! Face ROI proxy: bright/skin-like central region (no mesh weights).

use crate::cv::buffer::RgbView;

#[derive(Debug, Clone, Copy, Default)]
pub struct FaceRoi {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Heuristic face box: largest connected-ish high-R region in upper half (excellence: replace with mesh).
pub fn face_roi_center(src: RgbView<'_>) -> FaceRoi {
    let w = src.width;
    let h = src.height;
    // Default central upper ROI for rPPG when detector absent
    let rw = (w / 3).max(8);
    let rh = (h / 4).max(8);
    FaceRoi {
        x: (w.saturating_sub(rw)) / 2,
        y: h / 6,
        w: rw,
        h: rh,
    }
}

/// Mean RGB in ROI into `out` [r,g,b].
pub fn roi_mean_rgb(src: RgbView<'_>, roi: FaceRoi, out: &mut [f32; 3]) {
    let mut s = [0.0f32; 3];
    let mut n = 0.0f32;
    let x1 = roi.x.min(src.width.saturating_sub(1));
    let y1 = roi.y.min(src.height.saturating_sub(1));
    let x2 = (roi.x + roi.w).min(src.width);
    let y2 = (roi.y + roi.h).min(src.height);
    for y in y1..y2 {
        for x in x1..x2 {
            let (r, g, b) = src.pixel(x, y);
            s[0] += r as f32;
            s[1] += g as f32;
            s[2] += b as f32;
            n += 1.0;
        }
    }
    if n > 0.0 {
        out[0] = s[0] / n;
        out[1] = s[1] / n;
        out[2] = s[2] / n;
    }
}
