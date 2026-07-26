//! Native `Elementwise` op-node — per-element maps, the LLM activation/arithmetic kit.
//!
//! Three arities, one op-class:
//! - **unary** `out[i] = f(in[i])` — `Silu`, `Gelu`, `Exp`, `RecipSqrt`, `Relu`, `Recip`
//!   (SwiGLU's `silu(gate)`, softmax's `exp`, RMSNorm's `rsqrt`);
//! - **binary** `out[i] = a[i] ⊙ b[i]` — `Add`, `Mul` (SwiGLU's `silu(gate)·up`, residual add);
//! - **fma** `out[i] = a[i]·b[i] + c[i]`.
//!
//! Certified the forge way: exact CPU oracles, naga validation, A2000 GPU differential.
//! `Scale`/`Bias` (affine with a scalar parameter) are intentionally **not** handled here —
//! the certified `affine-f32` BuiltinKernel already covers them — so the lowerer returns an
//! explicit `Err` for those, never a silent no-op.

use crate::wgsl_forge::ir::graph::EwKind;
use crate::wgsl_forge::ForgeError;

/// Entry-point name of every emitted elementwise kernel.
pub const EWISE_ENTRY: &str = "ewise_main";

/// WGSL expression computing `f(v)` for a unary kind, or `None` if `kind` is not unary.
fn unary_expr(kind: EwKind) -> Option<&'static str> {
    Some(match kind {
        EwKind::Silu => "v / (1.0 + exp(-v))",
        // tanh approximation (the GGUF/LLM-standard GELU).
        EwKind::Gelu => "0.5 * v * (1.0 + tanh(0.7978845608028654 * (v + 0.044715 * v * v * v)))",
        EwKind::Exp => "exp(v)",
        EwKind::RecipSqrt => "inverseSqrt(v)",
        EwKind::Relu => "max(v, 0.0)",
        EwKind::Recip => "1.0 / v",
        _ => return None,
    })
}

/// WGSL expression computing `f(a, b)` for a binary kind, or `None` otherwise.
fn binary_expr(kind: EwKind) -> Option<&'static str> {
    Some(match kind {
        EwKind::Add => "a + b",
        EwKind::Sub => "a - b",
        EwKind::Mul => "a * b",
        EwKind::Div => "a / b",
        _ => return None,
    })
}

/// Whether a kind is the ternary fused-multiply-add.
fn is_fma(kind: EwKind) -> bool {
    matches!(kind, EwKind::Fma)
}

/// Emit the complete WGSL module for an elementwise `kind` at workgroup size `wg`,
/// dispatching the correct arity (unary / binary / fma). Returns `Err` for kinds with no
/// elementwise kernel here (`Scale`/`Bias` → use the affine kernel). Binding ABI:
/// inputs first (1, 2, or 3 storage-read), then `output` (storage read_write), then
/// `params` (storage read, `[n, …]` u32).
pub fn elementwise_wgsl(kind: EwKind, wg: u32) -> Result<String, ForgeError> {
    if let Some(expr) = unary_expr(kind) {
        return Ok(format!(
            r#"@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<storage, read> params: array<u32>;
@compute @workgroup_size({wg})
fn {entry}(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    if (i >= params[0]) {{ return; }}
    let v = input[i];
    output[i] = {expr};
}}
"#,
            entry = EWISE_ENTRY,
        ));
    }
    if let Some(expr) = binary_expr(kind) {
        return Ok(format!(
            r#"@group(0) @binding(0) var<storage, read> lhs: array<f32>;
@group(0) @binding(1) var<storage, read> rhs: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@group(0) @binding(3) var<storage, read> params: array<u32>;
@compute @workgroup_size({wg})
fn {entry}(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    if (i >= params[0]) {{ return; }}
    let a = lhs[i];
    let b = rhs[i];
    output[i] = {expr};
}}
"#,
            entry = EWISE_ENTRY,
        ));
    }
    if is_fma(kind) {
        return Ok(format!(
            r#"@group(0) @binding(0) var<storage, read> a_in: array<f32>;
@group(0) @binding(1) var<storage, read> b_in: array<f32>;
@group(0) @binding(2) var<storage, read> c_in: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;
@group(0) @binding(4) var<storage, read> params: array<u32>;
@compute @workgroup_size({wg})
fn {entry}(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let i = gid.x;
    if (i >= params[0]) {{ return; }}
    output[i] = fma(a_in[i], b_in[i], c_in[i]);
}}
"#,
            entry = EWISE_ENTRY,
        ));
    }
    Err(ForgeError::Emission(format!(
        "elementwise: kind {kind:?} has no elementwise kernel (Scale/Bias use the affine kernel)"
    )))
}

/// Apply a unary `f` on the CPU, matching the WGSL math in f32. Panics-free; mirrors the
/// kernel exactly so it is the differential oracle.
pub fn unary_cpu(input: &[f32], kind: EwKind) -> Vec<f32> {
    input
        .iter()
        .map(|&v| match kind {
            EwKind::Silu => v / (1.0 + (-v).exp()),
            EwKind::Gelu => {
                0.5 * v * (1.0 + (0.797_884_56_f32 * (v + 0.044715 * v * v * v)).tanh())
            }
            EwKind::Exp => v.exp(),
            EwKind::RecipSqrt => v.sqrt().recip(),
            EwKind::Relu => v.max(0.0),
            EwKind::Recip => 1.0 / v,
            _ => v,
        })
        .collect()
}

/// Apply a binary `f` (`Add`/`Mul`) on the CPU.
pub fn binary_cpu(a: &[f32], b: &[f32], kind: EwKind) -> Vec<f32> {
    a.iter()
        .zip(b.iter())
        .map(|(&a, &b)| match kind {
            EwKind::Add => a + b,
            EwKind::Sub => a - b,
            EwKind::Mul => a * b,
            EwKind::Div => a / b,
            _ => a,
        })
        .collect()
}

/// `out[i] = a[i]·b[i] + c[i]` on the CPU (matches the GPU `fma`).
pub fn fma_cpu(a: &[f32], b: &[f32], c: &[f32]) -> Vec<f32> {
    a.iter()
        .zip(b.iter())
        .zip(c.iter())
        .map(|((&a, &b), &c)| a.mul_add(b, c))
        .collect()
}

/// Run a unary elementwise on the GPU and read back the result.
pub fn unary_gpu(input: &[f32], kind: EwKind) -> Result<Vec<f32>, ForgeError> {
    use crate::wgsl_forge::execute::{
        BindingUsage, QualiaCompute, WgpuComputeContext, WgpuPipeline,
    };
    use crate::wgsl_forge::Schedule;

    let n = input.len();
    if n == 0 {
        return Err(ForgeError::GpuValidation(
            "unary_gpu: empty input".to_string(),
        ));
    }
    let wg: u32 = 64;
    let src = elementwise_wgsl(kind, wg)?;
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
    let view_params = ctx.allocate_and_write(
        bytemuck::cast_slice(&params),
        2,
        0,
        BindingUsage::StorageRead,
    )?;
    let pipeline = WgpuPipeline::compile(&ctx, &src, EWISE_ENTRY)?;
    let schedule = Schedule {
        workgroup_size: wg,
        ..Default::default()
    };
    pipeline.dispatch(&[view_in, view_out, view_params], &schedule, n)?;
    let mut out = ctx.read_buffer_f32(&view_out)?;
    out.truncate(n);
    Ok(out)
}

/// Run a binary elementwise (`Add`/`Mul`) on the GPU and read back the result.
pub fn binary_gpu(a: &[f32], b: &[f32], kind: EwKind) -> Result<Vec<f32>, ForgeError> {
    use crate::wgsl_forge::execute::{
        BindingUsage, QualiaCompute, WgpuComputeContext, WgpuPipeline,
    };
    use crate::wgsl_forge::Schedule;

    let n = a.len();
    if n == 0 || b.len() != n {
        return Err(ForgeError::GpuValidation(
            "binary_gpu: empty or mismatched inputs".to_string(),
        ));
    }
    let wg: u32 = 64;
    let src = elementwise_wgsl(kind, wg)?;
    let mut ctx = WgpuComputeContext::new((n * 4).max(4 << 20))?;
    let view_a =
        ctx.allocate_and_write(bytemuck::cast_slice(a), 0, 0, BindingUsage::StorageRead)?;
    let view_b =
        ctx.allocate_and_write(bytemuck::cast_slice(b), 1, 0, BindingUsage::StorageRead)?;
    let zeros = vec![0.0f32; n];
    let view_out = ctx.allocate_and_write(
        bytemuck::cast_slice(&zeros),
        2,
        0,
        BindingUsage::StorageReadWrite,
    )?;
    let params: [u32; 4] = [n as u32, 0, 0, 0];
    let view_params = ctx.allocate_and_write(
        bytemuck::cast_slice(&params),
        3,
        0,
        BindingUsage::StorageRead,
    )?;
    let pipeline = WgpuPipeline::compile(&ctx, &src, EWISE_ENTRY)?;
    let schedule = Schedule {
        workgroup_size: wg,
        ..Default::default()
    };
    pipeline.dispatch(&[view_a, view_b, view_out, view_params], &schedule, n)?;
    let mut out = ctx.read_buffer_f32(&view_out)?;
    out.truncate(n);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::validate::validate_wgsl;

    #[test]
    fn unary_cpu_hand_checked() {
        // Silu(0) = 0; Relu(-1) = 0, Relu(2) = 2; Exp(0) = 1; RecipSqrt(4) = 0.5.
        assert!((unary_cpu(&[0.0], EwKind::Silu)[0]).abs() < 1e-6);
        assert_eq!(unary_cpu(&[-1.0, 2.0], EwKind::Relu), vec![0.0, 2.0]);
        assert!((unary_cpu(&[0.0], EwKind::Exp)[0] - 1.0).abs() < 1e-6);
        assert!((unary_cpu(&[4.0], EwKind::RecipSqrt)[0] - 0.5).abs() < 1e-6);
        // Gelu(0) = 0, and is monotone-ish: Gelu(large +) ≈ x.
        assert!((unary_cpu(&[0.0], EwKind::Gelu)[0]).abs() < 1e-6);
        assert!((unary_cpu(&[6.0], EwKind::Gelu)[0] - 6.0).abs() < 1e-2);
    }

    #[test]
    fn binary_and_fma_cpu_hand_checked() {
        assert_eq!(
            binary_cpu(&[1.0, 2.0], &[3.0, 4.0], EwKind::Add),
            vec![4.0, 6.0]
        );
        assert_eq!(
            binary_cpu(&[2.0, 3.0], &[5.0, 7.0], EwKind::Mul),
            vec![10.0, 21.0]
        );
        assert_eq!(fma_cpu(&[2.0], &[3.0], &[1.0]), vec![7.0]);
    }

    #[test]
    fn elementwise_wgsl_validates_each_arity() {
        for kind in [
            EwKind::Silu,
            EwKind::Gelu,
            EwKind::Exp,
            EwKind::RecipSqrt,
            EwKind::Relu,
            EwKind::Recip,
            EwKind::Add,
            EwKind::Mul,
            EwKind::Fma,
        ] {
            let src = elementwise_wgsl(kind, 64).expect("kind should have a kernel");
            let report = validate_wgsl(&src).unwrap_or_else(|e| panic!("{kind:?}: {e}"));
            assert!(report.entry_points.iter().any(|e| e == EWISE_ENTRY));
        }
        // Scale/Bias have no elementwise kernel here (affine covers them).
        assert!(elementwise_wgsl(EwKind::Scale, 64).is_err());
        assert!(elementwise_wgsl(EwKind::Bias, 64).is_err());
    }

    #[test]
    #[serial_test::serial(gpu)]
    fn elementwise_gpu_matches_oracle() {
        if !crate::wgsl_forge::test_gpu_available() {
            return;
        }
        let n = 1000usize;
        // RecipSqrt needs positive inputs; keep the unary domain safe.
        let pos: Vec<f32> = (0..n).map(|i| 0.1 + (i as f32) * 0.01).collect();
        let signed: Vec<f32> = (0..n).map(|i| (i as f32) * 0.02 - 10.0).collect();
        for kind in [EwKind::Silu, EwKind::Gelu, EwKind::Exp, EwKind::Relu] {
            let gpu = unary_gpu(&signed, kind).expect("unary_gpu");
            let cpu = unary_cpu(&signed, kind);
            for (g, c) in gpu.iter().zip(cpu.iter()) {
                assert!(
                    (g - c).abs() <= 1e-3 * c.abs().max(1.0),
                    "{kind:?}: {g} vs {c}"
                );
            }
        }
        for kind in [EwKind::RecipSqrt, EwKind::Recip] {
            let gpu = unary_gpu(&pos, kind).expect("unary_gpu");
            let cpu = unary_cpu(&pos, kind);
            for (g, c) in gpu.iter().zip(cpu.iter()) {
                assert!(
                    (g - c).abs() <= 1e-3 * c.abs().max(1.0),
                    "{kind:?}: {g} vs {c}"
                );
            }
        }
        let a: Vec<f32> = signed.clone();
        let b: Vec<f32> = pos.clone();
        for kind in [EwKind::Add, EwKind::Mul] {
            let gpu = binary_gpu(&a, &b, kind).expect("binary_gpu");
            let cpu = binary_cpu(&a, &b, kind);
            for (g, c) in gpu.iter().zip(cpu.iter()) {
                assert!(
                    (g - c).abs() <= 1e-3 * c.abs().max(1.0),
                    "{kind:?}: {g} vs {c}"
                );
            }
        }
    }
}
