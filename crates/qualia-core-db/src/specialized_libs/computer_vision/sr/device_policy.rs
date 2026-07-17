//! B2 — SR device policy (classical path honesty before WGSL land).
//!
//! Prefer GPU when Cool + budget allows; today always runs CPU classical and
//! reports `Unavailable` / `degraded` until Forge SR kernels certify.

use super::super::gpu::policy::{thermal_allows_gpu_tiles, ThermalHint, VisionVramBudget};
use super::super::gpu::{VisionComputeDevice, VisionComputeReport};
use super::super_resolve::{super_resolve, ClassicalKernel, SrBackend, SrReport, SrRequest};
use super::super_resolve_tiled::super_resolve_tiled;
use super::tile_plan::TilePolicy;
use crate::specialized_libs::computer_vision::cv::error::CvError;

/// Super-resolve with explicit thermal + VRAM policy (B2 surface).
///
/// Classical CPU always executes; `prefer_gpu` is recorded honestly in
/// `compute` and does not invent a second adapter.
pub fn super_resolve_with_policy(
    req: &SrRequest<'_>,
    prefer_gpu: bool,
    thermal: ThermalHint,
    budget: VisionVramBudget,
    out: &mut [u8],
) -> Result<(SrReport, VisionComputeReport), CvError> {
    let est = VisionVramBudget::estimate_resize_scratch(3, req.height, req.width, req.scale as u32);
    let gpu_ok = prefer_gpu && thermal_allows_gpu_tiles(thermal) && budget.allows(est);

    // Classical kernels (bicubic/lanczos) stay on CPU. Nearest can ride Forge
    // Resize2d when Cool + budget allows (B2 shared_gpu path).
    if gpu_ok {
        if let SrBackend::Classical(ClassicalKernel::Nearest) = req.backend {
            if let Ok(compute) = try_nearest_via_forge(req, out) {
                let report = SrReport {
                    backend_id: "classical.nearest.forge",
                    device: "shared_gpu",
                    scale: req.scale,
                    out_width: req.width.saturating_mul(req.scale as u32),
                    out_height: req.height.saturating_mul(req.scale as u32),
                    generative: false,
                    tile_count: 1,
                };
                return Ok((report, compute));
            }
        }
    }

    let report = super_resolve(req, out)?;
    let compute = VisionComputeReport {
        device: if prefer_gpu {
            VisionComputeDevice::Unavailable
        } else {
            VisionComputeDevice::Cpu
        },
        degraded: prefer_gpu,
    };
    Ok((report, compute))
}

/// Pack RGB8 → NCHW f32, Forge nearest upscale, unpack to RGB8.
fn try_nearest_via_forge(
    req: &SrRequest<'_>,
    out: &mut [u8],
) -> Result<VisionComputeReport, CvError> {
    use crate::specialized_libs::computer_vision::gpu::forge_resize::try_resize_nearest_shared_gpu;

    let w = req.width as usize;
    let h = req.height as usize;
    let s = req.scale as usize;
    let need_in = w * h * 3;
    if req.rgb.len() < need_in {
        return Err(CvError::DimensionMismatch);
    }
    let ow = w * s;
    let oh = h * s;
    let need_out = ow * oh * 3;
    if out.len() < need_out {
        return Err(CvError::BufferTooSmall);
    }
    // NCHW: c=3
    let mut nchw_in = vec![0.0f32; 3 * w * h];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 3;
            nchw_in[0 * w * h + y * w + x] = req.rgb[i] as f32 / 255.0;
            nchw_in[1 * w * h + y * w + x] = req.rgb[i + 1] as f32 / 255.0;
            nchw_in[2 * w * h + y * w + x] = req.rgb[i + 2] as f32 / 255.0;
        }
    }
    let mut nchw_out = vec![0.0f32; 3 * ow * oh];
    let report = try_resize_nearest_shared_gpu(&nchw_in, 3, h, w, oh, ow, &mut nchw_out)
        .map_err(|_| CvError::InvalidParameter)?;
    for y in 0..oh {
        for x in 0..ow {
            let o = (y * ow + x) * 3;
            out[o] = (nchw_out[0 * ow * oh + y * ow + x] * 255.0).round().clamp(0.0, 255.0) as u8;
            out[o + 1] =
                (nchw_out[1 * ow * oh + y * ow + x] * 255.0).round().clamp(0.0, 255.0) as u8;
            out[o + 2] =
                (nchw_out[2 * ow * oh + y * ow + x] * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    Ok(report)
}

/// Tiled classical with the same device honesty.
pub fn super_resolve_tiled_with_policy(
    req: &SrRequest<'_>,
    policy: TilePolicy,
    prefer_gpu: bool,
    thermal: ThermalHint,
    budget: VisionVramBudget,
    out: &mut [u8],
) -> Result<(SrReport, VisionComputeReport), CvError> {
    let est = VisionVramBudget::estimate_resize_scratch(3, req.height, req.width, req.scale as u32);
    let _gpu_ok = prefer_gpu && thermal_allows_gpu_tiles(thermal) && budget.allows(est);
    let report = super_resolve_tiled(req, policy, out)?;
    let compute = VisionComputeReport {
        device: if prefer_gpu {
            VisionComputeDevice::Unavailable
        } else {
            VisionComputeDevice::Cpu
        },
        degraded: prefer_gpu,
    };
    Ok((report, compute))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computer_vision::sr::super_resolve::{
        ClassicalKernel, EnhancementMode, SrBackend, SrRequest,
    };

    #[test]
    fn prefer_gpu_marks_degraded_classical() {
        let rgb = [10u8, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
        let req = SrRequest {
            rgb: &rgb,
            width: 2,
            height: 2,
            scale: 2,
            backend: SrBackend::Classical(ClassicalKernel::Nearest),
            mode: EnhancementMode::Sharpen,
        };
        let mut out = [0u8; 48];
        let (sr, c) = super_resolve_with_policy(
            &req,
            true,
            ThermalHint::Cool,
            VisionVramBudget::default(),
            &mut out,
        )
        .unwrap();
        assert!(!sr.generative);
        // Nearest may hit SharedGpu (Forge) when adapter present; otherwise degraded CPU.
        match c.device {
            VisionComputeDevice::SharedGpu => {
                assert!(!c.degraded);
                assert_eq!(sr.device, "shared_gpu");
            }
            VisionComputeDevice::Unavailable | VisionComputeDevice::Cpu => {
                assert!(c.degraded);
            }
        }
    }

    #[test]
    fn critical_thermal_still_cpu() {
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
        let (_sr, c) = super_resolve_with_policy(
            &req,
            true,
            ThermalHint::Critical,
            VisionVramBudget::default(),
            &mut out,
        )
        .unwrap();
        assert!(c.degraded);
    }
}
