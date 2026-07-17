//! SR1 — extract an input tile (RGB8 packed) into a caller buffer.

use super::tile_plan::TileRect;
use crate::cv::error::CvError;

/// Copy `rect` from full-frame RGB8 (`src_w × src_h × 3`) into `out` (at least `rect.w*rect.h*3`).
pub fn extract_tile_rgb8(
    rgb: &[u8],
    src_w: u32,
    src_h: u32,
    rect: TileRect,
    out: &mut [u8],
) -> Result<(), CvError> {
    if src_w == 0 || src_h == 0 {
        return Err(CvError::EmptyInput);
    }
    if rect.w == 0 || rect.h == 0 {
        return Err(CvError::InvalidParameter);
    }
    if rect.x.saturating_add(rect.w) > src_w || rect.y.saturating_add(rect.h) > src_h {
        return Err(CvError::InvalidParameter);
    }
    let need_src = (src_w as usize)
        .checked_mul(src_h as usize)
        .and_then(|n| n.checked_mul(3))
        .ok_or(CvError::InvalidParameter)?;
    if rgb.len() < need_src {
        return Err(CvError::DimensionMismatch);
    }
    let need_out = (rect.w as usize)
        .checked_mul(rect.h as usize)
        .and_then(|n| n.checked_mul(3))
        .ok_or(CvError::InvalidParameter)?;
    if out.len() < need_out {
        return Err(CvError::BufferTooSmall);
    }

    let src_stride = (src_w as usize) * 3;
    let dst_stride = (rect.w as usize) * 3;
    for row in 0..rect.h as usize {
        let sy = rect.y as usize + row;
        let src_off = sy * src_stride + (rect.x as usize) * 3;
        let dst_off = row * dst_stride;
        out[dst_off..dst_off + dst_stride]
            .copy_from_slice(&rgb[src_off..src_off + dst_stride]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sr::tile_plan::TileRect;

    #[test]
    fn extracts_corner() {
        // 2×2: R G / B W
        let rgb = [
            255, 0, 0, 0, 255, 0, //
            0, 0, 255, 255, 255, 255,
        ];
        let mut out = [0u8; 3];
        extract_tile_rgb8(
            &rgb,
            2,
            2,
            TileRect {
                x: 1,
                y: 0,
                w: 1,
                h: 1,
            },
            &mut out,
        )
        .unwrap();
        assert_eq!(out, [0, 255, 0]);
    }
}
