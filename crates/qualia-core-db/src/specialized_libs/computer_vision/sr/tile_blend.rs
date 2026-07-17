//! SR1 — feather-blend an upscaled tile into a full-frame output.

use super::tile_plan::TileRect;
use crate::specialized_libs::computer_vision::cv::error::CvError;

/// Blend one **already scaled** tile into float accumulators.
///
/// - `tile_rgb`: packed RGB of size `(rect.w * scale) × (rect.h * scale) × 3`
/// - `rect`: input-space tile that produced this tile
/// - `overlap`: input-space overlap used in the plan (feather width ≈ `overlap * scale`)
/// - `accum`: RGB interleaved f32, length ≥ `out_w * out_h * 3` (caller zeros once)
/// - `weight`: per-pixel f32, length ≥ `out_w * out_h`
///
/// After all tiles, call [`finalize_blend`].
pub fn blend_tile_into_accum(
    tile_rgb: &[u8],
    rect: TileRect,
    scale: u8,
    overlap: u32,
    out_w: u32,
    out_h: u32,
    accum: &mut [f32],
    weight: &mut [f32],
) -> Result<(), CvError> {
    if scale < 1 {
        return Err(CvError::InvalidParameter);
    }
    let s = scale as u32;
    let tw = rect.w.checked_mul(s).ok_or(CvError::InvalidParameter)?;
    let th = rect.h.checked_mul(s).ok_or(CvError::InvalidParameter)?;
    let need_tile = (tw as usize)
        .checked_mul(th as usize)
        .and_then(|n| n.checked_mul(3))
        .ok_or(CvError::InvalidParameter)?;
    if tile_rgb.len() < need_tile {
        return Err(CvError::DimensionMismatch);
    }
    let need_pix = (out_w as usize)
        .checked_mul(out_h as usize)
        .ok_or(CvError::InvalidParameter)?;
    if accum.len() < need_pix * 3 || weight.len() < need_pix {
        return Err(CvError::BufferTooSmall);
    }

    let ox0 = rect.x.checked_mul(s).ok_or(CvError::InvalidParameter)?;
    let oy0 = rect.y.checked_mul(s).ok_or(CvError::InvalidParameter)?;
    let feather = (overlap.saturating_mul(s)).max(1);

    for ty in 0..th {
        for tx in 0..tw {
            let gx = ox0 + tx;
            let gy = oy0 + ty;
            if gx >= out_w || gy >= out_h {
                continue;
            }
            let wgt = edge_feather(tx, ty, tw, th, feather);
            let toff = ((ty * tw + tx) * 3) as usize;
            let pix = (gy as usize) * (out_w as usize) + (gx as usize);
            let base = pix * 3;
            accum[base] += tile_rgb[toff] as f32 * wgt;
            accum[base + 1] += tile_rgb[toff + 1] as f32 * wgt;
            accum[base + 2] += tile_rgb[toff + 2] as f32 * wgt;
            weight[pix] += wgt;
        }
    }
    Ok(())
}

/// Linear distance-to-edge feather in (0, 1].
fn edge_feather(tx: u32, ty: u32, tw: u32, th: u32, feather: u32) -> f32 {
    let f = feather as f32;
    let dx = (tx.min(tw.saturating_sub(1).saturating_sub(tx)) as f32 / f).min(1.0);
    let dy = (ty.min(th.saturating_sub(1).saturating_sub(ty)) as f32 / f).min(1.0);
    dx.min(dy).max(1e-3)
}

/// Divide accum by weight into `out` RGB8.
pub fn finalize_blend(
    accum: &[f32],
    weight: &[f32],
    out_w: u32,
    out_h: u32,
    out: &mut [u8],
) -> Result<(), CvError> {
    let n = (out_w as usize)
        .checked_mul(out_h as usize)
        .ok_or(CvError::InvalidParameter)?;
    if weight.len() < n || accum.len() < n * 3 || out.len() < n * 3 {
        return Err(CvError::BufferTooSmall);
    }
    for i in 0..n {
        let w = weight[i];
        let inv = if w > 1e-6 { 1.0 / w } else { 0.0 };
        let base = i * 3;
        out[base] = (accum[base] * inv).round().clamp(0.0, 255.0) as u8;
        out[base + 1] = (accum[base + 1] * inv).round().clamp(0.0, 255.0) as u8;
        out[base + 2] = (accum[base + 2] * inv).round().clamp(0.0, 255.0) as u8;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computer_vision::sr::tile_plan::TileRect;

    #[test]
    fn single_tile_identity() {
        let tile = [255u8, 0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0];
        let rect = TileRect {
            x: 0,
            y: 0,
            w: 2,
            h: 2,
        };
        let mut accum = [0f32; 2 * 2 * 3];
        let mut weight = [0f32; 4];
        blend_tile_into_accum(&tile, rect, 1, 0, 2, 2, &mut accum, &mut weight).unwrap();
        let mut out = [0u8; 12];
        finalize_blend(&accum, &weight, 2, 2, &mut out).unwrap();
        assert_eq!(&out[0..3], &[255, 0, 0]);
    }

    #[test]
    fn two_tiles_seamless_flat() {
        let full = [100u8; 4 * 1 * 3];
        let mut accum = [0f32; 4 * 1 * 3];
        let mut weight = [0f32; 4];
        let t0 = TileRect {
            x: 0,
            y: 0,
            w: 3,
            h: 1,
        };
        let t1 = TileRect {
            x: 1,
            y: 0,
            w: 3,
            h: 1,
        };
        let tile0 = &full[0..9];
        let tile1 = &full[3..12];
        blend_tile_into_accum(tile0, t0, 1, 2, 4, 1, &mut accum, &mut weight).unwrap();
        blend_tile_into_accum(tile1, t1, 1, 2, 4, 1, &mut accum, &mut weight).unwrap();
        let mut out = [0u8; 12];
        finalize_blend(&accum, &weight, 4, 1, &mut out).unwrap();
        for i in 0..4 {
            assert_eq!(&out[i * 3..i * 3 + 3], &[100, 100, 100]);
        }
    }
}
