//! q42 substrate binding (plan §5 / Phase 6) — serialize a [`ComputeGraph`] **to and from**
//! 48-byte [`NQuin`] records, so a compute DAG persists in the same Merkle-DAG quin store as
//! the rest of QualiaDB (zero-copy, page-aligned), with provenance.
//!
//! # The in-arena graph is the source of truth
//!
//! Per plan §5 the quin encoding is a *serialization / persistence / provenance view*, not a
//! second representation that lowering reads. So the contract this module guarantees is a
//! **round-trip identity**: `deserialize_graph(serialize_graph(g)) == g` (node-for-node), and
//! therefore re-lowering the round-tripped graph reproduces byte-identical WGSL → an
//! *identical certify*. The bytes are pure `NQuin` (zero-copy via `bytemuck`), so the same
//! `Vec<NQuin>` is what a q42 volume superblock would hold on disk.
//!
//! # Encoding (one node → a few quins, no `kernel.id` strings)
//!
//! Each node `i` (its [`NodeId`] index) emits companion quins keyed by predicate (all in the
//! single q42 data-flow namespace, distinct from the version-control `dag:parent` and causal
//! `causeOf` predicates):
//!
//! | predicate | carries |
//! |---|---|
//! | `q42:opKind`       | `object`=op-kind opcode (**0x10–0x1A**, the reserved-modality range — `mini_parser` owns 0x00–0x04, deontic owns 0x50+, no overlap, core invariant §6); `context`/`parity` = the op payload words |
//! | `q42:tensorShape`  | `object`=4×`u16` packed dims; `parity`=rank (the node's output shape) |
//! | `q42:dtype`        | `object`=dtype code (the node's output dtype) |
//! | `q42:scheduleHint` | `object`=`workgroup_size | items<<32`; `parity`=vector_width |
//! | `q42:feedsInto`    | one per input edge: `subject`=producer id (or `EXTERNAL`), `object`=consumer `i`, `context`=`slot | tensor<<32`, `metadata`=packed input dims, `parity`=rank/dtype/layout of the input edge |
//! | `q42:graphOutput`  | `subject`=an output node id |
//!
//! Shape dims are packed as `u16` per axis (the LLM/physics graphs are all ≤65535 per axis);
//! a larger dim is a hard error, never a silent truncation. The MatMul/Gemv op payload carries
//! `m`/`n`/`k` at **full `u32`** in the payload words, so matmul dimensions are exact.

use std::collections::HashMap;

use crate::wgsl_forge::ir::graph::{
    Axis, ComputeGraph, DType, EwKind, Layout, NbKind, NeighborEnc, NodeId, OpNode, RedKind,
    Shape, StencilKind, TensorId, TensorRef,
};
use crate::wgsl_forge::ir::graph::AccumKind;
use crate::wgsl_forge::{ForgeError, Schedule};
use crate::{q_hash, NQuin};

// ── Predicates (the q42 data-flow namespace) ─────────────────────────────────────────
const P_OPKIND: u64 = q_hash("urn:qualia:q42:opKind");
const P_SHAPE: u64 = q_hash("urn:qualia:q42:tensorShape");
const P_DTYPE: u64 = q_hash("urn:qualia:q42:dtype");
const P_SCHED: u64 = q_hash("urn:qualia:q42:scheduleHint");
const P_FEEDS: u64 = q_hash("urn:qualia:q42:feedsInto");
const P_OUTPUT: u64 = q_hash("urn:qualia:q42:graphOutput");

// ── Op-kind opcodes — reserved-modality range 0x10+ (no overlap with mini_parser 0x00–0x04
//    or deontic 0x50+; core invariant §6). One per OpNode arm. ──────────────────────────
const OP_ELEMENTWISE: u64 = 0x10;
const OP_MATMUL: u64 = 0x11;
const OP_GEMV: u64 = 0x12;
const OP_FFT: u64 = 0x13;
const OP_REDUCE: u64 = 0x14;
const OP_GATHER_DEQUANT: u64 = 0x15;
const OP_BROADCAST: u64 = 0x16;
const OP_SOFTMAX: u64 = 0x17;
const OP_STENCIL: u64 = 0x18;
const OP_SCATTER_ACCUM: u64 = 0x19;
const OP_NEIGHBOR: u64 = 0x1A;
const OP_SLICE: u64 = 0x1B;
const OP_ROPE: u64 = 0x1C;

/// The op-kind opcode for an [`OpNode`] (the value stored in a `q42:opKind` quin's `object`).
pub fn opcode_of(op: &OpNode) -> u64 {
    match op {
        OpNode::Elementwise { .. } => OP_ELEMENTWISE,
        OpNode::MatMul { .. } => OP_MATMUL,
        OpNode::Gemv { .. } => OP_GEMV,
        OpNode::Fft { .. } => OP_FFT,
        OpNode::Reduce { .. } => OP_REDUCE,
        OpNode::GatherDequant { .. } => OP_GATHER_DEQUANT,
        OpNode::Broadcast { .. } => OP_BROADCAST,
        OpNode::Softmax { .. } => OP_SOFTMAX,
        OpNode::Stencil { .. } => OP_STENCIL,
        OpNode::ScatterAccum { .. } => OP_SCATTER_ACCUM,
        OpNode::Neighbor { .. } => OP_NEIGHBOR,
        OpNode::Slice { .. } => OP_SLICE,
        OpNode::Rope { .. } => OP_ROPE,
    }
}

// ── Small enum ↔ code maps ───────────────────────────────────────────────────────────
fn err(msg: impl Into<String>) -> ForgeError {
    ForgeError::Serialization(msg.into())
}

fn dtype_code(d: DType) -> u64 {
    match d {
        DType::F32 => 0,
        DType::F16 => 1,
        DType::U32 => 2,
        DType::Q4K => 3,
        DType::Q8_0 => 4,
        DType::Ternary => 5,
    }
}
fn dtype_from(c: u64) -> Result<DType, ForgeError> {
    Ok(match c {
        0 => DType::F32,
        1 => DType::F16,
        2 => DType::U32,
        3 => DType::Q4K,
        4 => DType::Q8_0,
        5 => DType::Ternary,
        other => return Err(err(format!("q42: bad dtype code {other}"))),
    })
}

fn ewkind_code(f: EwKind) -> u64 {
    match f {
        EwKind::Mul => 0,
        EwKind::Add => 1,
        EwKind::Sub => 2,
        EwKind::Div => 3,
        EwKind::Fma => 4,
        EwKind::Scale => 5,
        EwKind::Bias => 6,
        EwKind::Silu => 7,
        EwKind::Gelu => 8,
        EwKind::Relu => 9,
        EwKind::Exp => 10,
        EwKind::RecipSqrt => 11,
        EwKind::Recip => 12,
    }
}
fn ewkind_from(c: u64) -> Result<EwKind, ForgeError> {
    Ok(match c {
        0 => EwKind::Mul,
        1 => EwKind::Add,
        2 => EwKind::Sub,
        3 => EwKind::Div,
        4 => EwKind::Fma,
        5 => EwKind::Scale,
        6 => EwKind::Bias,
        7 => EwKind::Silu,
        8 => EwKind::Gelu,
        9 => EwKind::Relu,
        10 => EwKind::Exp,
        11 => EwKind::RecipSqrt,
        12 => EwKind::Recip,
        other => return Err(err(format!("q42: bad EwKind code {other}"))),
    })
}

fn redkind_code(op: RedKind) -> u64 {
    match op {
        RedKind::Sum => 0,
        RedKind::Max => 1,
        RedKind::Mean => 2,
        RedKind::L2 => 3,
    }
}
fn redkind_from(c: u64) -> Result<RedKind, ForgeError> {
    Ok(match c {
        0 => RedKind::Sum,
        1 => RedKind::Max,
        2 => RedKind::Mean,
        3 => RedKind::L2,
        other => return Err(err(format!("q42: bad RedKind code {other}"))),
    })
}

/// Axis code: `Last=0`, `Penultimate=1`, `Index(n)=0x100 | n` (≤ 0x1FF, fits 16 bits).
fn axis_code(a: Axis) -> u64 {
    match a {
        Axis::Last => 0,
        Axis::Penultimate => 1,
        Axis::Index(n) => 0x100 | (n as u64),
    }
}
fn axis_from(c: u64) -> Result<Axis, ForgeError> {
    Ok(match c {
        0 => Axis::Last,
        1 => Axis::Penultimate,
        c if c & 0x100 != 0 => Axis::Index((c & 0xFF) as u8),
        other => return Err(err(format!("q42: bad Axis code {other}"))),
    })
}

fn stencil_code(k: StencilKind) -> u64 {
    match k {
        StencilKind::Laplacian => 0,
        StencilKind::Divergence => 1,
        StencilKind::Advection => 2,
        StencilKind::RopePair => 3,
    }
}
fn stencil_from(c: u64) -> Result<StencilKind, ForgeError> {
    Ok(match c {
        0 => StencilKind::Laplacian,
        1 => StencilKind::Divergence,
        2 => StencilKind::Advection,
        3 => StencilKind::RopePair,
        other => return Err(err(format!("q42: bad StencilKind code {other}"))),
    })
}

fn accum_code(op: AccumKind) -> u64 {
    match op {
        AccumKind::Add => 0,
        AccumKind::Max => 1,
    }
}
fn accum_from(c: u64) -> Result<AccumKind, ForgeError> {
    Ok(match c {
        0 => AccumKind::Add,
        1 => AccumKind::Max,
        other => return Err(err(format!("q42: bad AccumKind code {other}"))),
    })
}

fn nb_code(k: NbKind) -> u64 {
    match k {
        NbKind::Frnn => 0,
        NbKind::Knn => 1,
        NbKind::Range => 2,
    }
}
fn nb_from(c: u64) -> Result<NbKind, ForgeError> {
    Ok(match c {
        0 => NbKind::Frnn,
        1 => NbKind::Knn,
        2 => NbKind::Range,
        other => return Err(err(format!("q42: bad NbKind code {other}"))),
    })
}

fn enc_code(e: NeighborEnc) -> u64 {
    match e {
        NeighborEnc::Native3D => 0,
        NeighborEnc::Project => 1,
    }
}
fn enc_from(c: u64) -> Result<NeighborEnc, ForgeError> {
    Ok(match c {
        0 => NeighborEnc::Native3D,
        1 => NeighborEnc::Project,
        other => return Err(err(format!("q42: bad NeighborEnc code {other}"))),
    })
}

fn layout_code(l: Layout) -> u64 {
    match l {
        Layout::RowMajor => 0,
    }
}
fn layout_from(c: u64) -> Result<Layout, ForgeError> {
    Ok(match c {
        0 => Layout::RowMajor,
        other => return Err(err(format!("q42: bad Layout code {other}"))),
    })
}

/// Pack 4 dims as `u16` per axis (LSB-first). Errors if any dim exceeds `u16::MAX` — never a
/// silent truncation.
fn pack_dims(dims: [u32; 4]) -> Result<u64, ForgeError> {
    let mut out = 0u64;
    for (i, &d) in dims.iter().enumerate() {
        if d > u16::MAX as u32 {
            return Err(err(format!(
                "q42: dim {d} exceeds u16 (axis {i}); use the op payload for large matmul dims"
            )));
        }
        out |= (d as u64) << (i * 16);
    }
    Ok(out)
}
fn unpack_dims(w: u64) -> [u32; 4] {
    [
        (w & 0xFFFF) as u32,
        ((w >> 16) & 0xFFFF) as u32,
        ((w >> 32) & 0xFFFF) as u32,
        ((w >> 48) & 0xFFFF) as u32,
    ]
}

/// Encode an [`OpNode`] to `(opcode, payload_word0, payload_word1)`.
fn encode_op(op: &OpNode) -> Result<(u64, u64, u64), ForgeError> {
    Ok(match *op {
        OpNode::Elementwise { f } => (OP_ELEMENTWISE, ewkind_code(f), 0),
        OpNode::MatMul { m, n, k, tc, trans_b } => (
            OP_MATMUL,
            (m as u64) | ((n as u64) << 32),
            (k as u64) | ((tc as u64) << 32) | ((trans_b as u64) << 33),
        ),
        OpNode::Gemv { m, n } => (OP_GEMV, (m as u64) | ((n as u64) << 32), 0),
        OpNode::Fft { len, inverse } => {
            (OP_FFT, (len as u64) | ((inverse as u64) << 32), 0)
        }
        OpNode::Reduce { op, axis } => {
            (OP_REDUCE, redkind_code(op) | (axis_code(axis) << 16), 0)
        }
        OpNode::GatherDequant { scheme, block } => (
            OP_GATHER_DEQUANT,
            dtype_code(scheme) | ((block as u64) << 32),
            0,
        ),
        OpNode::Broadcast { shape } => {
            (OP_BROADCAST, pack_dims(shape.dims)?, shape.rank as u64)
        }
        OpNode::Softmax { axis } => (OP_SOFTMAX, axis_code(axis), 0),
        OpNode::Stencil { kind, halo, axis } => (
            OP_STENCIL,
            stencil_code(kind) | ((halo as u64) << 16) | (axis_code(axis) << 32),
            0,
        ),
        OpNode::ScatterAccum { op } => (OP_SCATTER_ACCUM, accum_code(op), 0),
        OpNode::Neighbor { kind, k_or_r, dims, enc } => (
            OP_NEIGHBOR,
            nb_code(kind)
                | ((dims as u64) << 8)
                | (enc_code(enc) << 16)
                | ((k_or_r.to_bits() as u64) << 32),
            0,
        ),
        OpNode::Slice { offset, len } => {
            (OP_SLICE, (offset as u64) | ((len as u64) << 32), 0)
        }
        OpNode::Rope { head_dim, pos, mode, base_bits } => (
            OP_ROPE,
            (head_dim as u64) | ((pos as u64) << 32),
            (mode as u64) | ((base_bits as u64) << 32),
        ),
    })
}

/// Decode `(opcode, word0, word1)` back to an [`OpNode`].
fn decode_op(opcode: u64, w0: u64, w1: u64) -> Result<OpNode, ForgeError> {
    Ok(match opcode {
        OP_ELEMENTWISE => OpNode::Elementwise { f: ewkind_from(w0 & 0xFFFF)? },
        OP_MATMUL => OpNode::MatMul {
            m: (w0 & 0xFFFF_FFFF) as u32,
            n: (w0 >> 32) as u32,
            k: (w1 & 0xFFFF_FFFF) as u32,
            tc: (w1 >> 32) & 1 == 1,
            trans_b: (w1 >> 33) & 1 == 1,
        },
        OP_GEMV => OpNode::Gemv { m: (w0 & 0xFFFF_FFFF) as u32, n: (w0 >> 32) as u32 },
        OP_FFT => OpNode::Fft { len: (w0 & 0xFFFF_FFFF) as u32, inverse: (w0 >> 32) & 1 == 1 },
        OP_REDUCE => OpNode::Reduce {
            op: redkind_from(w0 & 0xFFFF)?,
            axis: axis_from((w0 >> 16) & 0xFFFF)?,
        },
        OP_GATHER_DEQUANT => OpNode::GatherDequant {
            scheme: dtype_from(w0 & 0xFFFF)?,
            block: (w0 >> 32) as u32,
        },
        OP_BROADCAST => OpNode::Broadcast {
            shape: Shape { dims: unpack_dims(w0), rank: (w1 & 0xFF) as u8 },
        },
        OP_SOFTMAX => OpNode::Softmax { axis: axis_from(w0)? },
        OP_STENCIL => OpNode::Stencil {
            kind: stencil_from(w0 & 0xFFFF)?,
            halo: ((w0 >> 16) & 0xFFFF) as u32,
            axis: axis_from((w0 >> 32) & 0xFFFF)?,
        },
        OP_SCATTER_ACCUM => OpNode::ScatterAccum { op: accum_from(w0 & 0xFFFF)? },
        OP_NEIGHBOR => OpNode::Neighbor {
            kind: nb_from(w0 & 0xFF)?,
            dims: ((w0 >> 8) & 0xFF) as u8,
            enc: enc_from((w0 >> 16) & 0xFF)?,
            k_or_r: f32::from_bits((w0 >> 32) as u32),
        },
        OP_SLICE => OpNode::Slice {
            offset: (w0 & 0xFFFF_FFFF) as u32,
            len: (w0 >> 32) as u32,
        },
        OP_ROPE => OpNode::Rope {
            head_dim: (w0 & 0xFFFF_FFFF) as u32,
            pos: (w0 >> 32) as u32,
            mode: (w1 & 0xFFFF_FFFF) as u32,
            base_bits: (w1 >> 32) as u32,
        },
        other => return Err(err(format!("q42: unknown op-kind opcode {other:#x}"))),
    })
}

/// Pack an edge's rank/dtype/layout into one `u64` (the `parity` field of a quin).
fn pack_rank_dtype_layout(rank: u8, dtype: DType, layout: Layout) -> u64 {
    (rank as u64) | (dtype_code(dtype) << 8) | (layout_code(layout) << 16)
}

/// Serialize a [`ComputeGraph`] to a flat `Vec<NQuin>` (the persistence/provenance view). The
/// in-arena graph remains the source of truth; this is byte-for-byte what a q42 superblock
/// would store. Round-trips through [`deserialize_graph`] to an identical graph.
pub fn serialize_graph(graph: &ComputeGraph) -> Result<Vec<NQuin>, ForgeError> {
    let mut quins = Vec::with_capacity(graph.nodes.len() * 4 + graph.outputs.len());
    for (i, node) in graph.nodes.iter().enumerate() {
        let id = i as u64;
        // op-kind + payload
        let (opcode, w0, w1) = encode_op(&node.op)?;
        quins.push(NQuin {
            subject: id,
            predicate: P_OPKIND,
            object: opcode,
            context: w0,
            metadata: 0,
            parity: w1,
        });
        // output tensor shape
        quins.push(NQuin {
            subject: id,
            predicate: P_SHAPE,
            object: pack_dims(node.out.shape.dims)?,
            context: 0,
            metadata: 0,
            parity: node.out.shape.rank as u64,
        });
        // output dtype
        quins.push(NQuin {
            subject: id,
            predicate: P_DTYPE,
            object: dtype_code(node.out.dtype),
            context: 0,
            metadata: 0,
            parity: 0,
        });
        // schedule hint
        quins.push(NQuin {
            subject: id,
            predicate: P_SCHED,
            object: (node.sched.workgroup_size as u64)
                | ((node.sched.items_per_invocation as u64) << 32),
            context: 0,
            metadata: 0,
            parity: node.sched.vector_width as u64,
        });
        // input edges (feedsInto)
        for slot in 0..node.n_in as usize {
            let inp = node.ins[slot].ok_or_else(|| err("q42: declared input missing"))?;
            quins.push(NQuin {
                subject: inp.producer.0 as u64,
                predicate: P_FEEDS,
                object: id,
                context: (slot as u64) | ((inp.tensor.0 as u64) << 32),
                metadata: pack_dims(inp.shape.dims)?,
                parity: pack_rank_dtype_layout(inp.shape.rank, inp.dtype, inp.layout),
            });
        }
    }
    for out in &graph.outputs {
        quins.push(NQuin {
            subject: out.0 as u64,
            predicate: P_OUTPUT,
            object: 1,
            context: 0,
            metadata: 0,
            parity: 0,
        });
    }
    Ok(quins)
}

fn input_tensorref_from_feeds(q: &NQuin) -> Result<(u32, u32, TensorRef), ForgeError> {
    let consumer = q.object as u32;
    let slot = (q.context & 0xFFFF_FFFF) as u32;
    let tensor = (q.context >> 32) as u32;
    let producer = if q.subject == u32::MAX as u64 {
        NodeId::EXTERNAL
    } else {
        NodeId(q.subject as u32)
    };
    let dims = unpack_dims(q.metadata);
    let rank = (q.parity & 0xFF) as u8;
    let dtype = dtype_from((q.parity >> 8) & 0xFF)?;
    let layout = layout_from((q.parity >> 16) & 0xFF)?;
    let tr = TensorRef {
        producer,
        tensor: TensorId(tensor),
        shape: Shape { dims, rank },
        dtype,
        layout,
    };
    Ok((consumer, slot, tr))
}

/// Reconstruct a [`ComputeGraph`] from its [`NQuin`] encoding. The inverse of
/// [`serialize_graph`]: `deserialize_graph(serialize_graph(g))` equals `g` node-for-node.
pub fn deserialize_graph(quins: &[NQuin]) -> Result<ComputeGraph, ForgeError> {
    let mut opkind: HashMap<u32, (u64, u64, u64)> = HashMap::new();
    let mut shape: HashMap<u32, ([u32; 4], u8)> = HashMap::new();
    let mut dtype: HashMap<u32, DType> = HashMap::new();
    let mut sched: HashMap<u32, Schedule> = HashMap::new();
    let mut edges: Vec<(u32, u32, TensorRef)> = Vec::new();
    let mut outputs: Vec<u32> = Vec::new();

    for q in quins {
        match q.predicate {
            P_OPKIND => {
                opkind.insert(q.subject as u32, (q.object, q.context, q.parity));
            }
            P_SHAPE => {
                shape.insert(q.subject as u32, (unpack_dims(q.object), (q.parity & 0xFF) as u8));
            }
            P_DTYPE => {
                dtype.insert(q.subject as u32, dtype_from(q.object)?);
            }
            P_SCHED => {
                sched.insert(
                    q.subject as u32,
                    Schedule {
                        workgroup_size: (q.object & 0xFFFF_FFFF) as u32,
                        items_per_invocation: (q.object >> 32) as u32,
                        vector_width: (q.parity & 0xFFFF_FFFF) as u32,
                    },
                );
            }
            P_FEEDS => edges.push(input_tensorref_from_feeds(q)?),
            P_OUTPUT => outputs.push(q.subject as u32),
            _ => {} // foreign predicate — ignore (a quin store may hold many kinds)
        }
    }

    let n_nodes = opkind.keys().copied().max().map(|m| m as usize + 1).unwrap_or(0);
    let mut g = ComputeGraph::new();
    let mut out_refs: Vec<TensorRef> = Vec::with_capacity(n_nodes);
    for i in 0..n_nodes {
        let iu = i as u32;
        let (opcode, w0, w1) = *opkind
            .get(&iu)
            .ok_or_else(|| err(format!("q42: node {i} missing opKind quin")))?;
        let op = decode_op(opcode, w0, w1)?;
        let (dims, rank) = *shape
            .get(&iu)
            .ok_or_else(|| err(format!("q42: node {i} missing tensorShape quin")))?;
        let out_dtype = *dtype
            .get(&iu)
            .ok_or_else(|| err(format!("q42: node {i} missing dtype quin")))?;
        let s = *sched
            .get(&iu)
            .ok_or_else(|| err(format!("q42: node {i} missing scheduleHint quin")))?;
        let mut ins: Vec<(u32, TensorRef)> = edges
            .iter()
            .filter(|(c, _, _)| *c == iu)
            .map(|(_, slot, tr)| (*slot, *tr))
            .collect();
        ins.sort_by_key(|(slot, _)| *slot);
        let in_refs: Vec<TensorRef> = ins.into_iter().map(|(_, tr)| tr).collect();
        let out = g.push(op, &in_refs, Shape { dims, rank }, out_dtype, s)?;
        out_refs.push(out);
    }
    for o in outputs {
        let r = *out_refs
            .get(o as usize)
            .ok_or_else(|| err(format!("q42: graphOutput references missing node {o}")))?;
        g.mark_output(r);
    }
    Ok(g)
}

/// blake3 Merkle root over the flat `NQuin` byte image — the graph's content address
/// (provenance). Stable across serialize→bytes→deserialize→serialize. This is the value a q42
/// volume header / `DagStore` node would carry as the graph's `quins_merkle`.
pub fn graph_merkle_root(quins: &[NQuin]) -> [u8; 32] {
    let bytes: &[u8] = bytemuck::cast_slice(quins);
    *blake3::hash(bytes).as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::graph_ops::executor::{decode_block_graph, softmax_graph};
    use crate::wgsl_forge::ir::graph::OpNode;

    /// Round-trip a graph → quins → graph and assert node-for-node identity (the contract).
    fn assert_roundtrips(g: &ComputeGraph) {
        let quins = serialize_graph(g).expect("serialize");
        let g2 = deserialize_graph(&quins).expect("deserialize");
        assert_eq!(g2.nodes.len(), g.nodes.len(), "node count");
        for (i, (a, b)) in g.nodes.iter().zip(g2.nodes.iter()).enumerate() {
            assert_eq!(a, b, "node {i} mismatch");
        }
        assert_eq!(g2.outputs, g.outputs, "outputs");
        // The byte image is stable across a second serialization (zero-copy, deterministic).
        let quins2 = serialize_graph(&g2).expect("serialize2");
        assert_eq!(graph_merkle_root(&quins), graph_merkle_root(&quins2), "merkle");
    }

    #[test]
    fn softmax_graph_roundtrips_identically() {
        let g = softmax_graph(32).unwrap();
        assert_roundtrips(&g);
    }

    #[test]
    fn decode_block_graph_roundtrips_identically() {
        // Exercises MatMul (u32 dims), Reduce/Broadcast/Elementwise, residual adds — the full
        // node mix of a transformer decode block.
        let g = decode_block_graph(8, 12, 16).unwrap();
        assert_roundtrips(&g);
    }

    #[test]
    fn every_op_class_roundtrips() {
        // A hand-built single-node graph per op-class proves encode/decode of every payload.
        use crate::wgsl_forge::ir::graph::{
            AccumKind, Axis, DType, NbKind, NeighborEnc, RedKind, StencilKind, TensorRef,
        };
        let dyn1 = Shape::new(&[4]);
        let ops = [
            OpNode::Elementwise { f: EwKind::Gelu },
            OpNode::MatMul { m: 70000, n: 3, k: 99999, tc: true, trans_b: true },
            OpNode::Gemv { m: 5, n: 6 },
            OpNode::Fft { len: 1024, inverse: true },
            OpNode::Reduce { op: RedKind::L2, axis: Axis::Index(2) },
            OpNode::GatherDequant { scheme: DType::Ternary, block: 64 },
            OpNode::Broadcast { shape: Shape::new(&[3, 5]) },
            OpNode::Softmax { axis: Axis::Penultimate },
            OpNode::Stencil { kind: StencilKind::Advection, halo: 2, axis: Axis::Last },
            OpNode::ScatterAccum { op: AccumKind::Max },
            OpNode::Neighbor { kind: NbKind::Knn, k_or_r: 3.5, dims: 3, enc: NeighborEnc::Project },
            OpNode::Slice { offset: 7, len: 13 },
            OpNode::Rope { head_dim: 64, pos: 9, mode: 1, base_bits: 10000.0f32.to_bits() },
        ];
        for op in ops {
            let (opcode, w0, w1) = encode_op(&op).expect("encode");
            let back = decode_op(opcode, w0, w1).expect("decode");
            assert_eq!(op, back, "op {op:?} payload round-trip");
            // And a one-node graph round-trips end-to-end (with one external input).
            let mut g = ComputeGraph::new();
            let inp = TensorRef::external(dyn1, DType::F32);
            let out = g
                .push(op, &[inp], dyn1, DType::F32, Schedule::default())
                .expect("push");
            g.mark_output(out);
            assert_roundtrips(&g);
        }
    }

    /// The byte image is pure `NQuin` (zero-copy): serialize → bytes → quins → graph reproduces
    /// the graph. This is the on-disk persistence path (a q42 superblock holds exactly these bytes).
    #[test]
    fn graph_survives_a_byte_roundtrip() {
        let g = softmax_graph(16).unwrap();
        let quins = serialize_graph(&g).unwrap();
        // To raw bytes and back (what a q42 volume read/write does).
        let bytes: Vec<u8> = bytemuck::cast_slice(&quins).to_vec();
        let restored: &[NQuin] = bytemuck::cast_slice(&bytes);
        let g2 = deserialize_graph(restored).unwrap();
        assert_eq!(g.nodes, g2.nodes);
        assert_eq!(graph_merkle_root(&quins), graph_merkle_root(restored));
    }

    /// The op-kind opcodes live in the reserved 0x10+ modality range and never collide with
    /// `mini_parser`'s 0x00–0x04 or the deontic 0x50+ opcodes (core invariant §6).
    #[test]
    fn opcodes_are_in_the_reserved_modality_range() {
        let codes = [
            OP_ELEMENTWISE, OP_MATMUL, OP_GEMV, OP_FFT, OP_REDUCE, OP_GATHER_DEQUANT,
            OP_BROADCAST, OP_SOFTMAX, OP_STENCIL, OP_SCATTER_ACCUM, OP_NEIGHBOR,
        ];
        for &c in &codes {
            assert!((0x10..=0x1A).contains(&c), "opcode {c:#x} out of reserved range");
            assert!(c > 0x04, "collides with mini_parser 0x00-0x04");
            assert!(!(0x50..=0x5F).contains(&c), "collides with deontic 0x50+");
        }
        // All distinct.
        let mut sorted = codes.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), codes.len(), "opcodes must be distinct");
    }

    /// A dim exceeding `u16` is a hard error in the shape pack — never a silent truncation.
    #[test]
    fn oversize_shape_dim_errors_not_truncates() {
        assert!(pack_dims([70000, 1, 1, 1]).is_err());
    }
}
