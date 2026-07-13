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
//!
//! # Module layout
//!
//! - [`cpu_oracle`] — the composed CPU oracle ([`execute_graph_cpu`]).
//! - [`driver`] — the reusable GPU executor ([`ForgeGraphExecutor`]), the one-shot
//!   [`execute_graph`] free function, and the [`ResidentWeights`] handle.
//! - [`nodes`] — per-node preparation/dispatch (`prepare_node` and the op-class kernels).
//! - [`builders`] — the graph builders (softmax / RMSNorm / SwiGLU-FFN / attention / decode).

mod builders;
mod cpu_oracle;
mod driver;
mod nodes;

#[cfg(test)]
mod tests;

pub use builders::*;
pub use cpu_oracle::*;
pub use driver::*;
