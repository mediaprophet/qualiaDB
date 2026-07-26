//! Native `Stencil` op-node — local neighbourhood maps over a 1-D grid (plan §2 / P7). The
//! seed kind is the **Laplacian** `out[i] = in[i-1] − 2·in[i] + in[i+1]` with zero (Dirichlet)
//! boundaries — the discrete Poisson operator under fluid pressure-solve / diffusion, and the
//! `Reduce`-free half of a stencil sweep.
//!
//! `RopePair` is a **real rotary position embedding** (RoPE): each (a,b) pair is rotated by the
//! true per-position / per-dimension angle `θ_{m,j} = m · base^(−2j/head_dim)`, in both the
//! interleaved (GPT-J / GGUF `NORM`, pairs `(2j, 2j+1)`) and split (GPT-NeoX / HF `rotate_half` /
//! GGUF `NEOX`, pairs `(j, j+head_dim/2)`) conventions — see [`RopeConfig`], [`rope_cpu`],
//! [`rope_wgsl`]. It is genuine position-dependent RoPE, not a fixed structural rotation.
//!
//! The advection/divergence kinds need a **velocity field** as a second input (different arity)
//! plus a physical-model choice, so — like `fluid_dynamics` (rollout B2, a curation item) — they
//! return an explicit `Err` naming the missing field/model rather than guessing; never a no-op.
//!
//! Certified the forge way: exact CPU oracle, naga validation, A2000 GPU differential.

use crate::wgsl_forge::ir::graph::StencilKind;
use crate::wgsl_forge::ForgeError;

/// Entry-point name of the emitted stencil kernel.
pub const STENCIL_ENTRY: &str = "stencil_main";

/// Default RoPE frequency base (θ) — the Llama / GPT-NeoX value.
pub const ROPE_DEFAULT_THETA_BASE: f32 = 10000.0;

/// RoPE pairing convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RopeMode {
    /// GPT-J / original interleaved: rotate adjacent pairs `(2j, 2j+1)`. GGUF `ROPE_TYPE_NORM`.
    Interleaved,
    /// GPT-NeoX / HF `rotate_half`: rotate split pairs `(j, j + head_dim/2)`. GGUF `ROPE_TYPE_NEOX`.
    Neox,
}

impl RopeMode {
    #[inline]
    fn code(self) -> u32 {
        match self {
            RopeMode::Interleaved => 0,
            RopeMode::Neox => 1,
        }
    }
}

/// Parameters of a real rotary position embedding applied to a flat `[..,head_dim]` vector
/// (`input.len()` must be a multiple of `head_dim`, which must be even). Pairs form **within**
/// each head; the rotation angle for pair `j` at position `pos` is `pos · base^(−2j/head_dim)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RopeConfig {
    /// Per-head rotation width (even). Heads are contiguous `head_dim`-length spans of `input`.
    pub head_dim: u32,
    /// Absolute token position `m`.
    pub pos: u32,
    /// Pairing convention.
    pub mode: RopeMode,
    /// Frequency base; [`ROPE_DEFAULT_THETA_BASE`] (10000) is the Llama/NeoX default.
    pub theta_base: f32,
}

impl RopeConfig {
    /// A config at the default θ base (10000).
    pub fn new(head_dim: u32, pos: u32, mode: RopeMode) -> Self {
        Self {
            head_dim,
            pos,
            mode,
            theta_base: ROPE_DEFAULT_THETA_BASE,
        }
    }

    fn validate(&self, n: usize) -> Result<(), ForgeError> {
        let hd = self.head_dim as usize;
        if hd == 0 || hd % 2 != 0 || n % hd != 0 {
            return Err(ForgeError::Emission(format!(
                "rope: head_dim {} must be even and divide n {n}",
                self.head_dim
            )));
        }
        Ok(())
    }
}

/// The `[n, head_dim, pos, mode, theta_base_bits]` u32 params buffer the RoPE kernel reads
/// (binding 2). `theta_base` is carried as its IEEE-754 bit pattern and `bitcast` back in WGSL.
pub fn rope_params(n: u32, cfg: &RopeConfig) -> [u32; 5] {
    [
        n,
        cfg.head_dim,
        cfg.pos,
        cfg.mode.code(),
        cfg.theta_base.to_bits(),
    ]
}

/// Emit the WGSL module for stencil `kind` at workgroup size `wg`. One invocation per grid
/// point. Binding ABI: `input` (0, storage read), `output` (1, storage read_write), `params`
/// (2, storage read). For `Laplacian` params is `[n, halo, …]`; for `RopePair` it is the real
/// RoPE block `[n, head_dim, pos, mode, theta_base_bits]` ([`rope_params`]). Returns `Err` for
/// the kinds that need a velocity field / physical model (`Divergence`/`Advection`).
pub fn stencil_wgsl(kind: StencilKind, wg: u32) -> Result<String, ForgeError> {
    let body = match kind {
        // Discrete Laplacian, zero boundaries.
        StencilKind::Laplacian => {
            "    var lft = 0.0; if (i > 0u) { lft = input[i - 1u]; }\n\
             \x20   var rgt = 0.0; if (i + 1u < n) { rgt = input[i + 1u]; }\n\
             \x20   output[i] = lft - 2.0 * input[i] + rgt;"
        }
        // Real RoPE: rotate this lane's pair by θ = pos · base^(−2j/head_dim). params:
        // [n, head_dim, pos, mode(0=interleaved,1=neox), theta_base_bits].
        StencilKind::RopePair => ROPE_BODY,
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

/// WGSL body of the real RoPE kernel (shared by [`stencil_wgsl`] / [`rope_wgsl`]). Reads
/// `head_dim`/`pos`/`mode`/`theta_base` from `params[1..5]` and rotates lane `i`'s pair.
const ROPE_BODY: &str = "\
    let head_dim = params[1];\n\
    let pos = f32(params[2]);\n\
    let mode = params[3];\n\
    let base = bitcast<f32>(params[4]);\n\
    let half = head_dim / 2u;\n\
    let l = i % head_dim;\n\
    var j: u32; var partner: u32; var is_first: bool;\n\
    if (mode == 0u) {\n\
    \x20   j = l / 2u; partner = i ^ 1u; is_first = (l & 1u) == 0u;\n\
    } else {\n\
    \x20   if (l < half) { j = l; partner = i + half; is_first = true; }\n\
    \x20   else { j = l - half; partner = i - half; is_first = false; }\n\
    }\n\
    let freq = pow(base, -2.0 * f32(j) / f32(head_dim));\n\
    let theta = pos * freq;\n\
    let c = cos(theta); let s = sin(theta);\n\
    let xv = input[i];\n\
    var pv = 0.0; if (partner < n) { pv = input[partner]; }\n\
    if (is_first) { output[i] = xv * c - pv * s; }\n\
    else { output[i] = pv * s + xv * c; }";

/// Emit the standalone real-RoPE kernel (same binding ABI + params as `stencil_wgsl(RopePair)`).
pub fn rope_wgsl(wg: u32) -> Result<String, ForgeError> {
    stencil_wgsl(StencilKind::RopePair, wg)
}

/// Exact CPU oracle for [`stencil_wgsl`] — mirrors the kernel math in f32. `rope` supplies the
/// RoPE parameters; it is ignored for non-`RopePair` kinds.
pub fn stencil_cpu(
    input: &[f32],
    kind: StencilKind,
    rope: RopeConfig,
) -> Result<Vec<f32>, ForgeError> {
    let n = input.len();
    match kind {
        StencilKind::Laplacian => Ok((0..n)
            .map(|i| {
                let lft = if i > 0 { input[i - 1] } else { 0.0 };
                let rgt = if i + 1 < n { input[i + 1] } else { 0.0 };
                lft - 2.0 * input[i] + rgt
            })
            .collect()),
        StencilKind::RopePair => rope_cpu(input, &rope),
        StencilKind::Divergence | StencilKind::Advection => Err(ForgeError::Emission(format!(
            "stencil_cpu {kind:?} needs a velocity field + model direction"
        ))),
    }
}

/// Exact CPU oracle for the real RoPE kernel: rotates each pair by `pos · base^(−2j/head_dim)`
/// in the configured convention. Mirrors [`ROPE_BODY`] in f32.
pub fn rope_cpu(input: &[f32], cfg: &RopeConfig) -> Result<Vec<f32>, ForgeError> {
    let n = input.len();
    cfg.validate(n)?;
    let hd = cfg.head_dim as usize;
    let half = hd / 2;
    let pos = cfg.pos as f32;
    let base = cfg.theta_base;
    let mut out = vec![0.0f32; n];
    for i in 0..n {
        let l = i % hd;
        let (j, partner, is_first) = match cfg.mode {
            RopeMode::Interleaved => (l / 2, i ^ 1, l % 2 == 0),
            RopeMode::Neox => {
                if l < half {
                    (l, i + half, true)
                } else {
                    (l - half, i - half, false)
                }
            }
        };
        let freq = base.powf(-2.0 * j as f32 / hd as f32);
        let theta = pos * freq;
        let (c, s) = (theta.cos(), theta.sin());
        let xv = input[i];
        let pv = if partner < n { input[partner] } else { 0.0 };
        out[i] = if is_first {
            xv * c - pv * s
        } else {
            pv * s + xv * c
        };
    }
    Ok(out)
}

/// Run a stencil on the GPU (standalone) and read back the `n`-element result. `rope` supplies
/// the RoPE parameters for `RopePair`; ignored for `Laplacian`.
pub fn stencil_gpu(
    input: &[f32],
    kind: StencilKind,
    rope: RopeConfig,
) -> Result<Vec<f32>, ForgeError> {
    use crate::wgsl_forge::execute::{
        BindingUsage, QualiaCompute, WgpuComputeContext, WgpuPipeline,
    };
    use crate::wgsl_forge::Schedule;

    let n = input.len();
    if n == 0 {
        return Err(ForgeError::GpuValidation(
            "stencil_gpu: empty input".to_string(),
        ));
    }
    if kind == StencilKind::RopePair {
        rope.validate(n)?;
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
    // RopePair reads the full [n, head_dim, pos, mode, theta_bits] block; Laplacian reads only
    // params[0] = n (the rest are harmless padding).
    let params: [u32; 5] = match kind {
        StencilKind::RopePair => rope_params(n as u32, &rope),
        _ => [n as u32, 0, 0, 0, 0],
    };
    let view_params = ctx.allocate_and_write(
        bytemuck::cast_slice(&params),
        2,
        0,
        BindingUsage::StorageRead,
    )?;
    let pipeline = WgpuPipeline::compile(&ctx, &src, STENCIL_ENTRY)?;
    let schedule = Schedule {
        workgroup_size: wg,
        ..Default::default()
    };
    pipeline.dispatch(&[view_in, view_out, view_params], &schedule, n)?;
    let mut out = ctx.read_buffer_f32(&view_out)?;
    out.truncate(n);
    Ok(out)
}

/// Run real RoPE on the GPU (standalone): [`stencil_gpu`] specialised to `RopePair`.
pub fn rope_gpu(input: &[f32], cfg: &RopeConfig) -> Result<Vec<f32>, ForgeError> {
    stencil_gpu(input, StencilKind::RopePair, *cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::validate::validate_wgsl;

    /// Dot product (the quantity an attention score is built from).
    fn dot(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b).map(|(x, y)| x * y).sum()
    }

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
        let cfg = RopeConfig::new(2, 0, RopeMode::Interleaved);
        let out = stencil_cpu(&[1.0, 2.0, 3.0, 4.0], StencilKind::Laplacian, cfg).unwrap();
        assert_eq!(out, vec![0.0, 0.0, 0.0, -5.0]);
    }

    #[test]
    fn rope_single_pair_hand_checked() {
        // head_dim=2, pos=1, base=10000 → j=0, freq=base^0=1, θ=1 rad. input [1,0]:
        // out0 = 1·cos1 − 0·sin1 = cos1; out1 = 1·sin1 + 0·cos1 = sin1.
        let cfg = RopeConfig::new(2, 1, RopeMode::Interleaved);
        let out = rope_cpu(&[1.0, 0.0], &cfg).unwrap();
        assert!((out[0] - 1.0f32.cos()).abs() < 1e-6, "{}", out[0]);
        assert!((out[1] - 1.0f32.sin()).abs() < 1e-6, "{}", out[1]);
    }

    #[test]
    fn rope_pos_zero_is_identity() {
        // At position 0 every angle is 0 → rotation is the identity, both conventions.
        let inp: Vec<f32> = (0..16).map(|i| (i as f32) * 0.3 - 2.0).collect();
        for mode in [RopeMode::Interleaved, RopeMode::Neox] {
            let out = rope_cpu(&inp, &RopeConfig::new(8, 0, mode)).unwrap();
            for (a, b) in out.iter().zip(&inp) {
                assert!(
                    (a - b).abs() < 1e-6,
                    "pos-0 RoPE must be identity ({mode:?})"
                );
            }
        }
    }

    #[test]
    fn rope_preserves_per_pair_norm() {
        // A rotation preserves each pair's L2 norm, for any position/convention.
        let inp: Vec<f32> = (0..8).map(|i| ((i * 7 % 5) as f32) * 0.4 - 0.6).collect();
        for mode in [RopeMode::Interleaved, RopeMode::Neox] {
            let cfg = RopeConfig::new(4, 3, mode);
            let out = rope_cpu(&inp, &cfg).unwrap();
            let half = 2usize;
            for head in 0..2 {
                for j in 0..half {
                    let (ai, bi) = match mode {
                        RopeMode::Interleaved => (head * 4 + 2 * j, head * 4 + 2 * j + 1),
                        RopeMode::Neox => (head * 4 + j, head * 4 + j + half),
                    };
                    let n_in = inp[ai] * inp[ai] + inp[bi] * inp[bi];
                    let n_out = out[ai] * out[ai] + out[bi] * out[bi];
                    assert!(
                        (n_in - n_out).abs() < 1e-4,
                        "{mode:?} pair norm: {n_in} vs {n_out}"
                    );
                }
            }
        }
    }

    #[test]
    fn rope_relative_position_invariance() {
        // RoPE's defining property: the attention score dot(RoPE(q,m), RoPE(k,n)) depends only on
        // the RELATIVE offset (m−n). So shifting both positions by the same Δ leaves it unchanged.
        let head_dim = 8u32;
        let q: Vec<f32> = (0..head_dim)
            .map(|i| ((i * 3 % 7) as f32) * 0.2 - 0.5)
            .collect();
        let k: Vec<f32> = (0..head_dim)
            .map(|i| ((i * 5 % 11) as f32) * 0.15 - 0.4)
            .collect();
        for mode in [RopeMode::Interleaved, RopeMode::Neox] {
            let (m, n) = (3u32, 7u32);
            let base = dot(
                &rope_cpu(&q, &RopeConfig::new(head_dim, m, mode)).unwrap(),
                &rope_cpu(&k, &RopeConfig::new(head_dim, n, mode)).unwrap(),
            );
            for delta in [1u32, 4, 9] {
                let shifted = dot(
                    &rope_cpu(&q, &RopeConfig::new(head_dim, m + delta, mode)).unwrap(),
                    &rope_cpu(&k, &RopeConfig::new(head_dim, n + delta, mode)).unwrap(),
                );
                assert!(
                    (base - shifted).abs() <= 1e-4 * base.abs().max(1.0),
                    "{mode:?} RoPE not relative-position invariant: {base} vs {shifted} (Δ={delta})"
                );
            }
        }
    }

    #[test]
    fn rope_rejects_bad_head_dim() {
        assert!(rope_cpu(&[1.0; 6], &RopeConfig::new(3, 1, RopeMode::Interleaved)).is_err());
        assert!(rope_cpu(&[1.0; 6], &RopeConfig::new(4, 1, RopeMode::Interleaved)).is_err());
    }

    #[test]
    #[serial_test::serial(gpu)]
    fn stencil_gpu_matches_oracle() {
        if !crate::wgsl_forge::test_gpu_available() {
            return;
        }
        // Laplacian.
        let inp: Vec<f32> = (0..512)
            .map(|i| ((i * 7 % 31) as f32) * 0.1 - 1.5)
            .collect();
        let lap_cfg = RopeConfig::new(2, 0, RopeMode::Interleaved);
        let gpu = stencil_gpu(&inp, StencilKind::Laplacian, lap_cfg).expect("gpu laplacian");
        let cpu = stencil_cpu(&inp, StencilKind::Laplacian, lap_cfg).unwrap();
        for (g, c) in gpu.iter().zip(&cpu) {
            assert!(
                (g - c).abs() <= 1e-4 * c.abs().max(1.0),
                "laplacian: {g} vs {c}"
            );
        }
        // Real RoPE, both conventions, a non-trivial position and multi-head layout.
        let rope_in: Vec<f32> = (0..512)
            .map(|i| ((i * 11 % 23) as f32) * 0.08 - 0.9)
            .collect();
        for mode in [RopeMode::Interleaved, RopeMode::Neox] {
            let cfg = RopeConfig::new(64, 5, mode);
            let gpu = rope_gpu(&rope_in, &cfg).expect("gpu rope");
            let cpu = rope_cpu(&rope_in, &cfg).unwrap();
            for (g, c) in gpu.iter().zip(&cpu) {
                assert!(
                    (g - c).abs() <= 1e-4 * c.abs().max(1.0),
                    "rope {mode:?}: {g} vs {c}"
                );
            }
        }
    }
}
