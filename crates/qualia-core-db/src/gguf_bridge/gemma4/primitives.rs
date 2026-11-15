//! Mechanically extracted existing Gemma/Qwen scalar helpers.
#[inline]
pub(super) fn gelu_pytorch_tanh(x: f32) -> f32 {
    let inner = 0.797_884_56_f32 * (x + 0.044_715_f32 * x * x * x);
    0.5f32 * x * (1.0f32 + inner.tanh())
}

#[inline]
pub(super) fn unit_rms_norm_inplace(x: &mut [f32], eps: f32) {
    if x.is_empty() {
        return;
    }
    let mut sum_sq = 0.0f32;
    for &v in x.iter() {
        sum_sq += v * v;
    }
    let inv = 1.0f32 / ((sum_sq / x.len() as f32) + eps).sqrt();
    for v in x.iter_mut() {
        *v *= inv;
    }
}

/// Resolve a per-block GGUF tensor by name. Pinned etalon 6ace6cc `LayerTensors`
/// omits Gemma 4 optional weights, and `find_layer_tensor` is private there.
pub(super) fn named_layer_tensor(
    index: &crate::gguf_sharder::GgufTensorIndex,
    layer: u32,
    suffix: &[u8],
) -> Option<etalon_model::inference::TensorInfo> {
    let mut name = [0u8; 96];
    let n = crate::gguf_sharder::write_blk_tensor_name(layer, suffix, &mut name);
    if n == 0 {
        None
    } else {
        index.tensor_info(&name[..n])
    }
}

pub(super) use crate::gguf_bridge::gemma4_qwen35::rope_neox_partial_inplace;
