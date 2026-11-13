//! The existing PLE graph with actual token input and session-owned storage.
use super::{
    attention::norm,
    plan::Plan,
    primitives::{gelu_pytorch_tanh, named_layer_tensor},
};
use crate::gguf_bridge::{QTensorEngine, cpu_ops::rms_norm_inplace};

pub(super) fn prepare(
    e: &QTensorEngine,
    plan: &Plan,
    token: u32,
    scaled_embedding: &[f32],
    per_layer: &mut [f32],
    projected: &mut [f32],
) -> Option<()> {
    let p = &plan.profile;
    let count = p.ple * p.layers;
    if p.ple == 0 {
        return (per_layer.is_empty() && projected.is_empty()).then_some(());
    }
    if per_layer.len() != count || projected.len() != count {
        return None;
    }
    if plan
        .index
        .dequantize_ple_row_into(e.model_bytes()?, u64::from(token), per_layer)
        != count
    {
        return None;
    }
    if !e.dispatch_gemm_into(
        &plan.index,
        &plan.ple_projection?,
        scaled_embedding,
        projected,
        p.hidden,
        count,
    ) {
        return None;
    }
    let mut weights = [0.; 512];
    if e.dequant_norm_tensor_into(&plan.ple_norm?, &mut weights[..p.ple]) != p.ple {
        return None;
    }
    combine(
        p.ple,
        p.layers,
        p.hidden,
        p.epsilon,
        &weights[..p.ple],
        per_layer,
        projected,
    )
}
pub(super) fn combine(
    dim: usize,
    layers: usize,
    hidden: usize,
    epsilon: f32,
    weights: &[f32],
    token_rows: &mut [f32],
    projected: &mut [f32],
) -> Option<()> {
    if dim == 0
        || hidden == 0
        || weights.len() != dim
        || token_rows.len() != dim * layers
        || projected.len() != token_rows.len()
    {
        return None;
    }
    let embedding_scale = (dim as f32).sqrt();
    let projection_scale = (hidden as f32).powf(-0.5);
    let merge_scale = 2.0f32.powf(-0.5);
    for x in projected.iter_mut() {
        *x *= projection_scale;
    }
    for row in projected.chunks_exact_mut(dim) {
        rms_norm_inplace(row, weights, epsilon);
    }
    for (t, m) in token_rows.iter_mut().zip(projected) {
        *t = (*t * embedding_scale + *m) * merge_scale;
    }
    token_rows.iter().all(|v| v.is_finite()).then_some(())
}
pub(super) fn execute(
    e: &QTensorEngine,
    plan: &Plan,
    layer: usize,
    hidden: &mut [f32],
    per_layer: &[f32],
    out: &mut [f32],
) -> Option<()> {
    let p = &plan.profile;
    if p.ple == 0 {
        return Some(());
    }
    let input = named_layer_tensor(&plan.index, layer as u32, b"inp_gate.weight")?;
    let projection = named_layer_tensor(&plan.index, layer as u32, b"proj.weight")?;
    let post = named_layer_tensor(&plan.index, layer as u32, b"post_norm.weight")?;
    let mut gate = [0.; 512];
    if !e.dispatch_gemm_into(
        &plan.index,
        &input,
        hidden,
        &mut gate[..p.ple],
        p.hidden,
        p.ple,
    ) {
        return None;
    }
    for (g, t) in gate[..p.ple]
        .iter_mut()
        .zip(&per_layer[layer * p.ple..(layer + 1) * p.ple])
    {
        *g = gelu_pytorch_tanh(*g) * t;
    }
    if out.len() < p.hidden
        || !e.dispatch_gemm_into(
            &plan.index,
            &projection,
            &gate[..p.ple],
            &mut out[..p.hidden],
            p.ple,
            p.hidden,
        )
    {
        return None;
    }
    norm(e, &post, &mut out[..p.hidden], p.epsilon)?;
    for (h, v) in hidden.iter_mut().zip(&out[..p.hidden]) {
        *h += v;
    }
    hidden.iter().all(|v| v.is_finite()).then_some(())
}
