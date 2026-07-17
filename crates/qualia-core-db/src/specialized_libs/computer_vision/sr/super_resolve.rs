//! Unified classical super-resolution entry (Track B0 / SR0).
//!
//! Later tracks extend `SrBackend` (CNN-Light, ESRGAN, Swin) without changing this surface.

use crate::specialized_libs::computer_vision::cv::buffer::RgbView;
use crate::specialized_libs::computer_vision::cv::error::CvError;
use crate::specialized_libs::computer_vision::cv::sr::{bicubic_u8, bilinear_u8, lanczos3_u8};

/// Classical resampling kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassicalKernel {
    Nearest,
    Bilinear,
    Bicubic,
    Lanczos3,
}

/// Super-resolution backend. Only classical for B0; more variants later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SrBackend {
    Classical(ClassicalKernel),
}

/// Enhancement honesty mode. Medical/forensic paths use `Sharpen`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnhancementMode {
    /// Classical / non-generative sharpening. Does not invent textures.
    Sharpen,
}

/// Caller request for `super_resolve`.
#[derive(Debug, Clone, Copy)]
pub struct SrRequest<'a> {
    pub rgb: &'a [u8],
    pub width: u32,
    pub height: u32,
    pub scale: u8,
    pub backend: SrBackend,
    pub mode: EnhancementMode,
}

/// Report returned after a successful enhance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SrReport {
    pub backend_id: &'static str,
    pub device: &'static str,
    pub scale: u8,
    pub out_width: u32,
    pub out_height: u32,
    /// Always false for classical kernels.
    pub generative: bool,
    /// Full-frame classical = 1; tiling reports real count later.
    pub tile_count: u32,
}

/// Super-resolve RGB8 into caller buffer (`out` ≥ `w*scale * h*scale * 3`).
pub fn super_resolve(req: &SrRequest<'_>, out: &mut [u8]) -> Result<SrReport, CvError> {
    if req.scale < 2 || req.scale > 4 {
        return Err(CvError::InvalidParameter);
    }
    if req.width == 0 || req.height == 0 {
        return Err(CvError::EmptyInput);
    }
    let need_in = (req.width as usize)
        .checked_mul(req.height as usize)
        .and_then(|n| n.checked_mul(3))
        .ok_or(CvError::InvalidParameter)?;
    if req.rgb.len() < need_in {
        return Err(CvError::DimensionMismatch);
    }

    let stride = req.width.checked_mul(3).ok_or(CvError::InvalidParameter)?;
    let view = RgbView::new(req.width, req.height, stride, req.rgb)
        .ok_or(CvError::InvalidParameter)?;

    let out_w = req
        .width
        .checked_mul(req.scale as u32)
        .ok_or(CvError::InvalidParameter)?;
    let out_h = req
        .height
        .checked_mul(req.scale as u32)
        .ok_or(CvError::InvalidParameter)?;

    let backend_id = match req.backend {
        SrBackend::Classical(ClassicalKernel::Nearest) => {
            nearest_u8(view, req.scale, out)?;
            "classical.nearest"
        }
        SrBackend::Classical(ClassicalKernel::Bilinear) => {
            bilinear_u8(view, req.scale, out)?;
            "classical.bilinear"
        }
        SrBackend::Classical(ClassicalKernel::Bicubic) => {
            bicubic_u8(view, req.scale, out)?;
            "classical.bicubic"
        }
        SrBackend::Classical(ClassicalKernel::Lanczos3) => {
            lanczos3_u8(view, req.scale, out)?;
            "classical.lanczos3"
        }
    };

    // Classical path is always Sharpen-class (non-generative).
    let _ = req.mode;

    Ok(SrReport {
        backend_id,
        device: "cpu",
        scale: req.scale,
        out_width: out_w,
        out_height: out_h,
        generative: false,
        tile_count: 1,
    })
}

/// Nearest-neighbour upsample (baseline for A/B vs bicubic/Lanczos).
fn nearest_u8(src: RgbView<'_>, scale: u8, out: &mut [u8]) -> Result<(), CvError> {
    let w = src.width;
    let h = src.height;
    let out_w = w.checked_mul(scale as u32).ok_or(CvError::InvalidParameter)?;
    let out_h = h.checked_mul(scale as u32).ok_or(CvError::InvalidParameter)?;
    let need = (out_w as usize)
        .checked_mul(out_h as usize)
        .and_then(|n| n.checked_mul(3))
        .ok_or(CvError::InvalidParameter)?;
    if out.len() < need {
        return Err(CvError::BufferTooSmall);
    }
    let s = scale as u32;
    for oy in 0..out_h {
        let sy = oy / s;
        for ox in 0..out_w {
            let sx = ox / s;
            let (r, g, b) = src.pixel(sx, sy);
            let doff = ((oy * out_w + ox) * 3) as usize;
            out[doff] = r;
            out[doff + 1] = g;
            out[doff + 2] = b;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_2x2() -> [u8; 12] {
        [
            255, 0, 0, 0, 255, 0, //
            0, 0, 255, 128, 128, 128,
        ]
    }

    #[test]
    fn nearest_2x2_to_4x4() {
        let rgb = solid_2x2();
        let req = SrRequest {
            rgb: &rgb,
            width: 2,
            height: 2,
            scale: 2,
            backend: SrBackend::Classical(ClassicalKernel::Nearest),
            mode: EnhancementMode::Sharpen,
        };
        let mut out = [0u8; 4 * 4 * 3];
        let report = super_resolve(&req, &mut out).unwrap();
        assert_eq!(report.out_width, 4);
        assert_eq!(report.out_height, 4);
        assert_eq!(report.scale, 2);
        assert_eq!(report.device, "cpu");
        assert!(!report.generative);
        assert_eq!(report.backend_id, "classical.nearest");
        // Top-left 2×2 block is solid red
        assert_eq!(&out[0..3], &[255, 0, 0]);
        assert_eq!(&out[3..6], &[255, 0, 0]);
        assert_eq!(&out[(4 * 3)..(4 * 3 + 3)], &[255, 0, 0]);
    }

    #[test]
    fn bicubic_super_resolve_path() {
        let rgb = solid_2x2();
        let req = SrRequest {
            rgb: &rgb,
            width: 2,
            height: 2,
            scale: 2,
            backend: SrBackend::Classical(ClassicalKernel::Bicubic),
            mode: EnhancementMode::Sharpen,
        };
        let mut out = [0u8; 48];
        let report = super_resolve(&req, &mut out).unwrap();
        assert_eq!(report.backend_id, "classical.bicubic");
        assert_eq!(report.tile_count, 1);
        assert!(out.iter().any(|&c| c > 0));
    }

    #[test]
    fn lanczos_super_resolve_path() {
        let rgb = solid_2x2();
        let req = SrRequest {
            rgb: &rgb,
            width: 2,
            height: 2,
            scale: 2,
            backend: SrBackend::Classical(ClassicalKernel::Lanczos3),
            mode: EnhancementMode::Sharpen,
        };
        let mut out = [0u8; 48];
        let report = super_resolve(&req, &mut out).unwrap();
        assert_eq!(report.backend_id, "classical.lanczos3");
        assert!(!report.generative);
    }

    #[test]
    fn bilinear_super_resolve_path() {
        let rgb = solid_2x2();
        let req = SrRequest {
            rgb: &rgb,
            width: 2,
            height: 2,
            scale: 3,
            backend: SrBackend::Classical(ClassicalKernel::Bilinear),
            mode: EnhancementMode::Sharpen,
        };
        let mut out = [0u8; 6 * 6 * 3];
        let report = super_resolve(&req, &mut out).unwrap();
        assert_eq!(report.out_width, 6);
        assert_eq!(report.out_height, 6);
        assert_eq!(report.backend_id, "classical.bilinear");
    }

    #[test]
    fn bad_scale_fails() {
        let rgb = [1u8, 2, 3];
        let req = SrRequest {
            rgb: &rgb,
            width: 1,
            height: 1,
            scale: 5,
            backend: SrBackend::Classical(ClassicalKernel::Nearest),
            mode: EnhancementMode::Sharpen,
        };
        let mut out = [0u8; 75];
        assert_eq!(super_resolve(&req, &mut out), Err(CvError::InvalidParameter));
    }

    #[test]
    fn short_input_fails() {
        let rgb = [1u8, 2];
        let req = SrRequest {
            rgb: &rgb,
            width: 1,
            height: 1,
            scale: 2,
            backend: SrBackend::Classical(ClassicalKernel::Nearest),
            mode: EnhancementMode::Sharpen,
        };
        let mut out = [0u8; 12];
        assert_eq!(super_resolve(&req, &mut out), Err(CvError::DimensionMismatch));
    }
}
