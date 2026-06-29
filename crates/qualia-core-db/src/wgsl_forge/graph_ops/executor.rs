//! Multi-node graph executor — the keystone that runs a whole [`ComputeGraph`] on the GPU
//! with intermediates kept device-side, plus a topologically-composed CPU oracle. This is
//! what unblocks softmax, RMSNorm, the SwiGLU-FFN block, and the full LLM decode DAG. See
//! [`docs/plans/dag-ir-forge.md`] §7–§9.
//!
//! # Execution model (throughput pass — context reuse + single-encoder fusion)
//!
//! Nodes run in topological (insertion) order. The slab split matters: wgpu forbids the **same
//! buffer** being bound read-write *and* read-only within one dispatch (read_write is an
//! exclusive usage), so a node's read-only inputs/params and its read_write output cannot share
//! a slab. Therefore:
//! - graph inputs, params, and every node's *readable* tensor live in the **read slab**
//!   (`slab`); GEMM's 16-byte uniform params block likewise (the read slab is uniform-capable);
//! - a node writes its output into the **read_write slab** (`out_slab`), then it is copied
//!   (GPU→GPU) into a fresh read-slab buffer — the device-side hand-off to the next node, with
//!   **no host readback between nodes**.
//!
//! A producer's output is fed to a consumer by re-binding the (`Copy`) [`BufferView`] to the
//! consumer's binding slot ([`at`]). Two optimizations vs the original Option-A executor
//! (plan §8.1, both proven here against the same CPU oracle):
//!
//! 1. **Context reuse.** [`ForgeGraphExecutor`] owns one [`WgpuComputeContext`] (device, queue,
//!    and the two 64-MiB slabs created **once**); [`ForgeGraphExecutor::run`] resets the slab
//!    (the bump ring is freed) at the start of each call and reuses everything else. The
//!    free-function [`execute_graph`] keeps its one-shot signature by building a throwaway
//!    executor, but a caller decoding many tokens should hold a [`ForgeGraphExecutor`] and call
//!    [`run`](ForgeGraphExecutor::run) per step, paying device creation only once.
//! 2. **Single-encoder deferred submit (Option B).** Every node's dispatch *and* its GPU→GPU
//!    hand-off copy are recorded into **one** [`wgpu::CommandEncoder`] and submitted **once** per
//!    graph ([`WgpuComputeContext::submit_graph`]), instead of one `queue.submit()` per node.
//!    wgpu preserves command order within a command buffer and inserts the buffer hazard
//!    barriers, so the per-node data dependencies (already correct by insertion order) hold.
//!
//! Buffers are never freed *within* a run (the slab is a bump ring), so the context capacity
//! must hold the whole graph's tensors at once — fine for a decode block; long sequences will
//! want buffer-lifetime reuse (a follow-on). Pipelines are still compiled per node per call;
//! caching them across calls is a further, independent step.

use super::{broadcast, elementwise, gather_dequant, reduce};
use crate::wgsl_forge::execute::{
    BindingUsage, BufferView, GraphPass, WgpuComputeContext,
};
use crate::wgsl_forge::ir::graph::{ComputeGraph, DType, EwKind, GraphNode, NodeId, OpNode};
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
            OpNode::GatherDequant { scheme, .. } => {
                if scheme != DType::Ternary {
                    return Err(ForgeError::Emission(format!(
                        "execute_graph_cpu GatherDequant: scheme {scheme:?} unsupported"
                    )));
                }
                let rows = node.out.shape.dims[0] as usize;
                let cols = node.out.shape.dims[1] as usize;
                gather_dequant::gather_dequant_ternary_cpu(&ins[0], &ins[1], rows, cols)
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

/// Execute the whole graph on the GPU and read back the final tensor (one-shot). Differs from
/// the composed CPU floor only by f32 GPU arithmetic; certified against [`execute_graph_cpu`].
///
/// This builds a throwaway [`ForgeGraphExecutor`] (one device + slab per call). A caller that
/// runs many graphs — e.g. one decode block per generated token — should instead hold a
/// [`ForgeGraphExecutor`] and call [`ForgeGraphExecutor::run`] per step, so device/slab
/// creation is paid once rather than per call (the throughput pass, plan §8.1).
pub fn execute_graph(
    graph: &ComputeGraph,
    externals: &[Vec<f32>],
) -> Result<Vec<f32>, ForgeError> {
    ForgeGraphExecutor::new()?.run(graph, externals)
}

/// A reusable GPU graph executor: owns one [`WgpuComputeContext`] (device, queue, and the two
/// slabs created **once**) so running many graphs does not recreate the device per call. The
/// slab is reset between calls ([`ForgeGraphExecutor::run`]), so each call gets the full
/// capacity back (the slab is a bump ring; buffers are not freed mid-run).
pub struct ForgeGraphExecutor {
    ctx: WgpuComputeContext,
}

impl ForgeGraphExecutor {
    /// Create an executor with the default per-slab capacity ([`EXEC_CAPACITY`]). Acquires the
    /// GPU adapter + device once; reuse the returned value across calls/decode steps.
    pub fn new() -> Result<Self, ForgeError> {
        Self::with_capacity(EXEC_CAPACITY)
    }

    /// Create an executor with an explicit per-slab capacity in bytes.
    pub fn with_capacity(capacity_bytes: usize) -> Result<Self, ForgeError> {
        Ok(Self { ctx: WgpuComputeContext::new(capacity_bytes)? })
    }

    /// Borrow the underlying context (e.g. to read adapter identity / profile).
    pub fn context(&self) -> &WgpuComputeContext {
        &self.ctx
    }

    /// Execute `graph` on the GPU with `externals` as the graph inputs, returning the final
    /// output tensor. Intermediates are kept device-side; every node's dispatch and its GPU→GPU
    /// hand-off copy are recorded into ONE command encoder and submitted once
    /// ([`WgpuComputeContext::submit_graph`], Option B). Re-uses the device/slab across calls.
    pub fn run(
        &mut self,
        graph: &ComputeGraph,
        externals: &[Vec<f32>],
    ) -> Result<Vec<f32>, ForgeError> {
        let ctx = &mut self.ctx;

        // Per-call slab reset: free the previous call's transient allocations so this call
        // starts with the full capacity. Safe here — the previous `run`'s final readback
        // (`read_buffer_f32`) fully synchronized the device before returning.
        ctx.clear_transient_allocations();

        // Upload externals once into the READ slab (they are only ever read; binding is
        // overwritten per consumer). Every node's *readable* tensor lives in the read slab;
        // outputs are written to the read_write slab and copied back (the hand-off copy).
        let mut ext_views: Vec<BufferView> = Vec::with_capacity(externals.len());
        for data in externals {
            ext_views.push(ctx.allocate_and_write(
                bytemuck::cast_slice(data),
                0,
                0,
                BindingUsage::StorageRead,
            )?);
        }

        // Phase A — prepare every node: allocate its output + params (read/read_write slab
        // split), compile its pipeline, build its bind group. No GPU work is submitted yet.
        // Each node's output (the read-slab hand-off copy) is threaded forward as a `BufferView`.
        let mut passes: Vec<GraphPass> = Vec::with_capacity(graph.nodes.len());
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
            let (pass, out_view) = prepare_node(ctx, node, &ins)?;
            node_out[i] = Some(out_view);
            passes.push(pass);
        }

        // Phase B — record all node dispatches + GPU→GPU hand-off copies into ONE encoder and
        // submit once for the whole graph (Option B; eliminates per-node submit latency).
        ctx.submit_graph(&passes)?;

        let id = graph
            .outputs
            .last()
            .copied()
            .unwrap_or(NodeId(graph.nodes.len().saturating_sub(1) as u32));
        let out = node_out[id.0 as usize]
            .ok_or_else(|| ForgeError::Emission("graph has no output node".to_string()))?;
        ctx.read_buffer_f32(&out)
    }
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

/// Compile a node's kernel, build its bind group, and package it as a [`GraphPass`] to be
/// recorded later (no dispatch here — the whole graph is submitted in one encoder). `bindings`
/// is the node's full binding list (inputs at their slots + the read_write `out` view);
/// `out` is that read_write output. Allocates a fresh read-slab buffer for the GPU→GPU
/// hand-off copy and returns it as the node's output `BufferView` for downstream consumers.
fn record_kernel(
    ctx: &mut WgpuComputeContext,
    source: &str,
    entry: &str,
    bindings: &[BufferView],
    out: BufferView,
    sched: Schedule,
    element_count: usize,
) -> Result<(GraphPass, BufferView), ForgeError> {
    // Cached compile: a held ForgeGraphExecutor re-running the same graph (one decode block
    // per token) compiles each node's shader once total, then pays only bind-group + dispatch.
    let pipeline = ctx.compile_pipeline_cached(source, entry)?;
    let bind_group = ctx.create_compute_bind_group(&pipeline, bindings);
    let workgroups = sched.dispatch_workgroups(element_count);
    // Device-side hand-off: a fresh read-slab buffer the consumer binds read-only. The copy
    // (`out` read_write slab → `read_copy` read slab) is recorded into the shared encoder by
    // `submit_graph`, so it is ordered after this node's pass and before any consumer.
    let read_copy = ctx.allocate_transient(out.length_bytes, 0, 0, BindingUsage::StorageRead)?;
    let pass = GraphPass { pipeline, bind_group, workgroups, copy: Some((out, read_copy)) };
    Ok((pass, read_copy))
}

/// Prepare one node into a recordable [`GraphPass`], returning it plus the node's output as a
/// READ-slab [`BufferView`] (the hand-off copy, ready to feed the next node). Allocates +
/// uploads the node's buffers and compiles its pipeline, but submits no GPU work — the whole
/// graph is recorded into a single encoder and submitted once (see [`ForgeGraphExecutor::run`]).
fn prepare_node(
    ctx: &mut WgpuComputeContext,
    node: &GraphNode,
    ins: &[BufferView],
) -> Result<(GraphPass, BufferView), ForgeError> {
    match node.op {
        OpNode::Reduce { op, .. } => {
            const WG: u32 = 256;
            let n = elems(&ins[0]);
            let out = alloc_out(ctx, 1, 1)?;
            let params = alloc_params(ctx, &[n as u32, 0, 0, 0], 2, BindingUsage::StorageRead)?;
            let src = reduce::reduce_wgsl(op, WG);
            let sched = Schedule { workgroup_size: WG, ..Default::default() };
            // element_count == WG → one workgroup (the reduce is single-workgroup).
            record_kernel(ctx, &src, reduce::REDUCE_ENTRY, &[at(ins[0], 0), out, params], out, sched, WG as usize)
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
            let sched = Schedule { workgroup_size: WG, ..Default::default() };
            record_kernel(ctx, &src, broadcast::BROADCAST_ENTRY, &[at(ins[0], 0), out, params], out, sched, out_len)
        }
        OpNode::Elementwise { f } => prepare_elementwise(ctx, f, node.n_in, ins),
        OpNode::GatherDequant { scheme, .. } => {
            if scheme != DType::Ternary {
                return Err(ForgeError::Emission(format!(
                    "executor GatherDequant: scheme {scheme:?} unsupported (Ternary only this phase)"
                )));
            }
            const WG: u32 = 64;
            let rows = node.out.shape.dims[0];
            let cols = node.out.shape.dims[1];
            let k_words = cols.div_ceil(16);
            let out_elems = (rows as usize) * (cols as usize);
            let out = alloc_out(ctx, out_elems, 2)?;
            let params =
                alloc_params(ctx, &[rows, cols, k_words, 0], 3, BindingUsage::StorageRead)?;
            let src = gather_dequant::gather_dequant_ternary_wgsl(WG);
            let sched = Schedule { workgroup_size: WG, ..Default::default() };
            // packed (0), scale (1), output (2, read_write), params (3).
            record_kernel(
                ctx,
                &src,
                gather_dequant::GATHER_DEQUANT_ENTRY,
                &[at(ins[0], 0), at(ins[1], 1), out, params],
                out,
                sched,
                out_elems,
            )
        }
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
                prepare_matmul_coopmat(ctx, m, n, k, ins)
            } else {
                prepare_matmul_plain(ctx, m, n, k, ins)
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
fn prepare_matmul_plain(
    ctx: &mut WgpuComputeContext,
    m: u32,
    n: u32,
    k: u32,
    ins: &[BufferView],
) -> Result<(GraphPass, BufferView), ForgeError> {
    const WG: u32 = 64;
    let out_elems = (m as usize) * (n as usize);
    let out = alloc_out(ctx, out_elems, 2)?;
    // GEMM params is a 16-byte UNIFORM block [m, n, k, _pad].
    let params = alloc_params(ctx, &[m, n, k, 0], 3, BindingUsage::Uniform)?;
    let spec = BuiltinKernel::Gemm.spec();
    let sched = Schedule { workgroup_size: WG, ..Default::default() };
    let module = crate::wgsl_forge::emit::emit_wgsl(&spec, sched)?;
    record_kernel(
        ctx,
        &module.source,
        &spec.entry_point,
        &[at(ins[0], 0), at(ins[1], 1), out, params],
        out,
        sched,
        out_elems,
    )
}

/// Tensor-core f32 GEMM node via the tiled cooperative-matrix kernel
/// ([`matmul_tc_wgsl_tiled`](crate::wgsl_forge::emit::matmul_tc_wgsl_tiled)) — the portable
/// wgpu tensor-core path, kept device-side in the slab model like the plain path. One
/// workgroup (== one subgroup, `@workgroup_size(32)`) per 8×8 output tile. Callers gate this
/// on [`coopmat_usable`](crate::wgsl_forge::dispatch::coopmat_usable), so it runs only where
/// the coopmat multiply actually computes (dormant on wgpu 29.0.3 / #9741).
fn prepare_matmul_coopmat(
    ctx: &mut WgpuComputeContext,
    m: u32,
    n: u32,
    k: u32,
    ins: &[BufferView],
) -> Result<(GraphPass, BufferView), ForgeError> {
    use crate::wgsl_forge::emit::{matmul_tc_wgsl_tiled, MATMUL_TC_TILED_ENTRY};
    let out_elems = (m as usize) * (n as usize);
    // c is the read_write slab output AND the zero-seeded accumulator (the kernel loads it).
    let out = alloc_out(ctx, out_elems, 2)?;
    // dims = [m, n, k] as a u32 storage buffer (binding 3, read slab).
    let dims = alloc_params(ctx, &[m, n, k, 0], 3, BindingUsage::StorageRead)?;
    let src = matmul_tc_wgsl_tiled();
    let num_tiles = ((m / 8) * (n / 8)) as usize;
    let sched = Schedule { workgroup_size: 32, ..Default::default() };
    record_kernel(
        ctx,
        &src,
        MATMUL_TC_TILED_ENTRY,
        &[at(ins[0], 0), at(ins[1], 1), out, dims],
        out,
        sched,
        num_tiles * 32,
    )
}

fn prepare_elementwise(
    ctx: &mut WgpuComputeContext,
    f: EwKind,
    n_in: u8,
    ins: &[BufferView],
) -> Result<(GraphPass, BufferView), ForgeError> {
    const WG: u32 = 64;
    let n = elems(&ins[0]);
    let src = elementwise::elementwise_wgsl(f, WG)?;
    let sched = Schedule { workgroup_size: WG, ..Default::default() };
    match n_in {
        1 => {
            let out = alloc_out(ctx, n, 1)?;
            let params = alloc_params(ctx, &[n as u32, 0, 0, 0], 2, BindingUsage::StorageRead)?;
            record_kernel(ctx, &src, elementwise::EWISE_ENTRY, &[at(ins[0], 0), out, params], out, sched, n)
        }
        2 => {
            let out = alloc_out(ctx, n, 2)?;
            let params = alloc_params(ctx, &[n as u32, 0, 0, 0], 3, BindingUsage::StorageRead)?;
            record_kernel(
                ctx,
                &src,
                elementwise::EWISE_ENTRY,
                &[at(ins[0], 0), at(ins[1], 1), out, params],
                out,
                sched,
                n,
            )
        }
        3 => {
            let out = alloc_out(ctx, n, 3)?;
            let params = alloc_params(ctx, &[n as u32, 0, 0, 0], 4, BindingUsage::StorageRead)?;
            record_kernel(
                ctx,
                &src,
                elementwise::EWISE_ENTRY,
                &[at(ins[0], 0), at(ins[1], 1), at(ins[2], 2), out, params],
                out,
                sched,
                n,
            )
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

// ── Composable sub-block helpers (append nodes to an existing graph) ──────────────────

use crate::wgsl_forge::ir::graph::{Axis, RedKind, Shape, TensorRef};

/// Append **RMSNorm** (no learned weight) of `x` (a `len`-element row) to `g`, returning the
/// output `TensorRef`: `x · rsqrt(mean(x²) + eps)` — the real, numerically-stable RMSNorm
/// (`Mul(x,x) → Reduce(Mean) → Add(eps) → RecipSqrt → Broadcast → Mul(x,·)`). `eps_ref` is a
/// scalar `[1]` graph input (e.g. `1e-5`); the `+eps` guards `rsqrt` against a near-zero mean
/// (matching what trained models use). The per-feature learned scale `γ` is folded into the
/// caller's weight matrices in this decode block, so it is not a separate node here.
fn push_rmsnorm(
    g: &mut ComputeGraph,
    x: TensorRef,
    eps_ref: TensorRef,
    sh_row: Shape,
    sh_1: Shape,
    s: Schedule,
) -> Result<TensorRef, ForgeError> {
    let sq = g.push(OpNode::Elementwise { f: EwKind::Mul }, &[x, x], sh_row, DType::F32, s)?;
    let ms = g.push(OpNode::Reduce { op: RedKind::Mean, axis: Axis::Last }, &[sq], sh_1, DType::F32, s)?;
    let ms_eps = g.push(OpNode::Elementwise { f: EwKind::Add }, &[ms, eps_ref], sh_1, DType::F32, s)?;
    let r = g.push(OpNode::Elementwise { f: EwKind::RecipSqrt }, &[ms_eps], sh_1, DType::F32, s)?;
    let rb = g.push(OpNode::Broadcast { shape: sh_row }, &[r], sh_row, DType::F32, s)?;
    g.push(OpNode::Elementwise { f: EwKind::Mul }, &[x, rb], sh_row, DType::F32, s)
}

/// Append numerically-stable softmax of `scores` (a `len`-element vector) to `g`, returning
/// the output `TensorRef`. `Reduce(Max) → Broadcast → Sub → Exp → Reduce(Sum) → Broadcast → Div`.
fn push_softmax(
    g: &mut ComputeGraph,
    scores: TensorRef,
    sh_vec: Shape,
    sh_1: Shape,
    s: Schedule,
) -> Result<TensorRef, ForgeError> {
    let mx = g.push(OpNode::Reduce { op: RedKind::Max, axis: Axis::Last }, &[scores], sh_1, DType::F32, s)?;
    let mxb = g.push(OpNode::Broadcast { shape: sh_vec }, &[mx], sh_vec, DType::F32, s)?;
    let shifted = g.push(OpNode::Elementwise { f: EwKind::Sub }, &[scores, mxb], sh_vec, DType::F32, s)?;
    let e = g.push(OpNode::Elementwise { f: EwKind::Exp }, &[shifted], sh_vec, DType::F32, s)?;
    let sm = g.push(OpNode::Reduce { op: RedKind::Sum, axis: Axis::Last }, &[e], sh_1, DType::F32, s)?;
    let smb = g.push(OpNode::Broadcast { shape: sh_vec }, &[sm], sh_vec, DType::F32, s)?;
    g.push(OpNode::Elementwise { f: EwKind::Div }, &[e, smb], sh_vec, DType::F32, s)
}

/// Single-token (decode-step) **scaled** dot-product attention as one graph:
/// `probs = softmax((q · Kᵀ) · inv_scale)`, `out = probs · V` — the real attention, **with the
/// `1/√d_head` score scaling** (`inv_scale`, a scalar `[1]` graph input the caller sets to
/// `1/√d`). For a single query row the softmax is over the whole `kv`-length score vector —
/// exactly the LLM decode case (one new token attends to the cached keys/values).
///
/// Externals: `[0]=q [1,d]`, `[1]=kt = Kᵀ [d,kv]`, `[2]=v [kv,d]`, `[3]=inv_scale [1]` (=`1/√d`).
///
/// **Faithfulness notes (honest):** RoPE is assumed **already applied** to `q`/`kt` upstream
/// (or absent) — this graph does not rotate them. Multi-row / prefill attention needs a
/// *row-wise* (axis-aware) reduce — a later extension; this is the decode hot path.
pub fn attention_graph(d: u32, kv: u32) -> Result<ComputeGraph, ForgeError> {
    let mut g = ComputeGraph::new();
    let s = Schedule::default();
    let sh_q = Shape::new(&[1, d]);
    let sh_kt = Shape::new(&[d, kv]);
    let sh_v = Shape::new(&[kv, d]);
    let sh_scores = Shape::new(&[1, kv]);
    let sh_1 = Shape::new(&[1]);
    let sh_o = Shape::new(&[1, d]);
    let q = TensorRef::input(0, sh_q, DType::F32);
    let kt = TensorRef::input(1, sh_kt, DType::F32);
    let v = TensorRef::input(2, sh_v, DType::F32);
    let inv_scale = TensorRef::input(3, sh_1, DType::F32);
    let mm = |m, n, k| OpNode::MatMul { m, n, k, tc: false, trans_b: false };
    // scores = Q[1,d] · Kᵀ[d,kv] = [1,kv], scaled by 1/√d before softmax.
    let scores = g.push(mm(1, kv, d), &[q, kt], sh_scores, DType::F32, s)?;
    let inv_bc = g.push(OpNode::Broadcast { shape: sh_scores }, &[inv_scale], sh_scores, DType::F32, s)?;
    let scaled = g.push(OpNode::Elementwise { f: EwKind::Mul }, &[scores, inv_bc], sh_scores, DType::F32, s)?;
    let probs = push_softmax(&mut g, scaled, sh_scores, sh_1, s)?;
    // out = probs[1,kv] · V[kv,d] = [1,d]
    let out = g.push(mm(1, d, kv), &[probs, v], sh_o, DType::F32, s)?;
    g.mark_output(out);
    Ok(g)
}

/// A full single-token transformer **decode block** as one graph — the headline P4b
/// composition: `res1 = x + attn(RMSNorm(x))`, `out = res1 + SwiGLU-FFN(RMSNorm(res1))`, both
/// residuals, with the **`1/√d` attention scaling** and **RMSNorm `eps`** (so it is faithful to
/// a real transformer block). Uses the cached `Kᵀ`/`V` as externals (the stateful cache-append
/// of the current token's k/v is the engine's job, not the graph's).
///
/// Externals: `[0]=x [1,d]`, `[1]=kt [d,kv]`, `[2]=v [kv,d]`, `[3]=Wg [d,ffn]`, `[4]=Wu [d,ffn]`,
/// `[5]=Wd [ffn,d]`, `[6]=inv_scale [1]` (=`1/√d`), `[7]=eps [1]` (RMSNorm epsilon, e.g. `1e-5`).
///
/// **Faithfulness notes (honest):** single-head (one `d`-wide head); RoPE is assumed applied to
/// `q`/`kt` upstream or absent; the per-feature RMSNorm scale `γ` is folded into `Wg`/`Wu`; the
/// KV cache is given (not computed). These are decode-step modeling choices, all explicit.
pub fn decode_block_graph(d: u32, kv: u32, ffn: u32) -> Result<ComputeGraph, ForgeError> {
    let mut g = ComputeGraph::new();
    let s = Schedule::default();
    let sh_row = Shape::new(&[1, d]);
    let sh_1 = Shape::new(&[1]);
    let sh_kt = Shape::new(&[d, kv]);
    let sh_v = Shape::new(&[kv, d]);
    let sh_scores = Shape::new(&[1, kv]);
    let sh_w = Shape::new(&[d, ffn]);
    let sh_wd = Shape::new(&[ffn, d]);
    let sh_h = Shape::new(&[1, ffn]);
    let mm = |m, n, k| OpNode::MatMul { m, n, k, tc: false, trans_b: false };

    let x = TensorRef::input(0, sh_row, DType::F32);
    let kt = TensorRef::input(1, sh_kt, DType::F32);
    let v = TensorRef::input(2, sh_v, DType::F32);
    let wg = TensorRef::input(3, sh_w, DType::F32);
    let wu = TensorRef::input(4, sh_w, DType::F32);
    let wd = TensorRef::input(5, sh_wd, DType::F32);
    let inv_scale = TensorRef::input(6, sh_1, DType::F32);
    let eps = TensorRef::input(7, sh_1, DType::F32);

    // ── Attention sub-block over RMSNorm(x), residual back to x ──
    let n1 = push_rmsnorm(&mut g, x, eps, sh_row, sh_1, s)?;
    let scores = g.push(mm(1, kv, d), &[n1, kt], sh_scores, DType::F32, s)?;
    let inv_bc = g.push(OpNode::Broadcast { shape: sh_scores }, &[inv_scale], sh_scores, DType::F32, s)?;
    let scaled = g.push(OpNode::Elementwise { f: EwKind::Mul }, &[scores, inv_bc], sh_scores, DType::F32, s)?;
    let probs = push_softmax(&mut g, scaled, sh_scores, sh_1, s)?;
    let attn = g.push(mm(1, d, kv), &[probs, v], sh_row, DType::F32, s)?;
    let res1 = g.push(OpNode::Elementwise { f: EwKind::Add }, &[x, attn], sh_row, DType::F32, s)?;

    // ── SwiGLU-FFN sub-block over RMSNorm(res1), residual back to res1 ──
    let n2 = push_rmsnorm(&mut g, res1, eps, sh_row, sh_1, s)?;
    let gate = g.push(mm(1, ffn, d), &[n2, wg], sh_h, DType::F32, s)?;
    let up = g.push(mm(1, ffn, d), &[n2, wu], sh_h, DType::F32, s)?;
    let sg = g.push(OpNode::Elementwise { f: EwKind::Silu }, &[gate], sh_h, DType::F32, s)?;
    let h = g.push(OpNode::Elementwise { f: EwKind::Mul }, &[sg, up], sh_h, DType::F32, s)?;
    let ffn_out = g.push(mm(1, d, ffn), &[h, wd], sh_row, DType::F32, s)?;
    let out = g.push(OpNode::Elementwise { f: EwKind::Add }, &[res1, ffn_out], sh_row, DType::F32, s)?;
    g.mark_output(out);
    Ok(g)
}

/// A single-token GEMV against a **ternary-packed** weight matrix — the `{GatherDequant →
/// MatMul}` split that decompresses a BitNet-style weight on the fly and immediately consumes
/// it. `w_f32 = GatherDequant(packed, scale)`, `y = x · w_f32`. Externals: `[0]=x [1,rows]`,
/// `[1]=packed [rows·ceil(cols/16)] (u32-as-f32 codewords)`, `[2]=scale [rows]`.
/// `w_f32` is `[rows, cols]`, so `y = x[1,rows] · w[rows,cols] = [1,cols]`.
pub fn dequant_matmul_graph(rows: u32, cols: u32) -> Result<ComputeGraph, ForgeError> {
    use crate::wgsl_forge::ir::graph::DType as D;
    let mut g = ComputeGraph::new();
    let s = Schedule::default();
    let k_words = cols.div_ceil(16);
    let sh_x = Shape::new(&[1, rows]);
    let sh_packed = Shape::new(&[rows * k_words]);
    let sh_scale = Shape::new(&[rows]);
    let sh_w = Shape::new(&[rows, cols]);
    let sh_y = Shape::new(&[1, cols]);
    let x = TensorRef::input(0, sh_x, D::F32);
    let packed = TensorRef::input(1, sh_packed, D::F32);
    let scale = TensorRef::input(2, sh_scale, D::F32);
    let w = g.push(
        OpNode::GatherDequant { scheme: D::Ternary, block: cols },
        &[packed, scale],
        sh_w,
        D::F32,
        s,
    )?;
    let y = g.push(
        OpNode::MatMul { m: 1, n: cols, k: rows, tc: false, trans_b: false },
        &[x, w],
        sh_y,
        D::F32,
        s,
    )?;
    g.mark_output(y);
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

    // ── P4b: attention + GatherDequant + decode block ────────────────────────────────

    /// Row-major matmul helper for the test references.
    fn ref_mm(a: &[f32], b: &[f32], m: usize, k: usize, n: usize) -> Vec<f32> {
        let mut c = vec![0.0f32; m * n];
        for i in 0..m {
            for j in 0..n {
                let mut acc = 0.0f32;
                for kk in 0..k {
                    acc += a[i * k + kk] * b[kk * n + j];
                }
                c[i * n + j] = acc;
            }
        }
        c
    }

    fn ref_softmax(s: &[f32]) -> Vec<f32> {
        let m = s.iter().cloned().fold(f32::MIN, f32::max);
        let e: Vec<f32> = s.iter().map(|&v| (v - m).exp()).collect();
        let z: f32 = e.iter().sum();
        e.iter().map(|&x| x / z).collect()
    }

    fn ref_rmsnorm(x: &[f32], eps: f32) -> Vec<f32> {
        let ms = x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32;
        let inv = (ms + eps).sqrt().recip();
        x.iter().map(|&v| v * inv).collect()
    }

    /// The decode-step **scaled** attention graph's composed CPU oracle matches an independent
    /// `softmax((q·Kᵀ)/√d)·V` reference — with the 1/√d score scaling.
    #[test]
    fn attention_cpu_oracle_matches_reference() {
        let (d, kv) = (4usize, 6usize);
        let inv_scale = 1.0f32 / (d as f32).sqrt();
        let q: Vec<f32> = (0..d).map(|i| (i as f32) * 0.2 - 0.3).collect();
        let kt: Vec<f32> = (0..d * kv).map(|i| ((i * 5 % 7) as f32) * 0.1 - 0.25).collect();
        let v: Vec<f32> = (0..kv * d).map(|i| ((i * 3 % 5) as f32) * 0.15 - 0.2).collect();
        let g = attention_graph(d as u32, kv as u32).unwrap();
        let out = execute_graph_cpu(&g, &[q.clone(), kt.clone(), v.clone(), vec![inv_scale]]).unwrap();
        // Reference: scores = (q·Kᵀ)/√d [1,kv]; probs = softmax(scores); out = probs·V [1,d].
        let scores: Vec<f32> = ref_mm(&q, &kt, 1, d, kv).iter().map(|s| s * inv_scale).collect();
        let probs = ref_softmax(&scores);
        let want = ref_mm(&probs, &v, 1, kv, d);
        assert_eq!(out.len(), d);
        for (o, w) in out.iter().zip(&want) {
            assert!((o - w).abs() < 1e-5, "{o} vs {w}");
        }
    }

    /// The `{GatherDequant → MatMul}` graph dequantizes a ternary weight on the fly and
    /// matmuls it; its CPU oracle matches `x · (scale ⊙ vals)` from the *known* ternary
    /// values (an independent reference, not the same unpack code).
    #[test]
    fn dequant_matmul_cpu_oracle_matches_reference() {
        use crate::wgsl_forge::graph_ops::gather_dequant::pack_ternary_as_words;
        let (rows, cols) = (5usize, 8usize);
        let vals: Vec<f32> = (0..rows * cols)
            .map(|i| match (i * 7) % 3 {
                0 => 1.0,
                1 => -1.0,
                _ => 0.0,
            })
            .collect();
        let scale: Vec<f32> = (0..rows).map(|r| 0.5 + r as f32 * 0.1).collect();
        let packed = pack_ternary_as_words(&vals, rows, cols);
        let x: Vec<f32> = (0..rows).map(|i| (i as f32) * 0.3 - 0.6).collect();
        let g = dequant_matmul_graph(rows as u32, cols as u32).unwrap();
        let out = execute_graph_cpu(&g, &[x.clone(), packed, scale.clone()]).unwrap();
        // Independent reference W[r,c] = scale[r]*vals[r,c]; y = x·W.
        let w: Vec<f32> = (0..rows * cols)
            .map(|i| scale[i / cols] * vals[i])
            .collect();
        let want = ref_mm(&x, &w, 1, rows, cols);
        assert_eq!(out.len(), cols);
        for (o, r) in out.iter().zip(&want) {
            assert!((o - r).abs() < 1e-5, "{o} vs {r}");
        }
    }

    /// The full decode-block graph's composed CPU oracle matches an independent
    /// `x + attn(RMSNorm(x)); + SwiGLU(RMSNorm(·))` reference (both residuals).
    #[test]
    fn decode_block_cpu_oracle_matches_reference() {
        let (d, kv, ffn) = (4usize, 5usize, 6usize);
        let inv_scale = 1.0f32 / (d as f32).sqrt();
        let eps = 1e-5f32;
        let x: Vec<f32> = (0..d).map(|i| (i as f32) * 0.2 - 0.3).collect();
        let kt: Vec<f32> = (0..d * kv).map(|i| ((i * 5 % 7) as f32) * 0.1 - 0.25).collect();
        let v: Vec<f32> = (0..kv * d).map(|i| ((i * 3 % 5) as f32) * 0.15 - 0.2).collect();
        let wg: Vec<f32> = (0..d * ffn).map(|i| ((i % 11) as f32) * 0.03 - 0.15).collect();
        let wu: Vec<f32> = (0..d * ffn).map(|i| ((i % 7) as f32) * 0.02 - 0.07).collect();
        let wd: Vec<f32> = (0..ffn * d).map(|i| ((i % 5) as f32) * 0.04 - 0.08).collect();
        let g = decode_block_graph(d as u32, kv as u32, ffn as u32).unwrap();
        let ext = vec![
            x.clone(), kt.clone(), v.clone(), wg.clone(), wu.clone(), wd.clone(),
            vec![inv_scale], vec![eps],
        ];
        let out = execute_graph_cpu(&g, &ext).unwrap();

        // Reference (with 1/√d attention scale + RMSNorm eps).
        let n1 = ref_rmsnorm(&x, eps);
        let scores: Vec<f32> = ref_mm(&n1, &kt, 1, d, kv).iter().map(|s| s * inv_scale).collect();
        let probs = ref_softmax(&scores);
        let attn = ref_mm(&probs, &v, 1, kv, d);
        let res1: Vec<f32> = x.iter().zip(&attn).map(|(a, b)| a + b).collect();
        let n2 = ref_rmsnorm(&res1, eps);
        let gate = ref_mm(&n2, &wg, 1, d, ffn);
        let up = ref_mm(&n2, &wu, 1, d, ffn);
        let h: Vec<f32> = gate
            .iter()
            .zip(&up)
            .map(|(&gv, &uv)| (gv / (1.0 + (-gv).exp())) * uv)
            .collect();
        let ffn_out = ref_mm(&h, &wd, 1, ffn, d);
        let want: Vec<f32> = res1.iter().zip(&ffn_out).map(|(a, b)| a + b).collect();

        assert_eq!(out.len(), d);
        for (o, w) in out.iter().zip(&want) {
            assert!((o - w).abs() < 1e-5, "{o} vs {w}");
        }
    }

    /// GPU certify (A2000): attention, `{GatherDequant→MatMul}`, and the full decode block —
    /// each executed device-side — match their composed CPU oracle within f32 tolerance.
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn p4b_graphs_gpu_match_cpu_oracle() {
        use crate::wgsl_forge::graph_ops::gather_dequant::pack_ternary_as_words;
        // Attention (decode).
        {
            let (d, kv) = (64usize, 96usize);
            let q: Vec<f32> = (0..d).map(|i| ((i * 7 % 23) as f32) * 0.05 - 0.5).collect();
            let kt: Vec<f32> = (0..d * kv).map(|i| ((i % 19) as f32) * 0.02 - 0.18).collect();
            let v: Vec<f32> = (0..kv * d).map(|i| ((i % 13) as f32) * 0.03 - 0.18).collect();
            let g = attention_graph(d as u32, kv as u32).unwrap();
            let ext = vec![q, kt, v, vec![1.0f32 / (d as f32).sqrt()]];
            let gpu = execute_graph(&g, &ext).expect("attn gpu");
            let cpu = execute_graph_cpu(&g, &ext).unwrap();
            for (a, b) in gpu.iter().zip(&cpu) {
                assert!((a - b).abs() <= 1e-3 * b.abs().max(1.0), "attn: {a} vs {b}");
            }
        }
        // {GatherDequant → MatMul}.
        {
            let (rows, cols) = (48usize, 64usize);
            let vals: Vec<f32> = (0..rows * cols)
                .map(|i| match (i * 7) % 3 { 0 => 1.0, 1 => -1.0, _ => 0.0 })
                .collect();
            let scale: Vec<f32> = (0..rows).map(|r| 0.25 + (r % 5) as f32 * 0.1).collect();
            let packed = pack_ternary_as_words(&vals, rows, cols);
            let x: Vec<f32> = (0..rows).map(|i| ((i % 9) as f32) * 0.1 - 0.4).collect();
            let g = dequant_matmul_graph(rows as u32, cols as u32).unwrap();
            let ext = vec![x, packed, scale];
            let gpu = execute_graph(&g, &ext).expect("dequant gpu");
            let cpu = execute_graph_cpu(&g, &ext).unwrap();
            for (a, b) in gpu.iter().zip(&cpu) {
                assert!((a - b).abs() <= 1e-3 * b.abs().max(1.0), "dequant: {a} vs {b}");
            }
        }
        // Full decode block.
        {
            let (d, kv, ffn) = (64u32, 80u32, 128u32);
            let mk = |n: usize, m: u32| (0..n).map(|i| ((i as u32 % m) as f32) * 0.01 - 0.1).collect::<Vec<f32>>();
            let ext = vec![
                mk(d as usize, 17),
                mk((d * kv) as usize, 19),
                mk((kv * d) as usize, 13),
                mk((d * ffn) as usize, 11),
                mk((d * ffn) as usize, 7),
                mk((ffn * d) as usize, 5),
                vec![1.0f32 / (d as f32).sqrt()],
                vec![1e-5f32],
            ];
            let g = decode_block_graph(d, kv, ffn).unwrap();
            let gpu = execute_graph(&g, &ext).expect("decode gpu");
            let cpu = execute_graph_cpu(&g, &ext).unwrap();
            assert_eq!(gpu.len(), cpu.len());
            for (a, b) in gpu.iter().zip(&cpu) {
                assert!((a - b).abs() <= 1e-2 * b.abs().max(1.0), "decode: {a} vs {b}");
            }
        }
    }

    /// **Honest kernel-level uplift benchmark** — times one decode-block graph (≈SmolLM2-360M
    /// dims: d=576, kv=128, ffn=1536) executed on the GPU vs the composed CPU oracle. Reports
    /// wall-clock per call for two GPU paths so the throughput pass is attributable:
    /// - **reused** — one [`ForgeGraphExecutor`] held across calls (`run` per step): context
    ///   reuse **+** single-encoder deferred submit (the realistic decode-step usage);
    /// - **one-shot** — `execute_graph` per call (fresh device/slab each call): single-encoder
    ///   submit but no context reuse, so `reused` vs `one-shot` isolates the device-creation cost.
    ///
    /// **Caveats (do not over-read):** this is ONE decode block, not a full L-layer model; it is
    /// **not** end-to-end tokens/sec and does not include sampling, KV-cache management, or
    /// host↔device transfer beyond the final readback. The `reused` path now records the whole
    /// graph into one encoder + one submit **and** compiles each node's pipeline only once (the
    /// context-level pipeline cache), so the warmup run pays compilation and the timed loop pays
    /// only bind-group build + dispatch + readback — the realistic held-executor decode step.
    #[test]
    #[ignore = "benchmark; requires a GPU adapter. Run with --nocapture to see timings."]
    fn decode_block_kernel_uplift_bench() {
        use std::time::Instant;
        let (d, kv, ffn) = (576u32, 128u32, 1536u32);
        let mk = |n: usize, m: u32, off: f32| {
            (0..n).map(|i| ((i as u32 % m) as f32) * 0.001 - off).collect::<Vec<f32>>()
        };
        let ext = vec![
            mk(d as usize, 97, 0.05),
            mk((d * kv) as usize, 89, 0.04),
            mk((kv * d) as usize, 83, 0.04),
            mk((d * ffn) as usize, 79, 0.03),
            mk((d * ffn) as usize, 73, 0.03),
            mk((ffn * d) as usize, 71, 0.03),
            vec![1.0f32 / (d as f32).sqrt()],
            vec![1e-5f32],
        ];
        let g = decode_block_graph(d, kv, ffn).unwrap();
        let nodes = g.nodes.len();
        let iters = 20;

        // ── Reused executor: context reuse + single-encoder submit (the decode-step path) ──
        let mut exec = ForgeGraphExecutor::new().expect("executor");
        let _ = exec.run(&g, &ext).expect("warmup reused"); // shader compile + first dispatch
        let t0 = Instant::now();
        for _ in 0..iters {
            let _ = exec.run(&g, &ext).expect("gpu reused");
        }
        let gpu_reuse_ms = t0.elapsed().as_secs_f64() * 1e3 / iters as f64;

        // ── One-shot: fresh device/slab per call (single-encoder submit, no context reuse) ──
        let _ = execute_graph(&g, &ext).expect("warmup one-shot");
        let t1 = Instant::now();
        for _ in 0..iters {
            let _ = execute_graph(&g, &ext).expect("gpu one-shot");
        }
        let gpu_oneshot_ms = t1.elapsed().as_secs_f64() * 1e3 / iters as f64;

        let c0 = Instant::now();
        for _ in 0..iters {
            let _ = execute_graph_cpu(&g, &ext).expect("cpu");
        }
        let cpu_ms = c0.elapsed().as_secs_f64() * 1e3 / iters as f64;

        let cached = exec.context().cached_pipeline_count();
        eprintln!(
            "[decode-block uplift] d={d} kv={kv} ffn={ffn} nodes={nodes} cached_pipelines={cached} | \
             GPU reused {gpu_reuse_ms:.3} ms/call (ctx reuse + 1 encoder + pipeline cache; ~{:.3} ms/node) | \
             GPU one-shot {gpu_oneshot_ms:.3} ms/call (fresh device/slab per call) | \
             CPU oracle {cpu_ms:.3} ms/call | ratio (reused vs CPU) {:.2}x. \
             NOT end-to-end tok/s; one block, not L layers.",
            gpu_reuse_ms / nodes as f64,
            cpu_ms / gpu_reuse_ms,
        );
    }

    /// The context-level pipeline cache amortizes shader compilation across `run()` calls: a
    /// held [`ForgeGraphExecutor`] re-running the same graph compiles each distinct node kernel
    /// exactly once, so the cache count is **stable** after the first run (and bounded by the
    /// number of distinct kernels, well below the node count for a decode block with repeated
    /// op-classes). This is what turns the per-call compile cost into a one-time warmup.
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn pipeline_cache_amortizes_across_runs() {
        let (d, kv, ffn) = (64u32, 80u32, 128u32);
        let mk = |n: usize, m: u32| (0..n).map(|i| ((i as u32 % m) as f32) * 0.01 - 0.1).collect::<Vec<f32>>();
        let ext = vec![
            mk(d as usize, 17), mk((d * kv) as usize, 19), mk((kv * d) as usize, 13),
            mk((d * ffn) as usize, 11), mk((d * ffn) as usize, 7), mk((ffn * d) as usize, 5),
            vec![1.0f32 / (d as f32).sqrt()], vec![1e-5f32],
        ];
        let g = decode_block_graph(d, kv, ffn).unwrap();
        let mut exec = ForgeGraphExecutor::new().expect("executor");
        let _ = exec.run(&g, &ext).expect("run 1");
        let after_first = exec.context().cached_pipeline_count();
        let _ = exec.run(&g, &ext).expect("run 2");
        let after_second = exec.context().cached_pipeline_count();
        // Stable: the second run compiled nothing new.
        assert_eq!(after_first, after_second, "cache must be stable across runs");
        // Distinct kernels < node count (repeated op-classes share a pipeline).
        assert!(after_first > 0 && after_first <= g.nodes.len(), "cached={after_first} nodes={}", g.nodes.len());
        // The result is unchanged across runs (cache returns the same pipeline).
        let a = exec.run(&g, &ext).unwrap();
        let b = exec.run(&g, &ext).unwrap();
        assert_eq!(a, b, "cached runs must be deterministic");
    }
}
