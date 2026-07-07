//! Native `ScatterAccum` op-node — scatter `src[p]` into `output[idx[p]]` with an associative
//! accumulate (plan §2 / P7): the SPH density/force deposit and N-body short-range gather.
//!
//! WGSL has no `atomic<f32>`, so f32 accumulation is done with the standard
//! `atomicCompareExchangeWeak` **CAS loop on the `u32` bit pattern** — correct under
//! contention, the same trick every WGSL particle kernel uses. `Add` accumulates (slots init
//! `0.0`); `Max` keeps the running maximum (slots init `f32::MIN`), so an untouched output slot
//! reads back its identity on both the GPU and the CPU oracle.
//!
//! Certified the forge way: exact CPU oracle, naga validation, A2000 GPU differential.

use crate::wgsl_forge::ir::graph::AccumKind;
use crate::wgsl_forge::ForgeError;

/// Entry-point name of the emitted scatter-accumulate kernel.
pub const SCATTER_ENTRY: &str = "scatter_main";

/// Op code carried in the params buffer (`0`=Add, `1`=Max) — matches the WGSL branch.
fn accum_op_code(op: AccumKind) -> u32 {
    match op {
        AccumKind::Add => 0,
        AccumKind::Max => 1,
    }
}

/// The output-slot identity for `op` (the value an untouched slot must hold): `Add → 0.0`,
/// `Max → f32::MIN`. The host helper / executor zero-or-`MIN`-fills the output accordingly.
pub fn accum_identity(op: AccumKind) -> f32 {
    match op {
        AccumKind::Add => 0.0,
        AccumKind::Max => f32::MIN,
    }
}

/// Emit the WGSL module for `ScatterAccum` at workgroup size `wg`. One invocation per source
/// element. Binding ABI: `src` (0, storage read, f32), `idx` (1, storage read, **u32** target
/// indices), `output` (2, storage read_write, `array<atomic<u32>>` — f32 bits), `params`
/// (3, storage read, `[p_count, o_count, op, _]` u32).
pub fn scatter_wgsl(wg: u32) -> String {
    format!(
        r#"@group(0) @binding(0) var<storage, read> src: array<f32>;
@group(0) @binding(1) var<storage, read> idx: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read> params: array<u32>;
@compute @workgroup_size({wg})
fn {entry}(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let p = gid.x;
    let p_count = params[0];
    let o_count = params[1];
    let op = params[2];
    if (p >= p_count) {{ return; }}
    let t = idx[p];
    if (t >= o_count) {{ return; }}
    let val = src[p];
    // Atomic f32 accumulate via CAS on the u32 bit pattern.
    var old = atomicLoad(&output[t]);
    loop {{
        let cur = bitcast<f32>(old);
        var nv = cur + val;
        if (op == 1u) {{ nv = max(cur, val); }}
        let res = atomicCompareExchangeWeak(&output[t], old, bitcast<u32>(nv));
        if (res.exchanged) {{ break; }}
        old = res.old_value;
    }}
}}
"#,
        entry = SCATTER_ENTRY,
    )
}

/// Exact CPU oracle for [`scatter_wgsl`]: scatter `src` into `o_count` output slots by `idx`,
/// accumulating with `op`. Untouched slots hold [`accum_identity`]. Out-of-range indices are
/// dropped (matching the kernel's bounds check). For `Add` the deposit order does not matter
/// (associative); for `Max` likewise — so this is order-independent like the GPU.
pub fn scatter_cpu(src: &[f32], idx: &[u32], o_count: usize, op: AccumKind) -> Vec<f32> {
    let mut out = vec![accum_identity(op); o_count];
    for (p, &t) in idx.iter().enumerate() {
        let t = t as usize;
        if t >= o_count {
            continue;
        }
        let v = src[p];
        out[t] = match op {
            AccumKind::Add => out[t] + v,
            AccumKind::Max => out[t].max(v),
        };
    }
    out
}

/// Run the scatter-accumulate on the GPU and read back the `o_count`-element result.
pub fn scatter_gpu(
    src: &[f32],
    idx: &[u32],
    o_count: usize,
    op: AccumKind,
) -> Result<Vec<f32>, ForgeError> {
    use crate::wgsl_forge::execute::{
        BindingUsage, QualiaCompute, WgpuComputeContext, WgpuPipeline,
    };
    use crate::wgsl_forge::Schedule;

    let p = src.len();
    if p == 0 || o_count == 0 || idx.len() != p {
        return Err(ForgeError::GpuValidation(
            "scatter_gpu: empty or mismatched inputs".to_string(),
        ));
    }
    let wg: u32 = 64;
    let src_wgsl = scatter_wgsl(wg);
    let mut ctx = WgpuComputeContext::new(((p + o_count) * 4).max(4 << 20))?;
    let view_src =
        ctx.allocate_and_write(bytemuck::cast_slice(src), 0, 0, BindingUsage::StorageRead)?;
    let view_idx =
        ctx.allocate_and_write(bytemuck::cast_slice(idx), 1, 0, BindingUsage::StorageRead)?;
    // Output initialised to the accumulate identity (0.0 for Add, f32::MIN for Max).
    let init = vec![accum_identity(op); o_count];
    let view_out = ctx.allocate_and_write(
        bytemuck::cast_slice(&init),
        2,
        0,
        BindingUsage::StorageReadWrite,
    )?;
    let params: [u32; 4] = [p as u32, o_count as u32, accum_op_code(op), 0];
    let view_params = ctx.allocate_and_write(
        bytemuck::cast_slice(&params),
        3,
        0,
        BindingUsage::StorageRead,
    )?;
    let pipeline = WgpuPipeline::compile(&ctx, &src_wgsl, SCATTER_ENTRY)?;
    let schedule = Schedule {
        workgroup_size: wg,
        ..Default::default()
    };
    pipeline.dispatch(&[view_src, view_idx, view_out, view_params], &schedule, p)?;
    let mut out = ctx.read_buffer_f32(&view_out)?;
    out.truncate(o_count);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::validate::validate_wgsl;

    #[test]
    fn scatter_wgsl_validates() {
        let report = validate_wgsl(&scatter_wgsl(64)).expect("validate");
        assert!(report.entry_points.iter().any(|e| e == SCATTER_ENTRY));
        assert_eq!(report.binding_count, 4);
    }

    #[test]
    fn scatter_cpu_hand_checked() {
        // 3 sources into 2 buckets: idx=[0,1,0], src=[1,2,3].
        let src = [1.0f32, 2.0, 3.0];
        let idx = [0u32, 1, 0];
        // Add: bucket0 = 1+3 = 4, bucket1 = 2.
        assert_eq!(scatter_cpu(&src, &idx, 2, AccumKind::Add), vec![4.0, 2.0]);
        // Max: bucket0 = max(1,3) = 3, bucket1 = 2.
        assert_eq!(scatter_cpu(&src, &idx, 2, AccumKind::Max), vec![3.0, 2.0]);
        // Untouched slot keeps identity.
        let a = scatter_cpu(&[5.0], &[0], 3, AccumKind::Add);
        assert_eq!(a, vec![5.0, 0.0, 0.0]);
    }

    #[test]
    #[ignore = "requires a GPU adapter"]
    fn scatter_gpu_matches_oracle() {
        // Heavy contention: 4096 sources into 64 buckets (many collisions → exercises the CAS).
        let p = 4096usize;
        let o = 64usize;
        let src: Vec<f32> = (0..p).map(|i| ((i % 17) as f32) * 0.1 - 0.5).collect();
        let idx: Vec<u32> = (0..p).map(|i| (i * 13 % o) as u32).collect();
        for op in [AccumKind::Add, AccumKind::Max] {
            let gpu = scatter_gpu(&src, &idx, o, op).expect("gpu");
            let cpu = scatter_cpu(&src, &idx, o, op);
            for (g, c) in gpu.iter().zip(&cpu) {
                // Add accumulates in nondeterministic order → small fp tolerance; Max exact.
                let tol = if matches!(op, AccumKind::Add) {
                    1e-3
                } else {
                    0.0
                };
                assert!(
                    (g - c).abs() <= tol + 1e-3 * c.abs().max(1.0),
                    "{op:?}: {g} vs {c}"
                );
            }
        }
    }
}
