//! Native `Broadcast` op-node — tile a vector across a larger output by index remap
//! (`out[i] = input[i % in_len]`). This is RMSNorm/bias scale-fanout: a per-feature
//! vector `scale[d]` expanded across tokens so `out[t*d + j] = scale[j]`. No arithmetic,
//! one invocation per output element.
//!
//! Certified the forge way: exact CPU oracle, naga validation, GPU differential test.

use crate::wgsl_forge::ForgeError;

/// Entry-point name of the emitted broadcast kernel.
pub const BROADCAST_ENTRY: &str = "broadcast_main";

/// Emit the complete WGSL module for a `Broadcast` at workgroup size `wg`. Binding ABI:
/// `input` (0, storage read), `output` (1, storage read_write), `params` (2, storage
/// read, `[in_len, out_len, _, _]` u32).
pub fn broadcast_wgsl(wg: u32) -> String {
    format!(
        r#"@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<storage, read> params: array<u32>;
@compute @workgroup_size({wg})
fn {entry}(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    let out_len = params[1];
    if (i >= out_len) {{ return; }}
    let in_len = params[0];
    output[i] = input[i % in_len];
}}
"#,
        entry = BROADCAST_ENTRY,
    )
}

/// Exact CPU oracle for [`broadcast_wgsl`]: `out[i] = input[i % in_len]` for
/// `i ∈ [0, out_len)`. An empty `input` yields a zero-filled output.
pub fn broadcast_cpu(input: &[f32], out_len: usize) -> Vec<f32> {
    if input.is_empty() {
        return vec![0.0; out_len];
    }
    (0..out_len).map(|i| input[i % input.len()]).collect()
}

/// Run the broadcast on the GPU and read back the `out_len`-element result.
pub fn broadcast_gpu(input: &[f32], out_len: usize) -> Result<Vec<f32>, ForgeError> {
    use crate::wgsl_forge::execute::{
        BindingUsage, QualiaCompute, WgpuComputeContext, WgpuPipeline,
    };
    use crate::wgsl_forge::Schedule;

    if input.is_empty() || out_len == 0 {
        return Err(ForgeError::GpuValidation(
            "broadcast_gpu: empty input or zero out_len".to_string(),
        ));
    }
    let wg: u32 = 64;
    let src = broadcast_wgsl(wg);
    let capacity = (out_len * 4).max(4 << 20);
    let mut ctx = WgpuComputeContext::new(capacity)?;

    let view_in =
        ctx.allocate_and_write(bytemuck::cast_slice(input), 0, 0, BindingUsage::StorageRead)?;
    let out_zero = vec![0.0f32; out_len];
    let view_out = ctx.allocate_and_write(
        bytemuck::cast_slice(&out_zero),
        1,
        0,
        BindingUsage::StorageReadWrite,
    )?;
    let params: [u32; 4] = [input.len() as u32, out_len as u32, 0, 0];
    let view_params =
        ctx.allocate_and_write(bytemuck::cast_slice(&params), 2, 0, BindingUsage::StorageRead)?;

    let buffers = vec![view_in, view_out, view_params];
    let pipeline = WgpuPipeline::compile(&ctx, &src, BROADCAST_ENTRY)?;
    let schedule = Schedule { workgroup_size: wg, ..Default::default() };
    pipeline.dispatch(&buffers, &schedule, out_len)?;
    let mut out = ctx.read_buffer_f32(&view_out)?;
    out.truncate(out_len);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::validate::validate_wgsl;

    #[test]
    fn broadcast_cpu_tiles_the_vector() {
        // scale = [a, b, c] expanded across 2 tokens → [a,b,c, a,b,c].
        let scale = [1.0f32, 2.0, 3.0];
        assert_eq!(
            broadcast_cpu(&scale, 6),
            vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0]
        );
        // A scalar fans out to a constant vector.
        assert_eq!(broadcast_cpu(&[7.0], 4), vec![7.0, 7.0, 7.0, 7.0]);
    }

    #[test]
    fn broadcast_wgsl_validates() {
        let src = broadcast_wgsl(64);
        let report = validate_wgsl(&src).expect("broadcast WGSL must naga-validate");
        assert!(report.entry_points.iter().any(|e| e == BROADCAST_ENTRY));
    }

    #[test]
    #[ignore = "requires a GPU adapter"]
    fn broadcast_gpu_matches_oracle() {
        let scale: Vec<f32> = (0..50).map(|i| (i as f32) * 0.1 - 2.0).collect();
        let out_len = 50 * 37 + 13; // not a multiple of in_len, exercises the modulo
        let gpu = broadcast_gpu(&scale, out_len).expect("broadcast_gpu");
        let cpu = broadcast_cpu(&scale, out_len);
        assert_eq!(gpu.len(), cpu.len());
        for (g, c) in gpu.iter().zip(cpu.iter()) {
            assert_eq!(g, c, "broadcast must be exact (index remap)");
        }
    }
}
