pub mod capabilities;
pub mod core;
pub mod graph;
pub mod intrinsics;
pub mod q42_bridge;

pub use capabilities::{
    HardwareCapabilityMatrix, IntrinsicSupport, LoweringContext, LoweringPolicy64Bit,
};
pub use core::{
    BufferAccess, BufferElement, BufferSpec, BuiltinKernel, KernelSpec, Op, P64GpuWords64,
    ScalarType, SharedLen, SharedMemorySpec,
};
pub use graph::{
    lower_graph, ComputeGraph, DType, GraphNode, Layout, Lowerer, NodeId, OpNode, Shape, TensorRef,
};
pub use intrinsics::{Intrinsic, IntrinsicClass, SubgroupReduceOp};
pub use q42_bridge::{deserialize_graph, graph_merkle_root, opcode_of, serialize_graph};
