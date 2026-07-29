//! MSL (Metal) lowering of the **portable** compute-graph nodes (plan §4 / P7) — the third
//! backend the one `lower_graph` driver walks, after WGSL and CUDA-C. [`MslLowerer`] implements
//! the same [`Lowerer`](crate::wgsl_forge::ir::graph::Lowerer) trait, emitting Metal for the
//! portable native kit (`Elementwise`/`Reduce`/`Broadcast`) — same binding ABI + math as the
//! WGSL `graph_ops` kernels, so a portable graph lowers to Metal with no per-id branch.
//!
//! Non-portable / not-yet-built op-classes (`MatMul`/`Gemv`/`Fft`/`GatherDequant`/`Softmax`
//! sugar/`Stencil`/`ScatterAccum`/`Neighbor`) inherit the trait's explicit `Err`. `Neighbor`
//! is **deliberately** WGSL/CUDA-only — Metal ray tracing is a distinct API (the WGSL emitter
//! already documents this for `ray-probe`).
//!
//! Validation: there is no Metal toolchain on the build host (Windows/NVIDIA), so these are
//! checked structurally (entry point + expected Metal constructs); the SPIR-V/WGSL paths carry
//! the numeric certification.

use std::fmt::Write;

use super::GeneratedShader;
use crate::wgsl_forge::ir::graph::{ComputeGraph, EwKind, GraphNode, Lowerer, OpNode, RedKind};
use crate::wgsl_forge::{ForgeError, Schedule, FORGE_SCHEMA_VERSION};

/// Entry point name for graph-lowered GEMV kernels in MSL.
const GEMV_ENTRY: &str = "gemv_main";
/// Entry point name for graph-lowered GEMM kernels in MSL.
const GEMM_ENTRY: &str = "gemm_main";

/// MSL expression for a unary [`EwKind`] (`f(v)`), or `None` if not unary.
fn unary_expr_msl(kind: EwKind) -> Option<&'static str> {
    Some(match kind {
        EwKind::Silu => "v / (1.0f + exp(-v))",
        EwKind::Gelu => {
            "0.5f * v * (1.0f + tanh(0.7978845608028654f * (v + 0.044715f * v * v * v)))"
        }
        EwKind::Exp => "exp(v)",
        EwKind::RecipSqrt => "rsqrt(v)",
        EwKind::Relu => "fmax(v, 0.0f)",
        EwKind::Recip => "1.0f / v",
        _ => return None,
    })
}

fn binary_expr_msl(kind: EwKind) -> Option<&'static str> {
    Some(match kind {
        EwKind::Add => "a + b",
        EwKind::Sub => "a - b",
        EwKind::Mul => "a * b",
        EwKind::Div => "a / b",
        _ => return None,
    })
}

/// Emit the Metal kernel for an elementwise `kind` (unary / binary / fma). Same binding order
/// as the WGSL kernel; entry `ewise_main`.
pub fn elementwise_msl(kind: EwKind) -> Result<String, ForgeError> {
    let head = "#include <metal_stdlib>\nusing namespace metal;\n";
    if let Some(expr) = unary_expr_msl(kind) {
        return Ok(format!(
            "{head}kernel void ewise_main(device const float* input [[buffer(0)]], device float* output [[buffer(1)]], device const uint* params [[buffer(2)]], uint3 gid [[thread_position_in_grid]]) {{\n    uint i = gid.x;\n    if (i >= params[0]) return;\n    float v = input[i];\n    output[i] = {expr};\n}}\n"
        ));
    }
    if let Some(expr) = binary_expr_msl(kind) {
        return Ok(format!(
            "{head}kernel void ewise_main(device const float* lhs [[buffer(0)]], device const float* rhs [[buffer(1)]], device float* output [[buffer(2)]], device const uint* params [[buffer(3)]], uint3 gid [[thread_position_in_grid]]) {{\n    uint i = gid.x;\n    if (i >= params[0]) return;\n    float a = lhs[i];\n    float b = rhs[i];\n    output[i] = {expr};\n}}\n"
        ));
    }
    if matches!(kind, EwKind::Fma) {
        return Ok(format!(
            "{head}kernel void ewise_main(device const float* a_in [[buffer(0)]], device const float* b_in [[buffer(1)]], device const float* c_in [[buffer(2)]], device float* output [[buffer(3)]], device const uint* params [[buffer(4)]], uint3 gid [[thread_position_in_grid]]) {{\n    uint i = gid.x;\n    if (i >= params[0]) return;\n    output[i] = fma(a_in[i], b_in[i], c_in[i]);\n}}\n"
        ));
    }
    Err(ForgeError::Emission(format!(
        "elementwise_msl: kind {kind:?} has no kernel (Scale/Bias use the affine kernel)"
    )))
}

fn reduce_fragments_msl(op: RedKind) -> (&'static str, &'static str, &'static str, &'static str) {
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
            "as_type<float>(0xff7fffffu)",
            "fmax(acc, x)",
            "fmax(scratch[tid], scratch[tid + stride])",
            "scratch[0]",
        ),
    }
}

/// Emit the Metal single-threadgroup tree-reduction kernel for `op` at workgroup size `wg`.
pub fn reduce_msl(op: RedKind, wg: u32) -> String {
    let (init, fold, pair, finalize) = reduce_fragments_msl(op);
    format!(
        "#include <metal_stdlib>\nusing namespace metal;\nkernel void reduce_main(device const float* input [[buffer(0)]], device float* output [[buffer(1)]], device const uint* params [[buffer(2)]], uint tid [[thread_position_in_threadgroup]]) {{\n    const uint WG = {wg}u;\n    threadgroup float scratch[{wg}];\n    uint n = params[0];\n    float acc = {init};\n    for (uint i = tid; i < n; i += WG) {{ float x = input[i]; acc = {fold}; }}\n    scratch[tid] = acc;\n    threadgroup_barrier(mem_flags::mem_threadgroup);\n    for (uint stride = WG / 2u; stride > 0u; stride /= 2u) {{\n        if (tid < stride) {{ scratch[tid] = {pair}; }}\n        threadgroup_barrier(mem_flags::mem_threadgroup);\n    }}\n    if (tid == 0u) {{ output[0] = {finalize}; }}\n}}\n"
    )
}

/// Emit the Metal broadcast kernel (`out[i] = input[i % in_len]`).
pub fn broadcast_msl() -> String {
    "#include <metal_stdlib>\nusing namespace metal;\nkernel void broadcast_main(device const float* input [[buffer(0)]], device float* output [[buffer(1)]], device const uint* params [[buffer(2)]], uint3 gid [[thread_position_in_grid]]) {\n    uint i = gid.x;\n    uint out_len = params[1];\n    if (i >= out_len) return;\n    output[i] = input[i % params[0]];\n}\n".to_string()
}

/// The MSL [`Lowerer`]: appends one Metal kernel per portable node to `source`.
pub struct MslLowerer<'a> {
    pub source: &'a mut String,
}

impl Lowerer for MslLowerer<'_> {
    fn elementwise(&mut self, node: &GraphNode) -> Result<(), ForgeError> {
        if let OpNode::Elementwise { f } = node.op {
            self.source.push_str(&elementwise_msl(f)?);
            Ok(())
        } else {
            Err(ForgeError::Emission(
                "MslLowerer::elementwise on non-Elementwise".into(),
            ))
        }
    }
    fn reduce(&mut self, node: &GraphNode) -> Result<(), ForgeError> {
        if let OpNode::Reduce { op, .. } = node.op {
            self.source
                .push_str(&reduce_msl(op, node.sched.workgroup_size.max(1)));
            Ok(())
        } else {
            Err(ForgeError::Emission(
                "MslLowerer::reduce on non-Reduce".into(),
            ))
        }
    }
    fn broadcast(&mut self, _node: &GraphNode) -> Result<(), ForgeError> {
        self.source.push_str(&broadcast_msl());
        Ok(())
    }
    fn gemv(&mut self, node: &GraphNode) -> Result<(), ForgeError> {
        if let OpNode::Gemv { .. } = node.op {
            let wg = node.sched.workgroup_size.max(1);
            // Apple Silicon SIMD groups are 32 threads (same as NVIDIA warps).
            // Use simd_sum for cooperative reduction when workgroup is a multiple of 32.
            let use_simd = wg % 32 == 0 && wg >= 32;
            if use_simd {
                emit_gemv_simd_graph(self.source, wg, GEMV_ENTRY)?;
            } else {
                emit_gemv_scalar_graph(self.source, wg, GEMV_ENTRY)?;
            }
            Ok(())
        } else {
            Err(ForgeError::Emission("MslLowerer::gemv on non-Gemv".into()))
        }
    }
    fn matmul(&mut self, node: &GraphNode) -> Result<(), ForgeError> {
        if let OpNode::MatMul { .. } = node.op {
            emit_gemm_graph(self.source, node.sched.workgroup_size.max(1), GEMM_ENTRY)?;
            Ok(())
        } else {
            Err(ForgeError::Emission(
                "MslLowerer::matmul on non-MatMul".into(),
            ))
        }
    }
}

/// Emit a complete MSL module for a portable compute-graph (the MSL analogue of
/// `emit_graph_wgsl`). Non-portable nodes lower to an explicit `Err`.
pub fn conv2d_msl() -> String {
    format!(
        r#"#include <metal_stdlib>
using namespace metal;

kernel void conv2d_main(
    device const float* input [[buffer(0)]],
    device const float* weight [[buffer(1)]],
    device const float* bias [[buffer(2)]],
    device float* output [[buffer(3)]],
    device const uint* params [[buffer(4)]],
    uint3 gid [[thread_position_in_grid]]
) {{
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

/// SIMD-group cooperative GEMV for graph lowering: one SIMD group per output row,
/// `simd_sum` reduces partial dot products across lanes. Apple Silicon equivalent
/// of HLSL wave-intrinsic GEMV.
fn emit_gemv_simd_graph(source: &mut String, _wg: u32, entry: &str) -> Result<(), ForgeError> {
    writeln!(
        source,
        r#"#include <metal_stdlib>
using namespace metal;

struct GemvParams {{
    uint m;
    uint n;
    uint _pad0;
    uint _pad1;
}};

kernel void {entry}(
    device const float* a [[buffer(0)]],
    device const float* x [[buffer(1)]],
    device float* y [[buffer(2)]],
    constant GemvParams& params [[buffer(3)]],
    uint3 gid [[thread_position_in_grid]]
) {{
    uint simd_size = 32;
    uint row = gid.x / simd_size;
    if (row >= params.m) {{ return; }}
    uint lane = gid.x % simd_size;
    uint a_row = row * params.n;
    float partial = 0.0f;
    for (uint j = lane; j < params.n; j += simd_size) {{
        partial += a[a_row + j] * x[j];
    }}
    float acc = simd_sum(partial);
    if (lane == 0) {{
        y[row] = acc;
    }}
}}"#,
        entry = entry
    )
    .map_err(|e| ForgeError::Emission(e.to_string()))?;
    Ok(())
}

/// Scalar GEMV for graph lowering: one thread per output row.
fn emit_gemv_scalar_graph(source: &mut String, _wg: u32, entry: &str) -> Result<(), ForgeError> {
    writeln!(
        source,
        r#"#include <metal_stdlib>
using namespace metal;

struct GemvParams {{
    uint m;
    uint n;
    uint _pad0;
    uint _pad1;
}};

kernel void {entry}(
    device const float* a [[buffer(0)]],
    device const float* x [[buffer(1)]],
    device float* y [[buffer(2)]],
    constant GemvParams& params [[buffer(3)]],
    uint3 gid [[thread_position_in_grid]]
) {{
    uint i = gid.x;
    if (i >= params.m) {{ return; }}
    float acc = 0.0f;
    uint a_row = i * params.n;
    for (uint j = 0; j < params.n; j++) {{
        acc += a[a_row + j] * x[j];
    }}
    y[i] = acc;
}}"#,
        entry = entry
    )
    .map_err(|e| ForgeError::Emission(e.to_string()))?;
    Ok(())
}

/// Dense GEMM for graph lowering: one thread per output element.
fn emit_gemm_graph(source: &mut String, _wg: u32, entry: &str) -> Result<(), ForgeError> {
    writeln!(
        source,
        r#"#include <metal_stdlib>
using namespace metal;

struct GemmParams {{
    uint m;
    uint n;
    uint k;
    uint _pad;
}};

kernel void {entry}(
    device const float* a [[buffer(0)]],
    device const float* b [[buffer(1)]],
    device float* c [[buffer(2)]],
    constant GemmParams& params [[buffer(3)]],
    uint3 gid [[thread_position_in_grid]]
) {{
    uint o = gid.x;
    if (o >= params.m * params.n) {{ return; }}
    uint row = o / params.n;
    uint col = o % params.n;
    float acc = 0.0f;
    uint a_row = row * params.k;
    for (uint kk = 0; kk < params.k; kk++) {{
        acc += a[a_row + kk] * b[kk * params.n + col];
    }}
    c[o] = acc;
}}"#,
        entry = entry
    )
    .map_err(|e| ForgeError::Emission(e.to_string()))?;
    Ok(())
}

pub fn emit_graph_msl(
    graph: &ComputeGraph,
    schedule: Schedule,
) -> Result<GeneratedShader, ForgeError> {
    let mut source = String::with_capacity(1_024);
    writeln!(
        source,
        "// Qualia WGSL Forge schema {FORGE_SCHEMA_VERSION} (compute-graph → MSL)."
    )
    .map_err(|e| ForgeError::Emission(e.to_string()))?;
    let mut lowerer = MslLowerer {
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
    fn msl_portable_kit_emits_metal_constructs() {
        // Elementwise: a unary, a binary, fma.
        assert!(elementwise_msl(EwKind::Silu)
            .unwrap()
            .contains("kernel void ewise_main"));
        assert!(elementwise_msl(EwKind::Add).unwrap().contains("a + b"));
        assert!(elementwise_msl(EwKind::Fma).unwrap().contains("fma("));
        assert!(elementwise_msl(EwKind::Scale).is_err());
        // Reduce: each kind + threadgroup constructs.
        for op in [RedKind::Sum, RedKind::Mean, RedKind::L2, RedKind::Max] {
            let s = reduce_msl(op, 256);
            assert!(s.contains("threadgroup float scratch"));
            assert!(s.contains("threadgroup_barrier(mem_flags::mem_threadgroup)"));
        }
        assert!(reduce_msl(RedKind::Max, 64).contains("as_type<float>(0xff7fffffu)"));
        assert!(broadcast_msl().contains("i % params[0]"));
        assert!(conv2d_msl().contains("kernel void conv2d_main"));
    }

    #[test]
    fn emit_graph_msl_lowers_a_softmax_subgraph_node() {
        use crate::wgsl_forge::graph_ops::executor::softmax_graph;
        // softmax_graph is all portable nodes (Reduce/Broadcast/Elementwise) → lowers to MSL.
        let g = softmax_graph(16).unwrap();
        let shader = emit_graph_msl(&g, Schedule::default()).expect("msl");
        assert!(shader.source.contains("kernel void"));
        assert!(shader.source.contains("reduce_main") && shader.source.contains("ewise_main"));
    }

    #[test]
    fn emit_graph_msl_lowers_gemv_with_simd_group() {
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
        let shader = emit_graph_msl(
            &g,
            Schedule {
                workgroup_size: 64,
                ..Default::default()
            },
        )
        .expect("msl gemv");
        assert!(
            shader.source.contains("simd_sum"),
            "should use simd_sum for cooperative reduction"
        );
        assert!(
            shader.source.contains("gemv_main"),
            "should contain gemv entry point"
        );
    }

    #[test]
    fn emit_graph_msl_lowers_gemv_scalar_when_wg_not_multiple_of_32() {
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
        let shader = emit_graph_msl(
            &g,
            Schedule {
                workgroup_size: 16,
                ..Default::default()
            },
        )
        .expect("msl gemv scalar");
        assert!(
            !shader.source.contains("simd_sum"),
            "should NOT use simd_sum for small workgroup"
        );
        assert!(
            shader.source.contains("gemv_main"),
            "should contain gemv entry point"
        );
    }

    #[test]
    fn emit_graph_msl_lowers_matmul() {
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
        let shader = emit_graph_msl(
            &g,
            Schedule {
                workgroup_size: 64,
                ..Default::default()
            },
        )
        .expect("msl gemm");
        assert!(
            shader.source.contains("gemm_main"),
            "should contain gemm entry point"
        );
        assert!(
            shader.source.contains("params.k"),
            "should reference k dimension"
        );
    }
}
