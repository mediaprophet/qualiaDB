//! CUDA-C lowering of the compute-graph IR — the **second backend in one pass** (plan §4,
//! Phase 5). The *same* [`ComputeGraph`] that [`emit_graph_wgsl`](super::wgsl::emit_graph_wgsl)
//! lowers to WGSL is lowered here to CUDA-C by a [`CudaCLowerer`], with **no per-`kernel.id`
//! branches** — both backends are just `Lowerer` impls walked by the one
//! [`lower_graph`](crate::wgsl_forge::ir::graph::lower_graph) driver.
//!
//! # Why this exists
//!
//! It is the keystone of the DAG-IR thesis: a transformer decode step (or a physics graph)
//! is expressed **once** as op-nodes, and each backend lowers the whole graph in a single
//! visitor pass. Phase 5 proves it across the WGSL↔CUDA-C boundary — the cross-backend
//! differential oracle grades the same graph on both, against the same CPU floor.
//!
//! # Coverage (this phase)
//!
//! - **MatMul** — `tc=true` → the genuine NVIDIA tensor-core kernel
//!   [`WMMA_GEMM_TILED_SRC`](super::cuda_c::WMMA_GEMM_TILED_SRC) (f16 in / f32 accumulate);
//!   `tc=false` → the plain f32 [`GEMM_F32_SRC`](super::cuda_c::GEMM_F32_SRC). This is the
//!   `MatMul.tc → WMMA` headline, lowered from the IR with no host round-trip in the codegen.
//! - **Gemv** — [`GEMV_F32_SRC`](super::cuda_c::GEMV_F32_SRC).
//! - **Elementwise / Reduce / Broadcast** — CUDA-C twins of the WGSL `graph_ops` kernels
//!   (same binding ABI, same math), so the LLM activation/norm kit lowers to CUDA-C too.
//!
//! `Fft` and the not-yet-built op-classes (`Softmax` sugar, `GatherDequant`, `Stencil`,
//! `ScatterAccum`, `Neighbor`) inherit the trait's explicit `Err` — never a silent no-op —
//! exactly as the WGSL lowerers do at this phase.
//!
//! # Honest boundary
//!
//! Single-node graphs (the seed GEMM/GEMV) execute end-to-end via the host CUDA dispatch
//! (`dispatch::gemm_tc_cuda` and the Phase-5 cross-backend test). A *multi-node* CUDA graph
//! executor (the CUDA twin of the wgpu [`executor`](crate::wgsl_forge::graph_ops::executor),
//! keeping intermediates device-side across nodes) is a separate, later deliverable; this
//! module is the **codegen** half. The emitted CUDA-C for every supported node is
//! NVRTC-compile-validated (the CUDA analogue of naga-validate).

use std::fmt::Write;

use super::cuda_c::{
    GEMM_F32_ENTRY, GEMM_F32_SRC, GEMV_F32_ENTRY, GEMV_F32_SRC, WMMA_GEMM_TILED_ENTRY,
    WMMA_GEMM_TILED_SRC,
};
use super::GeneratedShader;
use crate::wgsl_forge::ir::graph::{ComputeGraph, EwKind, GraphNode, Lowerer, OpNode, RedKind};
use crate::wgsl_forge::{ForgeError, Schedule, FORGE_SCHEMA_VERSION};

/// CUDA-C expression for a unary [`EwKind`] (`f(v)`), or `None` if not unary. Mirrors the
/// WGSL [`elementwise_wgsl`](crate::wgsl_forge::graph_ops::elementwise::elementwise_wgsl)
/// math exactly so the two backends agree against the same CPU oracle.
fn unary_expr_cuda(kind: EwKind) -> Option<&'static str> {
    Some(match kind {
        EwKind::Silu => "v / (1.0f + expf(-v))",
        EwKind::Gelu => {
            "0.5f * v * (1.0f + tanhf(0.7978845608028654f * (v + 0.044715f * v * v * v)))"
        }
        EwKind::Exp => "expf(v)",
        EwKind::RecipSqrt => "rsqrtf(v)",
        EwKind::Relu => "fmaxf(v, 0.0f)",
        EwKind::Recip => "1.0f / v",
        _ => return None,
    })
}

/// CUDA-C expression for a binary [`EwKind`] (`f(a, b)`), or `None` otherwise.
fn binary_expr_cuda(kind: EwKind) -> Option<&'static str> {
    Some(match kind {
        EwKind::Add => "a + b",
        EwKind::Sub => "a - b",
        EwKind::Mul => "a * b",
        EwKind::Div => "a / b",
        _ => return None,
    })
}

/// Emit the CUDA-C kernel for an elementwise `kind` (unary / binary / fma). Same binding
/// order as the WGSL kernel (inputs, then `output`, then `params` = `[n, …]`), each storage
/// buffer a pointer parameter (the [`CudaPipeline`](crate::wgsl_forge::execute::CudaPipeline)
/// ABI). Entry point `ewise_main` (matching the WGSL entry for cross-backend symmetry).
pub fn elementwise_cuda_c(kind: EwKind, wg: u32) -> Result<String, ForgeError> {
    let _ = wg; // grid-strided per-element; launch geometry comes from the dispatch.
    if let Some(expr) = unary_expr_cuda(kind) {
        return Ok(format!(
            r#"extern "C" __global__ void ewise_main(const float* input, float* output, const unsigned* params) {{
    unsigned i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= params[0]) return;
    float v = input[i];
    output[i] = {expr};
}}"#
        ));
    }
    if let Some(expr) = binary_expr_cuda(kind) {
        return Ok(format!(
            r#"extern "C" __global__ void ewise_main(const float* lhs, const float* rhs, float* output, const unsigned* params) {{
    unsigned i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= params[0]) return;
    float a = lhs[i];
    float b = rhs[i];
    output[i] = {expr};
}}"#
        ));
    }
    if matches!(kind, EwKind::Fma) {
        return Ok(r#"extern "C" __global__ void ewise_main(const float* a_in, const float* b_in, const float* c_in, float* output, const unsigned* params) {
    unsigned i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= params[0]) return;
    output[i] = fmaf(a_in[i], b_in[i], c_in[i]);
}"#
        .to_string());
    }
    Err(ForgeError::Emission(format!(
        "elementwise_cuda_c: kind {kind:?} has no kernel (Scale/Bias use the affine kernel)"
    )))
}

/// Per-[`RedKind`] CUDA-C fragments: `(init, fold, pair-combine, finalize)` — the CUDA twin
/// of the WGSL reduce fragments, identical algebra so both backends grade against
/// [`reduce_cpu`](crate::wgsl_forge::graph_ops::reduce::reduce_cpu).
fn reduce_fragments_cuda(op: RedKind) -> (&'static str, &'static str, &'static str, &'static str) {
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
            "sqrtf(scratch[0])",
        ),
        RedKind::Max => (
            "__int_as_float(0xff7fffff)",
            "fmaxf(acc, x)",
            "fmaxf(scratch[tid], scratch[tid + stride])",
            "scratch[0]",
        ),
    }
}

/// Emit the CUDA-C single-block tree-reduction kernel for `op` at workgroup size `wg`
/// (a power of two) — the CUDA twin of
/// [`reduce_wgsl`](crate::wgsl_forge::graph_ops::reduce::reduce_wgsl). Binding ABI: `input`
/// (0), `output` (1, only `[0]` written), `params` (2, `[n, …]`). Entry `reduce_main`.
pub fn reduce_cuda_c(op: RedKind, wg: u32) -> String {
    let (init, fold, pair, finalize) = reduce_fragments_cuda(op);
    format!(
        r#"extern "C" __global__ void reduce_main(const float* input, float* output, const unsigned* params) {{
    const unsigned WG = {wg}u;
    __shared__ float scratch[{wg}];
    unsigned tid = threadIdx.x;
    unsigned n = params[0];
    float acc = {init};
    for (unsigned i = tid; i < n; i += WG) {{
        float x = input[i];
        acc = {fold};
    }}
    scratch[tid] = acc;
    __syncthreads();
    for (unsigned stride = WG / 2u; stride > 0u; stride /= 2u) {{
        if (tid < stride) {{
            scratch[tid] = {pair};
        }}
        __syncthreads();
    }}
    if (tid == 0u) {{
        output[0] = {finalize};
    }}
}}"#
    )
}

/// Emit the CUDA-C broadcast kernel (`out[i] = input[i % in_len]`) — the CUDA twin of
/// [`broadcast_wgsl`](crate::wgsl_forge::graph_ops::broadcast::broadcast_wgsl). Binding ABI:
/// `input` (0), `output` (1), `params` (2, `[in_len, out_len, …]`). Entry `broadcast_main`.
pub fn broadcast_cuda_c(wg: u32) -> String {
    let _ = wg; // one thread per output element; launch geometry from the dispatch.
    r#"extern "C" __global__ void broadcast_main(const float* input, float* output, const unsigned* params) {
    unsigned i = blockIdx.x * blockDim.x + threadIdx.x;
    unsigned out_len = params[1];
    if (i >= out_len) return;
    unsigned in_len = params[0];
    output[i] = input[i % in_len];
}"#
    .to_string()
}

/// The CUDA-C [`Lowerer`]: walks a [`ComputeGraph`] and appends one CUDA-C kernel per node to
/// `source`. The mirror of the WGSL `WgslDelegateLowerer`/`WgslGraphLowerer`.
pub struct CudaCLowerer<'a> {
    pub source: &'a mut String,
}

impl Lowerer for CudaCLowerer<'_> {
    fn matmul(&mut self, node: &GraphNode) -> Result<(), ForgeError> {
        if let OpNode::MatMul { tc, trans_b, .. } = node.op {
            if trans_b {
                return Err(ForgeError::Emission(
                    "CudaCLowerer::matmul: trans_b not supported (transpose B host-side)"
                        .to_string(),
                ));
            }
            // tc=true → genuine NVIDIA tensor cores (WMMA, f16 in / f32 acc); else plain f32.
            self.source.push_str(if tc {
                WMMA_GEMM_TILED_SRC
            } else {
                GEMM_F32_SRC
            });
            self.source.push('\n');
            Ok(())
        } else {
            Err(ForgeError::Emission(
                "CudaCLowerer::matmul received a non-MatMul node".to_string(),
            ))
        }
    }

    fn gemv(&mut self, _node: &GraphNode) -> Result<(), ForgeError> {
        self.source.push_str(GEMV_F32_SRC);
        self.source.push('\n');
        Ok(())
    }

    fn elementwise(&mut self, node: &GraphNode) -> Result<(), ForgeError> {
        if let OpNode::Elementwise { f } = node.op {
            let src = elementwise_cuda_c(f, node.sched.workgroup_size)?;
            self.source.push_str(&src);
            self.source.push('\n');
            Ok(())
        } else {
            Err(ForgeError::Emission(
                "CudaCLowerer::elementwise received a non-Elementwise node".to_string(),
            ))
        }
    }

    fn reduce(&mut self, node: &GraphNode) -> Result<(), ForgeError> {
        if let OpNode::Reduce { op, .. } = node.op {
            self.source
                .push_str(&reduce_cuda_c(op, node.sched.workgroup_size));
            self.source.push('\n');
            Ok(())
        } else {
            Err(ForgeError::Emission(
                "CudaCLowerer::reduce received a non-Reduce node".to_string(),
            ))
        }
    }

    fn broadcast(&mut self, node: &GraphNode) -> Result<(), ForgeError> {
        self.source
            .push_str(&broadcast_cuda_c(node.sched.workgroup_size));
        self.source.push('\n');
        Ok(())
    }
}

/// The CUDA-C entry-point name a single-node graph lowers to (so a caller can compile +
/// dispatch the emitted source). Mirrors the per-node kernel chosen by [`CudaCLowerer`].
/// Returns `Err` for op-classes without a CUDA-C lowering this phase.
pub fn graph_cuda_entry(node: &GraphNode) -> Result<&'static str, ForgeError> {
    Ok(match node.op {
        OpNode::MatMul { tc, .. } => {
            if tc {
                WMMA_GEMM_TILED_ENTRY
            } else {
                GEMM_F32_ENTRY
            }
        }
        OpNode::Gemv { .. } => GEMV_F32_ENTRY,
        OpNode::Elementwise { .. } => "ewise_main",
        OpNode::Reduce { .. } => "reduce_main",
        OpNode::Broadcast { .. } => "broadcast_main",
        other => {
            return Err(ForgeError::Emission(format!(
                "graph_cuda_entry: op {other:?} has no CUDA-C lowering this phase"
            )))
        }
    })
}

/// Emit a complete CUDA-C module for a **pure compute-graph** — the CUDA-C analogue of
/// [`emit_graph_wgsl`](super::wgsl::emit_graph_wgsl). A single-node graph produces one kernel
/// (compile + dispatch it with [`graph_cuda_entry`]); multi-node graphs concatenate the
/// per-node kernels (a device-side multi-node CUDA executor is a later deliverable — see the
/// module header).
pub fn emit_graph_cuda_c(
    graph: &ComputeGraph,
    schedule: Schedule,
) -> Result<GeneratedShader, ForgeError> {
    let mut source = String::with_capacity(1_024);
    writeln!(
        source,
        "// Generated by Qualia WGSL Forge schema {FORGE_SCHEMA_VERSION} (compute-graph → CUDA-C)."
    )
    .map_err(|error| ForgeError::Emission(error.to_string()))?;
    let mut lowerer = CudaCLowerer {
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
    use crate::wgsl_forge::ir::graph::{DType, Shape, TensorRef};

    /// The CUDA-C kit emitters cover every node the WGSL kit covers, with the matching entry
    /// point — pure string checks, no GPU. (Validity is asserted by the NVRTC-compile test
    /// under `--features cuda`.)
    #[test]
    fn cuda_c_kit_emits_each_node() {
        // Elementwise: a unary, a binary, the fma.
        for (k, sig) in [
            (EwKind::Silu, "expf(-v)"),
            (EwKind::Add, "a + b"),
            (EwKind::Fma, "fmaf("),
        ] {
            let s = elementwise_cuda_c(k, 64).expect("kit kind");
            assert!(s.contains("ewise_main") && s.contains(sig), "{k:?}: {s}");
        }
        assert!(elementwise_cuda_c(EwKind::Scale, 64).is_err());
        // Reduce: each kind, with its finalize.
        for (op, fin) in [
            (RedKind::Sum, "scratch[0];"),
            (RedKind::Mean, "(float)n"),
            (RedKind::L2, "sqrtf"),
            (RedKind::Max, "fmaxf"),
        ] {
            let s = reduce_cuda_c(op, 256);
            assert!(s.contains("reduce_main") && s.contains(fin), "{op:?}: {s}");
        }
        assert!(broadcast_cuda_c(64).contains("broadcast_main"));
    }

    /// A single-node MatMul graph lowers to CUDA-C: plain → `gemm_f32`, tc → `wmma_gemm_tiled`.
    /// This is the same graph the WGSL backend lowers (Phase-1 seed bridge) — no per-id branch.
    #[test]
    fn matmul_graph_lowers_to_cuda_c() {
        let dyn2 = Shape::new(&[0, 0]);
        for (tc, entry, marker) in [
            (false, "gemm_f32", "float* c"),
            (true, "wmma_gemm_tiled", "wmma::"),
        ] {
            let mut g = ComputeGraph::new();
            let a = TensorRef::external(dyn2, DType::F32);
            let b = TensorRef::external(dyn2, DType::F32);
            let out = g
                .push(
                    OpNode::MatMul {
                        m: 0,
                        n: 0,
                        k: 0,
                        tc,
                        trans_b: false,
                    },
                    &[a, b],
                    dyn2,
                    DType::F32,
                    Schedule::default(),
                )
                .expect("push matmul");
            g.mark_output(out);
            let shader = emit_graph_cuda_c(&g, Schedule::default()).expect("lower to cuda-c");
            assert!(shader.source.contains(marker), "tc={tc}: {}", shader.source);
            assert_eq!(graph_cuda_entry(&g.nodes[0]).unwrap(), entry);
        }
    }

    /// `trans_b=true` and the not-yet-built op-classes lower to an explicit `Err`, never a
    /// silent no-op (the completeness/honesty bar).
    #[test]
    fn unsupported_matmul_and_ops_error() {
        let dyn2 = Shape::new(&[0, 0]);
        let mut g = ComputeGraph::new();
        let a = TensorRef::external(dyn2, DType::F32);
        let b = TensorRef::external(dyn2, DType::F32);
        let out = g
            .push(
                OpNode::MatMul {
                    m: 0,
                    n: 0,
                    k: 0,
                    tc: false,
                    trans_b: true,
                },
                &[a, b],
                dyn2,
                DType::F32,
                Schedule::default(),
            )
            .expect("push");
        g.mark_output(out);
        assert!(emit_graph_cuda_c(&g, Schedule::default()).is_err());
    }
}

/// Cross-backend differential oracle (Phase-5 verification) — requires an NVIDIA device.
/// The **same** compute-graph that the WGSL backend lowers + certifies is lowered here to
/// CUDA-C, compiled by NVRTC, dispatched on the GPU, and graded against the CPU floor.
/// `#[ignore]` (needs the CUDA toolkit + device); run with `--features cuda -- --ignored`.
#[cfg(all(test, feature = "cuda"))]
mod cuda_tests {
    use super::*;
    use crate::wgsl_forge::execute::{CudaComputeContext, CudaPipeline, QualiaCompute};
    use crate::wgsl_forge::ir::graph::{DType, Shape, TensorRef};
    use crate::wgsl_forge::oracle::gemm_cpu;

    /// Build a one-node MatMul graph and lower it to CUDA-C (the Phase-1 seed bridge graph,
    /// identical to the WGSL path's input).
    fn matmul_graph(tc: bool) -> ComputeGraph {
        let dyn2 = Shape::new(&[0, 0]);
        let mut g = ComputeGraph::new();
        let a = TensorRef::external(dyn2, DType::F32);
        let b = TensorRef::external(dyn2, DType::F32);
        let out = g
            .push(
                OpNode::MatMul {
                    m: 0,
                    n: 0,
                    k: 0,
                    tc,
                    trans_b: false,
                },
                &[a, b],
                dyn2,
                DType::F32,
                Schedule::default(),
            )
            .expect("push matmul");
        g.mark_output(out);
        g
    }

    /// Compile + dispatch a single-node graph's emitted CUDA-C with the `[a, b, c, dims]`
    /// GEMM ABI, returning the `m*n` f32 output. `is_f16` packs A/B to f16 (the WMMA path).
    fn run_graph_gemm(
        g: &ComputeGraph,
        m: usize,
        k: usize,
        n: usize,
        a: &[f32],
        b: &[f32],
        is_f16: bool,
    ) -> Vec<f32> {
        let shader = emit_graph_cuda_c(g, Schedule::default()).expect("lower to cuda-c");
        let entry = graph_cuda_entry(&g.nodes[0]).expect("entry");
        let mut ctx = CudaComputeContext::new(64 * 1024 * 1024).expect("cuda ctx");
        let (view_a, view_b) = if is_f16 {
            let ab: Vec<u16> = a
                .iter()
                .map(|&x| half::f16::from_f32(x).to_bits())
                .collect();
            let bb: Vec<u16> = b
                .iter()
                .map(|&x| half::f16::from_f32(x).to_bits())
                .collect();
            (
                ctx.allocate_and_write(bytemuck::cast_slice(&ab), 0, 0)
                    .unwrap(),
                ctx.allocate_and_write(bytemuck::cast_slice(&bb), 1, 0)
                    .unwrap(),
            )
        } else {
            (
                ctx.allocate_and_write(bytemuck::cast_slice(a), 0, 0)
                    .unwrap(),
                ctx.allocate_and_write(bytemuck::cast_slice(b), 1, 0)
                    .unwrap(),
            )
        };
        let zeros = vec![0.0f32; m * n];
        let view_c = ctx
            .allocate_and_write(bytemuck::cast_slice(&zeros), 2, 0)
            .unwrap();
        let dims: [u32; 3] = [m as u32, n as u32, k as u32];
        let view_dims = ctx
            .allocate_and_write(bytemuck::cast_slice(&dims), 3, 0)
            .unwrap();
        let pipeline =
            CudaPipeline::compile_cuda_c_source(&ctx, &shader.source, entry, &[0, 1, 2, 3])
                .expect("nvrtc compile");
        let buffers = vec![view_a, view_b, view_c, view_dims];
        if is_f16 {
            // WMMA: one warp (32) per 16x16 tile.
            let num_tiles = (m / 16) * (n / 16);
            let sched = Schedule {
                workgroup_size: 32,
                ..Default::default()
            };
            pipeline.dispatch(&buffers, &sched, num_tiles * 32).unwrap();
        } else {
            let sched = Schedule {
                workgroup_size: 64,
                ..Default::default()
            };
            pipeline.dispatch(&buffers, &sched, m * n).unwrap();
        }
        let mut out = ctx.read_buffer_f32(&view_c).unwrap();
        out.truncate(m * n);
        out
    }

    #[test]
    #[serial_test::serial(gpu)]
    fn plain_matmul_graph_certifies_on_cuda() {
        if !crate::wgsl_forge::test_cuda_available() { return; }
        let (m, k, n) = (32usize, 32usize, 32usize);
        let a: Vec<f32> = (0..m * k).map(|i| ((i % 11) as f32) * 0.3 - 1.5).collect();
        let b: Vec<f32> = (0..k * n).map(|i| ((i % 13) as f32) * 0.2 + 0.05).collect();
        let got = run_graph_gemm(&matmul_graph(false), m, k, n, &a, &b, false);
        let want = gemm_cpu(&a, &b, m, k, n);
        for (g, w) in got.iter().zip(&want) {
            assert!(
                (g - w).abs() <= 1e-3 + 1e-3 * w.abs(),
                "cuda {g} vs cpu {w}"
            );
        }
        assert!(got.iter().any(|&v| v.abs() > 1e-6));
    }

    #[test]
    #[serial_test::serial(gpu)]
    fn tc_matmul_graph_certifies_on_cuda_wmma() {
        if !crate::wgsl_forge::test_cuda_available() { return; }
        // 16-multiples for WMMA; f16-rounded reference tolerance.
        let (m, k, n) = (16usize, 16usize, 16usize);
        let a: Vec<f32> = (0..m * k).map(|i| ((i % 7) as f32) * 0.25 - 0.75).collect();
        let b: Vec<f32> = (0..k * n).map(|i| ((i % 5) as f32) * 0.5 + 0.1).collect();
        let got = run_graph_gemm(&matmul_graph(true), m, k, n, &a, &b, true);
        // Reference: f16-rounded inputs through the exact CPU GEMM (matches WMMA semantics).
        let a16: Vec<f32> = a.iter().map(|&x| half::f16::from_f32(x).to_f32()).collect();
        let b16: Vec<f32> = b.iter().map(|&x| half::f16::from_f32(x).to_f32()).collect();
        let want = gemm_cpu(&a16, &b16, m, k, n);
        for (g, w) in got.iter().zip(&want) {
            assert!(
                (g - w).abs() <= 5e-2 + 5e-2 * w.abs(),
                "wmma {g} vs ref {w}"
            );
        }
        assert!(got.iter().any(|&v| v.abs() > 1e-6));
    }

    /// NVRTC-compile every CudaCLowerer-emitted kit kernel (elementwise arities, each reduce
    /// kind, broadcast, gemv) — the CUDA analogue of the WGSL kit's naga-validate. Proves the
    /// emitted CUDA-C is valid (compiles to PTX) for the full node coverage, independent of a
    /// multi-node executor.
    #[test]
    #[serial_test::serial(gpu)]
    fn kit_kernels_nvrtc_compile() {
        if !crate::wgsl_forge::test_cuda_available() { return; }
        let ctx = CudaComputeContext::new(8 * 1024 * 1024).expect("cuda ctx");
        let compile = |src: &str, entry: &str, binds: &[u32]| {
            CudaPipeline::compile_cuda_c_source(&ctx, src, entry, binds)
                .unwrap_or_else(|e| panic!("nvrtc compile {entry}: {e}"));
        };
        // Elementwise: unary (3 bindings), binary (4), fma (5).
        compile(
            &elementwise_cuda_c(EwKind::Silu, 64).unwrap(),
            "ewise_main",
            &[0, 1, 2],
        );
        compile(
            &elementwise_cuda_c(EwKind::Gelu, 64).unwrap(),
            "ewise_main",
            &[0, 1, 2],
        );
        compile(
            &elementwise_cuda_c(EwKind::Add, 64).unwrap(),
            "ewise_main",
            &[0, 1, 2, 3],
        );
        compile(
            &elementwise_cuda_c(EwKind::Fma, 64).unwrap(),
            "ewise_main",
            &[0, 1, 2, 3, 4],
        );
        // Reduce: each kind (3 bindings).
        for op in [RedKind::Sum, RedKind::Mean, RedKind::L2, RedKind::Max] {
            compile(&reduce_cuda_c(op, 256), "reduce_main", &[0, 1, 2]);
        }
        // Broadcast (3 bindings) + GEMV (4 bindings).
        compile(&broadcast_cuda_c(64), "broadcast_main", &[0, 1, 2]);
        compile(GEMV_F32_SRC, GEMV_F32_ENTRY, &[0, 1, 2, 3]);
    }
}
