//! Native compute-graph op-node kernels — WGSL templates emitted **directly from a graph
//! node** (not delegated to a legacy `BuiltinKernel` emitter, since these op-classes have
//! no legacy standalone kernel). Each carries an exact CPU oracle and a GPU differential
//! certify, exactly like the rest of the forge.
//!
//! Phase 2 lands the LLM-normalization building blocks: [`reduce`] (RMSNorm variance /
//! softmax max+denominator) and [`broadcast`] (scale/bias fanout). They are lowered from
//! `OpNode::Reduce` / `OpNode::Broadcast` by the WGSL graph lowerer. See
//! [`docs/plans/dag-ir-forge.md`].

pub mod broadcast;
pub mod elementwise;
pub mod executor;
pub mod gather_dequant;
pub mod neighbor;
pub mod reduce;
pub mod scatter;
pub mod stencil;
