//! Bicubic (Keys a=-0.5) NCHW f32 resize via Forge WGSL on the **auxiliary** GPU circuit.
//!
//! Matches the CPU Keys cubic used by `cv::sr::bicubic_u8` (per-channel NCHW).
//! Direct WGSL dispatch (not yet a graph OpNode) so product SR can use GPU
//! without waiting on IR lowerer expansion.
//!
//! Device routing: vision SR runs on the auxiliary circuit (the iGPU when present) to keep the
//! discrete GPU free for the LLM. The device comes from
//! [`crate::gpu_context::device_registry::try_auxiliary_gpu`], whose fallback chain is
//! auxiliary → primary → CPU: on a single-GPU box it transparently uses the primary device, and on
//! a GPU-less box it returns `None`, so this function reports `BackendUnavailable` and the caller
//! ([`super::dispatch`]) degrades to the CPU oracle. It never panics.

use super::dispatch::{VisionComputeDevice, VisionComputeReport};
use crate::specialized_libs::computer_vision::types::VisionError;

pub const BICUBIC_ENTRY: &str = "bicubic2d_main";

/// WGSL bicubic upsample. params u32: `[c, h_in, w_in, h_out, w_out, 0, 0, 0]`.
pub fn bicubic2d_wgsl(wg: u32) -> String {
    format!(
        r#"@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<storage, read> params: array<u32>;

fn cubic_weight(t: f32) -> f32 {{
    let a = -0.5;
    let x = abs(t);
    if (x <= 1.0) {{
        return ((a + 2.0) * x - (a + 3.0)) * x * x + 1.0;
    }} else if (x < 2.0) {{
        return ((a * x - 5.0 * a) * x + 8.0 * a) * x - 4.0 * a;
    }}
    return 0.0;
}}

fn clamp_i(v: i32, lo: i32, hi: i32) -> i32 {{
    return max(lo, min(v, hi));
}}

@compute @workgroup_size({wg})
fn {entry}(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let idx = gid.x;
    let c = params[0];
    let hi = params[1];
    let wi = params[2];
    let ho = params[3];
    let wo = params[4];
    let n_out = c * ho * wo;
    if (idx >= n_out) {{ return; }}
    let ch = idx / (ho * wo);
    let rem = idx % (ho * wo);
    let oy = rem / wo;
    let ox = rem % wo;
    let sy = (f32(oy) + 0.5) * f32(hi) / f32(ho) - 0.5;
    let sx = (f32(ox) + 0.5) * f32(wi) / f32(wo) - 0.5;
    let y_base = i32(floor(sy));
    let x_base = i32(floor(sx));
    let fy = sy - f32(y_base);
    let fx = sx - f32(x_base);
    var acc = 0.0;
    var wsum = 0.0;
    for (var j: i32 = -1; j <= 2; j++) {{
        let wy = cubic_weight(fy - f32(j));
        let yy = u32(clamp_i(y_base + j, 0, i32(hi) - 1));
        for (var i: i32 = -1; i <= 2; i++) {{
            let wx = cubic_weight(fx - f32(i));
            let w = wx * wy;
            let xx = u32(clamp_i(x_base + i, 0, i32(wi) - 1));
            acc = acc + input[ch * hi * wi + yy * wi + xx] * w;
            wsum = wsum + w;
        }}
    }}
    if (abs(wsum) > 1e-6) {{
        output[idx] = acc / wsum;
    }} else {{
        output[idx] = 0.0;
    }}
}}
"#,
        entry = BICUBIC_ENTRY,
    )
}

/// CPU bicubic NCHW oracle (same Keys weights as WGSL).
pub fn bicubic2d_cpu(
    input: &[f32],
    c: usize,
    h_in: usize,
    w_in: usize,
    h_out: usize,
    w_out: usize,
) -> Result<Vec<f32>, VisionError> {
    let need_in = c * h_in * w_in;
    if input.len() < need_in || c == 0 || h_in == 0 || w_in == 0 || h_out == 0 || w_out == 0 {
        return Err(VisionError::MalformedImage);
    }
    let mut out = vec![0.0f32; c * h_out * w_out];
    for ch in 0..c {
        for oy in 0..h_out {
            let sy = (oy as f32 + 0.5) * h_in as f32 / h_out as f32 - 0.5;
            let y_base = sy.floor() as i32;
            let fy = sy - y_base as f32;
            for ox in 0..w_out {
                let sx = (ox as f32 + 0.5) * w_in as f32 / w_out as f32 - 0.5;
                let x_base = sx.floor() as i32;
                let fx = sx - x_base as f32;
                let mut acc = 0.0f32;
                let mut wsum = 0.0f32;
                for j in -1i32..=2 {
                    let wy = cubic_weight(fy - j as f32);
                    let yy = (y_base + j).clamp(0, h_in as i32 - 1) as usize;
                    for i in -1i32..=2 {
                        let wx = cubic_weight(fx - i as f32);
                        let w = wx * wy;
                        let xx = (x_base + i).clamp(0, w_in as i32 - 1) as usize;
                        acc += input[ch * h_in * w_in + yy * w_in + xx] * w;
                        wsum += w;
                    }
                }
                let inv = if wsum.abs() > 1e-6 { 1.0 / wsum } else { 0.0 };
                out[ch * h_out * w_out + oy * w_out + ox] = acc * inv;
            }
        }
    }
    Ok(out)
}

#[inline]
fn cubic_weight(t: f32) -> f32 {
    let a = -0.5f32;
    let x = t.abs();
    if x <= 1.0 {
        (a + 2.0) * x * x * x - (a + 3.0) * x * x + 1.0
    } else if x < 2.0 {
        a * x * x * x - 5.0 * a * x * x + 8.0 * a * x - 4.0 * a
    } else {
        0.0
    }
}

/// Attempt bicubic NCHW resize on the auxiliary GPU circuit (iGPU when present; falls back
/// auxiliary → primary → CPU). Returns `Err(BackendUnavailable)` on a GPU-less box so the caller
/// degrades to the CPU oracle. Never panics.
#[cfg(all(feature = "gpu-runtime", not(target_arch = "wasm32")))]
pub fn try_resize_bicubic_shared_gpu(
    input: &[f32],
    c: usize,
    h_in: usize,
    w_in: usize,
    h_out: usize,
    w_out: usize,
    out: &mut [f32],
) -> Result<VisionComputeReport, VisionError> {
    use crate::wgsl_forge::execute::{BindingUsage, GraphPass, WgpuComputeContext};
    use crate::wgsl_forge::schedule::Schedule;

    let need_in = c.saturating_mul(h_in).saturating_mul(w_in);
    let need_out = c.saturating_mul(h_out).saturating_mul(w_out);
    if input.len() < need_in || out.len() < need_out || c == 0 || h_in == 0 || w_in == 0 {
        return Err(VisionError::MalformedImage);
    }
    if h_out == 0 || w_out == 0 {
        return Err(VisionError::MalformedImage);
    }

    // Route through the device-per-circuit registry: auxiliary (iGPU) → primary → None. This keeps
    // the discrete GPU free for the LLM, and — unlike `shared_gpu()`, which panics with no adapter —
    // fails closed to the caller's CPU fallback on a GPU-less machine (never panics).
    let gpu = match crate::gpu_context::device_registry::try_auxiliary_gpu() {
        Some(g) => g,
        None => return Err(VisionError::BackendUnavailable),
    };
    let mut ctx = WgpuComputeContext::from_device(
        gpu.device.clone(),
        gpu.queue.clone(),
        &gpu.adapter_caps,
        16 << 20,
    )
    .map_err(|_| VisionError::BackendUnavailable)?;

    let in_bytes = bytemuck::cast_slice(&input[..need_in]);
    let view_in = ctx
        .allocate_and_write(in_bytes, 0, 0, BindingUsage::StorageRead)
        .map_err(|_| VisionError::BackendUnavailable)?;
    let out_bytes = (need_out * 4).max(4);
    let view_out = ctx
        .allocate_transient(out_bytes, 1, 0, BindingUsage::StorageReadWrite)
        .map_err(|_| VisionError::BackendUnavailable)?;
    let params: [u32; 8] = [
        c as u32,
        h_in as u32,
        w_in as u32,
        h_out as u32,
        w_out as u32,
        0,
        0,
        0,
    ];
    let view_params = ctx
        .allocate_and_write(bytemuck::bytes_of(&params), 2, 0, BindingUsage::StorageRead)
        .map_err(|_| VisionError::BackendUnavailable)?;

    const WG: u32 = 64;
    let src = bicubic2d_wgsl(WG);
    let pipeline = ctx
        .compile_pipeline_cached(&src, BICUBIC_ENTRY)
        .map_err(|_| VisionError::BackendUnavailable)?;
    let bindings = [view_in, view_out, view_params];
    let bind_group = ctx.create_compute_bind_group(&pipeline, &bindings);
    let sched = Schedule {
        workgroup_size: WG,
        ..Default::default()
    };
    let workgroups = sched.dispatch_workgroups(need_out);
    let pass = GraphPass {
        pipeline,
        bind_group,
        workgroups,
        copy: None,
    };
    ctx.submit_graph(&[pass])
        .map_err(|_| VisionError::BackendUnavailable)?;
    let result = ctx
        .read_buffer_f32(&view_out)
        .map_err(|_| VisionError::BackendUnavailable)?;
    if result.len() < need_out {
        return Err(VisionError::OutputBufferTooSmall);
    }
    out[..need_out].copy_from_slice(&result[..need_out]);
    ctx.clear_transient_allocations();
    Ok(VisionComputeReport {
        device: VisionComputeDevice::SharedGpu,
        degraded: false,
    })
}

#[cfg(not(all(feature = "gpu-runtime", not(target_arch = "wasm32"))))]
pub fn try_resize_bicubic_shared_gpu(
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

    #[test]
    fn cpu_oracle_upscales_flat() {
        let input = [0.5f32; 4]; // 1×2×2
        let out = bicubic2d_cpu(&input, 1, 2, 2, 4, 4).unwrap();
        assert_eq!(out.len(), 16);
        for v in &out {
            assert!((*v - 0.5).abs() < 1e-3);
        }
    }

    #[test]
    fn gpu_or_unavailable() {
        let input = [0.0f32, 1.0, 0.0, 1.0];
        let cpu = bicubic2d_cpu(&input, 1, 2, 2, 4, 4).unwrap();
        let mut gpu = [0.0f32; 16];
        match try_resize_bicubic_shared_gpu(&input, 1, 2, 2, 4, 4, &mut gpu) {
            Ok(r) => {
                assert_eq!(r.device, VisionComputeDevice::SharedGpu);
                for i in 0..16 {
                    assert!(
                        (cpu[i] - gpu[i]).abs() < 2e-2,
                        "cpu {} gpu {} @{}",
                        cpu[i],
                        gpu[i],
                        i
                    );
                }
            }
            Err(VisionError::BackendUnavailable) => {}
            Err(e) => panic!("unexpected {e:?}"),
        }
    }
}
