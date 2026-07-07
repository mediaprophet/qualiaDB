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

use super::{broadcast, elementwise, gather_dequant, reduce, slice, stencil};
use crate::wgsl_forge::execute::{BindingUsage, BufferView, GraphPass, WgpuComputeContext};
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
            OpNode::MatMul {
                m, n, k, trans_b, ..
            } => {
                if trans_b {
                    // C[m,n] = A[m,k] · Bᵀ, B stored [n,k] row-major (native [out,in] weight layout).
                    let (mm, nn, kk) = (m as usize, n as usize, k as usize);
                    let (a, b) = (&ins[0], &ins[1]);
                    let mut c = vec![0.0f32; mm * nn];
                    for i in 0..mm {
                        for j in 0..nn {
                            let mut acc = 0.0f32;
                            for x in 0..kk {
                                acc += a[i * kk + x] * b[j * kk + x];
                            }
                            c[i * nn + j] = acc;
                        }
                    }
                    c
                } else {
                    crate::wgsl_forge::oracle::gemm_cpu(
                        &ins[0], &ins[1], m as usize, k as usize, n as usize,
                    )
                }
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
            OpNode::Slice { offset, len } => {
                slice::slice_cpu(&ins[0], offset as usize, len as usize)
            }
            OpNode::Rope {
                head_dim,
                pos,
                mode,
                base_bits,
            } => {
                let rope_mode = if mode == 0 {
                    stencil::RopeMode::Interleaved
                } else {
                    stencil::RopeMode::Neox
                };
                let cfg = stencil::RopeConfig {
                    head_dim,
                    pos,
                    mode: rope_mode,
                    theta_base: f32::from_bits(base_bits),
                };
                stencil::rope_cpu(&ins[0], &cfg)?
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
pub fn execute_graph(graph: &ComputeGraph, externals: &[Vec<f32>]) -> Result<Vec<f32>, ForgeError> {
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
    /// Create an executor with the default per-slab capacity ([`EXEC_CAPACITY`]). Acquires its
    /// **own** GPU adapter + device once; reuse the returned value across calls/decode steps.
    /// To instead run on the process-wide LLM device, use [`Self::on_shared_gpu`].
    pub fn new() -> Result<Self, ForgeError> {
        Self::with_capacity(EXEC_CAPACITY)
    }

    /// Create an executor with an explicit per-slab capacity in bytes, on its own device.
    pub fn with_capacity(capacity_bytes: usize) -> Result<Self, ForgeError> {
        Ok(Self::with_context(WgpuComputeContext::new(capacity_bytes)?))
    }

    /// Wrap an executor around a **caller-supplied** context — e.g. one built on the process-wide
    /// [`crate::gpu_context::shared_gpu`] device via [`WgpuComputeContext::from_device`]. This is
    /// how the forge is made to run on the SAME device that owns the resident LLM weights / KV
    /// cache instead of spinning up a second device (LLM-on-forge plan, Phase 1a).
    pub fn with_context(ctx: WgpuComputeContext) -> Self {
        Self { ctx }
    }

    /// Build an executor on the process-wide shared GPU device ([`crate::gpu_context::shared_gpu`]),
    /// at the default per-slab capacity, so the forge shares the device that owns LLM weights +
    /// KV cache. Native only (`shared_gpu` is host-side). The keystone entry point for decode-on-forge.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn on_shared_gpu() -> Result<Self, ForgeError> {
        Self::on_shared_gpu_with_capacity(EXEC_CAPACITY)
    }

    /// [`Self::on_shared_gpu`] with an explicit per-slab capacity in bytes.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn on_shared_gpu_with_capacity(capacity_bytes: usize) -> Result<Self, ForgeError> {
        let shared = crate::gpu_context::shared_gpu();
        let ctx = WgpuComputeContext::from_device(
            shared.device.clone(),
            shared.queue.clone(),
            &shared.adapter_caps,
            capacity_bytes,
        )?;
        Ok(Self::with_context(ctx))
    }

    /// Borrow the underlying context (e.g. to read adapter identity / profile).
    pub fn context(&self) -> &WgpuComputeContext {
        &self.ctx
    }

    /// Execute `graph` on the GPU with `externals` as the graph inputs, returning the final
    /// output tensor. Intermediates are kept device-side; every node's dispatch and its GPU→GPU
    /// hand-off copy are recorded into ONE command encoder and submitted once
    /// ([`WgpuComputeContext::submit_graph`], Option B). Re-uses the device/slab across calls.
    /// **Every** external is (re)uploaded into the transient ring this call; for the decode loop
    /// where the big matrices are constant across tokens, use [`Self::load_weights`] +
    /// [`Self::run_resident`] instead so they upload once.
    pub fn run(
        &mut self,
        graph: &ComputeGraph,
        externals: &[Vec<f32>],
    ) -> Result<Vec<f32>, ForgeError> {
        // Per-call slab reset: free the previous call's transient allocations so this call starts
        // with the full capacity. Safe — the previous run's readback fully synchronized the device.
        self.ctx.clear_transient_allocations();

        // Upload every external into the transient READ slab (binding overwritten per consumer).
        let mut ext_views: Vec<BufferView> = Vec::with_capacity(externals.len());
        for data in externals {
            ext_views.push(self.ctx.allocate_and_write(
                bytemuck::cast_slice(data),
                0,
                0,
                BindingUsage::StorageRead,
            )?);
        }
        self.run_prepared(graph, ext_views)
    }

    /// Like [`Self::run`], but external indices already uploaded into the persistent weight region
    /// (via [`Self::load_weights`]) are bound to their **resident** on-device buffers instead of
    /// being re-uploaded — only the *activation* externals (those not in `resident`) are written
    /// this call. This is the decode-step usage: the big projection / FFN matrices live on-device
    /// across every token; each token uploads just `x` (+ tiny scalars). `externals[i]` is ignored
    /// for any `i` that is resident (pass an empty `vec![]` there for clarity).
    pub fn run_resident(
        &mut self,
        graph: &ComputeGraph,
        externals: &[Vec<f32>],
        resident: &ResidentWeights,
    ) -> Result<Vec<f32>, ForgeError> {
        self.ctx.clear_transient_allocations();
        let mut ext_views: Vec<BufferView> = Vec::with_capacity(externals.len());
        for (i, data) in externals.iter().enumerate() {
            if let Some(view) = resident.view_for(i) {
                ext_views.push(view);
            } else {
                ext_views.push(self.ctx.allocate_and_write(
                    bytemuck::cast_slice(data),
                    0,
                    0,
                    BindingUsage::StorageRead,
                )?);
            }
        }
        self.run_prepared(graph, ext_views)
    }

    /// Upload a set of `(external_index, data)` weights ONCE into the executor's persistent weight
    /// region and return a [`ResidentWeights`] handle mapping those indices to their on-device
    /// views. Additive — successive calls accumulate (use [`Self::clear_weights`] to start over).
    /// Pass the handle to [`Self::run_resident`] so those externals are referenced by offset, not
    /// re-uploaded per token (the LLM-on-forge weight-residency lever).
    pub fn load_weights(
        &mut self,
        weights: &[(usize, Vec<f32>)],
    ) -> Result<ResidentWeights, ForgeError> {
        let mut views = std::collections::HashMap::with_capacity(weights.len());
        for (idx, data) in weights {
            let view = self.ctx.allocate_weight(bytemuck::cast_slice(data), 0, 0)?;
            views.insert(*idx, view);
        }
        Ok(ResidentWeights { views })
    }

    /// Drop all resident weights, freeing the persistent region for a new model/layer set. Any
    /// [`ResidentWeights`] handles from before this call are stale and must not be reused.
    pub fn clear_weights(&mut self) {
        self.ctx.clear_weights();
    }

    /// The shared execution core: given the externals already resolved to device [`BufferView`]s
    /// (freshly uploaded and/or resident), prepare every node, record the whole graph into one
    /// encoder, submit once, and read back the output. Both [`Self::run`] and [`Self::run_resident`]
    /// funnel through here so the residency path and the upload path share identical scheduling.
    fn run_prepared(
        &mut self,
        graph: &ComputeGraph,
        ext_views: Vec<BufferView>,
    ) -> Result<Vec<f32>, ForgeError> {
        let ctx = &mut self.ctx;

        // Phase A — prepare every node: allocate its output + params (read/read_write slab split),
        // compile its pipeline, build its bind group. No GPU work submitted yet. Each node's output
        // (the read-slab hand-off copy) is threaded forward as a `BufferView`.
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

/// A handle to weights uploaded once into a [`ForgeGraphExecutor`]'s persistent weight region by
/// [`ForgeGraphExecutor::load_weights`]. Maps a graph's **external input index** to its resident
/// on-device [`BufferView`]; passed to [`ForgeGraphExecutor::run_resident`] so those inputs are
/// referenced by offset across many runs instead of re-uploaded per call.
#[derive(Debug, Clone, Default)]
pub struct ResidentWeights {
    views: std::collections::HashMap<usize, BufferView>,
}

impl ResidentWeights {
    /// The resident device view for external index `i`, if it was loaded.
    pub fn view_for(&self, i: usize) -> Option<BufferView> {
        self.views.get(&i).copied()
    }
    /// Number of resident weight tensors held.
    pub fn len(&self) -> usize {
        self.views.len()
    }
    pub fn is_empty(&self) -> bool {
        self.views.is_empty()
    }
}

/// Allocate a zeroed `n`-element f32 output buffer at `binding` (read_write slab).
fn alloc_out(
    ctx: &mut WgpuComputeContext,
    n: usize,
    binding: u32,
) -> Result<BufferView, ForgeError> {
    let zeros = vec![0.0f32; n.max(1)];
    ctx.allocate_and_write(
        bytemuck::cast_slice(&zeros),
        binding,
        0,
        BindingUsage::StorageReadWrite,
    )
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
    let pass = GraphPass {
        pipeline,
        bind_group,
        workgroups,
        copy: Some((out, read_copy)),
    };
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
            let sched = Schedule {
                workgroup_size: WG,
                ..Default::default()
            };
            // element_count == WG → one workgroup (the reduce is single-workgroup).
            record_kernel(
                ctx,
                &src,
                reduce::REDUCE_ENTRY,
                &[at(ins[0], 0), out, params],
                out,
                sched,
                WG as usize,
            )
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
            let sched = Schedule {
                workgroup_size: WG,
                ..Default::default()
            };
            record_kernel(
                ctx,
                &src,
                broadcast::BROADCAST_ENTRY,
                &[at(ins[0], 0), out, params],
                out,
                sched,
                out_len,
            )
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
            let sched = Schedule {
                workgroup_size: WG,
                ..Default::default()
            };
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
        OpNode::MatMul {
            m,
            n,
            k,
            tc,
            trans_b,
        } => {
            // `trans_b` consumes B as `[n,k]` row-major (the native GGUF/p64 weight layout
            // `[out,in]`), computing `C[m,n] = A[m,k] · Bᵀ` with no transpose copy — this is what
            // lets the forge feed on p64 projection weights directly. `tc=true` requests tensor
            // cores: the portable wgpu coopmat tiled GEMM, taken only when the adapter advertises
            // coopmat AND the runtime probe confirms it computes (dormant on wgpu 29.0.3 / #9741,
            // falling to the certified plain GEMM floor). The CUDA WMMA path is reached host-side
            // via `dispatch::gemm_f32_tc` / the CudaCLowerer (P5), not this wgpu executor.
            if trans_b {
                prepare_matmul_trans_b(ctx, m, n, k, ins)
            } else if tc
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
        OpNode::Slice { offset, len } => {
            const WG: u32 = 64;
            let out_len = len as usize;
            let out = alloc_out(ctx, out_len, 1)?;
            let params = alloc_params(ctx, &[len, offset, 0, 0], 2, BindingUsage::StorageRead)?;
            let src = slice::slice_wgsl(WG);
            let sched = Schedule {
                workgroup_size: WG,
                ..Default::default()
            };
            // input (0), output (1, read_write), params (2).
            record_kernel(
                ctx,
                &src,
                slice::SLICE_ENTRY,
                &[at(ins[0], 0), out, params],
                out,
                sched,
                out_len,
            )
        }
        OpNode::Rope {
            head_dim,
            pos,
            mode,
            base_bits,
        } => {
            const WG: u32 = 64;
            let n = elems(&ins[0]);
            let out = alloc_out(ctx, n, 1)?;
            // params = [n, head_dim, pos, mode, theta_base_bits] (the real RoPE block). `pos` lives
            // in the buffer, not the source, so the pipeline cache stays warm across tokens.
            let params = alloc_params(
                ctx,
                &[n as u32, head_dim, pos, mode, base_bits],
                2,
                BindingUsage::StorageRead,
            )?;
            let src = stencil::rope_wgsl(WG)?;
            let sched = Schedule {
                workgroup_size: WG,
                ..Default::default()
            };
            record_kernel(
                ctx,
                &src,
                stencil::STENCIL_ENTRY,
                &[at(ins[0], 0), out, params],
                out,
                sched,
                n,
            )
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
    let sched = Schedule {
        workgroup_size: WG,
        ..Default::default()
    };
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

/// WGSL for `C[m,n] = A[m,k] · Bᵀ` with **B stored `[n,k]` row-major** (the native GGUF/p64 weight
/// layout `[out,in]`). One invocation per output element. Binding ABI: `a`(0), `b`(1), `c`(2,
/// read_write), `dims`(3, read storage `[m,n,k,_]`).
const GEMM_TRANS_B_ENTRY: &str = "gemm_trans_b_main";
fn gemm_trans_b_wgsl(wg: u32) -> String {
    format!(
        r#"@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> c: array<f32>;
@group(0) @binding(3) var<storage, read> dims: array<u32>;
@compute @workgroup_size({wg})
fn {entry}(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let idx = gid.x;
    let m = dims[0]; let n = dims[1]; let k = dims[2];
    if (idx >= m * n) {{ return; }}
    let row = idx / n; let col = idx % n;
    var acc = 0.0;
    for (var kk = 0u; kk < k; kk = kk + 1u) {{
        acc = acc + a[row * k + kk] * b[col * k + kk];
    }}
    c[idx] = acc;
}}
"#,
        entry = GEMM_TRANS_B_ENTRY,
    )
}

/// Transposed-B GEMM node: `C[m,n] = A[m,k] · Bᵀ`, B bound as `[n,k]` row-major. Lets the forge
/// consume native `[out,in]` GGUF/p64 projection weights without a host transpose.
fn prepare_matmul_trans_b(
    ctx: &mut WgpuComputeContext,
    m: u32,
    n: u32,
    k: u32,
    ins: &[BufferView],
) -> Result<(GraphPass, BufferView), ForgeError> {
    const WG: u32 = 64;
    let out_elems = (m as usize) * (n as usize);
    let out = alloc_out(ctx, out_elems, 2)?;
    let dims = alloc_params(ctx, &[m, n, k, 0], 3, BindingUsage::StorageRead)?;
    let src = gemm_trans_b_wgsl(WG);
    let sched = Schedule {
        workgroup_size: WG,
        ..Default::default()
    };
    record_kernel(
        ctx,
        &src,
        GEMM_TRANS_B_ENTRY,
        &[at(ins[0], 0), at(ins[1], 1), out, dims],
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
    let sched = Schedule {
        workgroup_size: 32,
        ..Default::default()
    };
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
    let sched = Schedule {
        workgroup_size: WG,
        ..Default::default()
    };
    match n_in {
        1 => {
            let out = alloc_out(ctx, n, 1)?;
            let params = alloc_params(ctx, &[n as u32, 0, 0, 0], 2, BindingUsage::StorageRead)?;
            record_kernel(
                ctx,
                &src,
                elementwise::EWISE_ENTRY,
                &[at(ins[0], 0), out, params],
                out,
                sched,
                n,
            )
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
    let mx = g.push(
        OpNode::Reduce {
            op: RedKind::Max,
            axis: Axis::Last,
        },
        &[x],
        sh_1,
        DType::F32,
        s,
    )?;
    let mxb = g.push(
        OpNode::Broadcast { shape: sh_n },
        &[mx],
        sh_n,
        DType::F32,
        s,
    )?;
    let shifted = g.push(
        OpNode::Elementwise { f: EwKind::Sub },
        &[x, mxb],
        sh_n,
        DType::F32,
        s,
    )?;
    let e = g.push(
        OpNode::Elementwise { f: EwKind::Exp },
        &[shifted],
        sh_n,
        DType::F32,
        s,
    )?;
    let sm = g.push(
        OpNode::Reduce {
            op: RedKind::Sum,
            axis: Axis::Last,
        },
        &[e],
        sh_1,
        DType::F32,
        s,
    )?;
    let smb = g.push(
        OpNode::Broadcast { shape: sh_n },
        &[sm],
        sh_n,
        DType::F32,
        s,
    )?;
    let out = g.push(
        OpNode::Elementwise { f: EwKind::Div },
        &[e, smb],
        sh_n,
        DType::F32,
        s,
    )?;
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
    let sq = g.push(
        OpNode::Elementwise { f: EwKind::Mul },
        &[x, x],
        sh_n,
        DType::F32,
        s,
    )?;
    let ms = g.push(
        OpNode::Reduce {
            op: RedKind::Mean,
            axis: Axis::Last,
        },
        &[sq],
        sh_1,
        DType::F32,
        s,
    )?;
    let r = g.push(
        OpNode::Elementwise {
            f: EwKind::RecipSqrt,
        },
        &[ms],
        sh_1,
        DType::F32,
        s,
    )?;
    let rb = g.push(OpNode::Broadcast { shape: sh_n }, &[r], sh_n, DType::F32, s)?;
    let out = g.push(
        OpNode::Elementwise { f: EwKind::Mul },
        &[x, rb],
        sh_n,
        DType::F32,
        s,
    )?;
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
    let mm = |m, n, k| OpNode::MatMul {
        m,
        n,
        k,
        tc: false,
        trans_b: false,
    };
    let gate = g.push(mm(seq, ffn, dim), &[x, wg], sh_h, DType::F32, s)?;
    let up = g.push(mm(seq, ffn, dim), &[x, wu], sh_h, DType::F32, s)?;
    let sg = g.push(
        OpNode::Elementwise { f: EwKind::Silu },
        &[gate],
        sh_h,
        DType::F32,
        s,
    )?;
    let h = g.push(
        OpNode::Elementwise { f: EwKind::Mul },
        &[sg, up],
        sh_h,
        DType::F32,
        s,
    )?;
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
    let sq = g.push(
        OpNode::Elementwise { f: EwKind::Mul },
        &[x, x],
        sh_row,
        DType::F32,
        s,
    )?;
    let ms = g.push(
        OpNode::Reduce {
            op: RedKind::Mean,
            axis: Axis::Last,
        },
        &[sq],
        sh_1,
        DType::F32,
        s,
    )?;
    let ms_eps = g.push(
        OpNode::Elementwise { f: EwKind::Add },
        &[ms, eps_ref],
        sh_1,
        DType::F32,
        s,
    )?;
    let r = g.push(
        OpNode::Elementwise {
            f: EwKind::RecipSqrt,
        },
        &[ms_eps],
        sh_1,
        DType::F32,
        s,
    )?;
    let rb = g.push(
        OpNode::Broadcast { shape: sh_row },
        &[r],
        sh_row,
        DType::F32,
        s,
    )?;
    g.push(
        OpNode::Elementwise { f: EwKind::Mul },
        &[x, rb],
        sh_row,
        DType::F32,
        s,
    )
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
    let mx = g.push(
        OpNode::Reduce {
            op: RedKind::Max,
            axis: Axis::Last,
        },
        &[scores],
        sh_1,
        DType::F32,
        s,
    )?;
    let mxb = g.push(
        OpNode::Broadcast { shape: sh_vec },
        &[mx],
        sh_vec,
        DType::F32,
        s,
    )?;
    let shifted = g.push(
        OpNode::Elementwise { f: EwKind::Sub },
        &[scores, mxb],
        sh_vec,
        DType::F32,
        s,
    )?;
    let e = g.push(
        OpNode::Elementwise { f: EwKind::Exp },
        &[shifted],
        sh_vec,
        DType::F32,
        s,
    )?;
    let sm = g.push(
        OpNode::Reduce {
            op: RedKind::Sum,
            axis: Axis::Last,
        },
        &[e],
        sh_1,
        DType::F32,
        s,
    )?;
    let smb = g.push(
        OpNode::Broadcast { shape: sh_vec },
        &[sm],
        sh_vec,
        DType::F32,
        s,
    )?;
    g.push(
        OpNode::Elementwise { f: EwKind::Div },
        &[e, smb],
        sh_vec,
        DType::F32,
        s,
    )
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
    let mm = |m, n, k| OpNode::MatMul {
        m,
        n,
        k,
        tc: false,
        trans_b: false,
    };
    // scores = Q[1,d] · Kᵀ[d,kv] = [1,kv], scaled by 1/√d before softmax.
    let scores = g.push(mm(1, kv, d), &[q, kt], sh_scores, DType::F32, s)?;
    let inv_bc = g.push(
        OpNode::Broadcast { shape: sh_scores },
        &[inv_scale],
        sh_scores,
        DType::F32,
        s,
    )?;
    let scaled = g.push(
        OpNode::Elementwise { f: EwKind::Mul },
        &[scores, inv_bc],
        sh_scores,
        DType::F32,
        s,
    )?;
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
    let mm = |m, n, k| OpNode::MatMul {
        m,
        n,
        k,
        tc: false,
        trans_b: false,
    };

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
    let inv_bc = g.push(
        OpNode::Broadcast { shape: sh_scores },
        &[inv_scale],
        sh_scores,
        DType::F32,
        s,
    )?;
    let scaled = g.push(
        OpNode::Elementwise { f: EwKind::Mul },
        &[scores, inv_bc],
        sh_scores,
        DType::F32,
        s,
    )?;
    let probs = push_softmax(&mut g, scaled, sh_scores, sh_1, s)?;
    let attn = g.push(mm(1, d, kv), &[probs, v], sh_row, DType::F32, s)?;
    let res1 = g.push(
        OpNode::Elementwise { f: EwKind::Add },
        &[x, attn],
        sh_row,
        DType::F32,
        s,
    )?;

    // ── SwiGLU-FFN sub-block over RMSNorm(res1), residual back to res1 ──
    let n2 = push_rmsnorm(&mut g, res1, eps, sh_row, sh_1, s)?;
    let gate = g.push(mm(1, ffn, d), &[n2, wg], sh_h, DType::F32, s)?;
    let up = g.push(mm(1, ffn, d), &[n2, wu], sh_h, DType::F32, s)?;
    let sg = g.push(
        OpNode::Elementwise { f: EwKind::Silu },
        &[gate],
        sh_h,
        DType::F32,
        s,
    )?;
    let h = g.push(
        OpNode::Elementwise { f: EwKind::Mul },
        &[sg, up],
        sh_h,
        DType::F32,
        s,
    )?;
    let ffn_out = g.push(mm(1, d, ffn), &[h, wd], sh_row, DType::F32, s)?;
    let out = g.push(
        OpNode::Elementwise { f: EwKind::Add },
        &[res1, ffn_out],
        sh_row,
        DType::F32,
        s,
    )?;
    g.mark_output(out);
    Ok(g)
}

/// A **real multi-head (GQA) decode layer** over a KV cache of length `seq`, composed entirely from
/// forge ops: `RMSNorm(x) → Q-proj → RoPE(q) → per-head { slice q_h / Kᵀ_h / V_h ; scaled
/// softmax(q_h·Kᵀ_h)·V_h ; o_h·Wo_h } summed → +x → RMSNorm → SwiGLU-FFN → +`. K/V come from the
/// (head-major) cache externals — already RoPE'd, as the engine stores them; the current token's
/// K/V projection + cache append is the decode-loop integration step (the cache is mutable state
/// living at that seam, not in this functional graph). Grouped-query attention: each kv-head serves
/// `n_heads/n_kv_heads` query heads. Per-head slicing is contiguous because the cache is head-major.
///
/// RMSNorm carries its **learned weight** (`x·inv_rms·w`), as the real engine does. Cache layout:
/// `Kt` = `[n_kv_heads, head_dim, seq]` (transposed keys), `V` = `[n_kv_heads, seq, head_dim]`.
/// Externals: `[0]=x[1,d] [1]=Kt [2]=V [3]=Wq[d,d] [4]=Wo[d,d] [5]=Wg[d,ffn] [6]=Wu[d,ffn]
/// [7]=Wd[ffn,d] [8]=attn_norm[d] [9]=ffn_norm[d] [10]=inv_scale[1] [11]=eps[1]`, with
/// `d=n_heads·head_dim`. The projection weights are `[in,out]` row-major (the GGUF/p64 layout), i.e.
/// exactly `[k,n]` for the `MatMul(m=1,n=out,k=in)` here — no transpose. `rope_mode` is
/// 0=interleaved (GGUF NORM) / 1=NeoX; `pos` is the query's absolute position.
#[allow(clippy::too_many_arguments)]
pub fn decode_layer_graph(
    n_heads: u32,
    n_kv_heads: u32,
    head_dim: u32,
    seq: u32,
    ffn: u32,
    pos: u32,
    rope_mode: u32,
    theta_base: f32,
) -> Result<ComputeGraph, ForgeError> {
    if n_kv_heads == 0 || n_heads % n_kv_heads != 0 {
        return Err(ForgeError::Emission(format!(
            "decode_layer: n_heads {n_heads} must be a positive multiple of n_kv_heads {n_kv_heads}"
        )));
    }
    let d = n_heads * head_dim;
    let group = n_heads / n_kv_heads;
    let base_bits = theta_base.to_bits();
    let mut g = ComputeGraph::new();
    let s = Schedule::default();
    let sh_d = Shape::new(&[1, d]);
    let sh_1 = Shape::new(&[1]);
    let sh_hd = Shape::new(&[1, head_dim]);
    let sh_seq = Shape::new(&[1, seq]);
    let sh_ffn = Shape::new(&[1, ffn]);
    let mm = |m, n, k| OpNode::MatMul {
        m,
        n,
        k,
        tc: false,
        trans_b: false,
    };

    let x = TensorRef::input(0, sh_d, DType::F32);
    let kt = TensorRef::input(1, Shape::new(&[n_kv_heads * head_dim * seq]), DType::F32);
    let v = TensorRef::input(2, Shape::new(&[n_kv_heads * seq * head_dim]), DType::F32);
    let wq = TensorRef::input(3, Shape::new(&[d, d]), DType::F32);
    let wo = TensorRef::input(4, Shape::new(&[d, d]), DType::F32);
    let wg = TensorRef::input(5, Shape::new(&[d, ffn]), DType::F32);
    let wu = TensorRef::input(6, Shape::new(&[d, ffn]), DType::F32);
    let wd = TensorRef::input(7, Shape::new(&[ffn, d]), DType::F32);
    let attn_norm = TensorRef::input(8, sh_d, DType::F32);
    let ffn_norm = TensorRef::input(9, sh_d, DType::F32);
    let inv_scale = TensorRef::input(10, sh_1, DType::F32);
    let eps = TensorRef::input(11, sh_1, DType::F32);

    // Attention: RMSNorm·w(x) → Q-proj → RoPE(q) → per-head GQA attention → residual.
    let n1 = push_rmsnorm(&mut g, x, eps, sh_d, sh_1, s)?;
    let n1 = g.push(
        OpNode::Elementwise { f: EwKind::Mul },
        &[n1, attn_norm],
        sh_d,
        DType::F32,
        s,
    )?;
    let q = g.push(mm(1, d, d), &[n1, wq], sh_d, DType::F32, s)?;
    let q = g.push(
        OpNode::Rope {
            head_dim,
            pos,
            mode: rope_mode,
            base_bits,
        },
        &[q],
        sh_d,
        DType::F32,
        s,
    )?;

    let mut head_parts: Vec<TensorRef> = Vec::with_capacity(n_heads as usize);
    for h in 0..n_heads {
        let kh = h / group; // GQA: query head h reads kv-head kh.
        let q_h = g.push(
            OpNode::Slice {
                offset: h * head_dim,
                len: head_dim,
            },
            &[q],
            sh_hd,
            DType::F32,
            s,
        )?;
        let kt_h = g.push(
            OpNode::Slice {
                offset: kh * head_dim * seq,
                len: head_dim * seq,
            },
            &[kt],
            Shape::new(&[head_dim, seq]),
            DType::F32,
            s,
        )?;
        let v_h = g.push(
            OpNode::Slice {
                offset: kh * seq * head_dim,
                len: seq * head_dim,
            },
            &[v],
            Shape::new(&[seq, head_dim]),
            DType::F32,
            s,
        )?;
        // scores = q_h · Kᵀ_h [1,seq]; scaled by 1/√head_dim; softmax; · V_h → o_h [1,head_dim].
        let scores = g.push(mm(1, seq, head_dim), &[q_h, kt_h], sh_seq, DType::F32, s)?;
        let inv_bc = g.push(
            OpNode::Broadcast { shape: sh_seq },
            &[inv_scale],
            sh_seq,
            DType::F32,
            s,
        )?;
        let scaled = g.push(
            OpNode::Elementwise { f: EwKind::Mul },
            &[scores, inv_bc],
            sh_seq,
            DType::F32,
            s,
        )?;
        let probs = push_softmax(&mut g, scaled, sh_seq, sh_1, s)?;
        let o_h = g.push(mm(1, head_dim, seq), &[probs, v_h], sh_hd, DType::F32, s)?;
        // Output projection, per head: o_h · Wo[h·head_dim : (h+1)·head_dim, :] → [1,d]; summed.
        let wo_h = g.push(
            OpNode::Slice {
                offset: h * head_dim * d,
                len: head_dim * d,
            },
            &[wo],
            Shape::new(&[head_dim, d]),
            DType::F32,
            s,
        )?;
        let part = g.push(mm(1, d, head_dim), &[o_h, wo_h], sh_d, DType::F32, s)?;
        head_parts.push(part);
    }
    let mut attn = head_parts[0];
    for &part in &head_parts[1..] {
        attn = g.push(
            OpNode::Elementwise { f: EwKind::Add },
            &[attn, part],
            sh_d,
            DType::F32,
            s,
        )?;
    }
    let res1 = g.push(
        OpNode::Elementwise { f: EwKind::Add },
        &[x, attn],
        sh_d,
        DType::F32,
        s,
    )?;

    // SwiGLU-FFN over RMSNorm·w(res1), residual.
    let n2 = push_rmsnorm(&mut g, res1, eps, sh_d, sh_1, s)?;
    let n2 = g.push(
        OpNode::Elementwise { f: EwKind::Mul },
        &[n2, ffn_norm],
        sh_d,
        DType::F32,
        s,
    )?;
    let gate = g.push(mm(1, ffn, d), &[n2, wg], sh_ffn, DType::F32, s)?;
    let up = g.push(mm(1, ffn, d), &[n2, wu], sh_ffn, DType::F32, s)?;
    let sg = g.push(
        OpNode::Elementwise { f: EwKind::Silu },
        &[gate],
        sh_ffn,
        DType::F32,
        s,
    )?;
    let hh = g.push(
        OpNode::Elementwise { f: EwKind::Mul },
        &[sg, up],
        sh_ffn,
        DType::F32,
        s,
    )?;
    let ffn_out = g.push(mm(1, d, ffn), &[hh, wd], sh_d, DType::F32, s)?;
    let out = g.push(
        OpNode::Elementwise { f: EwKind::Add },
        &[res1, ffn_out],
        sh_d,
        DType::F32,
        s,
    )?;
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
        OpNode::GatherDequant {
            scheme: D::Ternary,
            block: cols,
        },
        &[packed, scale],
        sh_w,
        D::F32,
        s,
    )?;
    let y = g.push(
        OpNode::MatMul {
            m: 1,
            n: cols,
            k: rows,
            tc: false,
            trans_b: false,
        },
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
                assert!(
                    (a - b).abs() <= 1e-3 * b.abs().max(1.0),
                    "rmsnorm: {a} vs {b}"
                );
            }
        }
        // SwiGLU-FFN block (the LLM workhorse) — MatMul + Elementwise multi-node DAG.
        {
            let (seq, dim, ffn) = (8u32, 64u32, 128u32);
            let x: Vec<f32> = (0..seq * dim)
                .map(|i| ((i % 17) as f32) * 0.05 - 0.4)
                .collect();
            let wg: Vec<f32> = (0..dim * ffn)
                .map(|i| ((i % 13) as f32) * 0.02 - 0.12)
                .collect();
            let wu: Vec<f32> = (0..dim * ffn)
                .map(|i| ((i % 11) as f32) * 0.015 - 0.07)
                .collect();
            let wd: Vec<f32> = (0..ffn * dim)
                .map(|i| ((i % 7) as f32) * 0.01 - 0.03)
                .collect();
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

    /// Device-unification cert (LLM-on-forge Phase 1a): the forge running on the **process-wide
    /// shared GPU device** ([`crate::gpu_context::shared_gpu`], the device that owns the LLM
    /// weights + KV cache) produces results identical (within f32 tol) to the composed CPU oracle —
    /// for a full **faithful decode block** (RMSNorm·eps → scaled attention → residual → SwiGLU-FFN
    /// → residual). Proves `WgpuComputeContext::from_device` + `ForgeGraphExecutor::on_shared_gpu`
    /// run real multi-node graphs correctly on the shared device, not a second one.
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn shared_device_executor_matches_cpu_oracle() {
        let mut exec =
            ForgeGraphExecutor::on_shared_gpu().expect("forge executor on shared_gpu device");

        // The forge must report the SAME adapter as the process-wide shared device — i.e. it did
        // not silently spin up a second adapter/device.
        let shared = crate::gpu_context::shared_gpu();
        let forge_adapter = &exec.context().adapter;
        assert_eq!(
            forge_adapter.vendor, shared.adapter_caps.vendor,
            "forge ran on a different vendor than shared_gpu"
        );
        assert_eq!(
            forge_adapter.device, shared.adapter_caps.device,
            "forge ran on a different device than shared_gpu"
        );

        // softmax (1024-wide) on the shared device matches the oracle and is a distribution.
        {
            let n = 1024usize;
            let x: Vec<f32> = (0..n).map(|i| ((i * 13 % 97) as f32) * 0.1 - 5.0).collect();
            let g = softmax_graph(n as u32).unwrap();
            let gpu = exec.run(&g, &[x.clone()]).expect("softmax shared-gpu");
            let cpu = execute_graph_cpu(&g, &[x]).unwrap();
            for (a, b) in gpu.iter().zip(cpu.iter()) {
                assert!((a - b).abs() <= 1e-4, "softmax(shared): {a} vs {b}");
            }
            assert!((gpu.iter().sum::<f32>() - 1.0).abs() < 1e-3);
        }

        // Full faithful decode block on the SAME held executor (the decode-step usage pattern):
        // externals = [x, Kᵀ, V, Wg, Wu, Wd, inv_scale, eps].
        {
            let (d, kv, ffn) = (64u32, 32u32, 128u32);
            let inv_scale = 1.0f32 / (d as f32).sqrt();
            let eps = 1e-5f32;
            let x: Vec<f32> = (0..d).map(|i| ((i % 17) as f32) * 0.05 - 0.4).collect();
            let kt: Vec<f32> = (0..d * kv)
                .map(|i| ((i * 5 % 7) as f32) * 0.03 - 0.09)
                .collect();
            let v: Vec<f32> = (0..kv * d)
                .map(|i| ((i * 3 % 5) as f32) * 0.04 - 0.08)
                .collect();
            let wg: Vec<f32> = (0..d * ffn)
                .map(|i| ((i % 13) as f32) * 0.02 - 0.12)
                .collect();
            let wu: Vec<f32> = (0..d * ffn)
                .map(|i| ((i % 11) as f32) * 0.015 - 0.07)
                .collect();
            let wd: Vec<f32> = (0..ffn * d)
                .map(|i| ((i % 7) as f32) * 0.01 - 0.03)
                .collect();
            let ext = vec![x, kt, v, wg, wu, wd, vec![inv_scale], vec![eps]];
            let g = decode_block_graph(d, kv, ffn).unwrap();
            let gpu = exec.run(&g, &ext).expect("decode-block shared-gpu");
            let cpu = execute_graph_cpu(&g, &ext).unwrap();
            assert_eq!(gpu.len(), cpu.len());
            for (a, b) in gpu.iter().zip(cpu.iter()) {
                assert!(
                    (a - b).abs() <= 1e-2 * b.abs().max(1.0),
                    "decode-block(shared): {a} vs {b}"
                );
            }
        }
    }

    /// Weight-residency cert + perf (LLM-on-forge Phase 1b): a decode block's FFN matrices
    /// (Wg, Wu, Wd) are uploaded ONCE via `load_weights`, then `run_resident` is called repeatedly
    /// with only the activations. Correctness: the resident path matches both the all-upload `run`
    /// path **exactly** (same kernels, same bytes) and the composed CPU oracle, across multiple
    /// calls (proving the resident weights survive the per-call transient-ring reset). Perf: prints
    /// ms/call for resident vs all-upload and the per-call weight bytes no longer re-uploaded.
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn resident_weights_decode_block() {
        use std::time::Instant;
        let (d, kv, ffn) = (576u32, 128u32, 1536u32);
        let inv_scale = 1.0f32 / (d as f32).sqrt();
        let eps = 1e-5f32;
        let x: Vec<f32> = (0..d).map(|i| ((i % 17) as f32) * 0.05 - 0.4).collect();
        let kt: Vec<f32> = (0..d * kv)
            .map(|i| ((i * 5 % 7) as f32) * 0.03 - 0.09)
            .collect();
        let v: Vec<f32> = (0..kv * d)
            .map(|i| ((i * 3 % 5) as f32) * 0.04 - 0.08)
            .collect();
        let wg: Vec<f32> = (0..d * ffn)
            .map(|i| ((i % 13) as f32) * 0.02 - 0.12)
            .collect();
        let wu: Vec<f32> = (0..d * ffn)
            .map(|i| ((i % 11) as f32) * 0.015 - 0.07)
            .collect();
        let wd: Vec<f32> = (0..ffn * d)
            .map(|i| ((i % 7) as f32) * 0.01 - 0.03)
            .collect();
        let g = decode_block_graph(d, kv, ffn).unwrap();

        // All-upload externals (the run() baseline) — every tensor provided.
        let full = vec![
            x.clone(),
            kt.clone(),
            v.clone(),
            wg.clone(),
            wu.clone(),
            wd.clone(),
            vec![inv_scale],
            vec![eps],
        ];
        // Resident-activation externals: indices 3,4,5 (Wg,Wu,Wd) are resident → empty placeholders.
        let acts = vec![
            x.clone(),
            kt.clone(),
            v.clone(),
            vec![],
            vec![],
            vec![],
            vec![inv_scale],
            vec![eps],
        ];

        let mut exec = ForgeGraphExecutor::new().expect("forge executor");
        // Upload the FFN weight matrices once into the persistent region.
        let resident = exec
            .load_weights(&[(3, wg.clone()), (4, wu.clone()), (5, wd.clone())])
            .expect("load_weights");
        assert_eq!(resident.len(), 3);
        let resident_bytes = exec.context().resident_weight_bytes();

        let cpu = execute_graph_cpu(&g, &full).unwrap();
        let upload_ref = exec.run(&g, &full).expect("run all-upload");

        // Resident path matches the all-upload path EXACTLY (identical kernels + bytes), and the
        // CPU oracle, on every one of several calls (resident weights persist across runs).
        for call in 0..3 {
            let res = exec
                .run_resident(&g, &acts, &resident)
                .expect("run_resident");
            assert_eq!(res.len(), upload_ref.len());
            for (a, b) in res.iter().zip(upload_ref.iter()) {
                assert_eq!(a, b, "resident != all-upload on call {call}");
            }
            for (a, b) in res.iter().zip(cpu.iter()) {
                assert!(
                    (a - b).abs() <= 1e-2 * b.abs().max(1.0),
                    "resident != oracle: {a} vs {b}"
                );
            }
        }

        // Perf: time resident vs all-upload (after warmup).
        let iters = 50;
        for _ in 0..5 {
            let _ = exec.run(&g, &full).unwrap();
            let _ = exec.run_resident(&g, &acts, &resident).unwrap();
        }
        let t0 = Instant::now();
        for _ in 0..iters {
            let _ = exec.run(&g, &full).unwrap();
        }
        let upload_ms = t0.elapsed().as_secs_f64() * 1e3 / iters as f64;
        let t1 = Instant::now();
        for _ in 0..iters {
            let _ = exec.run_resident(&g, &acts, &resident).unwrap();
        }
        let resident_ms = t1.elapsed().as_secs_f64() * 1e3 / iters as f64;
        let saved_bytes = (wg.len() + wu.len() + wd.len()) * std::mem::size_of::<f32>();
        println!(
            "[weight residency] decode block d={d} kv={kv} ffn={ffn} | resident {resident_ms:.3} ms/call vs all-upload {upload_ms:.3} ms/call | weights {resident_bytes} B resident, {saved_bytes} B/call NOT re-uploaded. Correctness: resident==all-upload (exact) + matches CPU oracle across 3 calls."
        );
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
        let kt: Vec<f32> = (0..d * kv)
            .map(|i| ((i * 5 % 7) as f32) * 0.1 - 0.25)
            .collect();
        let v: Vec<f32> = (0..kv * d)
            .map(|i| ((i * 3 % 5) as f32) * 0.15 - 0.2)
            .collect();
        let g = attention_graph(d as u32, kv as u32).unwrap();
        let out =
            execute_graph_cpu(&g, &[q.clone(), kt.clone(), v.clone(), vec![inv_scale]]).unwrap();
        // Reference: scores = (q·Kᵀ)/√d [1,kv]; probs = softmax(scores); out = probs·V [1,d].
        let scores: Vec<f32> = ref_mm(&q, &kt, 1, d, kv)
            .iter()
            .map(|s| s * inv_scale)
            .collect();
        let probs = ref_softmax(&scores);
        let want = ref_mm(&probs, &v, 1, kv, d);
        assert_eq!(out.len(), d);
        for (o, w) in out.iter().zip(&want) {
            assert!((o - w).abs() < 1e-5, "{o} vs {w}");
        }
    }

    // ── Real multi-head (GQA) decode layer ───────────────────────────────────────────

    /// Deterministic externals for [`decode_layer_graph`] (head-major K/V cache layout).
    fn decode_layer_externals(
        n_heads: u32,
        n_kv_heads: u32,
        head_dim: u32,
        seq: u32,
        ffn: u32,
    ) -> Vec<Vec<f32>> {
        let d = (n_heads * head_dim) as usize;
        let (hd, seqn, ffnn, n_kv) = (
            head_dim as usize,
            seq as usize,
            ffn as usize,
            n_kv_heads as usize,
        );
        let gen = |len: usize, salt: usize| -> Vec<f32> {
            (0..len)
                .map(|i| (((i * 7 + salt * 13) % 23) as f32) * 0.05 - 0.55)
                .collect()
        };
        let inv_scale = 1.0f32 / (head_dim as f32).sqrt();
        vec![
            gen(d, 1),                // x  [1,d]
            gen(n_kv * hd * seqn, 2), // Kt [n_kv, head_dim, seq]
            gen(n_kv * seqn * hd, 3), // V  [n_kv, seq, head_dim]
            gen(d * d, 4),            // Wq [d,d]
            gen(d * d, 5),            // Wo [d,d]
            gen(d * ffnn, 6),         // Wg [d,ffn]
            gen(d * ffnn, 7),         // Wu [d,ffn]
            gen(ffnn * d, 8),         // Wd [ffn,d]
            gen(d, 9),                // attn_norm [d]
            gen(d, 10),               // ffn_norm  [d]
            vec![inv_scale],          // inv_scale
            vec![1e-5],               // eps
        ]
    }

    /// Independent hand-written reference for the real decode layer (mirrors the math, not the
    /// graph code): RMSNorm → Q-proj → RoPE(q) → per-head GQA `softmax(q_h·Kᵀ_h/√hd)·V_h` →
    /// `o_h·Wo_h` summed → +x → RMSNorm → SwiGLU → +.
    #[allow(clippy::too_many_arguments)]
    fn ref_decode_layer(
        ext: &[Vec<f32>],
        n_heads: u32,
        n_kv_heads: u32,
        head_dim: u32,
        seq: u32,
        ffn: u32,
        pos: u32,
        mode: u32,
        base: f32,
    ) -> Vec<f32> {
        use crate::wgsl_forge::graph_ops::stencil::{rope_cpu, RopeConfig, RopeMode};
        let d = (n_heads * head_dim) as usize;
        let (hd, seqn, ffnn) = (head_dim as usize, seq as usize, ffn as usize);
        let group = (n_heads / n_kv_heads) as usize;
        let (x, kt, v, wq, wo, wg, wu, wd) = (
            &ext[0], &ext[1], &ext[2], &ext[3], &ext[4], &ext[5], &ext[6], &ext[7],
        );
        let attn_norm = &ext[8];
        let ffn_norm = &ext[9];
        let inv_scale = ext[10][0];
        let eps = ext[11][0];
        let n1: Vec<f32> = ref_rmsnorm(x, eps)
            .iter()
            .zip(attn_norm)
            .map(|(a, w)| a * w)
            .collect();
        let q = ref_mm(&n1, wq, 1, d, d);
        let rmode = if mode == 0 {
            RopeMode::Interleaved
        } else {
            RopeMode::Neox
        };
        let q = rope_cpu(
            &q,
            &RopeConfig {
                head_dim,
                pos,
                mode: rmode,
                theta_base: base,
            },
        )
        .unwrap();
        let mut attn = vec![0.0f32; d];
        for h in 0..n_heads as usize {
            let kh = h / group;
            let q_h = &q[h * hd..(h + 1) * hd];
            let kt_h = &kt[kh * hd * seqn..(kh + 1) * hd * seqn];
            let v_h = &v[kh * seqn * hd..(kh + 1) * seqn * hd];
            let scores: Vec<f32> = ref_mm(q_h, kt_h, 1, hd, seqn)
                .iter()
                .map(|s| s * inv_scale)
                .collect();
            let probs = ref_softmax(&scores);
            let o_h = ref_mm(&probs, v_h, 1, seqn, hd);
            let wo_h = &wo[h * hd * d..(h + 1) * hd * d];
            let part = ref_mm(&o_h, wo_h, 1, hd, d);
            for (a, p) in attn.iter_mut().zip(&part) {
                *a += *p;
            }
        }
        let res1: Vec<f32> = x.iter().zip(&attn).map(|(a, b)| a + b).collect();
        let n2: Vec<f32> = ref_rmsnorm(&res1, eps)
            .iter()
            .zip(ffn_norm)
            .map(|(a, w)| a * w)
            .collect();
        let gate = ref_mm(&n2, wg, 1, d, ffnn);
        let up = ref_mm(&n2, wu, 1, d, ffnn);
        let hsl: Vec<f32> = gate
            .iter()
            .zip(&up)
            .map(|(g, u)| (g / (1.0 + (-g).exp())) * u)
            .collect();
        let ffn_out = ref_mm(&hsl, wd, 1, ffnn, d);
        res1.iter().zip(&ffn_out).map(|(a, b)| a + b).collect()
    }

    /// The real multi-head (GQA) decode-layer graph's **composed CPU oracle** matches an
    /// INDEPENDENT hand-written reference — proving the graph computes a real decode layer (not
    /// merely that the GPU matches its own oracle). Covers both RoPE conventions + GQA (2:1).
    #[test]
    fn decode_layer_cpu_oracle_matches_reference() {
        let (n_heads, n_kv_heads, head_dim, seq, ffn) = (2u32, 1u32, 8u32, 4u32, 16u32);
        let (pos, base) = (3u32, 10000.0f32);
        let ext = decode_layer_externals(n_heads, n_kv_heads, head_dim, seq, ffn);
        for mode in [0u32, 1u32] {
            let g = decode_layer_graph(n_heads, n_kv_heads, head_dim, seq, ffn, pos, mode, base)
                .unwrap();
            let cpu = execute_graph_cpu(&g, &ext).unwrap();
            let want = ref_decode_layer(
                &ext, n_heads, n_kv_heads, head_dim, seq, ffn, pos, mode, base,
            );
            assert_eq!(cpu.len(), (n_heads * head_dim) as usize);
            for (a, b) in cpu.iter().zip(&want) {
                assert!(
                    (a - b).abs() <= 1e-4 * b.abs().max(1.0),
                    "decode-layer oracle vs ref (mode {mode}): {a} vs {b}"
                );
            }
        }
    }

    /// GPU certify: the real multi-head decode layer on the A2000 matches its composed CPU oracle,
    /// for both RoPE conventions, at a realistic shape (4 heads, 2 kv-heads = GQA 2:1, head_dim 16).
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn decode_layer_gpu_matches_cpu_oracle() {
        let (n_heads, n_kv_heads, head_dim, seq, ffn) = (4u32, 2u32, 16u32, 8u32, 32u32);
        let (pos, base) = (5u32, 10000.0f32);
        let ext = decode_layer_externals(n_heads, n_kv_heads, head_dim, seq, ffn);
        for mode in [0u32, 1u32] {
            let g = decode_layer_graph(n_heads, n_kv_heads, head_dim, seq, ffn, pos, mode, base)
                .unwrap();
            let gpu = execute_graph(&g, &ext).expect("decode-layer gpu");
            let cpu = execute_graph_cpu(&g, &ext).unwrap();
            assert_eq!(gpu.len(), (n_heads * head_dim) as usize);
            for (a, b) in gpu.iter().zip(&cpu) {
                assert!(
                    (a - b).abs() <= 1e-2 * b.abs().max(1.0),
                    "decode-layer gpu vs oracle (mode {mode}): {a} vs {b}"
                );
            }
        }
    }

    // ── MatMul.trans_b (native [out,in] weight layout) ───────────────────────────────

    fn trans_b_graph(m: u32, n: u32, k: u32) -> ComputeGraph {
        let mut g = ComputeGraph::new();
        let s = Schedule::default();
        let a = TensorRef::input(0, Shape::new(&[m, k]), DType::F32);
        let b = TensorRef::input(1, Shape::new(&[n, k]), DType::F32);
        let out = g
            .push(
                OpNode::MatMul {
                    m,
                    n,
                    k,
                    tc: false,
                    trans_b: true,
                },
                &[a, b],
                Shape::new(&[m, n]),
                DType::F32,
                s,
            )
            .unwrap();
        g.mark_output(out);
        g
    }

    /// `MatMul.trans_b` CPU oracle (B bound `[n,k]`) matches an independent `A·Bᵀ` reference —
    /// proving the previously-silently-dropped `trans_b` flag now computes.
    #[test]
    fn matmul_trans_b_cpu_oracle_matches_reference() {
        let (m, n, k) = (2usize, 3usize, 4usize);
        let a: Vec<f32> = (0..m * k).map(|i| (i as f32) * 0.2 - 0.5).collect();
        let b_nk: Vec<f32> = (0..n * k).map(|i| (i as f32) * 0.1 - 0.2).collect();
        let cpu = execute_graph_cpu(
            &trans_b_graph(m as u32, n as u32, k as u32),
            &[a.clone(), b_nk.clone()],
        )
        .unwrap();
        let mut want = vec![0.0f32; m * n];
        for i in 0..m {
            for j in 0..n {
                let mut acc = 0.0f32;
                for kk in 0..k {
                    acc += a[i * k + kk] * b_nk[j * k + kk];
                }
                want[i * n + j] = acc;
            }
        }
        for (c, w) in cpu.iter().zip(&want) {
            assert!((c - w).abs() < 1e-5, "trans_b cpu {c} vs ref {w}");
        }
    }

    /// GPU certify `trans_b` two ways on the A2000: vs the composed CPU oracle, AND vs the **plain
    /// GEMM run on an explicitly-transposed B** (a different kernel path) — the gold cross-check.
    #[test]
    #[ignore = "requires a GPU adapter"]
    fn matmul_trans_b_gpu_matches_plain_on_transposed() {
        let (m, n, k) = (3usize, 5usize, 4usize);
        let a: Vec<f32> = (0..m * k).map(|i| (i as f32) * 0.1 - 0.3).collect();
        let b_nk: Vec<f32> = (0..n * k)
            .map(|i| ((i * 3 % 7) as f32) * 0.05 - 0.15)
            .collect();
        let g = trans_b_graph(m as u32, n as u32, k as u32);
        let gpu = execute_graph(&g, &[a.clone(), b_nk.clone()]).expect("trans_b gpu");
        let cpu = execute_graph_cpu(&g, &[a.clone(), b_nk.clone()]).unwrap();
        for (gp, cp) in gpu.iter().zip(&cpu) {
            assert!(
                (gp - cp).abs() <= 1e-4 * cp.abs().max(1.0),
                "trans_b gpu {gp} vs cpu {cp}"
            );
        }
        // Cross-check: plain GEMM on B explicitly transposed to [k,n] must match.
        let mut b_kn = vec![0.0f32; k * n];
        for j in 0..n {
            for kk in 0..k {
                b_kn[kk * n + j] = b_nk[j * k + kk];
            }
        }
        let mut g2 = ComputeGraph::new();
        let s = Schedule::default();
        let a2 = TensorRef::input(0, Shape::new(&[m as u32, k as u32]), DType::F32);
        let b2 = TensorRef::input(1, Shape::new(&[k as u32, n as u32]), DType::F32);
        let out2 = g2
            .push(
                OpNode::MatMul {
                    m: m as u32,
                    n: n as u32,
                    k: k as u32,
                    tc: false,
                    trans_b: false,
                },
                &[a2, b2],
                Shape::new(&[m as u32, n as u32]),
                DType::F32,
                s,
            )
            .unwrap();
        g2.mark_output(out2);
        let plain = execute_graph(&g2, &[a, b_kn]).expect("plain gpu");
        for (t, p) in gpu.iter().zip(&plain) {
            assert!(
                (t - p).abs() <= 1e-4 * p.abs().max(1.0),
                "trans_b {t} vs plain-on-transposed {p}"
            );
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
        let kt: Vec<f32> = (0..d * kv)
            .map(|i| ((i * 5 % 7) as f32) * 0.1 - 0.25)
            .collect();
        let v: Vec<f32> = (0..kv * d)
            .map(|i| ((i * 3 % 5) as f32) * 0.15 - 0.2)
            .collect();
        let wg: Vec<f32> = (0..d * ffn)
            .map(|i| ((i % 11) as f32) * 0.03 - 0.15)
            .collect();
        let wu: Vec<f32> = (0..d * ffn)
            .map(|i| ((i % 7) as f32) * 0.02 - 0.07)
            .collect();
        let wd: Vec<f32> = (0..ffn * d)
            .map(|i| ((i % 5) as f32) * 0.04 - 0.08)
            .collect();
        let g = decode_block_graph(d as u32, kv as u32, ffn as u32).unwrap();
        let ext = vec![
            x.clone(),
            kt.clone(),
            v.clone(),
            wg.clone(),
            wu.clone(),
            wd.clone(),
            vec![inv_scale],
            vec![eps],
        ];
        let out = execute_graph_cpu(&g, &ext).unwrap();

        // Reference (with 1/√d attention scale + RMSNorm eps).
        let n1 = ref_rmsnorm(&x, eps);
        let scores: Vec<f32> = ref_mm(&n1, &kt, 1, d, kv)
            .iter()
            .map(|s| s * inv_scale)
            .collect();
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
            let kt: Vec<f32> = (0..d * kv)
                .map(|i| ((i % 19) as f32) * 0.02 - 0.18)
                .collect();
            let v: Vec<f32> = (0..kv * d)
                .map(|i| ((i % 13) as f32) * 0.03 - 0.18)
                .collect();
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
                .map(|i| match (i * 7) % 3 {
                    0 => 1.0,
                    1 => -1.0,
                    _ => 0.0,
                })
                .collect();
            let scale: Vec<f32> = (0..rows).map(|r| 0.25 + (r % 5) as f32 * 0.1).collect();
            let packed = pack_ternary_as_words(&vals, rows, cols);
            let x: Vec<f32> = (0..rows).map(|i| ((i % 9) as f32) * 0.1 - 0.4).collect();
            let g = dequant_matmul_graph(rows as u32, cols as u32).unwrap();
            let ext = vec![x, packed, scale];
            let gpu = execute_graph(&g, &ext).expect("dequant gpu");
            let cpu = execute_graph_cpu(&g, &ext).unwrap();
            for (a, b) in gpu.iter().zip(&cpu) {
                assert!(
                    (a - b).abs() <= 1e-3 * b.abs().max(1.0),
                    "dequant: {a} vs {b}"
                );
            }
        }
        // Full decode block.
        {
            let (d, kv, ffn) = (64u32, 80u32, 128u32);
            let mk = |n: usize, m: u32| {
                (0..n)
                    .map(|i| ((i as u32 % m) as f32) * 0.01 - 0.1)
                    .collect::<Vec<f32>>()
            };
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
                assert!(
                    (a - b).abs() <= 1e-2 * b.abs().max(1.0),
                    "decode: {a} vs {b}"
                );
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
            (0..n)
                .map(|i| ((i as u32 % m) as f32) * 0.001 - off)
                .collect::<Vec<f32>>()
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
        let mk = |n: usize, m: u32| {
            (0..n)
                .map(|i| ((i as u32 % m) as f32) * 0.01 - 0.1)
                .collect::<Vec<f32>>()
        };
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
        let mut exec = ForgeGraphExecutor::new().expect("executor");
        let _ = exec.run(&g, &ext).expect("run 1");
        let after_first = exec.context().cached_pipeline_count();
        let _ = exec.run(&g, &ext).expect("run 2");
        let after_second = exec.context().cached_pipeline_count();
        // Stable: the second run compiled nothing new.
        assert_eq!(
            after_first, after_second,
            "cache must be stable across runs"
        );
        // Distinct kernels < node count (repeated op-classes share a pipeline).
        assert!(
            after_first > 0 && after_first <= g.nodes.len(),
            "cached={after_first} nodes={}",
            g.nodes.len()
        );
        // The result is unchanged across runs (cache returns the same pipeline).
        let a = exec.run(&g, &ext).unwrap();
        let b = exec.run(&g, &ext).unwrap();
        assert_eq!(a, b, "cached runs must be deterministic");
    }
}
