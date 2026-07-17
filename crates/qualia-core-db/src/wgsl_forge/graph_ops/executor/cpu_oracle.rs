//! The composed CPU oracle: run a whole [`ComputeGraph`] on the CPU in topological order,
//! threading each node's output forward. This is the differential floor certified against the
//! GPU executor.

use crate::wgsl_forge::graph_ops::{
    broadcast, elementwise, gather_dequant, reduce, slice, stencil, vision,
};
use crate::wgsl_forge::ir::graph::{ComputeGraph, DType, NodeId, OpNode};
use crate::wgsl_forge::ForgeError;

/// Compose the graph on the CPU in topological order — the differential oracle for
/// [`execute_graph`](super::execute_graph). Each node's output is computed from its inputs (graph
/// externals or prior nodes' outputs) using the per-op-class CPU floor, and threaded forward.
/// Returns the final output tensor (the last `mark_output`, or the last node).
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
            OpNode::Pool2d {
                c,
                h,
                w,
                kh,
                kw,
                stride_h,
                stride_w,
            } => vision::max_pool2d_cpu(
                &ins[0],
                c as usize,
                h as usize,
                w as usize,
                kh as usize,
                kw as usize,
                stride_h as usize,
                stride_w as usize,
            )?,
            OpNode::Resize2d {
                c,
                h_in,
                w_in,
                h_out,
                w_out,
            } => vision::resize2d_cpu(
                &ins[0],
                c as usize,
                h_in as usize,
                w_in as usize,
                h_out as usize,
                w_out as usize,
            )?,
            OpNode::Conv2d {
                c_in,
                c_out,
                h,
                w,
                kh,
                kw,
                stride_h,
                stride_w,
                pad_h,
                pad_w,
            } => vision::conv2d_cpu(
                &ins[0],
                c_in as usize,
                h as usize,
                w as usize,
                &ins[1],
                c_out as usize,
                kh as usize,
                kw as usize,
                &ins[2],
                stride_h as usize,
                stride_w as usize,
                pad_h as usize,
                pad_w as usize,
            )?,
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
