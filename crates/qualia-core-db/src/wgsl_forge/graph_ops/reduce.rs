//! Native `Reduce` op-node — the first compute-graph node the WGSL backend emits
//! *from scratch* (Phase 1's gemm/gemv/fft delegate to legacy emitters; there is no
//! legacy standalone reduce, so this is a real graph-template lowering).
//!
//! A single-workgroup tree reduction along the (flattened) input: each thread folds a
//! grid-strided slice into a partial, the partials are tree-reduced in workgroup-shared
//! memory, and thread 0 writes the scalar result to `output[0]`. This is the RMSNorm
//! variance (`Sum` of squares → `L2`/`Mean`) and softmax (`Max`, `Sum`) primitive.
//!
//! Certified the forge way: an exact CPU oracle ([`reduce_cpu`]), naga validation, and a
//! GPU differential test on the A2000 ([`reduce_gpu`] vs the oracle within f32 tolerance).

use crate::wgsl_forge::ir::graph::RedKind;
use crate::wgsl_forge::ForgeError;

/// Entry-point name of the emitted reduce kernel.
pub const REDUCE_ENTRY: &str = "reduce_main";

/// Per-`RedKind` WGSL fragments: the per-thread accumulator init, the fold of one input
/// element `x` into `acc`, the pairwise combine of two shared slots, and the finalize
/// applied by thread 0 to `scratch[0]` (with `n` available as `f32`).
fn fragments(op: RedKind) -> (&'static str, &'static str, &'static str, &'static str) {
    match op {
        // init,        fold,            pair-combine,                       finalize
        RedKind::Sum => ("0.0", "acc + x", "scratch[tid] + scratch[tid + stride]", "scratch[0]"),
        RedKind::Mean => (
            "0.0",
            "acc + x",
            "scratch[tid] + scratch[tid + stride]",
            "scratch[0] / f32(n)",
        ),
        RedKind::L2 => (
            "0.0",
            "acc + x * x",
            "scratch[tid] + scratch[tid + stride]",
            "sqrt(scratch[0])",
        ),
        // f32::MIN sentinel so empty/past-end threads never win.
        RedKind::Max => (
            "bitcast<f32>(0xff7fffffu)",
            "max(acc, x)",
            "max(scratch[tid], scratch[tid + stride])",
            "scratch[0]",
        ),
    }
}

/// Emit the complete WGSL module for a `Reduce` of kind `op` at workgroup size `wg`
/// (a power of two). Binding ABI: `input` (0, storage read), `output` (1, storage
/// read_write, only `[0]` is written), `params` (2, storage read, `[n, _, _, _]` u32).
pub fn reduce_wgsl(op: RedKind, wg: u32) -> String {
    let (init, fold, pair, finalize) = fragments(op);
    format!(
        r#"@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<storage, read> params: array<u32>;
var<workgroup> scratch: array<f32, {wg}>;
@compute @workgroup_size({wg})
fn {entry}(@builtin(local_invocation_id) lid: vec3<u32>) {{
    let tid = lid.x;
    let n = params[0];
    var acc = {init};
    var i = tid;
    loop {{
        if (i >= n) {{ break; }}
        let x = input[i];
        acc = {fold};
        i = i + {wg}u;
    }}
    scratch[tid] = acc;
    workgroupBarrier();
    for (var stride: u32 = {wg}u / 2u; stride > 0u; stride = stride / 2u) {{
        if (tid < stride) {{
            scratch[tid] = {pair};
        }}
        workgroupBarrier();
    }}
    if (tid == 0u) {{
        output[0] = {finalize};
    }}
}}
"#,
        entry = REDUCE_ENTRY,
    )
}

/// Exact CPU oracle for [`reduce_wgsl`]: the scalar reduction of `input` under `op`.
/// `Sum`/`Mean`/`L2` accumulate in `f64` and cast back (a tighter floor than the GPU's
/// f32 tree); `Max` is exact. An empty input reduces to the identity
/// (`0`/`0`/`0`/`f32::MIN`).
pub fn reduce_cpu(input: &[f32], op: RedKind) -> f32 {
    match op {
        RedKind::Sum => input.iter().map(|&x| x as f64).sum::<f64>() as f32,
        RedKind::Mean => {
            if input.is_empty() {
                0.0
            } else {
                (input.iter().map(|&x| x as f64).sum::<f64>() / input.len() as f64) as f32
            }
        }
        RedKind::L2 => (input.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>().sqrt()) as f32,
        RedKind::Max => input.iter().copied().fold(f32::MIN, f32::max),
    }
}

/// Run the reduce on the GPU (one workgroup) and read back the scalar. Builds a transient
/// wgpu context, uploads `input` (0), a zeroed output (1) and `params=[n,0,0,0]` (2),
/// dispatches one workgroup of `wg` threads, and returns `output[0]`.
pub fn reduce_gpu(input: &[f32], op: RedKind) -> Result<f32, ForgeError> {
    use crate::wgsl_forge::execute::{
        BindingUsage, QualiaCompute, WgpuComputeContext, WgpuPipeline,
    };
    use crate::wgsl_forge::Schedule;

    if input.is_empty() {
        return Err(ForgeError::GpuValidation("reduce_gpu: empty input".to_string()));
    }
    let wg: u32 = 256;
    let src = reduce_wgsl(op, wg);
    let capacity = (input.len() * 4).max(4 << 20);
    let mut ctx = WgpuComputeContext::new(capacity)?;

    let view_in =
        ctx.allocate_and_write(bytemuck::cast_slice(input), 0, 0, BindingUsage::StorageRead)?;
    let out_zero = vec![0.0f32; wg as usize];
    let view_out = ctx.allocate_and_write(
        bytemuck::cast_slice(&out_zero),
        1,
        0,
        BindingUsage::StorageReadWrite,
    )?;
    let params: [u32; 4] = [input.len() as u32, 0, 0, 0];
    let view_params =
        ctx.allocate_and_write(bytemuck::cast_slice(&params), 2, 0, BindingUsage::StorageRead)?;

    let buffers = vec![view_in, view_out, view_params];
    let pipeline = WgpuPipeline::compile(&ctx, &src, REDUCE_ENTRY)?;
    let schedule = Schedule { workgroup_size: wg, ..Default::default() };
    // element_count == wg → ceil(wg / wg) == 1 workgroup (the kernel is single-workgroup).
    pipeline.dispatch(&buffers, &schedule, wg as usize)?;
    let out = ctx.read_buffer_f32(&view_out)?;
    out.first()
        .copied()
        .ok_or_else(|| ForgeError::GpuValidation("reduce_gpu: empty readback".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::validate::validate_wgsl;

    #[test]
    fn reduce_cpu_hand_checked() {
        let v = [1.0f32, 2.0, 3.0, 4.0];
        assert!((reduce_cpu(&v, RedKind::Sum) - 10.0).abs() < 1e-6);
        assert!((reduce_cpu(&v, RedKind::Mean) - 2.5).abs() < 1e-6);
        assert!((reduce_cpu(&v, RedKind::Max) - 4.0).abs() < 1e-6);
        // L2 = sqrt(1+4+9+16) = sqrt(30).
        assert!((reduce_cpu(&v, RedKind::L2) - 30.0f32.sqrt()).abs() < 1e-5);
    }

    #[test]
    fn reduce_wgsl_validates_each_kind() {
        for op in [RedKind::Sum, RedKind::Mean, RedKind::Max, RedKind::L2] {
            let src = reduce_wgsl(op, 256);
            let report = validate_wgsl(&src).expect("reduce WGSL must naga-validate");
            assert!(report.entry_points.iter().any(|e| e == REDUCE_ENTRY));
        }
    }

    /// GPU certify on a real adapter: each reduce kind must match the CPU oracle within
    /// f32 tolerance over a non-trivial input (length > workgroup size, exercising the
    /// grid-stride fold + tree reduction). Run by the orchestrator.
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn reduce_gpu_matches_oracle() {
        let n = 4096usize; // > wg (256) → multiple grid-stride iterations per thread
        let input: Vec<f32> = (0..n).map(|i| ((i * 7 % 23) as f32) * 0.5 - 5.0).collect();
        for op in [RedKind::Sum, RedKind::Mean, RedKind::Max, RedKind::L2] {
            let gpu = reduce_gpu(&input, op).expect("reduce_gpu");
            let cpu = reduce_cpu(&input, op);
            let tol = 1e-3 * cpu.abs().max(1.0);
            assert!(
                (gpu - cpu).abs() <= tol,
                "{op:?}: GPU {gpu} vs CPU {cpu} (tol {tol})"
            );
        }
    }
}
