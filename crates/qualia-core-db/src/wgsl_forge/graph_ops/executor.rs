//! Multi-node graph executor — the keystone that runs a whole [`ComputeGraph`] on the GPU
//! with intermediates kept device-side, plus a topologically-composed CPU oracle. This is
//! what unblocks softmax, RMSNorm, the SwiGLU-FFN block, and the full LLM decode DAG. See
//! [`docs/plans/dag-ir-forge.md`] §7–§9.
//!
//! # Execution model (Option A — honest)
//!
//! Nodes run in topological (insertion) order. The slab split matters: wgpu forbids the **same
//! buffer** being bound read-write *and* read-only within one dispatch (read_write is an
//! exclusive usage), so a node's read-only inputs/params and its read_write output cannot share
//! a slab. Therefore:
//! - graph inputs, params, and every node's *readable* tensor live in the **read slab**
//!   (`slab`); GEMM's 16-byte uniform params block likewise (the read slab is uniform-capable);
//! - a node writes its output into the **read_write slab** (`out_slab`), then it is copied
//!   (GPU→GPU, [`WgpuComputeContext::copy_view`]) into a fresh read-slab buffer — the device-side
//!   hand-off to the next node, with **no host readback between nodes**.
//!
//! A producer's output is fed to a consumer by re-binding the (`Copy`) [`BufferView`] to the
//! consumer's binding slot ([`at`]). Each node is one `dispatch()` (one `queue.submit()`) plus
//! one small copy — the per-node submit latency the design accepts; single-encoder multi-pass
//! fusion is a later perf pass. Buffers are never freed within a run (the slab is a bump ring),
//! so the context capacity must hold the whole graph's tensors at once — fine for a decode block;
//! long sequences will want buffer-lifetime reuse (a follow-on).

use super::{broadcast, elementwise, reduce};
use crate::wgsl_forge::execute::{
    BindingUsage, BufferView, QualiaCompute, WgpuComputeContext, WgpuPipeline,
};
use crate::wgsl_forge::ir::graph::{ComputeGraph, EwKind, GraphNode, NodeId, OpNode};
use crate::wgsl_forge::ir::BuiltinKernel;
use crate::wgsl_forge::{ForgeError, Schedule};

/// Context capacity (per slab). Large enough for a decode block's tensors held at once.
const EXEC_CAPACITY: usize = 64 << 20;

/// Resolve the f32 element count of a tensor view.
fn elems(v: &BufferView) -> usize {
    v.length_bytes / 4
}

/// Compose the graph on the CPU in topological order — the differential oracle for
/// [`execute_graph`]. Each node's output is computed from its inputs (graph externals or
/// prior nodes' outputs) using the per-op-class CPU floor, and threaded forward. Returns the
/// final output tensor (the last `mark_output`, or the last node).
pub fn execute_graph_cpu(
    graph: &ComputeGraph,
    externals: &[Vec<f32>],
) -> Result<Vec<f32>, ForgeError> {
    let mut node_out: Vec<Vec<f32>> = Vec::with_capacity(graph.nodes.len());
    for node in &graph.nodes {
        // Gather inputs as owned slices (cheap for these graphs; avoids borrow tangles).
        let ins: Vec<Vec<f32>> = (0..node.n_in as usize)
            .map(|k| {
                let tr = node.ins[k].expect("declared input present");
                if tr.producer == NodeId::EXTERNAL {
                    externals[tr.tensor.0 as usize].clone()
                } else {
                    node_out[tr.producer.0 as usize].clone()
                }
            })
            .collect();
        let out = match node.op {
            OpNode::Reduce { op, .. } => vec![reduce::reduce_cpu(&ins[0], op)],
            OpNode::Broadcast { .. } => {
                broadcast::broadcast_cpu(&ins[0], node.out.shape.elements() as usize)
            }
            OpNode::Elementwise { f } => match node.n_in {
                1 => elementwise::unary_cpu(&ins[0], f),
                2 => elementwise::binary_cpu(&ins[0], &ins[1], f),
                3 => elementwise::fma_cpu(&ins[0], &ins[1], &ins[2]),
                other => {
                    return Err(ForgeError::Emission(format!(
                        "elementwise arity {other} unsupported"
                    )))
                }
            },
            OpNode::MatMul { m, n, k, .. } => {
                crate::wgsl_forge::oracle::gemm_cpu(&ins[0], &ins[1], m as usize, k as usize, n as usize)
            }
            other => {
                return Err(ForgeError::Emission(format!(
                    "execute_graph_cpu: op {other:?} not supported"
                )))
            }
        };
        node_out.push(out);
    }
    final_output(graph, &node_out)
}

fn final_output<T: Clone>(graph: &ComputeGraph, node_out: &[T]) -> Result<T, ForgeError> {
    let id = graph
        .outputs
        .last()
        .copied()
        .unwrap_or(NodeId(graph.nodes.len().saturating_sub(1) as u32));
    node_out
        .get(id.0 as usize)
        .cloned()
        .ok_or_else(|| ForgeError::Emission("graph has no output node".to_string()))
}

/// Execute the whole graph on the GPU and read back the final tensor. Differs from the
/// composed CPU floor only by f32 GPU arithmetic; certified against [`execute_graph_cpu`].
pub fn execute_graph(
    graph: &ComputeGraph,
    externals: &[Vec<f32>],
) -> Result<Vec<f32>, ForgeError> {
    let mut ctx = WgpuComputeContext::new(EXEC_CAPACITY)?;

    // Upload externals once into the READ slab (they are only ever read; binding is
    // overwritten per consumer). Every node's *readable* tensor lives in the read slab;
    // outputs are written to the read_write slab and copied back (see `finish_node`).
    let mut ext_views: Vec<BufferView> = Vec::with_capacity(externals.len());
    for data in externals {
        ext_views.push(ctx.allocate_and_write(
            bytemuck::cast_slice(data),
            0,
            0,
            BindingUsage::StorageRead,
        )?);
    }

    let mut node_out: Vec<Option<BufferView>> = vec![None; graph.nodes.len()];
    for (i, node) in graph.nodes.iter().enumerate() {
        let mut ins: Vec<BufferView> = Vec::with_capacity(node.n_in as usize);
        for k in 0..node.n_in as usize {
            let tr = node.ins[k].expect("declared input present");
            let v = if tr.producer == NodeId::EXTERNAL {
                *ext_views
                    .get(tr.tensor.0 as usize)
                    .ok_or_else(|| ForgeError::Emission("missing external input".to_string()))?
            } else {
                node_out[tr.producer.0 as usize]
                    .ok_or_else(|| ForgeError::Emission("input from unrun node".to_string()))?
            };
            ins.push(v);
        }
        node_out[i] = Some(run_node(&mut ctx, node, &ins)?);
    }

    let id = graph
        .outputs
        .last()
        .copied()
        .unwrap_or(NodeId(graph.nodes.len().saturating_sub(1) as u32));
    let out = node_out[id.0 as usize]
        .ok_or_else(|| ForgeError::Emission("graph has no output node".to_string()))?;
    ctx.read_buffer_f32(&out)
}

/// Allocate a zeroed `n`-element f32 output buffer at `binding` (read_write slab).
fn alloc_out(ctx: &mut WgpuComputeContext, n: usize, binding: u32) -> Result<BufferView, ForgeError> {
    let zeros = vec![0.0f32; n.max(1)];
    ctx.allocate_and_write(bytemuck::cast_slice(&zeros), binding, 0, BindingUsage::StorageReadWrite)
}

/// Allocate a `u32` params buffer at `binding` with the given `usage`.
fn alloc_params(
    ctx: &mut WgpuComputeContext,
    vals: &[u32],
    binding: u32,
    usage: BindingUsage,
) -> Result<BufferView, ForgeError> {
    ctx.allocate_and_write(bytemuck::cast_slice(vals), binding, 0, usage)
}

/// Re-bind a (`Copy`) view to a specific binding slot, keeping its slab/offset/length.
fn at(mut v: BufferView, binding: u32) -> BufferView {
    v.binding = binding;
    v
}

/// After a node has written its output to the read_write slab, copy it (GPU→GPU) into a
/// fresh READ-slab buffer and return that — so a downstream node can bind it as a
/// read-only input without aliasing its own read_write output. This is the device-side
/// hand-off between nodes (no host readback).
fn finish_node(ctx: &mut WgpuComputeContext, out: BufferView) -> Result<BufferView, ForgeError> {
    let read_copy = ctx.allocate_transient(out.length_bytes, 0, 0, BindingUsage::StorageRead)?;
    ctx.copy_view(&out, &read_copy)?;
    Ok(read_copy)
}

/// Run one node on the GPU and return its output as a READ-slab [`BufferView`] (ready to
/// feed the next node).
fn run_node(
    ctx: &mut WgpuComputeContext,
    node: &GraphNode,
    ins: &[BufferView],
) -> Result<BufferView, ForgeError> {
    let out = run_node_to_out_slab(ctx, node, ins)?;
    finish_node(ctx, out)
}

/// Dispatch a node, leaving its output in the read_write slab (the caller copies it back).
fn run_node_to_out_slab(
    ctx: &mut WgpuComputeContext,
    node: &GraphNode,
    ins: &[BufferView],
) -> Result<BufferView, ForgeError> {
    match node.op {
        OpNode::Reduce { op, .. } => {
            const WG: u32 = 256;
            let n = elems(&ins[0]);
            let out = alloc_out(ctx, 1, 1)?;
            let params = alloc_params(ctx, &[n as u32, 0, 0, 0], 2, BindingUsage::StorageRead)?;
            let src = reduce::reduce_wgsl(op, WG);
            let pipeline = WgpuPipeline::compile(ctx, &src, reduce::REDUCE_ENTRY)?;
            let sched = Schedule { workgroup_size: WG, ..Default::default() };
            // element_count == WG → one workgroup (the reduce is single-workgroup).
            pipeline.dispatch(&[at(ins[0], 0), out, params], &sched, WG as usize)?;
            Ok(out)
        }
        OpNode::Broadcast { .. } => {
            const WG: u32 = 64;
            let in_len = elems(&ins[0]);
            let out_len = node.out.shape.elements() as usize;
            let out = alloc_out(ctx, out_len, 1)?;
            let params = alloc_params(
                ctx,
                &[in_len as u32, out_len as u32, 0, 0],
                2,
                BindingUsage::StorageRead,
            )?;
            let src = broadcast::broadcast_wgsl(WG);
            let pipeline = WgpuPipeline::compile(ctx, &src, broadcast::BROADCAST_ENTRY)?;
            let sched = Schedule { workgroup_size: WG, ..Default::default() };
            pipeline.dispatch(&[at(ins[0], 0), out, params], &sched, out_len)?;
            Ok(out)
        }
        OpNode::Elementwise { f } => run_elementwise(ctx, f, node.n_in, ins),
        OpNode::MatMul { m, n, k, tc, .. } => {
            // `tc=true` requests tensor cores. The portable wgpu path is the coopmat tiled
            // GEMM — taken only when the adapter advertises coopmat AND the runtime probe
            // confirms the multiply actually computes (dormant on wgpu 29.0.3 / #9741, so
            // this falls to the certified plain GEMM floor there). 8-multiple dims required.
            // The CUDA WMMA tensor-core path is reached host-side via `dispatch::gemm_f32_tc`
            // and wired graph-side by the CudaCLowerer (P5), not from this wgpu executor.
            if tc
                && m % 8 == 0
                && n % 8 == 0
                && k % 8 == 0
                && crate::wgsl_forge::dispatch::caps().coopmat
                && crate::wgsl_forge::dispatch::coopmat_usable()
            {
                dispatch_matmul_coopmat(ctx, m, n, k, ins)
            } else {
                dispatch_matmul_plain(ctx, m, n, k, ins)
            }
        }
        other => Err(ForgeError::Emission(format!(
            "executor: op {other:?} not supported"
        ))),
    }
}

/// Plain f32 GEMM node (the always-correct floor): the certified [`BuiltinKernel::Gemm`]
/// WGSL kernel, `C[m×n]=A·B` into the read_write slab. Used for `MatMul.tc=false` and as
/// the fallback when the tensor-core path is unavailable.
fn dispatch_matmul_plain(
    ctx: &mut WgpuComputeContext,
    m: u32,
    n: u32,
    k: u32,
    ins: &[BufferView],
) -> Result<BufferView, ForgeError> {
    const WG: u32 = 64;
    let out_elems = (m as usize) * (n as usize);
    let out = alloc_out(ctx, out_elems, 2)?;
    // GEMM params is a 16-byte UNIFORM block [m, n, k, _pad].
    let params = alloc_params(ctx, &[m, n, k, 0], 3, BindingUsage::Uniform)?;
    let spec = BuiltinKernel::Gemm.spec();
    let sched = Schedule { workgroup_size: WG, ..Default::default() };
    let module = crate::wgsl_forge::emit::emit_wgsl(&spec, sched)?;
    let pipeline = WgpuPipeline::compile(ctx, &module.source, &spec.entry_point)?;
    pipeline.dispatch(&[at(ins[0], 0), at(ins[1], 1), out, params], &sched, out_elems)?;
    Ok(out)
}

/// Tensor-core f32 GEMM node via the tiled cooperative-matrix kernel
/// ([`matmul_tc_wgsl_tiled`](crate::wgsl_forge::emit::matmul_tc_wgsl_tiled)) — the portable
/// wgpu tensor-core path, kept device-side in the slab model like the plain path. One
/// workgroup (== one subgroup, `@workgroup_size(32)`) per 8×8 output tile. Callers gate this
/// on [`coopmat_usable`](crate::wgsl_forge::dispatch::coopmat_usable), so it runs only where
/// the coopmat multiply actually computes (dormant on wgpu 29.0.3 / #9741).
fn dispatch_matmul_coopmat(
    ctx: &mut WgpuComputeContext,
    m: u32,
    n: u32,
    k: u32,
    ins: &[BufferView],
) -> Result<BufferView, ForgeError> {
    use crate::wgsl_forge::emit::{matmul_tc_wgsl_tiled, MATMUL_TC_TILED_ENTRY};
    let out_elems = (m as usize) * (n as usize);
    // c is the read_write slab output AND the zero-seeded accumulator (the kernel loads it).
    let out = alloc_out(ctx, out_elems, 2)?;
    // dims = [m, n, k] as a u32 storage buffer (binding 3, read slab).
    let dims = alloc_params(ctx, &[m, n, k, 0], 3, BindingUsage::StorageRead)?;
    let src = matmul_tc_wgsl_tiled();
    let pipeline = WgpuPipeline::compile(ctx, &src, MATMUL_TC_TILED_ENTRY)?;
    let num_tiles = ((m / 8) * (n / 8)) as usize;
    let sched = Schedule { workgroup_size: 32, ..Default::default() };
    pipeline.dispatch(&[at(ins[0], 0), at(ins[1], 1), out, dims], &sched, num_tiles * 32)?;
    Ok(out)
}

fn run_elementwise(
    ctx: &mut WgpuComputeContext,
    f: EwKind,
    n_in: u8,
    ins: &[BufferView],
) -> Result<BufferView, ForgeError> {
    const WG: u32 = 64;
    let n = elems(&ins[0]);
    let src = elementwise::elementwise_wgsl(f, WG)?;
    let sched = Schedule { workgroup_size: WG, ..Default::default() };
    match n_in {
        1 => {
            let out = alloc_out(ctx, n, 1)?;
            let params = alloc_params(ctx, &[n as u32, 0, 0, 0], 2, BindingUsage::StorageRead)?;
            let pipeline = WgpuPipeline::compile(ctx, &src, elementwise::EWISE_ENTRY)?;
            pipeline.dispatch(&[at(ins[0], 0), out, params], &sched, n)?;
            Ok(out)
        }
        2 => {
            let out = alloc_out(ctx, n, 2)?;
            let params = alloc_params(ctx, &[n as u32, 0, 0, 0], 3, BindingUsage::StorageRead)?;
            let pipeline = WgpuPipeline::compile(ctx, &src, elementwise::EWISE_ENTRY)?;
            pipeline.dispatch(&[at(ins[0], 0), at(ins[1], 1), out, params], &sched, n)?;
            Ok(out)
        }
        3 => {
            let out = alloc_out(ctx, n, 3)?;
            let params = alloc_params(ctx, &[n as u32, 0, 0, 0], 4, BindingUsage::StorageRead)?;
            let pipeline = WgpuPipeline::compile(ctx, &src, elementwise::EWISE_ENTRY)?;
            pipeline.dispatch(
                &[at(ins[0], 0), at(ins[1], 1), at(ins[2], 2), out, params],
                &sched,
                n,
            )?;
            Ok(out)
        }
        other => Err(ForgeError::Emission(format!(
            "elementwise arity {other} unsupported"
        ))),
    }
}

// ── Graph builders for the canonical multi-node LLM ops ──────────────────────────────

/// Numerically-stable softmax over a length-`n` vector, as a 7-node graph:
/// `Reduce(Max) → Broadcast → Sub → Exp → Reduce(Sum) → Broadcast → Div`. One external
/// input (`externals[0]` = the logits).
pub fn softmax_graph(n: u32) -> Result<ComputeGraph, ForgeError> {
    use crate::wgsl_forge::ir::graph::{Axis, DType, RedKind, Shape, TensorRef};
    let mut g = ComputeGraph::new();
    let s = Schedule::default();
    let (sh_n, sh_1) = (Shape::new(&[n]), Shape::new(&[1]));
    let x = TensorRef::input(0, sh_n, DType::F32);
    let mx = g.push(OpNode::Reduce { op: RedKind::Max, axis: Axis::Last }, &[x], sh_1, DType::F32, s)?;
    let mxb = g.push(OpNode::Broadcast { shape: sh_n }, &[mx], sh_n, DType::F32, s)?;
    let shifted = g.push(OpNode::Elementwise { f: EwKind::Sub }, &[x, mxb], sh_n, DType::F32, s)?;
    let e = g.push(OpNode::Elementwise { f: EwKind::Exp }, &[shifted], sh_n, DType::F32, s)?;
    let sm = g.push(OpNode::Reduce { op: RedKind::Sum, axis: Axis::Last }, &[e], sh_1, DType::F32, s)?;
    let smb = g.push(OpNode::Broadcast { shape: sh_n }, &[sm], sh_n, DType::F32, s)?;
    let out = g.push(OpNode::Elementwise { f: EwKind::Div }, &[e, smb], sh_n, DType::F32, s)?;
    g.mark_output(out);
    Ok(g)
}

/// RMSNorm (no weight/eps — the core) over a length-`n` vector, as a 5-node graph:
/// `Mul(x,x) → Reduce(Mean) → RecipSqrt → Broadcast → Mul(x, ·)`. One external input.
pub fn rmsnorm_graph(n: u32) -> Result<ComputeGraph, ForgeError> {
    use crate::wgsl_forge::ir::graph::{Axis, DType, RedKind, Shape, TensorRef};
    let mut g = ComputeGraph::new();
    let s = Schedule::default();
    let (sh_n, sh_1) = (Shape::new(&[n]), Shape::new(&[1]));
    let x = TensorRef::input(0, sh_n, DType::F32);
    let sq = g.push(OpNode::Elementwise { f: EwKind::Mul }, &[x, x], sh_n, DType::F32, s)?;
    let ms = g.push(OpNode::Reduce { op: RedKind::Mean, axis: Axis::Last }, &[sq], sh_1, DType::F32, s)?;
    let r = g.push(OpNode::Elementwise { f: EwKind::RecipSqrt }, &[ms], sh_1, DType::F32, s)?;
    let rb = g.push(OpNode::Broadcast { shape: sh_n }, &[r], sh_n, DType::F32, s)?;
    let out = g.push(OpNode::Elementwise { f: EwKind::Mul }, &[x, rb], sh_n, DType::F32, s)?;
    g.mark_output(out);
    Ok(g)
}

/// SwiGLU feed-forward block — the LLM workhorse — as a 5-node graph:
/// `gate = x·Wg`, `up = x·Wu`, `h = silu(gate)·up`, `out = h·Wd`. Externals:
/// `[0]=x [seq,dim], [1]=Wg [dim,ffn], [2]=Wu [dim,ffn], [3]=Wd [ffn,dim]`.
pub fn swiglu_ffn_graph(seq: u32, dim: u32, ffn: u32) -> Result<ComputeGraph, ForgeError> {
    use crate::wgsl_forge::ir::graph::{DType, Shape, TensorRef};
    let mut g = ComputeGraph::new();
    let s = Schedule::default();
    let sh_x = Shape::new(&[seq, dim]);
    let sh_w = Shape::new(&[dim, ffn]);
    let sh_wd = Shape::new(&[ffn, dim]);
    let sh_h = Shape::new(&[seq, ffn]);
    let sh_o = Shape::new(&[seq, dim]);
    let x = TensorRef::input(0, sh_x, DType::F32);
    let wg = TensorRef::input(1, sh_w, DType::F32);
    let wu = TensorRef::input(2, sh_w, DType::F32);
    let wd = TensorRef::input(3, sh_wd, DType::F32);
    let mm = |m, n, k| OpNode::MatMul { m, n, k, tc: false, trans_b: false };
    let gate = g.push(mm(seq, ffn, dim), &[x, wg], sh_h, DType::F32, s)?;
    let up = g.push(mm(seq, ffn, dim), &[x, wu], sh_h, DType::F32, s)?;
    let sg = g.push(OpNode::Elementwise { f: EwKind::Silu }, &[gate], sh_h, DType::F32, s)?;
    let h = g.push(OpNode::Elementwise { f: EwKind::Mul }, &[sg, up], sh_h, DType::F32, s)?;
    let out = g.push(mm(seq, dim, ffn), &[h, wd], sh_o, DType::F32, s)?;
    g.mark_output(out);
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The composed CPU oracle for softmax is a valid probability distribution
    /// (non-negative, sums to 1) and matches a direct reference.
    #[test]
    fn softmax_cpu_oracle_is_a_distribution() {
        let x: Vec<f32> = vec![1.0, 2.0, 3.0, 0.5, -1.0, 4.0, 2.5, 0.0];
        let g = softmax_graph(x.len() as u32).unwrap();
        let out = execute_graph_cpu(&g, &[x.clone()]).unwrap();
        let sum: f32 = out.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5, "softmax must sum to 1, got {sum}");
        assert!(out.iter().all(|&p| p >= 0.0));
        // Direct reference.
        let m = x.iter().cloned().fold(f32::MIN, f32::max);
        let exps: Vec<f32> = x.iter().map(|&v| (v - m).exp()).collect();
        let z: f32 = exps.iter().sum();
        for (o, e) in out.iter().zip(exps.iter()) {
            assert!((o - e / z).abs() < 1e-5);
        }
    }

    /// The composed CPU oracle for RMSNorm scales `x` by `1/rms(x)`.
    #[test]
    fn rmsnorm_cpu_oracle_matches_reference() {
        let x: Vec<f32> = vec![3.0, 4.0, 0.0, 0.0]; // rms = sqrt((9+16)/4) = 2.5
        let g = rmsnorm_graph(x.len() as u32).unwrap();
        let out = execute_graph_cpu(&g, &[x.clone()]).unwrap();
        let ms: f32 = x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32;
        let inv = ms.sqrt().recip();
        for (o, xi) in out.iter().zip(x.iter()) {
            assert!((o - xi * inv).abs() < 1e-5, "{o} vs {}", xi * inv);
        }
    }

    /// The composed CPU oracle for the SwiGLU-FFN block matches a hand-written reference.
    #[test]
    fn swiglu_ffn_cpu_oracle_matches_reference() {
        let (seq, dim, ffn) = (2usize, 3usize, 4usize);
        let x: Vec<f32> = (0..seq * dim).map(|i| (i as f32) * 0.1 - 0.3).collect();
        let wg: Vec<f32> = (0..dim * ffn).map(|i| (i as f32) * 0.05 - 0.2).collect();
        let wu: Vec<f32> = (0..dim * ffn).map(|i| (i as f32) * 0.03 - 0.1).collect();
        let wd: Vec<f32> = (0..ffn * dim).map(|i| (i as f32) * 0.02 - 0.1).collect();
        let g = swiglu_ffn_graph(seq as u32, dim as u32, ffn as u32).unwrap();
        let out = execute_graph_cpu(&g, &[x.clone(), wg.clone(), wu.clone(), wd.clone()]).unwrap();

        // Reference: gate = x·Wg, up = x·Wu, h = silu(gate)·up, y = h·Wd.
        let mm = |a: &[f32], b: &[f32], m: usize, k: usize, n: usize| {
            let mut c = vec![0.0f32; m * n];
            for i in 0..m {
                for j in 0..n {
                    let mut acc = 0.0;
                    for kk in 0..k {
                        acc += a[i * k + kk] * b[kk * n + j];
                    }
                    c[i * n + j] = acc;
                }
            }
            c
        };
        let gate = mm(&x, &wg, seq, dim, ffn);
        let up = mm(&x, &wu, seq, dim, ffn);
        let h: Vec<f32> = gate
            .iter()
            .zip(up.iter())
            .map(|(&g, &u)| (g / (1.0 + (-g).exp())) * u)
            .collect();
        let y = mm(&h, &wd, seq, ffn, dim);
        assert_eq!(out.len(), y.len());
        for (o, r) in out.iter().zip(y.iter()) {
            assert!((o - r).abs() < 1e-5, "{o} vs {r}");
        }
    }

    /// GPU certify: the full multi-node graph executed on the A2000 (intermediates kept
    /// device-side) must match the composed CPU oracle within f32 tolerance — for softmax,
    /// RMSNorm, and the SwiGLU-FFN block. Run by the orchestrator.
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn execute_graph_gpu_matches_cpu_oracle() {
        // softmax (1024-wide → exercises grid-stride reduce + broadcast + elementwise chain)
        {
            let n = 1024usize;
            let x: Vec<f32> = (0..n).map(|i| ((i * 13 % 97) as f32) * 0.1 - 5.0).collect();
            let g = softmax_graph(n as u32).unwrap();
            let gpu = execute_graph(&g, &[x.clone()]).expect("softmax gpu");
            let cpu = execute_graph_cpu(&g, &[x]).unwrap();
            for (a, b) in gpu.iter().zip(cpu.iter()) {
                assert!((a - b).abs() <= 1e-4, "softmax: {a} vs {b}");
            }
            assert!((gpu.iter().sum::<f32>() - 1.0).abs() < 1e-3);
        }
        // RMSNorm
        {
            let n = 768usize;
            let x: Vec<f32> = (0..n).map(|i| ((i * 7 % 31) as f32) * 0.2 - 3.0).collect();
            let g = rmsnorm_graph(n as u32).unwrap();
            let gpu = execute_graph(&g, &[x.clone()]).expect("rmsnorm gpu");
            let cpu = execute_graph_cpu(&g, &[x]).unwrap();
            for (a, b) in gpu.iter().zip(cpu.iter()) {
                assert!((a - b).abs() <= 1e-3 * b.abs().max(1.0), "rmsnorm: {a} vs {b}");
            }
        }
        // SwiGLU-FFN block (the LLM workhorse) — MatMul + Elementwise multi-node DAG.
        {
            let (seq, dim, ffn) = (8u32, 64u32, 128u32);
            let x: Vec<f32> = (0..seq * dim).map(|i| ((i % 17) as f32) * 0.05 - 0.4).collect();
            let wg: Vec<f32> = (0..dim * ffn).map(|i| ((i % 13) as f32) * 0.02 - 0.12).collect();
            let wu: Vec<f32> = (0..dim * ffn).map(|i| ((i % 11) as f32) * 0.015 - 0.07).collect();
            let wd: Vec<f32> = (0..ffn * dim).map(|i| ((i % 7) as f32) * 0.01 - 0.03).collect();
            let g = swiglu_ffn_graph(seq, dim, ffn).unwrap();
            let ext = vec![x, wg, wu, wd];
            let gpu = execute_graph(&g, &ext).expect("ffn gpu");
            let cpu = execute_graph_cpu(&g, &ext).unwrap();
            assert_eq!(gpu.len(), cpu.len());
            for (a, b) in gpu.iter().zip(cpu.iter()) {
                assert!((a - b).abs() <= 1e-2 * b.abs().max(1.0), "ffn: {a} vs {b}");
            }
        }
    }
}
