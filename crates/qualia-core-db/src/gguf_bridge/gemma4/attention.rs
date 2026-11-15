//! Existing Gemma normalized Q/K, unit-normalized V and unscaled causal GQA,
//! adapted to the admitted per-layer cache layout instead of a uniform arena.
use super::{
    plan::Plan,
    primitives::{named_layer_tensor, rope_neox_partial_inplace, unit_rms_norm_inplace},
};
use crate::gguf_bridge::{QTensorEngine, cpu_ops::rms_norm_inplace};

pub(super) fn execute(
    e: &QTensorEngine,
    plan: &Plan,
    layer: usize,
    position: u32,
    hidden: &mut [f32],
    cache: &mut [f32],
    written: &mut [Option<u32>],
    out: &mut [f32],
) -> Option<()> {
    let p = &plan.profile;
    let l = &plan.layers[layer];
    let t = &l.tensors;
    let dim = l.dim;
    let qn = p.heads * dim;
    let kn = l.kv_heads * dim;
    let mut normalized = [0.; 4096];
    normalized[..p.hidden].copy_from_slice(hidden);
    norm(e, &t.attn_norm?, &mut normalized[..p.hidden], p.epsilon)?;
    let mut q = [0.; 8192];
    if !e.dispatch_gemm_into(
        &plan.index,
        &t.attn_q?,
        &normalized[..p.hidden],
        &mut q[..qn],
        p.hidden,
        qn,
    ) {
        return None;
    }
    let mut weights = [0.; 512];
    if e.dequant_norm_tensor_into(&t.attn_q_norm?, &mut weights[..dim]) != dim {
        return None;
    }
    for head in q[..qn].chunks_exact_mut(dim) {
        rms_norm_inplace(head, &weights[..dim], p.epsilon);
    }
    let mut frequencies = [1.; 256];
    let factors = if l.sliding {
        None
    } else {
        let n = p.head_dim / 2;
        if e.dequant_norm_tensor_into(&plan.rope, &mut frequencies[..n]) != n
            || frequencies[..n].iter().any(|f| !f.is_finite() || *f <= 0.)
        {
            return None;
        }
        Some(&frequencies[..n])
    };
    let rope_dim = if l.sliding {
        p.swa_rope_dim
    } else {
        p.rope_dim
    };
    let base = if l.sliding { p.swa_base } else { p.rope_base };
    rope_neox_partial_inplace(
        &mut q[..qn],
        p.heads,
        dim,
        rope_dim,
        position,
        base,
        1.,
        factors,
    );
    if l.source == layer {
        let mut k = [0.; 4096];
        let mut v = [0.; 4096];
        if !e.dispatch_gemm_into(
            &plan.index,
            &t.attn_k?,
            &normalized[..p.hidden],
            &mut k[..kn],
            p.hidden,
            kn,
        ) {
            return None;
        }
        if let Some(info) = t.attn_v {
            if !e.dispatch_gemm_into(
                &plan.index,
                &info,
                &normalized[..p.hidden],
                &mut v[..kn],
                p.hidden,
                kn,
            ) {
                return None;
            }
        } else {
            v[..kn].copy_from_slice(&k[..kn]);
        }
        if e.dequant_norm_tensor_into(&t.attn_k_norm?, &mut weights[..dim]) != dim {
            return None;
        }
        for head in k[..kn].chunks_exact_mut(dim) {
            rms_norm_inplace(head, &weights[..dim], p.epsilon);
        }
        for head in v[..kn].chunks_exact_mut(dim) {
            unit_rms_norm_inplace(head, p.epsilon);
        }
        rope_neox_partial_inplace(
            &mut k[..kn],
            l.kv_heads,
            dim,
            rope_dim,
            position,
            base,
            1.,
            factors,
        );
        if k[..kn].iter().chain(&v[..kn]).any(|v| !v.is_finite()) {
            return None;
        }
        let slot = l.offset + (position as usize % l.slots) * 2 * kn;
        cache.get_mut(slot..slot + kn)?.copy_from_slice(&k[..kn]);
        cache
            .get_mut(slot + kn..slot + 2 * kn)?
            .copy_from_slice(&v[..kn]);
        written[layer] = Some(position);
    }
    if written[l.source] != Some(position) {
        return None;
    }
    let start = if l.sliding {
        (position as usize + 1).saturating_sub(p.window)
    } else {
        0
    };
    let mut attention = [0.; 8192];
    causal_gqa(
        &q[..qn],
        &cache[l.offset..l.offset + l.slots * 2 * kn],
        dim,
        p.heads,
        l.kv_heads,
        l.slots,
        start,
        position as usize,
        &mut attention[..qn],
    )?;
    if out.len() < p.hidden
        || !e.dispatch_gemm_into(
            &plan.index,
            &t.attn_output?,
            &attention[..qn],
            &mut out[..p.hidden],
            qn,
            p.hidden,
        )
    {
        return None;
    }
    norm(
        e,
        &named_layer_tensor(&plan.index, layer as u32, b"post_attention_norm.weight")?,
        &mut out[..p.hidden],
        p.epsilon,
    )?;
    for (h, v) in hidden.iter_mut().zip(&out[..p.hidden]) {
        *h += v;
    }
    hidden.iter().all(|v| v.is_finite()).then_some(())
}
#[allow(clippy::too_many_arguments)]
pub(super) fn causal_gqa(
    q: &[f32],
    kv: &[f32],
    dim: usize,
    heads: usize,
    kv_heads: usize,
    slots: usize,
    start: usize,
    position: usize,
    out: &mut [f32],
) -> Option<()> {
    if dim == 0
        || kv_heads == 0
        || heads % kv_heads != 0
        || slots == 0
        || start > position
        || position - start >= slots
        || q.len() != heads * dim
        || out.len() != q.len()
        || kv.len() != slots * 2 * kv_heads * dim
    {
        return None;
    }
    out.fill(0.);
    let kn = kv_heads * dim;
    let gqa = heads / kv_heads;
    for qh in 0..heads {
        let kh = qh / gqa;
        let query = &q[qh * dim..(qh + 1) * dim];
        let dest = &mut out[qh * dim..(qh + 1) * dim];
        let mut maximum = f32::NEG_INFINITY;
        let mut sum = 0.;
        for tok in start..=position {
            let offset = (tok % slots) * 2 * kn;
            let k = &kv[offset + kh * dim..offset + (kh + 1) * dim];
            let v = &kv[offset + kn + kh * dim..offset + kn + (kh + 1) * dim];
            let score: f32 = query.iter().zip(k).map(|(q, k)| q * k).sum();
            if !score.is_finite() {
                return None;
            }
            if score > maximum {
                let rescale = (maximum - score).exp();
                for x in dest.iter_mut() {
                    *x *= rescale;
                }
                sum = sum * rescale + 1.;
                maximum = score;
                for (x, v) in dest.iter_mut().zip(v) {
                    *x += v;
                }
            } else {
                let weight = (score - maximum).exp();
                sum += weight;
                for (x, v) in dest.iter_mut().zip(v) {
                    *x += weight * v;
                }
            }
        }
        if !sum.is_finite() || sum <= 0. {
            return None;
        }
        for x in dest {
            *x /= sum;
        }
    }
    out.iter().all(|v| v.is_finite()).then_some(())
}
pub(super) fn norm(
    e: &QTensorEngine,
    info: &etalon_model::inference::TensorInfo,
    x: &mut [f32],
    epsilon: f32,
) -> Option<()> {
    let mut w = [0.; 4096];
    if x.len() > w.len() || e.dequant_norm_tensor_into(info, &mut w[..x.len()]) != x.len() {
        return None;
    }
    let width = x.len();
    rms_norm_inplace(x, &w[..width], epsilon);
    x.iter().all(|v| v.is_finite()).then_some(())
}
