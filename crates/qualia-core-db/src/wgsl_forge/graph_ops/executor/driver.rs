//! The reusable GPU graph executor ([`ForgeGraphExecutor`]) and its one-shot free function
//! ([`execute_graph`]), plus the [`ResidentWeights`] handle for once-uploaded weights. This is
//! the throughput-pass driver: context reuse + single-encoder deferred submit.

use super::nodes::prepare_node;
use crate::wgsl_forge::execute::{BindingUsage, BufferView, GraphPass, WgpuComputeContext};
use crate::wgsl_forge::ir::graph::{ComputeGraph, NodeId};
use crate::wgsl_forge::ForgeError;

/// Context capacity (per slab). Large enough for a decode block's tensors held at once.
const EXEC_CAPACITY: usize = 64 << 20;

/// Execute the whole graph on the GPU and read back the final tensor (one-shot). Differs from
/// the composed CPU floor only by f32 GPU arithmetic; certified against
/// [`execute_graph_cpu`](super::execute_graph_cpu).
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
