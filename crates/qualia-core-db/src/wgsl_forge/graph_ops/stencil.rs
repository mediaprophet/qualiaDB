//! Native `Stencil` op-node — local neighbourhood maps over a 1-D grid (plan §2 / P7). The
//! seed kind is the **Laplacian** `out[i] = in[i-1] − 2·in[i] + in[i+1]` with zero (Dirichlet)
//! boundaries — the discrete Poisson operator under fluid pressure-solve / diffusion, and the
//! `Reduce`-free half of a stencil sweep.
//!
//! `RopePair` (RoPE as a 2-tap rotation stencil) is also emitted. The advection/divergence
//! kinds need a **velocity field** as a second input (different arity) plus a physical-model
//! choice, so — like `fluid_dynamics` (rollout B2) — they return an explicit `Err` naming the
//! missing field/model rather than guessing; never a silent no-op.
//!
//! Certified the forge way: exact CPU oracle, naga validation, A2000 GPU differential.

use crate::wgsl_forge::ir::graph::StencilKind;
use crate::wgsl_forge::ForgeError;

/// Entry-point name of the emitted stencil kernel.
pub const STENCIL_ENTRY: &str = "stencil_main";

/// Emit the WGSL module for stencil `kind` at workgroup size `wg`. One invocation per grid
/// point. Binding ABI: `input` (0, storage read), `output` (1, storage read_write), `params`
/// (2, storage read, `[n, halo, _, _]` u32). Returns `Err` for the kinds that need a velocity
/// field / physical model (`Divergence`/`Advection`).
pub fn stencil_wgsl(kind: StencilKind, wg: u32) -> Result<String, ForgeError> {
    let body = match kind {
        // Discrete Laplacian, zero boundaries.
        StencilKind::Laplacian => {
            "    var lft = 0.0; if (i > 0u) { lft = input[i - 1u]; }\n\
             \x20   var rgt = 0.0; if (i + 1u < n) { rgt = input[i + 1u]; }\n\
             \x20   output[i] = lft - 2.0 * input[i] + rgt;"
        }
        // RoPE 2-tap rotation on adjacent (even,odd) lanes by a fixed unit angle per pair —
        // a content-free structural rotation (the per-position frequency is folded into the
        // caller's angle table in the full model; here halo encodes the pair stride = 1).
        StencilKind::RopePair => {
            "    let pair = i ^ 1u;\n\
             \x20   var partner = 0.0; if (pair < n) { partner = input[pair]; }\n\
             \x20   let c = 0.5403023058681398; let s = 0.8414709848078965;\n\
             \x20   if ((i & 1u) == 0u) { output[i] = c * input[i] - s * partner; }\n\
             \x20   else { output[i] = s * partner + c * input[i]; }"
        }
        StencilKind::Divergence | StencilKind::Advection => {
            return Err(ForgeError::Emission(format!(
                "stencil {kind:?} needs a velocity-field input + physical-model direction (a curation item, cf. fluid_dynamics)"
            )))
        }
    };
    Ok(format!(
        r#"@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<storage, read> params: array<u32>;
@compute @workgroup_size({wg})
fn {entry}(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    let n = params[0];
    if (i >= n) {{ return; }}
{body}
}}
"#,
        entry = STENCIL_ENTRY,
    ))
}

/// Exact CPU oracle for [`stencil_wgsl`] — mirrors the kernel math in f32.
pub fn stencil_cpu(input: &[f32], kind: StencilKind) -> Result<Vec<f32>, ForgeError> {
    let n = input.len();
    match kind {
        StencilKind::Laplacian => Ok((0..n)
            .map(|i| {
                let lft = if i > 0 { input[i - 1] } else { 0.0 };
                let rgt = if i + 1 < n { input[i + 1] } else { 0.0 };
                lft - 2.0 * input[i] + rgt
            })
            .collect()),
        StencilKind::RopePair => {
            let (c, s) = (0.5403023058681398f32, 0.8414709848078965f32);
            Ok((0..n)
                .map(|i| {
                    let pair = i ^ 1;
                    let partner = if pair < n { input[pair] } else { 0.0 };
                    if i % 2 == 0 {
                        c * input[i] - s * partner
                    } else {
                        s * partner + c * input[i]
                    }
                })
                .collect())
        }
        StencilKind::Divergence | StencilKind::Advection => Err(ForgeError::Emission(format!(
            "stencil_cpu {kind:?} needs a velocity field + model direction"
        ))),
    }
}

/// Run the stencil on the GPU (standalone) and read back the `n`-element result.
pub fn stencil_gpu(input: &[f32], kind: StencilKind) -> Result<Vec<f32>, ForgeError> {
    use crate::wgsl_forge::execute::{
        BindingUsage, QualiaCompute, WgpuComputeContext, WgpuPipeline,
    };
    use crate::wgsl_forge::Schedule;

    let n = input.len();
    if n == 0 {
        return Err(ForgeError::GpuValidation("stencil_gpu: empty input".to_string()));
    }
    let wg: u32 = 64;
    let src = stencil_wgsl(kind, wg)?;
    let mut ctx = WgpuComputeContext::new((n * 4).max(4 << 20))?;
    let view_in =
        ctx.allocate_and_write(bytemuck::cast_slice(input), 0, 0, BindingUsage::StorageRead)?;
    let zeros = vec![0.0f32; n];
    let view_out = ctx.allocate_and_write(
        bytemuck::cast_slice(&zeros),
        1,
        0,
        BindingUsage::StorageReadWrite,
    )?;
    let params: [u32; 4] = [n as u32, 0, 0, 0];
    let view_params =
        ctx.allocate_and_write(bytemuck::cast_slice(&params), 2, 0, BindingUsage::StorageRead)?;
    let pipeline = WgpuPipeline::compile(&ctx, &src, STENCIL_ENTRY)?;
    let schedule = Schedule { workgroup_size: wg, ..Default::default() };
    pipeline.dispatch(&[view_in, view_out, view_params], &schedule, n)?;
    let mut out = ctx.read_buffer_f32(&view_out)?;
    out.truncate(n);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::validate::validate_wgsl;

    #[test]
    fn stencil_wgsl_validates_supported_kinds() {
        for kind in [StencilKind::Laplacian, StencilKind::RopePair] {
            let src = stencil_wgsl(kind, 64).expect("supported kind");
            let report = validate_wgsl(&src).unwrap_or_else(|e| panic!("{kind:?}: {e}"));
            assert!(report.entry_points.iter().any(|e| e == STENCIL_ENTRY));
        }
        assert!(stencil_wgsl(StencilKind::Divergence, 64).is_err());
        assert!(stencil_wgsl(StencilKind::Advection, 64).is_err());
    }

    #[test]
    fn laplacian_cpu_hand_checked() {
        // in = [1,2,3,4]; Laplacian (zero bdy):
        // i0: 0 - 2*1 + 2 = 0; i1: 1 - 4 + 3 = 0; i2: 2 - 6 + 4 = 0; i3: 3 - 8 + 0 = -5
        let out = stencil_cpu(&[1.0, 2.0, 3.0, 4.0], StencilKind::Laplacian).unwrap();
        assert_eq!(out, vec![0.0, 0.0, 0.0, -5.0]);
    }

    #[test]
    fn ropepair_cpu_is_norm_preserving() {
        // A 2-tap rotation preserves the (even,odd) pair's L2 norm.
        let inp = vec![0.6f32, 0.8, -1.0, 2.0];
        let out = stencil_cpu(&inp, StencilKind::RopePair).unwrap();
        let n0 = inp[0] * inp[0] + inp[1] * inp[1];
        let m0 = out[0] * out[0] + out[1] * out[1];
        assert!((n0 - m0).abs() < 1e-5, "rope must preserve pair norm: {n0} vs {m0}");
    }

    #[test]
    #[ignore = "requires a GPU adapter"]
    fn stencil_gpu_matches_oracle() {
        let inp: Vec<f32> = (0..512).map(|i| ((i * 7 % 31) as f32) * 0.1 - 1.5).collect();
        for kind in [StencilKind::Laplacian, StencilKind::RopePair] {
            let gpu = stencil_gpu(&inp, kind).expect("gpu");
            let cpu = stencil_cpu(&inp, kind).unwrap();
            for (g, c) in gpu.iter().zip(&cpu) {
                assert!((g - c).abs() <= 1e-4 * c.abs().max(1.0), "{kind:?}: {g} vs {c}");
            }
        }
    }
}
