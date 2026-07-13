//! Graph builders for the canonical multi-node LLM ops — softmax, RMSNorm, SwiGLU-FFN,
//! attention, the full decode block, the real multi-head (GQA) decode layer, and the
//! ternary dequant-matmul — plus the composable sub-block helpers (`push_rmsnorm` /
//! `push_softmax`) that append nodes to an existing graph.

use crate::wgsl_forge::ir::graph::{
    Axis, ComputeGraph, DType, EwKind, OpNode, RedKind, Shape, TensorRef,
};
use crate::wgsl_forge::{ForgeError, Schedule};

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
