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

use crate::wgsl_forge::ir::graph::AccumKind;
use crate::wgsl_forge::ir::graph::{
    Axis, ComputeGraph, DType, EwKind, Layout, NbKind, NeighborEnc, NodeId, OpNode, RedKind, Shape,
    StencilKind, TensorId, TensorRef,
};
use crate::wgsl_forge::{ForgeError, Schedule};
use crate::{q_hash, NQuin};

// ── Predicates (the q42 data-flow namespace) ─────────────────────────────────────────
const P_OPKIND: u64 = q_hash("urn:qualia:q42:opKind");
pub const P_OPPAYLOAD: u64 = q_hash("urn:qualia:q42:opPayload");
const P_SHAPE: u64 = q_hash("urn:qualia:q42:tensorShape");
const P_DTYPE: u64 = q_hash("urn:qualia:q42:dtype");
const P_SCHED: u64 = q_hash("urn:qualia:q42:scheduleHint");
const P_FEEDS: u64 = q_hash("urn:qualia:q42:feedsInto");
const P_OUTPUT: u64 = q_hash("urn:qualia:q42:graphOutput");

// ── Etalon evaluation & oracle predicates (ETA-14 / ETA-16) ──────────────────────────
pub const P_ETALON_EVAL_BACKEND: u64 = q_hash("urn:qualia:etalon:eval:deviceBackend");
pub const P_ETALON_EVAL_QUIN_COUNT: u64 = q_hash("urn:qualia:etalon:eval:quinCount");
pub const P_ETALON_EVAL_SIEVE_MATCH: u64 = q_hash("urn:qualia:etalon:eval:sieveMatchesCpu");
pub const P_ETALON_EVAL_RADIX_MATCH: u64 = q_hash("urn:qualia:etalon:eval:radixOrderMatchesCpu");
pub const P_ETALON_EVAL_PARITY_VERIFIED: u64 = q_hash("urn:qualia:etalon:eval:parityVerifiedAll");
pub const P_ETALON_EVAL_DISQUALIFIED: u64 = q_hash("urn:qualia:etalon:eval:disqualified");
pub const P_ETALON_ORACLE_MAX_ABS_ERR: u64 = q_hash("urn:qualia:etalon:oracle:verifiedMaxAbsError");
pub const S_ETALON_ORACLE_CALIBRATION: u64 =
    q_hash("urn:qualia:etalon:oracle:gpu_cpu_differential");
pub const C_IDF_CLAIM_PLANE: u64 = q_hash("urn:qualia:idf:plane:ClaimOpinion");
pub const C_IDF_INSTRUMENT_PLANE: u64 = q_hash("urn:qualia:idf:plane:Instrument");

#[inline(always)]
const fn make_quin(
    subject: u64,
    predicate: u64,
    object: u64,
    context: u64,
    metadata: u64,
) -> NQuin {
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity: subject ^ predicate ^ object ^ context ^ metadata,
    }
}

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
const OP_POOL2D: u64 = 0x1D;
const OP_RESIZE2D: u64 = 0x1E;
const OP_CONV2D: u64 = 0x1F;

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
        OpNode::Pool2d { .. } => OP_POOL2D,
        OpNode::Resize2d { .. } => OP_RESIZE2D,
        OpNode::Conv2d { .. } => OP_CONV2D,
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
        OpNode::MatMul {
            m,
            n,
            k,
            tc,
            trans_b,
        } => (
            OP_MATMUL,
            (m as u64) | ((n as u64) << 32),
            (k as u64) | ((tc as u64) << 32) | ((trans_b as u64) << 33),
        ),
        OpNode::Gemv { m, n, tc } => (OP_GEMV, (m as u64) | ((n as u64) << 32), tc as u64),
        OpNode::Fft { len, inverse } => (OP_FFT, (len as u64) | ((inverse as u64) << 32), 0),
        OpNode::Reduce { op, axis } => (OP_REDUCE, redkind_code(op) | (axis_code(axis) << 16), 0),
        OpNode::GatherDequant { scheme, block } => (
            OP_GATHER_DEQUANT,
            dtype_code(scheme) | ((block as u64) << 32),
            0,
        ),
        OpNode::Broadcast { shape } => (OP_BROADCAST, pack_dims(shape.dims)?, shape.rank as u64),
        OpNode::Softmax { axis } => (OP_SOFTMAX, axis_code(axis), 0),
        OpNode::Stencil { kind, halo, axis } => (
            OP_STENCIL,
            stencil_code(kind) | ((halo as u64) << 16) | (axis_code(axis) << 32),
            0,
        ),
        OpNode::ScatterAccum { op } => (OP_SCATTER_ACCUM, accum_code(op), 0),
        OpNode::Neighbor {
            kind,
            k_or_r,
            dims,
            enc,
        } => (
            OP_NEIGHBOR,
            nb_code(kind)
                | ((dims as u64) << 8)
                | (enc_code(enc) << 16)
                | ((k_or_r.to_bits() as u64) << 32),
            0,
        ),
        OpNode::Slice { offset, len } => (OP_SLICE, (offset as u64) | ((len as u64) << 32), 0),
        OpNode::Rope {
            head_dim,
            pos,
            mode,
            base_bits,
        } => (
            OP_ROPE,
            (head_dim as u64) | ((pos as u64) << 32),
            (mode as u64) | ((base_bits as u64) << 32),
        ),
        OpNode::Pool2d {
            c,
            h,
            w,
            kh,
            kw,
            stride_h,
            stride_w,
        } => (
            OP_POOL2D,
            (c as u64)
                | ((h as u64) << 16)
                | ((w as u64) << 32)
                | ((kh as u64) << 48)
                | ((kw as u64) << 56),
            (stride_h as u64) | ((stride_w as u64) << 16),
        ),
        OpNode::Resize2d {
            c,
            h_in,
            w_in,
            h_out,
            w_out,
        } => (
            OP_RESIZE2D,
            (c as u64) | ((h_in as u64) << 16) | ((w_in as u64) << 32) | ((h_out as u64) << 48),
            w_out as u64,
        ),
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
        } => (
            OP_CONV2D,
            (c_in as u64)
                | ((c_out as u64) << 8)
                | ((h as u64) << 16)
                | ((w as u64) << 32)
                | ((kh as u64) << 48)
                | ((kw as u64) << 56),
            (stride_h as u64)
                | ((stride_w as u64) << 8)
                | ((pad_h as u64) << 16)
                | ((pad_w as u64) << 24),
        ),
    })
}

/// Decode `(opcode, word0, word1)` back to an [`OpNode`].
fn decode_op(opcode: u64, w0: u64, w1: u64) -> Result<OpNode, ForgeError> {
    Ok(match opcode {
        OP_ELEMENTWISE => OpNode::Elementwise {
            f: ewkind_from(w0 & 0xFFFF)?,
        },
        OP_MATMUL => OpNode::MatMul {
            m: (w0 & 0xFFFF_FFFF) as u32,
            n: (w0 >> 32) as u32,
            k: (w1 & 0xFFFF_FFFF) as u32,
            tc: (w1 >> 32) & 1 == 1,
            trans_b: (w1 >> 33) & 1 == 1,
        },
        OP_GEMV => OpNode::Gemv {
            m: (w0 & 0xFFFF_FFFF) as u32,
            n: (w0 >> 32) as u32,
            tc: w1 & 1 == 1,
        },
        OP_FFT => OpNode::Fft {
            len: (w0 & 0xFFFF_FFFF) as u32,
            inverse: (w0 >> 32) & 1 == 1,
        },
        OP_REDUCE => OpNode::Reduce {
            op: redkind_from(w0 & 0xFFFF)?,
            axis: axis_from((w0 >> 16) & 0xFFFF)?,
        },
        OP_GATHER_DEQUANT => OpNode::GatherDequant {
            scheme: dtype_from(w0 & 0xFFFF)?,
            block: (w0 >> 32) as u32,
        },
        OP_BROADCAST => OpNode::Broadcast {
            shape: Shape {
                dims: unpack_dims(w0),
                rank: (w1 & 0xFF) as u8,
            },
        },
        OP_SOFTMAX => OpNode::Softmax {
            axis: axis_from(w0)?,
        },
        OP_STENCIL => OpNode::Stencil {
            kind: stencil_from(w0 & 0xFFFF)?,
            halo: ((w0 >> 16) & 0xFFFF) as u32,
            axis: axis_from((w0 >> 32) & 0xFFFF)?,
        },
        OP_SCATTER_ACCUM => OpNode::ScatterAccum {
            op: accum_from(w0 & 0xFFFF)?,
        },
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
        OP_POOL2D => OpNode::Pool2d {
            c: (w0 & 0xFFFF) as u32,
            h: ((w0 >> 16) & 0xFFFF) as u32,
            w: ((w0 >> 32) & 0xFFFF) as u32,
            kh: ((w0 >> 48) & 0xFF) as u32,
            kw: ((w0 >> 56) & 0xFF) as u32,
            stride_h: (w1 & 0xFFFF) as u32,
            stride_w: ((w1 >> 16) & 0xFFFF) as u32,
        },
        OP_RESIZE2D => OpNode::Resize2d {
            c: (w0 & 0xFFFF) as u32,
            h_in: ((w0 >> 16) & 0xFFFF) as u32,
            w_in: ((w0 >> 32) & 0xFFFF) as u32,
            h_out: ((w0 >> 48) & 0xFFFF) as u32,
            w_out: (w1 & 0xFFFF) as u32,
        },
        OP_CONV2D => OpNode::Conv2d {
            c_in: (w0 & 0xFF) as u32,
            c_out: ((w0 >> 8) & 0xFF) as u32,
            h: ((w0 >> 16) & 0xFFFF) as u32,
            w: ((w0 >> 32) & 0xFFFF) as u32,
            kh: ((w0 >> 48) & 0xFF) as u32,
            kw: ((w0 >> 56) & 0xFF) as u32,
            stride_h: (w1 & 0xFF) as u32,
            stride_w: ((w1 >> 8) & 0xFF) as u32,
            pad_h: ((w1 >> 16) & 0xFF) as u32,
            pad_w: ((w1 >> 24) & 0xFF) as u32,
        },
        other => return Err(err(format!("q42: unknown op-kind opcode {other:#x}"))),
    })
}

/// Pack an edge's rank/dtype/layout into a 24-bit value (stored in the upper 32 bits of `P_FEEDS.object`).
fn pack_rank_dtype_layout(rank: u8, dtype: DType, layout: Layout) -> u64 {
    (rank as u64) | (dtype_code(dtype) << 8) | (layout_code(layout) << 16)
}

/// Serialize a [`ComputeGraph`] to a flat `Vec<NQuin>` (the persistence/provenance view). The
/// in-arena graph remains the source of truth; this is byte-for-byte what a q42 superblock
/// would store. Every emitted [`NQuin`] satisfies canonical 5-field XOR parity
/// (`subject ^ predicate ^ object ^ context ^ metadata == parity`) and round-trips through
/// [`deserialize_graph`] to an identical graph.
pub fn serialize_graph(graph: &ComputeGraph) -> Result<Vec<NQuin>, ForgeError> {
    let mut quins = Vec::with_capacity(graph.nodes.len() * 4 + graph.outputs.len());
    for (i, node) in graph.nodes.iter().enumerate() {
        let id = i as u64;
        // op-kind + payload (w0 in context, w1 in metadata; parity is canonical 5-field XOR)
        let (opcode, w0, w1) = encode_op(&node.op)?;
        quins.push(make_quin(id, P_OPKIND, opcode, w0, w1));
        // output tensor shape (rank in context; parity is canonical 5-field XOR)
        quins.push(make_quin(
            id,
            P_SHAPE,
            pack_dims(node.out.shape.dims)?,
            node.out.shape.rank as u64,
            0,
        ));
        // output dtype
        quins.push(make_quin(id, P_DTYPE, dtype_code(node.out.dtype), 0, 0));
        // schedule hint (vector_width in context; parity is canonical 5-field XOR)
        quins.push(make_quin(
            id,
            P_SCHED,
            (node.sched.workgroup_size as u64)
                | ((node.sched.items_per_invocation as u64) << 32),
            node.sched.vector_width as u64,
            0,
        ));
        // input edges (feedsInto: consumer in object[0..31], rank/dtype/layout in object[32..63])
        for slot in 0..node.n_in as usize {
            let inp = node.ins[slot].ok_or_else(|| err("q42: declared input missing"))?;
            let rdl = pack_rank_dtype_layout(inp.shape.rank, inp.dtype, inp.layout);
            quins.push(make_quin(
                inp.producer.0 as u64,
                P_FEEDS,
                (id & 0xFFFF_FFFF) | (rdl << 32),
                (slot as u64) | ((inp.tensor.0 as u64) << 32),
                pack_dims(inp.shape.dims)?,
            ));
        }
    }
    for out in &graph.outputs {
        quins.push(make_quin(out.0 as u64, P_OUTPUT, 1, 0, 0));
    }
    Ok(quins)
}

fn input_tensorref_from_feeds(q: &NQuin) -> Result<(u32, u32, TensorRef), ForgeError> {
    let consumer = (q.object & 0xFFFF_FFFF) as u32;
    let rdl = q.object >> 32;
    let slot = (q.context & 0xFFFF_FFFF) as u32;
    let tensor = (q.context >> 32) as u32;
    let producer = if q.subject == u32::MAX as u64 {
        NodeId::EXTERNAL
    } else {
        NodeId(q.subject as u32)
    };
    let dims = unpack_dims(q.metadata);
    let rank = (rdl & 0xFF) as u8;
    let dtype = dtype_from((rdl >> 8) & 0xFF)?;
    let layout = layout_from((rdl >> 16) & 0xFF)?;
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
/// Verifies canonical 5-field XOR parity on every input [`NQuin`].
pub fn deserialize_graph(quins: &[NQuin]) -> Result<ComputeGraph, ForgeError> {
    let mut opkind: HashMap<u32, (u64, u64, u64)> = HashMap::new();
    let mut oppayload: HashMap<u32, u64> = HashMap::new();
    let mut shape: HashMap<u32, ([u32; 4], u8)> = HashMap::new();
    let mut dtype: HashMap<u32, DType> = HashMap::new();
    let mut sched: HashMap<u32, Schedule> = HashMap::new();
    let mut edges: Vec<(u32, u32, TensorRef)> = Vec::new();
    let mut outputs: Vec<u32> = Vec::new();

    for q in quins {
        if !q.verify_ecc_parity() {
            return Err(err("q42: NQuin 5-field ECC parity check failed"));
        }
        match q.predicate {
            P_OPKIND => {
                opkind.insert(q.subject as u32, (q.object, q.context, q.metadata));
            }
            P_OPPAYLOAD => {
                oppayload.insert(q.subject as u32, q.object);
            }
            P_SHAPE => {
                let rank_raw = if q.context != 0 { q.context } else { q.metadata };
                shape.insert(
                    q.subject as u32,
                    (unpack_dims(q.object), (rank_raw & 0xFF) as u8),
                );
            }
            P_DTYPE => {
                dtype.insert(q.subject as u32, dtype_from(q.object)?);
            }
            P_SCHED => {
                let vw_raw = if q.context != 0 { q.context } else { q.metadata };
                sched.insert(
                    q.subject as u32,
                    Schedule {
                        workgroup_size: (q.object & 0xFFFF_FFFF) as u32,
                        items_per_invocation: (q.object >> 32) as u32,
                        vector_width: (vw_raw & 0xFFFF_FFFF) as u32,
                    },
                );
            }
            P_FEEDS => edges.push(input_tensorref_from_feeds(q)?),
            P_OUTPUT => outputs.push(q.subject as u32),
            _ => {} // foreign predicate — ignore (a quin store may hold many kinds)
        }
    }

    let n_nodes = opkind
        .keys()
        .copied()
        .max()
        .map(|m| m as usize + 1)
        .unwrap_or(0);
    let mut g = ComputeGraph::new();
    let mut out_refs: Vec<TensorRef> = Vec::with_capacity(n_nodes);
    for i in 0..n_nodes {
        let iu = i as u32;
        let (opcode, w0, mut w1) = *opkind
            .get(&iu)
            .ok_or_else(|| err(format!("q42: node {i} missing opKind quin")))?;
        if let Some(&payload_w1) = oppayload.get(&iu) {
            w1 = payload_w1;
        }
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

// ── Packet ETA-14: Zero-Heap Etalon NQuin Evaluation Receipt & Enumeration ───────────

/// Zero-heap evaluation receipt for `etalon-nquin` GPU warp-scan sieve and stable LSD radix sort.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EtalonNQuinEvalReceipt {
    pub device_backend_hash: u64,
    pub quin_count: u32,
    pub sieve_matches_cpu: bool,
    pub radix_order_matches_cpu: bool,
    pub parity_verified_all: bool,
    pub disqualified: bool,
}

impl EtalonNQuinEvalReceipt {
    /// Construct an evaluation receipt and automatically mark `disqualified = true` if any check
    /// failed, `quin_count == 0`, or the backend is unrun/noop/skipped.
    #[inline]
    pub const fn new(
        device_backend_hash: u64,
        quin_count: u32,
        sieve_matches_cpu: bool,
        radix_order_matches_cpu: bool,
        parity_verified_all: bool,
    ) -> Self {
        let is_noop_or_skipped = device_backend_hash == 0
            || device_backend_hash == q_hash("noop")
            || device_backend_hash == q_hash("skipped")
            || device_backend_hash == q_hash("unsupported");
        let disqualified = is_noop_or_skipped
            || quin_count == 0
            || !sieve_matches_cpu
            || !radix_order_matches_cpu
            || !parity_verified_all;
        Self {
            device_backend_hash,
            quin_count,
            sieve_matches_cpu,
            radix_order_matches_cpu,
            parity_verified_all,
            disqualified,
        }
    }

    /// True only when all CPU-oracle and 5-field parity invariants hold on a qualified backend.
    #[inline]
    pub const fn is_qualified(&self) -> bool {
        !self.disqualified
            && self.quin_count > 0
            && self.sieve_matches_cpu
            && self.radix_order_matches_cpu
            && self.parity_verified_all
    }
}

/// Enumerate an [`EtalonNQuinEvalReceipt`] into 6 canonical 48-byte [`NQuin`]s with 5-field XOR
/// parity under the `urn:qualia:etalon:eval:` vocabulary on the `Instrument` and `ClaimOpinion` planes.
#[inline]
pub fn enumerate_etalon_eval_receipt_to_quins(
    receipt: &EtalonNQuinEvalReceipt,
    out: &mut [NQuin; 6],
) {
    let disqualified = receipt.disqualified
        || receipt.quin_count == 0
        || !receipt.sieve_matches_cpu
        || !receipt.radix_order_matches_cpu
        || !receipt.parity_verified_all;
    let s = receipt.device_backend_hash;
    out[0] = make_quin(
        s,
        P_ETALON_EVAL_BACKEND,
        receipt.device_backend_hash,
        C_IDF_INSTRUMENT_PLANE,
        receipt.quin_count as u64,
    );
    out[1] = make_quin(
        s,
        P_ETALON_EVAL_QUIN_COUNT,
        crate::frame_layout::INLINE_TAG_INTEGER | (receipt.quin_count as u64),
        C_IDF_CLAIM_PLANE,
        0,
    );
    out[2] = make_quin(
        s,
        P_ETALON_EVAL_SIEVE_MATCH,
        crate::frame_layout::INLINE_TAG_BOOLEAN | (receipt.sieve_matches_cpu as u64),
        C_IDF_CLAIM_PLANE,
        0,
    );
    out[3] = make_quin(
        s,
        P_ETALON_EVAL_RADIX_MATCH,
        crate::frame_layout::INLINE_TAG_BOOLEAN | (receipt.radix_order_matches_cpu as u64),
        C_IDF_CLAIM_PLANE,
        0,
    );
    out[4] = make_quin(
        s,
        P_ETALON_EVAL_PARITY_VERIFIED,
        crate::frame_layout::INLINE_TAG_BOOLEAN | (receipt.parity_verified_all as u64),
        C_IDF_CLAIM_PLANE,
        0,
    );
    out[5] = make_quin(
        s,
        P_ETALON_EVAL_DISQUALIFIED,
        crate::frame_layout::INLINE_TAG_BOOLEAN | (disqualified as u64),
        C_IDF_CLAIM_PLANE,
        0,
    );
}

/// Qualify a slice of [`NQuin`]s against `etalon_nquin` wire layout (`NQUIN_BYTES == 48`,
/// `NQUIN_ALIGN == 16`), 5-field XOR parity, CPU equality sieve, and CPU stable LSD radix sort.
pub fn qualify_nquin_wire_sieve_and_radix(
    device_backend_hash: u64,
    quins: &[NQuin],
    field: etalon_nquin::QuinField,
    needle: u64,
    candidate_sieve: &[etalon_nquin::NQuinWire],
    candidate_radix_keys: &[u64],
    candidate_radix_indices: &[u32],
) -> EtalonNQuinEvalReceipt {
    assert_eq!(etalon_nquin::NQUIN_BYTES, 48);
    assert_eq!(etalon_nquin::NQUIN_ALIGN, 16);
    assert_eq!(core::mem::size_of::<NQuin>(), etalon_nquin::NQUIN_BYTES);
    assert_eq!(core::mem::align_of::<NQuin>(), etalon_nquin::NQUIN_ALIGN);

    let wire: &[etalon_nquin::NQuinWire] = bytemuck::cast_slice(quins);
    let parity_verified_all = !wire.is_empty()
        && wire
            .iter()
            .zip(quins.iter())
            .all(|(w, q)| w.is_parity_valid() && q.verify_ecc_parity());

    let mut cpu_sieve = vec![etalon_nquin::NQuinWire::new(0, 0, 0, 0, 0, 0); wire.len()];
    let cpu_sieve_len = etalon_nquin::sieve_eq_cpu(wire, field, needle, &mut cpu_sieve);
    let sieve_matches_cpu = &cpu_sieve[..cpu_sieve_len] == candidate_sieve;

    let mut cpu_keys: Vec<u64> = wire.iter().map(|q| field.extract(q)).collect();
    let mut cpu_indices: Vec<u32> = (0..wire.len() as u32).collect();
    let _ = etalon_nquin::radix_sort_u64_cpu(&mut cpu_keys, &mut cpu_indices);
    let radix_order_matches_cpu =
        cpu_keys == candidate_radix_keys && cpu_indices == candidate_radix_indices;

    EtalonNQuinEvalReceipt::new(
        device_backend_hash,
        quins.len() as u32,
        sieve_matches_cpu,
        radix_order_matches_cpu,
        parity_verified_all,
    )
}

// ── Packet ETA-16: CPU-Oracle Differential Tolerance Verification (`mismatches == 0`) ──

/// Numerical comparison summary mirroring `etalon_qualification::NumericalComparison`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumericalComparison {
    pub compared: usize,
    pub mismatches: usize,
    pub max_absolute_error: f64,
}

/// Failure modes for GPU-vs-CPU oracle differential verification (`ADR 0003` / `ETA-16`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OracleVerificationError {
    EmptyInput,
    LengthMismatch,
    InvalidTolerance,
    NaN { index: usize },
    Infinity { index: usize },
    NonFiniteBound { index: usize },
    /// `compare_f32` completed, but `comparison.mismatches > 0`. Never admit `.is_ok()` alone!
    ToleranceExceeded {
        mismatches: usize,
        max_absolute_error_bits: u64,
    },
}

/// Verified differential receipt pairing the `mismatches == 0` comparison with a 5-field parity
/// capability-calibration [`NQuin`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OracleVerificationReceipt {
    pub comparison: NumericalComparison,
    pub compared: usize,
    pub mismatches: usize,
    pub max_absolute_error: f64,
    pub calibration_quin: NQuin,
}

/// Compare a candidate GPU/approximate buffer against a CPU reference oracle using
/// `etalon_qualification::compare_f32` semantics, strictly requiring `comparison.mismatches == 0`
/// before emitting a 5-field XOR parity capability-calibration [`NQuin`].
pub fn verify_gpu_against_cpu_oracle(
    cpu_ref: &[f32],
    gpu_out: &[f32],
    abs_tol: f64,
    rel_tol: f64,
) -> Result<OracleVerificationReceipt, OracleVerificationError> {
    if !abs_tol.is_finite() || !rel_tol.is_finite() || abs_tol < 0.0 || rel_tol < 0.0 {
        return Err(OracleVerificationError::InvalidTolerance);
    }
    if cpu_ref.len() != gpu_out.len() {
        return Err(OracleVerificationError::LengthMismatch);
    }
    if cpu_ref.is_empty() {
        return Err(OracleVerificationError::EmptyInput);
    }

    let mut mismatches = 0usize;
    let mut max_absolute_error = 0.0f64;
    for (index, (&expected, &actual)) in cpu_ref.iter().zip(gpu_out.iter()).enumerate() {
        if expected.is_nan() || actual.is_nan() {
            return Err(OracleVerificationError::NaN { index });
        }
        if !expected.is_finite() || !actual.is_finite() {
            return Err(OracleVerificationError::Infinity { index });
        }
        let difference = (f64::from(expected) - f64::from(actual)).abs();
        let bound = abs_tol + rel_tol * f64::from(expected).abs();
        if !bound.is_finite() {
            return Err(OracleVerificationError::NonFiniteBound { index });
        }
        if difference > max_absolute_error {
            max_absolute_error = difference;
        }
        if difference > bound {
            mismatches += 1;
        }
    }

    if mismatches != 0 {
        return Err(OracleVerificationError::ToleranceExceeded {
            mismatches,
            max_absolute_error_bits: max_absolute_error.to_bits(),
        });
    }

    let comparison = NumericalComparison {
        compared: cpu_ref.len(),
        mismatches: 0,
        max_absolute_error,
    };
    let calibration_quin = make_quin(
        S_ETALON_ORACLE_CALIBRATION,
        P_ETALON_ORACLE_MAX_ABS_ERR,
        crate::frame_layout::pack_float_object(max_absolute_error as f32),
        C_IDF_CLAIM_PLANE,
        (cpu_ref.len() as u64 & 0xFFFF_FFFF) | (((abs_tol as f32).to_bits() as u64) << 32),
    );

    Ok(OracleVerificationReceipt {
        comparison,
        compared: cpu_ref.len(),
        mismatches: 0,
        max_absolute_error,
        calibration_quin,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::graph_ops::executor::{decode_block_graph, softmax_graph};
    use crate::wgsl_forge::ir::graph::OpNode;

    /// Round-trip a graph → quins → graph and assert node-for-node identity (the contract).
    fn assert_roundtrips(g: &ComputeGraph) {
        let quins = serialize_graph(g).expect("serialize");
        for (idx, q) in quins.iter().enumerate() {
            assert!(
                q.verify_ecc_parity(),
                "quin {idx} failed canonical 5-field XOR parity"
            );
        }
        let g2 = deserialize_graph(&quins).expect("deserialize");
        assert_eq!(g2.nodes.len(), g.nodes.len(), "node count");
        for (i, (a, b)) in g.nodes.iter().zip(g2.nodes.iter()).enumerate() {
            assert_eq!(a, b, "node {i} mismatch");
        }
        assert_eq!(g2.outputs, g.outputs, "outputs");
        // The byte image is stable across a second serialization (zero-copy, deterministic).
        let quins2 = serialize_graph(&g2).expect("serialize2");
        assert_eq!(
            graph_merkle_root(&quins),
            graph_merkle_root(&quins2),
            "merkle"
        );
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
            OpNode::MatMul {
                m: 70000,
                n: 3,
                k: 99999,
                tc: true,
                trans_b: true,
            },
            OpNode::Gemv {
                m: 5,
                n: 6,
                tc: true,
            },
            OpNode::Fft {
                len: 1024,
                inverse: true,
            },
            OpNode::Reduce {
                op: RedKind::L2,
                axis: Axis::Index(2),
            },
            OpNode::GatherDequant {
                scheme: DType::Ternary,
                block: 64,
            },
            OpNode::Broadcast {
                shape: Shape::new(&[3, 5]),
            },
            OpNode::Softmax {
                axis: Axis::Penultimate,
            },
            OpNode::Stencil {
                kind: StencilKind::Advection,
                halo: 2,
                axis: Axis::Last,
            },
            OpNode::ScatterAccum { op: AccumKind::Max },
            OpNode::Neighbor {
                kind: NbKind::Knn,
                k_or_r: 3.5,
                dims: 3,
                enc: NeighborEnc::Project,
            },
            OpNode::Slice { offset: 7, len: 13 },
            OpNode::Rope {
                head_dim: 64,
                pos: 9,
                mode: 1,
                base_bits: 10000.0f32.to_bits(),
            },
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
            OP_ELEMENTWISE,
            OP_MATMUL,
            OP_GEMV,
            OP_FFT,
            OP_REDUCE,
            OP_GATHER_DEQUANT,
            OP_BROADCAST,
            OP_SOFTMAX,
            OP_STENCIL,
            OP_SCATTER_ACCUM,
            OP_NEIGHBOR,
            // Slice/Rope/vision (0x1B–0x1F) tested via encode/decode round-trip; range still reserved modality.
        ];
        for &c in &codes {
            assert!(
                (0x10..=0x1F).contains(&c),
                "opcode {c:#x} out of reserved range"
            );
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

    #[test]
    fn deserialize_graph_rejects_corrupted_5_field_parity() {
        let g = softmax_graph(16).unwrap();
        let mut quins = serialize_graph(&g).unwrap();
        quins[0].parity ^= 0x1;
        assert!(matches!(
            deserialize_graph(&quins),
            Err(ForgeError::Serialization(_))
        ));
    }

    #[test]
    fn etalon_eval_receipt_enumerates_six_5_field_parity_quins() {
        let receipt = EtalonNQuinEvalReceipt::new(0xABCD_1234, 64, true, true, true);
        assert!(!receipt.disqualified);
        let zero_quin = make_quin(0, 0, 0, 0, 0);
        let mut out = [zero_quin; 6];
        enumerate_etalon_eval_receipt_to_quins(&receipt, &mut out);
        for (idx, q) in out.iter().enumerate() {
            assert!(
                q.verify_ecc_parity(),
                "receipt quin {idx} failed canonical 5-field parity"
            );
        }
    }

    #[test]
    fn verify_gpu_against_cpu_oracle_requires_zero_mismatches() {
        let cpu = [1.0f32, 2.0, 3.0, 4.0];
        let gpu_ok = [1.0f32, 2.0 + 1e-6, 3.0 - 1e-6, 4.0];
        let ok = verify_gpu_against_cpu_oracle(&cpu, &gpu_ok, 1e-5, 1e-5).expect("within tol");
        assert_eq!(ok.mismatches, 0);
        assert_eq!(ok.comparison.mismatches, 0);
        assert!(ok.calibration_quin.verify_ecc_parity());

        let gpu_bad = [1.0f32, 2.5, 3.0, 4.0];
        assert!(matches!(
            verify_gpu_against_cpu_oracle(&cpu, &gpu_bad, 1e-5, 1e-5),
            Err(OracleVerificationError::ToleranceExceeded { mismatches: 1, .. })
        ));
    }
}
