//! Detection overlay — draw boxes onto caller-owned pixel buffers (desktop/renderer path).
//!
//! Pure Rust, no GPU required. Coordinates use the same normalized u16 boxes as `Detection`.

use crate::types::{Detection, VisionError};

/// Convert a detection box to CSS percentages (left, top, width, height) in 0…100.
#[inline]
pub fn box_css_percent(det: &Detection) -> (f32, f32, f32, f32) {
    let x0 = det.x_min_u16 as f32 / 65535.0 * 100.0;
    let y0 = det.y_min_u16 as f32 / 65535.0 * 100.0;
    let x1 = det.x_max_u16 as f32 / 65535.0 * 100.0;
    let y1 = det.y_max_u16 as f32 / 65535.0 * 100.0;
    (x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
}

/// Pixel bounds (inclusive min, exclusive max) for a detection on an image of size w×h.
#[inline]
pub fn box_pixel_bounds(det: &Detection, w: u32, h: u32) -> (u32, u32, u32, u32) {
    let x0 = ((det.x_min_u16 as u64 * w as u64) / 65535) as u32;
    let y0 = ((det.y_min_u16 as u64 * h as u64) / 65535) as u32;
    let x1 = ((det.x_max_u16 as u64 * w as u64) / 65535) as u32;
    let y1 = ((det.y_max_u16 as u64 * h as u64) / 65535) as u32;
    let x1 = x1.max(x0.saturating_add(1)).min(w);
    let y1 = y1.max(y0.saturating_add(1)).min(h);
    (x0.min(w), y0.min(h), x1, y1)
}

/// Draw axis-aligned box outlines onto an RGBA8 buffer (stride = w * 4).
/// `thickness` in pixels (clamped 1…8). Returns number of boxes drawn.
pub fn draw_boxes_rgba8(
    width: u32,
    height: u32,
    rgba: &mut [u8],
    detections: &[Detection],
    n: usize,
    color: [u8; 4],
    thickness: u32,
) -> Result<usize, VisionError> {
    let need = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4);
    if rgba.len() < need || width == 0 || height == 0 {
        return Err(VisionError::OutputBufferTooSmall);
    }
    let t = thickness.clamp(1, 8);
    let n = n.min(detections.len());
    let mut drawn = 0usize;
    for det in detections.iter().take(n) {
        if det.class_hash == 0 && det.score_u16 == 0 {
            continue;
        }
        let (x0, y0, x1, y1) = box_pixel_bounds(det, width, height);
        stroke_rect_rgba8(width, height, rgba, x0, y0, x1, y1, color, t);
        drawn += 1;
    }
    Ok(drawn)
}

/// Copy RGB8 source into RGBA8 (alpha=255), then draw boxes.
pub fn compose_rgb_overlay_rgba8(
    width: u32,
    height: u32,
    rgb: &[u8],
    detections: &[Detection],
    n: usize,
    color: [u8; 4],
    thickness: u32,
    out_rgba: &mut [u8],
) -> Result<usize, VisionError> {
    let px = (width as usize).saturating_mul(height as usize);
    if rgb.len() < px * 3 || out_rgba.len() < px * 4 {
        return Err(VisionError::OutputBufferTooSmall);
    }
    for i in 0..px {
        out_rgba[i * 4] = rgb[i * 3];
        out_rgba[i * 4 + 1] = rgb[i * 3 + 1];
        out_rgba[i * 4 + 2] = rgb[i * 3 + 2];
        out_rgba[i * 4 + 3] = 255;
    }
    draw_boxes_rgba8(width, height, out_rgba, detections, n, color, thickness)
}

fn stroke_rect_rgba8(
    w: u32,
    h: u32,
    rgba: &mut [u8],
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    color: [u8; 4],
    t: u32,
) {
    // Horizontal edges
    for dy in 0..t {
        let yt = y0.saturating_add(dy).min(h.saturating_sub(1));
        let yb = y1.saturating_sub(1).saturating_sub(dy).min(h.saturating_sub(1));
        for x in x0..x1 {
            put_rgba(w, rgba, x, yt, color);
            put_rgba(w, rgba, x, yb, color);
        }
    }
    // Vertical edges
    for dx in 0..t {
        let xl = x0.saturating_add(dx).min(w.saturating_sub(1));
        let xr = x1.saturating_sub(1).saturating_sub(dx).min(w.saturating_sub(1));
        for y in y0..y1 {
            put_rgba(w, rgba, xl, y, color);
            put_rgba(w, rgba, xr, y, color);
        }
    }
}

#[inline]
fn put_rgba(w: u32, rgba: &mut [u8], x: u32, y: u32, color: [u8; 4]) {
    let i = ((y as usize) * (w as usize) + (x as usize)) * 4;
    if i + 3 < rgba.len() {
        rgba[i] = color[0];
        rgba[i + 1] = color[1];
        rgba[i + 2] = color[2];
        rgba[i + 3] = color[3];
    }
}

/// Encode RGBA8 as a minimal uncompressed 32-bit BMP into `out`.
/// Returns bytes written. Need ≥ 54 + w*h*4.
pub fn encode_bmp_rgba8(
    width: u32,
    height: u32,
    rgba: &[u8],
    out: &mut [u8],
) -> Result<usize, VisionError> {
    let px = (width as usize).saturating_mul(height as usize);
    let data_len = px * 4;
    let file_len = 54 + data_len;
    if rgba.len() < data_len || out.len() < file_len || width == 0 || height == 0 {
        return Err(VisionError::OutputBufferTooSmall);
    }
    // BITMAPFILEHEADER
    out[0] = b'B';
    out[1] = b'M';
    write_u32_le(&mut out[2..6], file_len as u32);
    write_u32_le(&mut out[6..10], 0);
    write_u32_le(&mut out[10..14], 54);
    // BITMAPINFOHEADER
    write_u32_le(&mut out[14..18], 40);
    write_i32_le(&mut out[18..22], width as i32);
    // Negative height = top-down rows (matches our buffer order).
    write_i32_le(&mut out[22..26], -(height as i32));
    write_u16_le(&mut out[26..28], 1);
    write_u16_le(&mut out[28..30], 32);
    write_u32_le(&mut out[30..34], 0);
    write_u32_le(&mut out[34..38], data_len as u32);
    write_u32_le(&mut out[38..42], 2835);
    write_u32_le(&mut out[42..46], 2835);
    write_u32_le(&mut out[46..50], 0);
    write_u32_le(&mut out[50..54], 0);
    // BGRA pixel order for BMP
    for i in 0..px {
        let s = i * 4;
        let d = 54 + i * 4;
        out[d] = rgba[s + 2];
        out[d + 1] = rgba[s + 1];
        out[d + 2] = rgba[s];
        out[d + 3] = rgba[s + 3];
    }
    Ok(file_len)
}

#[inline]
fn write_u16_le(dst: &mut [u8], v: u16) {
    dst[0] = (v & 0xFF) as u8;
    dst[1] = (v >> 8) as u8;
}

#[inline]
fn write_u32_le(dst: &mut [u8], v: u32) {
    dst[0] = (v & 0xFF) as u8;
    dst[1] = ((v >> 8) & 0xFF) as u8;
    dst[2] = ((v >> 16) & 0xFF) as u8;
    dst[3] = ((v >> 24) & 0xFF) as u8;
}

#[inline]
fn write_i32_le(dst: &mut [u8], v: i32) {
    write_u32_le(dst, v as u32);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Detection;

    #[test]
    fn css_percent_full_frame() {
        let d = Detection {
            x_min_u16: 0,
            y_min_u16: 0,
            x_max_u16: 65535,
            y_max_u16: 65535,
            ..Detection::empty()
        };
        let (l, t, w, h) = box_css_percent(&d);
        assert!((l - 0.0).abs() < 0.01);
        assert!((t - 0.0).abs() < 0.01);
        assert!((w - 100.0).abs() < 0.1);
        assert!((h - 100.0).abs() < 0.1);
    }

    #[test]
    fn draw_box_changes_pixels() {
        let w = 8u32;
        let h = 8u32;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        let mut d = Detection::empty();
        d.class_hash = 1;
        d.score_u16 = 1000;
        d.x_min_u16 = 0;
        d.y_min_u16 = 0;
        d.x_max_u16 = 32768;
        d.y_max_u16 = 32768;
        let n = draw_boxes_rgba8(w, h, &mut rgba, &[d], 1, [255, 0, 0, 255], 1).unwrap();
        assert_eq!(n, 1);
        assert!(rgba.iter().any(|&b| b == 255));
    }

    #[test]
    fn bmp_roundtrip_size() {
        let w = 4u32;
        let h = 4u32;
        let rgba = vec![128u8; (w * h * 4) as usize];
        let mut out = vec![0u8; 54 + (w * h * 4) as usize];
        let n = encode_bmp_rgba8(w, h, &rgba, &mut out).unwrap();
        assert_eq!(n, 54 + 64);
        assert_eq!(&out[0..2], b"BM");
    }
}
