//! B2 — nearest NCHW resize via Forge on the **auxiliary** GPU circuit when available.
//!
//! Uses existing `OpNode::Resize2d` / WGSL `resize2d_main` (certified against
//! the same numerical contract as `ops::resize_nearest_nchw_f32`). Fail closed
//! to caller (CPU path) on any adapter / forge error.
//!
//! Device routing: vision SR runs on the auxiliary circuit (the iGPU when present) to keep the
//! discrete GPU free for the LLM. The device comes from
//! [`crate::gpu_context::device_registry::try_auxiliary_gpu`], whose fallback chain is
//! auxiliary → primary → CPU: on a single-GPU box it transparently uses the primary device, and on
//! a GPU-less box it returns `None`, so this function fails closed and the caller degrades to the
//! CPU oracle. It never panics.

use super::dispatch::VisionComputeReport;
use crate::specialized_libs::computer_vision::{types::VisionError, VisionComputeDevice};

/// Attempt GPU resize on the process-wide shared device.
///
/// Returns `Ok(report)` with `device = SharedGpu` on success. On any failure
/// returns `Err` so the caller falls back to the CPU oracle.
#[cfg(all(feature = "gpu-runtime", not(target_arch = "wasm32")))]
pub fn try_resize_nearest_shared_gpu(
    input: &[f32],
    c: usize,
    h_in: usize,
    w_in: usize,
    h_out: usize,
    w_out: usize,
    out: &mut [f32],
) -> Result<VisionComputeReport, VisionError> {
    use crate::wgsl_forge::execute::WgpuComputeContext;
    use crate::wgsl_forge::graph_ops::executor::ForgeGraphExecutor;
    use crate::wgsl_forge::ir::graph::{ComputeGraph, DType, OpNode, Shape, TensorRef};
    use crate::wgsl_forge::schedule::Schedule;

    /// Per-slab capacity for the vision resize executor (matches `ForgeGraphExecutor`'s default).
    const EXEC_CAPACITY: usize = 64 << 20;

    let need_in = c.saturating_mul(h_in).saturating_mul(w_in);
    let need_out = c.saturating_mul(h_out).saturating_mul(w_out);
    if input.len() < need_in || out.len() < need_out || c == 0 || h_in == 0 || w_in == 0 {
        return Err(VisionError::MalformedImage);
    }
    if h_out == 0 || w_out == 0 {
        return Err(VisionError::MalformedImage);
    }

    let mut g = ComputeGraph::new();
    let s = Schedule::default();
    let ext = TensorRef::input(
        0,
        Shape::new(&[c as u32, h_in as u32, w_in as u32]),
        DType::F32,
    );
    let tout = g
        .push(
            OpNode::Resize2d {
                c: c as u32,
                h_in: h_in as u32,
                w_in: w_in as u32,
                h_out: h_out as u32,
                w_out: w_out as u32,
            },
            &[ext],
            Shape::new(&[c as u32, h_out as u32, w_out as u32]),
            DType::F32,
            s,
        )
        .map_err(|_| VisionError::BackendUnavailable)?;
    g.mark_output(tout);

    // Route through the device-per-circuit registry: auxiliary (iGPU) → primary → None. This keeps
    // the discrete GPU free for the LLM, and — unlike `on_shared_gpu()` (which calls `shared_gpu()`
    // and panics with no adapter) — fails closed to the caller's CPU fallback on a GPU-less machine.
    let gpu = match crate::gpu_context::device_registry::try_auxiliary_gpu() {
        Some(g) => g,
        None => return Err(VisionError::BackendUnavailable),
    };
    let ctx = WgpuComputeContext::from_device(
        gpu.device.clone(),
        gpu.queue.clone(),
        &gpu.adapter_caps,
        EXEC_CAPACITY,
    )
    .map_err(|_| VisionError::BackendUnavailable)?;
    let mut exec = ForgeGraphExecutor::with_context(ctx);
    let result = exec
        .run(&g, &[input[..need_in].to_vec()])
        .map_err(|_| VisionError::BackendUnavailable)?;
    if result.len() < need_out {
        return Err(VisionError::OutputBufferTooSmall);
    }
    out[..need_out].copy_from_slice(&result[..need_out]);
    Ok(VisionComputeReport {
        device: VisionComputeDevice::SharedGpu,
        degraded: false,
    })
}

#[cfg(not(all(feature = "gpu-runtime", not(target_arch = "wasm32"))))]
pub fn try_resize_nearest_shared_gpu(
    _input: &[f32],
    _c: usize,
    _h_in: usize,
    _w_in: usize,
    _h_out: usize,
    _w_out: usize,
    _out: &mut [f32],
) -> Result<VisionComputeReport, VisionError> {
    Err(VisionError::BackendUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computer_vision::ops::resize_nearest_nchw_f32;

    #[test]
    fn forge_path_or_unavailable_matches_cpu_when_ok() {
        let input = [1.0f32, 2.0, 3.0, 4.0];
        let mut cpu = [0.0f32; 16];
        resize_nearest_nchw_f32(&input, 1, 2, 2, 4, 4, &mut cpu).unwrap();
        let mut gpu = [0.0f32; 16];
        match try_resize_nearest_shared_gpu(&input, 1, 2, 2, 4, 4, &mut gpu) {
            Ok(r) => {
                assert_eq!(r.device, VisionComputeDevice::SharedGpu);
                for i in 0..16 {
                    assert!(
                        (cpu[i] - gpu[i]).abs() < 1e-4,
                        "cpu {} vs gpu {} at {}",
                        cpu[i],
                        gpu[i],
                        i
                    );
                }
            }
            Err(VisionError::BackendUnavailable) => {
                // No adapter / CI headless — honest.
            }
            Err(e) => panic!("unexpected {e:?}"),
        }
    }
}
