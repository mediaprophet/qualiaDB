//! Fixed-buffer preprocess helpers (Phase 1/V1).
//!
//! Hot path: no heap. Caller supplies workspace and output buffers.

use crate::types::{Detection, ImageView, PixelFormat, VisionError};

/// Nearest-neighbour resize RGB8 → RGB8 into caller buffer (`out_w * out_h * 3` bytes).
pub fn resize_nearest_rgb8(
    src: ImageView<'_>,
    out_w: u32,
    out_h: u32,
    out: &mut [u8],
) -> Result<(), VisionError> {
    if !src.is_well_formed() || out_w == 0 || out_h == 0 {
        return Err(VisionError::MalformedImage);
    }
    let need = (out_w as usize)
        .saturating_mul(out_h as usize)
        .saturating_mul(3);
    if out.len() < need {
        return Err(VisionError::OutputBufferTooSmall);
    }
    let bpp = src.bytes_per_pixel() as usize;
    for oy in 0..out_h {
        let sy = ((oy as u64 * src.height as u64) / out_h as u64) as u32;
        for ox in 0..out_w {
            let sx = ((ox as u64 * src.width as u64) / out_w as u64) as u32;
            let soff = (sy as usize)
                .saturating_mul(src.row_stride as usize)
                .saturating_add((sx as usize).saturating_mul(bpp));
            let doff = ((oy * out_w + ox) * 3) as usize;
            let (r, g, b) = sample_rgb(src, soff, bpp);
            out[doff] = r;
            out[doff + 1] = g;
            out[doff + 2] = b;
        }
    }
    Ok(())
}

#[inline]
fn sample_rgb(src: ImageView<'_>, off: usize, bpp: usize) -> (u8, u8, u8) {
    if off + bpp > src.bytes.len() {
        return (0, 0, 0);
    }
    match src.format {
        PixelFormat::Gray8 => {
            let g = src.bytes[off];
            (g, g, g)
        }
        PixelFormat::Rgb8 | PixelFormat::Rgba8 => {
            (src.bytes[off], src.bytes[off + 1], src.bytes[off + 2])
        }
        PixelFormat::Bgr8 => (src.bytes[off + 2], src.bytes[off + 1], src.bytes[off]),
        PixelFormat::RgbF32 => (0, 0, 0),
    }
}

/// Normalize RGB8 to planar f32 CHW in `out` (len ≥ 3 * w * h). Mean/std per channel.
pub fn normalize_rgb8_to_f32_chw(
    rgb: &[u8],
    w: u32,
    h: u32,
    mean: [f32; 3],
    std: [f32; 3],
    out: &mut [f32],
) -> Result<(), VisionError> {
    let n = (w as usize).saturating_mul(h as usize);
    if rgb.len() < n * 3 || out.len() < n * 3 {
        return Err(VisionError::OutputBufferTooSmall);
    }
    let plane = n;
    for i in 0..n {
        let r = rgb[i * 3] as f32 / 255.0;
        let g = rgb[i * 3 + 1] as f32 / 255.0;
        let b = rgb[i * 3 + 2] as f32 / 255.0;
        out[i] = (r - mean[0]) / std[0];
        out[plane + i] = (g - mean[1]) / std[1];
        out[2 * plane + i] = (b - mean[2]) / std[2];
    }
    Ok(())
}

/// IoU of two axis-aligned boxes in normalized u16 coordinates.
#[inline]
pub fn iou_u16(a: &Detection, b: &Detection) -> f32 {
    let ax0 = a.x_min_u16 as f32;
    let ay0 = a.y_min_u16 as f32;
    let ax1 = a.x_max_u16 as f32;
    let ay1 = a.y_max_u16 as f32;
    let bx0 = b.x_min_u16 as f32;
    let by0 = b.y_min_u16 as f32;
    let bx1 = b.x_max_u16 as f32;
    let by1 = b.y_max_u16 as f32;
    let ix0 = ax0.max(bx0);
    let iy0 = ay0.max(by0);
    let ix1 = ax1.min(bx1);
    let iy1 = ay1.min(by1);
    let iw = (ix1 - ix0).max(0.0);
    let ih = (iy1 - iy0).max(0.0);
    let inter = iw * ih;
    let area_a = (ax1 - ax0).max(0.0) * (ay1 - ay0).max(0.0);
    let area_b = (bx1 - bx0).max(0.0) * (by1 - by0).max(0.0);
    let uni = area_a + area_b - inter;
    if uni <= 0.0 {
        0.0
    } else {
        inter / uni
    }
}

/// Class-agnostic NMS into `out` (copies survivors). Returns count written.
/// `workspace` must hold at least `n` bool flags as bytes (0/1).
pub fn nms_class_agnostic(
    dets: &[Detection],
    n: usize,
    iou_thresh: f32,
    out: &mut [Detection],
    workspace: &mut [u8],
) -> Result<usize, VisionError> {
    let n = n.min(dets.len());
    if out.is_empty() {
        return Err(VisionError::OutputBufferTooSmall);
    }
    if workspace.len() < n {
        return Err(VisionError::WorkspaceTooSmall);
    }
    // Selection sort by score descending into index order using workspace as suppressed flags.
    for i in 0..n {
        workspace[i] = 0;
    }
    let mut order = [0u16; 256];
    let n_ord = n.min(256);
    for i in 0..n_ord {
        order[i] = i as u16;
    }
    for i in 0..n_ord {
        let mut best = i;
        for j in (i + 1)..n_ord {
            if dets[order[j] as usize].score_u16 > dets[order[best] as usize].score_u16 {
                best = j;
            }
        }
        order.swap(i, best);
    }
    let mut written = 0usize;
    for oi in 0..n_ord {
        let i = order[oi] as usize;
        if workspace[i] != 0 {
            continue;
        }
        if written >= out.len() {
            break;
        }
        out[written] = dets[i];
        written += 1;
        for oj in (oi + 1)..n_ord {
            let j = order[oj] as usize;
            if workspace[j] != 0 {
                continue;
            }
            if iou_u16(&dets[i], &dets[j]) > iou_thresh {
                workspace[j] = 1;
            }
        }
    }
    Ok(written)
}

/// Workspace bytes for letterbox RGB8 of size `out_w × out_h`.
pub fn letterbox_workspace_bytes(out_w: u32, out_h: u32) -> usize {
    (out_w as usize)
        .saturating_mul(out_h as usize)
        .saturating_mul(3)
}

/// Letterbox RGB into square/out buffer with grey pad (114). Returns scale and pad (x,y) as u32 pixels.
pub fn letterbox_rgb8(
    src: ImageView<'_>,
    out_w: u32,
    out_h: u32,
    out: &mut [u8],
) -> Result<(f32, u32, u32), VisionError> {
    if !src.is_well_formed() || out_w == 0 || out_h == 0 {
        return Err(VisionError::MalformedImage);
    }
    let need = letterbox_workspace_bytes(out_w, out_h);
    if out.len() < need {
        return Err(VisionError::OutputBufferTooSmall);
    }
    for b in out.iter_mut().take(need) {
        *b = 114;
    }
    let scale = (out_w as f32 / src.width as f32).min(out_h as f32 / src.height as f32);
    let nw = ((src.width as f32 * scale) as u32).max(1).min(out_w);
    let nh = ((src.height as f32 * scale) as u32).max(1).min(out_h);
    let pad_x = (out_w - nw) / 2;
    let pad_y = (out_h - nh) / 2;
    // Resize into temp region via nearest into full out then we already padded — write into ROI.
    let bpp = src.bytes_per_pixel() as usize;
    for oy in 0..nh {
        let sy = ((oy as u64 * src.height as u64) / nh as u64) as u32;
        for ox in 0..nw {
            let sx = ((ox as u64 * src.width as u64) / nw as u64) as u32;
            let soff = (sy as usize)
                .saturating_mul(src.row_stride as usize)
                .saturating_add((sx as usize).saturating_mul(bpp));
            let dx = pad_x + ox;
            let dy = pad_y + oy;
            let doff = ((dy * out_w + dx) * 3) as usize;
            let (r, g, b) = sample_rgb(src, soff, bpp);
            out[doff] = r;
            out[doff + 1] = g;
            out[doff + 2] = b;
        }
    }
    Ok((scale, pad_x, pad_y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Detection;

    #[test]
    fn resize_preserves_red() {
        let mut rgb = [0u8; 12];
        rgb[0] = 255;
        rgb[1] = 0;
        rgb[2] = 0;
        // 2x2: first pixel red
        let img = ImageView {
            bytes: &rgb,
            width: 2,
            height: 2,
            row_stride: 6,
            format: PixelFormat::Rgb8,
        };
        let mut out = [0u8; 3];
        resize_nearest_rgb8(img, 1, 1, &mut out).unwrap();
        assert_eq!(out[0], 255);
    }

    #[test]
    fn iou_identical_is_one() {
        let d = Detection {
            class_hash: 1,
            instance_hash: 2,
            score_u16: 1000,
            x_min_u16: 1000,
            y_min_u16: 1000,
            x_max_u16: 2000,
            y_max_u16: 2000,
            frame_index: 0,
            track_id: 0,
            flags: 0,
        };
        assert!((iou_u16(&d, &d) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn nms_suppresses_overlap() {
        let mut a = Detection::empty();
        a.score_u16 = 50000;
        a.x_min_u16 = 0;
        a.y_min_u16 = 0;
        a.x_max_u16 = 30000;
        a.y_max_u16 = 30000;
        let mut b = a;
        b.score_u16 = 40000;
        b.x_min_u16 = 1000;
        b.y_min_u16 = 1000;
        let dets = [a, b];
        let mut out = [Detection::empty(); 4];
        let mut ws = [0u8; 8];
        let n = nms_class_agnostic(&dets, 2, 0.3, &mut out, &mut ws).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].score_u16, 50000);
    }
}
