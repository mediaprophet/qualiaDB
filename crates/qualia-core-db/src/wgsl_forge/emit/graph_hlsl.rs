//! HLSL lowering of the **portable** compute-graph nodes (plan §4 / P7) — the fourth backend
//! the one `lower_graph` driver walks. [`HlslLowerer`] implements the same
//! [`Lowerer`](crate::wgsl_forge::ir::graph::Lowerer) trait, emitting HLSL compute shaders for
//! the portable native kit (`Elementwise`/`Reduce`/`Broadcast`) with the same binding ABI +
//! math as the WGSL `graph_ops` kernels.
//!
//! Validation: the emitted HLSL is compiled to SPIR-V by **DXC** ([`compile_hlsl_to_spirv`],
//! behind the `dxc` feature + a `dxc` CLI) — a real toolchain check; structural checks run
//! unconditionally. Non-portable / not-yet-built op-classes inherit the trait's explicit `Err`.

use std::fmt::Write;

use super::GeneratedShader;
use crate::wgsl_forge::ir::graph::{ComputeGraph, EwKind, GraphNode, Lowerer, OpNode, RedKind};
use crate::wgsl_forge::{ForgeError, Schedule, FORGE_SCHEMA_VERSION};

/// Entry point name for graph-lowered GEMV kernels.
const GEMV_ENTRY: &str = "gemv_main";
/// Entry point name for graph-lowered GEMM kernels.
const GEMM_ENTRY: &str = "gemm_main";

fn unary_expr_hlsl(kind: EwKind) -> Option<&'static str> {
    Some(match kind {
        EwKind::Silu => "v / (1.0f + exp(-v))",
        EwKind::Gelu => {
            "0.5f * v * (1.0f + tanh(0.7978845608028654f * (v + 0.044715f * v * v * v)))"
        }
        EwKind::Exp => "exp(v)",
        EwKind::RecipSqrt => "rsqrt(v)",
        EwKind::Relu => "max(v, 0.0f)",
        EwKind::Recip => "1.0f / v",
        _ => return None,
    })
}

fn binary_expr_hlsl(kind: EwKind) -> Option<&'static str> {
    Some(match kind {
        EwKind::Add => "a + b",
        EwKind::Sub => "a - b",
        EwKind::Mul => "a * b",
        EwKind::Div => "a / b",
        _ => return None,
    })
}

/// Emit the HLSL compute shader for an elementwise `kind` at workgroup size `wg`. Entry
/// `ewise_main`; `StructuredBuffer`/`RWStructuredBuffer` binding registers mirror the WGSL
/// binding indices (t/u + `params` as `StructuredBuffer<uint>`).
pub fn elementwise_hlsl(kind: EwKind, wg: u32) -> Result<String, ForgeError> {
    if let Some(expr) = unary_expr_hlsl(kind) {
        return Ok(format!(
            "StructuredBuffer<float> input : register(t0);\nRWStructuredBuffer<float> output : register(u1);\nStructuredBuffer<uint> params : register(t2);\n[numthreads({wg}, 1, 1)]\nvoid ewise_main(uint3 gid : SV_DispatchThreadID) {{\n    uint i = gid.x;\n    if (i >= params[0]) return;\n    float v = input[i];\n    output[i] = {expr};\n}}\n"
        ));
    }
    if let Some(expr) = binary_expr_hlsl(kind) {
        return Ok(format!(
            "StructuredBuffer<float> lhs : register(t0);\nStructuredBuffer<float> rhs : register(t1);\nRWStructuredBuffer<float> output : register(u2);\nStructuredBuffer<uint> params : register(t3);\n[numthreads({wg}, 1, 1)]\nvoid ewise_main(uint3 gid : SV_DispatchThreadID) {{\n    uint i = gid.x;\n    if (i >= params[0]) return;\n    float a = lhs[i];\n    float b = rhs[i];\n    output[i] = {expr};\n}}\n"
        ));
    }
    if matches!(kind, EwKind::Fma) {
        return Ok(format!(
            "StructuredBuffer<float> a_in : register(t0);\nStructuredBuffer<float> b_in : register(t1);\nStructuredBuffer<float> c_in : register(t2);\nRWStructuredBuffer<float> output : register(u3);\nStructuredBuffer<uint> params : register(t4);\n[numthreads({wg}, 1, 1)]\nvoid ewise_main(uint3 gid : SV_DispatchThreadID) {{\n    uint i = gid.x;\n    if (i >= params[0]) return;\n    output[i] = mad(a_in[i], b_in[i], c_in[i]);\n}}\n"
        ));
    }
    Err(ForgeError::Emission(format!(
        "elementwise_hlsl: kind {kind:?} has no kernel (Scale/Bias use the affine kernel)"
    )))
}

fn reduce_fragments_hlsl(op: RedKind) -> (&'static str, &'static str, &'static str, &'static str) {
    match op {
        RedKind::Sum => (
            "0.0f",
            "acc + x",
            "scratch[tid] + scratch[tid + stride]",
            "scratch[0]",
        ),
        RedKind::Mean => (
            "0.0f",
            "acc + x",
            "scratch[tid] + scratch[tid + stride]",
            "scratch[0] / (float)n",
        ),
        RedKind::L2 => (
            "0.0f",
            "acc + x * x",
            "scratch[tid] + scratch[tid + stride]",
            "sqrt(scratch[0])",
        ),
        RedKind::Max => (
            "asfloat(0xff7fffff)",
            "max(acc, x)",
            "max(scratch[tid], scratch[tid + stride])",
            "scratch[0]",
        ),
    }
}

/// Emit the HLSL single-group tree-reduction kernel for `op` at workgroup size `wg`
/// (`groupshared` + `GroupMemoryBarrierWithGroupSync`).
pub fn reduce_hlsl(op: RedKind, wg: u32) -> String {
    let (init, fold, pair, finalize) = reduce_fragments_hlsl(op);
    format!(
        "StructuredBuffer<float> input : register(t0);\nRWStructuredBuffer<float> output : register(u1);\nStructuredBuffer<uint> params : register(t2);\ngroupshared float scratch[{wg}];\n[numthreads({wg}, 1, 1)]\nvoid reduce_main(uint tid : SV_GroupIndex) {{\n    const uint WG = {wg}u;\n    uint n = params[0];\n    float acc = {init};\n    for (uint i = tid; i < n; i += WG) {{ float x = input[i]; acc = {fold}; }}\n    scratch[tid] = acc;\n    GroupMemoryBarrierWithGroupSync();\n    for (uint stride = WG / 2u; stride > 0u; stride /= 2u) {{\n        if (tid < stride) {{ scratch[tid] = {pair}; }}\n        GroupMemoryBarrierWithGroupSync();\n    }}\n    if (tid == 0u) {{ output[0] = {finalize}; }}\n}}\n"
    )
}

/// Emit the HLSL broadcast kernel (`out[i] = input[i % in_len]`).
pub fn broadcast_hlsl(wg: u32) -> String {
    format!(
        "StructuredBuffer<float> input : register(t0);\nRWStructuredBuffer<float> output : register(u1);\nStructuredBuffer<uint> params : register(t2);\n[numthreads({wg}, 1, 1)]\nvoid broadcast_main(uint3 gid : SV_DispatchThreadID) {{\n    uint i = gid.x;\n    uint out_len = params[1];\n    if (i >= out_len) return;\n    output[i] = input[i % params[0]];\n}}\n"
    )
}

/// The HLSL [`Lowerer`]: appends one HLSL kernel per portable node to `source`.
pub struct HlslLowerer<'a> {
    pub source: &'a mut String,
}

impl Lowerer for HlslLowerer<'_> {
    fn elementwise(&mut self, node: &GraphNode) -> Result<(), ForgeError> {
        if let OpNode::Elementwise { f } = node.op {
            self.source
                .push_str(&elementwise_hlsl(f, node.sched.workgroup_size.max(1))?);
            Ok(())
        } else {
            Err(ForgeError::Emission(
                "HlslLowerer::elementwise on non-Elementwise".into(),
            ))
        }
    }
    fn reduce(&mut self, node: &GraphNode) -> Result<(), ForgeError> {
        if let OpNode::Reduce { op, .. } = node.op {
            self.source
                .push_str(&reduce_hlsl(op, node.sched.workgroup_size.max(1)));
            Ok(())
        } else {
            Err(ForgeError::Emission(
                "HlslLowerer::reduce on non-Reduce".into(),
            ))
        }
    }
    fn broadcast(&mut self, node: &GraphNode) -> Result<(), ForgeError> {
        self.source
            .push_str(&broadcast_hlsl(node.sched.workgroup_size.max(1)));
        Ok(())
    }
    fn gemv(&mut self, node: &GraphNode) -> Result<(), ForgeError> {
        if let OpNode::Gemv { .. } = node.op {
            let wg = node.sched.workgroup_size.max(1);
            let use_wave = wg % 32 == 0 && wg >= 32;
            if use_wave {
                emit_gemv_wave_graph(self.source, wg, GEMV_ENTRY)?;
            } else {
                emit_gemv_scalar_graph(self.source, wg, GEMV_ENTRY)?;
            }
            Ok(())
        } else {
            Err(ForgeError::Emission("HlslLowerer::gemv on non-Gemv".into()))
        }
    }
    fn matmul(&mut self, node: &GraphNode) -> Result<(), ForgeError> {
        if let OpNode::MatMul { .. } = node.op {
            emit_gemm_graph(self.source, node.sched.workgroup_size.max(1), GEMM_ENTRY)?;
            Ok(())
        } else {
            Err(ForgeError::Emission(
                "HlslLowerer::matmul on non-MatMul".into(),
            ))
        }
    }
}

/// Emit a complete HLSL module for a portable compute-graph (the HLSL analogue of
/// `emit_graph_wgsl`). Non-portable nodes lower to an explicit `Err`.
pub fn conv2d_hlsl(wg: u32) -> String {
    format!(
        r#"StructuredBuffer<float> input : register(t0);
StructuredBuffer<float> weight : register(t1);
StructuredBuffer<float> bias : register(t2);
RWStructuredBuffer<float> output : register(u3);
StructuredBuffer<uint> params : register(t4);

[numthreads({wg}, 1, 1)]
void conv2d_main(uint3 gid : SV_DispatchThreadID) {{
    uint idx = gid.x;
    uint c_in = params[0];
    uint c_out = params[1];
    uint h = params[2];
    uint w = params[3];
    uint kh = params[4];
    uint kw = params[5];
    uint sh = params[6];
    uint sw = params[7];
    uint ph = params[8];
    uint pw = params[9];
    uint ho = params[10];
    uint wo = params[11];
    uint n_out = c_out * ho * wo;
    if (idx >= n_out) return;
    uint oc = idx / (ho * wo);
    uint rem = idx % (ho * wo);
    uint oh = rem / wo;
    uint ow = rem % wo;
    float acc = bias[oc];
    for (uint ic = 0; ic < c_in; ic++) {{
        for (uint ky = 0; ky < kh; ky++) {{
            for (uint kx = 0; kx < kw; kx++) {{
                uint ih_p = oh * sh + ky;
                uint iw_p = ow * sw + kx;
                if (ih_p < ph || iw_p < pw) continue;
                uint ih = ih_p - ph;
                uint iw = iw_p - pw;
                if (ih >= h || iw >= w) continue;
                float iv = input[ic * h * w + ih * w + iw];
                float wv = weight[oc * (c_in * kh * kw) + ic * (kh * kw) + ky * kw + kx];
                acc += iv * wv;
            }}
        }}
    }}
    output[idx] = acc;
}}
"#
    )
}

/// Wave-cooperative GEMV for graph lowering: one wave per output row,
/// `WaveActiveSum` reduces partial dot products across lanes.
fn emit_gemv_wave_graph(source: &mut String, wg: u32, entry: &str) -> Result<(), ForgeError> {
    writeln!(
        source,
        r#"struct GemvParams {{
    uint m;
    uint n;
    uint _pad0;
    uint _pad1;
}};

StructuredBuffer<float> a : register(t0, space0);
StructuredBuffer<float> x : register(t1, space0);
RWStructuredBuffer<float> y : register(u2, space0);
ConstantBuffer<GemvParams> params : register(b3, space0);

[numthreads({wg}, 1, 1)]
void {entry}(uint3 gid : SV_DispatchThreadID) {{
    uint wave_size = WaveGetLaneCount();
    uint row = gid.x / wave_size;
    if (row >= params.m) {{ return; }}
    uint lane = WaveGetLaneIndex();
    uint a_row = row * params.n;
    float partial = 0.0;
    for (uint j = lane; j < params.n; j += wave_size) {{
        partial += a[a_row + j] * x[j];
    }}
    float acc = WaveActiveSum(partial);
    if (lane == 0) {{
        y[row] = acc;
    }}
}}"#,
        wg = wg,
        entry = entry
    )
    .map_err(|e| ForgeError::Emission(e.to_string()))?;
    Ok(())
}

/// Scalar GEMV for graph lowering: one thread per output row.
fn emit_gemv_scalar_graph(source: &mut String, wg: u32, entry: &str) -> Result<(), ForgeError> {
    writeln!(
        source,
        r#"struct GemvParams {{
    uint m;
    uint n;
    uint _pad0;
    uint _pad1;
}};

StructuredBuffer<float> a : register(t0, space0);
StructuredBuffer<float> x : register(t1, space0);
RWStructuredBuffer<float> y : register(u2, space0);
ConstantBuffer<GemvParams> params : register(b3, space0);

[numthreads({wg}, 1, 1)]
void {entry}(uint3 gid : SV_DispatchThreadID) {{
    uint i = gid.x;
    if (i >= params.m) {{ return; }}
    float acc = 0.0;
    uint a_row = i * params.n;
    for (uint j = 0; j < params.n; j++) {{
        acc += a[a_row + j] * x[j];
    }}
    y[i] = acc;
}}"#,
        wg = wg,
        entry = entry
    )
    .map_err(|e| ForgeError::Emission(e.to_string()))?;
    Ok(())
}

/// Dense GEMM for graph lowering: one thread per output element.
fn emit_gemm_graph(source: &mut String, wg: u32, entry: &str) -> Result<(), ForgeError> {
    writeln!(
        source,
        r#"struct GemmParams {{
    uint m;
    uint n;
    uint k;
    uint _pad;
}};

StructuredBuffer<float> a : register(t0, space0);
StructuredBuffer<float> b : register(t1, space0);
RWStructuredBuffer<float> c : register(u2, space0);
ConstantBuffer<GemmParams> params : register(b3, space0);

[numthreads({wg}, 1, 1)]
void {entry}(uint3 gid : SV_DispatchThreadID) {{
    uint o = gid.x;
    if (o >= params.m * params.n) {{ return; }}
    uint row = o / params.n;
    uint col = o % params.n;
    float acc = 0.0;
    uint a_row = row * params.k;
    for (uint kk = 0; kk < params.k; kk++) {{
        acc += a[a_row + kk] * b[kk * params.n + col];
    }}
    c[o] = acc;
}}"#,
        wg = wg,
        entry = entry
    )
    .map_err(|e| ForgeError::Emission(e.to_string()))?;
    Ok(())
}

pub fn emit_graph_hlsl(
    graph: &ComputeGraph,
    schedule: Schedule,
) -> Result<GeneratedShader, ForgeError> {
    let mut source = String::with_capacity(1_024);
    writeln!(
        source,
        "// Qualia WGSL Forge schema {FORGE_SCHEMA_VERSION} (compute-graph -> HLSL)."
    )
    .map_err(|e| ForgeError::Emission(e.to_string()))?;
    let mut lowerer = HlslLowerer {
        source: &mut source,
    };
    crate::wgsl_forge::ir::graph::lower_graph(graph, &mut lowerer)?;
    let source_hash = blake3::hash(source.as_bytes()).to_hex().to_string();
    Ok(GeneratedShader {
        kernel_id: "graph".to_string(),
        semantic_hash: source_hash.clone(),
        source_hash,
        schedule,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hlsl_portable_kit_emits_hlsl_constructs() {
        assert!(elementwise_hlsl(EwKind::Silu, 64)
            .unwrap()
            .contains("void ewise_main"));
        assert!(elementwise_hlsl(EwKind::Add, 64).unwrap().contains("a + b"));
        assert!(elementwise_hlsl(EwKind::Fma, 64).unwrap().contains("mad("));
        assert!(elementwise_hlsl(EwKind::Bias, 64).is_err());
        for op in [RedKind::Sum, RedKind::Mean, RedKind::L2, RedKind::Max] {
            let s = reduce_hlsl(op, 256);
            assert!(s.contains("groupshared float scratch"));
            assert!(s.contains("GroupMemoryBarrierWithGroupSync()"));
        }
        assert!(reduce_hlsl(RedKind::Max, 64).contains("asfloat(0xff7fffff)"));
        assert!(broadcast_hlsl(64).contains("i % params[0]"));
        assert!(conv2d_hlsl(64).contains("void conv2d_main"));
    }

    #[test]
    fn emit_graph_hlsl_lowers_a_softmax_subgraph() {
        use crate::wgsl_forge::graph_ops::executor::softmax_graph;
        let g = softmax_graph(16).unwrap();
        let shader = emit_graph_hlsl(&g, Schedule::default()).expect("hlsl");
        assert!(shader.source.contains("reduce_main") && shader.source.contains("ewise_main"));
    }

    #[test]
    fn emit_graph_hlsl_lowers_gemv_with_wave_intrinsics() {
        use crate::wgsl_forge::ir::graph::{ComputeGraph, DType, OpNode, Shape, TensorRef};
        let mut g = ComputeGraph::new();
        let a = TensorRef::external(Shape::new(&[0, 0]), DType::F32);
        let x = TensorRef::external(Shape::new(&[0]), DType::F32);
        let out = g
            .push(
                OpNode::Gemv {
                    m: 0,
                    n: 0,
                    tc: false,
                },
                &[a, x],
                Shape::new(&[0]),
                DType::F32,
                Schedule {
                    workgroup_size: 64,
                    ..Default::default()
                },
            )
            .unwrap();
        g.mark_output(out);
        let shader = emit_graph_hlsl(
            &g,
            Schedule {
                workgroup_size: 64,
                ..Default::default()
            },
        )
        .expect("hlsl gemv");
        assert!(
            shader.source.contains("WaveActiveSum"),
            "wave GEMV should use wave intrinsics"
        );
        assert!(shader.source.contains("WaveGetLaneCount"));
        assert!(shader.source.contains("gemv_main"));
    }

    #[test]
    fn emit_graph_hlsl_lowers_gemv_scalar_when_wg_not_multiple_of_32() {
        use crate::wgsl_forge::ir::graph::{ComputeGraph, DType, OpNode, Shape, TensorRef};
        let mut g = ComputeGraph::new();
        let a = TensorRef::external(Shape::new(&[0, 0]), DType::F32);
        let x = TensorRef::external(Shape::new(&[0]), DType::F32);
        let out = g
            .push(
                OpNode::Gemv {
                    m: 0,
                    n: 0,
                    tc: false,
                },
                &[a, x],
                Shape::new(&[0]),
                DType::F32,
                Schedule {
                    workgroup_size: 16,
                    ..Default::default()
                },
            )
            .unwrap();
        g.mark_output(out);
        let shader = emit_graph_hlsl(
            &g,
            Schedule {
                workgroup_size: 16,
                ..Default::default()
            },
        )
        .expect("hlsl gemv scalar");
        assert!(
            !shader.source.contains("WaveActiveSum"),
            "scalar GEMV should not use wave intrinsics"
        );
        assert!(shader.source.contains("gemv_main"));
    }

    #[test]
    fn emit_graph_hlsl_lowers_matmul() {
        use crate::wgsl_forge::ir::graph::{ComputeGraph, DType, OpNode, Shape, TensorRef};
        let mut g = ComputeGraph::new();
        let a = TensorRef::external(Shape::new(&[0, 0]), DType::F32);
        let b = TensorRef::external(Shape::new(&[0, 0]), DType::F32);
        let out = g
            .push(
                OpNode::MatMul {
                    m: 0,
                    n: 0,
                    k: 0,
                    tc: false,
                    trans_b: false,
                },
                &[a, b],
                Shape::new(&[0, 0]),
                DType::F32,
                Schedule {
                    workgroup_size: 64,
                    ..Default::default()
                },
            )
            .unwrap();
        g.mark_output(out);
        let shader = emit_graph_hlsl(
            &g,
            Schedule {
                workgroup_size: 64,
                ..Default::default()
            },
        )
        .expect("hlsl gemm");
        assert!(shader.source.contains("gemm_main"));
        assert!(shader.source.contains("params.k"));
    }

    /// Real DXC toolchain validation (needs `--features dxc` + a `dxc` CLI on PATH /
    /// `QUALIA_DXC_CLI_PATH`): every portable-kit HLSL kernel compiles to SPIR-V.
    #[cfg(feature = "dxc")]
    #[test]
    fn hlsl_portable_kit_dxc_compiles() {
        use crate::wgsl_forge::emit::dxc::compile_hlsl_to_spirv;
        let dxc = std::env::var("QUALIA_DXC_CLI_PATH").unwrap_or_else(|_| "dxc".to_string());
        if std::process::Command::new(&dxc)
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let mut srcs = vec![
            (elementwise_hlsl(EwKind::Silu, 64).unwrap(), "ewise_main"),
            (elementwise_hlsl(EwKind::Add, 64).unwrap(), "ewise_main"),
            (elementwise_hlsl(EwKind::Fma, 64).unwrap(), "ewise_main"),
            (broadcast_hlsl(64), "broadcast_main"),
        ];
        for op in [RedKind::Sum, RedKind::Mean, RedKind::L2, RedKind::Max] {
            srcs.push((reduce_hlsl(op, 256), "reduce_main"));
        }
        for (src, entry) in srcs {
            let spirv = compile_hlsl_to_spirv(&src, entry)
                .unwrap_or_else(|e| panic!("DXC failed for {entry}: {e}"));
            assert!(!spirv.is_empty(), "{entry}: empty SPIR-V");
        }
    }
}
