//! SR1 — tiled classical super-resolution (plan → extract → SR tile → blend).

use super::super_resolve::{
    super_resolve, ClassicalKernel, EnhancementMode, SrBackend, SrReport, SrRequest,
};
use super::tile_blend::{blend_tile_into_accum, finalize_blend};
use super::tile_extract::extract_tile_rgb8;
use super::tile_plan::{plan_tiles, TilePolicy};
use crate::cv::error::CvError;

/// Super-resolve using overlapping tiles and feather blend.
///
/// Cold-path: allocates per-tile and accum scratch (Tier-2). Full-frame
/// classical without tiling remains in [`super_resolve`].
pub fn super_resolve_tiled(
    req: &SrRequest<'_>,
    policy: TilePolicy,
    out: &mut [u8],
) -> Result<SrReport, CvError> {
    if req.scale < 2 || req.scale > 4 {
        return Err(CvError::InvalidParameter);
    }
    if req.width == 0 || req.height == 0 {
        return Err(CvError::EmptyInput);
    }

    let out_w = req
        .width
        .checked_mul(req.scale as u32)
        .ok_or(CvError::InvalidParameter)?;
    let out_h = req
        .height
        .checked_mul(req.scale as u32)
        .ok_or(CvError::InvalidParameter)?;
    let need_out = (out_w as usize)
        .checked_mul(out_h as usize)
        .and_then(|n| n.checked_mul(3))
        .ok_or(CvError::InvalidParameter)?;
    if out.len() < need_out {
        return Err(CvError::BufferTooSmall);
    }

    // Tiny frames: single-tile path without blend overhead.
    if req.width <= policy.tile_w && req.height <= policy.tile_h {
        let mut report = super_resolve(req, out)?;
        report.tile_count = 1;
        return Ok(report);
    }

    let tiles = plan_tiles(req.width, req.height, policy)?;
    let n_pix = (out_w as usize) * (out_h as usize);
    let mut accum = vec![0f32; n_pix * 3];
    let mut weight = vec![0f32; n_pix];

    let max_tw = policy.tile_w.min(req.width);
    let max_th = policy.tile_h.min(req.height);
    let tile_in_bytes = (max_tw as usize) * (max_th as usize) * 3;
    let tile_out_bytes =
        (max_tw as usize) * (req.scale as usize) * (max_th as usize) * (req.scale as usize) * 3;
    let mut tile_in = vec![0u8; tile_in_bytes];
    let mut tile_out = vec![0u8; tile_out_bytes];

    let mut backend_id = "classical.tiled";
    for rect in &tiles {
        let need_in = (rect.w as usize) * (rect.h as usize) * 3;
        extract_tile_rgb8(req.rgb, req.width, req.height, *rect, &mut tile_in[..need_in])?;
        let tile_req = SrRequest {
            rgb: &tile_in[..need_in],
            width: rect.w,
            height: rect.h,
            scale: req.scale,
            backend: req.backend,
            mode: req.mode,
        };
        let need_tile_out =
            (rect.w as usize) * (req.scale as usize) * (rect.h as usize) * (req.scale as usize) * 3;
        let report = super_resolve(&tile_req, &mut tile_out[..need_tile_out])?;
        backend_id = report.backend_id;
        blend_tile_into_accum(
            &tile_out[..need_tile_out],
            *rect,
            req.scale,
            policy.overlap,
            out_w,
            out_h,
            &mut accum,
            &mut weight,
        )?;
    }

    finalize_blend(&accum, &weight, out_w, out_h, out)?;

    let _ = (req.mode, ClassicalKernel::Nearest, SrBackend::Classical);
    Ok(SrReport {
        backend_id,
        device: "cpu",
        scale: req.scale,
        out_width: out_w,
        out_height: out_h,
        generative: false,
        tile_count: tiles.len() as u32,
    })
}

/// Convenience: tiled with default policy.
pub fn super_resolve_tiled_default(
    req: &SrRequest<'_>,
    out: &mut [u8],
) -> Result<SrReport, CvError> {
    super_resolve_tiled(req, TilePolicy::default(), out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sr::super_resolve::{ClassicalKernel, EnhancementMode, SrBackend, SrRequest};

    #[test]
    fn tiled_matches_full_on_flat() {
        // 40×40 solid — tile path should match full-frame nearest.
        let w = 40u32;
        let h = 40u32;
        let rgb = vec![90u8; (w * h * 3) as usize];
        let scale = 2u8;
        let out_n = (w * scale as u32 * h * scale as u32 * 3) as usize;
        let mut full = vec![0u8; out_n];
        let mut tiled = vec![0u8; out_n];
        let req = SrRequest {
            rgb: &rgb,
            width: w,
            height: h,
            scale,
            backend: SrBackend::Classical(ClassicalKernel::Nearest),
            mode: EnhancementMode::Sharpen,
        };
        let r_full = super_resolve(&req, &mut full).unwrap();
        let policy = TilePolicy {
            tile_w: 16,
            tile_h: 16,
            overlap: 4,
            max_tiles: 64,
        };
        let r_tiled = super_resolve_tiled(&req, policy, &mut tiled).unwrap();
        assert!(r_tiled.tile_count > 1);
        assert_eq!(r_full.out_width, r_tiled.out_width);
        // Flat image: every pixel identical.
        assert_eq!(full, tiled);
    }

    #[test]
    fn tiny_uses_one_tile() {
        let rgb = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let req = SrRequest {
            rgb: &rgb,
            width: 2,
            height: 2,
            scale: 2,
            backend: SrBackend::Classical(ClassicalKernel::Bilinear),
            mode: EnhancementMode::Sharpen,
        };
        let mut out = [0u8; 48];
        let r = super_resolve_tiled(&req, TilePolicy::default(), &mut out).unwrap();
        assert_eq!(r.tile_count, 1);
    }
}
