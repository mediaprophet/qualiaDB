//! Dispatch vision ops to CPU oracle (always) or GPU when feature lands (A1/A2).
//!
//! A2: same report surface for resize / conv / pool. Forge/`shared_gpu` path
//! is feature-gated; without an adapter the CPU oracle runs with `degraded`.

use crate::ops::{
    avg_pool2d_nchw_f32, conv2d_nchw_f32, max_pool2d_nchw_f32, resize_nearest_nchw_f32,
};
use crate::types::VisionError;

/// Device that executed the op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisionComputeDevice {
    Cpu,
    /// Reserved: `gpu_context::shared_gpu` via Forge (not wired without feature+adapter).
    SharedGpu,
    /// Feature off or no adapter — caller used CPU fallback.
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisionComputeReport {
    pub device: VisionComputeDevice,
    /// True if GPU was requested but CPU ran instead.
    pub degraded: bool,
}

/// Internal: map prefer_gpu + feature to report after CPU oracle ran.
fn report_after_cpu(prefer_gpu: bool) -> VisionComputeReport {
    #[cfg(feature = "gpu")]
    {
        VisionComputeReport {
            device: VisionComputeDevice::Cpu,
            // Prefer-gpu with feature on but no adapter yet → degraded CPU.
            degraded: prefer_gpu,
        }
    }
    #[cfg(not(feature = "gpu"))]
    {
        VisionComputeReport {
            device: if prefer_gpu {
                VisionComputeDevice::Unavailable
            } else {
                VisionComputeDevice::Cpu
            },
            degraded: prefer_gpu,
        }
    }
}

/// Prefer GPU when `prefer_gpu` and feature allows; currently always CPU with honest report.
pub fn resize_nearest_nchw_dispatch(
    input: &[f32],
    c: usize,
    h_in: usize,
    w_in: usize,
    h_out: usize,
    w_out: usize,
    out: &mut [f32],
    prefer_gpu: bool,
) -> Result<VisionComputeReport, VisionError> {
    // Feature `gpu` will attempt Forge here; default product path is CPU oracle.
    resize_nearest_nchw_f32(input, c, h_in, w_in, h_out, w_out, out)?;
    Ok(report_after_cpu(prefer_gpu))
}

/// Conv2d NCHW dispatch (CPU oracle; GPU when A2 Forge lands).
#[allow(clippy::too_many_arguments)]
pub fn conv2d_nchw_dispatch(
    input: &[f32],
    c_in: usize,
    h: usize,
    w: usize,
    weight: &[f32],
    c_out: usize,
    kh: usize,
    kw: usize,
    bias: &[f32],
    stride_h: usize,
    stride_w: usize,
    pad_h: usize,
    pad_w: usize,
    out: &mut [f32],
    prefer_gpu: bool,
) -> Result<(VisionComputeReport, usize, usize), VisionError> {
    let (ho, wo) = conv2d_nchw_f32(
        input, c_in, h, w, weight, c_out, kh, kw, bias, stride_h, stride_w, pad_h, pad_w, out,
    )?;
    Ok((report_after_cpu(prefer_gpu), ho, wo))
}

/// Max-pool dispatch wrapping the CPU oracle.
#[allow(clippy::too_many_arguments)]
pub fn max_pool2d_dispatch(
    input: &[f32],
    c: usize,
    h: usize,
    w: usize,
    kh: usize,
    kw: usize,
    stride_h: usize,
    stride_w: usize,
    out: &mut [f32],
    prefer_gpu: bool,
) -> Result<(VisionComputeReport, usize, usize), VisionError> {
    let (ho, wo) = max_pool2d_nchw_f32(input, c, h, w, kh, kw, stride_h, stride_w, out)?;
    Ok((report_after_cpu(prefer_gpu), ho, wo))
}

/// Avg-pool dispatch wrapping the CPU oracle.
#[allow(clippy::too_many_arguments)]
pub fn avg_pool2d_dispatch(
    input: &[f32],
    c: usize,
    h: usize,
    w: usize,
    kh: usize,
    kw: usize,
    stride_h: usize,
    stride_w: usize,
    out: &mut [f32],
    prefer_gpu: bool,
) -> Result<(VisionComputeReport, usize, usize), VisionError> {
    let (ho, wo) = avg_pool2d_nchw_f32(input, c, h, w, kh, kw, stride_h, stride_w, out)?;
    Ok((report_after_cpu(prefer_gpu), ho, wo))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_path_matches_oracle() {
        let input = [1.0f32, 2.0, 3.0, 4.0];
        let mut out = [0.0f32; 16];
        let r = resize_nearest_nchw_dispatch(&input, 1, 2, 2, 4, 4, &mut out, false).unwrap();
        assert_eq!(r.device, VisionComputeDevice::Cpu);
        assert_eq!(out[0], 1.0);
        assert_eq!(out[15], 4.0);
    }

    #[test]
    fn prefer_gpu_degrades_without_adapter() {
        let input = [1.0f32; 4];
        let mut out = [0.0f32; 16];
        let r = resize_nearest_nchw_dispatch(&input, 1, 2, 2, 4, 4, &mut out, true).unwrap();
        assert!(r.degraded || r.device == VisionComputeDevice::Unavailable);
    }

    #[test]
    fn pool_dispatch_runs() {
        let input = [1.0f32, 2.0, 3.0, 4.0];
        let mut out = [0.0f32; 1];
        let (r, ho, wo) =
            max_pool2d_dispatch(&input, 1, 2, 2, 2, 2, 2, 2, &mut out, false).unwrap();
        assert_eq!(r.device, VisionComputeDevice::Cpu);
        assert_eq!((ho, wo), (1, 1));
        assert_eq!(out[0], 4.0);
    }
}
