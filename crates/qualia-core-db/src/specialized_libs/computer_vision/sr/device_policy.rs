//! B2 — SR device policy (classical path honesty before WGSL land).
//!
//! Prefer GPU when Cool + budget allows; today always runs CPU classical and
//! reports `Unavailable` / `degraded` until Forge SR kernels certify.

use super::super::gpu::policy::{thermal_allows_gpu_tiles, ThermalHint, VisionVramBudget};
use super::super::gpu::{VisionComputeDevice, VisionComputeReport};
use super::super_resolve::{super_resolve, SrReport, SrRequest};
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
    let report = super_resolve(req, out)?;
    let compute = VisionComputeReport {
        device: if gpu_ok {
            // Reserved: would be SharedGpu after Forge SR cert.
            VisionComputeDevice::Unavailable
        } else if prefer_gpu {
            VisionComputeDevice::Unavailable
        } else {
            VisionComputeDevice::Cpu
        },
        degraded: prefer_gpu,
    };
    let _ = gpu_ok;
    Ok((report, compute))
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
        assert!(c.degraded);
        assert_eq!(c.device, VisionComputeDevice::Unavailable);
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
