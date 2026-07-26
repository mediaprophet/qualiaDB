//! CUDA-only context sizing.
//!
//! The portable engine keeps its conservative 1K host arena. A prepared CUDA plan can opt into a
//! larger dense device arena during cold construction without changing portable ABI state.

use crate::gguf_bridge::{KvCacheLayout, KV_CACHE_MAX_BYTES};

pub(crate) const MAX_CUDA_CONTEXT_WINDOW: u32 = 4096;

pub(super) fn configured_dense_layout(base: KvCacheLayout) -> Option<KvCacheLayout> {
    let mut layout = base.dense_device_layout()?;
    let requested = std::env::var("QUALIA_CUDA_CONTEXT")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(layout.max_context);
    if requested == 0 || requested > MAX_CUDA_CONTEXT_WINDOW {
        return None;
    }
    layout.max_context = requested;
    layout.layer_stride = requested
        .checked_mul(layout.slot_kv_elems)?
        .checked_mul(2)?;
    layout.total_f32_elems = (layout.n_layer as usize).checked_mul(layout.layer_stride as usize)?;
    layout
        .total_f32_elems
        .checked_mul(std::mem::size_of::<f32>())
        .filter(|bytes| *bytes <= KV_CACHE_MAX_BYTES)?;
    Some(layout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_k_smollm_dense_layout_stays_under_budget() {
        let base = KvCacheLayout {
            max_context: 1024,
            n_layer: 32,
            n_kv_head: 5,
            head_dim: 64,
            slot_kv_elems: 320,
            layer_stride: 1024 * 320 * 2,
            total_f32_elems: 32 * 1024 * 320 * 2,
            int8: false,
            dict_k: 0,
            dict_n_atoms: 0,
        };
        let mut expanded = base;
        expanded.max_context = MAX_CUDA_CONTEXT_WINDOW;
        expanded.layer_stride = MAX_CUDA_CONTEXT_WINDOW * expanded.slot_kv_elems * 2;
        expanded.total_f32_elems = expanded.n_layer as usize * expanded.layer_stride as usize;
        assert!(expanded.total_f32_elems * 4 <= KV_CACHE_MAX_BYTES);
    }
}
