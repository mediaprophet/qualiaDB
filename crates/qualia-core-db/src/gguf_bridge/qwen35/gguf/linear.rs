//! GGUF packing adapter over pinned Etalon recurrent kernels; no new recurrence.
use super::{Profile, session::Workspace};
use crate::{
    gguf_bridge::QTensorEngine,
    gguf_sharder::{GgufTensorIndex, LayerTensors},
};
use qualia_inference_kernel::recurrent::{
    QwenHeadLayout, gated_deltanet::step_causal_conv1d, gated_rms_norm, normalize_qwen_qk,
    qwen_gates, silu_into, step_qwen_delta,
};

pub(super) fn run(
    e: &QTensorEngine,
    index: &GgufTensorIndex,
    p: Profile,
    t: &LayerTensors,
    hidden: &mut [f32],
    state: &mut [f32],
    w: &mut Workspace,
) -> Option<()> {
    norm(
        e,
        t.attn_norm.as_ref()?,
        hidden,
        p.epsilon,
        &mut w.norm,
        &mut w.input,
    )?;
    for (info, out) in [
        (t.attn_qkv?, &mut w.qkv[..]),
        (t.attn_gate?, &mut w.z[..]),
        (t.ssm_alpha?, &mut w.alpha[..]),
        (t.ssm_beta?, &mut w.beta_proj[..]),
    ] {
        let output_len = out.len();
        if !e.dispatch_gemm_into(index, &info, &w.input, out, p.hidden, output_len) {
            return None;
        }
    }
    let info = t.ssm_conv1d?;
    let raw = e.fetch_tensor_raw_bytes(&info)?;
    if etalon_model::ggml::dequantize_row_into(raw, info.ggml_type, p.channels * 4, &mut w.conv)
        != Ok(p.channels * 4)
    {
        return None;
    }
    let (history, delta) = state.split_at_mut(3 * p.channels);
    step_causal_conv1d(history, &w.conv, &w.qkv, &mut w.activated).ok()?;
    silu_into(&w.activated, &mut w.qkv).ok()?;
    let layout = QwenHeadLayout {
        key_heads: p.kh,
        value_heads: p.vh,
        key_dim: p.kd,
        value_dim: p.vd,
    };
    normalize_qwen_qk(layout, &w.qkv, &mut w.q, &mut w.k).ok()?;
    if e.dequant_norm_tensor_into(&t.ssm_a?, &mut w.a_log) != p.vh
        || e.dequant_norm_tensor_into(&t.ssm_dt_bias?, &mut w.dt) != p.vh
    {
        return None;
    }
    // llama.cpp exports -exp(A_log); positive/raw HF values are a different profile.
    for a in &mut w.a_log {
        if !a.is_finite() || *a >= 0. {
            return None;
        }
        *a = (-*a).ln();
    }
    qwen_gates(
        &w.alpha,
        &w.beta_proj,
        &w.a_log,
        &w.dt,
        &mut w.decay,
        &mut w.beta,
    )
    .ok()?;
    let value = &w.qkv[2 * p.kh * p.kd..];
    for head in 0..p.vh {
        // GGUF value heads are tiled, whereas the canonical Q/K expansion is grouped.
        let qs = tiled_query_offset(p, head);
        let vs = head * p.vd;
        let ss = head * p.kd * p.vd;
        step_qwen_delta(
            &mut delta[ss..ss + p.kd * p.vd],
            &w.q[qs..qs + p.kd],
            &w.k[qs..qs + p.kd],
            &value[vs..vs + p.vd],
            w.decay[head],
            w.beta[head],
            p.kd,
            p.vd,
            &mut w.recurrent[vs..vs + p.vd],
        )
        .ok()?;
    }
    if e.dequant_norm_tensor_into(&t.ssm_norm?, &mut w.ssm_norm) != p.vd {
        return None;
    }
    for head in 0..p.vh {
        let r = head * p.vd..(head + 1) * p.vd;
        gated_rms_norm(
            &w.recurrent[r.clone()],
            &w.z[r.clone()],
            &w.ssm_norm,
            p.epsilon,
            &mut w.gated[r],
        )
        .ok()?;
    }
    if !e.dispatch_gemm_into(
        index,
        &t.ssm_out?,
        &w.gated,
        &mut w.output,
        p.value,
        p.hidden,
    ) {
        return None;
    }
    for (h, v) in hidden.iter_mut().zip(&w.output) {
        *h += v;
    }
    hidden.iter().all(|v| v.is_finite()).then_some(())
}
pub(super) fn tiled_query_offset(p: Profile, head: usize) -> usize {
    (head % p.kh) * (p.vh / p.kh) * p.kd
}
pub(super) fn norm(
    e: &QTensorEngine,
    info: &etalon_model::inference::TensorInfo,
    hidden: &[f32],
    epsilon: f32,
    weights: &mut [f32],
    out: &mut [f32],
) -> Option<()> {
    if e.dequant_norm_tensor_into(info, weights) != hidden.len() {
        return None;
    }
    etalon_inference::ordinary::qwen35::ordinary_rms_norm(hidden, weights, false, epsilon, out).ok()
}
